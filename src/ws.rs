mod transactions;
use axum::Router;
use std::env;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};

fn create_app() -> Router {
    Router::new()
        .route("/hello", axum::routing::get(hello_handler))
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let listen_addr = match env::var_os("WADDLE_LISTEN_ADDR") {
        Some(a) => a.into_string().unwrap(),
        None => "127.0.0.1:3000".to_string(),
    };

    let app = create_app();
    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    println!("Listenin on http://{listen_addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn hello_handler() -> &'static str {
    "Hello World!"
}
