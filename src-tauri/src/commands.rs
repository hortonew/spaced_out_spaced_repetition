use crate::card_service::CardService;
use crate::models::{
    BulkUpdateRequest, Card, CategoryStats, CreateCardRequest, ReviewDifficulty, ReviewStats, SearchRequest, UpdateCardRequest,
};
use tauri::State;

// Card management commands
#[tauri::command]
pub async fn create_card(service: State<'_, CardService>, request: CreateCardRequest) -> Result<Card, String> {
    service.create_card(request)
}

#[tauri::command]
pub async fn get_cards(service: State<'_, CardService>) -> Result<Vec<Card>, String> {
    service.get_cards()
}

#[tauri::command]
pub async fn get_card(service: State<'_, CardService>, id: String) -> Result<Option<Card>, String> {
    service.get_card(id)
}

#[tauri::command]
pub async fn update_card(service: State<'_, CardService>, id: String, request: UpdateCardRequest) -> Result<Card, String> {
    service.update_card(id, request)
}

#[tauri::command]
pub async fn delete_card(service: State<'_, CardService>, id: String) -> Result<(), String> {
    service.delete_card(id)
}

// Review session commands
#[tauri::command]
pub async fn get_due_cards(service: State<'_, CardService>) -> Result<Vec<Card>, String> {
    service.get_due_cards()
}

#[tauri::command]
pub async fn review_card(service: State<'_, CardService>, id: String, difficulty: u8) -> Result<Card, String> {
    let difficulty = ReviewDifficulty::from_u8(difficulty)?;
    service.review_card(id, difficulty)
}

#[tauri::command]
pub async fn get_review_stats(service: State<'_, CardService>) -> Result<ReviewStats, String> {
    service.get_review_stats()
}

// Organization and search commands
#[tauri::command]
pub async fn search_cards(service: State<'_, CardService>, request: SearchRequest) -> Result<Vec<Card>, String> {
    service.search_cards(request)
}

#[tauri::command]
pub async fn get_categories(service: State<'_, CardService>) -> Result<Vec<String>, String> {
    service.get_categories()
}

#[tauri::command]
pub async fn get_category_stats(service: State<'_, CardService>) -> Result<Vec<CategoryStats>, String> {
    service.get_category_stats()
}

#[tauri::command]
pub async fn bulk_update_category(service: State<'_, CardService>, request: BulkUpdateRequest) -> Result<Vec<Card>, String> {
    service.bulk_update_category(request)
}

