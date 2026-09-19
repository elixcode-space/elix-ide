use crate::core_engine::{initialize, run_query, stream_query};
use crate::svm_service::{load_model, predict};
use tauri::{command, AppHandle, Window};

#[command]
pub async fn initialize_core(app_handle: AppHandle) -> Result<String, String> {
    initialize(app_handle).await
}

#[command]
pub async fn run_agent_query(query: String, window: Window) -> Result<String, String> {
    run_query(query, window).await
}

#[command]
pub async fn start_agent_stream(query: String, window: Window) -> Result<(), String> {
    stream_query(query, window).await
}

#[command]
pub async fn load_svm_model(model_name: String, model_path: String) -> Result<String, String> {
    load_model(model_name, model_path)
}

#[command]
pub async fn run_svm_prediction(model_name: String, features: Vec<f64>) -> Result<f64, String> {
    predict(model_name, features)
}