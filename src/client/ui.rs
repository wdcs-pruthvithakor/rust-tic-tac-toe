
// // Note: This part is not implemented
// use eframe::egui;
// use futures::{SinkExt, StreamExt};
// use tokio::sync::mpsc;
// use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
// use std::sync::{Arc, Mutex};

// // Game state to track UI flow
// #[derive(PartialEq)]
// enum GameState {
//     EnteringName,
//     ChoosingGameMode,
//     EnteringGameId,
//     WaitingForPlayers,
//     Playing,
// }

// struct TicTacToeApp {
//     game_state: GameState,
//     player_name: String,
//     board: Vec<Vec<String>>,
//     game_id: String,
//     board_state: String,
//     current_turn: String,
//     message_log: Vec<String>,
//     tx: mpsc::UnboundedSender<String>,
//     last_message: Arc<Mutex<String>>,
// }

// fn strip_ansi_escape_codes(input: &str) -> String {
//     let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*[mGKF]|\x1b\[[0-9]+[ABCD]").unwrap();
//     ansi_regex.replace_all(input, "").to_string()
// }


// impl TicTacToeApp {
//     fn new(tx: mpsc::UnboundedSender<String>, last_message: Arc<Mutex<String>>) -> Self {
//         Self {
//             game_state: GameState::EnteringName,
//             player_name: String::new(),
//             game_id: String::new(),
//             board_state: String::new(),
//             board: Vec::new(),
//             message_log: Vec::new(),
//             current_turn: String::new(),
//             tx,
//             last_message,
//         }
//     }

//     fn handle_server_message(&mut self, message: &str) {
//         self.message_log.push(message.to_string());
//         let cleaned_message = strip_ansi_escape_codes(message);

//         if cleaned_message.contains("Welcome") {
//             self.game_state = GameState::EnteringName;
//         } else if cleaned_message.contains("Choose an option") {
//             self.game_state = GameState::ChoosingGameMode;
//         } else if cleaned_message.contains("Enter the game ID") {
//             self.game_state = GameState::EnteringGameId;
//         } else if cleaned_message.contains("Waiting for another player") {
//             self.game_state = GameState::WaitingForPlayers;
//         } else if cleaned_message.contains("║") {
//             // Parse the board state from the cleaned message
//             let board_lines: Vec<&str> = cleaned_message
//                 .split('\n')
//                 .filter(|line| line.contains("║"))
//                 .collect();
    
//             // Convert the board lines into a 2D array (3x3)
//             self.board = board_lines.iter().map(|&line| {
//                 line.split("║")
//                     .map(|cell| cell.trim().to_string())
//                     .collect::<Vec<String>>()
//             }).collect::<Vec<Vec<String>>>();
    
//             // Extract turn information: "It's sds's turn!"
//             if let Some(turn_info) = cleaned_message.split("It's").last() {
//                 self.current_turn = turn_info.split("'s turn").next().unwrap_or("Unknown").to_string();
//             }
    
//             self.game_state = GameState::Playing;
//         }
        
//     }
// }

// impl eframe::App for TicTacToeApp {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         // Check for new messages from the server
//         let mut message_temp = String::new();
//         if let Ok(message) = self.last_message.try_lock() {
//             message_temp = message.clone();
//         }
//         if !message_temp.is_empty() {
//             self.handle_server_message(&message_temp);
//         }
        
//         egui::CentralPanel::default().show(ctx, |ui| {
//             // Message log area at the top
//             egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
//                 for message in &self.message_log {
//                     ui.label(message);
//                 }
//             });

//             ui.add_space(20.0);

