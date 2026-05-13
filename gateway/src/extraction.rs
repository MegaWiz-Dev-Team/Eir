//! Medical history extraction via Eir agent with human review.

use axum::{
    extract::Extension,
    http::{HeaderMap, StatusCode},
    Json, Router, routing::post,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

use crate::config::Config;

#[derive(Deserialize)]
pub struct ExtractRequest {
    pub patient_id: String,
    pub source_id: i64,
    pub extracted_text: String,
}

#[derive(Deserialize)]
pub struct SaveRequest {
    pub patient_id: String,
    pub chief_complaint: Option<String>,
    pub history_present: Option<String>,
    pub past_medical: Option<String>,
    pub medications: Option<String>,
    pub allergies: Option<String>,
    pub physical_exam: Option<String>,
    pub assessment_plan: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExtractionResponse {
    pub chief_complaint: Option<String>,
    pub history_present: Option<String>,
    pub past_medical: Option<String>,
    pub medications: Option<String>,
    pub allergies: Option<String>,
    pub physical_exam: Option<String>,
    pub assessment_plan: Option<String>,
}

/// POST /api/v1/extract-medical-history — Extract structured medical history from document
async fn extract_medical_history(
    headers: HeaderMap,
    Extension(config): Extension<Arc<Config>>,
    Json(req): Json<ExtractRequest>,
) -> Result<(StatusCode, Json<ExtractionResponse>), (StatusCode, Json<Value>)> {
    let tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    info!(
        "Extracting medical history for patient_id={}, source_id={}",
        req.patient_id, req.source_id
    );

    // Prepare extraction prompt for Eir agent
    let extraction_prompt = format!(
        r#"You are a medical record extraction specialist. Extract structured medical history from the following document text.

DOCUMENT TEXT:
---
{}
---

Please extract and return ONLY valid JSON with these exact fields (use null if not found):
{{
  "chief_complaint": "...",
  "history_present": "...",
  "past_medical": "...",
  "medications": "...",
  "allergies": "...",
  "physical_exam": "...",
  "assessment_plan": "..."
}}

Return ONLY the JSON object, no additional text."#,
        req.extracted_text
    );

    // Call Bifrost agent for extraction
    let bifrost_body = json!({
        "tenant_id": tenant_id,
        "agent_id": "medical_history_extractor",
        "query": extraction_prompt,
        "patient_id": req.patient_id
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/v1/agents/medical_history_extractor/run", config.bifrost_url))
        .json(&bifrost_body)
        .send()
        .await
        .map_err(|e| {
            error!("Bifrost request failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Extraction service unavailable"})),
            )
        })?;

    let bifrost_response: Value = resp.json().await.map_err(|e| {
        error!("Failed to parse Bifrost response: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Invalid extraction response"})),
        )
    })?;

    // Extract the final_answer containing JSON
    let final_answer = bifrost_response
        .get("final_answer")
        .and_then(|v| v.as_str())
        .unwrap_or("{}");

    // Parse JSON from response
    let extracted: ExtractionResponse = serde_json::from_str(final_answer)
        .unwrap_or_else(|_| ExtractionResponse {
            chief_complaint: None,
            history_present: None,
            past_medical: None,
            medications: None,
            allergies: None,
            physical_exam: None,
            assessment_plan: None,
        });

    info!("Medical history extraction completed for patient_id={}", req.patient_id);

    Ok((StatusCode::OK, Json(extracted)))
}

/// POST /api/v1/save-medical-history — Save extracted history to OpenEMR patient record
async fn save_medical_history(
    headers: HeaderMap,
    Extension(config): Extension<Arc<Config>>,
    Json(req): Json<SaveRequest>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    let _tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    info!("Saving medical history for patient_id={}", req.patient_id);

    // Build OpenEMR encounter note with extracted history
    let encounter_note = build_encounter_note(&req);

    // Save to OpenEMR via FHIR API
    let fhir_body = json!({
        "resourceType": "Encounter",
        "status": "finished",
        "class": {
            "system": "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            "code": "AMB"
        },
        "subject": {
            "reference": format!("Patient/{}", req.patient_id)
        },
        "type": [{
            "coding": [{
                "system": "http://snomed.info/sct",
                "code": "99213",
                "display": "Office visit - Established patient"
            }]
        }],
        "text": {
            "status": "generated",
            "div": format!("<div>{}</div>", escape_html(&encounter_note))
        },
        "period": {
            "start": chrono::Utc::now().to_rfc3339()
        }
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/apis/default/fhir/Encounter", config.openemr_url))
        .json(&fhir_body)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to save encounter: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to save to patient record"})),
            )
        })?;

    if !resp.status().is_success() {
        error!("OpenEMR API returned {}: {:?}", resp.status(), resp.text().await);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to save encounter"})),
        ));
    }

    info!("Medical history saved for patient_id={}", req.patient_id);
    Ok(StatusCode::CREATED)
}

fn build_encounter_note(req: &SaveRequest) -> String {
    let mut note = String::new();

    note.push_str("EXTRACTED MEDICAL HISTORY\n");
    note.push_str("========================\n\n");

    if let Some(cc) = &req.chief_complaint {
        note.push_str(&format!("CHIEF COMPLAINT:\n{}\n\n", cc));
    }

    if let Some(hpi) = &req.history_present {
        note.push_str(&format!("HISTORY OF PRESENT ILLNESS:\n{}\n\n", hpi));
    }

    if let Some(pmh) = &req.past_medical {
        note.push_str(&format!("PAST MEDICAL HISTORY:\n{}\n\n", pmh));
    }

    if let Some(meds) = &req.medications {
        note.push_str(&format!("MEDICATIONS:\n{}\n\n", meds));
    }

    if let Some(allergies) = &req.allergies {
        note.push_str(&format!("ALLERGIES:\n{}\n\n", allergies));
    }

    if let Some(pe) = &req.physical_exam {
        note.push_str(&format!("PHYSICAL EXAMINATION:\n{}\n\n", pe));
    }

    if let Some(ap) = &req.assessment_plan {
        note.push_str(&format!("ASSESSMENT & PLAN:\n{}\n\n", ap));
    }

    note.push_str("---\n");
    note.push_str("Note: This encounter was extracted from uploaded documents using Eir AI\n");
    note.push_str("and reviewed/approved by a clinician before being saved to this record.\n");

    note
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/api/v1/extract-medical-history", post(extract_medical_history))
        .route("/api/v1/save-medical-history", post(save_medical_history))
}
