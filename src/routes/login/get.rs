use actix_web::HttpResponse;
use actix_web::http::header::ContentType;

use crate::routes::Content;

pub async fn login_form() -> HttpResponse {
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("login.html"))
}