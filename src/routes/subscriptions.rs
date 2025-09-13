use actix_web::{web, HttpResponse};
use actix_web::ResponseError;
use chrono::Utc;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
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
    let new_subscriber = match form.0.try_into() {
        Ok(form) => form,
        Err(_) => return Ok(HttpResponse::BadRequest().finish()),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let subscriber_id = match insert_subscriber(&mut transaction, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let subscription_token = generate_subscription_token();
    // '?'操作符帮我们自动调用'Into' trait,这样无须显示的调用'map_err'方法
    store_token(&mut transaction, subscriber_id, &subscription_token).await?;
    if transaction.commit().await.is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }
    if send_confirmation_email(
        &email_client, 
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .is_err() {
        return Ok(HttpResponse::InternalServerError().finish());
    }

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


pub struct StoreTokenError(sqlx::Error);

/// StoreTokenError为了实现ResponseError trait 必要条件
impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 遍历错误传播链
        error_chain_fmt(self, f)
    }
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

/// StoreTokenError为使用error_chain_fmt()函数，才实现Error trait,
impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // 编译器将'&sqlx::Error'隐式转换为'&dyn Error'
        Some(&self.0)
    }
}

/// 为所有实现了std::error::Error trait的任何类型
/// 提供类似、统一的表示格式；
fn error_chain_fmt(
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

#[derive(Debug)]
struct SubscriberError {}

impl std::fmt::Display for SubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to create a new subscriber."
        )
    }
}

impl std::error::Error for SubscriberError {}

impl ResponseError for SubscriberError {}