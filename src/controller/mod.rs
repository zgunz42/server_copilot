use std::sync::{Arc, RwLock};

use actix_web::{get, HttpResponse, Responder, web};

use crate::context;

#[get("/")]
pub async fn index(
    state: web::Data<Arc<RwLock<context::Context>>>,
) -> impl Responder {
    let ctxt = state.read().unwrap();
    let bot = ctxt.get_bot();

    let body = format!("Hello world! {:?}", bot.username()).to_string();

    HttpResponse::Ok().body(body)
}
