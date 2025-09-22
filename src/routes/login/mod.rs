mod get;
use actix_web::{HttpResponse, HttpServer};
pub use get::login_form;

pub async fn login() -> HttpResponse {
    HttpResponse::Ok().finish()
}