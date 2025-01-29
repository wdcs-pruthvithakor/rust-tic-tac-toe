use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use rand::Rng;
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use chrono::Utc;
use log::{info, warn, error};
use env_logger;

trait UniqueId {
    fn generate_id() -> String {
        let timestamp = Utc::now().timestamp_millis();
        let mut rng = rand::thread_rng();
        let random_part: u16 = rng.gen_range(0..10000);
        format!("{}-{:04}", timestamp, random_part)
    }
}

// Struct to represent a player
#[derive(Debug, Clone)]
struct Player {
    id: String,
    name: String,
    symbol: PlayerSymbol,
    ws_sink: Arc<Mutex<futures::stream::SplitSink<WebSocketStream<TcpStream>, Message>>>,
}

impl UniqueId for Player {}

impl Player {
    fn new(name: String, symbol: PlayerSymbol, ws_sink: Arc<Mutex<futures::stream::SplitSink<WebSocketStream<TcpStream>, Message>>>) -> Self {
        Self {
            name,
            symbol,
            id: Self::generate_id(),
            ws_sink,
        }
    }

    fn set_symbol(&mut self, symbol: PlayerSymbol) {
        self.symbol = symbol;
    }
}

// Struct to represent a game
#[derive(Debug, Clone)]
struct Game {
    id: String,
    board: Vec<Option<PlayerSymbol>>,
    players: Vec<Player>,
    current_turn: usize, // Index of the current player
    status: GameStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum GameStatus {
    WaitingForPlayers,
    InProgress,
    Finished,
}

#[derive(Debug, Clone, PartialEq)]
enum PlayerSymbol {
    X,
    O
}

// Server state to manage multiple games
#[derive(Debug)]
struct GameServer {
    games: HashMap<String, Arc<Mutex<Game>>>,
    // players: HashMap<String, Player>,
}

impl UniqueId for Game {}

impl GameServer {
    fn new() -> Self {
        GameServer {
            games: HashMap::new(),
            // players: HashMap::new(),
        }
    }

    // fn register_player(&mut self, player: Player) {
    //     self.players.insert(player.id.clone(), player);
    // }

    fn create_game(&mut self, player: Player) -> String {
        let game = Game::new(player);
        let game_id = game.get_id();
        self.games.insert(game_id.clone(), Arc::new(Mutex::new(game)));
        game_id
    }

    async fn join_game(&mut self, game_id: &str, player: Player) -> Result<(), String> {
        if let Some(game_c) = self.games.get_mut(game_id) {
            let mut game = game_c.lock().await;
            let player_name = player.name.clone();
            if game.status == GameStatus::WaitingForPlayers {
                game.players.push(player);
                game.broadcast_to_players(format!("player {} has joined the game !\n", player_name)).await;
                if game.players.len() == 2 {
                    game.status = GameStatus::InProgress;
                    let game_state = game.get_game_state();
                    game.broadcast_to_players(game_state).await;
                }
                Ok(())
            } else {
                Err("Game is not accepting players".to_string())
            }
        } else {
            Err("Game not found".to_string())
        }
    }

    async fn remove_game(&mut self, game_id: &str) {
        self.games.remove(game_id);
    }
}

impl Game {
    fn new(player: Player) -> Self {
        let game_id = Self::generate_id();
        Game {
            id: game_id.clone(),
            board: vec![None; 9],
            players: vec![player],
            current_turn: 0,
            status: GameStatus::WaitingForPlayers,
        }
    }

    fn get_id(&self) -> String{
        self.id.clone()
    }

    fn make_move(&mut self, player_id: &str, position: usize) -> Result<String, String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is not in progress".to_string());
        }

        let current_player = &self.players[self.current_turn];
        if current_player.id != player_id {
            return Err("Not your turn".to_string());
        }

        if position >= 9 || self.board[position].is_some() {
            return Err("Invalid move".to_string());
        }

        self.board[position] = Some(current_player.symbol.clone());
        self.current_turn = (self.current_turn + 1) % 2;

