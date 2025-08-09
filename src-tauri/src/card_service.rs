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
