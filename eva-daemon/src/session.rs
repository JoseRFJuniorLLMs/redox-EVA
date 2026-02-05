use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, password_hash::SaltString};

/// Role in conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::User => write!(f, "User"),
            Role::Assistant => write!(f, "Assistant"),
        }
    }
}

/// A single turn in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub role: Role,
    pub content: String,
    #[serde(skip)] // Don't persist audio buffer to save space
    pub audio: Option<Vec<u8>>,
    #[serde(with = "serde_millis")]
    pub timestamp: SystemTime,
}

/// Helper module for SystemTime serialization
mod serde_millis {
    use std::time::{SystemTime, UNIX_EPOCH};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let nanos = time.duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        serializer.serialize_u64(nanos)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + std::time::Duration::from_millis(millis))
    }
}

/// Conversation session manager
#[derive(Serialize, Deserialize)]
pub struct ConversationSession {
    session_id: String,
    history: Vec<Turn>,
    context: HashMap<String, String>,
    #[serde(with = "serde_millis")]
    started_at: SystemTime,
    max_history: usize,
}

impl ConversationSession {
    /// Create a new conversation session
    pub fn new() -> Self {
        Self {
            session_id: Self::generate_session_id(),
            history: Vec::new(),
            context: HashMap::new(),
            started_at: SystemTime::now(),
            max_history: 10, // Keep last 10 turns
        }
    }

    /// Generate a unique session ID
    fn generate_session_id() -> String {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        
        format!("session_{}", duration.as_secs())
    }

    /// Save session to file (encrypted)
    ///
    /// Uses AES-256-GCM encryption with a key derived from machine-specific data.
    /// Falls back to plaintext if encryption fails (with warning).
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string(self)?;

        // Try to encrypt
        match Self::encrypt_data(json.as_bytes()) {
            Ok(encrypted) => {
                // Save with .enc extension marker (first 4 bytes)
                let mut data = b"ENC1".to_vec(); // Magic bytes + version
                data.extend(encrypted);
                fs::write(path, data)?;
            }
            Err(e) => {
                eprintln!("[Session] Warning: Encryption failed ({}), saving plaintext", e);
                fs::write(path, json)?;
            }
        }

