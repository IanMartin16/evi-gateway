pub mod domain;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteConfig {
    pub service_name: String,
    pub route: String,
    pub method: String,
    pub target_url: String,
    pub required_scopes: Vec<String>,
    pub auth_required: bool,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiClient {
    pub client_id: String,
    pub api_key: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProxyRequest {
    pub route: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProxyResponse {
    pub request_id: String,
    pub route: String,
    pub status: u16,
    pub data: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoutesResponse {
    pub routes: Vec<RouteConfig>,
}