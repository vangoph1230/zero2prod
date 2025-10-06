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
use crate::session_state::TypedSession;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    name="POST /login"
    skip(form, pool, session),
    fields(
        username=tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn login(
    form: web::Form<FormData>, 
    pool: web::Data<PgPool>,
    session: TypedSession,
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
            // 用户登录时轮换会话令牌
            session.renew();
            session.insert_user_id(user_id)
                .map_err(|e| login_redirect(
                    LoginError::UnexpectedError(e.into())
                ))?;
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
            Err(login_redirect(e))
        }
    }
}

fn login_redirect(e: LoginError) -> InternalError<LoginError> {
    // 如果出了问题，用户将被重定向到/login页面，并给出适当的错误信息
    FlashMessage::error(e.to_string()).send();
    let response = HttpResponse::SeeOther()
        .insert_header((LOCATION, "/login"))
        .finish();
    
    // InternalError实现了ResponseError
    // 返回一个预先定制好的响应格式或内容
    // 不会向用户展示一个默认的错误页面,而是会直接返回我们精心准备的那个重定向响应
    // 错误会被保留下来，对于记录服务器端日志非常有用
    InternalError::from_response(e, response)
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