use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub front: String,
    pub back: String,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_reviewed: Option<DateTime<Utc>>,
    pub next_review: DateTime<Utc>,
    pub interval: i64,    // days
    pub ease_factor: f64, // SM-2 ease factor
    pub review_count: u32,
    pub correct_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum ReviewDifficulty {
    Again = 0, // Complete failure
    Hard = 1,  // Difficult recall
    Good = 2,  // Normal recall
    Easy = 3,  // Easy recall
}

impl ReviewDifficulty {
    pub fn from_u8(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(ReviewDifficulty::Again),
            1 => Ok(ReviewDifficulty::Hard),
            2 => Ok(ReviewDifficulty::Good),
            3 => Ok(ReviewDifficulty::Easy),
            _ => Err(format!("Invalid difficulty value: {}", value)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewStats {
    pub total_cards: usize,
    pub cards_due: usize,
    pub cards_new: usize,
    pub cards_learning: usize,
    pub cards_mature: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCardRequest {
    pub front: String,
    pub back: String,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCardRequest {
    pub front: String,
    pub back: String,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BulkUpdateRequest {
    pub card_ids: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryStats {
    pub name: String,
    pub total_cards: usize,
    pub cards_due: usize,
    pub cards_new: usize,
    pub cards_mature: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_review_difficulty_from_u8() {
        assert!(matches!(ReviewDifficulty::from_u8(0), Ok(ReviewDifficulty::Again)));
        assert!(matches!(ReviewDifficulty::from_u8(1), Ok(ReviewDifficulty::Hard)));
        assert!(matches!(ReviewDifficulty::from_u8(2), Ok(ReviewDifficulty::Good)));
        assert!(matches!(ReviewDifficulty::from_u8(3), Ok(ReviewDifficulty::Easy)));
        assert!(ReviewDifficulty::from_u8(4).is_err());
        assert!(ReviewDifficulty::from_u8(255).is_err());
    }

    #[test]
    fn test_review_difficulty_from_u8_error_message() {
        let result = ReviewDifficulty::from_u8(99);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid difficulty value: 99");
    }

    #[test]
    fn test_card_creation() {
        let card = Card {
            id: "test-id".to_string(),
            front: "What is 2+2?".to_string(),
            back: "4".to_string(),
            category: Some("Math".to_string()),
            created_at: Utc::now(),
            last_reviewed: None,
            next_review: Utc::now(),
            interval: 0,
            ease_factor: 2.5,
            review_count: 0,
            correct_count: 0,
        };

        assert_eq!(card.id, "test-id");
        assert_eq!(card.front, "What is 2+2?");
        assert_eq!(card.back, "4");
        assert_eq!(card.category, Some("Math".to_string()));
        assert_eq!(card.interval, 0);
        assert_eq!(card.ease_factor, 2.5);
        assert_eq!(card.review_count, 0);
        assert_eq!(card.correct_count, 0);
        assert!(card.last_reviewed.is_none());
    }

    #[test]
    fn test_card_serialization() {
        let card = Card {
            id: "test-id".to_string(),
            front: "Question".to_string(),
            back: "Answer".to_string(),
            category: None,
            created_at: Utc::now(),
            last_reviewed: None,
            next_review: Utc::now(),
            interval: 1,
            ease_factor: 2.5,
            review_count: 0,
            correct_count: 0,
        };

        let serialized = serde_json::to_string(&card).unwrap();
        let deserialized: Card = serde_json::from_str(&serialized).unwrap();

        assert_eq!(card.id, deserialized.id);
        assert_eq!(card.front, deserialized.front);
        assert_eq!(card.back, deserialized.back);
        assert_eq!(card.category, deserialized.category);
        assert_eq!(card.interval, deserialized.interval);
        assert_eq!(card.ease_factor, deserialized.ease_factor);
    }

    #[test]
    fn test_create_card_request() {
        let request = CreateCardRequest {
            front: "Question".to_string(),
            back: "Answer".to_string(),
            category: Some("Test".to_string()),
        };

        assert_eq!(request.front, "Question");
        assert_eq!(request.back, "Answer");
        assert_eq!(request.category, Some("Test".to_string()));
    }

    #[test]
    fn test_update_card_request() {
        let request = UpdateCardRequest {
            front: "Updated Question".to_string(),
            back: "Updated Answer".to_string(),
            category: None,
        };

        assert_eq!(request.front, "Updated Question");
        assert_eq!(request.back, "Updated Answer");
        assert_eq!(request.category, None);
    }

    #[test]
    fn test_search_request() {
        let request = SearchRequest {
            query: Some("test".to_string()),
            category: Some("Math".to_string()),
            tags: None,
        };

        assert_eq!(request.query, Some("test".to_string()));
        assert_eq!(request.category, Some("Math".to_string()));
        assert_eq!(request.tags, None);
    }

    #[test]
    fn test_bulk_update_request() {
        let request = BulkUpdateRequest {
            card_ids: vec!["id1".to_string(), "id2".to_string()],
            category: Some("New Category".to_string()),
        };

        assert_eq!(request.card_ids.len(), 2);
        assert_eq!(request.card_ids[0], "id1");
        assert_eq!(request.card_ids[1], "id2");
        assert_eq!(request.category, Some("New Category".to_string()));
    }

    #[test]
    fn test_category_stats() {
        let stats = CategoryStats {
            name: "Math".to_string(),
            total_cards: 10,
            cards_due: 3,
            cards_new: 2,
            cards_mature: 5,
        };

        assert_eq!(stats.name, "Math");
        assert_eq!(stats.total_cards, 10);
        assert_eq!(stats.cards_due, 3);
        assert_eq!(stats.cards_new, 2);
        assert_eq!(stats.cards_mature, 5);
    }

    #[test]
    fn test_review_stats() {
        let stats = ReviewStats {
            total_cards: 100,
            cards_due: 15,
            cards_new: 20,
            cards_learning: 30,
            cards_mature: 35,
        };

        assert_eq!(stats.total_cards, 100);
        assert_eq!(stats.cards_due, 15);
        assert_eq!(stats.cards_new, 20);
        assert_eq!(stats.cards_learning, 30);
        assert_eq!(stats.cards_mature, 35);
    }
}
