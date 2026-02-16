use std::env;
use std::sync::{Arc, Mutex};

use axum::{extract::State, Router, routing::get};
use axum::extract::Multipart;
use log::info;
use tower::ServiceExt;

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_handler_returns_message() {
        let state = AppState {
            message: "Test message\n".to_string(),
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = Router::new()
            .route("/", get(handler))
            .with_state(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"Test message\n");
    }

    #[tokio::test]
    async fn test_upload_transactions_success() {
        let state = AppState {
            message: "Hello".to_string(),
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = Router::new()
            .route("/transactions", axum::routing::put(upload_transactions))
            .with_state(state.clone());

        let csv_data = "date;symbol;number;price;commision;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let boundary = "----boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, csv_data, boundary
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"Transactions uploaded successfully\n");

        // Verify transactions were stored
        assert!(state.transactions.lock().unwrap().is_some());
    }

    #[tokio::test]
    async fn test_upload_transactions_invalid_csv() {
        let state = AppState {
            message: "Hello".to_string(),
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = Router::new()
            .route("/transactions", axum::routing::put(upload_transactions))
            .with_state(state.clone());

        let invalid_csv = "invalid,csv,data\n";
        let boundary = "----boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, invalid_csv, boundary
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header("content-type", format!("multipart/form-data; boundary={}", boundary))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        
        // Verify transactions were NOT stored
        assert!(state.transactions.lock().unwrap().is_none());
    }
}
