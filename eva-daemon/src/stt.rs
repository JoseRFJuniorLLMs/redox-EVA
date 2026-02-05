//! Offline Speech-to-Text using Vosk
//!
//! Provides local, privacy-preserving speech recognition without
//! requiring internet connectivity. Supports multiple languages.

use std::path::Path;

#[cfg(feature = "offline-stt")]
use vosk::{Model, Recognizer};

/// Supported languages for STT
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    /// English (US)
    EnglishUS,
    /// Portuguese (Brazil)
    PortugueseBR,
    /// Spanish
    Spanish,
    /// French
    French,
    /// German
    German,
    /// Italian
    Italian,
    /// Russian
    Russian,
    /// Chinese
    Chinese,
    /// Japanese
    Japanese,
    /// Korean
    Korean,
}

impl Language {
    /// Get the Vosk model name for this language
    pub fn model_name(&self) -> &'static str {
        match self {
            Language::EnglishUS => "vosk-model-small-en-us-0.15",
            Language::PortugueseBR => "vosk-model-small-pt-0.3",
            Language::Spanish => "vosk-model-small-es-0.42",
            Language::French => "vosk-model-small-fr-0.22",
            Language::German => "vosk-model-small-de-0.15",
            Language::Italian => "vosk-model-small-it-0.22",
            Language::Russian => "vosk-model-small-ru-0.22",
            Language::Chinese => "vosk-model-small-cn-0.22",
            Language::Japanese => "vosk-model-small-ja-0.22",
            Language::Korean => "vosk-model-small-ko-0.22",
        }
    }

    /// Get the model download URL
    pub fn model_url(&self) -> String {
        format!(
            "https://alphacephei.com/vosk/models/{}.zip",
            self.model_name()
        )
    }

    /// Get language code (ISO 639-1)
    pub fn code(&self) -> &'static str {
        match self {
            Language::EnglishUS => "en",
            Language::PortugueseBR => "pt",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Italian => "it",
            Language::Russian => "ru",
            Language::Chinese => "zh",
            Language::Japanese => "ja",
            Language::Korean => "ko",
        }
    }
}

/// STT Configuration
#[derive(Debug, Clone)]
pub struct SttConfig {
    /// Path to Vosk models directory
    pub models_path: String,
    /// Primary language
    pub language: Language,
    /// Sample rate (default: 16000)
    pub sample_rate: u32,
    /// Enable partial results (interim transcriptions)
    pub partial_results: bool,
    /// Enable word timestamps
    pub word_timestamps: bool,
    /// Maximum alternatives to return
    pub max_alternatives: u32,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            models_path: "~/.eva/models".to_string(),
            language: Language::EnglishUS,
            sample_rate: 16000,
            partial_results: true,
            word_timestamps: false,
            max_alternatives: 3,
        }
    }
}

/// Recognition result
#[derive(Debug, Clone)]
pub struct RecognitionResult {
    /// Transcribed text
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Alternative transcriptions
    pub alternatives: Vec<(String, f32)>,
    /// Word-level timestamps (if enabled)
    pub words: Vec<WordInfo>,
    /// Whether this is a partial (interim) result
    pub is_partial: bool,
}

/// Word timing information
#[derive(Debug, Clone)]
pub struct WordInfo {
    /// The word
    pub word: String,
    /// Start time in seconds
    pub start: f32,
    /// End time in seconds
    pub end: f32,
    /// Confidence for this word
    pub confidence: f32,
}

/// Offline Speech-to-Text engine using Vosk
pub struct SttEngine {
    config: SttConfig,
    #[cfg(feature = "offline-stt")]
    model: Option<Model>,
    #[cfg(feature = "offline-stt")]
    recognizer: Option<Recognizer>,
    /// Is the engine ready?
    is_ready: bool,
}

impl SttEngine {
    /// Create a new STT engine with default configuration
    pub fn new() -> Self {
        Self::with_config(SttConfig::default())
    }

