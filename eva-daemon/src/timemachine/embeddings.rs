use sha2::{Sha256, Digest};
use std::error::Error;
use unicode_segmentation::UnicodeSegmentation;

#[cfg(feature = "timemachine")]
use ort::{Session, Value};

/// Embedding dimension (matches MiniLM-L6-v2)
const EMBEDDING_DIM: usize = 384;

/// Embedding Engine for semantic text representation
///
/// Uses ONNX model (MiniLM) when available, falls back to
/// hash-based semantic embedding for offline operation
pub struct EmbeddingEngine {
    #[cfg(feature = "timemachine")]
    session: Option<Session>,
    /// Vocabulary for simple tokenization
    stop_words: Vec<&'static str>,
}

impl EmbeddingEngine {
    #[cfg(feature = "timemachine")]
    pub async fn new(npu: &super::npu_delegate::NPUDelegate) -> Result<Self, Box<dyn Error>> {
        let session = match npu.create_session("models/embeddings.onnx") {
            Ok(s) => {
                println!("[Embeddings] ONNX model loaded successfully");
                Some(s)
            }
            Err(e) => {
                println!("[Embeddings] ONNX model not available ({}), using hash-based embedding", e);
                None
            }
        };

        Ok(Self {
            session,
            stop_words: Self::default_stop_words(),
        })
    }

    #[cfg(not(feature = "timemachine"))]
    pub async fn new(_npu: &super::npu_delegate::NPUDelegate) -> Result<Self, Box<dyn Error>> {
        println!("[Embeddings] Running without ONNX (timemachine feature disabled)");
        Ok(Self {
            stop_words: Self::default_stop_words(),
        })
    }

