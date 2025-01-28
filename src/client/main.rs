use tokio::io::{self, AsyncBufReadExt};
use futures::{StreamExt, SinkExt};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // Connect to the WebSocket server
    let (ws_stream,_) = connect_async("ws://127.0.0.1:8080")
    .await
    .unwrap();
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

    // Wait for all tasks to complete
    let _ = tokio::join!(input_task, send_task, receive_task);
}
