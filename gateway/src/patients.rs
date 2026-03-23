//! Patients REST module — endpoints consumed by Hermóðr MCP sidecar.
//!
//! Sprint 6: Provides the REST surface that Hermóðr's MCP tools proxy to:
//! - GET  /api/patients?query=         → search_patients
//! - GET  /api/patients/:id/summary    → get_patient_summary
//! - POST /api/patients/:id/encounters → create_encounter
//! - GET  /api/patients/:id/sleep-reports?days= → get_sleep_reports

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::config::Config;

/// Build the patients router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/api/patients", get(search_patients))
        .route("/api/patients/{id}/summary", get(get_patient_summary))
        .route("/api/patients/{id}/encounters", post(create_encounter))
        .route("/api/patients/{id}/sleep-reports", get(get_sleep_reports))
}

// ─── DTOs ────────────────────────────────────────────────────────────────

/// Query parameters for patient search.
#[derive(Debug, Deserialize, Serialize)]
pub struct PatientSearchQuery {
    /// Free-text query: name, DOB, or PID.
    pub query: Option<String>,
}

/// Patient summary response.
#[derive(Debug, Serialize)]
pub struct PatientSummaryResponse {
    pub status: String,
    pub patient: Value,
    pub cpap_prescription: Value,
    pub sleep_data: Value,
    pub metadata: Value,
}

/// Request body to create an encounter.
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateEncounterRequest {
    /// Encounter type: follow_up, urgent, initial, data_review.
    #[serde(rename = "type")]
    pub encounter_type: String,
}

/// Query parameters for sleep reports.
#[derive(Debug, Deserialize, Serialize)]
pub struct SleepReportQuery {
    /// Number of days to look back (default: 30).
    pub days: Option<i32>,
}

/// Unified API response.
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub status: String,
    pub data: Value,
    pub metadata: Value,
}

// ─── Handlers ────────────────────────────────────────────────────────────

/// GET /api/patients?query= — Search patients by name, DOB, or identifier.
///
/// Proxies to FHIR Patient search with intelligent query parameter mapping.
async fn search_patients(
    State(config): State<Arc<Config>>,
    Query(params): Query<PatientSearchQuery>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<Value>)> {
    let query = params.query.unwrap_or_default();

    // Determine FHIR search parameter based on query format
    let fhir_params = build_patient_search_params(&query);

    let upstream_url = format!(
        "{}/apis/default/fhir/Patient?{}",
        config.openemr_url, fhir_params
    );

    tracing::info!(
        upstream_url = %upstream_url,
        query = %query,
        "Hermóðr search_patients"
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&upstream_url)
        .header("Accept", "application/fhir+json")
        .header("X-Gateway", "eir-gateway")
        .header("X-MCP-Tool", "search_patients")
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("FHIR upstream error: {e}")})),
            )
        })?;

    let status_code = response.status().as_u16();
    let body: Value = response
        .json()
        .await
        .unwrap_or(json!({"error": "Invalid JSON from upstream"}));

    Ok(Json(ApiResponse {
        status: if status_code < 400 {
            "success".into()
        } else {
            "error".into()
        },
        data: body,
        metadata: json!({
            "upstream_status": status_code,
            "tool": "search_patients",
            "query": query,
        }),
    }))
}

/// GET /api/patients/:id/summary — Aggregate patient demographics + CPAP + sleep data.
///
/// Fetches Patient resource, CPAP_Prescription LBF, and Sleep_Report_Data LBF
/// then merges into a single summary response.
async fn get_patient_summary(
    State(config): State<Arc<Config>>,
    Path(patient_id): Path<String>,
) -> Result<Json<PatientSummaryResponse>, (StatusCode, Json<Value>)> {
    let client = reqwest::Client::new();

    tracing::info!(
        patient_id = %patient_id,
        "Hermóðr get_patient_summary"
    );

    // 1. Fetch Patient demographics from FHIR
    let patient_url = format!(
        "{}/apis/default/fhir/Patient/{}",
        config.openemr_url, patient_id
    );
    let patient_data = fetch_json(&client, &patient_url).await;

    // 2. Fetch CPAP Prescription (LBF form via OpenEMR API)
    let cpap_url = format!(
        "{}/apis/default/api/patient/{}/medical_problem?type=CPAP_Prescription",
        config.openemr_url, patient_id
    );
    let cpap_data = fetch_json(&client, &cpap_url).await;

    // 3. Fetch latest Sleep Report data (LBF form via OpenEMR API)
    let sleep_url = format!(
        "{}/apis/default/api/patient/{}/medical_problem?type=Sleep_Report_Data&_count=7",
        config.openemr_url, patient_id
    );
    let sleep_data = fetch_json(&client, &sleep_url).await;

    Ok(Json(PatientSummaryResponse {
        status: "success".into(),
        patient: patient_data,
        cpap_prescription: cpap_data,
        sleep_data,
        metadata: json!({
            "patient_id": patient_id,
            "tool": "get_patient_summary",
            "gateway": "eir",
        }),
    }))
}

