use crate::player::{Player, PlayerSymbol};
use futures::SinkExt;
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Debug, Clone, PartialEq)]
pub enum GameStatus {
    WaitingForPlayers,
    InProgress,
    Finished,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub id: String,
    pub board: Vec<Option<PlayerSymbol>>,
    pub players: Vec<Player>,
    pub current_turn: usize,
    pub status: GameStatus,
}

impl Game {
    pub fn new(player: Player) -> Self {
        let game_id = crate::utils::generate_id();
        Game {
            id: game_id,
            board: vec![None; 9],
            players: vec![player],
            current_turn: 0,
            status: GameStatus::WaitingForPlayers,
        }
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn get_status(&self) -> GameStatus {
        self.status.clone()
    }

    pub fn set_status(&mut self, status: GameStatus) {
        self.status = status
    }

    pub fn get_players(&self) -> Vec<Player> {
        self.players.clone()
    }

    pub fn add_player(&mut self, player: Player) {
        self.players.push(player);
    }

    pub fn reset(&mut self) {
        self.board = vec![None; 9];
        self.current_turn = 0;
        if self.players.len() == 2 {
            self.status = GameStatus::InProgress;
        } else {
            self.status = GameStatus::WaitingForPlayers;
        }
    }

    pub fn make_move(&mut self, player_id: &str, position: usize) -> Result<String, String> {
        if self.status != GameStatus::InProgress {
            return Err("Game is not in progress".to_string());
        }

        let current_player = &self.players[self.current_turn];
        if current_player.get_id() != player_id {
            return Err("Not your turn".to_string());
        }

        if position >= 9 || self.board[position].is_some() {
            return Err("Invalid move".to_string());
        }

        self.board[position] = Some(current_player.get_symbol());
        self.current_turn = (self.current_turn + 1) % 2;

        Ok(self.get_game_state())
    }

    pub fn get_game_state(&mut self) -> String {
        let mut board_state = String::new();

        // Define ANSI color codes
        let reset = "\x1b[0m"; // Reset to default
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
                    some_count += 1;
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
                board_state.push_str(dark_line); // Dark separator
            }
        }
        if some_count == self.board.len() {
            self.status = GameStatus::Finished;
            return format!(
                "🎮 Game ID: {}\n\n{}\n🌟 Result: Draw!! 🎉",
                self.id, board_state
            );
        }
        let current_player = &self.players[self.current_turn].get_name();
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
                    self.id,
                    board_state,
                    self.players[0].get_name()
                )
            }
            Some(PlayerSymbol::O) => {
                self.status = GameStatus::Finished;
                format!(
                    "🎮 Game ID: {}\n\n{}\n🏆 Winner: {} (O) 🎉",
                    self.id,
                    board_state,
                    self.players[1].get_name()
                )
            }
        }
    }

    fn check_winner(&self) -> Option<PlayerSymbol> {
        // Check rows and columns
        for i in 0..3 {
            // Rows
            if let Some(symbol) = self.board[i * 3].clone() {
                if self.board[i * 3 + 1] == Some(symbol.clone())
                    && self.board[i * 3 + 2] == Some(symbol.clone())
                {
                    return Some(symbol.clone());
                }
            }
            // Columns
            if let Some(symbol) = self.board[i].clone() {
                // i+3*1 && i+3*2
                if self.board[i + 3] == Some(symbol.clone())
                    && self.board[i + 6] == Some(symbol.clone())
                {
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

    pub async fn broadcast_to_players(&self, message: String) {
        for player in self.players.clone() {
            let ws_sink = player.get_ws_sink();
            let mut player_ws = ws_sink.lock().await;
            player_ws
                .send(Message::Text(format!("📢 {}\n", message).into()))
                .await
                .unwrap();
        }
    }

    // async fn broadcast_close_players(&self) {
    //     for player in self.players.clone() {
    //         let mut player_ws = player.ws_sink.lock().await;
    //         player_ws
    //             .send(Message::Close(None))
    //             .await
    //             .unwrap();
    //     }
    // }
}
