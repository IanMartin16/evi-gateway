use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;
use std::time::Instant;
use utoipa::ToSchema;

use crate::config::Config;

#[derive(Serialize, ToSchema)]
pub struct UpstreamHealth {
    pub service_name: String,
    pub status: String,
    pub target: String,
    pub latency_ms: Option<u128>,
}

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
    pub status: String,
    pub service: String,
    pub version: String,
    pub env: String,
    pub routes_registered: usize,
    pub upstreams: Vec<UpstreamHealth>,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service health status", body = HealthResponse)
    ),
    tag = "evi-gate"
)]
#[get("/health")]
pub async fn health_check(config: web::Data<Config>) -> impl Responder {
    let routes_registered = config.registered_routes().len();

    let mcpone_health_url = format!(
        "{}{}",
        config.mcpone_url.trim_end_matches('/'),
        config.mcpone_health_path
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(config.default_timeout_ms))
        .build();

    let upstream_status = match client {
        Ok(client) => {
            let start = Instant::now();

            match client.get(&mcpone_health_url).send().await {
                Ok(response) if response.status().is_success() => UpstreamHealth {
                    service_name: "mcpone".to_string(),
                    status: "operational".to_string(),
                    target: mcpone_health_url,
                    latency_ms: Some(start.elapsed().as_millis()),
                },
                Ok(response) => {
                    log::warn!(
                        "upstream_health_degraded service=mcpone target={} status={}",
                        mcpone_health_url,
                        response.status()
                    );

                    UpstreamHealth {
                        service_name: "mcpone".to_string(),
                        status: "degraded".to_string(),
                        target: mcpone_health_url,
                        latency_ms: Some(start.elapsed().as_millis()),
                    }
                }
                Err(err) => {
                    log::warn!(
                        "upstream_health_check_failed service=mcpone target={} error={}",
                        mcpone_health_url,
                        err
                    );

                    UpstreamHealth {
                        service_name: "mcpone".to_string(),
                        status: "degraded".to_string(),
                        target: mcpone_health_url,
                        latency_ms: Some(start.elapsed().as_millis()),
                    }
                }
            }
        }
        Err(err) => {
            log::error!("failed_to_create_http_client error={}", err);

            UpstreamHealth {
                service_name: "mcpone".to_string(),
                status: "degraded".to_string(),
                target: mcpone_health_url,
                latency_ms: None,
            }
        }
    };

    let overall_status = if upstream_status.status == "operational" {
        "operational"
    } else {
        "degraded"
    };

    HttpResponse::Ok().json(HealthResponse {
        ok: overall_status == "operational",
        status: overall_status.to_string(),
        service: config.app_name.clone(),
        version: config.version.clone(),
        env: config.env.clone(),
        routes_registered,
        upstreams: vec![upstream_status],
    })
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(health_check);
}