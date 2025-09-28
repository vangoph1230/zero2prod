use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password",
    });
    // reqwest::Client看到303状态码，会自动调用GET /login,
    // 即Location请求头中指定的路径；
    // ClientBuilder的redirect::Policy自定义Client行为
    // ***尝试登录***
    let response = app.post_login(&login_body).await;
    assert_eq!(response.status().as_u16(), 303);
    assert_is_redirect_to(&response, "/login");

    // ***跟随重定向***
    for header in response.headers().get_all("set-cookie") {
        if let Ok(cookie_value) = header.to_str() {
            dbg!(cookie_value);
        }
    }
    let flash_cookie = response.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication failed");

    // ***重新加载登录页面***
    let response = app.get_login_html_html().await;
    for header in response.headers().get_all("set-cookie") {
        if let Ok(cookie_value) = header.to_str() {
            dbg!(cookie_value);
        }
    }

    // ***重新加载登录页面***
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

}