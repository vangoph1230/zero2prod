use reqwest::Response;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::get_connection_pool;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use once_cell::sync::Lazy;
use sha3::Digest;
use argon2::password_hash::SaltString;
use argon2::{Argon2, Algorithm, Params, PasswordHasher, Version};



//使用'once_cell'确保'tracing'最多只被初始化一次
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber); 
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {

    pub async fn get_login_html_html(&self) -> Response {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .unwrap()
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response 
        where
            Body:serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// 解析出 确认邮件中的 链接
    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(
            &email_request.body
        ).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { 
            html, 
            plain_text,
        }
    }

 
    pub async fn test_user(&self) -> (String, String) {
        // LIMIT 1 确保只返回最多一条记录
        // fetch_one() 方法要求查询必须返回恰好一条记录
        // 如果没有 LIMIT 1，且表中有多条记录，.fetch_one() 会失败并返回错误
        let row = sqlx::query!(
            r#"
            SELECT username, password_hash FROM users LIMIT 1
            "#,
        )
        .fetch_one(&self.db_pool)
        .await
        .expect("Failed to create test users.");
        (row.username, row.password_hash)
    }
}

impl TestUser {

    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn argon2_store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
   
        let password_hash = Argon2::new(
            Algorithm::Argon2d,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        
        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }

   
    async fn sha3_store(&self, pool: &PgPool) {
        let password_hash = sha3::Sha3_256::digest(
            self.password.as_bytes()
        );
        let password_hash = format!("{:x}", password_hash);
        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

/// 服务器的端口由Os随机分配,初始化应用配置，初始化数据库配置，启动服务
pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");

    let application_port = application.port();
    let _ = tokio::spawn(application.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        // 为客户端启用持久化 Cookie 存储
        // 响应中接收到的 Cookie 将被保存，并包含在后续的附加请求中
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        address: format!("http://127.0.0.1:{}", application_port),
        port: application_port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client: client,
    };
    test_app.test_user.argon2_store(&test_app.db_pool).await;
    test_app
}

/// 使用Uuid创建随机的username和password存到users表
async fn add_test_user(pool: &PgPool) {
    sqlx::query!(
        r#"
        INSERT INTO users (user_id, username, password_hash)
        VALUES($1, $2, $3)
        "#,
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    )
    .execute(pool)
    .await
    .expect("Failed to create test users.");
}

/// 在与关系型数据库交互的测试中，为每个集成测试都启动一个全新的逻辑数据库，确保测试隔离：
/// - 第一：创建 缺少数据库名 的数据库连接(PgConnection)，
/// - 第二：PgConnection根据Uuid::new_v4()的随机值 创建一个唯一新名字的数据库连接（完整的数据库连接字符串），
/// - 第三：根据PgConnection创建数据库连接池（PgPool),
/// - 第四：在连接池上运行数据库迁移。
async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("LOCATION").unwrap(), location);
}