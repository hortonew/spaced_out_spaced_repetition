use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

// Data structures for spaced repetition
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
    pub id: String,
    pub front: String,
    pub back: String,
    pub category: Option<String>,
}

// Application state
pub struct AppState {
    pub cards: Mutex<HashMap<String, Card>>,
    pub data_file: PathBuf,
}

impl AppState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = dirs::home_dir()
            .ok_or("Unable to find home directory")?
            .join(".spaced_repetition_app");

        std::fs::create_dir_all(&data_dir)?;
        let data_file = data_dir.join("cards.json");

        let cards = if data_file.exists() {
            let file = File::open(&data_file)?;
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap_or_default()
        } else {
            HashMap::new()
        };

        Ok(AppState {
            cards: Mutex::new(cards),
            data_file,
        })
    }

    pub fn save_cards(&self) -> Result<(), Box<dyn std::error::Error>> {
        let cards = self.cards.lock().unwrap();
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.data_file)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*cards)?;
        Ok(())
    }
}

impl Card {
    pub fn new(front: String, back: String, category: Option<String>) -> Self {
        let now = Utc::now();
        Card {
            id: Uuid::new_v4().to_string(),
            front,
            back,
            category,
            created_at: now,
            last_reviewed: None,
            next_review: now, // New cards are due immediately
            interval: 0,
            ease_factor: 2.5, // SM-2 default ease factor
            review_count: 0,
            correct_count: 0,
        }
    }

    pub fn review(&mut self, difficulty: ReviewDifficulty) {
        let now = Utc::now();
        self.last_reviewed = Some(now);
        self.review_count += 1;

        match difficulty {
            ReviewDifficulty::Again => {
                // Reset the card - start over
                self.interval = 0;
                self.ease_factor = (self.ease_factor - 0.2).max(1.3);
                self.next_review = now + Duration::minutes(1); // Review again in 1 minute
            }
            ReviewDifficulty::Hard => {
                self.correct_count += 1;
                self.ease_factor = (self.ease_factor - 0.15).max(1.3);

                if self.interval == 0 {
                    self.interval = 1;
                } else if self.interval == 1 {
                    self.interval = 4;
                } else {
                    self.interval = ((self.interval as f64) * self.ease_factor * 1.2) as i64;
                }

                self.next_review = now + Duration::days(self.interval);
            }
            ReviewDifficulty::Good => {
                self.correct_count += 1;

                if self.interval == 0 {
                    self.interval = 1;
                } else if self.interval == 1 {
                    self.interval = 6;
                } else {
                    self.interval = ((self.interval as f64) * self.ease_factor) as i64;
                }

                self.next_review = now + Duration::days(self.interval);
            }
            ReviewDifficulty::Easy => {
                self.correct_count += 1;
                self.ease_factor = (self.ease_factor + 0.15).min(5.0);

                if self.interval == 0 {
                    self.interval = 4;
                } else if self.interval == 1 {
                    self.interval = 6;
                } else {
                    self.interval = ((self.interval as f64) * self.ease_factor * 1.3) as i64;
                }

                self.next_review = now + Duration::days(self.interval);
            }
        }
    }

    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_review
    }
}

// Tauri commands
#[tauri::command]
pub fn create_card(state: State<AppState>, request: CreateCardRequest) -> Result<Card, String> {
    let card = Card::new(request.front, request.back, request.category);
    let card_id = card.id.clone();

    {
        let mut cards = state.cards.lock().unwrap();
        cards.insert(card_id, card.clone());
    }

    state.save_cards().map_err(|e| e.to_string())?;
    Ok(card)
}

#[tauri::command]
pub fn get_cards(state: State<AppState>) -> Result<Vec<Card>, String> {
    let cards = state.cards.lock().unwrap();
    Ok(cards.values().cloned().collect())
}

#[tauri::command]
pub fn get_card(state: State<AppState>, id: String) -> Result<Option<Card>, String> {
    let cards = state.cards.lock().unwrap();
    Ok(cards.get(&id).cloned())
}

#[tauri::command]
pub fn update_card(state: State<AppState>, request: UpdateCardRequest) -> Result<Card, String> {
    let mut cards = state.cards.lock().unwrap();

    let card = cards.get_mut(&request.id).ok_or("Card not found")?;

    card.front = request.front;
    card.back = request.back;
    card.category = request.category;

    let updated_card = card.clone();
    drop(cards);

    state.save_cards().map_err(|e| e.to_string())?;
    Ok(updated_card)
}

#[tauri::command]
pub fn delete_card(state: State<AppState>, id: String) -> Result<(), String> {
    {
        let mut cards = state.cards.lock().unwrap();
        cards.remove(&id).ok_or("Card not found")?;
    }

    state.save_cards().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_due_cards(state: State<AppState>) -> Result<Vec<Card>, String> {
    let cards = state.cards.lock().unwrap();
    let due_cards: Vec<Card> = cards
        .values()
        .filter(|card| card.is_due())
        .cloned()
        .collect();
    Ok(due_cards)
}

#[tauri::command]
pub fn review_card(state: State<AppState>, id: String, difficulty: u8) -> Result<Card, String> {
    let difficulty = ReviewDifficulty::from_u8(difficulty)?;

    let mut cards = state.cards.lock().unwrap();

    let card = cards.get_mut(&id).ok_or("Card not found")?;

    card.review(difficulty);
    let updated_card = card.clone();
    drop(cards);

    state.save_cards().map_err(|e| e.to_string())?;
    Ok(updated_card)
}

#[tauri::command]
pub fn get_review_stats(state: State<AppState>) -> Result<ReviewStats, String> {
    let cards = state.cards.lock().unwrap();
    let _now = Utc::now();

    let total_cards = cards.len();
    let cards_due = cards.values().filter(|card| card.is_due()).count();
    let cards_new = cards.values().filter(|card| card.review_count == 0).count();
    let cards_learning = cards
        .values()
        .filter(|card| card.review_count > 0 && card.interval < 21)
        .count();
    let cards_mature = cards.values().filter(|card| card.interval >= 21).count();

    Ok(ReviewStats {
        total_cards,
        cards_due,
        cards_new,
        cards_learning,
        cards_mature,
    })
}

// Legacy commands for demo purposes
#[tauri::command]
pub fn say_hello(name: String) -> String {
    format!("Hello, {name} 👋 (from Rust)")
}

#[tauri::command]
pub fn my_custom_command() {
    println!("I was invoked from JavaScript!");
}
