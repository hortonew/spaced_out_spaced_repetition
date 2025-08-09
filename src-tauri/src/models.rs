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
