use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{JsonStorageLayer, BunyanFormattingLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// tracing crate 提供了核心APi与抽象，其中提供了 Subsciber trait；
///     - tracing中的Subscriber trait 与log的Log trait 类似；
/// tracing-subscriber crate，核心实现和基础设施，Registry类实现了Subscriber trait，即订阅器，同时提供了Layer trait;
///     - Layer trait使得跨度数据能够以流水线的形式被处理：不用打造一
///     - 个全面的订阅器，只需要将多个层次的小功能拼在一起，组成一条流水线；
///     - 这种层次布局的基础是Registry类;
///     - Registry 负责记录跨度的元数据、关系、激活、关闭；下游的层次可以在Registry的基础上完成自己的功能；
/// tracing-bunyan-formatter crate，一个专门的Layer,即特定格式层，实现了Layer trait,定义了“如何格式化和输出”；

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // 层的实例，即某种小功能
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| 
            EnvFilter::new("info")
        );
    // 层的实例，即某种小功能
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(), 
        std::io::stdout,
    );
 
    // 初始化 订阅器实例，并向其添加 层次的小功能
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    // 设置为整个程序生命周期内的全局默认订阅者，但库中慎用，防止第二次初始化
    set_global_default(subscriber).expect("Failed to set subscriber");


    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
