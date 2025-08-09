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
