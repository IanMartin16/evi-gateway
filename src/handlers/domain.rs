use actix_web::{get, post, web, HttpResponse, Responder};
use uuid::Uuid;
use chrono::Utc;
use crate::models::domain::{Item, CreateItemRequest};

#[get("/items")]
pub async fn list_items() -> impl Responder {
    let items: Vec<Item> = vec![
        Item {
            id: Uuid::new_v4(),
            name: "Sample Item".to_string(),
            created_at: Utc::now(),
        },
    ];
    HttpResponse::Ok().json(items)
}

#[post("/items")]
pub async fn create_item(body: web::Json<CreateItemRequest>) -> impl Responder {
    let item = Item {
        id: Uuid::new_v4(),
        name: body.name.clone(),
        created_at: Utc::now(),
    };
    HttpResponse::Created().json(item)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_items)
       .service(create_item);
}

