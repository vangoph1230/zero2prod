use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

async  fn health_check(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().finish()
}

/// HttpServer调用run方法并await后，Server开始不断循环监听指定的地址，但永远不会自动关闭或完成
pub async fn run() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}