        Ok(self.get_game_state())
    }

    fn get_status(&self) -> GameStatus {
        self.status.clone()
    }

    fn get_game_state(&mut self) -> String {
        let mut board_state = String::new();
    
        // Define ANSI color codes
        let reset = "\x1b[0m";       // Reset to default
        let light_gray = "\x1b[90m"; // Light gray for numbers
        let bold_red = "\x1b[1;31m"; // Bold red for 'X'
        let bold_blue = "\x1b[1;34m"; // Bold blue for 'O'
        let dark_line = "\x1b[1;30m═══╬═══╬═══\x1b[0m\n"; // Dark line separator
    
        // Build the 3x3 grid
        let mut some_count = 0;
        for row in 0..3 {
            for col in 0..3 {
                let index = row * 3 + col;
                if self.board[index].is_some() {
                    some_count+=1;
                }
                let symbol = match self.board[index] {
                    Some(PlayerSymbol::X) => format!("{} X {}", bold_red, reset),
                    Some(PlayerSymbol::O) => format!("{} O {}", bold_blue, reset),
                    None => format!("{} {} {}", light_gray, index + 1, reset), // Light gray for numbers
                };
                board_state.push_str(&symbol);
                if col < 2 {
                    board_state.push_str("║"); // Vertical separator
                }
            }
            board_state.push('\n'); // Move to the next row
            if row < 2 {
                board_state.push_str(&dark_line); // Dark separator
            }
        }
        if some_count == self.board.len() {
            self.status = GameStatus::Finished;
            return format!(
                "🎮 Game ID: {}\n\n{}\n🌟 Result: Draw!! 🎉",
                self.id, board_state
            )
        }  
        let current_player = &self.players[self.current_turn].name;
        match self.check_winner() {
            None => {
                format!(
                    "🎮 Game ID: {}\n\n{}\n🌟 It's {}'s turn! (Enter a number from 1 to 9)",
                    self.id, board_state, current_player
                )
            }
            Some(PlayerSymbol::X) => {
                self.status = GameStatus::Finished;
                format!(
                    "🎮 Game ID: {}\n\n{}\n🏆 Winner: {} (X) 🎉",
                    self.id, board_state, self.players[0].name
                )
            }
            Some(PlayerSymbol::O) => {
                self.status = GameStatus::Finished;
                format!(
                    "🎮 Game ID: {}\n\n{}\n🏆 Winner: {} (O) 🎉",
                    self.id, board_state, self.players[1].name
                )
            }
        }
    }

    fn check_winner(&self) -> Option<PlayerSymbol> {
        // Check rows and columns
        for i in 0..3 {
            // Rows
            if let Some(symbol) = self.board[i * 3].clone() {
                if self.board[i * 3 + 1] == Some(symbol.clone()) && self.board[i * 3 + 2] == Some(symbol.clone()) {
                    return Some(symbol.clone());
                }
            }
            // Columns
            if let Some(symbol) = self.board[i].clone() {
                // i+3*1 && i+3*2
                if self.board[i + 3] == Some(symbol.clone()) && self.board[i + 6] == Some(symbol.clone()) {
                    return Some(symbol.clone());
                }
            }
        }
    
        // Check diagonals
        if let Some(symbol) = self.board[0].clone() {
            // 0+3*1+1 && 0+3*2+2
            if self.board[4] == Some(symbol.clone()) && self.board[8] == Some(symbol.clone()) {
                return Some(symbol.clone());
            }
        }
        if let Some(symbol) = self.board[2].clone() {
            // 2+3*1-1 && 2+3*2-2
            if self.board[4] == Some(symbol.clone()) && self.board[6] == Some(symbol.clone()) {
                return Some(symbol.clone());
            }
        }
    
        None
    }
    
    async fn broadcast_to_players(&self, message: String) {
        for player in self.players.clone() {
            let mut player_ws = player.ws_sink.lock().await;
            player_ws
                .send(Message::Text(format!("📢 {}\n", message).into()))
                .await
                .unwrap();
        }
    }

    async fn broadcast_close_players(&self) {
        for player in self.players.clone() {
            let mut player_ws = player.ws_sink.lock().await;
            player_ws
                .send(Message::Close(None))
                .await
                .unwrap();
        }
    }
        
}


