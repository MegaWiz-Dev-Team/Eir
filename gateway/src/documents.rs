//! Patient document management — upload with OCR, list, and FHIR integration.

use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

use crate::config::Config;

#[derive(Deserialize)]
pub struct UploadRequest {
    pub file_base64: String,
    pub filename: String,
    pub doc_type: Option<String>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub source_id: i64,
    pub document_reference_id: Option<String>,
    pub filename: String,
    pub extracted_text_preview: String,
    pub ocr_engine: String,
}

#[derive(Serialize)]
pub struct DocumentListItem {
    pub source_id: i64,
    pub name: String,
    pub status: String,
    pub size_mb: Option<f64>,
    pub created_at: Option<String>,
    pub mimir_download_url: String,
}

#[derive(Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentListItem>,
}

/// POST /v1/patients/{patient_id}/documents — Upload a patient document
async fn upload_document(
    Path(patient_id): Path<String>,
    headers: HeaderMap,
    Extension(config): Extension<Arc<Config>>,
    Json(req): Json<UploadRequest>,
) -> Result<(StatusCode, Json<UploadResponse>), (StatusCode, Json<Value>)> {
    let tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    // Decode base64 file data
    let data = base64_decode(&req.file_base64).map_err(|e| {
        error!("Failed to decode base64: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid base64 data: {}", e)})),
        )
    })?;

    let original_filename = &req.filename;
    let doc_type = req.doc_type.unwrap_or_else(|| "printed_thai".to_string());

    // 1. OCR via Syn
    let image_base64 = req.file_base64.clone();
    let syn_response = call_syn_ocr(
        &config.syn_url,
        &image_base64,
        original_filename,
        &doc_type,
        tenant_id,
    )
    .await
    .map_err(|e| {
        error!("Syn OCR failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("OCR failed: {}", e)})),
        )
    })?;

    let extracted_text = syn_response.get("extracted_text").and_then(|v| v.as_str()).unwrap_or("");
    let ocr_engine = syn_response.get("engine_used").and_then(|v| v.as_str()).unwrap_or("unknown");
    let audit_id = syn_response.get("audit_id").and_then(|v| v.as_str()).unwrap_or("");

    // 2. Upload to Mimir
    let source_id = upload_to_mimir(
        &config.mimir_url,
        &data,
        &original_filename,
        tenant_id,
        &patient_id,
    )
    .await
    .map_err(|e| {
        error!("Mimir upload failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Upload failed: {}", e)})),
        )
    })?;

    // 3. Trigger embed-chunks in Mimir
    let _ = trigger_mimir_embed(&config.mimir_url, source_id, tenant_id).await;

    // 4. Create FHIR DocumentReference in OpenEMR
    let document_reference_id = create_fhir_document_reference(
        &config.openemr_url,
        &patient_id,
        source_id,
        &original_filename,
        data.len(),
        ocr_engine,
        audit_id,
        &config.mimir_url,
    )
    .await
    .ok();

    // 5. Return response
    let preview = extracted_text.chars().take(300).collect::<String>();

    info!(
        "Document uploaded: patient_id={}, source_id={}, filename={}",
        patient_id, source_id, original_filename
    );

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            source_id,
            document_reference_id,
            filename: original_filename.clone(),
            extracted_text_preview: preview,
            ocr_engine: ocr_engine.to_string(),
        }),
    ))
}

/// GET /v1/patients/{patient_id}/documents — List patient's documents
async fn list_documents(
    Path(patient_id): Path<String>,
    headers: HeaderMap,
    Extension(config): Extension<Arc<Config>>,
) -> Result<Json<DocumentListResponse>, (StatusCode, Json<Value>)> {
    let tenant_id = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    // Query Mimir database (via HTTP call to search endpoint with filters)
    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/sources", config.mimir_url))
        .header("X-Tenant-ID", tenant_id)
        .query(&[("patient_id", &patient_id)])
        .send()
        .await
        .map_err(|e| {
            error!("Failed to query Mimir: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to list documents"})),
            )
        })?;

    let items: Vec<Value> = resp
        .json()
        .await
        .unwrap_or_default();

    let documents = items
        .into_iter()
        .map(|item| DocumentListItem {
            source_id: item.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            name: item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            status: item.get("last_sync_status").and_then(|v| v.as_str()).unwrap_or("PENDING").to_string(),
            size_mb: item.get("mb_size").and_then(|v| v.as_f64()),
            created_at: item.get("created_at").and_then(|v| v.as_str()).map(|s| s.to_string()),
            mimir_download_url: format!("{}/api/v1/sources/{}/file", config.mimir_url,
                item.get("id").and_then(|v| v.as_i64()).unwrap_or(0)),
        })
        .collect();

    Ok(Json(DocumentListResponse { documents }))
}

