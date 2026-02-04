use std::collections::HashMap;
use std::error::Error;

pub struct SemanticIndex {
    // Simple mock index for now
    // In production, this would use FAISS bindings or usearch
    vectors: HashMap<u64, Vec<f32>>,
}

impl SemanticIndex {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            vectors: HashMap::new(),
        })
    }
    
    pub fn add(&mut self, id: u64, vector: Vec<f32>, _text: &str) -> Result<(), Box<dyn Error>> {
        self.vectors.insert(id, vector);
        Ok(())
    }
    
    pub fn search(&self, query_vec: &[f32], limit: usize) -> Result<Vec<(u64, f32)>, Box<dyn Error>> {
        let mut scores: Vec<(u64, f32)> = self.vectors.iter()
            .map(|(id, vec)| {
                let score = cosine_similarity(query_vec, vec);
                (*id, score)
            })
            .collect();
            
        // Sort by score desc
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(scores.into_iter().take(limit).collect())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a * norm_b)
}
