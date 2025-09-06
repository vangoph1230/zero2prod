
use chrono::Utc;
use uuid::Uuid;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use tracing::{instrument, Instrument};


#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/// 一个久经考验的经验法则：在所有通过网络与外部系统交互的过程中，
/// 都要反复不断地记录当前状态。
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let request_id = Uuid::new_v4();
    // 创建一个新跨度，并将一些值绑定其上
    // 允许以键值对的方式与跨度关联起来
    // 使用 %符号作为前缀来修饰变量
    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );

    // 显示的调用.enter()来激活跨度，
    // .enter()返回一个Entered类型的值，这个是一个守卫对象：在这个变量被析构前，
    //  所有的下游跨度都会被注册为当前跨度的子跨度。 
    let _request_span_guard = request_span.enter();
    // 在异步函数中，请勿使用'enter'，可能导致灾难性后果，此处暂用；
    // _request_span_gurad在‘subscribe'结束时析构，
    // 此时就'退出'了这个跨度，
    // 可以反复的进入和退出一个跨度。而关闭一个跨度是终结性的，即被析构时发生

    //不用对跨度调用'.enter()'
    //'.instrument'会在合适的时机，根据future的状态调用'.enter()'
    let query_span = tracing::info_span!(
        "Saving new subscriber details in the database."
    );
    

    // Instrument是一个用于扩展future的trait,
    // 以跨度为参数，future被轮询时，进入该跨度
    // future被挂起时，退出该跨度。
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,

        Utc::now(),
    )
    .execute(pool.get_ref())
    .instrument(query_span)
    .await
    {
        Ok(_) => {
            HttpResponse::Ok().finish()
        },
        Err(e) => {
            tracing::error!("Failed to execute query: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}