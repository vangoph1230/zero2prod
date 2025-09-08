//! src/configuration.rs
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::PgConnectOptions;

#[derive(serde::Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. use either 'local' or 'production'.",
                other
            )),
        }
    }
}

impl DatabaseSettings {

    /// 返回 缺少数据库名 的数据库连接的配置
    pub fn without_db(&self) -> PgConnectOptions {
       PgConnectOptions::new()
        .host(&self.host)
        .username(&self.username)
        .password(&self.password.expose_secret())
        .port(self.port)
    }

    /// 构建完整的Postgres数据库连接的配置
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db().database(&self.database_name)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to detemine the current directory");
    let configuration_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let envrionment_filename = format!("{}.yaml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(configuration_directory.join("base.yaml")))
        .add_source(config::File::from(configuration_directory.join(&envrionment_filename)))
        .add_source(config::Environment::with_prefix("APP")
            .prefix_separator("_")
            .separator("__")
        )
        .build()?;
    settings.try_deserialize::<Settings>()
}
