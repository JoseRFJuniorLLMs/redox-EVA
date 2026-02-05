use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use image::DynamicImage;
use rusqlite::{params, Connection};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Default storage limits
const DEFAULT_MAX_STORAGE_MB: u64 = 5000; // 5GB default
const DEFAULT_RETENTION_DAYS: i64 = 30;   // 30 days default

pub struct Storage {
    base_path: PathBuf,
    db_path: PathBuf,
    cipher: Option<Aes256Gcm>,
    /// Maximum storage in megabytes
    max_storage_mb: u64,
    /// Retention period in days
    retention_days: i64,
}

pub struct Metadata {
    pub timestamp: DateTime<Utc>,
    pub text: String,
}

pub struct StorageStats {
    pub total_screenshots: u64,
    pub storage_used_mb: f64,
    pub oldest_screenshot: Option<DateTime<Utc>>,
    pub newest_screenshot: Option<DateTime<Utc>>,
}

impl Storage {
    pub async fn new(path_str: &str) -> Result<Self, Box<dyn Error>> {
        let base_path = if path_str.starts_with("~") {
            let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))?;
            PathBuf::from(path_str.replace("~", &home))
        } else {
            PathBuf::from(path_str)
        };

        if !base_path.exists() {
            fs::create_dir_all(&base_path)?;
        }

        let db_path = base_path.join("metadata.db");

        let storage = Self {
            base_path,
            db_path,
            cipher: None,
            max_storage_mb: DEFAULT_MAX_STORAGE_MB,
            retention_days: DEFAULT_RETENTION_DAYS,
        };

        storage.init_db()?;

        Ok(storage)
    }

    /// Initialize database with proper indices
    fn init_db(&self) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;

        // Create main table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS screenshots (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                text_content TEXT,
                tags TEXT,
                file_path TEXT,
                file_size INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Create indices for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_screenshots_timestamp ON screenshots(timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_screenshots_text ON screenshots(text_content)",
            [],
        )?;

        // Create FTS (Full-Text Search) virtual table for better text search
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS screenshots_fts USING fts5(
                text_content,
                content='screenshots',
                content_rowid='id'
            )",
            [],
        )?;

        // Create trigger to keep FTS in sync
        conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS screenshots_ai AFTER INSERT ON screenshots BEGIN
                INSERT INTO screenshots_fts(rowid, text_content) VALUES (new.id, new.text_content);
            END;
            CREATE TRIGGER IF NOT EXISTS screenshots_ad AFTER DELETE ON screenshots BEGIN
                INSERT INTO screenshots_fts(screenshots_fts, rowid, text_content) VALUES('delete', old.id, old.text_content);
            END;
            CREATE TRIGGER IF NOT EXISTS screenshots_au AFTER UPDATE ON screenshots BEGIN
                INSERT INTO screenshots_fts(screenshots_fts, rowid, text_content) VALUES('delete', old.id, old.text_content);
                INSERT INTO screenshots_fts(rowid, text_content) VALUES (new.id, new.text_content);
            END;
            "
        )?;

        println!("[Storage] Database initialized with indices and FTS");
        Ok(())
    }

    /// Set storage limits
    pub fn set_limits(&mut self, max_storage_mb: u64, retention_days: i64) {
        self.max_storage_mb = max_storage_mb;
        self.retention_days = retention_days;
    }

    pub fn set_encryption_key(&mut self, password: &str) -> Result<(), Box<dyn Error>> {
        // Use a fixed salt for deterministic key derivation
        let salt = SaltString::from_b64("RXZhVGltZU1hY2hpbmU")
            .map_err(|e| format!("Salt error: {}", e))?;

        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Hashing failed: {}", e))?;

        let hash = password_hash.hash.ok_or("No hash generated")?;
        let mut key_bytes = [0u8; 32];
        let len = std::cmp::min(hash.len(), 32);
        key_bytes[..len].copy_from_slice(&hash.as_bytes()[..len]);

        self.cipher = Some(Aes256Gcm::new(&key_bytes.into()));
        Ok(())
    }

    pub async fn save_screenshot(&self, image: DynamicImage) -> Result<u64, Box<dyn Error>> {
        let timestamp = Utc::now();
        let timestamp_str = timestamp.to_rfc3339();

        // 1. Convert image to bytes (PNG)
        let mut image_bytes = Vec::new();
        image.write_to(
            &mut std::io::Cursor::new(&mut image_bytes),
            image::ImageFormat::Png,
        )?;

        // 2. Compress
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&image_bytes)?;
        let compressed_bytes = encoder.finish()?;

        // 3. Encrypt
        let final_bytes = if let Some(cipher) = &self.cipher {
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
            let ciphertext = cipher
                .encrypt(&nonce, compressed_bytes.as_ref())
                .map_err(|e| format!("Encryption failed: {}", e))?;

            let mut result = nonce.to_vec();
            result.extend(ciphertext);
            result
        } else {
            eprintln!("[Storage] Warning: Saving unencrypted screenshot!");
            compressed_bytes
        };

        // 4. Save to disk
        let date_folder = timestamp.format("%Y-%m-%d").to_string();
        let file_name = format!("{}.enc", timestamp.format("%H-%M-%S-%3f"));

        let dir_path = self.base_path.join("screenshots").join(&date_folder);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
        }

        let file_path = dir_path.join(&file_name);
        let file_size = final_bytes.len() as i64;
        fs::write(&file_path, &final_bytes)?;

        // 5. Insert into DB with file path
        let conn = Connection::open(&self.db_path)?;
        let relative_path = format!("screenshots/{}/{}", date_folder, file_name);

        conn.execute(
            "INSERT INTO screenshots (timestamp, text_content, file_path, file_size) VALUES (?1, ?2, ?3, ?4)",
            params![timestamp_str, "", relative_path, file_size],
        )?;

        let id = conn.last_insert_rowid() as u64;
        Ok(id)
    }

    pub async fn save_metadata(&self, id: u64, text: &str) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "UPDATE screenshots SET text_content = ?1 WHERE id = ?2",
            params![text, id],
        )?;
        Ok(())
    }

    pub async fn load_metadata(&self, id: u64) -> Result<Metadata, Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt =
            conn.prepare("SELECT timestamp, text_content FROM screenshots WHERE id = ?1")?;

        let metadata = stmt.query_row(params![id], |row| {
            let ts_str: String = row.get(0)?;
            let text: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            Ok(Metadata { timestamp, text })
        })?;

        Ok(metadata)
    }

    /// Load and decrypt a screenshot by ID
    pub async fn load_screenshot(&self, id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;
        let file_path: String = conn.query_row(
            "SELECT file_path FROM screenshots WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        let full_path = self.base_path.join(&file_path);

        if !full_path.exists() {
            return Err(format!("Screenshot file not found: {}", file_path).into());
        }

        let encrypted_data = fs::read(&full_path)?;

        // Decrypt
        let compressed_data = if let Some(cipher) = &self.cipher {
            if encrypted_data.len() < 12 {
                return Err("Encrypted data too short".into());
            }

            let nonce = Nonce::from_slice(&encrypted_data[..12]);
            let ciphertext = &encrypted_data[12..];

            cipher
                .decrypt(nonce, ciphertext)
                .map_err(|e| format!("Decryption failed: {}", e))?
        } else {
            encrypted_data
        };

        // Decompress
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        Ok(decompressed)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> Result<StorageStats, Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;

        let total_screenshots: u64 = conn.query_row(
            "SELECT COUNT(*) FROM screenshots",
            [],
            |row| row.get(0),
        )?;

        let total_size: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(file_size), 0) FROM screenshots",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let oldest: Option<String> = conn
            .query_row(
                "SELECT MIN(timestamp) FROM screenshots",
                [],
                |row| row.get(0),
            )
            .ok();

        let newest: Option<String> = conn
            .query_row(
                "SELECT MAX(timestamp) FROM screenshots",
                [],
                |row| row.get(0),
            )
            .ok();

        Ok(StorageStats {
            total_screenshots,
            storage_used_mb: total_size as f64 / 1024.0 / 1024.0,
            oldest_screenshot: oldest.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            newest_screenshot: newest.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        })
    }

    /// Get current storage usage in MB
    pub async fn get_used_space_mb(&self) -> Result<f64, Box<dyn Error>> {
        let stats = self.get_stats().await?;
        Ok(stats.storage_used_mb)
    }

    /// Cleanup old screenshots based on retention policy
    pub async fn cleanup_old_snapshots(&self) -> Result<u64, Box<dyn Error>> {
        let cutoff = Utc::now() - Duration::days(self.retention_days);
        let cutoff_str = cutoff.to_rfc3339();

        let conn = Connection::open(&self.db_path)?;

        // Get files to delete
        let mut stmt = conn.prepare(
            "SELECT id, file_path FROM screenshots WHERE timestamp < ?1"
        )?;

        let files_to_delete: Vec<(u64, String)> = stmt
            .query_map(params![cutoff_str], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut deleted_count = 0;

        for (id, file_path) in &files_to_delete {
            // Delete file from disk
            let full_path = self.base_path.join(file_path);
            if full_path.exists() {
                if let Err(e) = fs::remove_file(&full_path) {
                    eprintln!("[Storage] Failed to delete file {}: {}", file_path, e);
                    continue;
                }
            }

            // Delete from database
            conn.execute("DELETE FROM screenshots WHERE id = ?1", params![id])?;
            deleted_count += 1;
        }

        // Clean up empty date folders
        self.cleanup_empty_folders()?;

        if deleted_count > 0 {
            println!("[Storage] Cleaned up {} old screenshots", deleted_count);
        }

        Ok(deleted_count)
    }

    /// Cleanup to meet storage limits
    pub async fn cleanup_to_limit(&self) -> Result<u64, Box<dyn Error>> {
        let mut deleted_count = 0;

        loop {
            let used_mb = self.get_used_space_mb().await?;

            if used_mb <= self.max_storage_mb as f64 {
                break;
            }

            // Delete oldest screenshot
            let conn = Connection::open(&self.db_path)?;

            let oldest: Option<(u64, String)> = conn
                .query_row(
                    "SELECT id, file_path FROM screenshots ORDER BY timestamp ASC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((id, file_path)) = oldest {
                let full_path = self.base_path.join(&file_path);
                if full_path.exists() {
                    fs::remove_file(&full_path)?;
                }
                conn.execute("DELETE FROM screenshots WHERE id = ?1", params![id])?;
                deleted_count += 1;
            } else {
                break; // No more screenshots to delete
            }
        }

        if deleted_count > 0 {
            self.cleanup_empty_folders()?;
            println!("[Storage] Removed {} screenshots to meet storage limit", deleted_count);
        }

        Ok(deleted_count)
    }

    /// Remove empty date folders
    fn cleanup_empty_folders(&self) -> Result<(), Box<dyn Error>> {
        let screenshots_dir = self.base_path.join("screenshots");

        if !screenshots_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&screenshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Check if directory is empty
                let is_empty = fs::read_dir(&path)?.next().is_none();
                if is_empty {
                    fs::remove_dir(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Full-text search in screenshots
    pub async fn search_text(&self, query: &str, limit: usize) -> Result<Vec<(u64, String, f64)>, Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;

        let mut stmt = conn.prepare(
            "SELECT id, text_content, bm25(screenshots_fts) as score
             FROM screenshots_fts
             WHERE text_content MATCH ?1
             ORDER BY score
             LIMIT ?2"
        )?;

        let results: Vec<(u64, String, f64)> = stmt
            .query_map(params![query, limit as i64], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_creation() {
        let temp_dir = std::env::temp_dir().join("eva_test_storage");
        let storage = Storage::new(temp_dir.to_str().unwrap()).await.unwrap();

        assert!(storage.db_path.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_stats() {
        let temp_dir = std::env::temp_dir().join("eva_test_stats");
        let storage = Storage::new(temp_dir.to_str().unwrap()).await.unwrap();

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_screenshots, 0);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
