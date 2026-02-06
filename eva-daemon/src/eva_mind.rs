use crate::websocket::WebSocketClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("eva_debug.log")
    {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        writeln!(file, "[{}] {}", timestamp, msg).ok();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaMindConfig {
    pub ws_url: String,
    pub cpf: String,
}

impl Default for EvaMindConfig {
    fn default() -> Self {
        Self {
            ws_url: "wss://eva-ia.org:8090/ws/pcm".to_string(),
            cpf: "64525430249".to_string(), // Creator CPF
        }
    }
}

pub struct EvaMindClient {
    ws: WebSocketClient,
    config: EvaMindConfig,
    session_id: String,
    connected: bool,
}

impl EvaMindClient {
    /// Connect to EVA-Mind WebSocket
    pub async fn connect(config: EvaMindConfig) -> Result<Self, Box<dyn std::error::Error>> {
        log_debug(&format!("🤖 Conectando ao EVA-Mind: {}", config.ws_url));

        let ws = WebSocketClient::connect(&config.ws_url).await?;
        log_debug("✅ WebSocket conectado");

        let session_id = format!("eva-os-{}", chrono::Local::now().timestamp_millis());

        let mut client = Self {
            ws,
            config,
            session_id,
            connected: false,
        };

        // Register client
        client.register().await?;

        Ok(client)
    }

    /// Register with EVA-Mind
    async fn register(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let register_msg = json!({
            "type": "register",
            "user_type": "patient",
            "cpf": self.config.cpf
        });

        log_debug(&format!("📤 Register: {}", register_msg));
        self.ws.send_text(&register_msg.to_string()).await?;
        log_debug("✅ Register enviado");

        Ok(())
    }

    /// Start call session
    pub async fn start_call(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let start_msg = json!({
            "type": "start_call",
            "cpf": self.config.cpf,
            "session_id": self.session_id
        });

        log_debug(&format!("📤 Start call: {}", start_msg));
        self.ws.send_text(&start_msg.to_string()).await?;
        log_debug("✅ Start call enviado");

        // Wait for session_created
        self.wait_for_session_created().await?;

        Ok(())
    }

    /// Wait for session_created message
    async fn wait_for_session_created(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log_debug("⏳ Aguardando session_created...");

        let timeout = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err("Timeout aguardando session_created".into());
            }

            let receive_timeout = tokio::time::timeout(
                tokio::time::Duration::from_secs(2),
                self.ws.receive()
            ).await;

            match receive_timeout {
                Ok(Ok(Some(msg))) => {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        log_debug(&format!("📥 Msg: {}", &text[..text.len().min(200)]));

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            if json.get("type").and_then(|v| v.as_str()) == Some("session_created") {
                                log_debug("✅ session_created recebido!");
                                self.connected = true;
                                return Ok(());
                            }
                            if json.get("type").and_then(|v| v.as_str()) == Some("error") {
                                let err_msg = json.get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown error");
                                return Err(format!("EVA-Mind error: {}", err_msg).into());
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {
                    return Err("WebSocket closed".into());
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(_) => {
                    // Timeout, continue
                    continue;
                }
            }
        }
    }

    /// Send audio data (PCM 16kHz bytes)
    pub async fn send_audio(&mut self, pcm_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected to session".into());
        }

        log_debug(&format!("🎤 Enviando áudio: {} bytes", pcm_data.len()));
        self.ws.send_binary(pcm_data.to_vec()).await?;
        log_debug("✅ Áudio enviado");
        Ok(())
    }

    /// Receive audio data (PCM bytes or control messages)
    pub async fn receive(&mut self) -> Result<Option<EvaMindResponse>, Box<dyn std::error::Error>> {
        let receive_timeout = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            self.ws.receive()
        ).await;

        match receive_timeout {
            Ok(Ok(Some(msg))) => {
                match msg {
                    tokio_tungstenite::tungstenite::Message::Binary(data) => {
                        log_debug(&format!("🔊 Áudio recebido: {} bytes", data.len()));
                        return Ok(Some(EvaMindResponse::Audio(data)));
                    }
                    tokio_tungstenite::tungstenite::Message::Text(text) => {
                        log_debug(&format!("📥 Msg: {}", &text[..text.len().min(100)]));
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                            return Ok(Some(EvaMindResponse::Control(json)));
                        }
                    }
                    _ => {}
                }
            }
            Ok(Ok(None)) => {
                return Err("WebSocket closed".into());
            }
            Ok(Err(e)) => {
                return Err(format!("WebSocket error: {}", e).into());
            }
            Err(_) => {
                // Timeout, no message
            }
        }

        Ok(None)
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Debug)]
pub enum EvaMindResponse {
    Audio(Vec<u8>),
    Control(serde_json::Value),
}
