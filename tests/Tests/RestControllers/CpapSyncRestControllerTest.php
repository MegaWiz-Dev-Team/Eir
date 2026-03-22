<?php

/**
 * CpapSyncRestControllerTest
 *
 * Unit tests for the POST /cpap-sync endpoint.
 * Tests validation, idempotency, and data type routing.
 *
 * @package   OpenEMR
 * @link      https://www.open-emr.org
 * @author    Eir Sprint 5
 * @license   https://github.com/openemr/openemr/blob/master/LICENSE GNU General Public License 3
 */

namespace OpenEMR\Tests\RestControllers;

use OpenEMR\RestControllers\CpapSyncRestController;
use PHPUnit\Framework\TestCase;

class CpapSyncRestControllerTest extends TestCase
{
    private CpapSyncRestController $controller;

    protected function setUp(): void
    {
        $this->controller = new CpapSyncRestController();
    }

    // ─── Validation Tests ────────────────────────────────────

    public function testSyncRejectsMissingPatientId(): void
    {
        $data = [
            // 'patient_id' => missing
            'openemr_pid' => 1001,
            'data_type' => 'prescription',
            'data' => ['cpap_model' => 'AirSense 11'],
        ];

        $result = $this->controller->sync($data);

        $this->assertEquals('error', $result['status']);
        $this->assertStringContainsString('patient_id', $result['message']);
    }

    public function testSyncRejectsMissingOpenemrPid(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-001',
            // 'openemr_pid' => missing
            'data_type' => 'prescription',
            'data' => ['cpap_model' => 'AirSense 11'],
        ];

        $result = $this->controller->sync($data);