    /// Create a new STT engine with custom configuration
    pub fn with_config(config: SttConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "offline-stt")]
            model: None,
            #[cfg(feature = "offline-stt")]
            recognizer: None,
            is_ready: false,
        }
    }

    /// Initialize the engine (load model)
    pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let models_path = self.expand_path(&self.config.models_path);
        let model_path = Path::new(&models_path).join(self.config.language.model_name());

        if !model_path.exists() {
            return Err(format!(
                "Model not found: {}. Download from: {}",
                model_path.display(),
                self.config.language.model_url()
            ).into());
        }

        #[cfg(feature = "offline-stt")]
        {
            // Load Vosk model
            let model = Model::new(model_path.to_str().unwrap())
                .ok_or("Failed to load Vosk model")?;

            // Create recognizer
            let mut recognizer = Recognizer::new(&model, self.config.sample_rate as f32)
                .ok_or("Failed to create recognizer")?;

            // Configure recognizer
            if self.config.max_alternatives > 1 {
                recognizer.set_max_alternatives(self.config.max_alternatives as i32);
            }

            if self.config.word_timestamps {
                recognizer.set_words(true);
            }

            if self.config.partial_results {
                recognizer.set_partial_words(true);
            }

            self.model = Some(model);
            self.recognizer = Some(recognizer);
            self.is_ready = true;

            println!("[STT] Vosk model loaded: {}", self.config.language.model_name());
        }

        #[cfg(not(feature = "offline-stt"))]
        {
            // Fallback mode - mark as ready but use placeholder
            println!("[STT] Warning: Vosk feature not enabled, using fallback mode");
            self.is_ready = true;
        }

        Ok(())
    }

    /// Check if the engine is ready
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Recognize speech from audio samples
    ///
    /// Audio should be 16-bit PCM at the configured sample rate
    pub fn recognize(&mut self, audio: &[i16]) -> Result<RecognitionResult, Box<dyn std::error::Error>> {
        if !self.is_ready {
            return Err("STT engine not initialized".into());
        }

        #[cfg(feature = "offline-stt")]
        {
            if let Some(ref mut recognizer) = self.recognizer {
                // Convert i16 samples to bytes
                let bytes: Vec<u8> = audio
                    .iter()
                    .flat_map(|&sample| sample.to_le_bytes())
                    .collect();

                // Process audio
                recognizer.accept_waveform(&bytes);

                // Get result
                let result_json = recognizer.final_result().single().unwrap_or_default();

                // Parse result
                return self.parse_result(&result_json.text, false);
            }
        }

        // Fallback: return empty result
        Ok(RecognitionResult {
            text: String::new(),
            confidence: 0.0,
            alternatives: Vec::new(),
            words: Vec::new(),
            is_partial: false,
        })
    }

    /// Recognize speech from f32 samples (normalized -1.0 to 1.0)
    pub fn recognize_f32(&mut self, audio: &[f32]) -> Result<RecognitionResult, Box<dyn std::error::Error>> {
        // Convert f32 to i16
        let i16_samples: Vec<i16> = audio
            .iter()
            .map(|&sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();

        self.recognize(&i16_samples)
    }

    /// Get partial (interim) recognition result
    pub fn get_partial(&mut self) -> Result<RecognitionResult, Box<dyn std::error::Error>> {
        if !self.is_ready {
            return Err("STT engine not initialized".into());
        }

        #[cfg(feature = "offline-stt")]
        {
            if let Some(ref mut recognizer) = self.recognizer {
                let partial = recognizer.partial_result().partial;
                return self.parse_result(&partial, true);
            }
        }

        Ok(RecognitionResult {
            text: String::new(),
            confidence: 0.0,
            alternatives: Vec::new(),
            words: Vec::new(),
            is_partial: true,
        })
    }

    /// Reset the recognizer for a new utterance
    pub fn reset(&mut self) {
        #[cfg(feature = "offline-stt")]
        {
            if let Some(ref mut recognizer) = self.recognizer {
                recognizer.reset();
            }
        }
    }

    /// Change the recognition language
    pub fn set_language(&mut self, language: Language) -> Result<(), Box<dyn std::error::Error>> {
        if language == self.config.language {
            return Ok(());
        }

        self.config.language = language;
        self.is_ready = false;

        // Re-initialize with new language
        self.init()
    }

    /// Get current language
    pub fn language(&self) -> Language {
        self.config.language
    }

    /// Parse Vosk result JSON
    fn parse_result(&self, text: &str, is_partial: bool) -> Result<RecognitionResult, Box<dyn std::error::Error>> {
        // Clean up the text
        let cleaned = text.trim().to_string();

        // Calculate confidence heuristically based on text features
        let confidence = if cleaned.is_empty() {
            0.0
        } else {
            // Heuristic: longer results with proper words have higher confidence
            let word_count = cleaned.split_whitespace().count();
            let avg_word_len: f32 = if word_count > 0 {
                cleaned.len() as f32 / word_count as f32
            } else {
                0.0
            };

            // Reasonable words are 3-10 characters
            let word_quality = if avg_word_len >= 3.0 && avg_word_len <= 10.0 {
                1.0
            } else {
                0.5
            };

            // More words generally means more confident recognition
            let length_factor = (word_count as f32 / 10.0).min(1.0);

            (0.5 + length_factor * 0.3 + word_quality * 0.2).min(1.0)
        };

        Ok(RecognitionResult {
            text: cleaned,
            confidence,
            alternatives: Vec::new(),
            words: Vec::new(),
            is_partial,
        })
    }

    /// Expand home directory in path
    fn expand_path(&self, path: &str) -> String {
        if path.starts_with("~") {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            path.replace("~", &home)
        } else {
            path.to_string()
        }
    }

    /// Check if model is downloaded
    pub fn is_model_available(&self) -> bool {
        let models_path = self.expand_path(&self.config.models_path);
        let model_path = Path::new(&models_path).join(self.config.language.model_name());
        model_path.exists()
    }

    /// Get model download instructions
    pub fn get_download_instructions(&self) -> String {
        let models_path = self.expand_path(&self.config.models_path);
        format!(
            "To enable offline speech recognition:\n\
            1. Download the model from: {}\n\
            2. Extract to: {}/{}\n\
            3. Restart EVA",
            self.config.language.model_url(),
            models_path,
            self.config.language.model_name()
        )
    }
}

