use crate::models::{
    BulkUpdateRequest, Card, CategoryStats, CreateCardRequest, ReviewDifficulty, ReviewStats, SearchRequest, UpdateCardRequest,
};
use crate::spaced_repetition::SpacedRepetition;
use crate::storage::Storage;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

pub struct CardService {
    cards: Mutex<HashMap<String, Card>>,
    storage: Storage,
}

impl CardService {
    pub fn new(storage: Storage) -> Result<Self, Box<dyn std::error::Error>> {
        let cards = storage.load_cards()?;
        Ok(CardService {
            cards: Mutex::new(cards),
            storage,
        })
    }

    pub fn create_card(&self, request: CreateCardRequest) -> Result<Card, String> {
        let mut cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;

        let card = Card {
            id: Uuid::new_v4().to_string(),
            front: request.front,
            back: request.back,
            category: request.category,
            created_at: Utc::now(),
            last_reviewed: None,
            next_review: Utc::now(), // Available immediately for first review
            interval: 0,
            ease_factor: 2.5, // SM-2 default
            review_count: 0,
            correct_count: 0,
        };

        cards.insert(card.id.clone(), card.clone());
        self.save_cards(&cards)?;
        Ok(card)
    }

    pub fn get_cards(&self) -> Result<Vec<Card>, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        Ok(cards.values().cloned().collect())
    }

    pub fn get_card(&self, id: String) -> Result<Option<Card>, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        Ok(cards.get(&id).cloned())
    }

    pub fn update_card(&self, id: String, request: UpdateCardRequest) -> Result<Card, String> {
        let mut cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;

        if let Some(card) = cards.get_mut(&id) {
            card.front = request.front;
            card.back = request.back;
            card.category = request.category;

            let updated_card = card.clone();
            self.save_cards(&cards)?;
            Ok(updated_card)
        } else {
            Err("Card not found".to_string())
        }
    }

    pub fn delete_card(&self, id: String) -> Result<(), String> {
        let mut cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;

        if cards.remove(&id).is_some() {
            self.save_cards(&cards)?;
            Ok(())
        } else {
            Err("Card not found".to_string())
        }
    }

    pub fn get_due_cards(&self) -> Result<Vec<Card>, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        Ok(SpacedRepetition::get_due_cards(&cards))
    }

    pub fn review_card(&self, id: String, difficulty: ReviewDifficulty) -> Result<Card, String> {
        let mut cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;

        if let Some(card) = cards.get_mut(&id) {
            let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(card, &difficulty);

            card.last_reviewed = Some(Utc::now());
            card.next_review = next_review;
            card.interval = new_interval;
            card.ease_factor = new_ease_factor;
            card.review_count += 1;

            // Increment correct count for Good and Easy responses
            if matches!(difficulty, ReviewDifficulty::Good | ReviewDifficulty::Easy) {
                card.correct_count += 1;
            }

            let updated_card = card.clone();
            self.save_cards(&cards)?;
            Ok(updated_card)
        } else {
            Err("Card not found".to_string())
        }
    }

    pub fn get_review_stats(&self) -> Result<ReviewStats, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        Ok(SpacedRepetition::calculate_stats(&cards))
    }

    // Organization and search methods
    pub fn search_cards(&self, request: SearchRequest) -> Result<Vec<Card>, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        let mut filtered_cards: Vec<Card> = cards.values().cloned().collect();

        // Filter by query (searches front and back text)
        if let Some(query) = &request.query {
            let query_lower = query.to_lowercase();
            filtered_cards
                .retain(|card| card.front.to_lowercase().contains(&query_lower) || card.back.to_lowercase().contains(&query_lower));
        }

        // Filter by category
        if let Some(category) = &request.category {
            filtered_cards.retain(|card| card.category.as_ref().map_or(false, |c| c == category));
        }

        Ok(filtered_cards)
    }

    pub fn get_categories(&self) -> Result<Vec<String>, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        let mut categories: Vec<String> = cards
            .values()
            .filter_map(|card| card.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        categories.sort();
        Ok(categories)
    }

    pub fn get_category_stats(&self) -> Result<Vec<CategoryStats>, String> {
        let cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        let mut category_map: HashMap<String, Vec<Card>> = HashMap::new();

        // Group cards by category
        for card in cards.values() {
            let category = card.category.clone().unwrap_or_else(|| "Uncategorized".to_string());
            category_map.entry(category).or_insert_with(Vec::new).push(card.clone());
        }

        let mut stats: Vec<CategoryStats> = category_map
            .into_iter()
            .map(|(name, cards)| {
                let due_cards = SpacedRepetition::get_due_cards_from_vec(&cards);
                let new_cards = cards.iter().filter(|c| c.review_count == 0).count();
                let mature_cards = cards.iter().filter(|c| c.review_count >= 5).count();

                CategoryStats {
                    name,
                    total_cards: cards.len(),
                    cards_due: due_cards.len(),
                    cards_new: new_cards,
                    cards_mature: mature_cards,
                }
            })
            .collect();

        stats.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(stats)
    }

    pub fn bulk_update_category(&self, request: BulkUpdateRequest) -> Result<Vec<Card>, String> {
        let mut cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        let mut updated_cards = Vec::new();

        for card_id in &request.card_ids {
            if let Some(card) = cards.get_mut(card_id) {
                card.category = request.category.clone();
                updated_cards.push(card.clone());
            }
        }

        if !updated_cards.is_empty() {
            self.save_cards(&cards)?;
        }

        Ok(updated_cards)
    }

    pub fn delete_multiple_cards(&self, card_ids: Vec<String>) -> Result<(), String> {
        let mut cards = self.cards.lock().map_err(|_| "Failed to lock cards")?;
        let mut deleted_count = 0;

        for card_id in card_ids {
            if cards.remove(&card_id).is_some() {
                deleted_count += 1;
            }
        }

        if deleted_count > 0 {
            self.save_cards(&cards)?;
        }

        Ok(())
    }

    // Helper method to save cards
    fn save_cards(&self, cards: &HashMap<String, Card>) -> Result<(), String> {
        self.storage.save_cards(cards).map_err(|e| format!("Failed to save cards: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use serial_test::serial;
    use tempfile::TempDir;

    // Create a test storage instance
    fn create_test_storage() -> (Storage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_file = temp_dir.path().join("test_cards.json");
        let storage = Storage::new_with_path(data_file);
        (storage, temp_dir)
    }

    // Create a test card service
    fn create_test_service() -> (CardService, TempDir) {
        let (storage, temp_dir) = create_test_storage();
        let service = CardService::new(storage).unwrap();
        (service, temp_dir)
    }

    // Create test card request
    fn create_test_request(front: &str, back: &str, category: Option<&str>) -> CreateCardRequest {
        CreateCardRequest {
            front: front.to_string(),
            back: back.to_string(),
            category: category.map(|c| c.to_string()),
        }
    }

    #[test]
    #[serial]
    fn test_create_card() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("What is 2+2?", "4", Some("Math"));

        let result = service.create_card(request);
        assert!(result.is_ok());

        let card = result.unwrap();
        assert_eq!(card.front, "What is 2+2?");
        assert_eq!(card.back, "4");
        assert_eq!(card.category, Some("Math".to_string()));
        assert_eq!(card.review_count, 0);
        assert_eq!(card.correct_count, 0);
        assert_eq!(card.interval, 0);
        assert_eq!(card.ease_factor, 2.5);
        assert!(card.last_reviewed.is_none());
        assert!(!card.id.is_empty());
    }

    #[test]
    #[serial]
    fn test_create_card_no_category() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("Question", "Answer", None);

        let result = service.create_card(request);
        assert!(result.is_ok());

        let card = result.unwrap();
        assert_eq!(card.category, None);
    }

    #[test]
    #[serial]
    fn test_get_cards_empty() {
        let (service, _temp_dir) = create_test_service();
        let result = service.get_cards();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn test_get_cards_with_data() {
        let (service, _temp_dir) = create_test_service();

        // Create multiple cards
        let request1 = create_test_request("Q1", "A1", Some("Cat1"));
        let request2 = create_test_request("Q2", "A2", Some("Cat2"));

        let card1 = service.create_card(request1).unwrap();
        let card2 = service.create_card(request2).unwrap();

        let result = service.get_cards();
        assert!(result.is_ok());

        let cards = result.unwrap();
        assert_eq!(cards.len(), 2);

        let card_ids: Vec<String> = cards.iter().map(|c| c.id.clone()).collect();
        assert!(card_ids.contains(&card1.id));
        assert!(card_ids.contains(&card2.id));
    }

    #[test]
    #[serial]
    fn test_get_card_exists() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("Question", "Answer", None);
        let created_card = service.create_card(request).unwrap();

        let result = service.get_card(created_card.id.clone());
        assert!(result.is_ok());

        let retrieved_card = result.unwrap();
        assert!(retrieved_card.is_some());

        let card = retrieved_card.unwrap();
        assert_eq!(card.id, created_card.id);
        assert_eq!(card.front, "Question");
        assert_eq!(card.back, "Answer");
    }

    #[test]
    #[serial]
    fn test_get_card_not_exists() {
        let (service, _temp_dir) = create_test_service();
        let result = service.get_card("nonexistent-id".to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_update_card_success() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("Original", "Original Answer", Some("Original"));
        let created_card = service.create_card(request).unwrap();

        let update_request = UpdateCardRequest {
            front: "Updated Question".to_string(),
            back: "Updated Answer".to_string(),
            category: Some("Updated Category".to_string()),
        };

        let result = service.update_card(created_card.id.clone(), update_request);
        assert!(result.is_ok());

        let updated_card = result.unwrap();
        assert_eq!(updated_card.id, created_card.id);
        assert_eq!(updated_card.front, "Updated Question");
        assert_eq!(updated_card.back, "Updated Answer");
        assert_eq!(updated_card.category, Some("Updated Category".to_string()));

        // Verify persistence
        let retrieved_card = service.get_card(created_card.id).unwrap().unwrap();
        assert_eq!(retrieved_card.front, "Updated Question");
    }

    #[test]
    #[serial]
    fn test_update_card_not_found() {
        let (service, _temp_dir) = create_test_service();
        let update_request = UpdateCardRequest {
            front: "Updated".to_string(),
            back: "Updated".to_string(),
            category: None,
        };

        let result = service.update_card("nonexistent-id".to_string(), update_request);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Card not found");
    }

    #[test]
    #[serial]
    fn test_delete_card_success() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("To Delete", "Answer", None);
        let created_card = service.create_card(request).unwrap();

        let result = service.delete_card(created_card.id.clone());
        assert!(result.is_ok());

        // Verify card is deleted
        let retrieved = service.get_card(created_card.id);
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_delete_card_not_found() {
        let (service, _temp_dir) = create_test_service();
        let result = service.delete_card("nonexistent-id".to_string());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Card not found");
    }

    #[test]
    #[serial]
    fn test_get_due_cards() {
        let (service, _temp_dir) = create_test_service();

        // Create a card that's due (next_review in the past)
        let request = create_test_request("Due Card", "Answer", None);
        let card = service.create_card(request).unwrap();

        // The card should be due immediately (next_review is set to now for new cards)
        let due_cards = service.get_due_cards().unwrap();
        assert_eq!(due_cards.len(), 1);
        assert_eq!(due_cards[0].id, card.id);
    }

    #[test]
    #[serial]
    fn test_review_card_success() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("Review Test", "Answer", None);
        let created_card = service.create_card(request).unwrap();

        let result = service.review_card(created_card.id.clone(), ReviewDifficulty::Good);
        assert!(result.is_ok());

        let reviewed_card = result.unwrap();
        assert_eq!(reviewed_card.id, created_card.id);
        assert_eq!(reviewed_card.review_count, 1);
        assert_eq!(reviewed_card.correct_count, 1);
        assert_eq!(reviewed_card.interval, 1);
        assert!(reviewed_card.last_reviewed.is_some());
        assert!(reviewed_card.next_review > Utc::now());
    }

    #[test]
    #[serial]
    fn test_review_card_again() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_request("Review Test", "Answer", None);
        let created_card = service.create_card(request).unwrap();

        let result = service.review_card(created_card.id.clone(), ReviewDifficulty::Again);
        assert!(result.is_ok());

        let reviewed_card = result.unwrap();
        assert_eq!(reviewed_card.review_count, 1);
        assert_eq!(reviewed_card.correct_count, 0); // Not incremented for "Again"
        assert_eq!(reviewed_card.interval, 1);
    }

    #[test]
    #[serial]
    fn test_review_card_not_found() {
        let (service, _temp_dir) = create_test_service();
        let result = service.review_card("nonexistent-id".to_string(), ReviewDifficulty::Good);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Card not found");
    }

    #[test]
    #[serial]
    fn test_get_review_stats() {
        let (service, _temp_dir) = create_test_service();

        // Create various types of cards
        let _new_card = service.create_card(create_test_request("New", "Answer", None)).unwrap();

        // Create a reviewed card
        let reviewed_request = create_test_request("Reviewed", "Answer", None);
        let reviewed_card = service.create_card(reviewed_request).unwrap();
        service.review_card(reviewed_card.id, ReviewDifficulty::Good).unwrap();

        let stats = service.get_review_stats().unwrap();
        assert_eq!(stats.total_cards, 2);
        assert_eq!(stats.cards_due, 1); // Only the new card is due (reviewed card has future review date)
        assert_eq!(stats.cards_new, 1); // Only the unreviewed card
    }

    #[test]
    #[serial]
    fn test_search_cards_by_query() {
        let (service, _temp_dir) = create_test_service();

        service
            .create_card(create_test_request("Python programming", "A language", Some("Tech")))
            .unwrap();
        service
            .create_card(create_test_request("Java programming", "Another language", Some("Tech")))
            .unwrap();
        service
            .create_card(create_test_request("Math problem", "2+2=4", Some("Math")))
            .unwrap();

        let search_request = SearchRequest {
            query: Some("programming".to_string()),
            category: None,
            tags: None,
        };

        let results = service.search_cards(search_request).unwrap();
        assert_eq!(results.len(), 2);

        let fronts: Vec<String> = results.iter().map(|c| c.front.clone()).collect();
        assert!(fronts.contains(&"Python programming".to_string()));
        assert!(fronts.contains(&"Java programming".to_string()));
    }

    #[test]
    #[serial]
    fn test_search_cards_by_category() {
        let (service, _temp_dir) = create_test_service();

        service.create_card(create_test_request("Q1", "A1", Some("Math"))).unwrap();
        service.create_card(create_test_request("Q2", "A2", Some("Science"))).unwrap();
        service.create_card(create_test_request("Q3", "A3", Some("Math"))).unwrap();

        let search_request = SearchRequest {
            query: None,
            category: Some("Math".to_string()),
            tags: None,
        };

        let results = service.search_cards(search_request).unwrap();
        assert_eq!(results.len(), 2);

        for card in results {
            assert_eq!(card.category, Some("Math".to_string()));
        }
    }

    #[test]
    #[serial]
    fn test_search_cards_combined() {
        let (service, _temp_dir) = create_test_service();

        service
            .create_card(create_test_request("Math addition", "A1", Some("Math")))
            .unwrap();
        service
            .create_card(create_test_request("Math subtraction", "A2", Some("Math")))
            .unwrap();
        service
            .create_card(create_test_request("Science addition", "A3", Some("Science")))
            .unwrap();

        let search_request = SearchRequest {
            query: Some("addition".to_string()),
            category: Some("Math".to_string()),
            tags: None,
        };

        let results = service.search_cards(search_request).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].front, "Math addition");
    }

    #[test]
    #[serial]
    fn test_get_categories() {
        let (service, _temp_dir) = create_test_service();

        service.create_card(create_test_request("Q1", "A1", Some("Math"))).unwrap();
        service.create_card(create_test_request("Q2", "A2", Some("Science"))).unwrap();
        service.create_card(create_test_request("Q3", "A3", Some("Math"))).unwrap();
        service.create_card(create_test_request("Q4", "A4", None)).unwrap();

        let categories = service.get_categories().unwrap();
        assert_eq!(categories.len(), 2);
        assert!(categories.contains(&"Math".to_string()));
        assert!(categories.contains(&"Science".to_string()));
    }

    #[test]
    #[serial]
    fn test_get_category_stats() {
        let (service, _temp_dir) = create_test_service();

        // Create cards in different categories
        service.create_card(create_test_request("Q1", "A1", Some("Math"))).unwrap();
        service.create_card(create_test_request("Q2", "A2", Some("Math"))).unwrap();
        service.create_card(create_test_request("Q3", "A3", Some("Science"))).unwrap();
        service.create_card(create_test_request("Q4", "A4", None)).unwrap();

        let category_stats = service.get_category_stats().unwrap();
        assert_eq!(category_stats.len(), 3); // Math, Science, Uncategorized

        let math_stats = category_stats.iter().find(|s| s.name == "Math").unwrap();
        assert_eq!(math_stats.total_cards, 2);
        assert_eq!(math_stats.cards_new, 2);

        let science_stats = category_stats.iter().find(|s| s.name == "Science").unwrap();
        assert_eq!(science_stats.total_cards, 1);

        let uncategorized_stats = category_stats.iter().find(|s| s.name == "Uncategorized").unwrap();
        assert_eq!(uncategorized_stats.total_cards, 1);
    }

    #[test]
    #[serial]
    fn test_bulk_update_category() {
        let (service, _temp_dir) = create_test_service();

        let card1 = service.create_card(create_test_request("Q1", "A1", Some("Old"))).unwrap();
        let card2 = service.create_card(create_test_request("Q2", "A2", Some("Old"))).unwrap();
        let card3 = service.create_card(create_test_request("Q3", "A3", Some("Other"))).unwrap();

        let bulk_request = BulkUpdateRequest {
            card_ids: vec![card1.id.clone(), card2.id.clone()],
            category: Some("New Category".to_string()),
        };

        let result = service.bulk_update_category(bulk_request);
        assert!(result.is_ok());

        let updated_cards = result.unwrap();
        assert_eq!(updated_cards.len(), 2);

        // Verify updates persisted
        let retrieved_card1 = service.get_card(card1.id).unwrap().unwrap();
        let retrieved_card2 = service.get_card(card2.id).unwrap().unwrap();
        let retrieved_card3 = service.get_card(card3.id).unwrap().unwrap();

        assert_eq!(retrieved_card1.category, Some("New Category".to_string()));
        assert_eq!(retrieved_card2.category, Some("New Category".to_string()));
        assert_eq!(retrieved_card3.category, Some("Other".to_string())); // Unchanged
    }

    #[test]
    #[serial]
    fn test_bulk_update_category_nonexistent_cards() {
        let (service, _temp_dir) = create_test_service();

        let bulk_request = BulkUpdateRequest {
            card_ids: vec!["nonexistent-1".to_string(), "nonexistent-2".to_string()],
            category: Some("New Category".to_string()),
        };

        let result = service.bulk_update_category(bulk_request);
        assert!(result.is_ok());

        let updated_cards = result.unwrap();
        assert!(updated_cards.is_empty());
    }

    #[test]
    #[serial]
    fn test_delete_multiple_cards() {
        let (service, _temp_dir) = create_test_service();

        let card1 = service.create_card(create_test_request("Q1", "A1", None)).unwrap();
        let card2 = service.create_card(create_test_request("Q2", "A2", None)).unwrap();
        let card3 = service.create_card(create_test_request("Q3", "A3", None)).unwrap();

        let card_ids = vec![card1.id.clone(), card2.id.clone()];
        let result = service.delete_multiple_cards(card_ids);
        assert!(result.is_ok());

        // Verify deletions
        assert!(service.get_card(card1.id).unwrap().is_none());
        assert!(service.get_card(card2.id).unwrap().is_none());
        assert!(service.get_card(card3.id).unwrap().is_some()); // Should still exist

        let remaining_cards = service.get_cards().unwrap();
        assert_eq!(remaining_cards.len(), 1);
    }

    #[test]
    #[serial]
    fn test_delete_multiple_cards_partial_success() {
        let (service, _temp_dir) = create_test_service();

        let card1 = service.create_card(create_test_request("Q1", "A1", None)).unwrap();

        let card_ids = vec![card1.id.clone(), "nonexistent".to_string()];
        let result = service.delete_multiple_cards(card_ids);
        assert!(result.is_ok());

        // The existing card should be deleted
        assert!(service.get_card(card1.id).unwrap().is_none());
    }

    #[test]
    #[serial]
    fn test_persistence_across_instances() {
        let (storage, temp_dir) = create_test_storage();

        // Create service and add a card
        {
            let service = CardService::new(storage).unwrap();
            let request = create_test_request("Persistent", "Data", Some("Test"));
            service.create_card(request).unwrap();
        }

        // Create new storage instance pointing to same file
        let new_storage = Storage::new_with_path(temp_dir.path().join("test_cards.json"));
        let new_service = CardService::new(new_storage).unwrap();

        // Verify data persisted
        let cards = new_service.get_cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].front, "Persistent");
        assert_eq!(cards[0].back, "Data");
        assert_eq!(cards[0].category, Some("Test".to_string()));
    }
}
