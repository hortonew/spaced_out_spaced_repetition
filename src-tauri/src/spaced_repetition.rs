use crate::models::{Card, ReviewDifficulty};
use chrono::{Duration, Utc};

/// SM-2 spaced repetition algorithm implementation
pub struct SpacedRepetition;

impl SpacedRepetition {
    /// Calculate next review parameters based on performance
    pub fn calculate_next_review(card: &Card, difficulty: &ReviewDifficulty) -> (i64, f64, chrono::DateTime<Utc>) {
        let mut new_interval = card.interval;
        let mut new_ease_factor = card.ease_factor;

        match difficulty {
            ReviewDifficulty::Again => {
                // Reset interval, reduce ease factor
                new_interval = 1;
                new_ease_factor = (card.ease_factor - 0.2).max(1.3);
            }
            ReviewDifficulty::Hard => {
                // Slightly increase interval, reduce ease factor
                new_interval = ((card.interval as f64) * 1.2).ceil() as i64;
                new_ease_factor = (card.ease_factor - 0.15).max(1.3);
            }
            ReviewDifficulty::Good => {
                // Normal progression
                if card.review_count == 0 {
                    new_interval = 1;
                } else if card.review_count == 1 {
                    new_interval = 6;
                } else {
                    new_interval = ((card.interval as f64) * card.ease_factor).ceil() as i64;
                }
            }
            ReviewDifficulty::Easy => {
                // Faster progression, increase ease factor
                if card.review_count == 0 {
                    new_interval = 4;
                } else if card.review_count == 1 {
                    new_interval = 6;
                } else {
                    new_interval = ((card.interval as f64) * card.ease_factor * 1.3).ceil() as i64;
                }
                new_ease_factor = card.ease_factor + 0.15;
            }
        }

        let next_review = Utc::now() + Duration::days(new_interval);
        (new_interval, new_ease_factor, next_review)
    }

    /// Check if a card is due for review
    pub fn is_due(card: &Card) -> bool {
        card.next_review <= Utc::now()
    }

    /// Get cards that are due for review
    pub fn get_due_cards(cards: &std::collections::HashMap<String, Card>) -> Vec<Card> {
        cards.values().filter(|card| Self::is_due(card)).cloned().collect()
    }

    /// Get cards that are due for review from a vector
    pub fn get_due_cards_from_vec(cards: &[Card]) -> Vec<Card> {
        cards.iter().filter(|card| Self::is_due(card)).cloned().collect()
    }

    /// Calculate review statistics
    pub fn calculate_stats(cards: &std::collections::HashMap<String, Card>) -> crate::models::ReviewStats {
        let total_cards = cards.len();
        let cards_due = cards.values().filter(|card| Self::is_due(card)).count();
        let cards_new = cards.values().filter(|card| card.review_count == 0).count();
        let cards_learning = cards.values().filter(|card| card.review_count > 0 && card.interval < 21).count();
        let cards_mature = cards.values().filter(|card| card.interval >= 21).count();

        crate::models::ReviewStats {
            total_cards,
            cards_due,
            cards_new,
            cards_learning,
            cards_mature,
        }
    }
}
