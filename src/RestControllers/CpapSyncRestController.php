<?php

/**
 * CpapSyncRestController
 *
 * Handles inbound CPAP data synchronization from Mega Care.
 * Receives JSON payloads containing CPAP prescription and sleep report data,
 * validates them, and writes to OpenEMR's LBF tables with idempotency.
 *
 * @package   OpenEMR
 * @link      https://www.open-emr.org
 * @author    Eir Sprint 5
 * @license   https://github.com/openemr/openemr/blob/master/LICENSE GNU General Public License 3
 */

namespace OpenEMR\RestControllers;

use OpenEMR\Common\Logging\SystemLogger;

class CpapSyncRestController
{
    private $logger;

    public function __construct()
    {
        $this->logger = new SystemLogger();
    }

    /**
     * POST /api/cpap-sync
     *
     * Accepts CPAP data from Mega Care and writes to OpenEMR LBF tables.
     *
     * Expected JSON payload:
     * {
     *   "patient_id": "SANDBOX-PT-001",       // Mega Care patient ID
     *   "openemr_pid": 1001,                  // OpenEMR patient PID
     *   "data_type": "prescription|daily_report|compliance_report",
     *   "source_doc_id": "firestore-doc-id",
     *   "data": { ... }                       // Type-specific data
     * }
     *
     * @param array $data Decoded JSON body
     * @return array Response with status and message
     */
    public function sync($data)
    {
        // Validate required fields
        $requiredFields = ['patient_id', 'openemr_pid', 'data_type', 'data'];
        foreach ($requiredFields as $field) {
            if (empty($data[$field])) {
                return $this->errorResponse("Missing required field: {$field}", 400);
            }
        }

        $patientId = $data['patient_id'];
        $openemrPid = intval($data['openemr_pid']);
        $dataType = $data['data_type'];
        $sourceDocId = $data['source_doc_id'] ?? null;
        $syncData = $data['data'];

        // Validate data_type
        $validTypes = ['prescription', 'daily_report', 'compliance_report'];
        if (!in_array($dataType, $validTypes)) {
            return $this->errorResponse(
                "Invalid data_type: {$dataType}. Must be one of: " . implode(', ', $validTypes),
                400
            );
        }

        // Verify patient exists in OpenEMR
        if (!$this->patientExists($openemrPid)) {
            return $this->errorResponse("Patient with PID {$openemrPid} not found in OpenEMR", 404);
        }

        // Idempotency check via migration_log
        $idempotencyKey = "{$patientId}:{$dataType}:{$sourceDocId}";
        if ($this->isDuplicate($idempotencyKey)) {
            $this->logger->info("CPAP_SYNC: Duplicate request skipped", [
                'idempotency_key' => $idempotencyKey
            ]);
            return [
                'status' => 'skipped',
                'message' => 'Duplicate sync request — already processed',
                'idempotency_key' => $idempotencyKey,
            ];
        }

        // Process based on data_type
        try {
            switch ($dataType) {
                case 'prescription':
                    $result = $this->syncPrescription($openemrPid, $syncData);
                    break;
                case 'daily_report':
                case 'compliance_report':
                    $result = $this->syncSleepReport($openemrPid, $syncData, $dataType);
                    break;
                default:
                    return $this->errorResponse("Unsupported data_type: {$dataType}", 400);
            }

            // Log successful migration
            $this->logMigration($patientId, $openemrPid, $dataType, $sourceDocId, 'success', null, $idempotencyKey);

            $this->logger->info("CPAP_SYNC: Successfully synced {$dataType} for patient {$patientId}");

            return [
                'status' => 'success',
                'message' => "Successfully synced {$dataType}",
                'patient_id' => $patientId,
                'openemr_pid' => $openemrPid,
                'form_id' => $result['form_id'] ?? null,
            ];
        } catch (\Exception $e) {
            // Log failed migration
            $this->logMigration(
                $patientId, $openemrPid, $dataType, $sourceDocId,
                'failed', $e->getMessage(), $idempotencyKey
            );

            $this->logger->error("CPAP_SYNC: Failed to sync {$dataType} for patient {$patientId}", [
                'error' => $e->getMessage()
            ]);

            return $this->errorResponse("Sync failed: " . $e->getMessage(), 500);
        }
    }

    /**
     * Write CPAP prescription data to LBFcpap form.
     */
    private function syncPrescription(int $pid, array $data): array
    {
        $formFields = [
            'cpap_model'       => $data['cpap_model'] ?? '',
            'cpap_serial_no'   => $data['cpap_sn'] ?? $data['cpap_serial_no'] ?? '',
            'cpap_manufacturer'=> $data['manufacturer'] ?? '',
            'therapy_mode'     => $data['therapy_mode'] ?? 'cpap',
            'pressure_min'     => $data['pressure_min'] ?? '',
            'pressure_max'     => $data['pressure_max'] ?? '',
            'epr_level'        => $data['epr_level'] ?? '',
            'ramp_time'        => $data['ramp_time'] ?? '',
            'mask_type'        => $data['mask_type'] ?? '',
            'mask_model'       => $data['mask_model'] ?? '',
            'mask_size'        => $data['mask_size'] ?? '',
            'humidifier_level' => $data['humidifier_level'] ?? '',
        ];

        return $this->writeLbfForm($pid, 'LBFcpap', $formFields);
    }

