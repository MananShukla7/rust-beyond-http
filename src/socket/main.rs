use axum::Router;
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data, SocketRef, State},
    SocketIo,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Default, Clone)]
struct AppState {
    client_count: Arc<RwLock<usize>>,
}

#[derive(Debug, Deserialize)]
struct JoinRoom {
    room: String,
}

#[derive(Debug, Serialize)]
struct ClientCount {
    count: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let state = AppState::default();

    let (layer, io) = SocketIo::builder()
        .with_state(state.clone())
        .build_layer();

    io.ns("/", |socket: SocketRef, State(state): State<AppState>| async move {
        info!("Client connected: {}", socket.id);

        {
            let mut count = state.client_count.write().await;
            *count += 1;
            let current = *count;
            socket.broadcast().emit("client_count", &ClientCount { count: current }).await.ok();
        }

        socket.on("join_room", |socket: SocketRef, Data::<JoinRoom>(data)| async move {
            info!("Client {} joining room: {}", socket.id, data.room);
            socket.join(data.room.clone());
            socket.emit("joined", &serde_json::json!({ "room": data.room })).ok();
        });

        socket.on("leave_room", |socket: SocketRef, Data::<JoinRoom>(data)| async move {
            socket.leave(data.room.clone());
        });

        socket.on_disconnect(|socket: SocketRef, State(state): State<AppState>| async move {
            info!("Client disconnected: {}", socket.id);
            let mut count = state.client_count.write().await;
            *count = count.saturating_sub(1);
            let current = *count;
            socket.broadcast().emit("client_count", &ClientCount { count: current }).await.ok();
        });
    });

    // socketioxide 0.18 works with axum 0.8
    let app = Router::new().layer(
        ServiceBuilder::new()
            .layer(CorsLayer::permissive())
            .layer(layer),
    );

    let addr = "0.0.0.0:3000";
    info!("Socket.IO server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
