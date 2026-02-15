use axum::{Router, routing::get};
use log::info;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello, World!\n" }));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl-c handler");
    info!("Ctr-c pressed");
}
