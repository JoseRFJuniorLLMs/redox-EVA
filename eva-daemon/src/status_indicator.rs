use std::fmt;
use std::time::SystemTime;

/// EVA status states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvaStatus {
    Initializing,   // Startup
    Idle,           // Waiting for wake word
    Listening,      // Recording command
    Processing,     // Sending to Gemini
    Speaking,       // Playing response
    Executing,      // Running command
    Error,          // Error state
}

impl fmt::Display for EvaStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaStatus::Initializing => write!(f, "⏳ Initializing"),
            EvaStatus::Idle => write!(f, "💤 Idle"),
            EvaStatus::Listening => write!(f, "👂 Listening"),
            EvaStatus::Processing => write!(f, "🧠 Processing"),
            EvaStatus::Speaking => write!(f, "🗣️  Speaking"),
            EvaStatus::Executing => write!(f, "⚙️  Executing"),
            EvaStatus::Error => write!(f, "❌ Error"),
        }
    }
}

use crate::emotion::Emotion;

/// Status indicator with history
pub struct StatusIndicator {
    current_status: EvaStatus,
    current_emotion: Emotion,
    status_history: Vec<(EvaStatus, SystemTime)>,
    max_history: usize,
    override_symbol: Option<String>,
}

impl StatusIndicator {
    /// Create a new status indicator
    pub fn new() -> Self {
        Self {
            current_status: EvaStatus::Initializing,
            current_emotion: Emotion::Neutral,
            status_history: Vec::new(),
            max_history: 100,
            override_symbol: None,
        }
    }

    /// Set current status
    pub fn set_status(&mut self, status: EvaStatus) {
        if self.current_status != status {
            self.status_history.push((self.current_status, SystemTime::now()));
            
            // Keep history limited
            if self.status_history.len() > self.max_history {
                self.status_history.remove(0);
            }
            
            self.current_status = status;
            self.override_symbol = None; // Reset override on status change
        }
    }

    /// Set current emotion
    pub fn set_emotion(&mut self, emotion: Emotion) {
        self.current_emotion = emotion;
    }

    /// Get current emotion
    pub fn get_emotion(&self) -> Emotion {
        self.current_emotion
    }

    /// Set a temporary symbol override (for animations)
    pub fn set_symbol(&mut self, symbol: &str) {
        self.override_symbol = Some(symbol.to_string());
    }

    /// Get current status
    pub fn get_status(&self) -> EvaStatus {
        self.current_status
    }

    /// Get status as string
    pub fn get_status_string(&self) -> String {
        let emotion_str = if self.current_emotion != Emotion::Neutral {
            format!(" | Emotion: {}", self.current_emotion)
        } else {
            String::new()
        };

        if let Some(ref symbol) = self.override_symbol {
            // Use the status name but replace the icon with the animation frame
            let status_name = match self.current_status {
                EvaStatus::Initializing => "Initializing",
                EvaStatus::Idle => "Idle",
                EvaStatus::Listening => "Listening",
                EvaStatus::Processing => "Processing",
                EvaStatus::Speaking => "Speaking",
                EvaStatus::Executing => "Executing",
                EvaStatus::Error => "Error",
            };
            format!("{} {}{}", symbol, status_name, emotion_str)
        } else {
            format!("{}{}", self.current_status, emotion_str)
        }
    }

    /// Get color for current status (for TUI)
    pub fn get_color_name(&self) -> &str {
        match self.current_status {
            EvaStatus::Initializing => "cyan",
            EvaStatus::Idle => "gray",
            EvaStatus::Listening => "yellow",
            EvaStatus::Processing => "blue",
            EvaStatus::Speaking => "green",
            EvaStatus::Executing => "cyan",
            EvaStatus::Error => "red",
        }
    }

    /// Get status history
    pub fn get_history(&self) -> &[(EvaStatus, SystemTime)] {
        &self.status_history
    }
}

impl Default for StatusIndicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_change() {
        let mut indicator = StatusIndicator::new();
        assert_eq!(indicator.get_status(), EvaStatus::Idle);

        indicator.set_status(EvaStatus::Listening);
        assert_eq!(indicator.get_status(), EvaStatus::Listening);
    }

    #[test]
    fn test_status_history() {
        let mut indicator = StatusIndicator::new();
        
        indicator.set_status(EvaStatus::Listening);
        indicator.set_status(EvaStatus::Processing);
        indicator.set_status(EvaStatus::Speaking);

        assert_eq!(indicator.get_history().len(), 3);
    }

    #[test]
    fn test_status_display() {
        let indicator = StatusIndicator::new();
        assert_eq!(indicator.get_status_string(), "💤 Idle");
    }

    #[test]
    fn test_color_names() {
        let mut indicator = StatusIndicator::new();
        
        indicator.set_status(EvaStatus::Listening);
        assert_eq!(indicator.get_color_name(), "yellow");
        
        indicator.set_status(EvaStatus::Error);
        assert_eq!(indicator.get_color_name(), "red");
    }
}
