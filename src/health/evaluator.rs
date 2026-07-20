// src/health/evaluator.rs

use std::collections::HashMap;

use crate::{
    config::Config,
    health::{
        models::HealthCheck,
        state::GatewayHealthState,
    },
};

pub struct GatewayHealthEvaluation {
    pub ready: bool,
    pub checks: HashMap<String, HealthCheck>,
}

fn operational() -> HealthCheck {
    HealthCheck {
        status: "operational".to_string(),
        message: None,
    }
}

fn degraded(message: &str) -> HealthCheck {
    HealthCheck {
        status: "degraded".to_string(),
        message: Some(message.to_string()),
    }
}

pub fn evaluate_gateway(
    config: &Config,
    state: &GatewayHealthState,
) -> GatewayHealthEvaluation {
    let routes_ready = !config.registered_routes().is_empty();
    let proxy_ready = state.proxy_initialized();

    let mut checks = HashMap::new();

    checks.insert(
        "application".to_string(),
        operational(),
    );

    checks.insert(
        "configuration".to_string(),
        operational(),
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
            degraded("Proxy client is not initialized")
        },
    );

    GatewayHealthEvaluation {
        ready: routes_ready && proxy_ready,
        checks,
    }
}