use crate::websocket::WebSocketClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub ws_url: String,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
            model: "gemini-2.0-flash-exp".to_string(),
            ws_url: "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent".to_string(),
        }
    }
}

pub struct GeminiClient {
    ws: WebSocketClient,
    config: GeminiConfig,
}

impl GeminiClient {
    /// Connect to Gemini API via WebSocket
    pub async fn connect(config: GeminiConfig) -> Result<Self, Box<dyn std::error::Error>> {
        if config.api_key.is_empty() {
            return Err("GOOGLE_API_KEY não configurada".into());
        }

        let url = format!("{}?key={}", config.ws_url, config.api_key);

        println!("🤖 Conectando ao Gemini...");
        let ws = WebSocketClient::connect(&url).await?;

        let mut client = Self { ws, config };

        // Enviar mensagem de setup
        client.send_setup().await?;

        Ok(client)
    }

    /// Send setup message to configure the session
    async fn send_setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let setup = json!({
            "setup": {
                "model": self.config.model,
                "generation_config": {
                    "response_modalities": ["AUDIO"],
                    "speech_config": {
                        "voice_config": {
                            "prebuilt_voice_config": {
                                "voice_name": "Kore"
                            }
                        }
                    }
                }
            }
        });

        self.ws.send_text(&setup.to_string()).await?;
        println!("✅ Setup enviado ao Gemini");

        Ok(())
    }

    /// Send audio data (PCM format)
    pub async fn send_audio(&mut self, pcm_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let base64_audio = BASE64.encode(pcm_data);

        let message = json!({
            "realtime_input": {
                "media_chunks": [{
                    "mime_type": "audio/pcm",
                    "data": base64_audio
                }]
            }
        });

        self.ws.send_text(&message.to_string()).await?;
        Ok(())
    }

    /// Send text message
    pub async fn send_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let message = json!({
            "client_content": {
                "turns": [{
                    "role": "user",
                    "parts": [{
                        "text": text
                    }]
                }],
                "turn_complete": true
            }
        });

        self.ws.send_text(&message.to_string()).await?;
        Ok(())
    }

    /// Receive response from Gemini
    pub async fn receive(&mut self) -> Result<Option<GeminiResponse>, Box<dyn std::error::Error>> {
        if let Some(msg) = self.ws.receive().await? {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                let response: GeminiResponse = serde_json::from_str(&text)?;
                return Ok(Some(response));
            }
        }
        Ok(None)
    }

    /// Keep connection alive
    pub async fn ping(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.ws.ping().await
    }
}

#[derive(Debug, Deserialize)]
pub struct GeminiResponse {
    #[serde(rename = "serverContent")]
    pub server_content: Option<ServerContent>,
}

#[derive(Debug, Deserialize)]
pub struct ServerContent {
    #[serde(rename = "modelTurn")]
    pub model_turn: Option<ModelTurn>,
}

#[derive(Debug, Deserialize)]
pub struct ModelTurn {
    pub parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
pub struct Part {
    pub text: Option<String>,
    #[serde(rename = "inlineData")]
    pub inline_data: Option<InlineData>,
}

#[derive(Debug, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String, // Base64
}
