use tracing::{Subscriber, subscriber::set_global_default};
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use tracing_subscriber::fmt::MakeWriter;
use tracing_log::LogTracer;
use tracing_bunyan_formatter::{JsonStorageLayer, BunyanFormattingLayer};

/// 获取tracing-subscriber中的注册表
/// - std::io:stdout 输出到终端，即日志可见，输出到屏幕
/// - std::io::sink 输出到空设备，即日志被丢弃，不可见
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync 
    where
        // 该语法结构是高阶trait bound,意思是Sink会实现MakeWrite trait
        Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| 
            EnvFilter::new(env_filter)
        );

    let formatting_layer = BunyanFormattingLayer::new(
        name, 
        sink,
    );
 
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// 设置应用程序全局默认的tracing-subscriber订阅器
/// 其中，set_global_default函数仅能调用一次
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    // set_global_default仅能调用一次
    set_global_default(subscriber).expect("Failed to set subscriber");
}