

# Tic-Tac-Toe WebSocket Server & Client in Rust

This project implements both the **WebSocket server** and a **client** for a multiplayer Tic-Tac-Toe game. The server allows players to join or create games, make moves, restart games, and check the status of the game. The client connects to the server and lets users play the game interactively in real-time.

## Features

- **WebSocket Communication**: Real-time bi-directional communication between the server and players.
- **Game Creation**: Players can create a new Tic-Tac-Toe game and wait for another player to join.
- **Game Joining**: Players can join an existing game by entering a game ID.
- **Real-Time Gameplay**: Players can make moves in the game, and the game state is updated in real-time.
- **Game Restart**: Once a game is finished, players can choose to restart or exit.
- **Help Command**: Players can get instructions on how to play the game.
- **Game Status**: Players can check the current status of the game board.
- **Disconnect Handling**: If a player disconnects, the game handles the disconnection appropriately and broadcasts to other players.


## Project Structure

- **`main.rs`**: The entry point for both the WebSocket server and the client. This file contains the logic for setting up connections and managing communication with the client.
- **`game.rs`**: Handles the logic for managing the game state, managing players, and processing moves.
- **`player.rs`**: Defines the `Player` struct, which stores the player's name, ID, and game symbol (`X` or `O`).
- **`server.rs`**: Manages all active games, and provides functionality to create, join, and retrieve games.
- **`websocket.rs`**: Handles WebSocket connections, managing game actions and communicating with clients.

### Dependencies

- `tokio`: Asynchronous runtime for Rust, used to handle WebSocket connections and other async tasks.
- `tokio-tungstenite`: WebSocket implementation for Tokio.
- `futures`: Asynchronous utilities for working with streams and sinks.
- `log`: Logging framework to capture important events and errors.

## Getting Started

To get the server up and running on your local machine, follow these steps:

### Prerequisites

Make sure you have the following installed:

