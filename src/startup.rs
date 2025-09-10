use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};
use crate::configuration::Settings;
use crate::configuration::DatabaseSettings;
use sqlx::postgres::PgPoolOptions;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;


pub struct Application {
    port: u16,
    server: Server,
}

impl Application {

    /// 根据配置信息初始化/配置应用程序
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");

        let timeout = configuration.email_client.timeout();
        let base_url = configuration.email_client.base_url.clone();
        let auth_token = configuration.email_client.authorization_token.clone();

        let email_client = EmailClient::new(
            base_url, 
            sender_email, 
            auth_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host,
            configuration.application.port,
        );
        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;

        Ok(Self { port, server})
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    // 运行应用程序
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

}

pub fn get_connection_pool(
    configuration: &DatabaseSettings
) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);

    // TracingLogger一个专门为 actix-web 框架设计的中间件,基于tracing而非log实现,
    // 能自带request_id等跨度信息，使用其代替 actix-web::Logger,
    let server = HttpServer::new(move || {
            App::new()
                // 替换"Logger::default()"
                .wrap(TracingLogger::default())
                .route("/health_check", web::get().to(health_check))
                .route("/subscriptions", web::post().to(subscribe))
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}