// websocket.rs
use crate::game::GameStatus;
use crate::player::{Player, PlayerSymbol};
use crate::server::GameServer;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

type WsStream = WebSocketStream<TcpStream>;
type WsSink = Arc<Mutex<futures::stream::SplitSink<WsStream, Message>>>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn handle_client(ws_stream: WsStream, server: Arc<Mutex<GameServer>>) {
    let (ws_sink, mut ws_stream) = ws_stream.split();
    let ws_sink = Arc::new(Mutex::new(ws_sink));

    match handle_connection(&mut ws_stream, ws_sink.clone(), server).await {
        Ok(_) => info!("Client connection handled successfully"),
        Err(e) => error!("Error handling client connection: {}", e),
    }
}

async fn handle_connection(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
) -> Result<()> {
    // Get player name
    let name = get_player_name(ws_stream, ws_sink.clone()).await?;
    info!("Player {} connected.", name);

    // Create player
    let mut player = Player::new(name.clone(), PlayerSymbol::X, ws_sink.clone());
    let player_id = player.get_id();

    // Handle game setup
    let game_id = handle_game_setup(ws_stream, ws_sink.clone(), &mut player, server.clone()).await?;

    // Main game loop
    handle_game_loop(
        ws_stream,
        ws_sink,
        server,
        &player_id,
        &game_id,
        &name,
    )
    .await
}

async fn get_player_name(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
) -> Result<String> {
    send_message(&ws_sink, "🎉 Welcome to Tic-Tac-Toe! Please enter your name:").await?;

    match ws_stream.next().await {
        Some(Ok(Message::Text(name))) => Ok(name.to_string()),
        _ => {
            let _ = ws_sink.lock().await.send(Message::Close(None)).await;
            Err("Failed to receive player name".into())
        }
    }
}

async fn handle_game_setup(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    player: &mut Player,
    server: Arc<Mutex<GameServer>>,
) -> Result<String> {
    send_message(
        &ws_sink,
        "🕹️ Choose an option:\n1️⃣ Create a new game\n2️⃣ Join an existing game",
    )
    .await?;

    match ws_stream.next().await {
        Some(Ok(Message::Text(choice))) => match choice.trim() {
            "1" => create_new_game(ws_sink, player, server).await,
            "2" => join_existing_game(ws_stream, ws_sink, player, server).await,
            _ => {
                send_message(&ws_sink, "❌ Invalid choice, please restart.").await?;
                Err("Invalid choice received from client".into())
            }
        },
        _ => Err("Invalid message format received".into()),
    }
}

async fn create_new_game(
    ws_sink: WsSink,
    player: &Player,
    server: Arc<Mutex<GameServer>>,
) -> Result<String> {
    let mut server = server.lock().await;
    let game_id = server.create_game(player.clone());
    
    send_message(
        &ws_sink,
        &format!(
            "✅ Game created! Your game ID is: {}\nWaiting for another player to join...",
            game_id
        ),
    )
    .await?;
    
    info!("Player {} created game {}", player.get_name(), game_id);
    Ok(game_id)
}

async fn join_existing_game(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    player: &mut Player,
    server: Arc<Mutex<GameServer>>,
) -> Result<String> {
    send_message(&ws_sink, "🔍 Enter the game ID to join:").await?;

    let game_id = match ws_stream.next().await {
        Some(Ok(Message::Text(id))) => id,
        _ => return Err("Invalid game ID received".into()),
    };

    player.set_symbol(PlayerSymbol::O);
    let mut server = server.lock().await;
    
    server.join_game(&game_id, player.clone()).await?;
    send_message(&ws_sink, &format!("🎮 Joined game: {}", game_id)).await?;
    
    info!("Player {} joined game {}", player.get_name(), game_id);
    Ok(game_id.to_string())
}

async fn handle_game_loop(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
    player_id: &str,
    game_id: &str,
    name: &str,
) -> Result<()> {
    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(Message::Text(text)) => {
                handle_text_message(
                    text.to_string(),
                    ws_sink.clone(),
                    server.clone(),
                    player_id,
                    game_id,
                    name,
                )
                .await?;
            }
            Ok(Message::Close(_)) | Err(_) => {
                handle_player_disconnect(server.clone(), game_id, player_id, name, ws_sink).await?;
                return Ok(());
            }
            _ => continue,
        }
    }
    Ok(())
}

async fn handle_text_message(
    text: String,
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
    player_id: &str,
    game_id: &str,
    name: &str,
) -> Result<()> {
    match text.trim().to_lowercase().as_str() {
        "exit" => {
            handle_player_disconnect(server, game_id, player_id, name, ws_sink).await?;
            Ok(())
        }
        "restart" => handle_game_restart(ws_sink, server, game_id).await,
        text => handle_game_move(text, ws_sink, server, player_id, game_id).await,
    }
}

async fn handle_game_move(
    text: &str,
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
    player_id: &str,
    game_id: &str,
) -> Result<()> {
    if let Ok(position) = text.parse::<usize>() {
        let position = position - 1;
        let mut server = server.lock().await;
        
        if let Some(game) = server.get_mut_game(&game_id.to_string()) {
            let mut game = game.lock().await;
            
            match game.get_status() {
                GameStatus::InProgress => {
                    match game.make_move(player_id, position) {
                        Ok(state) => game.broadcast_to_players(state).await,
                        Err(e) => send_message(&ws_sink, &format!("❌ Error: {}", e)).await?,
                    }
                }
                GameStatus::WaitingForPlayers => {
                    send_message(&ws_sink, "⏳ Waiting for players to join the game...").await?
                }
                GameStatus::Finished => {
                    game.broadcast_to_players(
                        "🎉 Game over! Type `RESTART` to play again or `EXIT` to leave.".to_string(),
                    )
                    .await
                }
            }
        }
    } else {
        send_message(&ws_sink, "❌ Invalid Input: Enter a number from 1 to 9").await?;
    }
    Ok(())
}

async fn handle_game_restart(
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
    game_id: &str,
) -> Result<()> {
    let mut server = server.lock().await;
    if let Some(game) = server.get_mut_game(&game_id.to_string()) {
        let mut game = game.lock().await;
        if game.get_status() == GameStatus::Finished {
            game.reset();
            game.broadcast_to_players("🔄 Game restarted!".to_string()).await;
            let status = game.get_game_state();
            game.broadcast_to_players(status).await;
        } else {
            send_message(
                &ws_sink,
                "❌ Error: You can't restart game before finishing current game ❗",
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_player_disconnect(
    server: Arc<Mutex<GameServer>>,
    game_id: &str,
    player_id: &str,
    name: &str,
    ws_sink: WsSink
) -> Result<()> {
    let mut server = server.lock().await;
    if let Some(game) = server.get_mut_game(&game_id.to_string()) {
        let mut game = game.lock().await;
        game.players.retain(|p| p.get_id() != player_id);
        game.broadcast_to_players(format!(
            "❗ Player {} has left the game. ⏳ Waiting for a new player...",
            name
        ))
        .await;
        warn!("Player {} disconnected.", name);
        send_close(&ws_sink).await?;

        game.reset();
        if game.players.is_empty() {
            drop(game);
            server.remove_game(game_id).await;
        }
    }
    Ok(())
}

async fn send_message(ws_sink: &WsSink, message: &str) -> Result<()> {
    ws_sink
        .lock()
        .await
        .send(Message::Text(message.to_string().into()))
        .await
        .map_err(|e| e.into())
}

async fn send_close(ws_sink: &WsSink) -> Result<()> {
    ws_sink
        .lock()
        .await
        .send(Message::Close(None))
        .await
        .map_err(|e| e.into())
}