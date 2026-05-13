<?php
/**
 * Medical History Extractor Module for OpenEMR
 *
 * Allows clinicians to upload medical documents and extract structured
 * medical history via Eir AI agent with human review before saving.
 */

declare(strict_types=1);

require_once('../../interface/globals.php');
require_once('../../src/Common/Session/SessionUtil.php');

use OpenEMR\Common\Acl\AclMain;
use OpenEMR\Services\PatientService;

// Check ACL - require patient_admin or similar permission
if (!AclMain::aclCheckCore('patients', 'med', '', 'write')) {
    http_response_code(403);
    exit(xlt("Access Denied"));
}

// Get patient ID from URL or session
$patient_id = $_GET['patient_id'] ?? $_SESSION['pid'] ?? null;
if (!$patient_id) {
    http_response_code(400);
    exit(xlt("Patient ID is required"));
}

// Verify patient exists and user has access
$patientService = new PatientService();
$patient = $patientService->getPatientData($patient_id);
if (!$patient) {
    http_response_code(404);
    exit(xlt("Patient not found"));
}

?>
<!DOCTYPE html>
<html lang="<?php echo attr(LANGUAGE_CHOICE) ?>">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title><?php echo xlt("Medical History Extractor") ?></title>
    <link href="<?php echo $GLOBALS['web_root']; ?>/images/favicon.ico" rel="icon">
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --mint-50: #f2fcf8;
            --mint-100: #e0f6eb;
            --mint-200: #bfeadb;
            --mint-300: #91d7c0;
            --mint-400: #5cbda0;
            --mint-500: #38a183;
            --mint-600: #298269;
            --mint-700: #236855;
            --mint-800: #1f5345;
            --mint-900: #1a453a;
            --text-primary: #0e2722;
            --text-secondary: #236855;
            --text-muted: #648a7f;
            --border: #e0ece6;
        }

        * { margin: 0; padding: 0; box-sizing: border-box; }

        body {
            font-family: 'Inter', -apple-system, sans-serif;
            background: #fcfdfe;
            color: var(--text-primary);
            padding: 20px;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
        }

        .header {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 24px;
        }

        .header h1 {
            font-size: 24px;
            font-weight: 700;
        }

        .header .patient-info {
            font-size: 14px;
            color: var(--text-muted);
            margin-left: auto;
        }

        .grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 24px;
        }

        .panel {
            background: white;
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 20px;
            box-shadow: 0 2px 8px rgba(41, 130, 105, 0.04);
        }

        .panel h2 {
            font-size: 16px;
            font-weight: 600;
            margin-bottom: 16px;
            padding-bottom: 12px;
            border-bottom: 1px solid var(--border);
        }

        .upload-zone {
            border: 2px dashed var(--mint-300);
            border-radius: 8px;
            padding: 24px;
            text-align: center;
            background: #f2fcf8;
            cursor: pointer;
            transition: all 0.3s;
        }

        .upload-zone:hover {
            border-color: var(--mint-500);
            background: rgba(56, 161, 131, 0.05);
        }

        .upload-zone.dragover {
            border-color: var(--mint-600);
            background: rgba(56, 161, 131, 0.1);
        }

        .upload-icon {
            font-size: 32px;
            margin-bottom: 12px;
        }

        .upload-text {
            font-size: 14px;
            font-weight: 500;
            margin-bottom: 4px;
        }

        .upload-hint {
            font-size: 12px;
            color: var(--text-muted);
        }

        #fileInput {
            display: none;
        }

        .button {
            padding: 8px 16px;
            border: none;
            border-radius: 6px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            transition: all 0.2s;
        }

        .button-primary {
            background: linear-gradient(135deg, var(--mint-500), var(--mint-600));
            color: white;
        }

        .button-primary:hover {
            transform: translateY(-1px);
            box-shadow: 0 4px 12px rgba(41, 130, 105, 0.2);
        }

        .button-secondary {
            background: white;
            color: var(--mint-600);
            border: 1px solid var(--mint-300);
        }

        .button-secondary:hover {
            background: var(--mint-50);
        }

        .button:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        .status {
            margin-top: 16px;
            padding: 12px;
            border-radius: 6px;
            font-size: 13px;
            display: none;
        }

        .status.loading {
            display: block;
            background: #e0f6eb;
            color: var(--mint-700);
        }

        .status.error {
            display: block;
            background: #fef2f2;
            color: #dc2626;
        }

        .status.success {
            display: block;
            background: #ecfdf5;
            color: #059669;
        }

        .extracted-data {
            display: none;
        }

        .extracted-data.visible {
            display: block;
        }

        .history-section {
            margin-bottom: 20px;
        }

        .history-section h3 {
            font-size: 13px;
            font-weight: 600;
            color: var(--text-secondary);
            margin-bottom: 8px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        textarea {
            width: 100%;
            padding: 8px;
            border: 1px solid var(--border);
            border-radius: 6px;
            font-size: 12px;
            font-family: 'SF Mono', 'Fira Code', monospace;
            resize: vertical;
            min-height: 80px;
        }

        textarea:focus {
            outline: none;
            border-color: var(--mint-400);
            box-shadow: 0 0 0 3px rgba(56, 161, 131, 0.1);
        }

        .button-group {
            display: flex;
            gap: 8px;
            margin-top: 16px;
        }

        .typing-indicator {
            display: flex;
            gap: 4px;
        }

        .typing-indicator span {
            width: 6px;
            height: 6px;
            background: var(--mint-500);
            border-radius: 50%;
            animation: typing 1.4s infinite;
        }

        .typing-indicator span:nth-child(2) { animation-delay: 0.2s; }
        .typing-indicator span:nth-child(3) { animation-delay: 0.4s; }

        @keyframes typing {
            0%, 60%, 100% { transform: translateY(0); }
            30% { transform: translateY(-6px); }
        }

        .extraction-log {
            background: #f8fafc;
            border: 1px solid var(--border);
            border-radius: 6px;
            padding: 12px;
            font-size: 12px;
            max-height: 200px;
            overflow-y: auto;
            font-family: 'SF Mono', 'Fira Code', monospace;
        }

        @media (max-width: 900px) {
            .grid {
                grid-template-columns: 1fr;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1><?php echo xlt("Medical History Extractor") ?></h1>
            <div class="patient-info">
                👤 <?php echo xlt("Patient") ?>: <strong><?php echo text($patient['fname'] . ' ' . $patient['lname']) ?></strong>
                (<?php echo text($patient_id) ?>)
            </div>
        </div>

        <div class="grid">
            <!-- Upload Panel -->
            <div class="panel">
                <h2><?php echo xlt("Upload Document") ?></h2>
                <div class="upload-zone" id="uploadZone">
                    <div class="upload-icon">📄</div>
                    <div class="upload-text"><?php echo xlt("Drag and drop or click to select") ?></div>
                    <div class="upload-hint"><?php echo xlt("Supported: PDF, JPG, PNG") ?></div>
                </div>
                <input type="file" id="fileInput" accept=".pdf,.jpg,.jpeg,.png">

                <div class="status" id="uploadStatus"></div>

                <div class="extraction-log" id="extractionLog" style="display: none;">
                    <div id="logContent"></div>
                </div>
            </div>

            <!-- Extracted Data Panel -->
            <div class="panel">
                <h2><?php echo xlt("Extracted Medical History") ?></h2>

                <div class="extracted-data" id="extractedData">
                    <div class="history-section">
                        <h3><?php echo xlt("Chief Complaint") ?></h3>
                        <textarea id="chiefComplaint" placeholder="<?php echo attr(xlt("Chief complaint will appear here")) ?>"></textarea>
                    </div>

                    <div class="history-section">
                        <h3><?php echo xlt("History of Present Illness") ?></h3>
                        <textarea id="historyPresent" placeholder="<?php echo attr(xlt("HPI will appear here")) ?>"></textarea>
                    </div>

                    <div class="history-section">
                        <h3><?php echo xlt("Past Medical History") ?></h3>
                        <textarea id="pastMedical" placeholder="<?php echo attr(xlt("PMH will appear here")) ?>"></textarea>
                    </div>

                    <div class="history-section">
                        <h3><?php echo xlt("Medications") ?></h3>
                        <textarea id="medications" placeholder="<?php echo attr(xlt("Current medications will appear here")) ?>"></textarea>
                    </div>

                    <div class="history-section">
                        <h3><?php echo xlt("Allergies") ?></h3>
                        <textarea id="allergies" placeholder="<?php echo attr(xlt("Known allergies will appear here")) ?>"></textarea>
                    </div>

                    <div class="history-section">
                        <h3><?php echo xlt("Physical Examination") ?></h3>
                        <textarea id="physicalExam" placeholder="<?php echo attr(xlt("Physical exam findings will appear here")) ?>"></textarea>
                    </div>

                    <div class="history-section">
                        <h3><?php echo xlt("Assessment & Plan") ?></h3>
                        <textarea id="assessmentPlan" placeholder="<?php echo attr(xlt("A&P will appear here")) ?>"></textarea>
                    </div>

                    <div class="button-group">
                        <button class="button button-primary" onclick="saveToPatientRecord()" id="saveBtn"><?php echo xlt("Save to Patient Record") ?></button>
                        <button class="button button-secondary" onclick="clearExtracted()"><?php echo xlt("Clear") ?></button>
                    </div>
                </div>

                <div id="emptyState" style="text-align: center; color: var(--text-muted); padding: 40px 20px;">
                    <div style="font-size: 32px; margin-bottom: 12px;">📋</div>
                    <div><?php echo xlt("Upload a document to extract medical history") ?></div>
                </div>
            </div>
        </div>
    </div>

    <script>
        const patientId = '<?php echo attr($patient_id) ?>';
        const uploadZone = document.getElementById('uploadZone');
        const fileInput = document.getElementById('fileInput');
        const uploadStatus = document.getElementById('uploadStatus');
        const extractedData = document.getElementById('extractedData');
        const emptyState = document.getElementById('emptyState');
        const extractionLog = document.getElementById('extractionLog');
        const logContent = document.getElementById('logContent');

        // File upload handlers
        uploadZone.addEventListener('click', () => fileInput.click());

        uploadZone.addEventListener('dragover', (e) => {
            e.preventDefault();
            uploadZone.classList.add('dragover');
        });

        uploadZone.addEventListener('dragleave', () => {
            uploadZone.classList.remove('dragover');
        });

        uploadZone.addEventListener('drop', (e) => {
            e.preventDefault();
            uploadZone.classList.remove('dragover');
            if (e.dataTransfer.files.length > 0) {
                fileInput.files = e.dataTransfer.files;
                handleFileSelect();
            }
        });

        fileInput.addEventListener('change', handleFileSelect);

        async function handleFileSelect() {
            const file = fileInput.files[0];
            if (!file) return;

            uploadStatus.textContent = '';
            extractionLog.style.display = 'block';
            logContent.innerHTML = '<div class="typing-indicator"><span></span><span></span><span></span></div> Uploading document...';
            uploadStatus.className = 'status loading';
            uploadStatus.textContent = '⏳ Extracting medical history...';

            const formData = new FormData();
            formData.append('file', file);

            try {
                // Upload to Eir gateway
                const uploadResp = await fetch('/v1/patients/' + patientId + '/documents', {
                    method: 'POST',
                    body: formData
                });

                if (!uploadResp.ok) {
                    throw new Error(await uploadResp.text());
                }

                const uploadData = await uploadResp.json();
                logContent.innerHTML += '<br>✓ Document uploaded (source_id: ' + uploadData.source_id + ')';

                // Call extraction endpoint
                const extractResp = await fetch('/api/v1/extract-medical-history', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        patient_id: patientId,
                        source_id: uploadData.source_id,
                        extracted_text: uploadData.extracted_text_preview
                    })
                });

                if (!extractResp.ok) {
                    throw new Error(await extractResp.text());
                }

                const extractData = await extractResp.json();
                logContent.innerHTML += '<br>✓ Extraction completed';
                logContent.innerHTML += '<br><br>Extracted fields:';
                logContent.innerHTML += '<br>- Chief Complaint: ' + (extractData.chief_complaint ? 'Yes' : 'No');
                logContent.innerHTML += '<br>- HPI: ' + (extractData.history_present ? 'Yes' : 'No');
                logContent.innerHTML += '<br>- PMH: ' + (extractData.past_medical ? 'Yes' : 'No');
                logContent.innerHTML += '<br>- Meds: ' + (extractData.medications ? 'Yes' : 'No');
                logContent.innerHTML += '<br>- Allergies: ' + (extractData.allergies ? 'Yes' : 'No');
                logContent.innerHTML += '<br>- Physical Exam: ' + (extractData.physical_exam ? 'Yes' : 'No');
                logContent.innerHTML += '<br>- A&P: ' + (extractData.assessment_plan ? 'Yes' : 'No');

                // Populate form
                document.getElementById('chiefComplaint').value = extractData.chief_complaint || '';
                document.getElementById('historyPresent').value = extractData.history_present || '';
                document.getElementById('pastMedical').value = extractData.past_medical || '';
                document.getElementById('medications').value = extractData.medications || '';
                document.getElementById('allergies').value = extractData.allergies || '';
                document.getElementById('physicalExam').value = extractData.physical_exam || '';
                document.getElementById('assessmentPlan').value = extractData.assessment_plan || '';

                emptyState.style.display = 'none';
                extractedData.classList.add('visible');

                uploadStatus.className = 'status success';
                uploadStatus.textContent = '✓ Medical history extracted successfully. Review and edit above, then save to patient record.';

            } catch (error) {
                logContent.innerHTML += '<br>❌ Error: ' + error.message;
                uploadStatus.className = 'status error';
                uploadStatus.textContent = '❌ Error: ' + error.message;
            }
        }

        async function saveToPatientRecord() {
            const saveBtn = document.getElementById('saveBtn');
            saveBtn.disabled = true;

            try {
                const resp = await fetch('/api/v1/save-medical-history', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        patient_id: patientId,
                        chief_complaint: document.getElementById('chiefComplaint').value,
                        history_present: document.getElementById('historyPresent').value,
                        past_medical: document.getElementById('pastMedical').value,
                        medications: document.getElementById('medications').value,
                        allergies: document.getElementById('allergies').value,
                        physical_exam: document.getElementById('physicalExam').value,
                        assessment_plan: document.getElementById('assessmentPlan').value
                    })
                });

                if (!resp.ok) {
                    throw new Error(await resp.text());
                }

                uploadStatus.className = 'status success';
                uploadStatus.textContent = '✓ Saved to patient record successfully!';
                setTimeout(() => {
                    uploadStatus.style.display = 'none';
                }, 3000);

            } catch (error) {
                uploadStatus.className = 'status error';
                uploadStatus.textContent = '❌ Error saving: ' + error.message;
            } finally {
                saveBtn.disabled = false;
            }
        }

        function clearExtracted() {
            document.querySelectorAll('textarea').forEach(ta => ta.value = '');
            emptyState.style.display = 'block';
            extractedData.classList.remove('visible');
            extractionLog.style.display = 'none';
            logContent.innerHTML = '';
            uploadStatus.style.display = 'none';
            fileInput.value = '';
        }
    </script>
</body>
</html>