/// POST /api/patients/:id/encounters — Create a new encounter.
///
/// Proxies to FHIR Encounter create with proper patient reference.
async fn create_encounter(
    State(config): State<Arc<Config>>,
    Path(patient_id): Path<String>,
    Json(payload): Json<CreateEncounterRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<Value>)> {
    tracing::info!(
        patient_id = %patient_id,
        encounter_type = %payload.encounter_type,
        "Hermóðr create_encounter"
    );

    // Build FHIR Encounter resource
    let encounter = json!({
        "resourceType": "Encounter",
        "status": "planned",
        "class": {
            "system": "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            "code": map_encounter_type(&payload.encounter_type),
            "display": &payload.encounter_type
        },
        "subject": {
            "reference": format!("Patient/{}", patient_id)
        },
        "type": [{
            "coding": [{
                "system": "http://snomed.info/sct",
                "code": encounter_type_to_snomed(&payload.encounter_type),
                "display": &payload.encounter_type
            }]
        }],
        "period": {
            "start": chrono::Utc::now().to_rfc3339()
        }
    });

    let upstream_url = format!(
        "{}/apis/default/fhir/Encounter",
        config.openemr_url
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&upstream_url)
        .header("Content-Type", "application/fhir+json")
        .header("Accept", "application/fhir+json")
        .header("X-Gateway", "eir-gateway")
        .header("X-MCP-Tool", "create_encounter")
        .json(&encounter)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("FHIR upstream error: {e}")})),
            )
        })?;

    let status_code = response.status().as_u16();
    let body: Value = response
        .json()
        .await
        .unwrap_or(json!({"error": "Invalid JSON from upstream"}));

    Ok(Json(ApiResponse {
        status: if status_code < 400 {
            "success".into()
        } else {
            "error".into()
        },
        data: body,
        metadata: json!({
            "upstream_status": status_code,
            "tool": "create_encounter",
            "patient_id": patient_id,
            "encounter_type": payload.encounter_type,
        }),
    }))
}

/// GET /api/patients/:id/sleep-reports?days= — Query sleep therapy reports.
///
/// Fetches Sleep_Report_Data LBF entries for the specified patient and time range.
async fn get_sleep_reports(
    State(config): State<Arc<Config>>,
    Path(patient_id): Path<String>,
    Query(params): Query<SleepReportQuery>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<Value>)> {
    let days = params.days.unwrap_or(30);

    tracing::info!(
        patient_id = %patient_id,
        days = days,
        "Hermóðr get_sleep_reports"
    );

    // Calculate date range
    let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
    let since_str = since.format("%Y-%m-%d").to_string();

    // Query Sleep_Report_Data LBF via OpenEMR custom API
    let upstream_url = format!(
        "{}/apis/default/api/patient/{}/medical_problem?type=Sleep_Report_Data&date_from={}",
        config.openemr_url, patient_id, since_str
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&upstream_url)
        .header("Accept", "application/json")
        .header("X-Gateway", "eir-gateway")
        .header("X-MCP-Tool", "get_sleep_reports")
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("OpenEMR API error: {e}")})),
            )
        })?;

    let status_code = response.status().as_u16();
    let body: Value = response
        .json()
        .await
        .unwrap_or(json!({"error": "Invalid JSON from upstream"}));

    Ok(Json(ApiResponse {
        status: if status_code < 400 {
            "success".into()
        } else {
            "error".into()
        },
        data: body,
        metadata: json!({
            "upstream_status": status_code,
            "tool": "get_sleep_reports",
            "patient_id": patient_id,
            "days": days,
            "since": since_str,
        }),
    }))
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/// Fetch JSON from an upstream URL, returning a Value. Errors become null.
async fn fetch_json(client: &reqwest::Client, url: &str) -> Value {
    match client
        .get(url)
        .header("Accept", "application/json")
        .header("X-Gateway", "eir-gateway")
        .send()
        .await
    {
        Ok(resp) => resp.json().await.unwrap_or(json!(null)),
        Err(e) => {
            tracing::warn!(url = %url, error = %e, "Failed to fetch upstream data");
            json!({"error": format!("fetch failed: {e}")})
        }
    }
}

