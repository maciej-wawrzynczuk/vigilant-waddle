use axum::{
    Json, Router,
    extract::{Multipart, State},
    response::{IntoResponse, Response},
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
use vigilant_waddle::transactions::{Portfolio, Transactions};

#[derive(Clone)]
struct AppState {
    transactions: Arc<Mutex<Option<Transactions>>>,
}

fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/transactions", axum::routing::put(upload_transactions))
        .route("/transactions", axum::routing::get(get_transactions))
        .route("/portfolio", axum::routing::get(get_portfolio))
        .layer(TraceLayer::new_for_http())
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

    let state = AppState {
        transactions: Arc::new(Mutex::new(None)),
    };

    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind(&listen_addr).await.unwrap();
    println!("Listening on http://{listen_addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn get_portfolio(State(s): State<AppState>) -> Response {
    let data = s.transactions.lock().unwrap();
    let Some(t) = data.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let portfolio = Portfolio::from_transactions(t);
    match serde_yaml::to_string(&portfolio) {
        Err(e) => {
            error!("YAML serialization error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(yaml) => (
            [(axum::http::header::CONTENT_TYPE, "application/yaml")],
            yaml,
        )
            .into_response(),
    }
}

async fn get_transactions(State(s): State<AppState>) -> Result<Json<Transactions>, StatusCode> {
    let data = s.transactions.lock().unwrap();
    match data.as_ref() {
        Some(t) => Ok(Json(t.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn upload_transactions(
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
            *guard = Some(t);
            return Ok(());
        }
    }
    error!("No transaction_log field in the request");
    Err(StatusCode::BAD_REQUEST)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_get_portfolio_empty() {
        let state = AppState {
            transactions: Arc::new(Mutex::new(None)),
        };
        let app = create_app(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/portfolio")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_portfolio_after_upload() {
        let state = AppState {
            transactions: Arc::new(Mutex::new(None)),
        };
        let csv_data =
            "date;symbol;number;price;commision;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let boundary = "----boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"transaction_log\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, csv_data, boundary
        );
        let _ = create_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = create_app(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/portfolio")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/yaml"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let yaml = std::str::from_utf8(&body).unwrap();
        assert!(yaml.contains("FOO: 1"), "yaml was: {yaml}");
    }

    #[tokio::test]
    async fn test_upload_transactions_success() {
        let state = AppState {
            transactions: Arc::new(Mutex::new(None)),
        };

        let app = create_app(state.clone());

        let csv_data =
            "date;symbol;number;price;commision;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let boundary = "----boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"transaction_log\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, csv_data, boundary
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

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
            "--{}\r\nContent-Disposition: form-data; name=\"transaction_log\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, invalid_csv, boundary
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

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
        let csv_data =
            "date;symbol;number;price;commision;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let boundary = "----boundary";
        let body = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"transaction_log\"; filename=\"test.csv\"\r\n\r\n{}\r\n--{}--\r\n",
            boundary, csv_data, boundary
        );

        let _ = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={}", boundary),
                    )
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
        assert!(json.as_array().unwrap().len() == 1);
        assert!(json[0]["symbol"] == "FOO");
    }
}
