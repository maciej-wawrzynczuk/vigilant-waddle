mod transactions;
use axum::{Router, extract::State};
use std::env;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};

#[derive(Clone, Debug)]
struct HelloMsg {
    msg: String,
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

async fn hello_handler(State(s): State<HelloMsg>) -> String {
    s.msg
}

fn create_app() -> Router {
    let s = HelloMsg {
        msg: "Hello World!".to_string(),
    };
    Router::new()
        .route("/hello", axum::routing::get(hello_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(s)
}
