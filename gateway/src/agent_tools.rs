//! Agent Tools — REST endpoints for Bifrost agent integration.
//!
//! Provides structured endpoints that Bifrost agents can call to interact
//! with OpenEMR's FHIR R4 backend through the Eir gateway.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::config::Config;

/// Build the agent tools router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/v1/fhir/query", post(fhir_query))
        .route("/v1/patients/search", get(patients_search))
        .route("/v1/clinical/summary", post(clinical_summary))
}

// === Request / Response DTOs ===

/// FHIR natural language query request.
#[derive(Debug, Deserialize, Serialize)]
pub struct FhirQueryRequest {
    /// Natural language or structured query string.
    pub query: String,
    /// Optional FHIR resource type filter (e.g. "Patient", "Condition").
    pub resource_type: Option<String>,
    /// Optional patient ID scope.
    pub patient_id: Option<String>,
}

/// Patient search query parameters.
#[derive(Debug, Deserialize, Serialize)]
pub struct PatientSearchParams {
    /// Patient name (partial match).
    pub name: Option<String>,
    /// Date of birth (YYYY-MM-DD).
    pub birthdate: Option<String>,
    /// Patient identifier (MRN, etc.).
    pub identifier: Option<String>,
}

/// Clinical summary request.
#[derive(Debug, Deserialize, Serialize)]
pub struct ClinicalSummaryRequest {
    /// Patient ID to summarize.
    pub patient_id: String,
    /// Optional list of resource types to include.
    pub include: Option<Vec<String>>,
}

/// Structured API response wrapper.
#[derive(Debug, Serialize)]
pub struct AgentResponse {
    pub status: String,
    pub data: Value,
    pub metadata: Value,
}

// === Handlers ===

/// POST /v1/fhir/query — Transform natural language → FHIR search query.
async fn fhir_query(
    State(config): State<Arc<Config>>,
    Json(payload): Json<FhirQueryRequest>,
) -> Result<Json<AgentResponse>, (StatusCode, Json<Value>)> {
    let resource_type = payload.resource_type.as_deref().unwrap_or("Patient");

    // Build FHIR search URL from structured query
    let search_params = build_fhir_search_params(&payload.query, payload.patient_id.as_deref());
    let upstream_url = format!(
        "{}/apis/default/fhir/{}?{}",
        config.openemr_url, resource_type, search_params
    );

    tracing::info!(
        upstream_url = %upstream_url,
        query = %payload.query,
        resource_type = %resource_type,
        "Agent FHIR query"
    );

    // Forward to upstream FHIR endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(&upstream_url)
        .header("Accept", "application/fhir+json")
        .header("X-Gateway", "eir-gateway")
        .header("X-Agent-Request", "bifrost")
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("FHIR upstream error: {e}")})),
            )
        })?;

    let status_code = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(json!({"error": "Invalid JSON from upstream"}));

    Ok(Json(AgentResponse {
        status: if status_code < 400 { "success".into() } else { "error".into() },
        data: body,
        metadata: json!({
            "upstream_status": status_code,
            "resource_type": resource_type,
            "query": payload.query,
            "gateway": "eir",
        }),
    }))
}

/// GET /v1/patients/search — Search patients for agent workflows.
async fn patients_search(
    State(config): State<Arc<Config>>,
    Query(params): Query<PatientSearchParams>,
) -> Result<Json<AgentResponse>, (StatusCode, Json<Value>)> {
    // Build FHIR Patient search query
    let mut query_parts: Vec<String> = Vec::new();
    if let Some(name) = &params.name {
        query_parts.push(format!("name={}", name));
    }
    if let Some(dob) = &params.birthdate {
        query_parts.push(format!("birthdate={}", dob));
    }
    if let Some(id) = &params.identifier {
        query_parts.push(format!("identifier={}", id));
    }

    let query_string = if query_parts.is_empty() {
        "_count=20".to_string()
    } else {
        query_parts.join("&")
    };

    let upstream_url = format!(
        "{}/apis/default/fhir/Patient?{}",
        config.openemr_url, query_string
    );

    tracing::info!(
        upstream_url = %upstream_url,
        "Agent patient search"
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&upstream_url)
        .header("Accept", "application/fhir+json")
        .header("X-Gateway", "eir-gateway")
        .header("X-Agent-Request", "bifrost")
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": format!("Patient search error: {e}")})),
            )
        })?;

    let status_code = response.status().as_u16();
    let body: Value = response.json().await.unwrap_or(json!({"error": "Invalid JSON"}));

    Ok(Json(AgentResponse {
        status: if status_code < 400 { "success".into() } else { "error".into() },
        data: body,
        metadata: json!({
            "upstream_status": status_code,
            "search_params": params,
            "gateway": "eir",
        }),
    }))
}

