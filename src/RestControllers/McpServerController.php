<?php

/**
 * McpServerController — MCP (Model Context Protocol) JSON-RPC Server
 *
 * Exposes OpenEMR FHIR tools via MCP JSON-RPC protocol for Bifrost AI agent.
 * Handles `tools/list` (discovery) and `tools/call` (execution).
 *
 * Tools:
 *   - search_patients: Search patients by name/DOB
 *   - get_patient_summary: Full patient summary (demographics + CPAP + sleep)
 *   - create_encounter: Create a new encounter
 *   - get_sleep_reports: Get sleep report history with triage status
 *
 * @package   OpenEMR
 * @link      https://www.open-emr.org
 * @author    Eir Sprint 6
 * @license   https://github.com/openemr/openemr/blob/master/LICENSE GNU General Public License 3
 */

namespace OpenEMR\RestControllers;

use OpenEMR\Common\Logging\SystemLogger;

class McpServerController
{
    private $logger;

    /** MCP protocol version */
    private const PROTOCOL_VERSION = '2024-11-05';

    /** Server capabilities */
    private const SERVER_INFO = [
        'name'    => 'eir-openemr',
        'version' => '0.5.0',
    ];

    /** Tool definitions with JSON Schema */
    private const TOOLS = [
        [
            'name'        => 'search_patients',
            'description' => 'Search patients by name, DOB, or patient ID. Returns matching patient records with demographics.',
            'inputSchema' => [
                'type'       => 'object',
                'properties' => [
                    'query' => [
                        'type'        => 'string',
                        'description' => 'Search query: patient name (Thai or English), DOB (YYYY-MM-DD), or PID number',
                    ],
                ],
                'required' => ['query'],
            ],
        ],
        [
            'name'        => 'get_patient_summary',
            'description' => 'Get comprehensive patient summary including demographics, CPAP prescription, and latest sleep therapy data with triage status.',
            'inputSchema' => [
                'type'       => 'object',
                'properties' => [
                    'patient_id' => [
                        'type'        => 'integer',
                        'description' => 'OpenEMR patient PID',
                    ],
                ],
                'required' => ['patient_id'],
            ],
        ],
        [
            'name'        => 'create_encounter',
            'description' => 'Create a new clinical encounter for a patient. Use for scheduling follow-ups or recording visits.',
            'inputSchema' => [
                'type'       => 'object',
                'properties' => [
                    'patient_id' => [
                        'type'        => 'integer',
                        'description' => 'OpenEMR patient PID',
                    ],
                    'type' => [
                        'type'        => 'string',
                        'description' => 'Encounter type: follow_up, urgent, initial, data_review',
                    ],
                ],
                'required' => ['patient_id', 'type'],
            ],
        ],
        [
            'name'        => 'get_sleep_reports',
            'description' => 'Get sleep therapy reports for a patient over a specified period. Shows usage hours, AHI, leak rates, and triage status.',
            'inputSchema' => [
                'type'       => 'object',
                'properties' => [
                    'patient_id' => [
                        'type'        => 'integer',
                        'description' => 'OpenEMR patient PID',
                    ],
                    'days' => [
                        'type'        => 'integer',
                        'description' => 'Number of days to look back (default: 30)',
                    ],
                ],
                'required' => ['patient_id'],
            ],
        ],
    ];

    public function __construct()
    {
        $this->logger = new SystemLogger();
    }

    /**
     * Handle incoming MCP JSON-RPC request.
     *
     * @param array $data Decoded JSON-RPC request body
     * @return array JSON-RPC response
     */
    public function handleRequest($data)
    {
        $jsonrpc = $data['jsonrpc'] ?? '';
        $id = $data['id'] ?? null;
        $method = $data['method'] ?? '';
        $params = $data['params'] ?? [];

        if ($jsonrpc !== '2.0') {
            return $this->jsonRpcError($id, -32600, 'Invalid JSON-RPC version');
        }

        $this->logger->info("MCP: Received {$method}", ['id' => $id]);

        switch ($method) {
            case 'initialize':
                return $this->handleInitialize($id, $params);

            case 'tools/list':
                return $this->handleToolsList($id);

            case 'tools/call':
                return $this->handleToolsCall($id, $params);

            case 'notifications/initialized':
                // Notification — no response needed
                return null;

            default:
                return $this->jsonRpcError($id, -32601, "Method not found: {$method}");
        }
    }

