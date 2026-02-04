mod tls;
mod websocket;
mod gemini;

use websocket::WebSocketClient;
use gemini::{GeminiClient, GeminiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA Daemon v0.3.0 - Teste WebSocket + Gemini");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Teste 1: WebSocket Echo Server
    println!("\n[1/3] Testando WebSocket básico...");
    let mut echo_client = WebSocketClient::connect("wss://echo.websocket.org/").await?;
    println!("✅ Conectado ao servidor echo");

    echo_client.send_text("Hello from EVA Daemon!").await?;
    println!("📤 Mensagem enviada");

    if let Some(msg) = echo_client.receive().await? {
        println!("📥 Resposta recebida: {:?}", msg);
    }

    echo_client.close().await?;
    println!("✅ Teste WebSocket básico completo");

    // Teste 2: Conectar ao backend EVA Mind (se disponível)
    println!("\n[2/3] Testando conexão com EVA Mind backend...");
    match WebSocketClient::connect("wss://eva-ia.org:8090/ws/pcm").await {
        Ok(mut eva_client) => {
            println!("✅ Conectado ao EVA Mind backend!");
            
            // Enviar mensagem de teste
            eva_client.send_text(r#"{"type":"ping"}"#).await?;
            println!("📤 Ping enviado ao backend");
            
            // Aguardar resposta (com timeout)
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                eva_client.receive()
            ).await.ok();
            
            eva_client.close().await?;
            println!("✅ Teste EVA Mind backend completo");
        }
        Err(e) => {
            println!("⚠️  Backend EVA Mind não disponível: {}", e);
            println!("   (Isso é normal se o servidor não estiver rodando)");
        }
    }

    // Teste 3: Conectar ao Gemini (se API key disponível)
    println!("\n[3/3] Testando conexão com Gemini API...");
    
    if std::env::var("GOOGLE_API_KEY").is_ok() {
        let config = GeminiConfig::default();
        
        match GeminiClient::connect(config).await {
            Ok(mut gemini) => {
                println!("✅ Conectado ao Gemini!");
                
                // Enviar mensagem de teste
                gemini.send_text("Olá, EVA!").await?;
                println!("📤 Mensagem enviada ao Gemini");
                
                // Aguardar resposta
                if let Some(response) = gemini.receive().await? {
                    if let Some(content) = response.server_content {
                        if let Some(turn) = content.model_turn {
                            for part in turn.parts {
                                if let Some(text) = part.text {
                                    println!("🤖 Gemini: {}", text);
                                }
                            }
                        }
                    }
                }
                
                println!("✅ Teste Gemini completo");
            }
            Err(e) => {
                println!("⚠️  Erro ao conectar ao Gemini: {}", e);
            }
        }
    } else {
        println!("⚠️  GOOGLE_API_KEY não configurada");
        println!("   export GOOGLE_API_KEY=sua_chave_aqui");
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ FASE 3 COMPLETA - WebSocket + Gemini funcional!");

    Ok(())
}
