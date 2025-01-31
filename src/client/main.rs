use futures::{SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::WebSocketStream;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type WsSink = futures::stream::SplitSink<WsStream, Message>;


#[tokio::main]
async fn main() {
    // Spawn task to listen for Ctrl+C
    let shutdown_signal = listen_for_shutdown_signal();

    // Connect to the WebSocket server
    let (ws_stream, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .expect("Failed to connect to WebSocket server");
    let (ws_write, ws_read) = ws_stream.split();

    // Set up the channel and tasks
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    
    let input_task = spawn_input_task(tx);
    let send_task = spawn_send_task(rx, ws_write);
    let receive_task = spawn_receive_task(ws_read);

    // Wait for tasks to complete or shutdown signal
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

// Task to listen for shutdown signal (Ctrl+C)
fn listen_for_shutdown_signal() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
        println!("Ctrl+C detected. Shutting down...");
    })
}

// Task to handle user input
fn spawn_input_task(tx: mpsc::UnboundedSender<String>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
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
    })
}

// Task to handle sending messages to the server
fn spawn_send_task(mut rx: mpsc::UnboundedReceiver<String>, mut ws_write: WsSink) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(input) = rx.recv().await {
            if ws_write.send(Message::Text(input.into())).await.is_err() {
                eprintln!("Failed to send message to server");
                break;
            }
        }
        let _ = ws_write.send(Message::Close(None)).await;
    })
}

// Task to handle receiving messages from the server
fn spawn_receive_task(mut ws_read: futures::stream::SplitStream<WsStream>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
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
    })
}
