use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use secrecy::Secret;
use sqlx::PgPool;
use crate::authentication::{AuthError, validate_credentials, Credentials};
use crate::{email_client::EmailClient, routes::error_chain_fmt};
use crate::domain::SubscriberEmail;
use actix_web::http::{header, StatusCode};
use actix_web::http::header::{HeaderMap, HeaderValue};

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn status_code(&self) -> StatusCode {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            PublishError::AuthError(_) => StatusCode::UNAUTHORIZED,
        }
    }

    /// 'status_code'被默认的'error_response'实现所调用
    /// 我们提供了一个定制的'error_response'实现
    /// 因此不再需要维护一个'status_code'实现
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#)
                    .unwrap();
                response.headers_mut()
                    // actix_web::http::header提供了一组常量
                    // 用于表示一些众所周知的/标准HTTP头的名称
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}

#[tracing::instrument(
    name = "Get confirmed subscribers.",
    skip(pool),
)]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email FROM subscriptions WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email}),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(confirmed_subscribers)
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(
        // 字段先设为空,即使后续代码没有给这些字段赋值，
        // 它们在 span 中也是存在的（值为空）
        username=tracing::field::Empty,  
        user_id=tracing::field::Empty,    
    )
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    // credentials 凭证
    let credentials = basic_authentication(request.headers())
        .map_err(PublishError::AuthError)?;
    // 在运行时给之前在 fields() 中声明的空字段赋值
    // tracing::Span::current() 获取由宏自动创建的当前 span
    // .record("field_name", value) 将值填入指定字段
    tracing::Span::current().record(
        "username", 
        &tracing::field::display(&credentials.username)
    );
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;
    // tracing::field::display() 是一个函数，它接受任何实现了 
    // std::fmt::Display trait 的值，并将其包装成一个
    // 实现了 tracing::field::Value trait 的特殊类型
    // record()的第二个参数要求是 &dyn Value
    tracing::Span::current().record(
        "user_id", 
        &tracing::field::display(&user_id),
    );
    let subscribers = get_confirmed_subscribers(&pool).await?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client.send_email(
                    &subscriber.email, 
                    &body.title, 
                    &body.content.html, 
                    &body.content.text,
                )
                .await
                .with_context(|| {
                    format!(
                        "Failed to send newsletter issue to {}",
                        subscriber.email,
                    )
                })?;
            }
            Err(error) => {
                tracing::warn!(
                    // 将错误传播链作为一个结构化的自命名的字段记录在日志中
                    // 字段命名：error.cause_chain
                    // 调试格式：?error ,等价于std::fmt::Debug::fmt(&error, formatter)，其中error是变量名
                    error.cause_chain = ?error,
                    // 使用'\'将长字符串字面值分成两行，而不创建'\n'字符
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid",
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

/// 一个用于处理 HTTP Basic 认证的函数,提取请求头中包含的用户名和密码
/// Anthorization: Basic<编码后的凭据>
/// - <编码后的凭据>是 {username}:{password}的base64编码格式
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decode_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decode_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    // 使用冒号":"作为分隔符将其分为两个部分
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials.next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials.next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials { 
        username, 
        password: Secret::new(password),
    })
}