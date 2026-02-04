mod tls;
mod websocket;
mod gemini;
mod audio;
mod wake_word;
mod vad;

use audio::AudioDevice;
use wake_word::WakeWordDetector;
use vad::VAD;
use gemini::{GeminiClient, GeminiConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA Daemon v0.4.0 - Always Listening Mode");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize components
    println!("\n[1/4] Initializing audio device...");
    let mut audio = AudioDevice::new()?;
    println!("✅ Audio device ready");

    println!("\n[2/4] Initializing wake word detector...");
    let mut wake_word = WakeWordDetector::new();
    wake_word.set_sensitivity(0.6); // Adjust sensitivity
    println!("✅ Wake word detector ready (sensitivity: 0.6)");

    println!("\n[3/4] Initializing Voice Activity Detection...");
    let mut vad = VAD::new();
    println!("✅ VAD ready");

    println!("\n[4/4] Connecting to Gemini API...");
    let config = GeminiConfig::default();
    
    if config.api_key.is_empty() {
        println!("⚠️  GOOGLE_API_KEY not set - running in demo mode");
        println!("   Set API key: export GOOGLE_API_KEY=your_key");
        
        // Demo mode: just test wake word and VAD
        demo_mode(&mut audio, &mut wake_word, &mut vad).await?;
        return Ok(());
    }

    let mut gemini = match GeminiClient::connect(config).await {
        Ok(client) => {
            println!("✅ Connected to Gemini API");
            Some(client)
        }
        Err(e) => {
            println!("⚠️  Could not connect to Gemini: {}", e);
            println!("   Running in demo mode");
            None
        }
    };

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("👂 EVA is now listening for 'Hey EVA'...");
    println!("   (Press Ctrl+C to stop)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Main listening loop
    loop {
        // 1. Capture audio chunk
        let chunk = audio.capture_chunk().await?;

        // 2. Check for wake word
        if wake_word.detect(&chunk) {
            println!("\n🎤 Wake word detected! Listening for command...");
            wake_word.reset();
            
            // 3. Active listening mode
            let mut audio_buffer = Vec::new();
            let mut silence_count = 0;
            
            loop {
                let audio_chunk = audio.capture_chunk().await?;
                
                // 4. Check if still speaking
                if vad.is_speech(&audio_chunk) {
                    audio_buffer.extend_from_slice(&audio_chunk);
                    silence_count = 0;
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout())?;
                } else {
                    silence_count += 1;
                    
                    // End of speech after 1 second of silence
                    if silence_count > 10 {
                        println!("\n✅ Command captured ({} samples)", audio_buffer.len());
                        break;
                    }
                }
                
                // Safety: max 30 seconds of recording
                if audio_buffer.len() > 48000 * 30 {
                    println!("\n⚠️  Maximum recording time reached");
                    break;
                }
            }
            
            // 5. Process with Gemini (if available)
            if let Some(ref mut gemini_client) = gemini {
                println!("🤖 Processing with Gemini...");
                
                // Convert to bytes for Gemini
                let audio_bytes: Vec<u8> = audio_buffer
                    .iter()
                    .flat_map(|&sample| {
                        let sample_i16 = (sample * i16::MAX as f32) as i16;
                        sample_i16.to_le_bytes()
                    })
                    .collect();
                
                // Send to Gemini
                if let Err(e) = gemini_client.send_audio(&audio_bytes).await {
                    println!("❌ Error sending audio: {}", e);
                } else {
                    // Wait for response
                    match gemini_client.receive().await {
                        Ok(Some(response)) => {
                            if let Some(content) = response.server_content {
                                if let Some(turn) = content.model_turn {
                                    for part in turn.parts {
                                        if let Some(text) = part.text {
                                            println!("🤖 EVA: {}", text);
                                        }
                                        
                                        // TODO: Play audio response
                                        if let Some(_audio_data) = part.inline_data {
                                            println!("🔊 [Audio response received]");
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => println!("⚠️  No response from Gemini"),
                        Err(e) => println!("❌ Error receiving response: {}", e),
                    }
                }
            } else {
                println!("ℹ️  Demo mode: Command would be sent to Gemini");
                println!("   Captured {} samples", audio_buffer.len());
            }
            
            // Reset for next wake word
            vad.reset();
            println!("\n👂 Listening for 'Hey EVA'...\n");
        }

        // Small delay to prevent CPU spinning
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

/// Demo mode - test wake word and VAD without Gemini
async fn demo_mode(
    audio: &mut AudioDevice,
    wake_word: &mut WakeWordDetector,
    vad: &mut VAD,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎮 DEMO MODE - Testing wake word and VAD");
    println!("   (Press Ctrl+C to stop)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut frame_count = 0;
    
    loop {
        let chunk = audio.capture_chunk().await?;
        
        // Check wake word
        if wake_word.detect(&chunk) {
            println!("\n🎤 Wake word detected!");
            wake_word.reset();
            
            // Test VAD
            println!("Testing VAD for 5 seconds...");
            for i in 0..50 {
                let audio_chunk = audio.capture_chunk().await?;
                let is_speech = vad.is_speech(&audio_chunk);
                
                if i % 10 == 0 {
                    println!("  Frame {}: Speech = {}", i, is_speech);
                }
            }
            
            vad.reset();
            println!("✅ VAD test complete\n");
        }
        
        frame_count += 1;
        if frame_count % 100 == 0 {
            println!("  Processed {} frames...", frame_count);
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
