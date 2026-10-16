use clap::Parser;
use journal_server::config::{Cli, Settings};
use journal_server::state::{AppState, BuildOptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tantivy logs every segment file it deletes at INFO; that is noise for a journal server
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tantivy=warn")))
        .init();

    let settings = Settings::from_cli(Cli::parse());
    tracing::info!("Data: {}", settings.db_path.display());
    tracing::info!("Index: {}", settings.index_dir.display());
    tracing::info!("Config: {}", settings.config_dir.display());

    let port = settings.port;
    let state = tokio::task::spawn_blocking(move || AppState::build(settings, BuildOptions { interactive: true }))
        .await??;
    state.start_background_tasks();

    let app = journal_server::build_router(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("PetaJournal server listening on http://localhost:{port}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("Shutting down");
        })
        .await?;
    Ok(())
}
