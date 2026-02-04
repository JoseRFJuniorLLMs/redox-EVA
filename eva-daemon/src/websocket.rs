use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::{SinkExt, StreamExt};
use url::Url;

pub struct WebSocketClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WebSocketClient {
    /// Connect to a WebSocket server with automatic TLS support
    pub async fn connect(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔗 Conectando ao WebSocket: {}", url);

        // Conectar (TLS automático para wss://)
        let url = Url::parse(url)?;
        let (ws_stream, response) = connect_async(url).await?;

        println!("✅ WebSocket conectado! Status: {}", response.status());

        Ok(Self { stream: ws_stream })
    }

    /// Send text message
    pub async fn send_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Text(text.to_string())).await?;
        Ok(())
    }

    /// Send binary message (for audio PCM data)
    pub async fn send_binary(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Binary(data)).await?;
        Ok(())
    }

    /// Receive next message
    pub async fn receive(&mut self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        match self.stream.next().await {
            Some(Ok(msg)) => Ok(Some(msg)),
            Some(Err(e)) => Err(Box::new(e)),
            None => Ok(None),
        }
    }

    /// Close the WebSocket connection
    pub async fn close(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.close(None).await?;
        Ok(())
    }

    /// Send ping to keep connection alive
    pub async fn ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.stream.send(Message::Ping(vec![])).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_echo() {
        // Usar servidor de teste público
        let mut client = WebSocketClient::connect("wss://echo.websocket.org/")
            .await
            .expect("Failed to connect");

        client.send_text("Hello WebSocket!").await.expect("Failed to send");
        
        let response = client.receive().await.expect("Failed to receive");
        assert!(response.is_some());
        
        client.close().await.expect("Failed to close");
    }
}