async fn handle_client(
    ws_stream: WebSocketStream<TcpStream>,
    server: Arc<Mutex<GameServer>>,
) {
    let (ws_sink, mut ws_stream) = ws_stream.split();
    let ws_sink = Arc::new(Mutex::new(ws_sink));

    // Request player name
    if ws_sink.lock().await
        .send(Message::Text("🎉 Welcome to Tic-Tac-Toe! Please enter your name:".to_string().into()))
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
    let player_id = player.id.clone();

    // Game selection logic
    if ws_sink.lock().await
        .send(Message::Text("🕹️ Choose an option:\n1️⃣ Create a new game\n2️⃣ Join an existing game".to_string().into()))
        .await
        .is_err()
    {
        error!("Failed to send game selection prompt for player: {player_id}.");
        return;
    }

    let mut game_id = String::new();
    match ws_stream.next().await {
        Some(Ok(Message::Text(choice))) => match choice.trim() {
            "1" => {
                let mut server = server.lock().await;
                game_id = server.create_game(player.clone());
                if ws_sink.lock().await
                    .send(Message::Text(format!("✅ Game created! Your game ID is: {}\nWaiting for another player to join...", game_id).into()))
                    .await
                    .is_err()
                {
                    error!("Failed to send game creation message for player: {player_id}.");
                    return;
                }
                info!("Player {} created game {}", name, game_id);
            }
            "2" => {
                if ws_sink.lock().await
                    .send(Message::Text("🔍 Enter the game ID to join:".to_string().into()))
                    .await
                    .is_err()
                {
                    error!("Failed to send game ID request message for player: {player_id}.");
                    return;
                }
                if let Some(Ok(Message::Text(id))) = ws_stream.next().await {
                    let mut server = server.lock().await;
                    player.set_symbol(PlayerSymbol::O);
                    match server.join_game(&id, player.clone()).await {
                        Ok(_) => {
                            game_id = id.to_string();
                            info!("Player {} joined game {}", name, game_id);
                            if ws_sink.lock().await
                                .send(Message::Text(format!("🎮 Joined game: {}", game_id).into()))
                                .await
                                .is_err()
                            {
                                error!("Failed to send join confirmation for player: {player_id} game: {game_id}.");
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = ws_sink.lock().await
                                .send(Message::Text(format!("❌ Error: {}", e).into()))
                                .await;
                            error!("Player: {player_id} Failed to join game {}: {}", id, e);
                            return;
                        }
                    }
                } else {
                    let _ = ws_sink.lock().await.send(Message::Close(None)).await;
                    return;
                }
            }
            _ => {
                let _ = ws_sink.lock().await
                    .send(Message::Text("❌ Invalid choice, please restart.".to_string().into()))
                    .await;
                error!("Invalid choice received from client {player_id}.");
                return;
            }
        }
        _ => return,
    }

    // Game loop
    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(Message::Text(text)) => {
                // Process move or input
                if let Ok(position) = text.trim().parse::<usize>() {
                    let position = position - 1;
                    let mut server = server.lock().await;
                    if let Some(game) = server.games.get_mut(&game_id) {
                        let mut game = game.lock().await;
                        match game.make_move(&player_id, position) {
                            Ok(state) => {
                                game.broadcast_to_players(state).await;
                            }
                            Err(e) => {
                                if ws_sink.lock().await
                                    .send(Message::Text(format!("❌ Error: {}", e).into()))
                                    .await
                                    .is_err()
                                {
                                    error!("Player: {player_id} Game: {game_id}, Failed to make move: {}", e);
                                    return;
                                }
                            }
                        }

                        if game.get_status() == GameStatus::Finished {
                            game.broadcast_to_players("🎉 Game over! Thanks for playing.".to_string().into()).await;
                            game.broadcast_close_players().await;
                            drop(game);
                            server.remove_game(&game_id).await;
                            return;
                        }
                    }
                } else {
                    // Invalid input
                    if ws_sink.lock().await
                        .send(Message::Text("❌ Invalid input, please enter a number from 1 to 9.".to_string().into()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
            Ok(Message::Close(_)) | Err(_) => {
                warn!("Player {} disconnected. Cleaning up...", player_id);

                // Handle disconnection
                let mut server = server.lock().await;
                if let Some(game) = server.games.get_mut(&game_id) {
                    
                    let mut game = game.lock().await;
                    game.players.retain(|p| p.id != player_id);
                    
                    // Notify remaining players
                    if !game.players.is_empty() {
                        game.broadcast_to_players("❗ A player has left the game. Game ending.".to_string()).await;
                        game.broadcast_close_players().await;
                    }
                }
                server.remove_game(&game_id).await;
                break;
            }
            _ => {}
        }
    }

    // Close connection and exit
    let _ = ws_sink.lock().await.send(Message::Close(None)).await;
    info!("Player {} has disconnected.", player_id);
}



#[tokio::main]
async fn main() {
    env_logger::init();
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    info!("WebSocket server listening on ws://127.0.0.1:8080");

    let server = Arc::new(Mutex::new(GameServer::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = accept_async(stream).await.unwrap();
        let server_clone = server.clone();
        tokio::spawn(handle_client(ws_stream, server_clone));
    }
}
