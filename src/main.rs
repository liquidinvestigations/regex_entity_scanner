//! The service shell: load the vendored data, compile the rules, serve.

use std::sync::Arc;

use anyhow::{Context, Result};
use regex_entity_scanner::data::VendoredData;
use regex_entity_scanner::scan::Scanner;
use regex_entity_scanner::service::{self, AppState};

/// Ten mebibytes. Large enough for any fragment a windowing caller should be sending, small enough
/// that a request cannot be used to make the process allocate without bound.
const DEFAULT_MAX_BODY_BYTES: usize = 10_485_760;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RES_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let data = VendoredData::load_from_env()?;
    let scanner = Arc::new(Scanner::new(data)?);
    tracing::info!(
        rules = scanner.rule_ids().len(),
        tlds = scanner.data().tld_count(),
        "rules compiled"
    );

    let max_body_bytes = max_body_bytes()?;
    let state = Arc::new(AppState {
        scanner,
        max_body_bytes,
    });

    let bind = std::env::var("RES_BIND").unwrap_or_else(|_| "0.0.0.0:19705".to_string());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, "listening");

    axum::serve(listener, service::router(state))
        .with_graceful_shutdown(shutdown())
        .await
        .context("serving")?;
    Ok(())
}

/// The request and fragment size limit. An operational parameter that is silently wrong is worse
/// than one that refuses to boot, so an unparseable value fails startup instead of falling back.
fn max_body_bytes() -> Result<usize> {
    match std::env::var("RES_MAX_BODY_BYTES") {
        Ok(raw) => raw
            .trim()
            .parse()
            .with_context(|| format!("RES_MAX_BODY_BYTES is {raw:?}, which is not a byte count")),
        Err(_) => Ok(DEFAULT_MAX_BODY_BYTES),
    }
}

/// Ctrl-C and SIGTERM both end the process cleanly, so `docker stop` does not have to escalate.
async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
