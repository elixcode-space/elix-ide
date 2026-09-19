use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEvent {
    pub name: String,
    pub properties: Option<serde_json::Value>,
    pub timestamp: u64,
    pub session_id: String,
    pub version: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryMetrics {
    pub files_opened: u64,
    pub files_saved: u64,
    pub characters_typed: u64,
    pub commands_executed: u64,
    pub terminals_created: u64,
    pub terminal_commands_run: u64,
    pub git_operations: u64,
    pub git_commits: u64,
    pub git_pushes: u64,
    pub git_pulls: u64,
    pub extensions_installed: u64,
    pub extensions_activated: u64,
    pub ai_queries: u64,
    pub ai_completions_accepted: u64,
    pub ai_completions_rejected: u64,
    pub startup_time: u64,
    pub memory_usage: u64,
    pub cpu_usage: u64,
    pub errors: u64,
    pub crashes: u64,
}

#[command]
pub async fn telemetry_flush(
    events: Vec<TelemetryEvent>,
    metrics: TelemetryMetrics,
    session_id: String,
) -> Result<(), String> {
    // Log telemetry data (in production, this would be sent to an analytics service)
    // For now, we just log it locally
    log::info!(
        "Telemetry flush: session={}, events={}, metrics={:?}",
        session_id,
        events.len(),
        metrics
    );

    // In a real implementation, you would:
    // 1. Write to local log file for debugging
    // 2. Optionally send to analytics endpoint if configured
    // 3. Respect user privacy settings

    Ok(())
}