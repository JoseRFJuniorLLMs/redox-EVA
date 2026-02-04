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
mod timemachine;

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
use animations::Animation;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize UI components first
    let mut status_indicator = StatusIndicator::new();
    let mut statistics = Statistics::new();
    let mut terminal_ui = TerminalUI::new()?;

    // Initial draw
    terminal_ui.add_system_message("EVA OS Starting...");
    terminal_ui.draw(&status_indicator, &statistics);

    // Initialize components
    terminal_ui.add_system_message("[1/13] Initializing audio device...");
    terminal_ui.draw(&status_indicator, &statistics);
    let mut audio = AudioDevice::new()?;
    terminal_ui.add_system_message("✅ Audio device ready");
    terminal_ui.draw(&status_indicator, &statistics);
    


    terminal_ui.add_system_message("[2/13] Initializing wake word detector...");
    terminal_ui.draw(&status_indicator, &statistics);
    let mut wake_word = WakeWordDetector::new();
    wake_word.set_sensitivity(0.6);
    terminal_ui.add_system_message("✅ Wake word detector ready (sensitivity: 0.6)");
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[3/13] Initializing Voice Activity Detection...");
    terminal_ui.draw(&status_indicator, &statistics);
    let mut vad = VAD::new();
    terminal_ui.add_system_message("✅ VAD ready");
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[4/13] Initializing audio player...");
    terminal_ui.draw(&status_indicator, &statistics);
    let audio_device_clone = AudioDevice::new()?;
    let mut audio_player = AudioPlayer::new(audio_device_clone)?;
    terminal_ui.add_system_message("✅ Audio player ready");
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[5/13] Initializing conversation session...");
    terminal_ui.draw(&status_indicator, &statistics);
    // Load session from file or create new
    let mut session = ConversationSession::load_from_file("session.json").unwrap_or_else(|_| {
        terminal_ui.add_system_message("No previous session found, starting new.");
        ConversationSession::new()
    });
    terminal_ui.add_system_message(&format!("✅ Session ready (ID: {}, Turns: {})", session.session_id(), session.turn_count()));
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[6/13] Initializing command parser...");
    terminal_ui.draw(&status_indicator, &statistics);
    let command_parser = CommandParser::new();
    terminal_ui.add_system_message("✅ Command parser ready");
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[7/13] Initializing command executor...");
    terminal_ui.draw(&status_indicator, &statistics);
    let mut command_executor = CommandExecutor::new()?;
    terminal_ui.add_system_message("✅ Command executor ready (sandbox enabled)");
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[8/13] Loading user profile...");
    terminal_ui.draw(&status_indicator, &statistics);
    let _profile = UserProfile::load()?;
    terminal_ui.add_system_message(&format!("✅ User profile loaded (User: {}, Language: {})", _profile.name, _profile.language));
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[9/13] Initializing custom commands...");
    terminal_ui.draw(&status_indicator, &statistics);
    let mut _custom_commands = CustomCommandManager::new()?;
    terminal_ui.add_system_message(&format!("✅ Custom commands ready ({} commands)", _custom_commands.count()));
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[10/13] Initializing macros...");
    terminal_ui.draw(&status_indicator, &statistics);
    let mut _macros = MacroManager::new()?;
    terminal_ui.add_system_message(&format!("✅ Macros ready ({} macros)", _macros.count()));
    terminal_ui.draw(&status_indicator, &statistics);

    terminal_ui.add_system_message("[11/13] Initializing emotion detection...");
    terminal_ui.draw(&status_indicator, &statistics);
    let _emotion_detector = EmotionDetector::new();
    terminal_ui.add_system_message("✅ Emotion detection ready");
    terminal_ui.draw(&status_indicator, &statistics);

    // Initialize animations
    let mut anim_listening = Animation::listening();
    let mut anim_processing = Animation::processing();
    let mut anim_speaking = Animation::speaking();

    terminal_ui.add_system_message("[12/13] Connecting to Gemini API...");
    terminal_ui.draw(&status_indicator, &statistics);
    let config = GeminiConfig::default();
    
    let mut gemini = if config.api_key.is_empty() {
        terminal_ui.add_system_message("⚠️  GOOGLE_API_KEY not set - running in demo mode");
        terminal_ui.add_system_message("   Set API key: export GOOGLE_API_KEY=your_key");
        terminal_ui.draw(&status_indicator, &statistics);
        None
    } else {
        match GeminiClient::connect(config).await {
            Ok(client) => {
                terminal_ui.add_system_message("✅ Connected to Gemini API");
                terminal_ui.draw(&status_indicator, &statistics);
                Some(client)
            }
            Err(e) => {
                terminal_ui.add_system_message(&format!("⚠️  Could not connect to Gemini: {}", e));
                terminal_ui.add_system_message("   Running in demo mode");
                terminal_ui.draw(&status_indicator, &statistics);
                None
            }
        }
    };

    // [13/13] Initialize Time Machine
    terminal_ui.add_system_message("[13/13] Initializing Time Machine (NPU)...");
    terminal_ui.draw(&status_indicator, &statistics);
    
    #[cfg(feature = "timemachine")]
    let timemachine_res = crate::timemachine::TimeMachine::new().await;
    
    #[cfg(not(feature = "timemachine"))]
    let timemachine_res: Result<crate::timemachine::TimeMachine, Box<dyn std::error::Error>> = Err("Feature disabled".into());

    let _timemachine = match timemachine_res {
        Ok(tm) => {
            terminal_ui.add_system_message("✅ Time Machine ready (Encrypted & Local)");
            let tm_arc = std::sync::Arc::new(tm);
            let tm_clone = tm_arc.clone();
            
            // Start background recording
            tokio::spawn(async move {
                tm_clone.start_recording().await;
            });
            Some(tm_arc)
        },
        Err(e) => {
            terminal_ui.add_system_message(&format!("⚠️ Time Machine disabled: {}", e));
            None
        }
    };
    terminal_ui.draw(&status_indicator, &statistics);

    // Start UI
    terminal_ui.add_system_message("EVA OS Started");
    terminal_ui.add_system_message(&format!("Session ID: {}", session.session_id()));
    
    if gemini.is_none() {
        terminal_ui.add_system_message("Running in DEMO MODE (No API Key)");
    }

    status_indicator.set_status(EvaStatus::Idle);
    terminal_ui.draw(&status_indicator, &statistics);

    // Main conversation loop
    loop {
        // Reset animations
        anim_listening.reset();
        anim_processing.reset();
        anim_speaking.reset();

        // 1. Capture audio chunk
        let chunk = match audio.capture_chunk().await {
            Ok(c) => c,
            Err(e) => {
                terminal_ui.add_system_message(&format!("Audio Error: {}", e));
                continue;
            }
        };

        // 2. Check for wake word
        if wake_word.detect(&chunk) {
            status_indicator.set_status(EvaStatus::Listening);
            terminal_ui.add_system_message("Wake word detected! Listening...");
            statistics.update_all();
            terminal_ui.draw(&status_indicator, &statistics);
            
            wake_word.reset();
            
            // 3. Capture command until silence
            let mut audio_buffer = Vec::new();
            let mut silence_count = 0;
            
            loop {
                // Update stats occasionally to show activity? 
                // For now, blocking audio capture in loop prevents frequent UI updates unless we spawn
                // But simplified TUI: just capture
                
                let audio_chunk = match audio.capture_chunk().await {
                    Ok(c) => c,
                    Err(_) => break, // Stop recording on error
                };
                
                if vad.is_speech(&audio_chunk) {
                    audio_buffer.extend_from_slice(&audio_chunk);
                    silence_count = 0;
                } else {
                    silence_count += 1;
                    
                    if silence_count > 10 {
                        // End of speech
                        break;
                    }
                }
                
                if audio_buffer.len() > 48000 * 30 {
                    terminal_ui.add_system_message("Max recording time reached");
                    break;
                }

                // Animate listening (update every few frames to avoid too much flickering)
                if audio_buffer.len() % 2 == 0 {
                    statistics.update_all();
                    status_indicator.set_symbol(anim_listening.next_frame());
                    terminal_ui.draw(&status_indicator, &statistics);
                }
            }
            
            terminal_ui.add_system_message(&format!("Captured {} samples", audio_buffer.len()));
            statistics.turns += 1;
            terminal_ui.draw(&status_indicator, &statistics);

            // 4. Process
            status_indicator.set_status(EvaStatus::Processing);
            terminal_ui.draw(&status_indicator, &statistics);

            if let Some(ref mut gemini_client) = gemini {
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
                    status_indicator.set_status(EvaStatus::Error);
                    terminal_ui.add_system_message(&format!("Gemini Error: {}", e));
                } else {
                    // Wait for response with animation
                    let receive_future = gemini_client.receive();
                    tokio::pin!(receive_future);

                    let response_result = loop {
                        tokio::select! {
                            result = &mut receive_future => break result,
                            _ = tokio::time::sleep(anim_processing.frame_duration()) => {
                                statistics.update_all();
                                status_indicator.set_symbol(anim_processing.next_frame());
                                terminal_ui.draw(&status_indicator, &statistics);
                            }
                        }
                    };

                    match response_result {
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
                                            
                                            // Play audio response with animation
                                            status_indicator.set_status(EvaStatus::Speaking);
                                            
                                            let play_future = audio_player.play_response(&audio_data.data);
                                            tokio::pin!(play_future);
                                            
                                            let play_result = loop {
                                                tokio::select! {
                                                    result = &mut play_future => break result,
                                                    _ = tokio::time::sleep(anim_speaking.frame_duration()) => {
                                                        statistics.update_all();
                                                        status_indicator.set_symbol(anim_speaking.next_frame());
                                                        terminal_ui.draw(&status_indicator, &statistics);
                                                    }
                                                }
                                            };

                                            if let Err(e) = play_result {
                                                terminal_ui.add_system_message(&format!("Audio Playback Error: {}", e));
                                                // Fallback to text handled by printing later
                                            }
                                        }
                                    }
                                    
                                    // Update conversation log
                                    if !response_text.is_empty() {
                                        terminal_ui.add_eva_message(&response_text);
                                        session.add_turn(Role::Assistant, response_text.clone());
                                        
                                        // Detect emotion
                                        let detected_emotion = _emotion_detector.detect(&response_text);
                                        status_indicator.set_emotion(detected_emotion);
                                        terminal_ui.draw(&status_indicator, &statistics);

                                        // Save session
                                        if let Err(e) = session.save_to_file("session.json") {
                                            terminal_ui.add_system_message(&format!("Failed to save session: {}", e));
                                        }
                                        
                                        // Parse for commands
                                        if let Ok(intent) = command_parser.parse(&response_text) {
                                            use command_parser::CommandIntent;
                                            
                                            match intent {
                                                CommandIntent::Unknown => {
                                                    // Just conversation
                                                }
                                                _ => {
                                                    // Execute command
                                                    status_indicator.set_status(EvaStatus::Executing);
                                                    terminal_ui.draw(&status_indicator, &statistics);
                                                    
                                                    match command_executor.execute(intent).await {
                                                        Ok(result) => {
                                                            statistics.commands_executed += 1;
                                                            terminal_ui.add_system_message(&format!("Command Result: {}", result));
                                                            
                                                            session.add_turn(
                                                                Role::Assistant,
                                                                format!("Command result: {}", result)
                                                            );
                                                            // Save session
                                                            session.save_to_file("session.json").ok();
                                                        }
                                                        Err(e) => {
                                                            status_indicator.set_status(EvaStatus::Error);
                                                            terminal_ui.add_system_message(&format!("Command Error: {}", e));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(None) => terminal_ui.add_system_message("No response from Gemini"),
                        Err(e) => terminal_ui.add_system_message(&format!("Receive Error: {}", e)),
                    }
                }
            } else {
                // Demo mode logic
                terminal_ui.add_system_message("Processing (Demo Mode)...");
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                
                let demo_response = "I heard you! This is a demo response.";
                terminal_ui.add_eva_message(demo_response);
                
                status_indicator.set_status(EvaStatus::Speaking);
                
                // Simulate speaking time with animation
                let speak_duration = tokio::time::Duration::from_millis(2000);
                let start_time = tokio::time::Instant::now();
                
                while start_time.elapsed() < speak_duration {
                    statistics.update_all();
                    status_indicator.set_symbol(anim_speaking.next_frame());
                    terminal_ui.draw(&status_indicator, &statistics);
                    tokio::time::sleep(anim_speaking.frame_duration()).await;
                }
            }
            
            // Reset to idle
            status_indicator.set_status(EvaStatus::Idle);
            vad.reset();
            terminal_ui.draw(&status_indicator, &statistics);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