- [Rust](https://www.rust-lang.org/) (version 1.50+)
- [Cargo](https://doc.rust-lang.org/cargo/) (comes with Rust)
- [Rustup](https://rustup.rs/) (for managing Rust versions)

## Setup

### 1. Clone the repository:

```bash
git clone https://github.com/wdcs-pruthvithakor/rust-tic-tac-toe.git
cd rust-tic-tac-toe
```

### 2. Install dependencies:

The project uses `tokio` and `tokio-tungstenite`. To install dependencies, run:

```bash
cargo build
```

### 3. Run the Server:

To start the WebSocket server, run the following command in one terminal:

```bash
RUST_LOG=info cargo run --bin server
```

The server will start listening on `ws://127.0.0.1:8080`. It waits for incoming WebSocket connections from clients.

### 4. Run the Client:

To run the client (in a different terminal), use:

```bash
cargo run --bin client
```

The client will connect to the server, and the user will be prompted to either create or join a game.

## How to Play

1. **Start a Game**: After connecting, players are prompted to enter their name. Once the name is entered, the player is presented with two options:
   - Create a new game
   - Join an existing game by providing a game ID

2. **Making Moves**: Players can make a move by entering a number from 1 to 9, corresponding to the positions on the Tic-Tac-Toe board. Players take turns making moves, and the game state is updated in real-time.

3. **Game Over**: Once a game is finished, players can choose to restart the game or exit. If the game is restarted, the board is reset.

4. **Help**: Players can type `help` at any time to see the available commands and gameplay instructions.

5. **Exit**: Players can exit the game at any time by typing `exit`. 

## Game Actions

Here are the available actions players can take during a game:

- **Move**: Players enter a number from 1 to 9 to make their move.
- **Restart**: Type `restart` to restart the current game after it has finished.
- **Exit**: Type `exit` to leave the game and disconnect.
- **Help**: Type `help` to get a list of commands and instructions.
- **Status**: Type `status` to check the current game status.

### Game Flow Example

1. Player connects to the server and enters their name.
2. Player chooses to create a new game.
3. The game waits for the second player to join.
4. Second player joins the game by entering the game ID.
5. Players take turns making moves.
6. Once a player wins or the game ends in a draw, the game is over.
7. Players are given the option to restart or exit.

## Server Implementation Details

- The game server (`GameServer`) handles the creation, management, and removal of games.
- The game state (`Game`) tracks the board, players, and game status.
- Players are managed in the `Player` struct, which holds player information and their WebSocket connection.
- WebSocket messages are handled by the `websocket.rs` file, where different actions and events are processed asynchronously.

### Handling Game Logic

The game logic involves several key steps:

1. **Game Creation**: When a player creates a game, the server assigns a unique game ID and waits for another player to join.
2. **Player Moves**: Players take turns, and the server verifies the move, updates the game board, and broadcasts the new state to all players.
3. **Game End**: The game ends when there is a winner or the game reaches a draw. Players are notified, and the option to restart is presented.

### Handling WebSocket Communication

- The server uses `tokio-tungstenite` to establish WebSocket connections.
- Players send commands to the server, which are parsed and processed based on the current game state.
- The server responds with messages regarding game status, player moves, and other interactions.

### Inactivity
If no actions are received for a certain time (e.g., 30 seconds for a player's turn), the player will be disconnected due to inactivity.

### Player Disconnection
If a player disconnects, they will be removed from the game and a message will be broadcasted to all other players in the game.


### **Server Logs**  

The Tic-Tac-Toe WebSocket server uses the `log` crate for structured logging. Logs help track server activity, player actions, and errors.  

#### **Log Levels**  
- `info!` → General server events (e.g., connections, moves, game results).  
- `warn!` → Potential issues (e.g., invalid moves, disconnections).  
- `error!` → Critical failures (e.g., WebSocket errors).  

#### **Examples**  
```rust
info!("New game created: {}", game_id);
warn!("Player {} disconnected.", player_id);
error!("WebSocket error: {:?}", err);
```

#### **Log Output & Storage**  
By default, logs are printed to the console. To save logs to a file:  
```bash
RUST_LOG=info cargo run --bin server 2>&1 | tee server.log
```

For advanced logging, use `fern` or `env_logger`.  

## Example of WebSocket Interaction

Here’s an example of how a WebSocket interaction might look like in the console:

```
Welcome to Tic-Tac-Toe! Please enter your name:
> Alice

🎉 Welcome, Alice! Choose an option:
1️⃣ Create a new game
2️⃣ Join an existing game

> 1

✅ Game created! Your game ID is: abc123
Waiting for another player to join...

[Another player joins...]

🎮 Joined game: abc123
Your turn! Enter a number (1-9):

> 5

[Game updates and other messages follow...]

🎉 Game over! Type `RESTART` to play again or `EXIT` to leave.
```

### Game Status

When the player requests the game status, the server will respond with a visual representation of the Tic-Tac-Toe board and the game’s state.

### Sample Status Output

```plaintext
📊 Game Status: 🎮 Game ID: 1738331017305-4141

 1 ║ X ║ 3 
═══╬═══╬═══
 4 ║ O ║ 6 
═══╬═══╬═══
 7 ║ 8 ║ 9 

🌟 It's a's turn! (Enter a number from 1 to 9)
```

- The **board** will be displayed in a grid format with numbers representing empty spaces and `X` and `O` representing player moves.
- The **game ID** is shown at the top to identify the specific game session.
- The **current player** (e.g., `a` or `b`) is indicated, and it will prompt the player to enter a number corresponding to the next available spot on the board.

### Example Flow

1. A player enters the `status` command.
2. The server responds with the board layout and information about whose turn it is.


## Error Handling

- If a player tries to make a move out of turn or in an invalid state, they will receive an error message from the server.
- If a player attempts to restart a game that hasn’t finished, the server will return an error message indicating that the game cannot be restarted yet.
- Disconnects are handled gracefully, and the server will remove any games that no longer have players.

## Troubleshooting

- **Issue**: The server doesn’t start or builds incorrectly.
  - **Solution**: Ensure that you have the required dependencies installed by running `cargo build`.

- **Issue**: WebSocket connection issues.
  - **Solution**: Ensure that you are connecting to `ws://127.0.0.1:8080`. If you are using a front-end client, verify the WebSocket URL.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
