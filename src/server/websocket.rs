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

    pub async fn handle_client(ws_stream: WebSocketStream<TcpStream>, server: Arc<Mutex<GameServer>>) {
        let (ws_sink, mut ws_stream) = ws_stream.split();
        let ws_sink = Arc::new(Mutex::new(ws_sink));

        // Request player name
        if ws_sink
            .lock()
            .await
            .send(Message::Text(
                "🎉 Welcome to Tic-Tac-Toe! Please enter your name:"
                    .to_string()
                    .into(),
            ))
            .await
            .is_err()
        {
            error!("Failed to send initial message to client.");
            return;
        }

        let name = match ws_stream.next().await {
            Some(Ok(Message::Text(name))) => name,
            _ => {
                let _ = ws_sink.lock().await.send(Message::Close(None)).await;
                error!("Failed to receive player name.");
                return;
            }
        };
        info!("Player {} connected.", name);
        let mut player = Player::new(name.to_string().clone(), PlayerSymbol::X, ws_sink.clone());
        let player_id = player.get_id();

        // Game selection logic
        if ws_sink
            .lock()
            .await
            .send(Message::Text(
                "🕹️ Choose an option:\n1️⃣ Create a new game\n2️⃣ Join an existing game"
                    .to_string()
                    .into(),
            ))
            .await
            .is_err()
        {
            error!("Failed to send game selection prompt for player: {player_id}.");
            return;
        }
        let choice = match ws_stream.next().await {
            Some(Ok(Message::Text(choice))) => choice,
            _ => return, //Handle error
        };

        let game_id = match choice.trim() { // game_id declaration moved inside match
            "1" => {
                let mut server = server.lock().await;
                let game_id = server.create_game(player); // game_id assigned here
                if ws_sink.lock().await
                    .send(Message::Text(format!("✅ Game created! Your game ID is: {}\nWaiting for another player to join...", game_id).into()))
                    .await
                    .is_err()
                {
                    error!("Failed to send game creation message for player: {player_id}.");
                    return;
                }
                info!("Player {} created game {}", name, game_id);
                game_id // Return game_id from this branch
            }
            "2" => {
                if ws_sink
                    .lock()
                    .await
                    .send(Message::Text(
                        "🔍 Enter the game ID to join:".to_string().into(),
                    ))
                    .await
                    .is_err()
                {
                    error!("Failed to send game ID request message for player: {player_id}.");
                    return;
                }
                if let Some(Ok(Message::Text(id))) = ws_stream.next().await {
                    let mut server = server.lock().await;
                    player.set_symbol(PlayerSymbol::O);
                    match server.join_game(&id, player).await {
                        Ok(_) => {
                            let game_id = id.to_string(); // game_id assigned only on success
                            info!("Player {} joined game {}", name, game_id);
                            if ws_sink
                                .lock()
                                .await
                                .send(Message::Text(format!("🎮 Joined game: {}", game_id).into()))
                                .await
                                .is_err()
                            {
                                error!("Failed to send join confirmation for player: {player_id} game: {game_id}.");
                                return;
                            }
                            game_id // Return game_id from this branch
                        }
                        Err(e) => {
                            let _ = ws_sink
                                .lock()
                                .await
                                .send(Message::Text(format!("❌ Error: {}", e).into()))
                                .await;
                            error!("Player: {player_id} Failed to join game {}: {}", id, e);
                            return; // Early return if join fails
                        }
                    }
                } else {
                    let _ = ws_sink.lock().await.send(Message::Close(None)).await;
                    return; //Handle error
                }
            }
            _ => {
                let _ = ws_sink
                    .lock()
                    .await
                    .send(Message::Text(
                        "❌ Invalid choice, please restart.".to_string().into(),
                    ))
                    .await;
                error!("Invalid choice received from client {player_id}.");
                return;
            }
        }; 

        // Game loop
        while let Some(message) = ws_stream.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if text.trim().to_lowercase() == "exit" {
                        let mut server = server.lock().await;
                        if let Some(game) = server.get_mut_game(&game_id) {
                            let mut game = game.lock().await;
                            game.players.retain(|p| p.get_id() != player_id);
                            game.broadcast_to_players(format!(
                                "❗ Player {} has left the game. ⏳ Waiting for a new player...",
                                name
                            ))
                            .await;
                            info!("Player {} disconnected.", name);
                            game.reset();
                            if game.players.is_empty() {
                                drop(game);
                                server.remove_game(&game_id).await;
                            }
                        }
                        return;
                    }

                    if let Ok(position) = text.trim().parse::<usize>() {
                        let position = position - 1;
                        let mut server = server.lock().await;
                        if let Some(game) = server.get_mut_game(&game_id) {
                            let mut game = game.lock().await;
                            if game.get_status() == GameStatus::InProgress {
                                match game.make_move(&player_id, position) {
                                    Ok(state) => {
                                        game.broadcast_to_players(state).await;
                                    }
                                    Err(e) => {
                                        if ws_sink
                                            .lock()
                                            .await
                                            .send(Message::Text(format!("❌ Error: {}", e).into()))
                                            .await
                                            .is_err()
                                        {
                                            error!("Player: {player_id} Game: {game_id}, Failed to make move: {}", e);
                                            return;
                                        }
                                    }
                                }
                            } else if game.get_status() == GameStatus::WaitingForPlayers {
                                let _ = ws_sink
                                    .lock()
                                    .await
                                    .send(Message::Text(
                                        "⏳ Waiting for players to join the game..."
                                            .to_string()
                                            .into(),
                                    ))
                                    .await;
                            }
                            if game.get_status() == GameStatus::Finished {
                                game.broadcast_to_players(
                                    "🎉 Game over! Type `RESTART` to play again or `EXIT` to leave."
                                        .to_string(),
                                )
                                .await;
                            }
                        }
                    } else if text.trim().to_lowercase() == "restart" {
                        let mut server = server.lock().await;
                        if let Some(game) = server.get_mut_game(&game_id) {
                            let mut game = game.lock().await;
                            if game.get_status() == GameStatus::Finished {
                                game.reset();
                                game.broadcast_to_players("🔄 Game restarted!".to_string())
                                    .await;
                                let status = game.get_game_state();
                                game.broadcast_to_players(status).await;
                            } else {
                                let _ = ws_sink.lock().await
                                    .send(Message::Text("❌ Error: You can't restart game before finishing current game ❗".to_string().into()))
                                    .await;
                            }
                        }
                    } else {
                        let _ = ws_sink
                            .lock()
                            .await
                            .send(Message::Text(
                                "❌ Invalid Input: Enter a number from 1 to 9"
                                    .to_string()
                                    .into(),
                            ))
                            .await;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => {
                    let mut server = server.lock().await;
                    if let Some(game) = server.get_mut_game(&game_id) {
                        let mut game = game.lock().await;
                        game.players.retain(|p| p.get_id() != player_id);
                        game.broadcast_to_players(format!(
                            "❗ Player {} disconnected. ⏳ Waiting for a new player...",
                            name
                        ))
                        .await;
                        warn!("Player {} disconnected.", name);
                        game.reset();
                        if game.players.is_empty() {
                            drop(game);
                            server.remove_game(&game_id).await;
                        }
                    }
                    return;
                }
                _ => {}
            }
        }
    }
