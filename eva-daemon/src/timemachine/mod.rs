pub mod capture;
pub mod embeddings;
pub mod index;
pub mod npu_delegate;
pub mod ocr;
pub mod search;
pub mod storage;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Default configuration values
const DEFAULT_CAPTURE_INTERVAL_SECS: u64 = 10;
const DEFAULT_CLEANUP_INTERVAL_CAPTURES: u64 = 100; // Run cleanup every 100 captures
const DEFAULT_MAX_STORAGE_MB: u64 = 5000;
const DEFAULT_RETENTION_DAYS: i64 = 30;

/// TimeMachine configuration
#[derive(Clone)]
pub struct TimeMachineConfig {
    /// Interval between captures in seconds
    pub capture_interval_secs: u64,
    /// Maximum storage in megabytes
    pub max_storage_mb: u64,
    /// Retention period in days
    pub retention_days: i64,
    /// Run cleanup every N captures
    pub cleanup_interval: u64,
}

impl Default for TimeMachineConfig {
    fn default() -> Self {
        Self {
            capture_interval_secs: DEFAULT_CAPTURE_INTERVAL_SECS,
            max_storage_mb: DEFAULT_MAX_STORAGE_MB,
            retention_days: DEFAULT_RETENTION_DAYS,
            cleanup_interval: DEFAULT_CLEANUP_INTERVAL_CAPTURES,
        }
    }
}

/// TimeMachine statistics
#[derive(Debug, Clone, Default)]
pub struct TimeMachineStats {
    pub total_captures: u64,
    pub successful_captures: u64,
    pub blocked_by_privacy: u64,
    pub errors: u64,
    pub storage_used_mb: f64,
}

/// Time Machine AI - Captures, indexes, and searches your digital life
pub struct TimeMachine {
    capture: capture::ScreenCapture,
    ocr: ocr::OCREngine,
    embeddings: embeddings::EmbeddingEngine,
    index: Arc<RwLock<index::SemanticIndex>>,
    storage: storage::Storage,
    #[allow(dead_code)]
    npu: npu_delegate::NPUDelegate,
    /// Configuration
    config: TimeMachineConfig,
    /// Recording state
    is_recording: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    /// Statistics
    capture_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    privacy_blocked_count: Arc<AtomicU64>,
    error_count: Arc<AtomicU64>,
}