    // ─── JSON-RPC Handlers ─────────────────────────

    private function handleInitialize(int|string|null $id, array $params): array
    {
        return [
            'jsonrpc' => '2.0',
            'id'      => $id,
            'result'  => [
                'protocolVersion' => self::PROTOCOL_VERSION,
                'capabilities'    => [
                    'tools' => ['listChanged' => false],
                ],
                'serverInfo' => self::SERVER_INFO,
            ],
        ];
    }

    private function handleToolsList(int|string|null $id): array
    {
        return [
            'jsonrpc' => '2.0',
            'id'      => $id,
            'result'  => [
                'tools' => self::TOOLS,
            ],
        ];
    }

    private function handleToolsCall(int|string|null $id, array $params): array
    {
        $toolName = $params['name'] ?? '';
        $arguments = $params['arguments'] ?? [];

        try {
            $result = match ($toolName) {
                'search_patients'     => $this->toolSearchPatients($arguments),
                'get_patient_summary' => $this->toolGetPatientSummary($arguments),
                'create_encounter'    => $this->toolCreateEncounter($arguments),
                'get_sleep_reports'   => $this->toolGetSleepReports($arguments),
                default               => throw new \InvalidArgumentException("Unknown tool: {$toolName}"),
            };

            $this->logger->info("MCP: Tool {$toolName} executed successfully");

            return [
                'jsonrpc' => '2.0',
                'id'      => $id,
                'result'  => [
                    'content' => [
                        ['type' => 'text', 'text' => $result],
                    ],
                ],
            ];
        } catch (\Exception $e) {
            $this->logger->error("MCP: Tool {$toolName} failed", ['error' => $e->getMessage()]);

            return $this->jsonRpcError($id, -32000, "Tool error: " . $e->getMessage());
        }
    }

    // ─── Tool Implementations ──────────────────────

    /**
     * search_patients — Search by name, DOB, or PID.
     */
    private function toolSearchPatients(array $args): string
    {
        $query = trim($args['query'] ?? '');
        if (empty($query)) {
            throw new \InvalidArgumentException('query is required');
        }

        // Try PID match first
        if (is_numeric($query)) {
            $rows = sqlStatement(
                "SELECT pid, fname, lname, DOB, sex, phone_home FROM patient_data WHERE pid = ?",
                [$query]
            );
        } else {
            // Name search (supports Thai and English)
            $like = "%{$query}%";
            $rows = sqlStatement(
                "SELECT pid, fname, lname, DOB, sex, phone_home FROM patient_data " .
                "WHERE fname LIKE ? OR lname LIKE ? OR CONCAT(fname, ' ', lname) LIKE ? " .
                "ORDER BY lname, fname LIMIT 20",
                [$like, $like, $like]
            );
        }

        $patients = [];
        while ($row = sqlFetchArray($rows)) {
            $patients[] = [
                'pid'   => $row['pid'],
                'name'  => trim($row['fname'] . ' ' . $row['lname']),
                'dob'   => $row['DOB'],
                'sex'   => $row['sex'],
                'phone' => $row['phone_home'],
            ];
        }

        if (empty($patients)) {
            return "ไม่พบผู้ป่วยที่ตรงกับ \"{$query}\"";
        }

        $lines = ["พบผู้ป่วย " . count($patients) . " ราย:"];
        foreach ($patients as $p) {
            $lines[] = "- PID {$p['pid']}: {$p['name']} (DOB: {$p['dob']}, {$p['sex']}, Tel: {$p['phone']})";
        }
        return implode("\n", $lines);
    }

