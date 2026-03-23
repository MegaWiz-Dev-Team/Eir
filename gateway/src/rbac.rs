//! RBAC middleware — Role-based access control for Hermóðr MCP tools.
//!
//! Sprint 6: Controls which MCP tools each role can access via the chat interface.
//!
//! Role matrix:
//!   doctor → all tools
//!   nurse  → search_patients, get_patient_summary, get_sleep_reports (no create_encounter)
//!   admin  → no clinical data access

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Json, Response},
};
use serde::Serialize;
use serde_json::json;

/// Role enum for RBAC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Role {
    Doctor,
    Nurse,
    Admin,
    Unknown,
}

impl Role {
    /// Parse role from string (case-insensitive).
    pub fn from_str_role(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "doctor" | "physician" | "md" => Role::Doctor,
            "nurse" | "rn" | "lpn" => Role::Nurse,
            "admin" | "administrator" | "receptionist" => Role::Admin,
            _ => Role::Unknown,
        }
    }
}

/// Permission check for a given role and path + method.
pub fn check_permission(role: &Role, path: &str, method: &str) -> bool {
    // Doctor: full access
    if *role == Role::Doctor {
        return true;
    }

    // Admin: no clinical data access at all
    if *role == Role::Admin {
        return false;
    }

    // Nurse: read-only clinical access (no encounter creation)
    if *role == Role::Nurse {
        // Block encounter creation
        if path.contains("/encounters") && method == "POST" {
            return false;
        }
        // Allow everything else under /api/patients
        return true;
    }

    // Unknown role: deny by default
    false
}

/// Extract role from request headers.
///
/// Priority:
/// 1. JWT claim (X-User-Role injected by auth middleware after JWT decode)
/// 2. Explicit X-User-Role header (for dev/testing)
///
/// Falls back to Unknown if no role found.
pub fn extract_role(request: &Request) -> Role {
    // Check X-User-Role header
    if let Some(role_header) = request
        .headers()
        .get("x-user-role")
        .and_then(|v| v.to_str().ok())
    {
        return Role::from_str_role(role_header);
    }

    // No role found → Unknown (deny by default)
    Role::Unknown
}

/// RBAC middleware for patient API endpoints.
///
/// Extracts the user role and checks permissions against the endpoint path.
/// Returns 403 Forbidden if the role lacks permission.
pub async fn rbac_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let path = request.uri().path().to_string();
    let method = request.method().as_str().to_string();

    // Only enforce RBAC on /api/patients and /v1/chat paths
    if !path.starts_with("/api/patients") && !path.starts_with("/v1/chat") {
        return Ok(next.run(request).await);
    }

    // Allow GET /v1/chat/status unauthenticated — it only reports Bifrost reachability,
    // contains no patient data, and is needed for the widget health indicator.
    if path == "/v1/chat/status" && method == "GET" {
        return Ok(next.run(request).await);
    }

    let role = extract_role(&request);

    tracing::debug!(
        role = ?role,
        path = %path,
        method = %method,
        "RBAC check"
    );

    if !check_permission(&role, &path, &method) {
        tracing::warn!(
            role = ?role,
            path = %path,
            method = %method,
            "RBAC denied"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Access denied",
                "role": format!("{:?}", role),
                "required": "Insufficient permissions for this endpoint",
                "hint": "Set X-User-Role header or ensure JWT contains role claims"
            })),
        ));
    }

    Ok(next.run(request).await)
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_parsing_doctor() {
        assert_eq!(Role::from_str_role("doctor"), Role::Doctor);
        assert_eq!(Role::from_str_role("Doctor"), Role::Doctor);
        assert_eq!(Role::from_str_role("physician"), Role::Doctor);
        assert_eq!(Role::from_str_role("MD"), Role::Doctor);
    }

    #[test]
    fn test_role_parsing_nurse() {
        assert_eq!(Role::from_str_role("nurse"), Role::Nurse);
        assert_eq!(Role::from_str_role("RN"), Role::Nurse);
        assert_eq!(Role::from_str_role("lpn"), Role::Nurse);
    }

    #[test]
    fn test_role_parsing_admin() {
        assert_eq!(Role::from_str_role("admin"), Role::Admin);
        assert_eq!(Role::from_str_role("Administrator"), Role::Admin);
        assert_eq!(Role::from_str_role("receptionist"), Role::Admin);
    }

    #[test]
    fn test_role_parsing_unknown() {
        assert_eq!(Role::from_str_role("random"), Role::Unknown);
        assert_eq!(Role::from_str_role(""), Role::Unknown);
    }

    #[test]
    fn test_doctor_full_access() {
        assert!(check_permission(&Role::Doctor, "/api/patients", "GET"));
        assert!(check_permission(&Role::Doctor, "/api/patients/1/summary", "GET"));
        assert!(check_permission(&Role::Doctor, "/api/patients/1/encounters", "POST"));
        assert!(check_permission(&Role::Doctor, "/api/patients/1/sleep-reports", "GET"));
    }

    #[test]
    fn test_nurse_read_access() {
        assert!(check_permission(&Role::Nurse, "/api/patients", "GET"));
        assert!(check_permission(&Role::Nurse, "/api/patients/1/summary", "GET"));
        assert!(check_permission(&Role::Nurse, "/api/patients/1/sleep-reports", "GET"));
    }

    #[test]
    fn test_nurse_no_encounter_creation() {
        assert!(!check_permission(&Role::Nurse, "/api/patients/1/encounters", "POST"));
    }

    #[test]
    fn test_admin_no_clinical_access() {
        assert!(!check_permission(&Role::Admin, "/api/patients", "GET"));
        assert!(!check_permission(&Role::Admin, "/api/patients/1/summary", "GET"));
        assert!(!check_permission(&Role::Admin, "/api/patients/1/encounters", "POST"));
        assert!(!check_permission(&Role::Admin, "/api/patients/1/sleep-reports", "GET"));
    }

    #[test]
    fn test_unknown_role_denied() {
        assert!(!check_permission(&Role::Unknown, "/api/patients", "GET"));
        assert!(!check_permission(&Role::Unknown, "/api/patients/1/encounters", "POST"));
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::Doctor;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"Doctor\"");
    }
}
