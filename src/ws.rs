use std::env;
use std::sync::{Arc, Mutex};

use axum::{extract::State, Router, routing::get};
use axum::extract::Multipart;
use log::info;

use crate::transactions::Transactions;

#[derive(Clone)]
struct AppState {
    message: String,
    transactions: Arc<Mutex<Option<Transactions>>>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let listen_addr = match env::var_os("WADDLE_LISTEN_PORT") {
        Some(a) => a.into_string().unwrap(),
        None => "127.0.0.1:3000".to_string(),
    };

    let state = AppState {
        message: "Hello, World!\n".to_string(),
        transactions: Arc::new(Mutex::new(None)),
    };

    let app = Router::new()
        .route("/", get(handler))
        .route("/transactions", axum::routing::put(upload_transactions))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn handler(State(state): State<AppState>) -> String {
    log::info!("GET / request received");
    state.message
}

async fn upload_transactions(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<String, String> {
    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let data = field.bytes().await.map_err(|e| e.to_string())?;
        let cursor = std::io::Cursor::new(data);
        
        match Transactions::try_from_reader(cursor) {
            Ok(transactions) => {
                *state.transactions.lock().unwrap() = Some(transactions);
                log::info!("Transactions uploaded successfully");
                return Ok("Transactions uploaded successfully\n".to_string());
            }
            Err(e) => {
                log::error!("Failed to parse transactions: {}", e);
                return Err(format!("Failed to parse CSV: {}\n", e));
            }
        }
    }
    
    Err("No file provided\n".to_string())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl-c handler");
    info!("Ctr-c pressed");
}
