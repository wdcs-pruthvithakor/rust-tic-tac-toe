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
use std::error::Error;
use websocket::handle_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    info!("WebSocket server listening on ws://127.0.0.1:8080");

    let server = Arc::new(Mutex::new(GameServer::new()));
    GameServer::start_logging_active_games(server.clone());

    loop {
        let (stream, _) = listener.accept().await?;

        match accept_async(stream).await {
            Ok(ws_stream) => {
                let server_clone = server.clone();
                tokio::spawn(handle_client(ws_stream, server_clone));
            }
            Err(_) => {
                continue;
            }
        }
    }
}
