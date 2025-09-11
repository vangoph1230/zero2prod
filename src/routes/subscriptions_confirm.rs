use actix_web::{HttpResponse, web};
use serde::de;

/// 在传入的请求中所预期的所有查询参数
/// 参数类型web:Query<Parameters> 仅在成功
/// 提取查询参数的情况下，调用处理函数
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(_parameters),
)]
pub async fn confirm(_parameters: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}