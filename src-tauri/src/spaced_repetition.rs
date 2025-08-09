use crate::models::{Card, ReviewDifficulty};
use chrono::{Duration, Utc};

/// SM-2 spaced repetition algorithm implementation
pub struct SpacedRepetition;

impl SpacedRepetition {
    /// Calculate next review parameters based on performance
    pub fn calculate_next_review(card: &Card, difficulty: &ReviewDifficulty) -> (i64, f64, chrono::DateTime<Utc>) {
        let new_interval;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Card, ReviewDifficulty};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    fn create_test_card(id: &str, review_count: u32, interval: i64, ease_factor: f64) -> Card {
        Card {
            id: id.to_string(),
            front: format!("Question {}", id),
            back: format!("Answer {}", id),
            category: None,
            created_at: Utc::now(),
            last_reviewed: if review_count > 0 { Some(Utc::now()) } else { None },
            next_review: Utc::now() + Duration::days(interval),
            interval,
            ease_factor,
            review_count,
            correct_count: review_count / 2, // Assume half correct
        }
    }

    fn create_due_card(id: &str) -> Card {
        Card {
            id: id.to_string(),
            front: format!("Question {}", id),
            back: format!("Answer {}", id),
            category: None,
            created_at: Utc::now(),
            last_reviewed: Some(Utc::now() - Duration::days(1)),
            next_review: Utc::now() - Duration::hours(1), // Due 1 hour ago
            interval: 1,
            ease_factor: 2.5,
            review_count: 1,
            correct_count: 0,
        }
    }

    #[test]
    fn test_calculate_next_review_again() {
        let card = create_test_card("1", 5, 10, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Again);

        assert_eq!(new_interval, 1);
        assert_eq!(new_ease_factor, 2.3); // 2.5 - 0.2
        assert!(next_review > Utc::now());
        assert!(next_review <= Utc::now() + Duration::days(1) + Duration::seconds(1));
    }

    #[test]
    fn test_calculate_next_review_hard() {
        let card = create_test_card("1", 5, 10, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Hard);

        assert_eq!(new_interval, 12); // ceil(10 * 1.2)
        assert_eq!(new_ease_factor, 2.35); // 2.5 - 0.15
        assert!(next_review > Utc::now() + Duration::days(11));
        assert!(next_review <= Utc::now() + Duration::days(12) + Duration::seconds(1));
    }

    #[test]
    fn test_calculate_next_review_good_new_card() {
        let card = create_test_card("1", 0, 0, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Good);

        assert_eq!(new_interval, 1);
        assert_eq!(new_ease_factor, 2.5);
        assert!(next_review > Utc::now());
        assert!(next_review <= Utc::now() + Duration::days(1) + Duration::seconds(1));
    }

    #[test]
    fn test_calculate_next_review_good_second_review() {
        let card = create_test_card("1", 1, 1, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Good);

        assert_eq!(new_interval, 6);
        assert_eq!(new_ease_factor, 2.5);
        assert!(next_review > Utc::now() + Duration::days(5));
        assert!(next_review <= Utc::now() + Duration::days(6) + Duration::seconds(1));
    }

    #[test]
    fn test_calculate_next_review_good_mature_card() {
        let card = create_test_card("1", 5, 10, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Good);

        assert_eq!(new_interval, 25); // ceil(10 * 2.5)
        assert_eq!(new_ease_factor, 2.5);
        assert!(next_review > Utc::now() + Duration::days(24));
        assert!(next_review <= Utc::now() + Duration::days(25) + Duration::seconds(1));
    }

    #[test]
    fn test_calculate_next_review_easy_new_card() {
        let card = create_test_card("1", 0, 0, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Easy);

        assert_eq!(new_interval, 4);
        assert_eq!(new_ease_factor, 2.65); // 2.5 + 0.15
        assert!(next_review > Utc::now() + Duration::days(3));
        assert!(next_review <= Utc::now() + Duration::days(4) + Duration::seconds(1));
    }

    #[test]
    fn test_calculate_next_review_easy_mature_card() {
        let card = create_test_card("1", 5, 10, 2.5);
        let (new_interval, new_ease_factor, next_review) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Easy);

