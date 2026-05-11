use std::time::Duration;

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

use crate::config::Config;
use crate::errors::AppError;
use crate::models::{ApiClient, ProxyRequest, ProxyResponse, RouteConfig, RoutesResponse};

#[utoipa::path(
    get,
    path = "/api/routes",
    responses(
        (status = 200, description = "Registered gateway routes", body = RoutesResponse)
    ),
    tag = "evi-gate"
)]
#[get("/routes")]
pub async fn get_routes(config: web::Data<Config>) -> impl Responder {
    HttpResponse::Ok().json(RoutesResponse {
        routes: config.registered_routes(),
    })
}

#[utoipa::path(
    post,
    path = "/api/proxy",
    request_body = ProxyRequest,
    responses(
        (status = 200, description = "Proxy response", body = ProxyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Route not found"),
        (status = 502, description = "Upstream error"),
        (status = 504, description = "Upstream timeout")
    ),
    tag = "evi-gate"
)]
#[post("/proxy")]
pub async fn proxy(
    req: HttpRequest,
    body: web::Json<ProxyRequest>,
    config: web::Data<Config>,
) -> HttpResponse {
    let request_id = get_or_create_request_id(&req);
    let mcpone_request_id = extract_payload_request_id(&body.payload);
    let upstream_payload = ensure_payload_request_id(&body.payload, &request_id);

    let routes = config.registered_routes();
    let clients = config.api_clients();
    let start = std::time::Instant::now();

    let route = match find_route(&routes, &body.route) {
        Some(route) => route,
        None => {
            return AppError::RouteNotFound(format!("Route not registered: {}", body.route))
                .to_response(Some(request_id));
        }
    };

    if route.auth_required {
        let api_key = match extract_api_key(&req) {
            Some(key) => key,
            None => {
                return AppError::Unauthorized("Missing X-API-Key header".to_string())
                    .to_response(Some(request_id));
            }
        };

        let client = match validate_api_key(&clients, &api_key) {
            Some(client) => client,
            None => {
                return AppError::Unauthorized("Invalid API key".to_string())
                    .to_response(Some(request_id));
            }
        };

        if !has_required_scopes(client, &route.required_scopes) {
            return AppError::Forbidden("Client does not have required scope".to_string())
                .to_response(Some(request_id));
        }
    }

    let http_client = Client::builder()
        .timeout(Duration::from_millis(route.timeout_ms))
        .build();

    let http_client = match http_client {
        Ok(client) => client,
        Err(_) => {
            return AppError::Internal("Failed to create HTTP client".to_string())
                .to_response(Some(request_id));
        }
    };

    let request_builder = match route.method.as_str() {
        "GET" => http_client.get(&route.target_url),
        "POST" => http_client.post(&route.target_url).json(&upstream_payload),
        _ => {
            return AppError::BadRequest(format!("Unsopported method: {}", route.method))
                .to_response(Some(request_id));
        }
    };

    let upstream_result = request_builder
        .header("X-Request-ID", request_id.clone())
        .send()
        .await;

    match upstream_result {
        Ok(response) => {
            let status = response.status().as_u16();

            let data = response
                .json::<serde_json::Value>()
                .await
                .unwrap_or_else(|_| json!({ "message": "Upstream returned non-JSON response" }));

            let latency_ms = start.elapsed().as_millis();

            log::info!(
                "proxy_success gateway_request_id={} upstream_request_id={} route={} service={} status={} latency_ms={}",
                request_id,
                mcpone_request_id.clone().unwrap_or_else(|| request_id.clone()),
                route.route,
                route.service_name,
                status,
                latency_ms
            );   

            HttpResponse::Ok()
            .insert_header(("X-Request-ID", request_id.clone()))
            .json(ProxyResponse {
                request_id,
                route: route.route.clone(),
                status,
                data,
            })
        }
        Err(err) if err.is_timeout() => {
            AppError::UpstreamTimeout(format!("Timeout calling {}", route.service_name))
                .to_response(Some(request_id))
        }
        Err(_) => {
            AppError::UpstreamError(format!("Failed calling {}", route.service_name))
                .to_response(Some(request_id))
        }
    }
}

fn find_route<'a>(routes: &'a [RouteConfig], route_name: &str) -> Option<&'a RouteConfig> {
    routes.iter().find(|route| route.route == route_name)
}

fn extract_api_key(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
}

fn validate_api_key<'a>(clients: &'a [ApiClient], api_key: &str) -> Option<&'a ApiClient> {
    clients.iter().find(|client| client.api_key == api_key)
}

fn has_required_scopes(client: &ApiClient, required_scopes: &[String]) -> bool {
    required_scopes
        .iter()
        .all(|required| client.scopes.contains(required))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_routes);
    cfg.service(proxy);
}

fn get_or_create_request_id(req: &HttpRequest) -> String {
    req.headers()
        .get("X-Request-ID")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

fn extract_payload_request_id(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("request_id")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn ensure_payload_request_id(
    payload: &serde_json::Value,
    request_id: &str,
) -> serde_json::Value {
    let mut enriched_payload = payload.clone();

    if enriched_payload.get("request_id").is_none() {
        if let Some(obj) = enriched_payload.as_object_mut() {
            obj.insert(
                "request_id".to_string(),
                serde_json::Value::String(request_id.to_string()),
            );
        }
    }

    enriched_payload
}