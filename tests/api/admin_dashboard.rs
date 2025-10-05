use crate::helper::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    let app = spawn_app().await;
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    // 第一部分，登录
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // 第二部分，跟随重定向
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // 第三部分，退出登录
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // 第四部分，跟随重定向
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // 第五部分，尝试加载管理面板
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}