        assert_eq!(new_interval, 33); // ceil(10 * 2.5 * 1.3)
        assert_eq!(new_ease_factor, 2.65); // 2.5 + 0.15
        assert!(next_review > Utc::now() + Duration::days(32));
        assert!(next_review <= Utc::now() + Duration::days(33) + Duration::seconds(1));
    }

    #[test]
    fn test_ease_factor_minimum() {
        let mut card = create_test_card("1", 5, 10, 1.3); // Already at minimum
        let (_, new_ease_factor, _) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Again);

        assert_eq!(new_ease_factor, 1.3); // Should not go below 1.3

        // Multiple "Again" responses should not decrease below 1.3
        card.ease_factor = 1.4;
        let (_, new_ease_factor, _) = SpacedRepetition::calculate_next_review(&card, &ReviewDifficulty::Again);
        assert_eq!(new_ease_factor, 1.3);
    }

    #[test]
    fn test_is_due() {
        let due_card = create_due_card("1");
        assert!(SpacedRepetition::is_due(&due_card));

        let future_card = create_test_card("2", 1, 5, 2.5);
        assert!(!SpacedRepetition::is_due(&future_card));

        let now_card = Card {
            id: "3".to_string(),
            front: "Question".to_string(),
            back: "Answer".to_string(),
            category: None,
            created_at: Utc::now(),
            last_reviewed: None,
            next_review: Utc::now(),
            interval: 0,
            ease_factor: 2.5,
            review_count: 0,
            correct_count: 0,
        };
        assert!(SpacedRepetition::is_due(&now_card));
    }

    #[test]
    fn test_get_due_cards() {
        let mut cards = HashMap::new();

        let due_card = create_due_card("1");
        let future_card = create_test_card("2", 1, 5, 2.5);
        let another_due_card = create_due_card("3");

        cards.insert("1".to_string(), due_card);
        cards.insert("2".to_string(), future_card);
        cards.insert("3".to_string(), another_due_card);

        let due_cards = SpacedRepetition::get_due_cards(&cards);
        assert_eq!(due_cards.len(), 2);

        let due_ids: Vec<String> = due_cards.iter().map(|c| c.id.clone()).collect();
        assert!(due_ids.contains(&"1".to_string()));
        assert!(due_ids.contains(&"3".to_string()));
        assert!(!due_ids.contains(&"2".to_string()));
    }

    #[test]
    fn test_get_due_cards_from_vec() {
        let due_card = create_due_card("1");
        let future_card = create_test_card("2", 1, 5, 2.5);
        let another_due_card = create_due_card("3");

        let cards = vec![due_card, future_card, another_due_card];
        let due_cards = SpacedRepetition::get_due_cards_from_vec(&cards);

        assert_eq!(due_cards.len(), 2);
        let due_ids: Vec<String> = due_cards.iter().map(|c| c.id.clone()).collect();
        assert!(due_ids.contains(&"1".to_string()));
        assert!(due_ids.contains(&"3".to_string()));
        assert!(!due_ids.contains(&"2".to_string()));
    }

    #[test]
    fn test_calculate_stats() {
        let mut cards = HashMap::new();

        // New card (review_count = 0)
        cards.insert("1".to_string(), create_test_card("1", 0, 0, 2.5));

        // Learning card (review_count > 0, interval < 21)
        cards.insert("2".to_string(), create_test_card("2", 3, 10, 2.5));

        // Mature card (interval >= 21)
        cards.insert("3".to_string(), create_test_card("3", 8, 25, 2.5));

        // Due card
        cards.insert("4".to_string(), create_due_card("4"));

        let stats = SpacedRepetition::calculate_stats(&cards);

        assert_eq!(stats.total_cards, 4);
        assert_eq!(stats.cards_due, 2); // Cards "1" (new but due) and "4" (due)
        assert_eq!(stats.cards_new, 1); // Card "1"
        assert_eq!(stats.cards_learning, 2); // Cards "2" and "4"
        assert_eq!(stats.cards_mature, 1); // Card "3"
    }

    #[test]
    fn test_calculate_stats_empty() {
        let cards = HashMap::new();
        let stats = SpacedRepetition::calculate_stats(&cards);

        assert_eq!(stats.total_cards, 0);
        assert_eq!(stats.cards_due, 0);
        assert_eq!(stats.cards_new, 0);
        assert_eq!(stats.cards_learning, 0);
        assert_eq!(stats.cards_mature, 0);
    }
}
