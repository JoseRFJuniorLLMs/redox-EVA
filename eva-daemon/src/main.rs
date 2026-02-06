mod tls;
mod websocket;
mod gemini;
mod eva_mind;
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
mod logging;
mod stt;

use audio::AudioDevice;
use wake_word::WakeWordDetector;
use vad::VAD;
use eva_mind::{EvaMindClient, EvaMindConfig, EvaMindResponse};
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

    terminal_ui.add_system_message("[12/13] Connecting to EVA-Mind...");
    terminal_ui.draw(&status_indicator, &statistics);

    let eva_config = EvaMindConfig::default();
    terminal_ui.add_system_message(&format!("   URL: {}", eva_config.ws_url));
    terminal_ui.draw(&status_indicator, &statistics);

    let mut eva_mind: Option<EvaMindClient> = match EvaMindClient::connect(eva_config).await {
        Ok(mut client) => {
            terminal_ui.add_system_message("✅ Connected to EVA-Mind");
            terminal_ui.draw(&status_indicator, &statistics);

            // Start call session
            match client.start_call().await {
                Ok(_) => {
                    terminal_ui.add_system_message(&format!("✅ Session started: {}", client.session_id()));
                    terminal_ui.draw(&status_indicator, &statistics);
                    Some(client)
                }
                Err(e) => {
                    terminal_ui.add_system_message(&format!("⚠️  Could not start session: {}", e));
                    terminal_ui.add_system_message("   Running in demo mode");
                    terminal_ui.draw(&status_indicator, &statistics);
                    None
                }
            }
        }
        Err(e) => {
            terminal_ui.add_system_message(&format!("⚠️  Could not connect to EVA-Mind: {}", e));
            terminal_ui.add_system_message("   Running in demo mode");
            terminal_ui.draw(&status_indicator, &statistics);
            None
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

    if eva_mind.is_none() {
        terminal_ui.add_system_message("Running in DEMO MODE (No Connection)");
    }

    status_indicator.set_status(EvaStatus::Idle);
    terminal_ui.draw(&status_indicator, &statistics);

    // Pronto para receber áudio
    if eva_mind.is_some() {
        terminal_ui.add_system_message("🎤 Diga 'Hey EVA' para começar...");
    }
    terminal_ui.draw(&status_indicator, &statistics);

    // Main conversation loop
    let mut frame_count = 0u64;
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

        // Silently process audio
        frame_count += 1;

        // 2. Check for wake word
        if wake_word.detect(&chunk) {
            status_indicator.set_status(EvaStatus::Listening);
            terminal_ui.add_system_message("Wake word detected! Listening...");
            statistics.update_all();
            terminal_ui.draw(&status_indicator, &statistics);
            
            wake_word.reset();
            
            // 3. Capture and STREAM audio in real-time (like EVA-Mobile)
            let mut total_samples = 0usize;
            let mut silence_count = 0;
            let mut chunk_count = 0u32;
            let mut response_chunks = 0u32;

            loop {
                let audio_chunk = match audio.capture_chunk().await {
                    Ok(c) => c,
                    Err(_) => break,
                };

                // Stream audio to EVA-Mind in real-time (like EVA-Mobile)
                if let Some(ref mut eva_client) = eva_mind {
                    // Convert f32 samples to PCM16 bytes
                    let audio_bytes: Vec<u8> = audio_chunk
                        .iter()
                        .flat_map(|&sample| {
                            let sample_i16 = (sample * i16::MAX as f32) as i16;
                            sample_i16.to_le_bytes()
                        })
                        .collect();

                    // Send immediately (streaming)
                    if let Err(e) = eva_client.send_audio(&audio_bytes).await {
                        terminal_ui.add_system_message(&format!("Stream error: {}", e));
                        break;
                    }
                    chunk_count += 1;

                    // Also check for incoming audio response (non-blocking)
                    match eva_client.receive().await {
                        Ok(Some(EvaMindResponse::Audio(audio_data))) => {
                            response_chunks += 1;
                            if let Err(e) = audio_player.play_pcm(&audio_data).await {
                                terminal_ui.add_system_message(&format!("Playback error: {}", e));
                            }
                        }
                        Ok(Some(EvaMindResponse::Control(msg))) => {
                            if let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) {
                                terminal_ui.add_system_message(&format!("Control: {}", msg_type));
                            }
                        }
                        _ => {}
                    }
                }

                total_samples += audio_chunk.len();

                // Check for silence to end turn
                if vad.is_speech(&audio_chunk) {
                    silence_count = 0;
                } else {
                    silence_count += 1;
                    if silence_count > 15 {
                        // End of speech - stop streaming
                        break;
                    }
                }

                if total_samples > 48000 * 30 {
                    terminal_ui.add_system_message("Max recording time reached");
                    break;
                }

                // Animate listening
                if chunk_count % 5 == 0 {
                    statistics.update_all();
                    status_indicator.set_symbol(anim_listening.next_frame());
                    terminal_ui.draw(&status_indicator, &statistics);
                }
            }

            terminal_ui.add_system_message(&format!("Streamed {} chunks, received {} responses", chunk_count, response_chunks));
            statistics.turns += 1;
            terminal_ui.draw(&status_indicator, &statistics);

            // 4. Wait for response audio
            if let Some(ref mut eva_client) = eva_mind {
                status_indicator.set_status(EvaStatus::Speaking);
                terminal_ui.draw(&status_indicator, &statistics);

                let timeout = tokio::time::Duration::from_secs(15);
                let start = tokio::time::Instant::now();
                let mut received_audio = false;

                while start.elapsed() < timeout {
                    // Animate while waiting
                    statistics.update_all();
                    status_indicator.set_symbol(anim_speaking.next_frame());
                    terminal_ui.draw(&status_indicator, &statistics);

                    // Try to receive audio
                    match eva_client.receive().await {
                        Ok(Some(EvaMindResponse::Audio(audio_data))) => {
                            received_audio = true;
                            // Play audio (raw PCM bytes from EVA-Mind)
                            if let Err(e) = audio_player.play_pcm(&audio_data).await {
                                terminal_ui.add_system_message(&format!("Audio Playback Error: {}", e));
                            }
                        }
                        Ok(Some(EvaMindResponse::Control(msg))) => {
                            // Handle control messages
                            if let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) {
                                terminal_ui.add_system_message(&format!("Control: {}", msg_type));
                            }
                        }
                        Ok(None) => {
                            // No message, continue
                            if received_audio {
                                // Got audio, small pause then continue
                                break;
                            }
                        }
                        Err(e) => {
                            terminal_ui.add_system_message(&format!("Receive Error: {}", e));
                            break;
                        }
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                }

                if !received_audio {
                    terminal_ui.add_system_message("No audio response received");
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