    /**
     * get_patient_summary — Demographics + CPAP + latest sleep data.
     */
    private function toolGetPatientSummary(array $args): string
    {
        $pid = intval($args['patient_id'] ?? 0);
        if ($pid <= 0) {
            throw new \InvalidArgumentException('patient_id is required');
        }

        // Demographics
        $patient = sqlQuery(
            "SELECT pid, fname, lname, DOB, sex, street, city, phone_home FROM patient_data WHERE pid = ?",
            [$pid]
        );
        if (empty($patient)) {
            return "ไม่พบผู้ป่วย PID {$pid}";
        }

        $name = trim($patient['fname'] . ' ' . $patient['lname']);
        $age = date_diff(date_create($patient['DOB']), date_create('today'))->y;

        $lines = [
            "## ข้อมูลผู้ป่วย: {$name}",
            "- PID: {$pid}",
            "- อายุ: {$age} ปี ({$patient['DOB']})",
            "- เพศ: {$patient['sex']}",
            "- ที่อยู่: {$patient['street']}, {$patient['city']}",
            "- โทร: {$patient['phone_home']}",
        ];

        // CPAP Prescription (latest LBFcpap form)
        $cpapForm = sqlQuery(
            "SELECT f.form_id FROM forms f " .
            "WHERE f.pid = ? AND f.formdir = 'LBFcpap' AND f.deleted = 0 " .
            "ORDER BY f.date DESC LIMIT 1",
            [$pid]
        );

        if (!empty($cpapForm)) {
            $formId = $cpapForm['form_id'];
            $cpapData = $this->getLbfData($formId);

            $lines[] = "";
            $lines[] = "## CPAP Prescription";
            $lines[] = "- เครื่อง: " . ($cpapData['cpap_model'] ?? 'N/A');
            $lines[] = "- S/N: " . ($cpapData['cpap_serial_no'] ?? 'N/A');
            $lines[] = "- Mode: " . ($cpapData['therapy_mode'] ?? 'N/A');
            $lines[] = "- Pressure: " . ($cpapData['pressure_min'] ?? '?') . "-" . ($cpapData['pressure_max'] ?? '?') . " cmH₂O";
            $lines[] = "- EPR: Level " . ($cpapData['epr_level'] ?? 'N/A');
            $lines[] = "- Mask: " . ($cpapData['mask_type'] ?? 'N/A') . " (" . ($cpapData['mask_model'] ?? '') . " " . ($cpapData['mask_size'] ?? '') . ")";
        }

        // Latest Sleep Report (latest LBFsleep form)
        $sleepForm = sqlQuery(
            "SELECT f.form_id FROM forms f " .
            "WHERE f.pid = ? AND f.formdir = 'LBFsleep' AND f.deleted = 0 " .
            "ORDER BY f.date DESC LIMIT 1",
            [$pid]
        );

        if (!empty($sleepForm)) {
            $formId = $sleepForm['form_id'];
            $sleepData = $this->getLbfData($formId);

            $triage = $sleepData['triage_status'] ?? 'unknown';
            $triageEmoji = match ($triage) {
                'green'  => '🟢',
                'yellow' => '🟡',
                'red'    => '🔴',
                default  => '⚪',
            };

            $lines[] = "";
            $lines[] = "## Latest Sleep Report ({$sleepData['report_date']})";
            $lines[] = "- Triage: {$triageEmoji} {$triage}";
            $lines[] = "- Usage: {$sleepData['usage_hours']} ชม./คืน";
            $lines[] = "- AHI: {$sleepData['ahi']} events/hr";
            $lines[] = "- Leak 95th: {$sleepData['leak_95th']} L/min";
            $lines[] = "- Pressure: {$sleepData['pressure_median']} (median), {$sleepData['pressure_95th']} (95th)";
            $lines[] = "- Compliance: " . ($sleepData['compliance_status'] ?? 'N/A');
            if (!empty($sleepData['clinical_notes'])) {
                $lines[] = "- Notes: {$sleepData['clinical_notes']}";
            }
        }

        return implode("\n", $lines);
    }

