use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use crate::startup::HmacSecret;


#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {

    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!(
            "error={}",
            urlencoding::Encoded::new(&self.error),
        );
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(
            secret.0.expose_secret().as_bytes()
        ).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error)
    }
}

// 不再需要访问原始的请求了
pub async fn login_form(flash_message: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    // 无论检索所传入的闪现消息，还是确保在读取后将其清除
    // actix-web-flash-message会处理这些事，再调用请求处理函数之前，
    // 还会在后台验证Cookie签名的有效性
    for m in flash_message.iter().filter(|m| m.level() == Level::Error) {
        writeln!(error_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
                <html lang="en">
                    <head>
                        <meta http-equiv="content-type" content="text/html; charset=utf-8">
                        <title>Login</title>
                    </head>
                    <body>
                        {error_html}
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
                                    type="Password"
                                    placeholder="Enter Password"
                                    name="password"
                                >
                            </label>
                            <button type="submit">Login</button>
                        </form>
                    </body>
                </html>
            "#,
        ))
}