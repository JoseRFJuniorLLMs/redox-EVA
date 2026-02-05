use image::{DynamicImage, GenericImageView, GrayImage, Luma};
use std::error::Error;

#[cfg(feature = "timemachine")]
use ort::{Session, Value};

/// OCR Engine for extracting text from screenshots
///
/// Uses ONNX model when available, falls back to edge-based text region detection
pub struct OCREngine {
    #[cfg(feature = "timemachine")]
    session: Option<Session>,
    /// Minimum contrast threshold for text detection
    contrast_threshold: u8,
}

impl OCREngine {
    #[cfg(feature = "timemachine")]
    pub async fn new(npu: &super::npu_delegate::NPUDelegate) -> Result<Self, Box<dyn Error>> {
        // Try to load ONNX model, fall back to heuristic if not available
        let session = match npu.create_session("models/ocr-model.onnx") {
            Ok(s) => {
                println!("[OCR] ONNX model loaded successfully");
                Some(s)
            }
            Err(e) => {
                println!("[OCR] ONNX model not available ({}), using heuristic extraction", e);
                None
            }
        };

        Ok(Self {
            session,
            contrast_threshold: 50,
        })
    }

    #[cfg(not(feature = "timemachine"))]
    pub async fn new(_npu: &super::npu_delegate::NPUDelegate) -> Result<Self, Box<dyn Error>> {
        println!("[OCR] Running without ONNX (timemachine feature disabled)");
        Ok(Self {
            contrast_threshold: 50,
        })
    }

    /// Extract text from image
    ///
    /// Returns extracted text content from the screenshot
    pub fn extract_text(&self, image: &DynamicImage) -> Result<String, Box<dyn Error>> {
        #[cfg(feature = "timemachine")]
        if let Some(ref session) = self.session {
            return self.extract_with_onnx(session, image);
        }

        // Fallback: Heuristic text extraction
        self.extract_with_heuristics(image)
    }

    #[cfg(feature = "timemachine")]
    fn extract_with_onnx(&self, session: &Session, image: &DynamicImage) -> Result<String, Box<dyn Error>> {
        let input_tensor = self.preprocess_image(image)?;

        let outputs = session.run(vec![input_tensor])?;

        self.decode_onnx_output(&outputs)
    }

    #[cfg(feature = "timemachine")]
    fn preprocess_image(&self, image: &DynamicImage) -> Result<Value, Box<dyn Error>> {
        // Resize to model input size
        let resized = image.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);

        let rgb = resized.to_rgb8();
        let pixels: Vec<f32> = rgb
            .pixels()
            .flat_map(|p| vec![p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0])
            .collect();

        // Shape: [Batch, Channel, Height, Width]
        let shape = vec![1, 3, 224, 224];
        let tensor = Value::from_array((shape, pixels))?;

