use axum::serve::ListenerExt;
use clap::Parser;
use journal_server::application::{asset_names, page_files};
use journal_server::config::{Cli, Command, Settings};
use journal_server::listen::{self, Listeners, Reach};
use journal_server::state::{AppState, BuildOptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tantivy logs every segment file it deletes at INFO; that is noise for a journal server
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tantivy=warn")),
        )
        .init();

    let mut cli = Cli::parse();
    let command = cli.command.take();
    let settings = Settings::from_cli(cli);
    tracing::info!("Data: {}", settings.db_path.display());
    tracing::info!("Index: {}", settings.index_dir.display());
    tracing::info!("Config: {}", settings.config_dir.display());
    match &settings.frontend_dir {
        Some(dir) => tracing::info!("Frontend: {}", dir.display()),
        None => tracing::info!("Frontend: not served (no --frontend-dir)"),
    }

    if let Some(command) = command {
        let told = tokio::task::spawn_blocking(move || match command {
            Command::AssetNames { step, file } => asset_names::run(settings, step, &file),
            Command::WritePages => page_files::run(settings),
        })
        .await??;
        println!("{told}");
        return Ok(());
    }

    let port = settings.port;
    let reach = settings.reach();
    let state = tokio::task::spawn_blocking(move || {
        AppState::build(settings, BuildOptions { interactive: true })
    })
    .await??;
    state.start_background_tasks();

    let app = journal_server::build_router(state.clone());
    let listener: Listeners = match reach {
        Reach::Everywhere => bind_everywhere(port).await?.into(),
        Reach::ThisMachine => listen::this_machine(port).await?,
    };
    tracing::info!("PetaJournal server listening on http://localhost:{port} ({reach:?})");
    // Without TCP_NODELAY a response written in two pieces waits for the client's delayed ACK,
    // which adds tens of milliseconds to small requests on keep-alive connections
    let listener = listener.tap_io(|tcp| {
        if let Err(e) = tcp.set_nodelay(true) {
            tracing::debug!("failed to set TCP_NODELAY: {e}");
        }
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("Shutting down");
        })
        .await?;
    tokio::task::spawn_blocking(move || state.shutdown()).await?;
    Ok(())
}

/// Listens on the port for every machine, over both IPv6 and IPv4.
///
/// On Windows, `localhost` is `::1` first. A client then waits about 2s for a refused IPv6
/// connect before it tries IPv4, each time it opens a new connection.
async fn bind_everywhere(port: u16) -> std::io::Result<tokio::net::TcpListener> {
    use socket2::{Domain, Socket, Type};
    let socket = match Socket::new(Domain::IPV6, Type::STREAM, None) {
        Ok(socket) => socket,
        Err(e) => {
            // A machine with IPv6 turned off must still run the server
            tracing::warn!("IPv6 is not available, listening on IPv4 only: {e}");
            return tokio::net::TcpListener::bind(("0.0.0.0", port)).await;
        }
    };
    // Windows sets IPV6_V6ONLY by default, so `[::]` alone would refuse IPv4
    socket.set_only_v6(false)?;
    // As tokio does: elsewhere a restart would fail while old connections are in TIME_WAIT.
    // On Windows the option would let another program take the port
    #[cfg(not(windows))]
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    let address = std::net::SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port));
    socket.bind(&address.into())?;
    socket.listen(1024)?;
    tokio::net::TcpListener::from_std(socket.into())
}
