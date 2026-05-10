use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(crate::handlers::domain::configure);
    cfg.configure(crate::handlers::gateway::configure);
}
