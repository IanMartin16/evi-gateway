// src/health/models.rs

use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub environment: String,
    pub stack: String,
}

#[derive(Serialize, ToSchema)]
pub struct HealthCheck {
    pub status: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct HealthV1Response {
    pub contract_version: String,
    pub service: ServiceInfo,
    pub status: String,
    pub readiness: String,
    pub timestamp: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, HealthCheck>,
}

#[derive(Serialize, ToSchema)]
pub struct LiveResponse {
    pub contract_version: String,
    pub service_id: String,
    pub status: String,
    pub timestamp: String,
}

#[derive(Serialize, ToSchema)]
pub struct ReadyResponse {
    pub contract_version: String,
    pub service_id: String,
    pub status: String,
    pub timestamp: String,
    pub checks: HashMap<String, HealthCheck>,
}