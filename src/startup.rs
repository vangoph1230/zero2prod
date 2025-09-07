use crate::routes::{health_check, subscribe};
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;


pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);

    // TracingLogger一个专门为 actix-web 框架设计的中间件,基于tracing而非log实现,
    // 能自带request_id等跨度信息，使用其代替 actix-web::Logger,
    let server = HttpServer::new(move || {
            App::new()
                // 替换"Logger::default()"
                .wrap(TracingLogger::default())
                .route("/health_check", web::get().to(health_check))
                .route("/subscriptions", web::post().to(subscribe))
                .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}