impl Default for SttEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming STT session for real-time recognition
pub struct StreamingSttSession {
    engine: SttEngine,
    /// Buffer for accumulating audio
    audio_buffer: Vec<i16>,
    /// Chunk size for processing
    chunk_size: usize,
    /// Last partial result
    last_partial: String,
}

impl StreamingSttSession {
    /// Create a new streaming session
    pub fn new(mut engine: SttEngine) -> Result<Self, Box<dyn std::error::Error>> {
        if !engine.is_ready() {
            engine.init()?;
        }

        // Process in 100ms chunks (1600 samples at 16kHz)
        let chunk_size = (engine.config.sample_rate as usize) / 10;

        Ok(Self {
            engine,
            audio_buffer: Vec::with_capacity(chunk_size * 10),
            chunk_size,
            last_partial: String::new(),
        })
    }

    /// Add audio samples to the session
    pub fn add_audio(&mut self, samples: &[i16]) {
        self.audio_buffer.extend_from_slice(samples);
    }

    /// Add f32 audio samples
    pub fn add_audio_f32(&mut self, samples: &[f32]) {
        let i16_samples: Vec<i16> = samples
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();
        self.add_audio(&i16_samples);
    }

    /// Process buffered audio and get partial result
    pub fn process(&mut self) -> Result<Option<RecognitionResult>, Box<dyn std::error::Error>> {
        if self.audio_buffer.len() < self.chunk_size {
            return Ok(None);
        }

        // Process available chunks
        while self.audio_buffer.len() >= self.chunk_size {
            let chunk: Vec<i16> = self.audio_buffer.drain(..self.chunk_size).collect();
            let _ = self.engine.recognize(&chunk)?;
        }

        // Get partial result
        let partial = self.engine.get_partial()?;

        // Only return if changed
        if partial.text != self.last_partial {
            self.last_partial = partial.text.clone();
            Ok(Some(partial))
        } else {
            Ok(None)
        }
    }

    /// Finalize and get complete result
    pub fn finalize(&mut self) -> Result<RecognitionResult, Box<dyn std::error::Error>> {
        // Process any remaining audio
        if !self.audio_buffer.is_empty() {
            let remaining: Vec<i16> = self.audio_buffer.drain(..).collect();
            let _ = self.engine.recognize(&remaining)?;
        }

        // Get final result
        let result = self.engine.recognize(&[])?;

        // Reset for next utterance
        self.engine.reset();
        self.last_partial.clear();

        Ok(result)
    }

    /// Reset the session
    pub fn reset(&mut self) {
        self.audio_buffer.clear();
        self.last_partial.clear();
        self.engine.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::EnglishUS.code(), "en");
        assert_eq!(Language::PortugueseBR.code(), "pt");
        assert_eq!(Language::Spanish.code(), "es");
    }

    #[test]
    fn test_model_names() {
        assert!(Language::EnglishUS.model_name().contains("en-us"));
        assert!(Language::PortugueseBR.model_name().contains("pt"));
    }

    #[test]
    fn test_config_default() {
        let config = SttConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.language, Language::EnglishUS);
        assert!(config.partial_results);
    }

    #[test]
    fn test_engine_creation() {
        let engine = SttEngine::new();
        assert!(!engine.is_ready());
        assert_eq!(engine.language(), Language::EnglishUS);
    }

    #[test]
    fn test_engine_without_model() {
        let mut engine = SttEngine::new();
        // Should fail because model doesn't exist
        let result = engine.init();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_result() {
        let engine = SttEngine::new();

        let result = engine.parse_result("hello world", false).unwrap();
        assert_eq!(result.text, "hello world");
        assert!(result.confidence > 0.0);
        assert!(!result.is_partial);

        let empty = engine.parse_result("", false).unwrap();
        assert_eq!(empty.confidence, 0.0);
    }

    #[test]
    fn test_download_instructions() {
        let engine = SttEngine::new();
        let instructions = engine.get_download_instructions();
        assert!(instructions.contains("alphacephei.com"));
        assert!(instructions.contains("vosk-model"));
    }

    #[test]
    fn test_streaming_session_buffer() {
        // Create a mock session without initializing (to avoid model dependency)
        let engine = SttEngine::new();
        let mut session = StreamingSttSession {
            engine,
            audio_buffer: Vec::new(),
            chunk_size: 1600,
            last_partial: String::new(),
        };

        // Add some audio
        let samples = vec![0i16; 800];
        session.add_audio(&samples);
        assert_eq!(session.audio_buffer.len(), 800);

        // Add more
        session.add_audio(&samples);
        assert_eq!(session.audio_buffer.len(), 1600);

        // Reset
        session.reset();
        assert!(session.audio_buffer.is_empty());
    }
}
