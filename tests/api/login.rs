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
    assert_is_redirect_to(&response, "/login");

    // ***跟随重定向***
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    // ***重新加载登录页面***
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
    // 原文中的代码通过了测试，这里失败
}