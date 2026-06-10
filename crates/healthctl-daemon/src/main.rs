mod db;
mod handler;

use anyhow::Result;
use healthctl_lib::ipc;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing.
    let use_journald = try_init_journald();
    if !use_journald {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    tracing::info!("healthctl-daemon starting");

    let db = db::Database::open().await?;
    let db = Arc::new(db);
    let shutdown = Arc::new(Notify::new());

    let listener = create_listener()?;
    tracing::info!("listening on {}", ipc::socket_path().display());

    let shutdown_clone = shutdown.clone();
    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let db = db.clone();
                        let shutdown = shutdown_clone.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, db, shutdown).await {
                                tracing::error!("connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("accept error: {e}");
                    }
                }
            }
            _ = shutdown_clone.notified() => {
                tracing::info!("shutdown requested");
                break;
            }
        }
    }

    // Cleanup socket file.
    let _ = std::fs::remove_file(ipc::socket_path());
    tracing::info!("daemon exiting");
    Ok(())
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    db: Arc<db::Database>,
    shutdown: Arc<Notify>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line).await?;
    let request: ipc::Request = serde_json::from_str(line.trim())?;

    let is_shutdown = matches!(request, ipc::Request::Shutdown);
    let response = handler::handle_request(request, &db).await;

    let response_json = serde_json::to_string(&response)?;
    writer.write_all(response_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    if is_shutdown {
        shutdown.notify_one();
    }

    Ok(())
}

/// Create the UNIX listener, checking for systemd socket activation first.
fn create_listener() -> Result<UnixListener> {
    // Check for systemd socket activation: FD 3.
    if let Ok(listen_fds) = std::env::var("LISTEN_FDS")
        && listen_fds.parse::<u32>().unwrap_or(0) >= 1
    {
        tracing::info!("using systemd socket activation (FD 3)");
        use std::os::unix::io::FromRawFd;
        let std_listener = unsafe { StdUnixListener::from_raw_fd(3) };
        std_listener.set_nonblocking(true)?;
        return Ok(UnixListener::from_std(std_listener)?);
    }

    // Otherwise, create our own socket.
    let socket_path = ipc::socket_path();

    // Remove stale socket.
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }

    // Ensure parent directory exists.
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let std_listener = StdUnixListener::bind(&socket_path)?;
    std_listener.set_nonblocking(true)?;
    Ok(UnixListener::from_std(std_listener)?)
}

/// Try to initialize tracing-journald. Returns true if successful.
fn try_init_journald() -> bool {
    // Only use journald if running under systemd (indicated by JOURNAL_STREAM).
    if std::env::var("JOURNAL_STREAM").is_ok()
        && let Ok(layer) = tracing_journald::layer()
    {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        tracing_subscriber::registry()
            .with(layer)
            .with(EnvFilter::from_default_env())
            .init();
        return true;
    }
    false
}
