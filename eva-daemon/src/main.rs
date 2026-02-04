mod tls;
mod websocket;
mod gemini;
mod audio;
mod wake_word;
mod vad;
mod audio_player;
mod session;
mod command_parser;
mod command_executor;
mod user_profile;
mod custom_commands;
mod macros;
mod emotion;
mod status_indicator;
mod statistics;
mod animations;
mod terminal_ui;

use audio::AudioDevice;
use wake_word::WakeWordDetector;
use vad::VAD;
use gemini::{GeminiClient, GeminiConfig};
use audio_player::AudioPlayer;
use session::{ConversationSession, Role};
use command_parser::CommandParser;
use command_executor::CommandExecutor;
use user_profile::UserProfile;
use custom_commands::CustomCommandManager;
use macros::MacroManager;
use emotion::EmotionDetector;
use status_indicator::{StatusIndicator, EvaStatus};
use statistics::Statistics;
use terminal_ui::TerminalUI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 EVA OS v0.8.0 - Visual Feedback");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Initialize components
    println!("\n[1/12] Initializing audio device...");
    let mut audio = AudioDevice::new()?;
    println!("✅ Audio device ready");

    println!("\n[2/12] Initializing wake word detector...");
    let mut wake_word = WakeWordDetector::new();
    wake_word.set_sensitivity(0.6);
    println!("✅ Wake word detector ready (sensitivity: 0.6)");

    println!("\n[3/12] Initializing Voice Activity Detection...");
    let mut vad = VAD::new();
    println!("✅ VAD ready");

    println!("\n[4/12] Initializing audio player...");
    let audio_device_clone = AudioDevice::new()?;
    let mut audio_player = AudioPlayer::new(audio_device_clone)?;
    println!("✅ Audio player ready");

    println!("\n[5/12] Initializing conversation session...");
    let mut session = ConversationSession::new();
    println!("✅ Session ready (ID: {})", session.session_id());

    println!("\n[6/12] Initializing command parser...");
    let command_parser = CommandParser::new();
    println!("✅ Command parser ready");

    println!("\n[7/12] Initializing command executor...");
    let mut command_executor = CommandExecutor::new()?;
    println!("✅ Command executor ready (sandbox enabled)");

    println!("\n[8/12] Loading user profile...");
    let _profile = UserProfile::load()?;
    println!("✅ User profile loaded (User: {}, Language: {})", _profile.name, _profile.language);

    println!("\n[9/12] Initializing custom commands...");
    let mut _custom_commands = CustomCommandManager::new()?;
    println!("✅ Custom commands ready ({} commands)", _custom_commands.count());

    println!("\n[10/12] Initializing macros...");
    let mut _macros = MacroManager::new()?;
    println!("✅ Macros ready ({} macros)", _macros.count());

    println!("\n[11/12] Initializing emotion detection...");
    let _emotion_detector = EmotionDetector::new();
    println!("✅ Emotion detection ready");

    println!("\n[12/15] Initializing status indicator...");
    let mut status_indicator = StatusIndicator::new();
    println!("✅ Status indicator ready");

    println!("\n[13/15] Initializing statistics...");
    let mut statistics = Statistics::new();
    println!("✅ Statistics ready");

    println!("\n[14/15] Initializing terminal UI...");
    let mut terminal_ui = TerminalUI::new()?;
    println!("✅ Terminal UI ready");

    println!("\n[15/15] Connecting to Gemini API...");
    let config = GeminiConfig::default();
    
    if config.api_key.is_empty() {
        println!("⚠️  GOOGLE_API_KEY not set - running in demo mode");
        println!("   Set API key: export GOOGLE_API_KEY=your_key");
        
        demo_mode_phase5(&mut audio, &mut wake_word, &mut vad, &mut session).await?;
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
    println!("   Session: {}", session.session_id());
    println!("   (Press Ctrl+C to stop)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Main conversation loop
    loop {
        // 1. Capture audio chunk
        let chunk = audio.capture_chunk().await?;

        // 2. Check for wake word
        if wake_word.detect(&chunk) {
            println!("\n🎤 Wake word detected! Listening for command...");
            wake_word.reset();
            
            // 3. Capture command until silence
            let mut audio_buffer = Vec::new();
            let mut silence_count = 0;
            
            loop {
                let audio_chunk = audio.capture_chunk().await?;
                
                if vad.is_speech(&audio_chunk) {
                    audio_buffer.extend_from_slice(&audio_chunk);
                    silence_count = 0;
                    print!(".");
                    std::io::Write::flush(&mut std::io::stdout())?;
                } else {
                    silence_count += 1;
                    
                    if silence_count > 10 {
                        println!("\n✅ Command captured ({} samples)", audio_buffer.len());
                        break;
                    }
                }
                
                if audio_buffer.len() > 48000 * 30 {
                    println!("\n⚠️  Maximum recording time reached");
                    break;
                }
            }
            
            // 4. Process with Gemini (if available)
            if let Some(ref mut gemini_client) = gemini {
                println!("🤖 Processing with Gemini...");
                
                // Convert to bytes
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
                                    let mut response_text = String::new();
                                    let mut has_audio = false;
                                    
                                    for part in turn.parts {
                                        // Extract text
                                        if let Some(text) = part.text {
                                            response_text.push_str(&text);
                                        }
                                        
                                        // Extract audio
                                        if let Some(audio_data) = part.inline_data {
                                            has_audio = true;
                                            
                                            // Play audio response
                                            println!("🔊 Playing audio response...");
                                            if let Err(e) = audio_player.play_response(&audio_data.data).await {
                                                println!("⚠️  Error playing audio: {}", e);
                                                // Fallback to text
                                                println!("🤖 EVA: {}", response_text);
                                            }
                                        }
                                    }
                                    
                                    // If no audio, just show text
                                    if !has_audio && !response_text.is_empty() {
                                        println!("🤖 EVA: {}", response_text);
                                    }
                                    
                                    // Add to session
                                    if !response_text.is_empty() {
                                        session.add_turn(Role::Assistant, response_text.clone());
                                        
                                        // Parse for commands
                                        if let Ok(intent) = command_parser.parse(&response_text) {
                                            use command_parser::CommandIntent;
                                            
                                            match intent {
                                                CommandIntent::Unknown => {
                                                    // Just conversation, no command
                                                }
                                                _ => {
                                                    // Execute command
                                                    println!("⚙️  Executing command...");
                                                    
                                                    match command_executor.execute(intent).await {
                                                        Ok(result) => {
                                                            println!("✅ {}", result);
                                                            
                                                            // Add result to session
                                                            session.add_turn(
                                                                Role::Assistant,
                                                                format!("Command result: {}", result)
                                                            );
                                                        }
                                                        Err(e) => {
                                                            println!("❌ Error: {}", e);
                                                        }
                                                    }
                                                }
                                            }
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
                // Demo mode
                println!("ℹ️  Demo mode: Command would be sent to Gemini");
                println!("   Captured {} samples", audio_buffer.len());
                
                // Simulate response
                let demo_response = "This is a demo response. In production, Gemini would respond here.";
                println!("🤖 EVA (demo): {}", demo_response);
                session.add_turn(Role::Assistant, demo_response.to_string());
            }
            
            // Show session stats
            println!("\n📊 Session stats:");
            println!("   Turns: {}", session.turn_count());
            println!("   Duration: {:?}", session.duration());
            
            // Reset for next wake word
            vad.reset();
            println!("\n👂 Listening for 'Hey EVA'...\n");
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

/// Demo mode for Phase 5
async fn demo_mode_phase5(
    audio: &mut AudioDevice,
    wake_word: &mut WakeWordDetector,
    vad: &mut VAD,
    session: &mut ConversationSession,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎮 DEMO MODE - Phase 5 Conversation Loop");
    println!("   Session: {}", session.session_id());
    println!("   (Press Ctrl+C to stop)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut frame_count = 0;
    
    loop {
        let chunk = audio.capture_chunk().await?;
        
        if wake_word.detect(&chunk) {
            println!("\n🎤 Wake word detected!");
            wake_word.reset();
            
            // Simulate command capture
            println!("Capturing command...");
            for i in 0..20 {
                let audio_chunk = audio.capture_chunk().await?;
                let is_speech = vad.is_speech(&audio_chunk);
                
                if i % 5 == 0 {
                    if is_speech {
                        print!(".");
                    }
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
            }
            
            println!("\n✅ Command captured");
            
            // Add to session
            session.add_turn(Role::User, "Demo user command".to_string());
            
            // Simulate response
            let responses = vec![
                "Hello! How can I help you today?",
                "I'm EVA, your voice assistant.",
                "That's an interesting question!",
                "Let me think about that...",
            ];
            
            let response = responses[session.turn_count() % responses.len()];
            println!("🤖 EVA (demo): {}", response);
            
            session.add_turn(Role::Assistant, response.to_string());
            
            // Show session info
            println!("\n📊 Session stats:");
            println!("   Turns: {}", session.turn_count());
            println!("   Duration: {:?}", session.duration());
            
            if session.turn_count() > 2 {
                println!("\n📝 Recent conversation:");
                for turn in session.get_recent_turns(4) {
                    println!("   {}: {}", turn.role, turn.content);
                }
            }
            
            vad.reset();
            println!("\n👂 Listening for 'Hey EVA'...\n");
        }
        
        frame_count += 1;
        if frame_count % 100 == 0 {
            println!("  Processed {} frames... (Session: {} turns)", 
                     frame_count, session.turn_count());
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
