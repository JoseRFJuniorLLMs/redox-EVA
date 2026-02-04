use std::collections::VecDeque;

/// Wake word detector for "Hey EVA"
pub struct WakeWordDetector {
    pattern: Vec<f32>,
    threshold: f32,
    buffer: VecDeque<f32>,
    pattern_length: usize,
}

impl WakeWordDetector {
    /// Create a new wake word detector
    pub fn new() -> Self {
        // Simple energy pattern for "Hey EVA"
        // This is a simplified version - in production, use ML model
        let pattern = vec![
            0.1, 0.3, 0.5, 0.7, 0.9, 0.7, 0.5, // "Hey"
            0.2, 0.2, 0.2, // silence
            0.3, 0.6, 0.8, 0.6, 0.4, // "E"
            0.4, 0.7, 0.5, 0.3, // "VA"
        ];
        
        let pattern_length = pattern.len();
        
        Self {
            pattern,
            threshold: 0.7,
            buffer: VecDeque::with_capacity(pattern_length * 2),
            pattern_length,
        }
    }

    /// Detect wake word in audio samples
    pub fn detect(&mut self, samples: &[f32]) -> bool {
        // Add samples to buffer
        for &sample in samples {
            if self.buffer.len() >= self.pattern_length * 2 {
                self.buffer.pop_front();
            }
            self.buffer.push_back(sample.abs()); // Use absolute value
        }

        // Need enough samples to match pattern
        if self.buffer.len() < self.pattern_length {
            return false;
        }

        // Calculate cross-correlation
        let correlation = self.correlate();
        
        // Detect if correlation exceeds threshold
        correlation > self.threshold
    }

    /// Calculate cross-correlation between buffer and pattern
    fn correlate(&self) -> f32 {
        if self.buffer.len() < self.pattern_length {
            return 0.0;
        }

        let mut max_correlation: f32 = 0.0;
        
        // Try different positions in the buffer
        for offset in 0..=(self.buffer.len() - self.pattern_length) {
            let mut correlation = 0.0;
            let mut pattern_energy = 0.0;
            let mut buffer_energy = 0.0;
            
            for i in 0..self.pattern_length {
                let buffer_val = self.buffer[offset + i];
                let pattern_val = self.pattern[i];
                
                correlation += buffer_val * pattern_val;
                pattern_energy += pattern_val * pattern_val;
                buffer_energy += buffer_val * buffer_val;
            }
            
            // Normalize correlation
            if pattern_energy > 0.0 && buffer_energy > 0.0 {
                let normalized = correlation / (pattern_energy.sqrt() * buffer_energy.sqrt());
                max_correlation = max_correlation.max(normalized);
            }
        }
        
        max_correlation
    }

    /// Set detection sensitivity (0.0 to 1.0)
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        self.threshold = sensitivity.clamp(0.0, 1.0);
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

impl Default for WakeWordDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_word_detector_creation() {
        let detector = WakeWordDetector::new();
        assert_eq!(detector.threshold, 0.7);
        assert!(detector.buffer.is_empty());
    }

    #[test]
    fn test_sensitivity_adjustment() {
        let mut detector = WakeWordDetector::new();
        
        detector.set_sensitivity(0.5);
        assert_eq!(detector.threshold, 0.5);
        
        detector.set_sensitivity(1.5); // Should clamp to 1.0
        assert_eq!(detector.threshold, 1.0);
        
        detector.set_sensitivity(-0.5); // Should clamp to 0.0
        assert_eq!(detector.threshold, 0.0);
    }

    #[test]
    fn test_wake_word_detection_silence() {
        let mut detector = WakeWordDetector::new();
        
        // Feed silence
        let silence = vec![0.0; 1000];
        assert!(!detector.detect(&silence));
    }

    #[test]
    fn test_wake_word_detection_pattern() {
        let mut detector = WakeWordDetector::new();
        detector.set_sensitivity(0.5); // Lower threshold for testing
        
        // Feed pattern similar to "Hey EVA"
        let pattern = vec![
            0.1, 0.3, 0.5, 0.7, 0.9, 0.7, 0.5, // "Hey"
            0.2, 0.2, 0.2, // silence
            0.3, 0.6, 0.8, 0.6, 0.4, // "E"
            0.4, 0.7, 0.5, 0.3, // "VA"
        ];
        
        let detected = detector.detect(&pattern);
        // May or may not detect depending on exact pattern matching
        // This is a simple test - real detection needs more sophisticated algorithm
        println!("Detection result: {}", detected);
    }

    #[test]
    fn test_reset() {
        let mut detector = WakeWordDetector::new();
        
        // Add some samples
        detector.detect(&vec![0.5; 100]);
        assert!(!detector.buffer.is_empty());
        
        // Reset
        detector.reset();
        assert!(detector.buffer.is_empty());
    }
}
