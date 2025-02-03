// main.rs

use log::info;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
mod game;
mod player;
mod server;
mod utils;
mod websocket;
use server::GameServer;
use websocket::handle_client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?; // Use ? here

    info!("WebSocket server listening on ws://127.0.0.1:8080");

    let server = Arc::new(Mutex::new(GameServer::new()));
    GameServer::start_logging_active_games(server.clone());

    loop {
        let (stream, _) = listener.accept().await?; // Use ? here

        let ws_stream = accept_async(stream).await?; // Use ? here

        let server_clone = server.clone();
        tokio::spawn(handle_client(ws_stream, server_clone));
    }
}
