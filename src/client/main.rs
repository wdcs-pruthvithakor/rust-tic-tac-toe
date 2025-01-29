use tokio::io::{self, AsyncBufReadExt};
use futures::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // Listen for Ctrl+C
    let shutdown_signal = tokio::spawn(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("Ctrl+C detected. Shutting down...");
    });

    // Connect to the WebSocket server
    let (ws_stream, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .expect("Failed to connect to WebSocket server");
    let (mut ws_write, mut ws_read) = ws_stream.split();

    // Set up a channel to handle user input
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Task to handle user input
    let input_task = tokio::spawn(async move {
        let stdin = io::stdin();
        let mut reader = io::BufReader::new(stdin);

        loop {
            let mut input = String::new();
            if reader.read_line(&mut input).await.is_err() {
                eprintln!("Failed to read user input");
                break;
            }

            if tx.send(input.trim().to_string()).is_err() {
                eprintln!("Failed to send user input to WebSocket task");
                break;
            }
        }
    });

    // Task to handle outgoing messages (user input) to the server
    let send_task = tokio::spawn(async move {
        while let Some(input) = rx.recv().await {
            if ws_write.send(Message::Text(input.into())).await.is_err() {
                eprintln!("Failed to send message to server");
                break;
            }
        }
        let _ = ws_write.send(Message::Close(None)).await;
    });

    // Task to handle incoming messages (server updates) and print to the console
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(Message::Text(text)) => println!("\n{}", text),
                Ok(Message::Close(_)) | Err(_) => {
                    println!("Connection closed by server.");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for Ctrl+C or any task to finish
    tokio::select! {
        _ = shutdown_signal => {
            println!("Ctrl+C pressed. Exiting...");
        }
        _ = input_task => {
            println!("Input task finished. Exiting...");
        }
        _ = send_task => {
            println!("Send task finished. Exiting...");
        }
        _ = receive_task => {
            println!("Receive task finished. Exiting...");
        }
    }
    println!("Client terminated.");
}
