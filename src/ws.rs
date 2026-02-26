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
use tracing::{error, info};
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};
use vigilant_waddle::transactions::Transactions;

#[derive(Clone, Debug)]
struct AppState {
    msg: String,
    transactions: Arc<Mutex<Transactions>>,
}

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
    State(s): State<AppState>,
    mut data: Multipart,
) -> Result<(), StatusCode> {
    while let Some(f) = data.next_field().await.map_err(|_| {
        error!("Multipart error");
        StatusCode::BAD_REQUEST
    })? {
        if let Some(name) = f.name()
            && name == "transaction_log"
        {
            let csv = f
                .bytes()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let csv_c = Cursor::new(csv);
            let t = Transactions::try_from_reader(csv_c).map_err(|e| {
                error!("CSV read error: {e}");
                StatusCode::BAD_REQUEST
            })?;
            info!("{} loaded", t.iter().count());
            let mut guard = s
                .transactions
                .lock()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            *guard = t;
            return Ok(());
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
