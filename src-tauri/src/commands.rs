use crate::card_service::CardService;
use crate::models::{Card, CreateCardRequest, ReviewDifficulty, ReviewStats, UpdateCardRequest};
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