    fn default_stop_words() -> Vec<&'static str> {
        vec![
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
            "have", "has", "had", "do", "does", "did", "will", "would", "could",
            "should", "may", "might", "must", "shall", "can", "need", "dare",
            "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
            "into", "through", "during", "before", "after", "above", "below",
            "and", "but", "or", "nor", "so", "yet", "both", "either", "neither",
            "not", "only", "own", "same", "than", "too", "very", "just",
            "o", "e", "de", "da", "do", "em", "para", "com", "por", "um", "uma",
            "os", "as", "no", "na", "que", "se", "ou", "mas", "como", "mais",
        ]
    }

    /// Encode text into embedding vector
    pub fn encode(&self, text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        #[cfg(feature = "timemachine")]
        if let Some(ref session) = self.session {
            return self.encode_with_onnx(session, text);
        }

        // Fallback: Hash-based semantic embedding
        Ok(self.encode_with_hash(text))
    }

    #[cfg(feature = "timemachine")]
    fn encode_with_onnx(&self, session: &Session, text: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        let input_tensor = self.tokenize_for_onnx(text)?;
        let outputs = session.run(vec![input_tensor])?;
        self.extract_embedding_from_onnx(&outputs)
    }

    #[cfg(feature = "timemachine")]
    fn tokenize_for_onnx(&self, text: &str) -> Result<Value, Box<dyn Error>> {
        // Simple tokenization for ONNX model
        // In production, use HuggingFace tokenizers
        let tokens: Vec<i64> = text
            .unicode_words()
            .take(512)
            .enumerate()
            .map(|(i, word)| {
                // Create deterministic token ID from word
                let mut hasher = Sha256::new();
                hasher.update(word.to_lowercase().as_bytes());
                let hash = hasher.finalize();
                // Map to vocab range (0-30000)
                let id = u64::from_le_bytes(hash[0..8].try_into().unwrap()) % 30000;
                id as i64
            })
            .collect();

        let len = tokens.len().max(1);
        let shape = vec![1, len];
        let tensor = Value::from_array((shape, tokens))?;
        Ok(tensor)
    }

    #[cfg(feature = "timemachine")]
    fn extract_embedding_from_onnx(&self, outputs: &[Value]) -> Result<Vec<f32>, Box<dyn Error>> {
        if outputs.is_empty() {
            return Ok(vec![0.0; EMBEDDING_DIM]);
        }

        // Try to extract tensor data
        // Placeholder: In production, properly extract from ONNX output
        Ok(vec![0.0; EMBEDDING_DIM])
    }

    /// Hash-based semantic embedding (offline fallback)
    ///
    /// Creates a deterministic embedding vector based on:
    /// 1. Word-level hashing
    /// 2. N-gram features
    /// 3. Positional encoding
    fn encode_with_hash(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; EMBEDDING_DIM];

        // Normalize and tokenize
        let words: Vec<&str> = text
            .unicode_words()
            .filter(|w| w.len() > 1)
            .filter(|w| !self.stop_words.contains(&w.to_lowercase().as_str()))
            .collect();

        if words.is_empty() {
            // Return normalized zero vector for empty input
            return embedding;
        }

        // 1. Word-level features (first 128 dimensions)
        for (i, word) in words.iter().enumerate() {
            let word_hash = self.hash_word(word);
            let weight = 1.0 / (1.0 + i as f32 * 0.1); // Position decay

            for j in 0..32 {
                let dim = (word_hash[j % 8] as usize + j * 4) % 128;
                let value = ((word_hash[(j + 1) % 8] as f32) / 255.0 - 0.5) * weight;
                embedding[dim] += value;
            }
        }

        // 2. Bigram features (dimensions 128-256)
        for window in words.windows(2) {
            let bigram = format!("{} {}", window[0], window[1]);
            let bigram_hash = self.hash_word(&bigram);

            for j in 0..16 {
                let dim = 128 + (bigram_hash[j % 8] as usize + j * 8) % 128;
                let value = (bigram_hash[(j + 1) % 8] as f32) / 255.0 - 0.5;
                embedding[dim] += value * 0.5;
            }
        }

        // 3. Character-level features (dimensions 256-384)
        let chars: Vec<char> = text.chars().collect();
        for (i, ch) in chars.iter().enumerate() {
            let char_code = *ch as u32;
            let dim = 256 + (char_code as usize + i) % 128;
            embedding[dim] += 0.1 / (1.0 + i as f32 * 0.01);
        }

        // 4. Global features
        embedding[EMBEDDING_DIM - 4] = words.len() as f32 / 100.0; // Word count
        embedding[EMBEDDING_DIM - 3] = chars.len() as f32 / 1000.0; // Char count
        embedding[EMBEDDING_DIM - 2] = if text.contains('?') { 1.0 } else { 0.0 }; // Question
        embedding[EMBEDDING_DIM - 1] = if text.contains('!') { 1.0 } else { 0.0 }; // Exclamation

        // Normalize to unit vector
        self.normalize(&mut embedding);

        embedding
    }

    /// Hash a word using SHA-256 (first 8 bytes)
    fn hash_word(&self, word: &str) -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(word.to_lowercase().as_bytes());
        let hash = hasher.finalize();
        let mut result = [0u8; 8];
        result.copy_from_slice(&hash[0..8]);
        result
    }

    /// Normalize vector to unit length
    fn normalize(&self, vec: &mut [f32]) {
        let magnitude: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 1e-10 {
            for x in vec.iter_mut() {
                *x /= magnitude;
            }
        }
    }

    /// Calculate cosine similarity between two embeddings
    pub fn similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a > 1e-10 && mag_b > 1e-10 {
            dot / (mag_a * mag_b)
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_basic() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let embedding = engine.encode_with_hash("Hello world");
        assert_eq!(embedding.len(), EMBEDDING_DIM);

        // Should be normalized
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01 || magnitude < 0.01); // Unit vector or zero
    }

    #[test]
    fn test_similar_texts() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let e1 = engine.encode_with_hash("programming code software");
        let e2 = engine.encode_with_hash("coding software programming");
        let e3 = engine.encode_with_hash("cooking food recipe");

        let sim_12 = EmbeddingEngine::similarity(&e1, &e2);
        let sim_13 = EmbeddingEngine::similarity(&e1, &e3);

        // Similar texts should have higher similarity
        assert!(sim_12 > sim_13, "Expected sim({}, {}) > sim({}, {})", sim_12, sim_13, sim_12, sim_13);
    }

    #[test]
    fn test_empty_text() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let embedding = engine.encode_with_hash("");
        assert_eq!(embedding.len(), EMBEDDING_DIM);
    }

    #[test]
    fn test_deterministic() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let e1 = engine.encode_with_hash("test text");
        let e2 = engine.encode_with_hash("test text");

        // Same input should produce same output
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_stop_words_filtered() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        // "the" and "a" are stop words, should be filtered
        let e1 = engine.encode_with_hash("the quick fox");
        let e2 = engine.encode_with_hash("quick fox");

        // Should be very similar since "the" is filtered
        let similarity = EmbeddingEngine::similarity(&e1, &e2);
        assert!(similarity > 0.8, "Stop words should be filtered, similarity: {}", similarity);
    }

    #[test]
    fn test_question_mark_feature() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let question = engine.encode_with_hash("What is programming?");
        let statement = engine.encode_with_hash("What is programming");

        // Question should have 1.0 in the question dimension
        assert_eq!(question[EMBEDDING_DIM - 2], 1.0);
        assert_eq!(statement[EMBEDDING_DIM - 2], 0.0);
    }

    #[test]
    fn test_exclamation_feature() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let excited = engine.encode_with_hash("Hello world!");
        let calm = engine.encode_with_hash("Hello world");

        // Exclamation should have 1.0 in the exclamation dimension
        assert_eq!(excited[EMBEDDING_DIM - 1], 1.0);
        assert_eq!(calm[EMBEDDING_DIM - 1], 0.0);
    }

    #[test]
    fn test_similarity_self() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        let embedding = engine.encode_with_hash("Hello world");
        let similarity = EmbeddingEngine::similarity(&embedding, &embedding);

        // Self-similarity should be 1.0 (or very close)
        assert!((similarity - 1.0).abs() < 0.001, "Self-similarity should be ~1.0, got {}", similarity);
    }

    #[test]
    fn test_similarity_different_lengths() {
        // Different length vectors should return 0.0
        let a = vec![0.1, 0.2, 0.3];
        let b = vec![0.1, 0.2];

        let similarity = EmbeddingEngine::similarity(&a, &b);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn test_similarity_zero_vectors() {
        let zero = vec![0.0; EMBEDDING_DIM];
        let normal = vec![0.1; EMBEDDING_DIM];

        let similarity = EmbeddingEngine::similarity(&zero, &normal);
        assert_eq!(similarity, 0.0, "Zero vector similarity should be 0.0");
    }

    #[test]
    fn test_portuguese_stop_words() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        // Portuguese stop words should be in the list
        assert!(engine.stop_words.contains(&"de"));
        assert!(engine.stop_words.contains(&"para"));
        assert!(engine.stop_words.contains(&"que"));
    }

    #[test]
    fn test_unicode_support() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        // Should handle UTF-8 properly
        let e1 = engine.encode_with_hash("café résumé naïve");
        let e2 = engine.encode_with_hash("日本語 テスト");
        let e3 = engine.encode_with_hash("Привет мир");

        assert_eq!(e1.len(), EMBEDDING_DIM);
        assert_eq!(e2.len(), EMBEDDING_DIM);
        assert_eq!(e3.len(), EMBEDDING_DIM);
    }

    #[test]
    fn test_long_text() {
        let engine = EmbeddingEngine {
            #[cfg(feature = "timemachine")]
            session: None,
            stop_words: EmbeddingEngine::default_stop_words(),
        };

        // Very long text should still produce valid embedding
        let long_text = "word ".repeat(1000);
        let embedding = engine.encode_with_hash(&long_text);

        assert_eq!(embedding.len(), EMBEDDING_DIM);

        // Should be normalized
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01, "Should be normalized, got magnitude {}", magnitude);
    }
}
