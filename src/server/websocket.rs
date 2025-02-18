// websocket.rs
use crate::game::GameStatus;
use crate::player::{Player, PlayerSymbol};
use crate::server::GameServer;
use futures::{SinkExt, StreamExt};
use log::{error, info, warn};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

// Type aliases for convenience
type WsStream = WebSocketStream<TcpStream>;
type WsSink = Arc<Mutex<futures::stream::SplitSink<WsStream, Message>>>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
struct MyCustomError(String);

impl fmt::Display for MyCustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for MyCustomError {}

// Enum to represent different game actions
#[derive(Debug)]
enum GameAction {
    Move(usize),
    Restart,
    Exit,
    Help,
    GetStatus,
    Invalid,
}

enum SessionState {
    Continue,
    Exit,
}

// Enum to represent different game messages
#[derive(Debug)]
enum GameMessage {
    Welcome,
    ChooseOption,
    GameCreated(String),
    EnterGameId,
    GameJoined(String),
    InvalidChoice,
    InvalidInput,
    WaitingForPlayers,
    GameOver,
    CantRestart,
    PlayerDisconnected(String),
    GameRestarted,
    Error(String),
    InactiveDisconnect,
    Help,
    GameStatus(String),
    // Custom(String),
}

// Struct to manage game session state
struct GameSession {
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
    player_id: String,
    game_id: String,
    player_name: String,
}

impl GameSession {
    fn new(
        ws_sink: WsSink,
        server: Arc<Mutex<GameServer>>,
        player_id: String,
        game_id: String,
        player_name: String,
    ) -> Self {
        Self {
            ws_sink,
            server,
            player_id,
            game_id,
            player_name,
        }
    }

    async fn send_game_status(&self) -> Result<()> {
        let server = self.server.lock().await;
        if let Some(game) = server.get_game(&self.game_id) {
            let mut game = game.lock().await;
            let status = game.get_game_state();
            self.send_message(GameMessage::GameStatus(status)).await?;
        } else {
            self.send_message(GameMessage::Error("Game not found".to_string()))
                .await?;
        }
        Ok(())
    }

    async fn handle_action(&self, action: GameAction) -> Result<SessionState> {
        match action {
            GameAction::Move(position) => self.handle_move(position).await,
            GameAction::Restart => self.handle_restart().await,
            GameAction::Exit => self.handle_disconnect().await,
            GameAction::Help => {
                self.send_message(GameMessage::Help).await?;
                Ok(SessionState::Continue)
            }
            GameAction::GetStatus => {
                self.send_game_status().await?;
                Ok(SessionState::Continue)
            }
            GameAction::Invalid => {
                self.send_message(GameMessage::InvalidInput).await?;
                Ok(SessionState::Continue) // Keep session active
            }
        }
    }

    async fn handle_move(&self, position: usize) -> Result<SessionState> {
        let mut server = self.server.lock().await;
        if let Some(game) = server.get_mut_game(&self.game_id) {
            let mut game = game.lock().await;

            match game.get_status() {
                GameStatus::InProgress => match game.make_move(&self.player_id, position - 1) {
                    Ok(state) => {
                        game.broadcast_to_players(state).await;
                        if game.get_status() == GameStatus::Finished {
                            game.broadcast_to_players(GameMessage::GameOver.to_string())
                                .await
                        }
                    }
                    Err(e) => self.send_message(GameMessage::Error(e.to_string())).await?,
                },
                GameStatus::WaitingForPlayers => {
                    self.send_message(GameMessage::WaitingForPlayers).await?
                }
                GameStatus::Finished => {
                    game.broadcast_to_players(GameMessage::GameOver.to_string())
                        .await
                }
            }
        }
        Ok(SessionState::Continue)
    }

    async fn handle_restart(&self) -> Result<SessionState> {
        let mut server = self.server.lock().await;
        if let Some(game) = server.get_mut_game(&self.game_id) {
            let mut game = game.lock().await;
            if game.get_status() == GameStatus::Finished {
                game.reset();
                game.broadcast_to_players(GameMessage::GameRestarted.to_string())
                    .await;
                let status = game.get_game_state();
                game.broadcast_to_players(status).await;
            } else {
                self.send_message(GameMessage::CantRestart).await?;
            }
        }
        Ok(SessionState::Continue)
    }

