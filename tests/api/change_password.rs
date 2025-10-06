use crate::helper::{spawn_app, assert_is_redirect_to};
use uuid::Uuid;
use validator::HasLen;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let app = spawn_app().await;
    let response = app.get_change_password().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/login");

}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // 第一部分：登录
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // 第二部分：尝试修改密码
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": new_password,
            "new_password_check": another_new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // 第三部分：跟随重定向
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match.</i></p>"
    ));


}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // 第一部分：登录
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    //第二部分： 尝试修改密码
    let response = app.post_change_password(&serde_json::json!({
        "current_password": &wrong_password,
        "new_password": &new_password,
        "new_password_check": &new_password,
    }))
    .await;
    assert_is_redirect_to(&response, "/admin/password");

    // 第三部分： 跟随重定向
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>The current password is incorrect.</i></p>"
    ));

}

#[tokio::test]
async fn changing_password_works() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    //第一部分，登录
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    // 第二部分，修改密码
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    // 第三部分，跟随重定向(书上源代码有问题，这里是正确的)
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    // 第四部分，退出登录
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // 第五部分，跟随重定向
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    // 第六部分，使用新密码登录
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password,
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

}