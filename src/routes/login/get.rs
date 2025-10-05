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

    // 展示所有的消息层级，而不仅仅是错误
    for m in flash_message.iter() {
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