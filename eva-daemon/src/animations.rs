use std::time::Duration;

/// Animation frames for visual feedback
pub struct Animation {
    frames: Vec<String>,
    current_frame: usize,
    frame_duration: Duration,
}

impl Animation {
    /// Listening animation
    pub fn listening() -> Self {
        Self {
            frames: vec![
                "👂    ".to_string(),
                " 👂   ".to_string(),
                "  👂  ".to_string(),
                "   👂 ".to_string(),
                "    👂".to_string(),
                "   👂 ".to_string(),
                "  👂  ".to_string(),
                " 👂   ".to_string(),
            ],
            current_frame: 0,
            frame_duration: Duration::from_millis(150),
        }
    }

    /// Processing animation (spinner)
    pub fn processing() -> Self {
        Self {
            frames: vec![
                "🧠⠋".to_string(),
                "🧠⠙".to_string(),
                "🧠⠹".to_string(),
                "🧠⠸".to_string(),
                "🧠⠼".to_string(),
                "🧠⠴".to_string(),
                "🧠⠦".to_string(),
                "🧠⠧".to_string(),
                "🧠⠇".to_string(),
                "🧠⠏".to_string(),
            ],
            current_frame: 0,
            frame_duration: Duration::from_millis(80),
        }
    }

    /// Speaking animation
    pub fn speaking() -> Self {
        Self {
            frames: vec![
                "🗣️ ▁".to_string(),
                "🗣️ ▂".to_string(),
                "🗣️ ▃".to_string(),
                "🗣️ ▄".to_string(),
                "🗣️ ▅".to_string(),
                "🗣️ ▆".to_string(),
                "🗣️ ▇".to_string(),
                "🗣️ █".to_string(),
                "🗣️ ▇".to_string(),
                "🗣️ ▆".to_string(),
                "🗣️ ▅".to_string(),
                "🗣️ ▄".to_string(),
                "🗣️ ▃".to_string(),
                "🗣️ ▂".to_string(),
            ],
            current_frame: 0,
            frame_duration: Duration::from_millis(100),
        }
    }

    /// Executing animation
    pub fn executing() -> Self {
        Self {
            frames: vec![
                "⚙️ ◐".to_string(),
                "⚙️ ◓".to_string(),
                "⚙️ ◑".to_string(),
                "⚙️ ◒".to_string(),
            ],
            current_frame: 0,
            frame_duration: Duration::from_millis(200),
        }
    }

    /// Get next frame
    pub fn next_frame(&mut self) -> &str {
        let frame = &self.frames[self.current_frame];
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        frame
    }

    /// Get current frame without advancing
    pub fn current_frame(&self) -> &str {
        &self.frames[self.current_frame]
    }

    /// Get frame duration
    pub fn frame_duration(&self) -> Duration {
        self.frame_duration
    }

    /// Reset to first frame
    pub fn reset(&mut self) {
        self.current_frame = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listening_animation() {
        let mut anim = Animation::listening();
        let first = anim.next_frame().to_string();
        let second = anim.next_frame().to_string();
        
        assert_ne!(first, second);
    }

    #[test]
    fn test_animation_cycles() {
        let mut anim = Animation::processing();
        let frame_count = 10;
        
        for _ in 0..frame_count {
            anim.next_frame();
        }
        
        // Should cycle back
        anim.next_frame();
    }

    #[test]
    fn test_frame_duration() {
        let anim = Animation::listening();
        assert_eq!(anim.frame_duration(), Duration::from_millis(150));
    }

    #[test]
    fn test_reset() {
        let mut anim = Animation::processing();
        anim.next_frame();
        anim.next_frame();
        
        anim.reset();
        assert_eq!(anim.current_frame, 0);
    }
}
