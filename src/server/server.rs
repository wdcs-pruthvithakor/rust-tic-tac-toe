use crate::game::{Game, GameStatus};
use crate::player::Player;
use crate::player::PlayerSymbol;
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Debug)]
pub struct GameServer {
    games: HashMap<String, Arc<Mutex<Game>>>,
}

impl GameServer {
    pub fn new() -> Self {
        GameServer {
            games: HashMap::new(),
        }
    }

    pub fn get_mut_game(&mut self, id: &String) -> Option<&mut Arc<Mutex<Game>>> {
        self.games.get_mut(id)
    }
    pub fn create_game(&mut self, player: Player) -> String {
        let game = Game::new(player);
        let game_id = game.get_id();
        self.games
            .insert(game_id.clone(), Arc::new(Mutex::new(game)));
        game_id
    }

    pub async fn join_game(&mut self, game_id: &str, mut player: Player) -> Result<(), String> {
        if let Some(game_c) = self.games.get_mut(game_id) {
            let mut game = game_c.lock().await;
            let player_name = player.get_name();
            if game.get_status() == GameStatus::WaitingForPlayers {
                // game.players
                let players = game.get_players();
                if let Some(game_player) = players.first() {
                    if game_player.get_symbol() == PlayerSymbol::O {
                        player.set_symbol(PlayerSymbol::X);
                    }
                }
                game.add_player(player);
                game.broadcast_to_players(format!(
                    "player {} has joined the game !\n",
                    player_name
                ))
                .await;
                if game.get_players().len() == 2 {
                    game.set_status(GameStatus::InProgress);
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

    pub async fn remove_game(&mut self, game_id: &str) {
        self.games.remove(game_id);
    }

    pub fn start_logging_active_games(server: Arc<Mutex<Self>>) {
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(10)).await; // Log every 10 seconds

                let server = server.lock().await;
                let mut active_games = Vec::new();
                let games_len = server.games.len();
                for (game_id, game) in server.games.iter() {
                    let game = game.lock().await;
                    let players: Vec<String> = game.players.iter().map(|p| p.get_name()).collect();
                    active_games.push(format!(
                        "Game {}: Players [{}] | Status: {:?}",
                        game_id,
                        players.join(", "),
                        game.get_status()
                    ));
                }

                if !active_games.is_empty() {
                    info!(
                        "🟢 Active Games: {}\n{}",
                        games_len,
                        active_games.join("\n")
                    );
                } else {
                    info!("⚪ No active games.");
                }
            }
        });
    }
}
