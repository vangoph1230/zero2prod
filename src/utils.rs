use actix_web::HttpResponse;
use actix_web::http::header::LOCATION;

/// 返回一个不透明的500， 同时保留错误
/// 的根本原因，以便记录
pub fn e500<T>(e: T) -> actix_web::Error
    where
        T: std::fmt::Debug + std::fmt::Display + 'static
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}