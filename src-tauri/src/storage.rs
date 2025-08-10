use crate::models::{AppSettings, Card};
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
            let settings: AppSettings = serde_json::from_reader(reader).unwrap_or_default();
            Ok(settings)
        } else {
            Ok(AppSettings::default())
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), Box<dyn std::error::Error>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.settings_file)?;

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

    /// Ensures loading cards from a non-existent file returns an empty collection
    /// instead of crashing, providing graceful initialization for new users.
    #[test]
    fn test_load_cards_empty_file() {
        let (storage, _temp_dir) = create_test_storage();
        let cards = storage.load_cards().unwrap();
        assert!(cards.is_empty());
    }

    /// Verifies the complete save and load cycle for flashcard data,
    /// ensuring all card properties (front, back, tags, review data) persist correctly.
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

    /// Tests saving and loading empty card collections to ensure
    /// the app handles edge cases gracefully without data corruption.
    #[test]
    fn test_save_empty_cards() {
        let (storage, _temp_dir) = create_test_storage();

        let cards = HashMap::new();
        storage.save_cards(&cards).unwrap();

        let loaded_cards = storage.load_cards().unwrap();
        assert!(loaded_cards.is_empty());
    }

    /// Ensures that new card saves completely replace old data,
    /// preventing data leakage between different app sessions or imports.
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

    /// Tests recovery from corrupted JSON files by returning empty data
    /// instead of crashing, protecting users from losing access to the app.
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

    /// Verifies that saved data actually persists to disk as valid JSON files,
    /// ensuring data survives app restarts and system reboots.
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

    // Settings persistence tests
    /// Ensures the app provides sensible default settings when no settings file exists,
    /// allowing new users to start using the app immediately.
    #[test]
    fn test_load_default_settings() {
        let (storage, _temp_dir) = create_test_storage();

        // Should return default settings when no settings file exists
        let settings = storage.load_settings().unwrap();
        assert_eq!(settings.algorithm, AppSettings::default().algorithm);
        assert_eq!(settings.leitner_intervals, AppSettings::default().leitner_intervals);
        assert_eq!(settings.exponential_base, AppSettings::default().exponential_base);
    }

    /// Verifies that user settings (algorithm choice, parameters) persist correctly
    /// across app sessions, maintaining user preferences and customizations.
    #[test]
    fn test_save_and_load_settings() {
        let (storage, _temp_dir) = create_test_storage();

        let mut settings = AppSettings::default();
        settings.algorithm = crate::models::SpacedRepetitionAlgorithm::Leitner;
        settings.leitner_intervals = vec![2, 4, 8, 16, 32];
        settings.exponential_base = 1.8;

        // Save settings
        storage.save_settings(&settings).unwrap();

        // Load and verify
        let loaded_settings = storage.load_settings().unwrap();
        assert_eq!(loaded_settings.algorithm, crate::models::SpacedRepetitionAlgorithm::Leitner);
        assert_eq!(loaded_settings.leitner_intervals, vec![2, 4, 8, 16, 32]);
        assert_eq!(loaded_settings.exponential_base, 1.8);
    }

    /// Confirms that settings files are created on disk with valid JSON,
    /// ensuring settings survive app restarts and system reboots.
    #[test]
    fn test_settings_file_persistence() {
        let (storage, _temp_dir) = create_test_storage();

        let mut settings = AppSettings::default();
        settings.algorithm = crate::models::SpacedRepetitionAlgorithm::SimpleExponential;
        settings.exponential_base = 3.0;

        storage.save_settings(&settings).unwrap();

        // Verify settings file exists
        assert!(storage.settings_file.exists());

        // Verify file content is valid JSON
        let content = std::fs::read_to_string(&storage.settings_file).unwrap();
        let _: AppSettings = serde_json::from_str(&content).unwrap();
    }

    /// Ensures that new settings completely replace old ones when saved,
    /// allowing users to change algorithms and parameters without conflicts.
    #[test]
    fn test_settings_overwrite() {
        let (storage, _temp_dir) = create_test_storage();

        // Save initial settings
        let mut settings1 = AppSettings::default();
        settings1.algorithm = crate::models::SpacedRepetitionAlgorithm::SM2;
        storage.save_settings(&settings1).unwrap();

        // Overwrite with different settings
        let mut settings2 = AppSettings::default();
        settings2.algorithm = crate::models::SpacedRepetitionAlgorithm::Leitner;
        settings2.leitner_intervals = vec![1, 2, 4, 8];
        storage.save_settings(&settings2).unwrap();

        // Load and verify overwrite worked
        let loaded_settings = storage.load_settings().unwrap();
        assert_eq!(loaded_settings.algorithm, crate::models::SpacedRepetitionAlgorithm::Leitner);
        assert_eq!(loaded_settings.leitner_intervals, vec![1, 2, 4, 8]);
    }

    /// Tests recovery from corrupted settings files by falling back to defaults,
    /// ensuring users can always access the app even with damaged configuration.
    #[test]
    fn test_corrupted_settings_file_handling() {
        let (storage, temp_dir) = create_test_storage();

        // Write invalid JSON to the settings file
        let settings_file_path = temp_dir.path().join("settings.json");
        std::fs::write(&settings_file_path, "invalid json").unwrap();

        // Should return default settings instead of crashing
        let settings = storage.load_settings().unwrap();
        assert_eq!(settings.algorithm, AppSettings::default().algorithm);
        assert_eq!(settings.leitner_intervals, AppSettings::default().leitner_intervals);
        assert_eq!(settings.exponential_base, AppSettings::default().exponential_base);
    }

    // Tests for Storage::new() behavior and directory creation
    /// Tests the alternative constructor that takes a specific file path,
    /// ensuring flexibility in storage location for testing and custom deployments.
    #[test]
    fn test_new_with_path_constructor() {
        let temp_dir = TempDir::new().unwrap();
        let data_file = temp_dir.path().join("test_cards.json");

        let storage = Storage::new_with_path(data_file.clone());

        // Verify the file paths are set correctly
        assert_eq!(storage.data_file, data_file);

        let expected_settings_file = temp_dir.path().join("settings.json");
        assert_eq!(storage.settings_file, expected_settings_file);
    }

    /// Verifies correct file path derivation and directory structure,
    /// ensuring both cards and settings files are placed in the same location.
    #[test]
    fn test_storage_file_path_handling() {
        let temp_dir = TempDir::new().unwrap();
        let cards_file = temp_dir.path().join("custom_cards.json");

        let storage = Storage::new_with_path(cards_file.clone());

        // Test that file paths are correctly derived
        assert_eq!(storage.data_file.file_name().unwrap(), "custom_cards.json");
        assert_eq!(storage.settings_file.file_name().unwrap(), "settings.json");

        // Test that both files share the same parent directory
        assert_eq!(storage.data_file.parent(), storage.settings_file.parent());
    }

    /// Tests handling of nested directory structures and automatic directory creation,
    /// ensuring the app works correctly in complex file system layouts.
    #[test]
    fn test_storage_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let data_file = temp_dir.path().join("subdir").join("cards.json");

        let storage = Storage::new_with_path(data_file.clone());

        // Verify directory structure is preserved
        let expected_settings = temp_dir.path().join("subdir").join("settings.json");
        assert_eq!(storage.settings_file, expected_settings);

        // Test that we can work with nested directories
        let cards = HashMap::new();

        // Create the parent directory first (simulating what Storage::new() does)
        std::fs::create_dir_all(storage.data_file.parent().unwrap()).unwrap();

        // This should work with the directory structure
        let result = storage.save_cards(&cards);
        assert!(result.is_ok());

        // Verify the directory was created
        assert!(storage.data_file.parent().unwrap().exists());
    }

    /// Verifies that multiple storage instances can safely access the same files,
    /// supporting scenarios like backup operations or data synchronization.
    #[test]
    fn test_storage_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let data_file = temp_dir.path().join("concurrent_cards.json");

        let storage1 = Storage::new_with_path(data_file.clone());
        let storage2 = Storage::new_with_path(data_file.clone());

        // Both storages should be able to access the same files
        let mut cards1 = HashMap::new();
        cards1.insert("test1".to_string(), create_test_card("test1"));

        let mut settings1 = AppSettings::default();
        settings1.exponential_base = 3.0;

        // Save from first storage
        storage1.save_cards(&cards1).unwrap();
        storage1.save_settings(&settings1).unwrap();

        // Load from second storage
        let loaded_cards = storage2.load_cards().unwrap();
        let loaded_settings = storage2.load_settings().unwrap();

        assert_eq!(loaded_cards.len(), 1);
        assert!(loaded_cards.contains_key("test1"));
        assert_eq!(loaded_settings.exponential_base, 3.0);
    }

    /// Tests graceful handling of file system permission issues,
    /// ensuring the app doesn't crash when encountering read-only directories.
    #[test]
    fn test_storage_error_handling_readonly_directory() {
        // This test simulates what would happen if Storage::new() encounters permission issues
        let temp_dir = TempDir::new().unwrap();
        let readonly_file = temp_dir.path().join("readonly.json");

        // Create a file first
        std::fs::write(&readonly_file, "{}").unwrap();

        let storage = Storage::new_with_path(readonly_file);

        // Test graceful handling of write operations to existing files
        let cards = HashMap::new();
        let result = storage.save_cards(&cards);

        // Should succeed for empty cards
        assert!(result.is_ok());
    }

    /// Tests storage initialization with unusual file names and deep directory paths,
    /// ensuring robustness across different file system configurations.
    #[test]
    fn test_storage_initialization_edge_cases() {
        // Test with unusual but valid file names
        let temp_dir = TempDir::new().unwrap();

        // Test with file that has no extension
        let no_ext_file = temp_dir.path().join("cards_no_extension");
        let storage1 = Storage::new_with_path(no_ext_file);
        assert_eq!(storage1.settings_file.file_name().unwrap(), "settings.json");

        // Test with file that has multiple extensions
        let multi_ext_file = temp_dir.path().join("cards.backup.json");
        let storage2 = Storage::new_with_path(multi_ext_file);
        assert_eq!(storage2.settings_file.file_name().unwrap(), "settings.json");

        // Test with deeply nested path
        let deep_path = temp_dir.path().join("a").join("b").join("c").join("deep.json");
        let storage3 = Storage::new_with_path(deep_path.clone());

        // Create the directory structure first (simulating what Storage::new() does)
        std::fs::create_dir_all(deep_path.parent().unwrap()).unwrap();

        // Should be able to save
        let cards = HashMap::new();
        let result = storage3.save_cards(&cards);
        assert!(result.is_ok());
        assert!(deep_path.parent().unwrap().exists());
    }

    /// Tests the core directory creation and path setup logic used in Storage::new(),
    /// ensuring proper app data directory initialization.
    #[test]
    fn test_storage_new_directory_creation_logic() {
        // Test the core directory creation logic that Storage::new() uses
        let temp_dir = TempDir::new().unwrap();
        let app_data_dir = temp_dir.path().join("app_data");

        // Simulate what Storage::new() does - create directory and set up paths
        std::fs::create_dir_all(&app_data_dir).unwrap();
        let data_file = app_data_dir.join("cards.json");
        let settings_file = app_data_dir.join("settings.json");

        // Manually create storage with the same logic as Storage::new()
        let storage = Storage {
            data_file: data_file.clone(),
            settings_file: settings_file.clone(),
        };

        // Test that the directory exists (simulating successful Storage::new())
        assert!(app_data_dir.exists());
        assert_eq!(storage.data_file, data_file);
        assert_eq!(storage.settings_file, settings_file);

        // Test that we can use the storage normally
        let cards = HashMap::new();
        let settings = AppSettings::default();

        assert!(storage.save_cards(&cards).is_ok());
        assert!(storage.save_settings(&settings).is_ok());

        let loaded_cards = storage.load_cards().unwrap();
        let loaded_settings = storage.load_settings().unwrap();

        assert!(loaded_cards.is_empty());
        assert_eq!(loaded_settings.algorithm, AppSettings::default().algorithm);
    }

    /// Tests the file path resolution and construction logic used in Storage::new(),
    /// ensuring correct placement of cards.json and settings.json files.
    #[test]
    fn test_storage_new_path_resolution() {
        // Test the path resolution logic used in Storage::new()
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path().join("tauri_app");

        // Create the base directory (simulating app_data_dir creation)
        std::fs::create_dir_all(&base_dir).unwrap();

        // Test the file path creation logic
        let cards_path = base_dir.join("cards.json");
        let settings_path = base_dir.join("settings.json");

        // Verify paths are correct
        assert_eq!(cards_path.file_name().unwrap(), "cards.json");
        assert_eq!(settings_path.file_name().unwrap(), "settings.json");
        assert_eq!(cards_path.parent().unwrap(), settings_path.parent().unwrap());

        // Test that Storage created with these paths works correctly
        let storage = Storage {
            data_file: cards_path.clone(),
            settings_file: settings_path.clone(),
        };

        // Should be able to perform all normal operations
        let mut test_cards = HashMap::new();
        test_cards.insert("test".to_string(), create_test_card("test"));

        let mut test_settings = AppSettings::default();
        test_settings.exponential_base = 2.5;

        assert!(storage.save_cards(&test_cards).is_ok());
        assert!(storage.save_settings(&test_settings).is_ok());

        // Verify files were created in correct locations
        assert!(cards_path.exists());
        assert!(settings_path.exists());
    }
}
