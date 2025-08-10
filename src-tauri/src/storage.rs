use crate::models::{Card, AppSettings};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub struct Storage {
    data_file: PathBuf,
    settings_file: PathBuf,
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
        let settings_file = data_dir.join("settings.json");

        Ok(Storage { data_file, settings_file })
    }

    // Constructor for testing
    #[cfg(test)]
    pub fn new_with_path(data_file: PathBuf) -> Self {
        let mut settings_file = data_file.clone();
        settings_file.set_file_name("settings.json");
        Storage { data_file, settings_file }
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

    pub fn load_settings(&self) -> Result<AppSettings, Box<dyn std::error::Error>> {
        if self.settings_file.exists() {
            let file = File::open(&self.settings_file)?;
            let reader = BufReader::new(file);
            let settings: AppSettings = serde_json::from_reader(reader)?;
            Ok(settings)
        } else {
            Ok(AppSettings::default())
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(&self.settings_file)?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, settings)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Card;
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_card(id: &str) -> Card {
        Card {
            id: id.to_string(),
            front: format!("Question {}", id),
            back: format!("Answer {}", id),
            tag: Some("Test".to_string()),
            created_at: Utc::now(),
            last_reviewed: None,
            next_review: Utc::now(),
            interval: 0,
            ease_factor: 2.5,
            review_count: 0,
            correct_count: 0,
            leitner_box: 0,
            exponential_factor: 1.0,
        }
    }

    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_file = temp_dir.path().join("cards.json");
        let settings_file = temp_dir.path().join("settings.json");
        let storage = Storage { data_file, settings_file };
        (storage, temp_dir)
    }

    #[test]
    fn test_load_cards_empty_file() {
        let (storage, _temp_dir) = create_test_storage();
        let cards = storage.load_cards().unwrap();
        assert!(cards.is_empty());
    }

    #[test]
    fn test_save_and_load_cards() {
        let (storage, _temp_dir) = create_test_storage();

        let mut cards = HashMap::new();
        let card1 = create_test_card("1");
        let card2 = create_test_card("2");

        cards.insert("1".to_string(), card1.clone());
        cards.insert("2".to_string(), card2.clone());

        // Save cards
        storage.save_cards(&cards).unwrap();

        // Load cards
        let loaded_cards = storage.load_cards().unwrap();

        assert_eq!(loaded_cards.len(), 2);
        assert!(loaded_cards.contains_key("1"));
        assert!(loaded_cards.contains_key("2"));

        let loaded_card1 = &loaded_cards["1"];
        assert_eq!(loaded_card1.id, "1");
        assert_eq!(loaded_card1.front, "Question 1");
        assert_eq!(loaded_card1.back, "Answer 1");
        assert_eq!(loaded_card1.tag, Some("Test".to_string()));
    }

    #[test]
    fn test_save_empty_cards() {
        let (storage, _temp_dir) = create_test_storage();

        let cards = HashMap::new();
        storage.save_cards(&cards).unwrap();

        let loaded_cards = storage.load_cards().unwrap();
        assert!(loaded_cards.is_empty());
    }

    #[test]
    fn test_overwrite_cards() {
        let (storage, _temp_dir) = create_test_storage();

        // Save initial cards
        let mut cards1 = HashMap::new();
        cards1.insert("1".to_string(), create_test_card("1"));
        storage.save_cards(&cards1).unwrap();

        // Overwrite with different cards
        let mut cards2 = HashMap::new();
        cards2.insert("2".to_string(), create_test_card("2"));
        cards2.insert("3".to_string(), create_test_card("3"));
        storage.save_cards(&cards2).unwrap();

        // Load and verify
        let loaded_cards = storage.load_cards().unwrap();
        assert_eq!(loaded_cards.len(), 2);
        assert!(!loaded_cards.contains_key("1"));
        assert!(loaded_cards.contains_key("2"));
        assert!(loaded_cards.contains_key("3"));
    }

    #[test]
    fn test_corrupted_file_handling() {
        let (storage, temp_dir) = create_test_storage();

        // Write invalid JSON to the file
        let data_file_path = temp_dir.path().join("cards.json");
        std::fs::write(&data_file_path, "invalid json").unwrap();

        // Should return empty HashMap instead of crashing
        let cards = storage.load_cards().unwrap();
        assert!(cards.is_empty());
    }

    #[test]
    fn test_file_persistence() {
        let (storage, _temp_dir) = create_test_storage();

        let mut cards = HashMap::new();
        let card = create_test_card("persistence_test");
        cards.insert("persistence_test".to_string(), card);

        storage.save_cards(&cards).unwrap();

        // Verify file exists
        assert!(storage.data_file.exists());

        // Verify file content is valid JSON
        let content = std::fs::read_to_string(&storage.data_file).unwrap();
        let _: HashMap<String, Card> = serde_json::from_str(&content).unwrap();
    }
}
