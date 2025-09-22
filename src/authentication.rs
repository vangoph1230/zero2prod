use anyhow::Context;
use secrecy::Secret;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use crate::telemetry::spawn_blocking_with_tracing;
use argon2::{Argon2, PasswordHash, PasswordVerifier};

/// 使用枚举，是因为希望能够根据错误类型做出不同的响应
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

/// 验证 凭据的 有效性
/// - 1、先从数据库中查询存储的HPC格式的哈希值
/// - 2、使用PHC格式的哈希值初始化PasswrodHash(PHC的实现)
/// - 3、使用PHC实例验证password
#[tracing::instrument(
    name = "Validate credentials",
    skip(credentials, pool),
)]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$gZiv/M1gPc22ElAH/Jh1Hw$\
        CwOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
        .to_string(),
    );

    if let Some((stored_user_id, stored_password_hash)) = get_stored_credentials(
        &credentials.username,
        &pool,
    )
    .await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(
            expected_password_hash, 
            credentials.password,
        )
    })
    .await
    .context("Failed to spawn Blocking task.")??;

    // 只有在存储中找到凭据，才会将其设置为'Some'
    // 因此，即使默认密码与所提供的密码匹配(以某种方式)
    // 也永远不会对不存在的用户进行身份验证
    user_id.ok_or_else(|| 
        anyhow::anyhow!("Unkonw username")
    )
    .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate),
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash = PasswordHash::new(
        expected_password_hash.expose_secret()
    )
    .context("Failed to parse hash in PHC string format.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(), 
            &expected_password_hash
        )
        .context("Invalid password.")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name = "Get stored credentials",
    skip(username, pool),
)]
async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}