use screenshots::Screen;
use image::DynamicImage;
use std::error::Error;

pub struct ScreenCapture {
    screens: Vec<Screen>,
}

impl ScreenCapture {
    pub fn new() -> Self {
        let screens = Screen::all().unwrap_or_else(|_| vec![]);
        Self { screens }
    }

    pub fn take_screenshot(&self) -> Result<DynamicImage, Box<dyn Error>> {
        if self.screens.is_empty() {
            return Err("No screens found".into());
        }

        // Capture primary screen
        let screen = self.screens[0]; 
        let image = screen.capture()?;
        
        let buffer = image.buffer();
        let dynamic_image = image::ImageBuffer::from_raw(image.width(), image.height(), buffer.clone())
            .map(DynamicImage::ImageRgba8)
            .ok_or("Failed to convert screenshot to image")?;
            
        // SMART FILTERING
        if self.should_block(&dynamic_image) {
            return Err("Screenshot blocked by privacy filter".into());
        }

        Ok(dynamic_image)
    }

    fn should_block(&self, _image: &DynamicImage) -> bool {
        // Placeholder for privacy logic
        // In real implementation:
        // 1. Check active window title (e.g. "Incognito", "PayPal")
        // 2. Run lightweight OCR to detect "Password", "Credit Card"
        // 3. User blocklist
        
        // For now, always allow
        false
    }
}
