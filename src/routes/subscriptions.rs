use actix_web::{web, HttpResponse};
use actix_web::ResponseError;
use anyhow::Context;
use chrono::Utc;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use unicode_segmentation::UnicodeSegmentation;
use crate::{domain::{NewSubscriber, SubscriberEmail, SubscriberName}, email_client::EmailClient};
use crate::startup::ApplicationBaseUrl;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// 讲一个跨度绑定到函数上
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscriberError> {
    let new_subscriber = form.0
        .try_into()
        .map_err(SubscriberError::ValidationError)?;
    let mut transaction = pool.begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool.")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;

    let subscription_token = generate_subscription_token();
    // '?'操作符帮我们自动调用'Into' trait,这样无须显示的调用'map_err'方法
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    transaction.commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    send_confirmation_email(
        &email_client, 
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction),
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

pub fn is_valid_name(s: &str) -> bool {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_too_long = s.graphemes(true).count() > 256;
        let forbiden_characters = ['/', '(', ')', ',', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbiden_characters.contains(&g));

    !(is_empty_or_whitespace || is_too_long || contains_forbidden_characters)
}

/// 正确解析出表单中的name、email信息
pub fn parse_subscriber(form: FormData) -> Result<NewSubscriber, String> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber {name, email})
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(Self { email, name})
    }
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url),
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}", 
        base_url,
        subscription_token,
    );
    let plain_body = &format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_bdoy = &format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(
        new_subscriber.email, 
        "Welcome!", 
        &html_bdoy, 
    &plain_body,
        )
        .await
}

/// 生成随机的长度为25个字符且大小写敏感的订阅令牌
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    )
    .execute(transaction)
    .await
    .map_err(|e| {
        tracing::error!("Failedto execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}




/// StoreTokenError为了实现ResponseError trait 必要条件
impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}


pub struct StoreTokenError(sqlx::Error);

/// StoreTokenError为了实现ResponseError trait 必要条件
impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 遍历错误传播链
        error_chain_fmt(self, f)
    }
}

/// StoreTokenError为使用error_chain_fmt()函数，才实现Error trait,
impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // 编译器将'&sqlx::Error'隐式转换为'&dyn Error'
        Some(&self.0)
    }
}


/// 为所有实现了std::error::Error trait的任何类型
/// 提供类似、统一的表示格式；
pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(case) = current {
        writeln!(f, "Caused by:\n\t{}", case)?;
        // 遍历错误传播链，直到打印出底层错误
        current = case.source();
    }
    Ok(())
}

/*
/// SubscriberError第一版数据结构
#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Failed to acquire a Postgres connectiuon from the pool")]
    PoolError(#[source] sqlx::Error),
    #[error("Failed to insert new subscriber in the database.")]
    InsertSubscriberError(#[source] sqlx::Error),
    #[error("Failed to store the confirmation token for a new subscriber.")]
    StoreTokenError(#[from] StoreTokenError),
    #[error("Failed to commit SQL transaction to store a new subscriber.")]
    TransactionCommitError(#[source] sqlx::Error),
    #[error("Failed to send a confirmation email.")]
    SendEmailError(#[from] reqwest::Error),
}

/// SubscriberError第二版数据结构
#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    #[error("transparent")]
    UnexpectedError(#[from] Box<dyn std::error::Error>),
}

/// SubscriberError第三版数据结构
#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{1}")]
    UnexpectedError(#[source] Box<dyn std::error::Error>, String),
}

*/

/// SubscriberError第四版数据结构
/// anyhow::Error用于包装一个动态的错误类型,会自动为错误类型
/// 提供了额外的上下文，实现了该字段原来的功能
#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    // from 表示可以将anyhow::Error类型自动转换为SubscriberError
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriberError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            SubscriberError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscriberError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}



/* 
#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Failed to acquire a Postgres connectiuon from the pool")]
    PoolError(#[source] sqlx::Error),
    #[error("Failed to insert new subscriber in the database.")]
    InsertSubscriberError(#[source] sqlx::Error),
    #[error("Failed to store the confirmation token for a new subscriber.")]
    StoreTokenError(#[from] StoreTokenError),
    #[error("Failed to commit SQL transaction to store a new subscriber.")]
    TransactionCommitError(#[source] sqlx::Error),
    #[error("Failed to send a confirmation email.")]
    SendEmailError(#[from] reqwest::Error),
}

impl std::error::Error for SubscriberError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubscriberError::ValidationError(_) => None,
            SubscriberError::StoreTokenError(e) => Some(e),
            SubscriberError::SendEmailError(e) => Some(e),
            SubscriberError::PoolError(e) => Some(e),
            SubscriberError::InsertSubscriberError(e) => Some(e),
            SubscriberError::TransactionCommitError(e) => Some(e),
        }
    }
}

impl std::fmt::Display for SubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriberError::ValidationError(e) => write!(f, "{}", e),
            SubscriberError::StoreTokenError(_) => write!(
                f,
                "Failed to store the confirmation token for a new subscrier."
            ),
            SubscriberError::SendEmailError(_) => {
                write!(f, "Failed to send a confirmation email.")
            },
            SubscriberError::PoolError(_) => {
                write!(f, "Failed to acquire a Postgres connection from the pool")
            },
            SubscriberError::InsertSubscriberError(_) => {
                write!(f, "Failed to insert new subscriber in the database.")
            },
            SubscriberError::TransactionCommitError(_) => {
                write!(
                    f,
                    "Failed to cimmit SQL transaction to store a new subscriber."
                )
            }
        }
    }
}

// StoreTokenError(StoreTokenError)变体实现From trait
impl From<StoreTokenError> for SubscriberError {
    fn from(e: StoreTokenError) -> Self {
        Self::StoreTokenError(e)
    }
}

// SendEmailError(reqwest::Error)变体实现From trait
impl From<reqwest::Error> for SubscriberError {
    fn from(e: reqwest::Error) -> Self {
        Self::SendEmailError(e)
    }
}

// ValidationError(String)变体实现From trait
impl From<String> for SubscriberError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

*/




