use log::info;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
mod game;
mod player;
mod server;
mod utils;
mod websocket; // Import the module
use server::GameServer;
use websocket::handle_client;

#[tokio::main]
async fn main() {
    env_logger::init();
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    info!("WebSocket server listening on ws://127.0.0.1:8080");

    let server = Arc::new(Mutex::new(GameServer::new()));
    // Start logging active games
    GameServer::start_logging_active_games(server.clone());
    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let server_clone = server.clone();
        tokio::spawn(handle_client(ws_stream, server_clone));
    }
}