    /**
     * create_encounter — Create a new encounter.
     */
    private function toolCreateEncounter(array $args): string
    {
        $pid = intval($args['patient_id'] ?? 0);
        $type = $args['type'] ?? 'follow_up';

        if ($pid <= 0) {
            throw new \InvalidArgumentException('patient_id is required');
        }

        // Verify patient exists
        $patient = sqlQuery("SELECT pid, fname, lname FROM patient_data WHERE pid = ?", [$pid]);
        if (empty($patient)) {
            throw new \InvalidArgumentException("Patient PID {$pid} not found");
        }

        $reasonMap = [
            'follow_up'   => 'CPAP Follow-up',
            'urgent'      => 'CPAP Urgent Review',
            'initial'     => 'CPAP Initial Assessment',
            'data_review' => 'Sleep Data Review',
        ];
        $reason = $reasonMap[$type] ?? "CPAP {$type}";

        // Generate encounter number
        $maxEnc = sqlQuery("SELECT MAX(encounter) AS max_enc FROM form_encounter WHERE pid = ?", [$pid]);
        $newEncounter = ($maxEnc['max_enc'] ?? 0) + 1;

        sqlInsert(
            "INSERT INTO form_encounter (date, reason, facility, pid, encounter, onset_date, sensitivity, pc_catid) " .
            "VALUES (NOW(), ?, 'Sandbox Clinic', ?, ?, NOW(), 'normal', 5)",
            [$reason, $pid, $newEncounter]
        );

        $name = trim($patient['fname'] . ' ' . $patient['lname']);
        return "สร้าง Encounter สำเร็จ\n- ผู้ป่วย: {$name} (PID {$pid})\n- Encounter: #{$newEncounter}\n- ประเภท: {$reason}\n- วันที่: " . date('Y-m-d H:i');
    }

    /**
     * get_sleep_reports — Sleep report history with triage.
     */
    private function toolGetSleepReports(array $args): string
    {
        $pid = intval($args['patient_id'] ?? 0);
        $days = intval($args['days'] ?? 30);

        if ($pid <= 0) {
            throw new \InvalidArgumentException('patient_id is required');
        }

        $patient = sqlQuery("SELECT fname, lname FROM patient_data WHERE pid = ?", [$pid]);
        if (empty($patient)) {
            return "ไม่พบผู้ป่วย PID {$pid}";
        }

        $cutoff = date('Y-m-d', strtotime("-{$days} days"));
        $forms = sqlStatement(
            "SELECT f.form_id, f.date FROM forms f " .
            "WHERE f.pid = ? AND f.formdir = 'LBFsleep' AND f.deleted = 0 AND f.date >= ? " .
            "ORDER BY f.date DESC",
            [$pid, $cutoff]
        );

        $reports = [];
        while ($row = sqlFetchArray($forms)) {
            $data = $this->getLbfData($row['form_id']);
            $triage = $data['triage_status'] ?? 'unknown';
            $triageEmoji = match ($triage) {
                'green'  => '🟢',
                'yellow' => '🟡',
                'red'    => '🔴',
                default  => '⚪',
            };

            $reports[] = [
                'date'       => $data['report_date'] ?? $row['date'],
                'triage'     => "{$triageEmoji} {$triage}",
                'usage'      => ($data['usage_hours'] ?? '?') . ' ชม.',
                'ahi'        => $data['ahi'] ?? '?',
                'leak_95th'  => $data['leak_95th'] ?? '?',
                'compliance' => $data['compliance_status'] ?? '?',
            ];
        }

        $name = trim($patient['fname'] . ' ' . $patient['lname']);

        if (empty($reports)) {
            return "ไม่พบ Sleep Report ของ {$name} (PID {$pid}) ใน {$days} วันที่ผ่านมา";
        }

        $lines = ["## Sleep Reports: {$name} (PID {$pid}) — {$days} วันล่าสุด", ""];
        $lines[] = "| วันที่ | Triage | Usage | AHI | Leak 95th | Compliance |";
        $lines[] = "|--------|--------|-------|-----|-----------|------------|";
        foreach ($reports as $r) {
            $lines[] = "| {$r['date']} | {$r['triage']} | {$r['usage']} | {$r['ahi']} | {$r['leak_95th']} | {$r['compliance']} |";
        }

        return implode("\n", $lines);
    }

    // ─── Helpers ───────────────────────────────────

    /**
     * Get all LBF field values for a form instance.
     */
    private function getLbfData(int $formId): array
    {
        $rows = sqlStatement(
            "SELECT field_id, field_value FROM lbf_data WHERE form_id = ?",
            [$formId]
        );

        $data = [];
        while ($row = sqlFetchArray($rows)) {
            $data[$row['field_id']] = $row['field_value'];
        }
        return $data;
    }

    /**
     * Build a JSON-RPC error response.
     */
    private function jsonRpcError(int|string|null $id, int $code, string $message): array
    {
        return [
            'jsonrpc' => '2.0',
            'id'      => $id,
            'error'   => [
                'code'    => $code,
                'message' => $message,
            ],
        ];
    }
}
