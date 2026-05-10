use std::env;

use crate::models::{ApiClient, RouteConfig};

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub app_name: String,
    pub version: String,
    pub mcpone_url: String,
    pub default_timeout_ms: u64,
    pub api_keys_raw: String,
    pub mcpone_orchestrate_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            app_name: "evi-gate".to_string(),
            version: "0.1.0".to_string(),
            mcpone_orchestrate_path: env::var("MCPONE_ORCHESTRATE_PATH")
                .unwrap_or_else(|_| "/orchestrate".to_string()),
            mcpone_url: env::var("MCPONE_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            default_timeout_ms: env::var("DEFAULT_TIMEOUT_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .unwrap_or(5000),
            api_keys_raw: env::var("EVIGATE_API_KEYS")
                .unwrap_or_else(|_| "nexus:nexus_dev_key:mcpone.execute,mcpone.read".to_string()),
        }
    }

    pub fn registered_routes(&self) -> Vec<RouteConfig> {
        vec![RouteConfig {
            service_name: "mcpone".to_string(),
            route: "mcpone.execute".to_string(),
            target_url: format!("{}{}", self.mcpone_url.trim_end_matches('/'), self.mcpone_orchestrate_path),
            required_scopes: vec!["mcpone.execute".to_string()],
            auth_required: true,
            timeout_ms: self.default_timeout_ms,
        }]
    }

    pub fn api_clients(&self) -> Vec<ApiClient> {
        parse_api_clients(&self.api_keys_raw)
    }
}

fn parse_api_clients(raw: &str) -> Vec<ApiClient> {
    raw.split(';')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(':').collect();

            if parts.len() != 3 {
                return None;
            }

            let client_id = parts[0].trim().to_string();
            let api_key = parts[1].trim().to_string();
            let scopes = parts[2]
                .split(',')
                .map(|scope| scope.trim().to_string())
                .filter(|scope| !scope.is_empty())
                .collect();

            Some(ApiClient {
                client_id,
                api_key,
                scopes,
            })
        })
        .collect()
}