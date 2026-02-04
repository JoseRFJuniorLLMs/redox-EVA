use ort::{Session, Value};
use std::error::Error;

pub struct EmbeddingEngine {
    session: Session,
}

impl EmbeddingEngine {
    pub fn new(npu: &super::npu_delegate::NPUDelegate) -> Result<Self, Box<dyn Error>> {
        // Load Embedding model (MiniLM ONNX)
        let session = npu.create_session("models/embeddings.onnx")?;
        Ok(Self { session })
    }
    
    pub fn encode(&self, text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        let input_tensor = self.tokenize(text)?;
        
        let outputs = self.session.run(vec![input_tensor])?;
        
        let embedding = self.extract_embedding(&outputs)?;
        Ok(embedding)
    }
    
    fn tokenize(&self, text: &str) -> Result<Value, Box<dyn Error>> {
        // Simple mock tokenizer. In reality use HuggingFace Tokenizers
        let tokens: Vec<i64> = text.chars()
            .take(512)
            .map(|c| c as i64 % 30000) // Mock token ID
            .collect();
            
        let shape = vec![1, tokens.len()];
        let tensor = Value::from_array((shape, tokens))?;
        Ok(tensor)
    }
    
    fn extract_embedding(&self, _outputs: &[Value]) -> Result<Vec<f32>, Box<dyn Error>> {
        // Mock 384-d vector (MiniLM size)
        Ok(vec![0.0; 384])
    }
}
