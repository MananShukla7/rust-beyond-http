use axum::Router;
use serde::{Deserialize, Serialize};
use socketioxide::{extract::SocketRef, SocketIo};
use std::path::PathBuf;
use suppaftp::AsyncFtpStream;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadEvent {
    pub filename:    String,
    pub size_bytes:  u64,
    pub uploaded_at: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

async fn run_socket_server(
    mut upload_rx: broadcast::Receiver<UploadEvent>,
) -> anyhow::Result<()> {
    let (layer, io) = SocketIo::builder().build_layer();
    let io_handle   = io.clone();

    // Receive upload events and push to all connected browsers
    tokio::spawn(async move {
        loop {
            match upload_rx.recv().await {
                Ok(event) => {
                    info!("Broadcasting: {}", event.filename);
                    io_handle.emit("file_uploaded", &event).await.ok();
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    error!("Missed {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    io.ns("/", |socket: SocketRef| async move {
        info!("Browser connected: {}", socket.id);
        socket.on_disconnect(|socket: SocketRef| async move {
            info!("Browser disconnected: {}", socket.id);
        });
    });

    let app = Router::new().layer(
        ServiceBuilder::new()
            .layer(CorsLayer::permissive())
            .layer(layer),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Socket.IO ready on :3000");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn upload_and_notify(
    host: &str,
    user: &str,
    pass: &str,
    local_path: &PathBuf,
    tx: broadcast::Sender<UploadEvent>,
) -> anyhow::Result<()> {
    let filename = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let data = tokio::fs::read(local_path).await?;
    let size = data.len() as u64;

    let mut ftp = AsyncFtpStream::connect(host).await?;
    ftp.login(user, pass).await?;
    let mut reader = data.as_slice();
    ftp.put_file(&filename, &mut reader).await?;
    ftp.quit().await?;
    info!("FTP upload complete: {} ({} bytes)", filename, size);

    // Notify all connected Socket.IO clients
    tx.send(UploadEvent {
        filename,
        size_bytes:  size,
        uploaded_at: now_ms(),
    }).ok();

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    // broadcast channel bridges FTP uploader → Socket.IO broadcaster
    let (tx, rx) = broadcast::channel::<UploadEvent>(16);

    tokio::spawn(run_socket_server(rx));
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let host = std::env::var("FTP_HOST").unwrap_or_else(|_| "127.0.0.1:2121".to_string());
    let user = std::env::var("FTP_USER").unwrap_or_else(|_| "uploader".to_string());
    let pass = std::env::var("FTP_PASS").unwrap_or_else(|_| "secret123".to_string());

    // Create and upload a test file
    let test_file = PathBuf::from("/tmp/test-upload.txt");
    tokio::fs::write(&test_file, b"Hello from the combined system!").await?;
    upload_and_notify(&host, &user, &pass, &test_file, tx).await?;

    // Keep the socket server alive
    tokio::signal::ctrl_c().await?;
    Ok(())
}
