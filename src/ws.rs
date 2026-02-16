use std::env;

use axum::{Router, routing::get};
use log::info;

#[tokio::main]
async fn main() {
    env_logger::init();
    let listen_addr = match env::var_os("WADDLE_LISTEN_PORT") {
        Some(a) => a.into_string().unwrap(),
        None => "127.0.0.1:3000".to_string(),
    };

    let app = Router::new().route("/", get(handler));
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn handler() -> &'static str {
    log::info!("GET / request received");
    "Hello, World!\n"
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl-c handler");
    info!("Ctr-c pressed");
}
