use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerSymbol {
    X,
    O,
}

#[derive(Debug, Clone)]
pub struct Player {
    id: String,
    name: String,
    symbol: PlayerSymbol,
    ws_sink: Arc<Mutex<futures::stream::SplitSink<WebSocketStream<TcpStream>, Message>>>,
}

impl Player {
    pub fn new(
        name: String,
        symbol: PlayerSymbol,
        ws_sink: Arc<Mutex<futures::stream::SplitSink<WebSocketStream<TcpStream>, Message>>>,
    ) -> Self {
        Self {
            name,
            symbol,
            id: crate::utils::generate_id(),
            ws_sink,
        }
    }

    pub fn set_symbol(&mut self, symbol: PlayerSymbol) {
        self.symbol = symbol;
    }

    pub fn get_symbol(&self) -> PlayerSymbol {
        self.symbol.clone()
    }

    pub fn get_id(&self) -> String {
        self.id.clone()
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_ws_sink(
        &self,
    ) -> Arc<Mutex<futures::stream::SplitSink<WebSocketStream<TcpStream>, Message>>> {
        self.ws_sink.clone()
    }
}
