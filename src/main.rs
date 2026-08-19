//! The service shell: load the vendored data, compile the rules, serve.

use std::sync::Arc;

use anyhow::{Context, Result};
use regex_entity_scanner::data::VendoredData;
use regex_entity_scanner::scan::Scanner;
use regex_entity_scanner::service;

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

    let bind = std::env::var("RES_BIND").unwrap_or_else(|_| "0.0.0.0:19705".to_string());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, "listening");

    axum::serve(listener, service::router(scanner))
        .with_graceful_shutdown(shutdown())
        .await
        .context("serving")?;
    Ok(())
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
