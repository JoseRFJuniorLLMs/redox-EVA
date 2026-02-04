use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use argon2::{
    password_hash::{
        rand_core::RngCore,
        PasswordHasher, SaltString
    },
    Argon2
};
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};
use std::error::Error;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use image::DynamicImage;
use chrono::{DateTime, Utc};

pub struct Storage {
    base_path: PathBuf,
    db_path: PathBuf,
    cipher: Option<Aes256Gcm>,
}

pub struct Metadata {
    pub timestamp: DateTime<Utc>,
    pub text: String,
}

impl Storage {
    pub async fn new(path_str: &str) -> Result<Self, Box<dyn Error>> {
        // Expand ~ to user home if needed (simplified here, assumes valid path or absolute)
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
            cipher: None, // Initialized later with password
        };

        storage.init_db()?;
        
        Ok(storage)
    }

    fn init_db(&self) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS screenshots (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                text_content TEXT,
                tags TEXT
            )",
            [],
        )?;
        Ok(())
    }

    pub fn set_encryption_key(&mut self, password: &str) -> Result<(), Box<dyn Error>> {
        // In a real app, we would read/store a salt from a config file
        // For now, generating a dummy key derivation for demonstration
        let salt = SaltString::generate(&mut OsRng); // Should be persistent
        let argon2 = Argon2::default();
        
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
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
        image.write_to(&mut std::io::Cursor::new(&mut image_bytes), image::ImageOutputFormat::Png)?;

        // 2. Compress
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&image_bytes)?;
        let compressed_bytes = encoder.finish()?;

        // 3. Encrypt
        let final_bytes = if let Some(cipher) = &self.cipher {
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            let ciphertext = cipher.encrypt(&nonce, compressed_bytes.as_ref())
                .map_err(|e| format!("Encryption failed: {}", e))?;
            
            // Prepend nonce to ciphertext so we can decrypt later
            let mut result = nonce.to_vec();
            result.extend(ciphertext);
            result
        } else {
            compressed_bytes // Warning: Unencrypted if key not set!
        };

        // 4. Save to disk
        // Organize by Date: YYYY-MM-DD/HH-MM-SS.enc
        let date_folder = timestamp.format("%Y-%m-%d").to_string();
        let file_name = format!("{}.enc", timestamp.format("%H-%M-%S"));
        
        let dir_path = self.base_path.join("screenshots").join(&date_folder);
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
        }
        
        let file_path = dir_path.join(file_name);
        fs::write(file_path, final_bytes)?;

        // 5. Insert into dummy DB entry (ID returned)
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO screenshots (timestamp, text_content) VALUES (?1, ?2)",
            params![timestamp_str, ""], // Text content update later
        )?;
        
        let id = conn.last_insert_rowid() as u64;
        Ok(id)
    }

    pub async fn save_metadata(&self, id: u64, text: &str) -> Result<(), Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;
        // Ideally metadata is also encrypted in the DB (SQLCipher)
        // Here we store plaintext for search demo, or we would index encrypted terms
        conn.execute(
            "UPDATE screenshots SET text_content = ?1 WHERE id = ?2",
            params![text, id],
        )?;
        Ok(())
    }
    
    pub async fn load_metadata(&self, id: u64) -> Result<Metadata, Box<dyn Error>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare("SELECT timestamp, text_content FROM screenshots WHERE id = ?1")?;
        
        let metadata = stmt.query_row(params![id], |row| {
            let ts_str: String = row.get(0)?;
            let text: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
                
            Ok(Metadata { timestamp, text })
        })?;
        
        Ok(metadata)
    }
    
    pub async fn load_screenshot(&self, _id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
        // Placeholder: Needs mapping ID to filename logic (reverse of save)
        // For now returning empty
        Ok(vec![]) 
    }
}