    async fn handle_disconnect(&self) -> Result<SessionState> {
        let mut server = self.server.lock().await;
        if let Some(game) = server.get_mut_game(&self.game_id) {
            let mut game = game.lock().await;
            game.players.retain(|p| p.get_id() != self.player_id);
            game.broadcast_to_players(
                GameMessage::PlayerDisconnected(self.player_name.clone()).to_string(),
            )
            .await;
            warn!("Player {} disconnected.", self.player_name);
            game.reset();
            if game.players.is_empty() {
                drop(game);
                server.remove_game(&self.game_id).await;
            }
        }
        Ok(SessionState::Exit)
    }

    async fn send_message(&self, message: GameMessage) -> Result<()> {
        let message_text = message.to_string();
        self.ws_sink
            .lock()
            .await
            .send(Message::Text(message_text.into()))
            .await
            .map_err(|e| e.into())
    }

    async fn is_game_in_progress(&self) -> bool {
        let server = self.server.lock().await;
        if let Some(game) = server.get_game(&self.game_id) {
            let game = game.lock().await;
            if game.get_status() == GameStatus::InProgress {
                return true;
            }
        }
        false
    }

    async fn get_current_turn_player(&self) -> Option<String> {
        let server = self.server.lock().await;
        if let Some(game) = server.get_game(&self.game_id) {
            let game = game.lock().await;
            return game.get_current_turn_player();
        }
        None
    }
}

// Implementation for GameMessage to convert to string
impl ToString for GameMessage {
    fn to_string(&self) -> String {
        match self {
            GameMessage::Welcome => "🎉 Welcome to Tic-Tac-Toe! Please enter your name:".into(),
            GameMessage::ChooseOption => "🕹️ Choose an option:\n1️⃣ Create a new game\n2️⃣ Join an existing game".into(),
            GameMessage::GameCreated(id) => format!("✅ Game created! Your game ID is: {}\nWaiting for another player to join...", id),
            GameMessage::EnterGameId => "🔍 Enter the game ID to join:".into(),
            GameMessage::GameJoined(id) => format!("🎮 Joined game: {}", id),
            GameMessage::InvalidChoice => "❌ Invalid choice, please restart.".into(),
            GameMessage::InvalidInput => "❌ Invalid Input: Enter a number from 1 to 9".into(),
            GameMessage::WaitingForPlayers => "⏳ Waiting for players to join the game...".into(),
            GameMessage::GameOver => "🎉 Game over! Type `RESTART` to play again or `EXIT` to leave.".into(),
            GameMessage::CantRestart => "❌ Error: You can't restart game before finishing current game ❗".into(),
            GameMessage::PlayerDisconnected(name) => format!("❗ Player {} has left the game. ⏳ Waiting for a new player...", name),
            GameMessage::GameRestarted => "🔄 Game restarted!".into(),
            GameMessage::Error(e) => format!("❌ Error: {}", e),
            GameMessage::InactiveDisconnect => "❗ Disconnected due to inactivity ⏰".into(),
            GameMessage::Help => "🆘 Available Commands:\n- Enter 1-9 to make a move\n- Type 'restart' to restart the game\n- Type 'exit' to leave\n- Type 'status' to check the game status".into(),
            GameMessage::GameStatus(status) => format!("📊 Game Status: {}", status),

            // GameMessage::Custom(msg) => msg.clone(),
        }
    }
}

// Main client handler
pub async fn handle_client(ws_stream: WsStream, server: Arc<Mutex<GameServer>>) {
    let (ws_sink, mut ws_stream) = ws_stream.split();
    let ws_sink = Arc::new(Mutex::new(ws_sink));

    match handle_connection(&mut ws_stream, ws_sink, server).await {
        Ok(_) => info!("Client connection handled successfully"),
        Err(e) => error!("Error handling client connection: {}", e),
    }
}

async fn handle_connection(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
) -> Result<()> {
    // Initial setup
    let (name, player_id, game_id) =
        match setup_player(ws_stream, ws_sink.clone(), server.clone()).await {
            Ok((name, player_id, game_id)) => (name, player_id, game_id),
            Err(_) => return Ok(()),
        };

    // Create game session
    let session = GameSession::new(
        ws_sink.clone(),
        server.clone(),
        player_id,
        game_id,
        name.clone(),
    );

    // Main game loop
    handle_game_loop(ws_stream, &session).await
}

