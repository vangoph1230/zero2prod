use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use actix_web::web;
use actix_web::error::InternalError;
use actix_web_flash_messages::FlashMessage;
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
    name="POST /login"
    skip(form, pool),
    fields(
        username=tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn login(
    form: web::Form<FormData>, 
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current()
        .record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, &pool).await {
        Ok(user_id) => {
            tracing::Span::current()
                .record("user_id", &tracing::field::display(&user_id));
            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/admin/dashboard"))
                .finish()
            )
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            FlashMessage::error(e.to_string()).send();
         
            // HTTP 303 See Other 重定向响应,将用户跳转到登录页面
            // HTTP 303 适用于 POST 后的重定向，确保后续请求使用 GET 方法
            // 浏览器收到 HTTP 303 响应,识别到 303 状态码和 Location 头部，
            // 会自动地、立即地向新的 URL发起一个新的 GET 请求
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login",))
                .finish();
            // 构建一个包含预定义响应的InternalError
            // InternalError实现了ResponseError
            Err(InternalError::from_response(e, response))
        }
    }
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