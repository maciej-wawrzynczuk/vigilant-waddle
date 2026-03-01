use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
};
use http::{HeaderMap, StatusCode};
use std::{
    env,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};
use vigilant_waddle::{
    stooq::StooqQuotes,
    transactions::{MockQuotes, Portfolio, PortfolioValuation, Quotes, Transactions},
};

#[derive(Clone)]
struct AppState {
    transactions: Arc<Mutex<Option<Transactions>>>,
    quotes: Arc<dyn Quotes>,
}

fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/transactions", axum::routing::put(upload_transactions))
        .route("/transactions", axum::routing::get(get_transactions))
        .route("/portfolio", axum::routing::get(get_portfolio))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let listen_addr = match env::var("WADDLE_LISTEN_ADDR") {
        Ok(a) => a,
        Err(env::VarError::NotUnicode(v)) => {
            error!("WADDLE_LISTEN_ADDR is not valid UTF-8: {v:?}, using default");
            "127.0.0.1:3000".to_string()
        }
        Err(env::VarError::NotPresent) => "127.0.0.1:3000".to_string(),
    };

    let provider = env::var("WADDLE_QUOTES_PROVIDER").unwrap_or_else(|_| {
        eprintln!("WADDLE_QUOTES_PROVIDER env var is required (stooq|mock)");
        std::process::exit(1);
    });

    let quotes: Arc<dyn Quotes> = match provider.as_str() {
        "stooq" => {
            let map_path = env::var("STOOQ_SYMBOL_MAP").unwrap_or_else(|_| {
                eprintln!("STOOQ_SYMBOL_MAP env var is required when using stooq provider");
                std::process::exit(1);
            });
            let sq = StooqQuotes::from_toml_file(&PathBuf::from(map_path)).unwrap_or_else(|e| {
                eprintln!("Failed to load stooq symbol map: {e}");
                std::process::exit(1);
            });
            Arc::new(sq)
        }
        "mock" => Arc::new(MockQuotes::new()),
        other => {
            eprintln!("Unknown WADDLE_QUOTES_PROVIDER '{other}'; expected stooq or mock");
            std::process::exit(1);
        }
    };

    let state = AppState {
        transactions: Arc::new(Mutex::new(None)),
        quotes,
    };

    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("failed to bind to listen address");
    info!("Listening on http://{listen_addr}");
    axum::serve(listener, app).await.expect("server error");
}

async fn get_portfolio(State(s): State<AppState>) -> Result<Json<PortfolioValuation>, StatusCode> {
    let t = {
        let data = s
            .transactions
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        data.as_ref().cloned().ok_or(StatusCode::NOT_FOUND)?
    };
    let portfolio = Portfolio::from_transactions(&t);
    let val = portfolio
        .valuation(&*s.quotes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(val))
}

async fn get_transactions(State(s): State<AppState>) -> Result<Json<Transactions>, StatusCode> {
    let data = s
        .transactions
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match data.as_ref() {
        Some(t) => Ok(Json(t.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn upload_transactions(
    State(s): State<AppState>,
    headers: HeaderMap,
    mut data: Multipart,
) -> Result<(), StatusCode> {
    {
        let guard = s
            .transactions
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if guard.is_some() {
            let confirmed =
                headers.get("x-overwrite").and_then(|v| v.to_str().ok()) == Some("confirm");
            if !confirmed {
                return Err(StatusCode::CONFLICT);
            }
        }
    }
    while let Some(f) = data.next_field().await.map_err(|e| {
        error!("Multipart error: {e}");
        StatusCode::BAD_REQUEST
    })? {
        if let Some(name) = f.name()
            && name == "transaction_log"
        {
            let csv = f.bytes().await.map_err(|e| {
                error!("Failed to read upload bytes: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            let csv_c = Cursor::new(csv);
            let t = Transactions::try_from_reader(csv_c).map_err(|e| {
                error!("CSV read error: {e}");
                StatusCode::BAD_REQUEST
            })?;
            info!("{} transactions loaded", t.len());
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

    fn empty_state() -> AppState {
        AppState {
            transactions: Arc::new(Mutex::new(None)),
            quotes: Arc::new(MockQuotes::new()),
        }
    }

    fn multipart_csv(csv: &str) -> (String, String) {
        let boundary = "----boundary".to_string();
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; \
             name=\"transaction_log\"; filename=\"test.csv\"\r\n\r\n\
             {csv}\r\n--{boundary}--\r\n"
        );
        (boundary, body)
    }

    #[tokio::test]
    async fn test_get_portfolio_empty() {
        let state = empty_state();
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
        let state = empty_state();
        let csv = "date;symbol;number;price;commission;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let (boundary, body) = multipart_csv(csv);
        let upload = create_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(upload.status(), StatusCode::OK);

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
        assert!(
            response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("application/json"),
            "expected application/json content-type"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json[0]["symbol"], "FOO");
        assert_eq!(json[0]["quantity"], 1);
        assert_eq!(json[0]["value"], "1");
        assert_eq!(json[0]["currency"], "BAR");
    }

    #[tokio::test]
    async fn test_upload_transactions_success() {
        let state = empty_state();

        let app = create_app(state.clone());

        let csv = "date;symbol;number;price;commission;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let (boundary, body) = multipart_csv(csv);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let get = create_app(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/transactions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_upload_transactions_invalid_csv() {
        let state = empty_state();

        let app = create_app(state.clone());

        let (boundary, body) = multipart_csv("invalid,csv,data\n");

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let get = create_app(state)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/transactions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_upload_transactions_conflict_requires_header() {
        let state = empty_state();
        let csv = "date;symbol;number;price;commission;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";

        // First upload succeeds (no existing data)
        let (boundary, body) = multipart_csv(csv);
        let first = create_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        // Second upload without header returns 409
        let (boundary, body) = multipart_csv(csv);
        let second = create_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::CONFLICT);

        // Third upload with confirmation header succeeds
        let (boundary, body) = multipart_csv(csv);
        let third = create_app(state)
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .header("x-overwrite", "confirm")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(third.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_transactions_empty() {
        let state = empty_state();

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
        let state = empty_state();

        let app = create_app(state.clone());

        // First upload
        let csv = "date;symbol;number;price;commission;currency\n2000-01-01;FOO;1;42.42;4.2;BAR\n";
        let (boundary, body) = multipart_csv(csv);
        let upload = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/transactions")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(upload.status(), StatusCode::OK);

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
