use crate::email_client::EmailClient;
use crate::routes::{admin_dashboard, health_check, home, login, login_form, publish_newsletter, subscribe};
use crate::configuration::Settings;
use crate::configuration::DatabaseSettings;
use crate::routes::confirm;
use crate::routes::{change_password, change_password_form};
use crate::routes::log_out;
use crate::authentication::reject_anonymous_users;
use actix_web_lab::middleware::from_fn;
use actix_web::cookie::Key;
use actix_web::web::Data;
use actix_web_flash_messages::storage::CookieMessageStore;
use sqlx::postgres::PgPoolOptions;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::FlashMessagesFramework;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;
use secrecy::Secret;
use secrecy::ExposeSecret;
use actix_session::SessionMiddleware;
use actix_session::storage::RedisSessionStore;

#[derive(Clone)]
pub struct HmacSecret(pub Secret<String>);
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {

    /// 根据配置信息初始化/配置应用程序
    /// 现在是异步的！返回anyhow::Error而不是std::io::Error
    /// anyhow::Error通用错误类型,错误类型擦除：可以包装任何实现了 std::error::Error trait 的错误类型
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
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
        let server = run(
            listener, 
            connection_pool, 
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri
        ).await?;

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

pub struct ApplicationBaseUrl(pub String);

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: Secret<String>,
    redis_uri: Secret<String>,
) -> Result<Server, anyhow::Error> {
    let db_pool = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));

    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());
    let message_store = CookieMessageStore::builder(
        secret_key.clone()
    ).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    // TracingLogger一个专门为 actix-web 框架设计的中间件,基于tracing而非log实现,
    // 能自带request_id等跨度信息，使用其代替 actix-web::Logger,
    let server = HttpServer::new(move || {
            App::new()
                .wrap(message_framework.clone())
                .wrap(SessionMiddleware::new(
                    redis_store.clone(), 
                    secret_key.clone()
                ))
                // 替换"Logger::default()"
                .wrap(TracingLogger::default())
                .route("/", web::get().to(home))
                .route("/login", web::get().to(login_form))
                .route("/login", web::post().to(login))
                .route("/health_check", web::get().to(health_check))
                .route("/newsletters", web::post().to(publish_newsletter))
                .route("/subscriptions", web::post().to(subscribe))
                .route("/subscriptions/confirm", web::get().to(confirm))
                .service(
                    web::scope("/admin")
                                .wrap(from_fn(reject_anonymous_users))
                                .route("/dashboard", web::get().to(admin_dashboard))
                                .route("/password", web::get().to(change_password_form))
                                .route("/password", web::post().to(change_password))
                                .route("/logout", web::post().to(log_out))
                )
                .app_data(db_pool.clone())
                .app_data(email_client.clone())
                .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}