// ─── Helper Functions ───────────────────────────────────────────────────────

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let bytes = s.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        let b1 = alphabet.find(bytes[i] as char).ok_or("Invalid base64 char")?;
        let b2 = if i + 1 < bytes.len() {
            alphabet.find(bytes[i + 1] as char).ok_or("Invalid base64 char")?
        } else {
            0
        };
        let b3 = if i + 2 < bytes.len() && bytes[i + 2] != b'=' as u8 {
            alphabet.find(bytes[i + 2] as char).ok_or("Invalid base64 char")?
        } else {
            0
        };
        let b4 = if i + 3 < bytes.len() && bytes[i + 3] != b'=' as u8 {
            alphabet.find(bytes[i + 3] as char).ok_or("Invalid base64 char")?
        } else {
            0
        };

        let n = ((b1 << 18) | (b2 << 12) | (b3 << 6) | b4) as u32;
        result.push((n >> 16) as u8);
        if i + 2 < bytes.len() && bytes[i + 2] != b'=' as u8 {
            result.push((n >> 8) as u8);
        }
        if i + 3 < bytes.len() && bytes[i + 3] != b'=' as u8 {
            result.push(n as u8);
        }

        i += 4;
    }

    Ok(result)
}

async fn call_syn_ocr(
    syn_url: &str,
    image_base64: &str,
    filename: &str,
    doc_type: &str,
    tenant_id: &str,
) -> Result<Value, String> {
    let body = json!({
        "image_base64": image_base64,
        "filename": filename,
        "doc_type": doc_type,
        "high_stakes": true,
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/syn/ocr/extract-json", syn_url))
        .header("X-Tenant-Id", tenant_id)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Syn request failed: {}", e))?;

    resp.json().await.map_err(|e| format!("Failed to parse Syn response: {}", e))
}

async fn upload_to_mimir(
    mimir_url: &str,
    file_data: &[u8],
    filename: &str,
    tenant_id: &str,
    patient_id: &str,
) -> Result<i64, String> {
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(file_data.to_vec()).file_name(filename.to_string()))
        .text("name", filename.to_string())
        .text("source_type", "document".to_string())
        .text("patient_id", patient_id.to_string());

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/v1/sources/upload", mimir_url))
        .header("X-Tenant-ID", tenant_id)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Mimir upload request failed: {}", e))?;

    let json: Value = resp.json().await.map_err(|e| format!("Failed to parse Mimir response: {}", e))?;
    json.get("source_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "source_id not found in Mimir response".to_string())
}

async fn trigger_mimir_embed(mimir_url: &str, source_id: i64, tenant_id: &str) -> Result<(), String> {
    let body = json!({
        "source_id": source_id,
        "tenant_id": tenant_id,
    });

    reqwest::Client::new()
        .post(format!("{}/vector/embed-chunks", mimir_url))
        .header("X-Tenant-ID", tenant_id)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Embed trigger failed: {}", e))?;

    Ok(())
}

async fn create_fhir_document_reference(
    openemr_url: &str,
    patient_id: &str,
    source_id: i64,
    filename: &str,
    size_bytes: usize,
    ocr_engine: &str,
    audit_id: &str,
    mimir_url: &str,
) -> Result<String, String> {
    let now = chrono::Utc::now().to_rfc3339();

    let body = json!({
        "resourceType": "DocumentReference",
        "status": "current",
        "type": {
            "coding": [{
                "system": "http://loinc.org",
                "code": "34133-9"
            }]
        },
        "subject": {
            "reference": format!("Patient/{}", patient_id)
        },
        "date": now,
        "content": [{
            "attachment": {
                "contentType": "application/pdf",
                "url": format!("{}/api/v1/sources/{}/file", mimir_url, source_id),
                "title": filename,
                "size": size_bytes
            }
        }],
        "description": format!("OCR extracted via {} (audit: {})", ocr_engine, audit_id)
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/apis/default/fhir/DocumentReference", openemr_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("FHIR write failed: {}", e))?;

    let json: Value = resp.json().await.map_err(|e| format!("Failed to parse FHIR response: {}", e))?;
    json.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "id not found in FHIR response".to_string())
}

/// Router for document endpoints
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/v1/patients/{patient_id}/documents", post(upload_document))
        .route("/v1/patients/{patient_id}/documents", get(list_documents))
}
