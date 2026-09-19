use crate::svm_service::{load_model, predict};
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::TcpListener;

// Unikernel-compatible core engine (simplified, no Tauri deps)
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::Mutex as TokioMutex;
use serde_json::Value;
    
use piscis_kernel::headless::{open_kernel_state, register_default_cli_tools, run_piscis_turn, HeadlessDeps, KernelState};
use piscis_core::host::{EventSink, ToolRegistryHandle, HeadlessCliRequest, HeadlessCliMode};
use piscis_kernel::agent::tool::ToolRegistry;

struct UniKernelState {
    kernel_state: Option<KernelState>,
    app_data_dir: Option<PathBuf>,
}

static UNIKERNEL_STATE: Lazy<TokioMutex<UniKernelState>> = Lazy::new(|| TokioMutex::new(UniKernelState {
    kernel_state: None,
    app_data_dir: None,
}));

fn create_tool_registry(db: &Arc<TokioMutex<piscis_kernel::store::db::Database>>, settings: &Arc<TokioMutex<piscis_kernel::store::settings::Settings>>) -> ToolRegistry {
    let mut tool_registry = ToolRegistry::new();
    let mut handle = ToolRegistryHandle::new(tool_registry);
    register_default_cli_tools(&mut handle, db.clone(), settings.clone());
    match handle.into_inner() {
        Ok(reg) => reg,
        Err(_) => ToolRegistry::new(),
    }
}

async fn initialize_unikernel() -> Result<String, String> {
    let mut state = UNIKERNEL_STATE.lock().await;

    if state.kernel_state.is_some() {
        return Ok("Core engine already initialized".to_string());
    }

    let app_data_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?
        .join("data");

    let kernel_state = open_kernel_state(&app_data_dir)
        .map_err(|e| format!("Failed to open kernel state: {}", e))?;

    state.kernel_state = Some(kernel_state);
    state.app_data_dir = Some(app_data_dir);

    Ok("Core engine initialized with piscis-engine".to_string())
}

async fn run_query_unikernel(query: String) -> Result<String, String> {
    let state = UNIKERNEL_STATE.lock().await;

    let kernel_state = match &state.kernel_state {
        Some(ks) => ks.clone(),
        _ => return Err("Core engine not initialized. Call initialize_core first.".to_string()),
    };

    let (db, settings) = &kernel_state;
    let tool_registry = create_tool_registry(db, settings);

    let session_id = format!("session-{}", uuid::Uuid::new_v4().simple());
    
    // Simple event sink that does nothing (for unikernel)
    struct NullEventSink;
    impl EventSink for NullEventSink {
        fn emit_session(&self, _session_id: &str, _event: &str, _payload: Value) {}
        fn emit_broadcast(&self, _event: &str, _payload: Value) {}
    }
    let event_sink = Arc::new(NullEventSink);

    let deps = HeadlessDeps::new(
        db.clone(),
        settings.clone(),
        tool_registry,
        event_sink,
    );

    let request = HeadlessCliRequest {
        prompt: query,
        workspace: state.app_data_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
        mode: HeadlessCliMode::Piscis,
        session_id: Some(session_id.clone()),
        ..Default::default()
    };

    let response = run_piscis_turn(request, deps)
        .await
        .map_err(|e| format!("Agent query failed: {}", e))?;

    Ok(response.response_text)
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Debug, Serialize)]
struct QueryResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct LoadModelRequest {
    name: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct LoadModelResponse {
    message: String,
}

#[derive(Debug, Deserialize)]
struct PredictRequest {
    model_name: String,
    features: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct PredictResponse {
    prediction: f64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Clone)]
struct AppState {
    initialized: Arc<Mutex<bool>>,
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

async fn init(State(state): State<AppState>) -> impl IntoResponse {
    let mut initialized = state.initialized.lock().await;
    if *initialized {
        return (StatusCode::OK, Json(serde_json::json!({"message": "Already initialized"}))).into_response();
    }

    match initialize_unikernel().await {
        Ok(msg) => {
            *initialized = true;
            (StatusCode::OK, Json(serde_json::json!({"message": msg}))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e })).into_response()
        }
    }
}

async fn query(State(state): State<AppState>, Json(req): Json<QueryRequest>) -> impl IntoResponse {
    let initialized = *state.initialized.lock().await;
    if !initialized {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse { error: "Not initialized".to_string() })).into_response();
    }

    match run_query_unikernel(req.query).await {
        Ok(response) => (StatusCode::OK, Json(QueryResponse { response })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e })).into_response(),
    }
}

async fn load_model_endpoint(State(state): State<AppState>, Json(req): Json<LoadModelRequest>) -> impl IntoResponse {
    let initialized = *state.initialized.lock().await;
    if !initialized {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse { error: "Not initialized".to_string() })).into_response();
    }

    match load_model(req.name, req.path) {
        Ok(message) => (StatusCode::OK, Json(LoadModelResponse { message })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e })).into_response(),
    }
}

async fn predict_endpoint(State(state): State<AppState>, Json(req): Json<PredictRequest>) -> impl IntoResponse {
    let initialized = *state.initialized.lock().await;
    if !initialized {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse { error: "Not initialized".to_string() })).into_response();
    }

    match predict(req.model_name, req.features) {
        Ok(prediction) => (StatusCode::OK, Json(PredictResponse { prediction })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e })).into_response(),
    }
}

async fn list_models() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"models": []}))).into_response()
}

// WebSocket streaming endpoint
async fn stream_query(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let initialized = *state.initialized.lock().await;
    if !initialized {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(ErrorResponse { error: "Not initialized".to_string() })).into_response();
    }

    let query = params.get("query").cloned().unwrap_or_default();
    ws.on_upgrade(move |socket| handle_stream_socket(socket, query))
}

async fn handle_stream_socket(mut socket: WebSocket, query: String) {
    // Send start event
    let _ = socket.send(Message::Text(
        serde_json::to_string(&serde_json::json!({
            "type": "start",
            "query": query
        })).unwrap()
    )).await;

    // Run the query and stream chunks
    match run_query_unikernel(query).await {
        Ok(response) => {
            // Simulate streaming by sending the response in chunks
            let chunks: Vec<&str> = response.split_whitespace().collect();
            for chunk in chunks {
                let _ = socket.send(Message::Text(
                    serde_json::to_string(&serde_json::json!({
                        "type": "chunk",
                        "content": chunk
                    })).unwrap()
                )).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            // Send end event
            let _ = socket.send(Message::Text(
                serde_json::to_string(&serde_json::json!({
                    "type": "end",
                    "response": response
                })).unwrap()
            )).await;
        }
        Err(e) => {
            let _ = socket.send(Message::Text(
                serde_json::to_string(&serde_json::json!({
                    "type": "error",
                    "error": e
                })).unwrap()
            )).await;
        }
    }
}

pub fn run_unikernel() -> ! {
    println!("ElixIDE Core Engine (Unikernel Mode) starting...");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        if let Err(e) = initialize_unikernel().await {
            eprintln!("Failed to initialize core engine: {}", e);
            std::process::exit(1);
        }

        println!("READY");

        let state = AppState {
            initialized: Arc::new(Mutex::new(true)),
        };

        let app = Router::new()
            .route("/health", get(health))
            .route("/init", post(init))
            .route("/query", post(query))
            .route("/model/load", post(load_model_endpoint))
            .route("/model/predict", post(predict_endpoint))
            .route("/models", get(list_models))
            .route("/stream", get(stream_query))
            .with_state(state);

        let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
        println!("HTTP server listening on 0.0.0.0:8080");
        axum::serve(listener, app).await.unwrap();
    });

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}