//             match self.game_state {
//                 GameState::EnteringName => {
//                     ui.heading("Enter Your Name");
//                     let name_response = ui.text_edit_singleline(&mut self.player_name);
//                     if name_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
//                         if !self.player_name.is_empty() {
//                             self.tx.send(self.player_name.clone()).unwrap();
//                         }
//                     }
//                 }
//                 GameState::ChoosingGameMode => {
//                     ui.heading("Choose Game Mode");
//                     if ui.button("Create New Game").clicked() {
//                         self.tx.send("1".to_string()).unwrap();
//                     }
//                     if ui.button("Join Existing Game").clicked() {
//                         self.tx.send("2".to_string()).unwrap();
//                     }
//                 }
//                 GameState::EnteringGameId => {
//                     ui.heading("Enter Game ID");
//                     let id_response = ui.text_edit_singleline(&mut self.game_id);
//                     if id_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
//                         if !self.game_id.is_empty() {
//                             self.tx.send(self.game_id.clone()).unwrap();
//                         }
//                     }
//                 }
//                 GameState::WaitingForPlayers => {
//                     ui.heading("Waiting for Players");
//                     ui.spinner();
//                 }
//                 GameState::Playing => {
//                     if !self.board.is_empty() {
//                         ui.heading("Tic-tac-toe Game");
                
//                         // Display the game board
//                         ui.horizontal_wrapped(|ui| {
//                             for i in 0..3 {
//                                 ui.vertical(|ui| {
//                                     for j in 0..3 {
//                                         let cell_content = &self.board[i][j];
//                                         if cell_content == "X" || cell_content == "O" {
//                                             // Display X or O
//                                             ui.label(cell_content.clone());
//                                         } else {
//                                             // Display number for empty cells and make them clickable
//                                             let cell_number = (i * 3 + j + 1).to_string();
//                                             if ui.button(cell_number.clone()).clicked() {
//                                                 self.tx.send(cell_number).unwrap(); // Send the move
//                                             }
//                                         }
//                                     }
//                                 });
//                             }
//                         });
                
//                         // Show whose turn it is
//                         ui.label(format!("It's {}'s turn!", self.current_turn));
                
//                         // Game controls
//                         ui.horizontal(|ui| {
//                             if ui.button("Restart").clicked() {
//                                 self.tx.send("restart".to_string()).unwrap();
//                             }
//                             if ui.button("Exit").clicked() {
//                                 self.tx.send("exit".to_string()).unwrap();
//                             }
//                         });
//                     }
//                 }
                
                
//             }
//         });

//         // Request a repaint frequently to update the UI with new messages
//         ctx.request_repaint();
//     }
// }

// #[tokio::main]
// async fn main() -> Result<(), eframe::Error> {
//     // Set up WebSocket connection
//     let (ws_stream, _) = connect_async("ws://127.0.0.1:8080")
//         .await
//         .expect("Failed to connect to WebSocket server");
//     let (mut ws_write, mut ws_read) = ws_stream.split();

//     // Channel for sending messages to the WebSocket
//     let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    
//     // Shared state for the last received message
//     let last_message = Arc::new(Mutex::new(String::new()));
//     let last_message_clone = last_message.clone();

//     // Task to send messages to the server
//     let send_task = tokio::spawn(async move {
//         while let Some(message) = rx.recv().await {
//             if ws_write.send(Message::Text(message.into())).await.is_err() {
//                 break;
//             }
//         }
//     });

//     // Task to receive messages from the server
//     let receive_task = tokio::spawn(async move {
//         while let Some(msg) = ws_read.next().await {
//             if let Ok(Message::Text(text)) = msg {
//                 if let Ok(mut last_msg) = last_message_clone.lock() {
//                     *last_msg = text.to_string();
//                 }
//             }
//         }
//     });

//     // Set up the native options
//     let native_options = eframe::NativeOptions {
//         initial_window_size: Some(egui::vec2(800.0, 600.0)),
//         ..Default::default()
//     };

//     // Run the egui application
//     eframe::run_native(
//         "Tic-tac-toe",
//         native_options,
//         Box::new(|_cc| Box::new(TicTacToeApp::new(tx, last_message)))
//     )?;

//     Ok(())
// }