        Ok(())
    }

    /// Load session from file (handles both encrypted and plaintext)
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = fs::read(&path)?;

        let json_str = if content.starts_with(b"ENC1") {
            // Encrypted file
            match Self::decrypt_data(&content[4..]) {
                Ok(decrypted) => String::from_utf8(decrypted)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Decryption failed: {}", e),
                    ));
                }
            }
        } else {
            // Plaintext file (legacy or failed encryption)
            String::from_utf8(content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
        };

        let mut session: Self = serde_json::from_str(&json_str)?;

        // Ensure max_history is set
        if session.max_history == 0 {
            session.max_history = 10;
        }

        Ok(session)
    }

    /// Derive encryption key from machine-specific data
    fn get_session_key() -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "eva_user".to_string());

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "eva_host".to_string());

        let password = format!("eva_session_{}_{}", username, hostname);

        // Use a fixed salt for deterministic key derivation
        // In production, this should be stored securely
        let salt = SaltString::from_b64("RXZhT1NTYWx0MTIzNA")
            .map_err(|e| format!("Salt error: {}", e))?;

        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Hash error: {}", e))?;

        let hash_bytes = hash.hash.ok_or("No hash generated")?;
        let mut key = [0u8; 32];
        let len = std::cmp::min(hash_bytes.len(), 32);
        key[..len].copy_from_slice(&hash_bytes.as_bytes()[..len]);

        Ok(key)
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let key = Self::get_session_key()?;
        let cipher = Aes256Gcm::new(&key.into());
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|e| format!("Encryption error: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);

        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt_data(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if data.len() < 12 {
            return Err("Data too short for decryption".into());
        }

        let key = Self::get_session_key()?;
        let cipher = Aes256Gcm::new(&key.into());

        // Extract nonce (first 12 bytes)
        let nonce = Nonce::from_slice(&data[..12]);
        let ciphertext = &data[12..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption error: {}", e))?;

        Ok(plaintext)
    }

    /// Add a turn to the conversation
    pub fn add_turn(&mut self, role: Role, content: String) {
        self.history.push(Turn {
            role,
            content,
            audio: None,
            timestamp: SystemTime::now(),
        });

        // Keep only last N turns
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Add a turn with audio
    pub fn add_turn_with_audio(&mut self, role: Role, content: String, audio: Vec<u8>) {
        self.history.push(Turn {
            role,
            content,
            audio: Some(audio),
            timestamp: SystemTime::now(),
        });

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Get conversation context as string
    pub fn get_context(&self) -> String {
        if self.history.is_empty() {
            return String::new();
        }

        self.history
            .iter()
            .map(|turn| format!("{}: {}", turn.role, turn.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get last N turns
    pub fn get_recent_turns(&self, n: usize) -> Vec<&Turn> {
        let start = if self.history.len() > n {
            self.history.len() - n
        } else {
            0
        };
        
        self.history[start..].iter().collect()
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get session duration
    pub fn duration(&self) -> std::time::Duration {
        SystemTime::now()
            .duration_since(self.started_at)
            .unwrap_or_default()
    }

    /// Get turn count
    pub fn turn_count(&self) -> usize {
        self.history.len()
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.history.clear();
        self.context.clear();
    }

    /// Check if session should continue
    pub fn should_continue(&self) -> bool {
        // Continue if last turn was from assistant
        // This allows for follow-up questions
        self.history
            .last()
            .map(|turn| turn.role == Role::Assistant)
            .unwrap_or(false)
    }

    /// Set context value
    pub fn set_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }

    /// Get context value
    pub fn get_context_value(&self, key: &str) -> Option<&String> {
        self.context.get(key)
    }
}

impl Default for ConversationSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ConversationSession::new();
        assert!(!session.session_id().is_empty());
        assert_eq!(session.turn_count(), 0);
    }

    #[test]
    fn test_add_turn() {
        let mut session = ConversationSession::new();
        
        session.add_turn(Role::User, "Hello".to_string());
        assert_eq!(session.turn_count(), 1);
        
        session.add_turn(Role::Assistant, "Hi there!".to_string());
        assert_eq!(session.turn_count(), 2);
    }

    #[test]
    fn test_context_building() {
        let mut session = ConversationSession::new();
        
        session.add_turn(Role::User, "What's the weather?".to_string());
        session.add_turn(Role::Assistant, "It's sunny today.".to_string());
        
        let context = session.get_context();
        assert!(context.contains("What's the weather?"));
        assert!(context.contains("It's sunny today."));
    }

    #[test]
    fn test_max_history() {
        let mut session = ConversationSession::new();
        session.max_history = 3;
        
        // Add 5 turns
        for i in 0..5 {
            session.add_turn(Role::User, format!("Message {}", i));
        }
        
        // Should only keep last 3
        assert_eq!(session.turn_count(), 3);
        
        let context = session.get_context();
        assert!(context.contains("Message 2"));
        assert!(context.contains("Message 4"));
        assert!(!context.contains("Message 0"));
    }

    #[test]
    fn test_recent_turns() {
        let mut session = ConversationSession::new();
        
        session.add_turn(Role::User, "First".to_string());
        session.add_turn(Role::Assistant, "Second".to_string());
        session.add_turn(Role::User, "Third".to_string());
        
        let recent = session.get_recent_turns(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "Second");
        assert_eq!(recent[1].content, "Third");
    }

    #[test]
    fn test_should_continue() {
        let mut session = ConversationSession::new();
        
        // Empty session - should not continue
        assert!(!session.should_continue());
        
        // User turn - should not continue
        session.add_turn(Role::User, "Hello".to_string());
        assert!(!session.should_continue());
        
        // Assistant turn - should continue
        session.add_turn(Role::Assistant, "Hi!".to_string());
        assert!(session.should_continue());
    }

    #[test]
    fn test_context_values() {
        let mut session = ConversationSession::new();

        session.set_context("user_name".to_string(), "Alice".to_string());
        session.set_context("location".to_string(), "Lisbon".to_string());

        assert_eq!(session.get_context_value("user_name"), Some(&"Alice".to_string()));
        assert_eq!(session.get_context_value("location"), Some(&"Lisbon".to_string()));
        assert_eq!(session.get_context_value("unknown"), None);
    }

    #[test]
    fn test_clear_session() {
        let mut session = ConversationSession::new();

        session.add_turn(Role::User, "Test".to_string());
        session.set_context("key".to_string(), "value".to_string());

        assert_eq!(session.turn_count(), 1);

        session.clear();

        assert_eq!(session.turn_count(), 0);
        assert_eq!(session.get_context_value("key"), None);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(format!("{}", Role::User), "User");
        assert_eq!(format!("{}", Role::Assistant), "Assistant");
    }

    #[test]
    fn test_add_turn_with_audio() {
        let mut session = ConversationSession::new();
        let audio_data = vec![1u8, 2, 3, 4];

        session.add_turn_with_audio(Role::User, "Test".to_string(), audio_data.clone());

        assert_eq!(session.turn_count(), 1);
        // Note: audio is not persisted, but should exist in memory
    }

    #[test]
    fn test_session_duration() {
        let session = ConversationSession::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = session.duration();
        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_encryption_roundtrip() {
        // Test that encryption/decryption works correctly
        let test_data = b"This is a secret message for EVA";

        let encrypted = ConversationSession::encrypt_data(test_data);
        assert!(encrypted.is_ok(), "Encryption should succeed");

        let encrypted_data = encrypted.unwrap();
        assert!(encrypted_data.len() > test_data.len(), "Encrypted data should be larger (nonce + ciphertext)");

        let decrypted = ConversationSession::decrypt_data(&encrypted_data);
        assert!(decrypted.is_ok(), "Decryption should succeed");

        assert_eq!(decrypted.unwrap(), test_data);
    }

    #[test]
    fn test_decrypt_too_short() {
        // Test that short data fails gracefully
        let short_data = vec![1, 2, 3, 4]; // Less than 12 bytes (nonce size)
        let result = ConversationSession::decrypt_data(&short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load() {
        let temp_path = std::env::temp_dir().join("eva_test_session.json");

        // Create and save session
        let mut session = ConversationSession::new();
        session.add_turn(Role::User, "Hello".to_string());
        session.add_turn(Role::Assistant, "Hi there!".to_string());
        session.set_context("test_key".to_string(), "test_value".to_string());

        let original_id = session.session_id().to_string();
        let original_count = session.turn_count();

        session.save_to_file(&temp_path).expect("Save should succeed");

        // Load and verify
        let loaded = ConversationSession::load_from_file(&temp_path).expect("Load should succeed");

        assert_eq!(loaded.session_id(), original_id);
        assert_eq!(loaded.turn_count(), original_count);
        assert_eq!(loaded.get_context_value("test_key"), Some(&"test_value".to_string()));

        // Cleanup
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_default_trait() {
        let session: ConversationSession = Default::default();
        assert!(!session.session_id().is_empty());
        assert_eq!(session.turn_count(), 0);
    }

    #[test]
    fn test_get_recent_turns_more_than_available() {
        let mut session = ConversationSession::new();
        session.add_turn(Role::User, "First".to_string());
        session.add_turn(Role::Assistant, "Second".to_string());

        // Request more turns than available
        let recent = session.get_recent_turns(10);
        assert_eq!(recent.len(), 2);
    }
}
