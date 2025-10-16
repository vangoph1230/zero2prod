use actix_web::HttpResponse;
use reqwest::header::LOCATION;
use uuid::Uuid;
use actix_web::http::header::ContentType;
use actix_web::web;
use anyhow::Context;
use sqlx::PgPool;
use crate::session_state::TypedSession;
use crate::utils::e500;

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session
        .get_user_id()
        .map_err(e500)?
        {
            get_username(user_id, &pool).await.map_err(e500)?
        } else {
            // 未登录的用户重定向到登录页面
            return Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish()
                );
        };
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
                <head>
                    <meta http-equiv="content-type" content="text/html; charset=utf-8">
                    <title>Admin dashboard</title>
                </head>
                <body>
                    <p>Welcome {username}!</p>
                    <p>Available actions:</p>
                        <ol>
                            <li><a href="/admin/password">Change password</a></li>
                            <li><a href="/admin/newsletters">Pulish newsletters</a></li>
                            <li>
                                <form name="logoutForm" action="/admin/logout" method="post">
                                    <input type="submit" value="Logout">
                                </form>
                            </li>
                        </ol>
                </body>
            </html>
            "#,
        ))
    )
}

pub async fn get_username(
    user_id: Uuid,
    pool: &PgPool,
) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username FROM users
        WHERE user_id = $1
        "#,
        user_id,
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username.")?;

    Ok(row.username)
}