use ort::{Environment, ExecutionProvider, Session, SessionBuilder};
use std::sync::Arc;

pub struct NPUDelegate {
    env: Arc<Environment>,
}

impl NPUDelegate {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize ONNX Runtime environment
        // We configure it to prefer NPU (DirectML on Windows, CoreML on Mac) then GPU, then CPU
        let builder = Environment::builder()
            .with_name("EVA-TimeMachine")
            .with_execution_providers([
                ExecutionProvider::DirectML(Default::default()),  // Windows NPU/GPU
                ExecutionProvider::CUDA(Default::default()),      // NVIDIA GPU
                ExecutionProvider::CPU(Default::default()),       // Fallback
            ]);

        let env = builder.build()?.into_arc();
        
        println!("[NPU] Initialized ONNX Runtime environment");
        
        Ok(Self { env })
    }
    
    pub fn create_session(&self, model_path: &str) -> Result<Session, Box<dyn std::error::Error>> {
        let session = SessionBuilder::new(&self.env)?
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_model_from_file(model_path)?;
        
        Ok(session)
    }
}
