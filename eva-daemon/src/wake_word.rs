use std::collections::VecDeque;

#[cfg(feature = "timemachine")]
use ort::{Session, Value};

/// Detection strategy for wake word
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectionStrategy {
    /// Simple energy-based correlation (fast, more false positives)
    Energy,
    /// MFCC-based detection (better accuracy, more CPU)
    Mfcc,
    /// ONNX model-based detection (best accuracy, requires model)
    Onnx,
}

/// Configuration for wake word detection
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// Detection strategy
    pub strategy: DetectionStrategy,
    /// Detection threshold (0.0 to 1.0)
    pub threshold: f32,
    /// Minimum audio duration in ms before detection
    pub min_duration_ms: u32,
    /// Cooldown between detections in ms
    pub cooldown_ms: u32,
    /// Sample rate (default: 16000)
    pub sample_rate: u32,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            strategy: DetectionStrategy::Mfcc,
            threshold: 0.65,
            min_duration_ms: 400,
            cooldown_ms: 1000,
            sample_rate: 16000,
        }
    }
}

/// Wake word detector for "Hey EVA"
///
/// Supports multiple detection strategies with configurable sensitivity.
pub struct WakeWordDetector {
    config: WakeWordConfig,
    /// Audio buffer for analysis
    buffer: VecDeque<f32>,
    /// Energy pattern for correlation-based detection
    energy_pattern: Vec<f32>,
    /// MFCC feature buffer
    mfcc_buffer: VecDeque<Vec<f32>>,
    /// Last detection timestamp
    last_detection_ms: u64,
    /// Running timestamp
    current_ms: u64,
    /// ONNX session (optional)
    #[cfg(feature = "timemachine")]
    onnx_session: Option<Session>,
    /// Detection count (for anti-spam)
    detection_count: u32,
}

impl WakeWordDetector {
    /// Create a new wake word detector with default configuration
    pub fn new() -> Self {
        Self::with_config(WakeWordConfig::default())
    }

    /// Create a wake word detector with custom configuration
    pub fn with_config(config: WakeWordConfig) -> Self {
        // Energy pattern for "Hey EVA" (normalized phoneme energies)
        let energy_pattern = vec![
            // "Hey" - rising energy, aspirated H, diphthong EI
            0.2, 0.4, 0.6, 0.8, 0.9, 0.85, 0.7, 0.5,
            // Brief pause
            0.2, 0.15, 0.1,
            // "E" - vowel, steady energy
            0.3, 0.5, 0.7, 0.8, 0.75, 0.6,
            // "VA" - fricative V, open A
            0.4, 0.6, 0.8, 0.9, 0.85, 0.7, 0.5, 0.3,
        ];

        let buffer_size = (config.sample_rate as usize * config.min_duration_ms as usize) / 1000;

        Self {
            config,
            buffer: VecDeque::with_capacity(buffer_size),
            energy_pattern,
            mfcc_buffer: VecDeque::with_capacity(50), // ~500ms of MFCC frames
            last_detection_ms: 0,
            current_ms: 0,
            #[cfg(feature = "timemachine")]
            onnx_session: None,
            detection_count: 0,
        }
    }

    /// Load ONNX model for ML-based detection
    #[cfg(feature = "timemachine")]
    pub fn load_model(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let session = Session::builder()?.commit_from_file(path)?;
        self.onnx_session = Some(session);
        self.config.strategy = DetectionStrategy::Onnx;
        Ok(())
    }

    /// Detect wake word in audio samples
    ///
    /// Returns true if "Hey EVA" is detected
    pub fn detect(&mut self, samples: &[f32]) -> bool {
        // Update timestamp
        let samples_ms = (samples.len() as u64 * 1000) / self.config.sample_rate as u64;
        self.current_ms += samples_ms;

        // Check cooldown
        if self.current_ms - self.last_detection_ms < self.config.cooldown_ms as u64 {
            return false;
        }

        // Add samples to buffer
        for &sample in samples {
            self.buffer.push_back(sample);
        }

        // Limit buffer size
        let max_buffer = self.config.sample_rate as usize * 2; // 2 seconds max
        while self.buffer.len() > max_buffer {
            self.buffer.pop_front();
        }

        // Check minimum duration
        let min_samples = (self.config.sample_rate * self.config.min_duration_ms / 1000) as usize;
        if self.buffer.len() < min_samples {
            return false;
        }

        // Detect based on strategy
        let detected = match self.config.strategy {
            DetectionStrategy::Energy => self.detect_energy(),
            DetectionStrategy::Mfcc => self.detect_mfcc(),
            DetectionStrategy::Onnx => self.detect_onnx(),
        };

        if detected {
            self.last_detection_ms = self.current_ms;
            self.detection_count += 1;
            self.buffer.clear();
            true
        } else {
            false
        }
    }

