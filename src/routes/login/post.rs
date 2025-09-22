use actix_web::http::header::ContentType;
use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use actix_web::web;
use actix_web::ResponseError;
use actix_web::http::StatusCode;
use secrecy::Secret;
use sqlx::PgPool;

use crate::authentication::validate_credentials;
use crate::authentication::Credentials;
use crate::authentication::AuthError;
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(form, pool),
    fields(
        username=tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn login(form: web::Form<FormData>, pool: web::Data<PgPool>) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));
    Ok(HttpResponse::SeeOther()
            // 向 HTTP 响应中添加一个名为 Location 的头部，其值为 "/"
            .insert_header((LOCATION, "/"))
            // 结束响应体的构建，并生成最终的 HttpResponse 对象
            .finish()
    )   
}


#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, "/login"))
            .finish()
    }
}

/*
fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .content_type(ContentType::html())
            .body(format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html;charset=utf-8">
                    <title>Login</title>
                </head>
                    <body>
                        <p><i>{}</i></p>
                        <form action="/login" method="post">
                            <label>Username
                                <input
                                    type="text"
                                    placeholder="Enter Username"
                                    name="username"
                                >
                            </label>
                            <label>Password
                                <input
                                    type="password"
                                    placeholder="Enter password"
                                    name="password"
                                >
                            </label>
                            <button type="submit">Login</button>
                        </form>
                    </body>
                </html>
                "#,
                self
            ))
    }
}
*/