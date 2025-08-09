use crate::models::Card;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Storage {
    data_file: PathBuf,
}

impl Storage {
    pub fn new(app_handle: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        // Use Tauri's app data directory (cross-platform)
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;

        std::fs::create_dir_all(&data_dir)?;
        let data_file = data_dir.join("cards.json");

        Ok(Storage { data_file })
    }

    pub fn load_cards(&self) -> Result<HashMap<String, Card>, Box<dyn std::error::Error>> {
        if self.data_file.exists() {
            let file = File::open(&self.data_file)?;
            let reader = BufReader::new(file);
            let cards: HashMap<String, Card> = serde_json::from_reader(reader).unwrap_or_default();
            Ok(cards)
        } else {
            Ok(HashMap::new())
        }
    }

    pub fn save_cards(&self, cards: &HashMap<String, Card>) -> Result<(), Box<dyn std::error::Error>> {
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(&self.data_file)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, cards)?;
        Ok(())
    }

    pub fn get_data_file_path(&self) -> &PathBuf {
        &self.data_file
    }
}
