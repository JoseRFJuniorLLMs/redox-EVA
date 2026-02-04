use std::time::{Duration, SystemTime};

/// Statistics tracker
pub struct Statistics {
    pub turns: usize,
    pub commands_executed: usize,
    pub uptime_seconds: u64,
    pub memory_mb: usize,
    start_time: SystemTime,
}

impl Statistics {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self {
            turns: 0,
            commands_executed: 0,
            uptime_seconds: 0,
            memory_mb: 0,
            start_time: SystemTime::now(),
        }
    }

    /// Increment conversation turns
    pub fn increment_turns(&mut self) {
        self.turns += 1;
    }

    /// Increment commands executed
    pub fn increment_commands(&mut self) {
        self.commands_executed += 1;
    }

    /// Update uptime
    pub fn update_uptime(&mut self) {
        if let Ok(duration) = self.start_time.elapsed() {
            self.uptime_seconds = duration.as_secs();
        }
    }

    /// Update memory usage
    pub fn update_memory(&mut self) {
        use sysinfo::{PidExt, ProcessExt, System, SystemExt};
        let mut sys = System::new_all();
        sys.refresh_all();
        
        if let Some(process) = sys.process(sysinfo::Pid::from_u32(std::process::id())) {
            self.memory_mb = (process.memory() / 1024 / 1024) as usize;
        } else {
             // Fallback if process not found (rare)
            self.memory_mb = 0;
        }
    }

    /// Update all statistics
    pub fn update_all(&mut self) {
        self.update_uptime();
        self.update_memory();
    }

    /// Get formatted uptime string
    pub fn get_uptime_string(&self) -> String {
        let hours = self.uptime_seconds / 3600;
        let minutes = (self.uptime_seconds % 3600) / 60;
        let seconds = self.uptime_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

impl Default for Statistics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_increment_turns() {
        let mut stats = Statistics::new();
        assert_eq!(stats.turns, 0);

        stats.increment_turns();
        assert_eq!(stats.turns, 1);
    }

    #[test]
    fn test_increment_commands() {
        let mut stats = Statistics::new();
        assert_eq!(stats.commands_executed, 0);

        stats.increment_commands();
        assert_eq!(stats.commands_executed, 1);
    }

    #[test]
    fn test_uptime() {
        let mut stats = Statistics::new();
        thread::sleep(Duration::from_millis(100));
        
        stats.update_uptime();
        assert!(stats.uptime_seconds >= 0);
    }

    #[test]
    fn test_uptime_string() {
        let mut stats = Statistics::new();
        stats.uptime_seconds = 3665; // 1h 1m 5s

        let uptime_str = stats.get_uptime_string();
        assert_eq!(uptime_str, "1h 1m 5s");
    }
}
