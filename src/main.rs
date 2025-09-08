//! src/lib.rs
use secrecy::ExposeSecret;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::net::TcpListener;

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
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    // connect_lazy不再是异步，因为实际上并没有尝试建立连接，
    // 它只会在首次使用连接池时尝试建立连接
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        // connect_lazy替换为 connect_lazy_with
        .connect_lazy_with(
            configuration.database.with_db()
        );
    let address = format!(
        "{}:{}", 
        configuration.application.host,
        configuration.application.port,
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await?;
    Ok(())
}