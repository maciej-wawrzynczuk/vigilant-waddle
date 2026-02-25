mod transactions;
use axum::{
    Json, Router,
    extract::{Multipart, State},
};
use http::StatusCode;
use std::{
    env,
    io::Cursor,
    sync::{Arc, Mutex},
};
use tower_http::trace::TraceLayer;
use tracing::error;
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};
use transactions::Transactions;

#[derive(Clone, Debug)]
struct AppState {
    msg: String,
    transactions: Arc<Mutex<Transactions>>,
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

async fn hello_handler(State(s): State<AppState>) -> String {
    s.msg
}

async fn get_tranasactions(State(s): State<AppState>) -> Json<Transactions> {
    let data = s.transactions.lock().unwrap();
    Json(data.clone())
}

async fn post_transactions(
    State(mut s): State<AppState>,
    mut data: Multipart,
) -> Result<(), StatusCode> {
    while let Some(f) = data.next_field().await.map_err(|_| {
        error!("Multipart error");
        StatusCode::BAD_REQUEST
    })? {
        if let Some(name) = f.name() {
            if name == "transaction_log" {
                let csv = f
                    .bytes()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let csv_c = Cursor::new(csv);
                s.transactions = Arc::new(Mutex::new(
                    Transactions::try_from_reader(csv_c).map_err(|e| {
                        error!("CSV read error: {e}");
                        StatusCode::BAD_REQUEST
                    })?,
                ));
                return Ok(());
            }
        }
    }
    error!("No transaction_log field in the request");
    Err(StatusCode::BAD_REQUEST)
}

fn create_app() -> Router {
    let s = AppState {
        msg: "Hello World!".to_string(),
        transactions: Arc::new(Mutex::new(Transactions::new())),
    };
    Router::new()
        .route("/hello", axum::routing::get(hello_handler))
        .route("/transactions", axum::routing::get(get_tranasactions))
        .route("/transactions", axum::routing::post(post_transactions))
        .layer(TraceLayer::new_for_http())
        .with_state(s)
}
