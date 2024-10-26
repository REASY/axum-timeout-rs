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
use tokio::io::AsyncWriteExt;
use tokio::signal;
use tower::{BoxError, ServiceBuilder};
use tracing::{info, warn};

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

async fn write_to_disk(path: &str) -> errors::Result<()> {
    let mut file = tokio::fs::File::create(path).await?;
    for i in 1..100 {
        file.write_all(format!("{}\n", i).as_bytes()).await?;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    file.flush().await?;
    Ok(())
}

async fn info_handler() -> errors::Result<Json<Info>> {
    info!("Starting executing `info_handler`");
    let info = Info {
        version: APP_VERSION.to_string(),
        current_timestamp: Utc::now(),
    };
    write_to_disk("data.txt").await?;
    info!("Finished executing `info_handler`");
    Ok(Json(info))
}

const TIMEOUT_DURATION: Duration = Duration::from_secs(1);

async fn handle_timeout_error(err: BoxError) -> (StatusCode, Json<ErrorMessage>) {
    if err.is::<tower::timeout::error::Elapsed>() {
        warn!("Slow request timed out");
        (
            StatusCode::REQUEST_TIMEOUT,
            Json(ErrorMessage {
                code: StatusCode::REQUEST_TIMEOUT.as_u16(),
                reason: StatusCode::REQUEST_TIMEOUT
                    .canonical_reason()
                    .map(|s| s.to_string()),
                message: format!("Request took longer than {:?}", TIMEOUT_DURATION),
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
    logger::setup("INFO");
    let route = Router::new().route("/version", get(info_handler)).layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(handle_timeout_error))
            .timeout(TIMEOUT_DURATION)
            .into_inner(),
    );

    let http_addr: SocketAddr = format!("{}:{}", "127.0.0.1", "18080").parse().unwrap();

    info!("Server listening for HTTP on {}", &http_addr);
    let svc = route.into_make_service_with_connect_info::<SocketAddr>();
    let http_listener = tokio::net::TcpListener::bind(http_addr).await.unwrap();
    axum::serve(http_listener, svc.clone())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Failed to start server");
    info!("Server shutdown");

    Ok(())
}

async fn shutdown_signal() {
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
        },
        _ = terminate => {
        },
    }

    info!("signal received, starting graceful shutdown");
}