async fn handle_game_setup(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    player: &mut Player,
    server: Arc<Mutex<GameServer>>,
) -> Result<String> {
    send_message(&ws_sink, GameMessage::ChooseOption).await?;

    match ws_stream.next().await {
        Some(Ok(Message::Text(choice))) => match choice.trim() {
            "1" => match create_new_game(ws_sink, player, server).await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            },
            "2" => match join_existing_game(ws_stream, ws_sink, player, server).await {
                Ok(message) => Ok(message),
                Err(e) => Err(e),
            },
            _ => {
                send_message(&ws_sink, GameMessage::InvalidChoice).await?;
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

    send_message(&ws_sink, GameMessage::GameCreated(game_id.clone())).await?;

    info!("Player {} created game {}", player.get_name(), game_id);
    Ok(game_id)
}

async fn join_existing_game(
    _ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    player: &mut Player,
    server: Arc<Mutex<GameServer>>,
) -> Result<String> {
    send_message(&ws_sink, GameMessage::EnterGameId).await?;

    // let game_id = match ws_stream.next().await {
    //     Some(Ok(Message::Text(id))) => id,
    //     _ => {
    //         send_message(
    //             &ws_sink,
    //             GameMessage::Error("Invalid game ID recieved".to_string()),
    //         )
    //         .await?;
    //         return Err("Invalid game ID received".into());
    //     }
    // };

    player.set_symbol(PlayerSymbol::O);
    let mut server = server.lock().await;
    let game_id = match server.join_game(player.clone()).await {
        Ok(id) => {
            send_message(&ws_sink, GameMessage::GameJoined(id.to_string())).await?;
            id
        },
        Err(e) => {
            send_message(&ws_sink, GameMessage::Error(e.clone())).await?;
            return Err(Box::new(MyCustomError(e)));
        }
    };

    info!("Player {} joined game {}", player.get_name(), game_id);
    Ok(game_id.to_string())
}

async fn setup_player(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
    server: Arc<Mutex<GameServer>>,
) -> Result<(String, String, String)> {
    // Get player name
    let name = get_player_name(ws_stream, ws_sink.clone()).await?;
    info!("Player {} connected.", name);

    // Create player
    let mut player = Player::new(name.clone(), PlayerSymbol::X, ws_sink.clone());
    let player_id = player.get_id();

    // Handle game setup
    let game_id = match handle_game_setup(ws_stream, ws_sink, &mut player, server).await {
        Ok(game_id) => game_id,
        Err(err) => return Err(err),
    };

    Ok((name, player_id, game_id))
}

// Existing helper functions remain the same but use GameMessage enum
async fn get_player_name(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    ws_sink: WsSink,
) -> Result<String> {
    send_message(&ws_sink, GameMessage::Welcome).await?;

    match ws_stream.next().await {
        Some(Ok(Message::Text(name))) => Ok(name.to_string()),
        _ => {
            let _ = ws_sink.lock().await.send(Message::Close(None)).await;
            Err("Failed to receive player name".into())
        }
    }
}

async fn handle_game_loop(
    ws_stream: &mut futures::stream::SplitStream<WsStream>,
    session: &GameSession,
) -> Result<()> {
    'game_loop: while let Some(message) = {
        if session.is_game_in_progress().await
            && session.get_current_turn_player().await == Some(session.player_id.clone())
        {
            match timeout(Duration::from_secs(30), ws_stream.next()).await {
                Ok(Some(msg_result)) => Some(msg_result),
                Ok(None) => continue 'game_loop,
                Err(_) => {
                    session
                        .send_message(GameMessage::InactiveDisconnect)
                        .await?;
                    session.handle_disconnect().await?;
                    return Ok(());
                }
            }
        } else {
            match timeout(Duration::from_secs(10), ws_stream.next()).await {
                Ok(Some(msg_result)) => Some(msg_result),
                _ => continue 'game_loop,
            }
        }
    } {
        match message {
            Ok(Message::Text(text)) => {
                let action = parse_game_action(&text);
                match session.handle_action(action).await {
                    Ok(SessionState::Exit) => break,
                    _ => continue,
                }
            }
            Ok(Message::Close(_)) | Err(_) => {
                session.handle_disconnect().await?;
                return Ok(());
            }
            _ => continue,
        }
    }
    Ok(())
}

fn parse_game_action(text: &str) -> GameAction {
    match text.trim().to_lowercase().as_str() {
        "exit" => GameAction::Exit,
        "restart" => GameAction::Restart,
        "help" => GameAction::Help,
        "status" => GameAction::GetStatus,
        text => match text.parse::<usize>() {
            Ok(position) if (1..=9).contains(&position) => GameAction::Move(position),
            _ => GameAction::Invalid,
        },
    }
}

async fn send_message(ws_sink: &WsSink, message: GameMessage) -> Result<()> {
    ws_sink
        .lock()
        .await
        .send(Message::Text(message.to_string().into()))
        .await
        .map_err(|e| e.into())
}
