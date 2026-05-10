use actix_cors::Cors;
use actix_web::{http::header, middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod config;
mod errors;
mod handlers;
mod models;
mod routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
        handlers::gateway::get_routes,
        handlers::gateway::proxy
    ),
    components(schemas(
        handlers::health::HealthResponse,
        models::RouteConfig,
        models::ApiClient,
        models::ProxyRequest,
        models::ProxyResponse,
        models::RoutesResponse
    )),
    tags(
        (
            name = "evi-gate",
            description = "Secure API Gateway for the IO Module. Provides route registry, API key validation, scope validation, controlled proxying, request tracing, and standardized gateway responses."
        )
    )
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let app_config = config::Config::from_env();
    let bind_addr = format!("{}:{}", app_config.host, app_config.port);

    log::info!("Starting evi-gate on {}", bind_addr);
    log::info!("Swagger UI available at http://{}/swagger-ui/", bind_addr);
    log::info!("OpenAPI JSON available at http://{}/api-docs/openapi.json", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::HeaderName::from_static("x-api-key"),
            ])
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(app_config.clone()))
            .wrap(Logger::default())
            .wrap(cors)
            .service(
                web::scope("/api")
                    .configure(handlers::health::configure)
                    .configure(routes::api::configure),
            )
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}