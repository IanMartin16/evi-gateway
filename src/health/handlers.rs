use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::OnceLock,
    time::Instant,
};
use utoipa::ToSchema;

use crate::config::Config;


static STARTED_AT: OnceLock<Instant> = OnceLock::new();


fn uptime_seconds() -> u64 {
    STARTED_AT
        .get_or_init(Instant::now)
        .elapsed()
        .as_secs()
}


fn current_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}


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
pub struct HealthResponse {
    pub contract_version: String,
    pub service: ServiceInfo,
    pub status: String,
    pub readiness: String,
    pub timestamp: String,
    pub uptime_seconds: u64,
    pub checks: BTreeMap<String, HealthCheck>,
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
    pub checks: BTreeMap<String, HealthCheck>,
}


struct GatewayEvaluation {
    ready: bool,
    checks: BTreeMap<String, HealthCheck>,
}


fn operational() -> HealthCheck {
    HealthCheck {
        status: "operational".to_string(),
        message: None,
    }
}


fn degraded(message: impl Into<String>) -> HealthCheck {
    HealthCheck {
        status: "degraded".to_string(),
        message: Some(message.into()),
    }
}


fn evaluate_gateway(config: &Config) -> GatewayEvaluation {
    let routes_registered = config.registered_routes().len();

    let routes_ready = routes_registered > 0;

    // Solo valida la configuración local.
    // No realiza ninguna llamada de red.
    let upstream_url_valid = reqwest::Url::parse(&config.mcpone_url).is_ok();

    let proxy_ready = routes_ready && upstream_url_valid;

    let mut checks = BTreeMap::new();

    checks.insert(
        "application".to_string(),
        operational(),
    );

    checks.insert(
        "configuration".to_string(),
        if upstream_url_valid {
            operational()
        } else {
            degraded("MCPONE_URL is not a valid URL")
        },
    );

    checks.insert(
        "routes".to_string(),
        if routes_ready {
            operational()
        } else {
            degraded("No gateway routes are registered")
        },
    );

    checks.insert(
        "proxy".to_string(),
        if proxy_ready {
            operational()
        } else {
            degraded("Gateway proxy is not ready")
        },
    );

    GatewayEvaluation {
        ready: routes_ready && upstream_url_valid && proxy_ready,
        checks,
    }
}


#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (
            status = 200,
            description = "Gateway health status",
            body = HealthResponse
        )
    ),
    tag = "evi-gateway"
)]
#[get("/health")]
pub async fn health_check(
    config: web::Data<Config>,
) -> impl Responder {
    let evaluation = evaluate_gateway(config.get_ref());

    HttpResponse::Ok().json(HealthResponse {
        contract_version: "health.v1".to_string(),

        service: ServiceInfo {
            id: config.app_id.clone(),
            name: config.app_name.clone(),
            version: config.version.clone(),
            environment: config.env.clone(),
            stack: config.stack.clone(),
        },

        status: if evaluation.ready {
            "operational".to_string()
        } else {
            "degraded".to_string()
        },

        readiness: if evaluation.ready {
            "ready".to_string()
        } else {
            "not_ready".to_string()
        },

        timestamp: current_timestamp(),
        uptime_seconds: uptime_seconds(),
        checks: evaluation.checks,
    })
}


#[utoipa::path(
    get,
    path = "/api/health/live",
    responses(
        (
            status = 200,
            description = "Gateway liveness status",
            body = LiveResponse
        )
    ),
    tag = "evi-gateway"
)]
#[get("/health/live")]
pub async fn health_live(
    config: web::Data<Config>,
) -> impl Responder {
    HttpResponse::Ok().json(LiveResponse {
        contract_version: "health.v1".to_string(),
        service_id: config.app_id.clone(),
        status: "alive".to_string(),
        timestamp: current_timestamp(),
    })
}


#[utoipa::path(
    get,
    path = "/api/health/ready",
    responses(
        (
            status = 200,
            description = "Gateway is ready",
            body = ReadyResponse
        ),
        (
            status = 503,
            description = "Gateway is not ready",
            body = ReadyResponse
        )
    ),
    tag = "evi-gateway"
)]
#[get("/health/ready")]
pub async fn health_ready(
    config: web::Data<Config>,
) -> impl Responder {
    let evaluation = evaluate_gateway(config.get_ref());

    let readiness_checks = evaluation
        .checks
        .into_iter()
        .filter(|(name, _)| name != "application")
        .collect();

    let payload = ReadyResponse {
        contract_version: "health.v1".to_string(),
        service_id: config.app_id.clone(),

        status: if evaluation.ready {
            "ready".to_string()
        } else {
            "not_ready".to_string()
        },

        timestamp: current_timestamp(),
        checks: readiness_checks,
    };

    if evaluation.ready {
        HttpResponse::Ok().json(payload)
    } else {
        HttpResponse::ServiceUnavailable().json(payload)
    }
}


pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg
        .service(health_check)
        .service(health_live)
        .service(health_ready);
}