/// Build FHIR Patient search parameters from free-text query.
///
/// Heuristic: if query looks like a date → birthdate, if digits → identifier, else → name.
pub fn build_patient_search_params(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return "_count=20".to_string();
    }

    // Date pattern: YYYY-MM-DD
    if trimmed.len() == 10 && trimmed.chars().nth(4) == Some('-') && trimmed.chars().nth(7) == Some('-') {
        return format!("birthdate={}", trimmed);
    }

    // Numeric pattern: likely PID or identifier
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return format!("identifier={}", trimmed);
    }

    // Identifier pattern: contains prefix like SANDBOX-PT-
    if trimmed.contains('-') && trimmed.chars().any(|c| c.is_ascii_uppercase()) {
        return format!("identifier={}", trimmed);
    }

    // Default: name search
    format!("name={}", trimmed)
}

/// Map encounter type string to FHIR v3-ActCode.
fn map_encounter_type(t: &str) -> &str {
    match t {
        "follow_up" => "AMB",
        "urgent" => "EMER",
        "initial" => "AMB",
        "data_review" => "VR",
        _ => "AMB",
    }
}

/// Map encounter type to SNOMED-CT code.
fn encounter_type_to_snomed(t: &str) -> &str {
    match t {
        "follow_up" => "390906007",
        "urgent" => "103391001",
        "initial" => "185349003",
        "data_review" => "308335008",
        _ => "390906007",
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patients_router_creation() {
        let config = Arc::new(Config::from_env());
        let _app: axum::Router = router().with_state(config);
    }

    #[test]
    fn test_patient_search_query_serde() {
        let json_str = r#"{"query": "John Smith"}"#;
        let q: PatientSearchQuery = serde_json::from_str(json_str).unwrap();
        assert_eq!(q.query.unwrap(), "John Smith");
    }

    #[test]
    fn test_create_encounter_request_serde() {
        let json_str = r#"{"type": "follow_up"}"#;
        let req: CreateEncounterRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.encounter_type, "follow_up");
    }

    #[test]
    fn test_sleep_report_query_default_days() {
        let json_str = r#"{}"#;
        let q: SleepReportQuery = serde_json::from_str(json_str).unwrap();
        assert_eq!(q.days, None);
    }

    #[test]
    fn test_build_search_params_name() {
        let params = build_patient_search_params("สมชาย");
        assert!(params.starts_with("name="));
    }

    #[test]
    fn test_build_search_params_date() {
        let params = build_patient_search_params("1990-01-15");
        assert!(params.starts_with("birthdate="));
    }

    #[test]
    fn test_build_search_params_pid() {
        let params = build_patient_search_params("12345");
        assert!(params.starts_with("identifier="));
    }

    #[test]
    fn test_build_search_params_identifier() {
        let params = build_patient_search_params("SANDBOX-PT-001");
        assert!(params.starts_with("identifier="));
    }

    #[test]
    fn test_build_search_params_empty() {
        let params = build_patient_search_params("");
        assert_eq!(params, "_count=20");
    }

    #[test]
    fn test_map_encounter_type() {
        assert_eq!(map_encounter_type("follow_up"), "AMB");
        assert_eq!(map_encounter_type("urgent"), "EMER");
        assert_eq!(map_encounter_type("data_review"), "VR");
        assert_eq!(map_encounter_type("unknown"), "AMB");
    }

    #[test]
    fn test_encounter_type_to_snomed() {
        assert_eq!(encounter_type_to_snomed("follow_up"), "390906007");
        assert_eq!(encounter_type_to_snomed("urgent"), "103391001");
        assert_eq!(encounter_type_to_snomed("initial"), "185349003");
    }

    #[test]
    fn test_api_response_serialization() {
        let resp = ApiResponse {
            status: "success".into(),
            data: json!({"patients": []}),
            metadata: json!({"tool": "search_patients"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"tool\":\"search_patients\""));
    }
}
