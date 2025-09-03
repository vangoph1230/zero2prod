use actix_web::{web, App, HttpRequest, HttpServer, Responder};

// 返回 dyn Responder 和 impl Responder的区别：
//  - 前者运行时动态分派，有运行时开销；后者在编译时解析为具体类型。
//  - 返回impl Responder： 返回一个实现了 Responder trait 的具体类型，
//    但我不需要告诉你具体是哪种类型。一种静态分派的写法，结合了 trait 约束和类型推断的优势，在编译时，编译器就知道具体的返回类型。
// Responder trait的类型可以被转换为Http响应,
// 常见的实现Responder trait的类型包括：
//  - String: 自动设置为 text/plain 内容类型
//  - &str: 同String
//  - json<T>: 自动设置为 application/json 内容类型
//  - HttpResponse: 手动构建的完整响应
//  - (): 空响应，返回 200 OK
//  - Result<T,E>: 其中 T: Responder, E: Into<Error>
async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
