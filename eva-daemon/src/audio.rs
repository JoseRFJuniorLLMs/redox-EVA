use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

#[cfg(not(target_os = "redox"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub const SAMPLE_RATE: u32 = 16000; // Gemini expects 16kHz
pub const CHANNELS: u16 = 1;
pub const CHUNK_SIZE: usize = 1600; // 100ms at 16kHz

/// Audio device manager
pub struct AudioDevice {
    #[cfg(not(target_os = "redox"))]
    input_buffer: Arc<Mutex<VecDeque<f32>>>,
    #[cfg(not(target_os = "redox"))]
    output_buffer: Arc<Mutex<VecDeque<f32>>>,

    #[cfg(not(target_os = "redox"))]
    _input_stream: Option<cpal::Stream>,
    #[cfg(not(target_os = "redox"))]
    _output_stream: Option<cpal::Stream>,

    #[cfg(target_os = "redox")]
    input: Option<std::fs::File>,
    #[cfg(target_os = "redox")]
    output: Option<std::fs::File>,
}

impl AudioDevice {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[cfg(not(target_os = "redox"))]
        {
            let host = cpal::default_host();

            // INPUT (Microphone)
            let input_device = host.default_input_device()
                .ok_or("No input device available")?;
            println!("🎤 Microfone: {}", input_device.name().unwrap_or_default());

            let input_config = input_device.default_input_config()?;
            let in_sample_rate = input_config.sample_rate().0;
            let in_channels = input_config.channels() as usize;
            println!("   Input: {}Hz, {} canais", in_sample_rate, in_channels);

            let input_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(in_sample_rate as usize)));
            let input_buffer_clone = Arc::clone(&input_buffer);

            let input_stream_config: cpal::StreamConfig = input_config.into();
            let input_stream = input_device.build_input_stream(
                &input_stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut buffer) = input_buffer_clone.lock() {
                        for chunk in data.chunks(in_channels) {
                            let mono: f32 = chunk.iter().sum::<f32>() / in_channels as f32;
                            buffer.push_back(mono);
                            if buffer.len() > in_sample_rate as usize * 2 {
                                buffer.pop_front();
                            }
                        }
                    }
                },
                |err| eprintln!("❌ Input error: {}", err),
                None,
            )?;
            input_stream.play()?;

            // OUTPUT (Speaker)
            let output_device = host.default_output_device()
                .ok_or("No output device available")?;
            println!("🔊 Speaker: {}", output_device.name().unwrap_or_default());

            let output_config = output_device.default_output_config()?;
            let out_sample_rate = output_config.sample_rate().0;
            let out_channels = output_config.channels() as usize;
            println!("   Output: {}Hz, {} canais", out_sample_rate, out_channels);

            let output_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(out_sample_rate as usize * 2)));
            let output_buffer_clone = Arc::clone(&output_buffer);

            let output_stream_config: cpal::StreamConfig = output_config.into();
            let output_stream = output_device.build_output_stream(
                &output_stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut buffer) = output_buffer_clone.lock() {
                        for frame in data.chunks_mut(out_channels) {
                            let sample = buffer.pop_front().unwrap_or(0.0);
                            for s in frame.iter_mut() {
                                *s = sample;
                            }
                        }
                    }
                },
                |err| eprintln!("❌ Output error: {}", err),
                None,
            )?;
            output_stream.play()?;

            println!("✅ Áudio iniciado");

            Ok(Self {
                input_buffer,
                output_buffer,
                _input_stream: Some(input_stream),
                _output_stream: Some(output_stream),
            })
        }

        #[cfg(target_os = "redox")]
        {
            use std::fs::File;
            let input = File::open("audio:record").ok();
            let output = File::create("audio:play").ok();
            Ok(Self { input, output })
        }
    }

    pub async fn capture_chunk(&mut self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        #[cfg(not(target_os = "redox"))]
        {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let samples = {
                let mut buffer = self.input_buffer.lock().map_err(|e| format!("Lock: {}", e))?;
                let len = buffer.len().min(CHUNK_SIZE);
                let mut samples = Vec::with_capacity(len);
                for _ in 0..len {
                    if let Some(s) = buffer.pop_front() {
                        samples.push(s);
                    }
                }
                samples
            };

            let mut result = samples;
            result.resize(CHUNK_SIZE, 0.0);
            Ok(result)
        }

        #[cfg(target_os = "redox")]
        {
            use std::io::Read;
            if let Some(ref mut input) = self.input {
                let mut buffer = vec![0u8; CHUNK_SIZE * 2];
                input.read_exact(&mut buffer)?;
                let samples: Vec<f32> = buffer
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                    .collect();
                Ok(samples)
            } else {
                Ok(vec![0.0; CHUNK_SIZE])
            }
        }
    }

    pub async fn play(&mut self, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(not(target_os = "redox"))]
        {
            if let Ok(mut buffer) = self.output_buffer.lock() {
                for &s in samples {
                    buffer.push_back(s);
                }
            }
            Ok(())
        }

        #[cfg(target_os = "redox")]
        {
            use std::io::Write;
            if let Some(ref mut output) = self.output {
                let buffer: Vec<u8> = samples
                    .iter()
                    .flat_map(|&s| ((s * 32767.0) as i16).to_le_bytes())
                    .collect();
                output.write_all(&buffer)?;
                output.flush()?;
            }
            Ok(())
        }
    }
}

pub struct RingBuffer {
    buffer: VecDeque<f32>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { buffer: VecDeque::with_capacity(capacity), capacity }
    }
    pub fn write(&mut self, data: &[f32]) {
        for &s in data {
            if self.buffer.len() >= self.capacity { self.buffer.pop_front(); }
            self.buffer.push_back(s);
        }
    }
    pub fn read(&mut self, output: &mut [f32]) -> usize {
        let len = output.len().min(self.buffer.len());
        for i in 0..len { output[i] = self.buffer.pop_front().unwrap_or(0.0); }
        len
    }
    pub fn len(&self) -> usize { self.buffer.len() }
    pub fn is_empty(&self) -> bool { self.buffer.is_empty() }
    pub fn clear(&mut self) { self.buffer.clear(); }
}

pub const BUFFER_SIZE: usize = 16000;
pub const BIT_DEPTH: u16 = 16;