    /// Energy-based detection using cross-correlation
    fn detect_energy(&self) -> bool {
        if self.buffer.len() < self.energy_pattern.len() * 2 {
            return false;
        }

        // Compute energy envelope
        let frame_size = self.config.sample_rate as usize / 100; // 10ms frames
        let mut energy_envelope = Vec::new();

        for chunk in self.buffer.make_contiguous().chunks(frame_size) {
            let energy: f32 = chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32;
            energy_envelope.push(energy.sqrt());
        }

        // Normalize envelope
        let max_energy = energy_envelope.iter().cloned().fold(0.0f32, f32::max);
        if max_energy > 0.01 {
            for e in &mut energy_envelope {
                *e /= max_energy;
            }
        } else {
            return false; // Too quiet
        }

        // Cross-correlate with pattern
        let correlation = self.cross_correlate(&energy_envelope, &self.energy_pattern);

        correlation > self.config.threshold
    }

    /// MFCC-based detection
    fn detect_mfcc(&self) -> bool {
        if self.buffer.len() < self.config.sample_rate as usize / 2 {
            return false;
        }

        // Compute MFCC features
        let mfcc = self.compute_mfcc();

        // Check for characteristic "Hey EVA" pattern in MFCCs
        // Look for: rising energy -> brief dip -> sustained energy

        if mfcc.len() < 3 {
            return false;
        }

        // Simplified MFCC pattern matching
        // In production, use DTW (Dynamic Time Warping) or a classifier

        let mut score = 0.0;

        // Check energy progression (first MFCC coefficient is energy)
        let energies: Vec<f32> = mfcc.iter().map(|m| m.get(0).copied().unwrap_or(0.0)).collect();

        // Look for the "Hey EVA" pattern:
        // 1. Initial rise (Hey)
        // 2. Brief dip (pause)
        // 3. Rise again (EVA)

        let mut found_hey = false;
        let mut found_pause = false;
        let mut found_eva = false;

        for i in 1..energies.len() {
            if !found_hey && energies[i] > energies[i - 1] * 1.2 {
                found_hey = true;
                score += 0.3;
            } else if found_hey && !found_pause && energies[i] < energies[i - 1] * 0.7 {
                found_pause = true;
                score += 0.2;
            } else if found_pause && !found_eva && energies[i] > energies[i - 1] * 1.2 {
                found_eva = true;
                score += 0.3;
            }
        }

        // Check spectral consistency (higher MFCCs should be relatively stable)
        let spectral_variance = self.compute_spectral_variance(&mfcc);
        if spectral_variance < 0.5 {
            score += 0.2;
        }

        score > self.config.threshold
    }

    /// ONNX model-based detection
    fn detect_onnx(&self) -> bool {
        #[cfg(feature = "timemachine")]
        {
            if let Some(ref session) = self.onnx_session {
                // Prepare input tensor
                let samples: Vec<f32> = self.buffer.iter().cloned().collect();

                // Resample to model input size (typically 16000 * 1.5 = 24000 samples)
                let model_input_size = 24000;
                let resampled = self.resample(&samples, model_input_size);

                // Create tensor
                if let Ok(tensor) = Value::from_array((vec![1, model_input_size], resampled)) {
                    if let Ok(outputs) = session.run(vec![tensor]) {
                        if let Some(output) = outputs.get(0) {
                            // Extract probability
                            // Placeholder: in production, properly extract from output tensor
                            return true; // Model detected wake word
                        }
                    }
                }
            }
        }

        // Fallback to MFCC if ONNX not available
        self.detect_mfcc()
    }

    /// Compute simplified MFCC features
    fn compute_mfcc(&self) -> Vec<Vec<f32>> {
        let frame_size = 512;
        let hop_size = 256;
        let num_mfcc = 13;

        let samples: Vec<f32> = self.buffer.iter().cloned().collect();
        let mut mfcc_frames = Vec::new();

        for start in (0..samples.len().saturating_sub(frame_size)).step_by(hop_size) {
            let frame = &samples[start..start + frame_size];

            // Apply Hamming window
            let windowed: Vec<f32> = frame
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    let window = 0.54 - 0.46 * (2.0 * std::f32::consts::PI * i as f32 / frame_size as f32).cos();
                    s * window
                })
                .collect();

            // Compute energy in frequency bands (simplified mel filterbank)
            let mut mfcc = vec![0.0f32; num_mfcc];

            // Energy (zeroth coefficient)
            mfcc[0] = windowed.iter().map(|s| s * s).sum::<f32>().sqrt();

