mod errors;
mod logger;

use shadow_rs::shadow;

use axum::error_handling::HandleErrorLayer;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tower::{BoxError, ServiceBuilder};
use tracing::info;

shadow!(build);

pub const APP_VERSION: &str = shadow_rs::formatcp!(
    "{} ({} {}), build_env: {}, {}, {}",
    build::PKG_VERSION,
    build::SHORT_COMMIT,
    build::BUILD_TIME,
    build::RUST_VERSION,
    build::RUST_CHANNEL,
    build::CARGO_VERSION
);

#[derive(Serialize)]
struct Info {
    version: String,
    current_timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    reason: Option<String>,
    message: String,
}

async fn info_handler() -> errors::Result<Json<Info>> {
    info!("Starting executing `info_handler`");
    let info = Info {
        version: APP_VERSION.to_string(),
        current_timestamp: Utc::now(),
    };
    info!("Finished executing `info_handler`");
    Ok(Json(info))
}

async fn handle_timeout_error(err: BoxError) -> (StatusCode, Json<ErrorMessage>) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            Json(ErrorMessage {
                code: StatusCode::REQUEST_TIMEOUT.as_u16(),
                reason: StatusCode::REQUEST_TIMEOUT
                    .canonical_reason()
                    .map(|s| s.to_string()),
                message: "Request took too long".to_string(),
            }),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorMessage {
                code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                reason: StatusCode::INTERNAL_SERVER_ERROR
                    .canonical_reason()
                    .map(|s| s.to_string()),
                message: format!("Unhandled internal error: {err}"),
            }),
        )
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> errors::Result<()> {
    let token = CancellationToken::new();
    logger::setup("INFO");
    let route = Router::new().route("/version", get(info_handler)).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_timeout_error))
            .timeout(Duration::from_secs(2))
            .into_inner(),
    );

    let http_addr: SocketAddr = format!("{}:{}", "127.0.0.1", "18080").parse().unwrap();

    info!("Server listening for HTTP on {}", &http_addr);
    let svc = route.into_make_service_with_connect_info::<SocketAddr>();
    let http_listener = tokio::net::TcpListener::bind(http_addr).await.unwrap();
    axum::serve(http_listener, svc.clone())
        .with_graceful_shutdown(shutdown_signal(token.clone()))
        .await
        .expect("Failed to start server");
    info!("Server shutdown");

    Ok(())
}

async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            token.cancel()
        },
        _ = terminate => {
            token.cancel()
        },
    }

    info!("signal received, starting graceful shutdown");
}
