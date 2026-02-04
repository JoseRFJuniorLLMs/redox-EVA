/// Voice Activity Detection (VAD) module
pub struct VAD {
    energy_threshold: f32,
    zcr_threshold: f32,
    silence_frames: usize,
    speech_frames: usize,
    current_silence: usize,
    current_speech: usize,
}

impl VAD {
    /// Create a new VAD with default thresholds
    pub fn new() -> Self {
        Self {
            energy_threshold: 0.02,
            zcr_threshold: 0.1,
            silence_frames: 10,  // ~1 second at 100ms chunks
            speech_frames: 3,    // ~300ms
            current_silence: 0,
            current_speech: 0,
        }
    }

    /// Check if audio contains speech
    pub fn is_speech(&mut self, samples: &[f32]) -> bool {
        let energy = self.calculate_energy(samples);
        let zcr = self.zero_crossing_rate(samples);
        
        // Speech detected if both energy and ZCR are above thresholds
        let is_active = energy > self.energy_threshold && zcr > self.zcr_threshold;
        
        if is_active {
            self.current_speech += 1;
            self.current_silence = 0;
            
            // Require minimum speech frames
            self.current_speech >= self.speech_frames
        } else {
            self.current_silence += 1;
            self.current_speech = 0;
            
            // Not silence yet if we haven't had enough silence frames
            self.current_silence < self.silence_frames
        }
    }

    /// Calculate audio energy (RMS)
    fn calculate_energy(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let sum: f32 = samples.iter().map(|&s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    /// Calculate zero-crossing rate
    fn zero_crossing_rate(&self, samples: &[f32]) -> f32 {
        if samples.len() < 2 {
            return 0.0;
        }
        
        let mut crossings = 0;
        for i in 1..samples.len() {
            if (samples[i] >= 0.0 && samples[i - 1] < 0.0) ||
               (samples[i] < 0.0 && samples[i - 1] >= 0.0) {
                crossings += 1;
            }
        }
        
        crossings as f32 / samples.len() as f32
    }

    /// Set energy threshold
    pub fn set_energy_threshold(&mut self, threshold: f32) {
        self.energy_threshold = threshold.max(0.0);
    }

    /// Set zero-crossing rate threshold
    pub fn set_zcr_threshold(&mut self, threshold: f32) {
        self.zcr_threshold = threshold.max(0.0);
    }

    /// Reset VAD state
    pub fn reset(&mut self) {
        self.current_silence = 0;
        self.current_speech = 0;
    }

    /// Get current energy threshold
    pub fn energy_threshold(&self) -> f32 {
        self.energy_threshold
    }

    /// Get current ZCR threshold
    pub fn zcr_threshold(&self) -> f32 {
        self.zcr_threshold
    }
}

impl Default for VAD {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_creation() {
        let vad = VAD::new();
        assert_eq!(vad.energy_threshold, 0.02);
        assert_eq!(vad.zcr_threshold, 0.1);
    }

    #[test]
    fn test_energy_calculation() {
        let vad = VAD::new();
        
        // Silence
        let silence = vec![0.0; 100];
        assert_eq!(vad.calculate_energy(&silence), 0.0);
        
        // Full scale
        let loud = vec![1.0; 100];
        assert_eq!(vad.calculate_energy(&loud), 1.0);
        
        // Half scale
        let medium = vec![0.5; 100];
        assert_eq!(vad.calculate_energy(&medium), 0.5);
    }

    #[test]
    fn test_zero_crossing_rate() {
        let vad = VAD::new();
        
        // No crossings (DC signal)
        let dc = vec![0.5; 100];
        assert_eq!(vad.zero_crossing_rate(&dc), 0.0);
        
        // Maximum crossings (alternating signal)
        let alternating: Vec<f32> = (0..100).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let zcr = vad.zero_crossing_rate(&alternating);
        assert!(zcr > 0.9); // Should be close to 1.0
    }

    #[test]
    fn test_speech_detection_silence() {
        let mut vad = VAD::new();
        
        // Feed silence
        let silence = vec![0.0; 100];
        
        // Should not detect speech in silence
        for _ in 0..20 {
            assert!(!vad.is_speech(&silence));
        }
    }

    #[test]
    fn test_speech_detection_loud_signal() {
        let mut vad = VAD::new();
        vad.set_energy_threshold(0.01); // Lower threshold for testing
        
        // Create a signal with energy and zero crossings (sine-like)
        use std::f32::consts::PI;
        let speech: Vec<f32> = (0..100)
            .map(|i| (2.0 * PI * i as f32 / 10.0).sin() * 0.5)
            .collect();
        
        // Feed speech multiple times to trigger detection
        for _ in 0..5 {
            vad.is_speech(&speech);
        }
        
        // After enough frames, should detect speech
        assert!(vad.is_speech(&speech));
    }

    #[test]
    fn test_threshold_adjustment() {
        let mut vad = VAD::new();
        
        vad.set_energy_threshold(0.5);
        assert_eq!(vad.energy_threshold(), 0.5);
        
        vad.set_zcr_threshold(0.3);
        assert_eq!(vad.zcr_threshold(), 0.3);
        
        // Negative values should be clamped to 0
        vad.set_energy_threshold(-0.1);
        assert_eq!(vad.energy_threshold(), 0.0);
    }

    #[test]
    fn test_reset() {
        let mut vad = VAD::new();
        
        // Trigger some state
        let loud: Vec<f32> = vec![0.5; 100];
        vad.is_speech(&loud);
        
        // Reset
        vad.reset();
        assert_eq!(vad.current_silence, 0);
        assert_eq!(vad.current_speech, 0);
    }
}