impl TimeMachine {
    /// Create a new TimeMachine instance with default configuration
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(TimeMachineConfig::default()).await
    }

    /// Create a new TimeMachine instance with custom configuration
    pub async fn with_config(config: TimeMachineConfig) -> Result<Self, Box<dyn std::error::Error>> {
        println!("[TimeMachine] Initializing...");

        // 1. Initialize NPU
        let npu = npu_delegate::NPUDelegate::new()?;

        // 2. Load Models
        let ocr = ocr::OCREngine::new(&npu).await?;
        let embeddings = embeddings::EmbeddingEngine::new(&npu).await?;

        // 3. Setup Storage (Encrypted)
        let mut storage = storage::Storage::new("~/.eva/timemachine").await?;
        storage.set_limits(config.max_storage_mb, config.retention_days);

        // Get encryption key securely
        let encryption_key = Self::get_encryption_key()?;
        storage.set_encryption_key(&encryption_key)?;

        // 4. Setup Index
        let index = Arc::new(RwLock::new(index::SemanticIndex::new()?));

        // 5. Setup Capture with privacy filter
        let capture = capture::ScreenCapture::new();

        println!(
            "[TimeMachine] Ready (interval: {}s, max: {}MB, retention: {} days)",
            config.capture_interval_secs, config.max_storage_mb, config.retention_days
        );

        Ok(Self {
            capture,
            ocr,
            embeddings,
            index,
            storage,
            npu,
            config,
            is_recording: Arc::new(AtomicBool::new(false)),
            is_paused: Arc::new(AtomicBool::new(false)),
            capture_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            privacy_blocked_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get encryption key from environment or derive from machine-specific data
    fn get_encryption_key() -> Result<String, Box<dyn std::error::Error>> {
        // Priority 1: Environment variable
        if let Ok(key) = std::env::var("EVA_TIMEMACHINE_KEY") {
            if key.len() >= 16 {
                println!("[TimeMachine] Using encryption key from environment");
                return Ok(key);
            } else {
                eprintln!(
                    "[TimeMachine] Warning: EVA_TIMEMACHINE_KEY too short (min 16 chars), using fallback"
                );
            }
        }

        // Priority 2: Derive from machine-specific data
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "eva_user".to_string());

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "eva_host".to_string());

        let derived_key = format!("eva_tm_{}_{}_secret", username, hostname);

        println!(
            "[TimeMachine] Warning: Using machine-derived key. Set EVA_TIMEMACHINE_KEY for production!"
        );

        Ok(derived_key)
    }

    /// Start recording (non-blocking, returns immediately)
    pub fn start_recording_async(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let tm = self.clone();
        tokio::spawn(async move {
            tm.start_recording().await;
        })
    }

    /// Start recording (blocking)
    pub async fn start_recording(&self) {
        if self.is_recording.load(Ordering::SeqCst) {
            println!("[TimeMachine] Already recording");
            return;
        }

        self.is_recording.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        println!("[TimeMachine] Recording started");

        let interval = tokio::time::Duration::from_secs(self.config.capture_interval_secs);

        while self.is_recording.load(Ordering::SeqCst) {
            tokio::time::sleep(interval).await;

            // Skip if paused
            if self.is_paused.load(Ordering::SeqCst) {
                continue;
            }

            // Capture
            self.capture_count.fetch_add(1, Ordering::SeqCst);

            match self.capture_and_process().await {
                Ok(_) => {
                    self.success_count.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("blocked") || err_msg.contains("privacy") {
                        self.privacy_blocked_count.fetch_add(1, Ordering::SeqCst);
                        // Privacy blocks are expected, don't log as error
                    } else {
                        self.error_count.fetch_add(1, Ordering::SeqCst);
                        eprintln!("[TimeMachine] Error: {}", err_msg);
                    }
                }
            }

            // Periodic cleanup
            let count = self.capture_count.load(Ordering::SeqCst);
            if count > 0 && count % self.config.cleanup_interval == 0 {
                if let Err(e) = self.run_cleanup().await {
                    eprintln!("[TimeMachine] Cleanup error: {}", e);
                }
            }
        }

        println!("[TimeMachine] Recording stopped");
    }

    /// Stop recording
    pub fn stop_recording(&self) {
        self.is_recording.store(false, Ordering::SeqCst);
        println!("[TimeMachine] Stop requested");
    }

    /// Pause recording (keeps running but skips captures)
    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::SeqCst);
        println!("[TimeMachine] Paused");
    }

    /// Resume recording
    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::SeqCst);
        println!("[TimeMachine] Resumed");
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<TimeMachineStats, Box<dyn std::error::Error>> {
        let storage_stats = self.storage.get_stats().await?;

        Ok(TimeMachineStats {
            total_captures: self.capture_count.load(Ordering::SeqCst),
            successful_captures: self.success_count.load(Ordering::SeqCst),
            blocked_by_privacy: self.privacy_blocked_count.load(Ordering::SeqCst),
            errors: self.error_count.load(Ordering::SeqCst),
            storage_used_mb: storage_stats.storage_used_mb,
        })
    }

    /// Run cleanup (storage rotation)
    async fn run_cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Cleanup old snapshots (by retention period)
        let deleted_by_age = self.storage.cleanup_old_snapshots().await?;

        // 2. Cleanup to meet storage limit
        let deleted_by_size = self.storage.cleanup_to_limit().await?;

        if deleted_by_age > 0 || deleted_by_size > 0 {
            println!(
                "[TimeMachine] Cleanup: removed {} by age, {} by size",
                deleted_by_age, deleted_by_size
            );
        }

        Ok(())
    }

    /// Capture and process a single screenshot
    async fn capture_and_process(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Capture (Privacy filtered)
        let screenshot = self.capture.take_screenshot()?;

        // 2. OCR
        let text = self.ocr.extract_text(&screenshot)?;

        // 3. Embed
        let embedding = self.embeddings.encode(&text)?;

        // 4. Storage (Encrypted)
        let screenshot_id = self.storage.save_screenshot(screenshot).await?;
        self.storage.save_metadata(screenshot_id, &text).await?;

        // 5. Index
        let mut idx = self.index.write().await;
        idx.add(screenshot_id, embedding, &text)?;

        Ok(())
    }

    /// Search by semantic similarity
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(u64, f32, String)>, Box<dyn std::error::Error>> {
        let query_vec = self.embeddings.encode(query)?;

        let idx = self.index.read().await;
        let results = idx.search(&query_vec, limit)?;

        let mut final_results = Vec::new();
        for (id, score) in results {
            let metadata = self.storage.load_metadata(id).await?;
            final_results.push((id, score, metadata.text));
        }

        Ok(final_results)
    }

    /// Search by full-text (SQL FTS5)
    pub async fn search_text(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(u64, String, f64)>, Box<dyn std::error::Error>> {
        self.storage.search_text(query, limit).await
    }

    /// Get a screenshot by ID
    pub async fn get_screenshot(&self, id: u64) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.storage.load_screenshot(id).await
    }

    /// Delete history for today (privacy feature)
    pub async fn delete_today(&self) -> Result<u64, Box<dyn std::error::Error>> {
        // This would need to be implemented in storage
        // For now, just trigger cleanup
        self.run_cleanup().await?;
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TimeMachineConfig::default();
        assert_eq!(config.capture_interval_secs, 10);
        assert_eq!(config.max_storage_mb, 5000);
        assert_eq!(config.retention_days, 30);
    }

    #[test]
    fn test_stats_default() {
        let stats = TimeMachineStats::default();
        assert_eq!(stats.total_captures, 0);
        assert_eq!(stats.successful_captures, 0);
    }
}