/// POST /v1/clinical/summary — Aggregate clinical data for a patient.
async fn clinical_summary(
    State(config): State<Arc<Config>>,
    Json(payload): Json<ClinicalSummaryRequest>,
) -> Result<Json<AgentResponse>, (StatusCode, Json<Value>)> {
    let default_resources = vec![
        "Patient".to_string(),
        "Condition".to_string(),
        "MedicationRequest".to_string(),
        "AllergyIntolerance".to_string(),
    ];
    let resource_types = payload.include.as_ref().unwrap_or(&default_resources);

    let client = reqwest::Client::new();
    let mut summary = json!({});

    for resource_type in resource_types {
        let url = if resource_type == "Patient" {
            format!(
                "{}/apis/default/fhir/Patient/{}",
                config.openemr_url, payload.patient_id
            )
        } else {
            format!(
                "{}/apis/default/fhir/{}?patient={}",
                config.openemr_url, resource_type, payload.patient_id
            )
        };

        tracing::info!(
            url = %url,
            resource_type = %resource_type,
            patient_id = %payload.patient_id,
            "Fetching clinical data"
        );

        match client
            .get(&url)
            .header("Accept", "application/fhir+json")
            .header("X-Gateway", "eir-gateway")
            .send()
            .await
        {
            Ok(resp) => {
                let data: Value = resp.json().await.unwrap_or(json!(null));
                summary[resource_type.to_lowercase()] = data;
            }
            Err(e) => {
                tracing::warn!(
                    resource_type = %resource_type,
                    error = %e,
                    "Failed to fetch clinical resource"
                );
                summary[resource_type.to_lowercase()] =
                    json!({"error": format!("fetch failed: {e}")});
            }
        }
    }

    Ok(Json(AgentResponse {
        status: "success".into(),
        data: summary,
        metadata: json!({
            "patient_id": payload.patient_id,
            "resource_types": resource_types,
            "gateway": "eir",
        }),
    }))
}

/// Build FHIR search parameters from a natural language query string.
fn build_fhir_search_params(query: &str, patient_id: Option<&str>) -> String {
    let mut params = Vec::new();

    // Basic NL → FHIR param mapping
    let lower = query.to_lowercase();
    if lower.contains("name") || lower.contains("patient") {
        // Extract potential name value after common patterns
        params.push(format!("name={}", query.replace("name:", "").trim()));
    } else {
        // Fallback: use _content search
        params.push(format!("_content={}", query));
    }

    if let Some(pid) = patient_id {
        params.push(format!("patient={}", pid));
    }

    params.join("&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_tools_router_creation() {
        let config = Arc::new(Config::from_env());
        let _app: axum::Router = router().with_state(config);
    }

    #[test]
    fn test_fhir_query_request_serde() {
        let req = FhirQueryRequest {
            query: "find patients named John".into(),
            resource_type: Some("Patient".into()),
            patient_id: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("find patients named John"));
        let parsed: FhirQueryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.query, "find patients named John");
    }

    #[test]
    fn test_patient_search_params_serde() {
        let params = PatientSearchParams {
            name: Some("Smith".into()),
            birthdate: Some("1990-01-01".into()),
            identifier: None,
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("Smith"));
        assert!(json.contains("1990-01-01"));
    }

    #[test]
    fn test_clinical_summary_request_serde() {
        let req = ClinicalSummaryRequest {
            patient_id: "patient-123".into(),
            include: Some(vec!["Patient".into(), "Condition".into()]),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: ClinicalSummaryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.patient_id, "patient-123");
        assert_eq!(parsed.include.unwrap().len(), 2);
    }

    #[test]
    fn test_build_fhir_search_params_with_name() {
        let params = build_fhir_search_params("name: John Smith", None);
        assert!(params.contains("name="));
        assert!(params.contains("John Smith"));
    }

    #[test]
    fn test_build_fhir_search_params_with_patient_id() {
        let params = build_fhir_search_params("conditions", Some("patient-456"));
        assert!(params.contains("patient=patient-456"));
    }

    #[test]
    fn test_agent_response_serialization() {
        let resp = AgentResponse {
            status: "success".into(),
            data: json!({"test": true}),
            metadata: json!({"gateway": "eir"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"gateway\":\"eir\""));
    }
}
