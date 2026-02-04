use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

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

    /// Save session to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load session from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut session: Self = serde_json::from_str(&content)?;
        
        // Ensure max_history is set (in case it wasn't in the file or we change defaults)
        if session.max_history == 0 {
            session.max_history = 10;
        }
        
        Ok(session)
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
}