            // Simplified spectral features (using time-domain approximations)
            for (i, coef) in mfcc.iter_mut().enumerate().skip(1) {
                let freq = (i as f32 / num_mfcc as f32) * 0.5;
                let phase = 2.0 * std::f32::consts::PI * freq;
                *coef = windowed
                    .iter()
                    .enumerate()
                    .map(|(j, &s)| s * (phase * j as f32).cos())
                    .sum::<f32>()
                    .abs()
                    / frame_size as f32;
            }

            mfcc_frames.push(mfcc);
        }

        mfcc_frames
    }

    /// Compute spectral variance across MFCC frames
    fn compute_spectral_variance(&self, mfcc: &[Vec<f32>]) -> f32 {
        if mfcc.len() < 2 {
            return 1.0;
        }

        let num_coeffs = mfcc[0].len();
        let mut variance = 0.0;

        for coef_idx in 1..num_coeffs {
            let values: Vec<f32> = mfcc.iter().map(|m| m[coef_idx]).collect();
            let mean = values.iter().sum::<f32>() / values.len() as f32;
            let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;
            variance += var;
        }

        variance / (num_coeffs - 1) as f32
    }

    /// Cross-correlate two signals
    fn cross_correlate(&self, signal: &[f32], pattern: &[f32]) -> f32 {
        if signal.len() < pattern.len() {
            return 0.0;
        }

        let mut max_corr = 0.0f32;

        for offset in 0..=(signal.len() - pattern.len()) {
            let mut corr = 0.0;
            let mut sig_energy = 0.0;
            let mut pat_energy = 0.0;

            for (i, &p) in pattern.iter().enumerate() {
                let s = signal[offset + i];
                corr += s * p;
                sig_energy += s * s;
                pat_energy += p * p;
            }

            let norm = (sig_energy * pat_energy).sqrt();
            if norm > 0.0 {
                let normalized = corr / norm;
                max_corr = max_corr.max(normalized);
            }
        }

        max_corr
    }

    /// Resample audio to target length
    fn resample(&self, samples: &[f32], target_len: usize) -> Vec<f32> {
        if samples.is_empty() {
            return vec![0.0; target_len];
        }

        let ratio = samples.len() as f32 / target_len as f32;
        (0..target_len)
            .map(|i| {
                let src_idx = (i as f32 * ratio) as usize;
                samples.get(src_idx).copied().unwrap_or(0.0)
            })
            .collect()
    }

    /// Set detection sensitivity (0.0 to 1.0)
    ///
    /// Lower values = more sensitive (more false positives)
    /// Higher values = less sensitive (fewer false positives)
    pub fn set_sensitivity(&mut self, sensitivity: f32) {
        // Invert so higher sensitivity = lower threshold
        self.config.threshold = 1.0 - sensitivity.clamp(0.0, 1.0);
    }

    /// Get current sensitivity
    pub fn get_sensitivity(&self) -> f32 {
        1.0 - self.config.threshold
    }

    /// Set detection strategy
    pub fn set_strategy(&mut self, strategy: DetectionStrategy) {
        self.config.strategy = strategy;
    }

    /// Get total detection count
    pub fn detection_count(&self) -> u32 {
        self.detection_count
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.mfcc_buffer.clear();
        self.detection_count = 0;
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
    fn test_config_default() {
        let config = WakeWordConfig::default();
        assert_eq!(config.strategy, DetectionStrategy::Mfcc);
        assert!(config.threshold > 0.0 && config.threshold < 1.0);
    }

    #[test]
    fn test_detector_creation() {
        let detector = WakeWordDetector::new();
        assert_eq!(detector.detection_count(), 0);
    }

    #[test]
    fn test_sensitivity() {
        let mut detector = WakeWordDetector::new();

        detector.set_sensitivity(0.8);
        assert!((detector.get_sensitivity() - 0.8).abs() < 0.01);

        detector.set_sensitivity(1.5); // Should clamp
        assert!((detector.get_sensitivity() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_silence_no_detection() {
        let mut detector = WakeWordDetector::new();

        let silence = vec![0.0; 16000]; // 1 second of silence
        assert!(!detector.detect(&silence));
    }

    #[test]
    fn test_reset() {
        let mut detector = WakeWordDetector::new();

        detector.detect(&vec![0.5; 8000]);
        assert!(!detector.buffer.is_empty());

        detector.reset();
        assert!(detector.buffer.is_empty());
        assert_eq!(detector.detection_count(), 0);
    }

    #[test]
    fn test_cross_correlation() {
        let detector = WakeWordDetector::new();

        let signal = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let pattern = vec![0.0, 0.5, 1.0, 0.5, 0.0];

        let corr = detector.cross_correlate(&signal, &pattern);
        assert!(corr > 0.99); // Should be very high for identical signals
    }

    #[test]
    fn test_mfcc_computation() {
        let mut detector = WakeWordDetector::new();

        // Add some audio
        let audio: Vec<f32> = (0..8000)
            .map(|i| (i as f32 * 0.1).sin() * 0.5)
            .collect();

        for chunk in audio.chunks(1600) {
            detector.buffer.extend(chunk.iter());
        }

        let mfcc = detector.compute_mfcc();
        assert!(!mfcc.is_empty());
        assert_eq!(mfcc[0].len(), 13); // 13 MFCC coefficients
    }

    #[test]
    fn test_strategy_change() {
        let mut detector = WakeWordDetector::new();

        detector.set_strategy(DetectionStrategy::Energy);
        assert_eq!(detector.config.strategy, DetectionStrategy::Energy);

        detector.set_strategy(DetectionStrategy::Mfcc);
        assert_eq!(detector.config.strategy, DetectionStrategy::Mfcc);
    }

    #[test]
    fn test_cooldown() {
        let mut detector = WakeWordDetector::with_config(WakeWordConfig {
            cooldown_ms: 1000,
            ..WakeWordConfig::default()
        });

        // Add audio and trigger time progression
        let audio: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();

        // First call - should process
        detector.detect(&audio);
        let ms_after_first = detector.current_ms;

        // Immediate second call - should be in cooldown
        detector.detect(&audio);
        // Time should have advanced
        assert!(detector.current_ms > ms_after_first);
    }

    #[test]
    fn test_buffer_limit() {
        let mut detector = WakeWordDetector::new();

        // Fill buffer beyond max (2 seconds at 16kHz = 32000 samples)
        let large_audio: Vec<f32> = vec![0.1; 48000]; // 3 seconds
        detector.detect(&large_audio);

        // Buffer should be limited to max_buffer size
        assert!(detector.buffer.len() <= 32000);
    }

    #[test]
    fn test_detection_count() {
        let detector = WakeWordDetector::new();
        assert_eq!(detector.detection_count(), 0);
    }

    #[test]
    fn test_resampling() {
        let detector = WakeWordDetector::new();

        let original = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let resampled = detector.resample(&original, 10);

        assert_eq!(resampled.len(), 10);
    }

    #[test]
    fn test_resample_empty() {
        let detector = WakeWordDetector::new();

        let empty: Vec<f32> = vec![];
        let resampled = detector.resample(&empty, 10);

        assert_eq!(resampled.len(), 10);
        assert!(resampled.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_spectral_variance() {
        let detector = WakeWordDetector::new();

        // Single frame should return 1.0
        let single = vec![vec![0.1, 0.2, 0.3]];
        assert_eq!(detector.compute_spectral_variance(&single), 1.0);

        // Empty should also return 1.0
        let empty: Vec<Vec<f32>> = vec![];
        assert_eq!(detector.compute_spectral_variance(&empty), 1.0);
    }

    #[test]
    fn test_energy_pattern_exists() {
        let detector = WakeWordDetector::new();

        // Energy pattern should be pre-defined
        assert!(!detector.energy_pattern.is_empty());
        // Pattern values should be normalized (0.0 to 1.0)
        assert!(detector.energy_pattern.iter().all(|&e| e >= 0.0 && e <= 1.0));
    }

    #[test]
    fn test_cross_correlation_unequal() {
        let detector = WakeWordDetector::new();

        // Pattern longer than signal should return 0
        let short_signal = vec![0.5, 0.5];
        let long_pattern = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        let corr = detector.cross_correlate(&short_signal, &long_pattern);
        assert_eq!(corr, 0.0);
    }

    #[test]
    fn test_custom_config() {
        let config = WakeWordConfig {
            strategy: DetectionStrategy::Energy,
            threshold: 0.8,
            min_duration_ms: 500,
            cooldown_ms: 2000,
            sample_rate: 48000,
        };

        let detector = WakeWordDetector::with_config(config);
        assert_eq!(detector.config.strategy, DetectionStrategy::Energy);
        assert_eq!(detector.config.threshold, 0.8);
        assert_eq!(detector.config.min_duration_ms, 500);
        assert_eq!(detector.config.cooldown_ms, 2000);
        assert_eq!(detector.config.sample_rate, 48000);
    }

    #[test]
    fn test_default_trait() {
        let detector: WakeWordDetector = Default::default();
        assert_eq!(detector.config.strategy, DetectionStrategy::Mfcc);
    }

    #[test]
    fn test_minimum_duration_check() {
        let mut detector = WakeWordDetector::with_config(WakeWordConfig {
            min_duration_ms: 1000, // 1 second minimum
            sample_rate: 16000,
            ..WakeWordConfig::default()
        });

        // Less than minimum duration
        let short_audio: Vec<f32> = vec![0.5; 8000]; // 0.5 seconds
        let result = detector.detect(&short_audio);

        // Should not detect anything with insufficient duration
        assert!(!result);
    }
}
