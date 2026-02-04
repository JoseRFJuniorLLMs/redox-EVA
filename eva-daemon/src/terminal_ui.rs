use crate::status_indicator::{EvaStatus, StatusIndicator};
use crate::statistics::Statistics;
use std::io::{self, Write};

/// Simple terminal UI (without heavy TUI dependencies)
pub struct TerminalUI {
    conversation_log: Vec<String>,
    max_log_size: usize,
}

impl TerminalUI {
    /// Create new terminal UI
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            conversation_log: Vec::new(),
            max_log_size: 50,
        })
    }

    /// Clear screen
    pub fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().ok();
    }

    /// Draw header
    pub fn draw_header(&self) {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║          🧠 EVA OS v0.8.0 - Visual Feedback              ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!();
    }

    /// Draw status bar
    pub fn draw_status(&self, status: &StatusIndicator) {
        let status_str = status.get_status_string();
        let color = self.get_ansi_color(status.get_color_name());
        
        println!("┌─ Status ────────────────────────────────────────────────┐");
        println!("│ {}{}\x1B[0m", color, status_str);
        println!("└─────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Draw statistics
    pub fn draw_statistics(&self, stats: &Statistics) {
        println!("┌─ Statistics ────────────────────────────────────────────┐");
        println!("│ Turns: {} | Commands: {} | Uptime: {} | Memory: {}MB",
            stats.turns,
            stats.commands_executed,
            stats.get_uptime_string(),
            stats.memory_mb
        );
        println!("└─────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Draw conversation log
    pub fn draw_conversation(&self) {
        println!("┌─ Conversation ──────────────────────────────────────────┐");
        
        let start = if self.conversation_log.len() > 10 {
            self.conversation_log.len() - 10
        } else {
            0
        };
        
        for msg in &self.conversation_log[start..] {
            println!("│ {}", msg);
        }
        
        if self.conversation_log.is_empty() {
            println!("│ (No messages yet)");
        }
        
        println!("└─────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Draw complete UI
    pub fn draw(&self, status: &StatusIndicator, stats: &Statistics) {
        self.clear_screen();
        self.draw_header();
        self.draw_status(status);
        self.draw_statistics(stats);
        self.draw_conversation();
    }

    /// Add message to conversation log
    pub fn add_message(&mut self, message: String) {
        self.conversation_log.push(message);
        
        // Keep log size limited
        if self.conversation_log.len() > self.max_log_size {
            self.conversation_log.remove(0);
        }
    }

    /// Add user message
    pub fn add_user_message(&mut self, message: &str) {
        self.add_message(format!("👤 User: {}", message));
    }

    /// Add EVA message
    pub fn add_eva_message(&mut self, message: &str) {
        self.add_message(format!("🤖 EVA: {}", message));
    }

    /// Add system message
    pub fn add_system_message(&mut self, message: &str) {
        self.add_message(format!("ℹ️  System: {}", message));
    }

    /// Get ANSI color code
    fn get_ansi_color(&self, color_name: &str) -> &str {
        match color_name {
            "gray" => "\x1B[90m",
            "yellow" => "\x1B[33m",
            "blue" => "\x1B[34m",
            "green" => "\x1B[32m",
            "cyan" => "\x1B[36m",
            "red" => "\x1B[31m",
            _ => "\x1B[0m",
        }
    }

    /// Cleanup (placeholder for compatibility)
    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.clear_screen();
        Ok(())
    }
}

impl Default for TerminalUI {
    fn default() -> Self {
        Self::new().expect("Failed to create TerminalUI")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_message() {
        let mut ui = TerminalUI::new().unwrap();
        ui.add_message("Test message".to_string());
        
        assert_eq!(ui.conversation_log.len(), 1);
    }

    #[test]
    fn test_max_log_size() {
        let mut ui = TerminalUI::new().unwrap();
        
        for i in 0..60 {
            ui.add_message(format!("Message {}", i));
        }
        
        assert_eq!(ui.conversation_log.len(), 50);
    }

    #[test]
    fn test_user_message() {
        let mut ui = TerminalUI::new().unwrap();
        ui.add_user_message("Hello");
        
        assert!(ui.conversation_log[0].contains("User:"));
    }

    #[test]
    fn test_eva_message() {
        let mut ui = TerminalUI::new().unwrap();
        ui.add_eva_message("Hello");
        
        assert!(ui.conversation_log[0].contains("EVA:"));
    }
}
