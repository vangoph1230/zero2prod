use zero2prod::configuration;
use zero2prod::startup;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use secrecy::ExposeSecret;




#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "debug".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = configuration::get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(
            &configuration.database.connection_string().expose_secret()
        )
        .expect("Failed to create Postgres connection pool.");

    tracing::debug!("socket-address: {}:{}", &configuration.application.host, &configuration.application.port);
    let address = format!("{}:{}",configuration.application.host, configuration.application.port);
    let listener = TcpListener::bind(address)?;
    let _ = startup::run(listener, connection_pool)?.await;
    Ok(())
}

