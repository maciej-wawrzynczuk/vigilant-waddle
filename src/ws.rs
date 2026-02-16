use std::env;
use std::sync::{Arc, Mutex};

use axum::{extract::State, Router};
use axum::extract::Multipart;
use log::info;

use crate::transactions::Transactions;

#[path = "transactions.rs"]
mod transactions;

#[derive(Clone)]
struct AppState {
    transactions: Arc<Mutex<Option<Transactions>>>,
}

fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/transactions", axum::routing::put(upload_transactions))
        .route("/transactions", axum::routing::get(get_transactions))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let listen_addr = match env::var_os("WADDLE_LISTEN_PORT") {
        Some(a) => a.into_string().unwrap(),
        None => "127.0.0.1:3000".to_string(),
    };

    let state = AppState {
        transactions: Arc::new(Mutex::new(None)),
    };

    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn upload_transactions(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> (axum::http::StatusCode, String) {
    let mut all_data = Vec::new();
    let mut field_count = 0;
    
    while let Ok(Some(field)) = multipart.next_field().await {
        field_count += 1;
        
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => return (axum::http::StatusCode::BAD_REQUEST, format!("Failed to read bytes: {}\n", e)),
        };
        
        all_data.extend_from_slice(&data);
    }
    
    if field_count == 0 {
        return (axum::http::StatusCode::BAD_REQUEST, "No file provided\n".to_string());
    }
    
    let cursor = std::io::Cursor::new(all_data);
    
    match Transactions::try_from_reader(cursor) {
        Ok(transactions) => {
            *state.transactions.lock().unwrap() = Some(transactions);
            log::info!("Transactions uploaded successfully");
            (axum::http::StatusCode::OK, "Transactions uploaded successfully\n".to_string())
        }
        Err(e) => {
            log::error!("Failed to parse transactions: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse CSV: {}\n", e))
        }
    }
}

async fn get_transactions(
    State(state): State<AppState>,
) -> (axum::http::StatusCode, String) {
    let guard = state.transactions.lock().unwrap();
    match &*guard {
        Some(transactions) => {
            match transactions.to_json() {
                Ok(json) => (axum::http::StatusCode::OK, json),
                Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to serialize: {}\n", e)),
            }
        }
        None => (axum::http::StatusCode::NOT_FOUND, "No transactions uploaded\n".to_string()),
    }
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
    async fn test_upload_transactions_success() {
        let state = AppState {
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = create_app(state.clone());

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
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = create_app(state.clone());

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

    #[tokio::test]
    async fn test_get_transactions_empty() {
        let state = AppState {
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = create_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/transactions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_transactions_after_upload() {
        let state = AppState {
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = create_app(state.clone());

        // First upload
        let csv_data = "date;symbol;number;price;commision;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let boundary = "----boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, csv_data, boundary
        );

        let _ = app
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

        // Then GET
        let app = create_app(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/transactions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["p"].as_array().unwrap().len() == 1);
        assert!(json["p"][0]["symbol"] == "FOO");
    }
}
