use crate::websocket::WebSocketClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
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
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
    pub ws_url: String,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GOOGLE_API_KEY").unwrap_or_default(),
            model: "gemini-2.5-flash-native-audio-preview-12-2025".to_string(),
            ws_url: "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent".to_string(),
        }
    }
}

pub struct GeminiClient {
    ws: WebSocketClient,
    config: GeminiConfig,
    setup_complete: bool,
}

impl GeminiClient {
    /// Connect to Gemini API via WebSocket
    pub async fn connect(config: GeminiConfig) -> Result<Self, Box<dyn std::error::Error>> {
        if config.api_key.is_empty() {
            return Err("GOOGLE_API_KEY não configurada".into());
        }

        let url = format!("{}?key={}", config.ws_url, config.api_key);

        log_debug("🤖 Conectando ao Gemini...");
        let ws = WebSocketClient::connect(&url).await?;
        log_debug("✅ WebSocket conectado");

        let mut client = Self { ws, config, setup_complete: false };

        // Send setup
        client.send_setup().await?;

        // Wait for setupComplete (CRITICAL!)
        client.wait_for_setup_complete().await?;

        Ok(client)
    }

    /// Send setup message
    async fn send_setup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let model_full = format!("models/{}", self.config.model);

        let setup = json!({
            "setup": {
                "model": model_full,
                "generation_config": {
                    "response_modalities": ["AUDIO"],
                    "speech_config": {
                        "voice_config": {
                            "prebuilt_voice_config": {
                                "voice_name": "Aoede"
                            }
                        }
                    },
                    "temperature": 0.6
                },
                "system_instruction": {
                    "parts": [{
                        "text": "Você é EVA, uma assistente de voz amigável. Responda em português brasileiro de forma natural e concisa."
                    }]
                }
            }
        });

        log_debug(&format!("📤 Setup: {}", setup.to_string()));
        self.ws.send_text(&setup.to_string()).await?;
        log_debug("✅ Setup enviado");

        Ok(())
    }

    /// Wait for setupComplete from Gemini (with timeout)
    async fn wait_for_setup_complete(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log_debug("⏳ Aguardando setupComplete...");

        let timeout = tokio::time::Duration::from_secs(10);
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err("Timeout aguardando setupComplete".into());
            }

            // Use timeout for each receive
            let receive_timeout = tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                self.ws.receive()
            ).await;

            match receive_timeout {
                Ok(Ok(Some(msg))) => {
                    if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                        log_debug(&format!("📥 Setup resp: {}", &text[..text.len().min(200)]));

                        let json: Value = serde_json::from_str(&text)?;

                        // Check for setupComplete
                        if json.get("setupComplete").is_some() {
                            log_debug("✅ setupComplete recebido - Pronto!");
                            self.setup_complete = true;
                            return Ok(());
                        }

                        // Check for error
                        if let Some(error) = json.get("error") {
                            let err_msg = format!("Gemini error: {:?}", error);
                            log_debug(&format!("❌ {}", err_msg));
                            return Err(err_msg.into());
                        }
                    }
                }
                Ok(Ok(None)) => {
                    // No message, continue waiting
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
                Ok(Err(e)) => {
                    log_debug(&format!("❌ Erro ao receber: {}", e));
                    return Err(e);
                }
                Err(_) => {
                    // Timeout, continue waiting
                    continue;
                }
            }
        }
    }

    /// Send audio data (PCM 16kHz)
    pub async fn send_audio(&mut self, pcm_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        log_debug(&format!("🎤 Enviando áudio: {} bytes", pcm_data.len()));

        let base64_audio = BASE64.encode(pcm_data);

        // ✅ FIX: Usar mime_type com rate como EVA-Mind
        let message = json!({
            "realtime_input": {
                "media_chunks": [{
                    "mime_type": "audio/pcm;rate=16000",
                    "data": base64_audio
                }]
            }
        });

        self.ws.send_text(&message.to_string()).await?;
        log_debug("✅ Áudio enviado");
        Ok(())
    }

    /// Send text message
    pub async fn send_text(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        log_debug(&format!("📤 Enviando texto: {}", text));

        let message = json!({
            "client_content": {
                "turn_complete": true,
                "turns": [{
                    "role": "user",
                    "parts": [{
                        "text": text
                    }]
                }]
            }
        });

        self.ws.send_text(&message.to_string()).await?;
        log_debug("✅ Texto enviado");
        Ok(())
    }

    /// Receive a single message from WebSocket (non-blocking with short timeout)
    async fn receive_message(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let receive_result = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            self.ws.receive()
        ).await;

        match receive_result {
            Ok(Ok(Some(msg))) => {
                if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                    return Ok(Some(text));
                }
                Ok(None)
            }
            Ok(Ok(None)) => Err("WebSocket closed".into()),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None), // Timeout, no message available
        }
    }

    /// Try to receive one response, returns immediately if no message available
    pub async fn try_receive(&mut self) -> Result<Option<GeminiResponse>, Box<dyn std::error::Error>> {
        if let Some(text) = self.receive_message().await? {
            let preview = &text[..text.len().min(200)];
            log_debug(&format!("📥 Msg: {}", preview));

            // Parse JSON
            let json: Value = serde_json::from_str(&text)?;

            // Check for error
            if let Some(error) = json.get("error") {
                let err_msg = format!("Gemini error: {:?}", error);
                log_debug(&format!("❌ {}", err_msg));
                return Err(err_msg.into());
            }

            // Check for serverContent with modelTurn (audio response)
            if json.get("serverContent").is_some() {
                match serde_json::from_str::<GeminiResponse>(&text) {
                    Ok(response) => {
                        if let Some(ref content) = response.server_content {
                            if let Some(ref turn) = content.model_turn {
                                for part in &turn.parts {
                                    if let Some(ref txt) = part.text {
                                        log_debug(&format!("💬 Texto: {}", txt));
                                    }
                                    if let Some(ref data) = part.inline_data {
                                        log_debug(&format!("🔊 Áudio: {} ({} bytes)",
                                            data.mime_type, data.data.len()));
                                    }
                                }
                                // Return when we have parts with content
                                if !turn.parts.is_empty() {
                                    return Ok(Some(response));
                                }
                            }
                            if content.turn_complete.unwrap_or(false) {
                                log_debug("✅ Turn complete");
                            }
                        }
                        return Ok(Some(response));
                    }
                    Err(e) => {
                        log_debug(&format!("⚠️ Parse error: {}", e));
                    }
                }
            } else {
                log_debug("📥 (non-content msg)");
            }
        }
        Ok(None)
    }

    /// Receive response - keeps trying until content or timeout
    pub async fn receive(&mut self) -> Result<Option<GeminiResponse>, Box<dyn std::error::Error>> {
        let timeout = tokio::time::Duration::from_secs(30);
        let start = tokio::time::Instant::now();

        log_debug("👂 Aguardando resposta...");

        while start.elapsed() < timeout {
            match self.try_receive().await {
                Ok(Some(response)) => {
                    // Check if has actual audio/text content
                    if let Some(ref content) = response.server_content {
                        if let Some(ref turn) = content.model_turn {
                            let has_content = turn.parts.iter().any(|p| {
                                p.text.is_some() || p.inline_data.is_some()
                            });
                            if has_content {
                                return Ok(Some(response));
                            }
                        }
                    }
                    // Got message but no content yet, continue
                }
                Ok(None) => {
                    // No message, small sleep and continue
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        log_debug("⏱️ Timeout");
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
    #[serde(rename = "turnComplete")]
    pub turn_complete: Option<bool>,
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
    pub data: String,
}
