use std::fs::File;
use std::io::{Read, Write};
use std::collections::VecDeque;

/// Audio configuration constants
pub const SAMPLE_RATE: u32 = 48000;
pub const CHANNELS: u16 = 1;
pub const BIT_DEPTH: u16 = 16;
pub const CHUNK_SIZE: usize = 4800; // 100ms at 48kHz
pub const BUFFER_SIZE: usize = 48000; // 1 second buffer

/// Ring buffer for audio streaming
pub struct RingBuffer {
    buffer: VecDeque<f32>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn write(&mut self, data: &[f32]) {
        for &sample in data {
            if self.buffer.len() >= self.capacity {
                self.buffer.pop_front();
            }
            self.buffer.push_back(sample);
        }
    }

    pub fn read(&mut self, output: &mut [f32]) -> usize {
        let len = output.len().min(self.buffer.len());
        for i in 0..len {
            output[i] = self.buffer.pop_front().unwrap_or(0.0);
        }
        len
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Audio device manager
pub struct AudioDevice {
    #[cfg(target_os = "redox")]
    input: Option<File>,
    #[cfg(target_os = "redox")]
    output: Option<File>,
    
    #[cfg(not(target_os = "redox"))]
    mock_mode: bool,
}

impl AudioDevice {
    /// Create a new audio device
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(target_os = "redox")]
        {
            let input = File::open("audio:record").ok();
            let output = File::create("audio:play").ok();
            
            if input.is_none() || output.is_none() {
                eprintln!("⚠️  Warning: Redox audio scheme not available, using mock mode");
            }
            
            Ok(Self { input, output })
        }
        
        #[cfg(not(target_os = "redox"))]
        {
            println!("ℹ️  Running in mock mode (not on Redox OS)");
            Ok(Self { mock_mode: true })
        }
    }

    /// Capture audio chunk from microphone
    pub async fn capture_chunk(&mut self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "redox")]
        {
            if let Some(ref mut input) = self.input {
                let mut buffer = vec![0u8; CHUNK_SIZE * 2]; // 16-bit samples
                input.read_exact(&mut buffer)?;
                
                // Convert i16 to f32
                let samples: Vec<f32> = buffer
                    .chunks_exact(2)
                    .map(|chunk| {
                        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                        sample as f32 / i16::MAX as f32
                    })
                    .collect();
                
                Ok(samples)
            } else {
                // Fallback: generate silence
                Ok(vec![0.0; CHUNK_SIZE])
            }
        }
        
        #[cfg(not(target_os = "redox"))]
        {
            // Mock mode: generate sine wave for testing
            use std::f32::consts::PI;
            
            let frequency = 440.0; // A4 note
            let samples: Vec<f32> = (0..CHUNK_SIZE)
                .map(|i| {
                    let t = i as f32 / SAMPLE_RATE as f32;
                    (2.0 * PI * frequency * t).sin() * 0.1 // Low amplitude
                })
                .collect();
            
            // Simulate audio capture delay
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            Ok(samples)
        }
    }

    /// Play audio chunk to speaker
    pub async fn play(&mut self, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "redox")]
        {
            if let Some(ref mut output) = self.output {
                // Convert f32 to i16
                let buffer: Vec<u8> = samples
                    .iter()
                    .flat_map(|&sample| {
                        let sample_i16 = (sample * i16::MAX as f32) as i16;
                        sample_i16.to_le_bytes()
                    })
                    .collect();
                
                output.write_all(&buffer)?;
                output.flush()?;
            }
            Ok(())
        }
        
        #[cfg(not(target_os = "redox"))]
        {
            // Mock mode: just log
            println!("🔊 Playing {} samples", samples.len());
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }
    }

    /// Test loopback: capture and immediately play
    pub async fn test_loopback(&mut self, duration_secs: u32) -> Result<(), Box<dyn std::error::Error>> {
        println!("🎤 Testing audio loopback for {} seconds...", duration_secs);
        
        let chunks = (duration_secs * SAMPLE_RATE) / CHUNK_SIZE as u32;
        
        for i in 0..chunks {
            let audio = self.capture_chunk().await?;
            self.play(&audio).await?;
            
            if i % 10 == 0 {
                println!("  Processed {} chunks", i);
            }
        }
        
        println!("✅ Loopback test complete");
        Ok(())
    }
}

/// Audio processor for noise reduction and preprocessing
pub struct AudioProcessor {
    gain: f32,
}

impl AudioProcessor {
    pub fn new() -> Self {
        Self { gain: 1.0 }
    }

    /// Apply automatic gain control
    pub fn apply_agc(&mut self, samples: &mut [f32]) {
        // Calculate RMS (Root Mean Square)
        let rms: f32 = (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        
        if rms > 0.0 {
            let target_rms = 0.1; // Target level
            let new_gain = target_rms / rms;
            
            // Smooth gain changes
            self.gain = self.gain * 0.9 + new_gain * 0.1;
            
            // Apply gain
            for sample in samples.iter_mut() {
                *sample *= self.gain;
                // Clip to prevent distortion
                *sample = sample.clamp(-1.0, 1.0);
            }
        }
    }

    /// Simple noise gate
    pub fn apply_noise_gate(&self, samples: &mut [f32], threshold: f32) {
        for sample in samples.iter_mut() {
            if sample.abs() < threshold {
                *sample = 0.0;
            }
        }
    }

    /// Calculate audio energy
    pub fn calculate_energy(&self, samples: &[f32]) -> f32 {
        samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let mut buffer = RingBuffer::new(10);
        
        // Write data
        buffer.write(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(buffer.len(), 4);
        
        // Read data
        let mut output = vec![0.0; 4];
        let read = buffer.read(&mut output);
        assert_eq!(read, 4);
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
        
        // Buffer should be empty now
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = RingBuffer::new(5);
        
        // Write more than capacity
        buffer.write(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        
        // Should only keep last 5
        assert_eq!(buffer.len(), 5);
        
        let mut output = vec![0.0; 5];
        buffer.read(&mut output);
        assert_eq!(output, vec![3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[tokio::test]
    async fn test_audio_device_creation() {
        let device = AudioDevice::new();
        assert!(device.is_ok());
    }

    #[tokio::test]
    async fn test_audio_capture() {
        let mut device = AudioDevice::new().unwrap();
        let chunk = device.capture_chunk().await.unwrap();
        
        assert_eq!(chunk.len(), CHUNK_SIZE);
        
        // All samples should be in valid range
        for &sample in &chunk {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_audio_processor_agc() {
        let mut processor = AudioProcessor::new();
        let mut samples = vec![0.5; 100];
        
        processor.apply_agc(&mut samples);
        
        // Samples should be adjusted
        assert!(samples[0] != 0.5);
        
        // All samples should still be in valid range
        for &sample in &samples {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_noise_gate() {
        let processor = AudioProcessor::new();
        let mut samples = vec![0.01, 0.5, 0.02, 0.8, 0.001];
        
        processor.apply_noise_gate(&mut samples, 0.05);
        
        // Small samples should be zeroed
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[2], 0.0);
        assert_eq!(samples[4], 0.0);
        
        // Large samples should remain
        assert_eq!(samples[1], 0.5);
        assert_eq!(samples[3], 0.8);
    }

    #[test]
    fn test_energy_calculation() {
        let processor = AudioProcessor::new();
        
        // Silence
        let silence = vec![0.0; 100];
        assert_eq!(processor.calculate_energy(&silence), 0.0);
        
        // Full scale
        let loud = vec![1.0; 100];
        assert_eq!(processor.calculate_energy(&loud), 1.0);
    }
}