#[tauri::command]
pub async fn delete_multiple_cards(service: State<'_, CardService>, card_ids: Vec<String>) -> Result<(), String> {
    service.delete_multiple_cards(card_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use serial_test::serial;
    use tempfile::TempDir;

    // Helper to create a test card service wrapped in State-like structure
    fn create_test_service() -> (CardService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_file = temp_dir.path().join("test_cards.json");
        let storage = Storage::new_with_path(data_file);
        let service = CardService::new(storage).unwrap();
        (service, temp_dir)
    }

    #[tokio::test]
    #[serial]
    async fn test_create_card_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "Test Question".to_string(),
            back: "Test Answer".to_string(),
            category: Some("Test".to_string()),
        };

        let result = service.create_card(request);
        assert!(result.is_ok());

        let card = result.unwrap();
        assert_eq!(card.front, "Test Question");
        assert_eq!(card.back, "Test Answer");
        assert_eq!(card.category, Some("Test".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_get_cards_command() {
        let (service, _temp_dir) = create_test_service();

        // Initially empty
        let result = service.get_cards();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Add a card
        let request = CreateCardRequest {
            front: "Q".to_string(),
            back: "A".to_string(),
            category: None,
        };
        service.create_card(request).unwrap();

        let result = service.get_cards();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_card_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "Question".to_string(),
            back: "Answer".to_string(),
            category: None,
        };
        let created_card = service.create_card(request).unwrap();

        let result = service.get_card(created_card.id.clone());
        assert!(result.is_ok());

        let retrieved = result.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, created_card.id);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_card_command() {
        let (service, _temp_dir) = create_test_service();
        let create_request = CreateCardRequest {
            front: "Original".to_string(),
            back: "Original".to_string(),
            category: None,
        };
        let created_card = service.create_card(create_request).unwrap();

        let update_request = UpdateCardRequest {
            front: "Updated".to_string(),
            back: "Updated".to_string(),
            category: Some("New Category".to_string()),
        };

        let result = service.update_card(created_card.id, update_request);
        assert!(result.is_ok());

        let updated_card = result.unwrap();
        assert_eq!(updated_card.front, "Updated");
        assert_eq!(updated_card.back, "Updated");
        assert_eq!(updated_card.category, Some("New Category".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_card_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "To Delete".to_string(),
            back: "Answer".to_string(),
            category: None,
        };
        let created_card = service.create_card(request).unwrap();

        let result = service.delete_card(created_card.id.clone());
        assert!(result.is_ok());

        // Verify deletion
        let get_result = service.get_card(created_card.id);
        assert!(get_result.is_ok());
        assert!(get_result.unwrap().is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_review_card_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "Review Test".to_string(),
            back: "Answer".to_string(),
            category: None,
        };
        let created_card = service.create_card(request).unwrap();

        let result = service.review_card(created_card.id, ReviewDifficulty::Good);
        assert!(result.is_ok());

        let reviewed_card = result.unwrap();
        assert_eq!(reviewed_card.review_count, 1);
        assert_eq!(reviewed_card.correct_count, 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_due_cards_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "Due Card".to_string(),
            back: "Answer".to_string(),
            category: None,
        };
        service.create_card(request).unwrap();

        let result = service.get_due_cards();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_review_stats_command() {
        let (service, _temp_dir) = create_test_service();

        let result = service.get_review_stats();
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.total_cards, 0);
        assert_eq!(stats.cards_due, 0);
    }

    #[tokio::test]
    #[serial]
    async fn test_search_cards_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "Searchable content".to_string(),
            back: "Answer".to_string(),
            category: Some("Test".to_string()),
        };
        service.create_card(request).unwrap();

        let search_request = SearchRequest {
            query: Some("Searchable".to_string()),
            category: None,
            tags: None,
        };

        let result = service.search_cards(search_request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_categories_command() {
        let (service, _temp_dir) = create_test_service();
        let request = CreateCardRequest {
            front: "Q".to_string(),
            back: "A".to_string(),
            category: Some("TestCategory".to_string()),
        };
        service.create_card(request).unwrap();

        let result = service.get_categories();
        assert!(result.is_ok());

        let categories = result.unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0], "TestCategory");
    }

    #[tokio::test]
    #[serial]
    async fn test_bulk_update_category_command() {
        let (service, _temp_dir) = create_test_service();
        let card1 = service
            .create_card(CreateCardRequest {
                front: "Q1".to_string(),
                back: "A1".to_string(),
                category: Some("Old".to_string()),
            })
            .unwrap();

        let bulk_request = BulkUpdateRequest {
            card_ids: vec![card1.id],
            category: Some("New".to_string()),
        };

        let result = service.bulk_update_category(bulk_request);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_multiple_cards_command() {
        let (service, _temp_dir) = create_test_service();
        let card1 = service
            .create_card(CreateCardRequest {
                front: "Q1".to_string(),
                back: "A1".to_string(),
                category: None,
            })
            .unwrap();

        let card2 = service
            .create_card(CreateCardRequest {
                front: "Q2".to_string(),
                back: "A2".to_string(),
                category: None,
            })
            .unwrap();

        let result = service.delete_multiple_cards(vec![card1.id, card2.id]);
        assert!(result.is_ok());

        let remaining_cards = service.get_cards().unwrap();
        assert!(remaining_cards.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_review_difficulty_conversion() {
        // Test the u8 to ReviewDifficulty conversion used in review_card command
        assert!(matches!(ReviewDifficulty::from_u8(0), Ok(ReviewDifficulty::Again)));
        assert!(matches!(ReviewDifficulty::from_u8(1), Ok(ReviewDifficulty::Hard)));
        assert!(matches!(ReviewDifficulty::from_u8(2), Ok(ReviewDifficulty::Good)));
        assert!(matches!(ReviewDifficulty::from_u8(3), Ok(ReviewDifficulty::Easy)));
        assert!(ReviewDifficulty::from_u8(4).is_err());
    }
}