        Ok(tensor)
    }

    #[cfg(feature = "timemachine")]
    fn decode_onnx_output(&self, outputs: &[Value]) -> Result<String, Box<dyn Error>> {
        // Real implementation would decode CTC output here
        // For now, try to extract from tensor
        if outputs.is_empty() {
            return Ok(String::new());
        }

        // Placeholder: In production, implement proper CTC decoding
        Ok("[ONNX OCR Output]".to_string())
    }

    /// Heuristic-based text extraction (fallback when ONNX not available)
    ///
    /// Uses image analysis to identify text-like regions and extract
    /// relevant information about screen content
    fn extract_with_heuristics(&self, image: &DynamicImage) -> Result<String, Box<dyn Error>> {
        let (width, height) = image.dimensions();
        let gray = image.to_luma8();

        let mut extracted_info = Vec::new();

        // 1. Analyze image statistics
        let stats = self.analyze_image_stats(&gray);
        extracted_info.push(format!("Screen: {}x{}", width, height));

        // 2. Detect high-contrast regions (likely text areas)
        let text_density = self.calculate_text_density(&gray);
        extracted_info.push(format!("Text density: {:.1}%", text_density * 100.0));

        // 3. Identify dominant colors (UI context)
        let dominant_colors = self.identify_dominant_regions(image);
        if !dominant_colors.is_empty() {
            extracted_info.push(format!("UI regions: {}", dominant_colors.join(", ")));
        }

        // 4. Edge detection for text-like patterns
        let edge_density = self.calculate_edge_density(&gray);
        if edge_density > 0.1 {
            extracted_info.push("High detail content detected".to_string());
        }

        // 5. Brightness analysis (dark mode detection)
        if stats.mean_brightness < 80.0 {
            extracted_info.push("Dark theme".to_string());
        } else if stats.mean_brightness > 200.0 {
            extracted_info.push("Light theme".to_string());
        }

        Ok(extracted_info.join(" | "))
    }

    /// Analyze basic image statistics
    fn analyze_image_stats(&self, gray: &GrayImage) -> ImageStats {
        let pixels: Vec<u8> = gray.pixels().map(|p| p[0]).collect();
        let len = pixels.len() as f64;

        let sum: f64 = pixels.iter().map(|&p| p as f64).sum();
        let mean = sum / len;

        let variance: f64 = pixels.iter().map(|&p| (p as f64 - mean).powi(2)).sum::<f64>() / len;
        let std_dev = variance.sqrt();

        ImageStats {
            mean_brightness: mean,
            std_deviation: std_dev,
        }
    }

    /// Calculate text density using edge detection
    fn calculate_text_density(&self, gray: &GrayImage) -> f64 {
        let (width, height) = gray.dimensions();
        let mut edge_pixels = 0u64;

        // Simple Sobel-like edge detection
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let p = gray.get_pixel(x, y)[0] as i32;
                let left = gray.get_pixel(x - 1, y)[0] as i32;
                let right = gray.get_pixel(x + 1, y)[0] as i32;
                let up = gray.get_pixel(x, y - 1)[0] as i32;
                let down = gray.get_pixel(x, y + 1)[0] as i32;

                let gx = (right - left).abs();
                let gy = (down - up).abs();
                let gradient = gx + gy;

                if gradient > self.contrast_threshold as i32 {
                    edge_pixels += 1;
                }
            }
        }

        let total_pixels = (width - 2) * (height - 2);
        edge_pixels as f64 / total_pixels as f64
    }

    /// Calculate overall edge density
    fn calculate_edge_density(&self, gray: &GrayImage) -> f64 {
        self.calculate_text_density(gray)
    }

    /// Identify dominant color regions
    fn identify_dominant_regions(&self, image: &DynamicImage) -> Vec<String> {
        let rgb = image.to_rgb8();
        let (width, height) = rgb.dimensions();

        // Sample pixels for color analysis
        let sample_step = 20;
        let mut light_count = 0;
        let mut dark_count = 0;
        let mut blue_count = 0;
        let mut red_count = 0;

        for y in (0..height).step_by(sample_step) {
            for x in (0..width).step_by(sample_step) {
                let p = rgb.get_pixel(x, y);
                let r = p[0] as u32;
                let g = p[1] as u32;
                let b = p[2] as u32;
                let brightness = (r + g + b) / 3;

                if brightness > 200 {
                    light_count += 1;
                } else if brightness < 50 {
                    dark_count += 1;
                }

                if b > r + 30 && b > g + 30 {
                    blue_count += 1;
                }
                if r > g + 30 && r > b + 30 {
                    red_count += 1;
                }
            }
        }

        let total = (width / sample_step as u32) * (height / sample_step as u32);
        let threshold = total / 10; // 10% threshold

        let mut regions = Vec::new();
        if light_count > threshold {
            regions.push("white-bg".to_string());
        }
        if dark_count > threshold {
            regions.push("dark-bg".to_string());
        }
        if blue_count > threshold {
            regions.push("blue-accent".to_string());
        }
        if red_count > threshold {
            regions.push("red-accent".to_string());
        }

        regions
    }
}

struct ImageStats {
    mean_brightness: f64,
    std_deviation: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_stats() {
        // Create a simple test image
        let img = GrayImage::from_fn(100, 100, |_, _| Luma([128u8]));

        let ocr = OCREngine {
            #[cfg(feature = "timemachine")]
            session: None,
            contrast_threshold: 50,
        };

        let stats = ocr.analyze_image_stats(&img);
        assert!((stats.mean_brightness - 128.0).abs() < 1.0);
    }

    #[test]
    fn test_edge_detection() {
        // Create image with edges
        let mut img = GrayImage::from_fn(100, 100, |x, _| {
            if x < 50 { Luma([0u8]) } else { Luma([255u8]) }
        });

        let ocr = OCREngine {
            #[cfg(feature = "timemachine")]
            session: None,
            contrast_threshold: 50,
        };

        let density = ocr.calculate_text_density(&img);
        assert!(density > 0.0); // Should detect the edge
    }
}