    /**
     * Write sleep report data to LBFsleep form.
     */
    private function syncSleepReport(int $pid, array $data, string $reportType): array
    {
        $lbfReportType = ($reportType === 'compliance_report') ? 'compliance' : 'detail';

        $formFields = [
            'report_date'       => $data['report_date'] ?? date('Y-m-d'),
            'report_type'       => $lbfReportType,
            'usage_hours'       => $data['usage_hours'] ?? '',
            'usage_seconds'     => $data['usage_seconds'] ?? '',
            'total_usage_days'  => $data['total_usage_days'] ?? '',
            'ahi'               => $data['ahi'] ?? '',
            'leak_median'       => $data['leak_median'] ?? '',
            'leak_95th'         => $data['leak_95th'] ?? '',
            'pressure_median'   => $data['pressure_median'] ?? '',
            'pressure_95th'     => $data['pressure_95th'] ?? '',
            'csr_duration'      => $data['csr_duration'] ?? '',
            'triage_status'     => $data['triage_status'] ?? '',
            'pressure_efficacy' => $data['pressure_efficacy'] ?? '',
            'compliance_status' => $data['compliance_status'] ?? '',
            'clinical_notes'    => $data['clinical_notes'] ?? '',
            'source_gcs_uri'    => $data['source_gcs_uri'] ?? '',
        ];

        return $this->writeLbfForm($pid, 'LBFsleep', $formFields);
    }

    /**
     * Write data to an LBF form using OpenEMR's form mechanism.
     */
    private function writeLbfForm(int $pid, string $formDir, array $fields): array
    {
        // Create encounter if needed (auto-create for data migration)
        $encounterId = $this->getOrCreateEncounter($pid);

        // Insert into forms table
        $formId = sqlInsert(
            "INSERT INTO forms (date, encounter, form_name, form_id, pid, user, formdir) " .
            "VALUES (NOW(), ?, ?, 0, ?, 'cpap-sync-api', ?)",
            [$encounterId, $formDir, $pid, $formDir]
        );

        // Insert each field into lbf_data
        foreach ($fields as $fieldId => $value) {
            if ($value !== '' && $value !== null) {
                sqlStatement(
                    "INSERT INTO lbf_data (form_id, field_id, field_value) VALUES (?, ?, ?)",
                    [$formId, $fieldId, $value]
                );
            }
        }

        return ['form_id' => $formId, 'encounter_id' => $encounterId];
    }

    /**
     * Get existing encounter or create a new one for data migration.
     */
    private function getOrCreateEncounter(int $pid): int
    {
        // Check for today's migration encounter
        $today = date('Y-m-d');
        $existing = sqlQuery(
            "SELECT encounter FROM form_encounter " .
            "WHERE pid = ? AND date LIKE ? AND reason = 'CPAP Data Migration' " .
            "ORDER BY encounter DESC LIMIT 1",
            [$pid, "{$today}%"]
        );

        if ($existing && $existing['encounter']) {
            return intval($existing['encounter']);
        }

        // Generate new encounter number
        $maxEnc = sqlQuery("SELECT MAX(encounter) AS max_enc FROM form_encounter WHERE pid = ?", [$pid]);
        $newEncounter = ($maxEnc['max_enc'] ?? 0) + 1;

        sqlInsert(
            "INSERT INTO form_encounter (date, reason, facility, pid, encounter, onset_date) " .
            "VALUES (NOW(), 'CPAP Data Migration', '', ?, ?, NOW())",
            [$pid, $newEncounter]
        );

        return $newEncounter;
    }

    /**
     * Check if a patient exists in OpenEMR.
     */
    private function patientExists(int $pid): bool
    {
        $result = sqlQuery("SELECT pid FROM patient_data WHERE pid = ?", [$pid]);
        return !empty($result);
    }

    /**
     * Check if this sync operation was already processed (idempotency).
     */
    private function isDuplicate(string $idempotencyKey): bool
    {
        $result = sqlQuery(
            "SELECT id FROM migration_log WHERE idempotency_key = ? AND status = 'success'",
            [$idempotencyKey]
        );
        return !empty($result);
    }

    /**
     * Log migration event for audit and tracking.
     */
    private function logMigration(
        string $patientId,
        int $openemrPid,
        string $dataType,
        ?string $sourceDocId,
        string $status,
        ?string $errorMessage,
        string $idempotencyKey
    ): void {
        sqlInsert(
            "INSERT INTO migration_log " .
            "(patient_id, openemr_pid, data_type, source_doc_id, status, error_message, idempotency_key) " .
            "VALUES (?, ?, ?, ?, ?, ?, ?) " .
            "ON DUPLICATE KEY UPDATE status = VALUES(status), error_message = VALUES(error_message)",
            [$patientId, $openemrPid, $dataType, $sourceDocId, $status, $errorMessage, $idempotencyKey]
        );
    }

    /**
     * Helper to create error response.
     */
    private function errorResponse(string $message, int $code): array
    {
        http_response_code($code);
        return [
            'status' => 'error',
            'message' => $message,
            'code' => $code,
        ];
    }
}
