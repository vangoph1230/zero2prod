use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::configuration::{DatabaseSettings, get_configuration};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::startup::{build, get_connection_pool};
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use std::net::TcpListener;
use once_cell::sync::Lazy;



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

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// 服务器的端口由Os随机分配
pub async fn spawn_app() -> TestApp {
    // 只在第一次使用'TRACING'时调用initialize，其他时候都会直接跳过
    // 当第一次调用initialize时将执行'TRACING'中的代码
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Faild to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    let server = run(listener, connection_pool.clone(), email_client)
        .expect("Failed to bind Address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
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