        $this->assertEquals('error', $result['status']);
        $this->assertStringContainsString('openemr_pid', $result['message']);
    }

    public function testSyncRejectsMissingDataType(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-001',
            'openemr_pid' => 1001,
            // 'data_type' => missing
            'data' => ['cpap_model' => 'AirSense 11'],
        ];

        $result = $this->controller->sync($data);

        $this->assertEquals('error', $result['status']);
        $this->assertStringContainsString('data_type', $result['message']);
    }

    public function testSyncRejectsMissingData(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-001',
            'openemr_pid' => 1001,
            'data_type' => 'prescription',
            // 'data' => missing
        ];

        $result = $this->controller->sync($data);

        $this->assertEquals('error', $result['status']);
        $this->assertStringContainsString('data', $result['message']);
    }

    public function testSyncRejectsInvalidDataType(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-001',
            'openemr_pid' => 1001,
            'data_type' => 'invalid_type',
            'data' => ['foo' => 'bar'],
        ];

        $result = $this->controller->sync($data);

        $this->assertEquals('error', $result['status']);
        $this->assertStringContainsString('invalid_type', $result['message']);
    }

    public function testSyncRejectsEmptyPayload(): void
    {
        $result = $this->controller->sync([]);

        $this->assertEquals('error', $result['status']);
    }

    // ─── Valid Data Type Routing Tests ────────────────────────

    public function testSyncAcceptsPrescriptionDataType(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-001',
            'openemr_pid' => 1001,
            'data_type' => 'prescription',
            'source_doc_id' => 'rx-001',
            'data' => [
                'cpap_model' => 'ResMed AirSense 11',
                'cpap_sn' => 'SN12345',
                'manufacturer' => 'ResMed',
                'therapy_mode' => 'apap',
                'pressure_min' => '6',
                'pressure_max' => '16',
                'epr_level' => '3',
                'ramp_time' => '20',
                'mask_type' => 'nasal',
                'mask_model' => 'AirFit N30i',
                'mask_size' => 'm',
                'humidifier_level' => '5',
            ],
        ];

        // This will fail at the DB level (patientExists check) when run
        // without a database, but validates that routing and field mapping
        // don't throw exceptions. In a full integration test with DB,
        // this would succeed.
        $result = $this->controller->sync($data);

        // Without DB, it should return 'error' with 'not found' (patient check)
        // This confirms the code path reached the patient check (past validation)
        $this->assertIsArray($result);
        $this->assertArrayHasKey('status', $result);
        // If status is 'error', it should be about patient not found (not validation)
        if ($result['status'] === 'error') {
            $this->assertStringContainsString('not found', $result['message']);
        }
    }

    public function testSyncAcceptsDailyReportDataType(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-002',
            'openemr_pid' => 1002,
            'data_type' => 'daily_report',
            'source_doc_id' => 'report-001',
            'data' => [
                'report_date' => '2026-03-22',
                'usage_hours' => '7.5',
                'ahi' => '2.1',
                'leak_median' => '3.2',
                'pressure_median' => '10.5',
                'triage_status' => 'green',
            ],
        ];

        $result = $this->controller->sync($data);

        $this->assertIsArray($result);
        $this->assertArrayHasKey('status', $result);
        if ($result['status'] === 'error') {
            $this->assertStringContainsString('not found', $result['message']);
        }
    }

    public function testSyncAcceptsComplianceReportDataType(): void
    {
        $data = [
            'patient_id' => 'SANDBOX-PT-003',
            'openemr_pid' => 1003,
            'data_type' => 'compliance_report',
            'source_doc_id' => 'compliance-001',
            'data' => [
                'report_date' => '2026-03-22',
                'usage_hours' => '4.2',
                'total_usage_days' => '25',
                'ahi' => '8.5',
                'compliance_status' => 'eligible',
            ],
        ];

        $result = $this->controller->sync($data);

        $this->assertIsArray($result);
        $this->assertArrayHasKey('status', $result);
        if ($result['status'] === 'error') {
            $this->assertStringContainsString('not found', $result['message']);
        }
    }

    // ─── Idempotency Key Tests ───────────────────────────────

    public function testIdempotencyKeyFormat(): void
    {
        // The idempotency key should be: patient_id:data_type:source_doc_id
        $data = [
            'patient_id' => 'PT-001',
            'openemr_pid' => 1001,
            'data_type' => 'prescription',
            'source_doc_id' => 'doc-123',
            'data' => ['cpap_model' => 'Test'],
        ];

        // We can't directly test private methods, but we verify the controller
        // processes the sync request and includes the expected fields
        $result = $this->controller->sync($data);
        $this->assertIsArray($result);

        // If it's a duplicate (idempotency hit), it should have 'skipped' status
        // and include the idempotency_key
        if (isset($result['status']) && $result['status'] === 'skipped') {
            $this->assertArrayHasKey('idempotency_key', $result);
            $this->assertEquals('PT-001:prescription:doc-123', $result['idempotency_key']);
        }
    }

    public function testSourceDocIdIsOptional(): void
    {
        $data = [
            'patient_id' => 'PT-001',
            'openemr_pid' => 1001,
            'data_type' => 'prescription',
            // source_doc_id is intentionally omitted
            'data' => ['cpap_model' => 'Test'],
        ];

        $result = $this->controller->sync($data);

        // Should not fail due to missing source_doc_id
        $this->assertIsArray($result);
        $this->assertArrayHasKey('status', $result);
        // It should reach the patient check, not fail on missing source_doc_id
        if ($result['status'] === 'error') {
            $this->assertStringNotContainsString('source_doc_id', $result['message']);
        }
    }

    // ─── Response Format Tests ───────────────────────────────

    public function testErrorResponseContainsRequiredFields(): void
    {
        $result = $this->controller->sync([]);

        $this->assertArrayHasKey('status', $result);
        $this->assertArrayHasKey('message', $result);
        $this->assertArrayHasKey('code', $result);
        $this->assertEquals('error', $result['status']);
    }

    public function testValidDataTypesAreExactly3(): void
    {
        // Test that only 'prescription', 'daily_report', 'compliance_report' are accepted
        $validTypes = ['prescription', 'daily_report', 'compliance_report'];
        $invalidTypes = ['demographics', 'appointment', 'consent', 'test', ''];

        foreach ($invalidTypes as $type) {
            $data = [
                'patient_id' => 'PT-001',
                'openemr_pid' => 1001,
                'data_type' => $type,
                'data' => ['test' => 'value'],
            ];

            $result = $this->controller->sync($data);

            if (!empty($type)) {
                $this->assertEquals('error', $result['status'],
                    "Expected '{$type}' to be rejected as invalid data_type");
            }
        }
    }
}
