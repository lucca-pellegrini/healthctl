use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::event::Event;

/// Request from healthctl client → daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Request {
    /// Add a new event.
    Add(Event),
    /// Clone an existing event with optional overrides (as partial JSON).
    Clone {
        source_id: Uuid,
        overrides: serde_json::Value,
    },
    /// Get an event by full ID.
    Get { id: Uuid },
    /// Get an event by ID prefix (short IDs).
    GetByPrefix { prefix: String },
    /// Update an event (after editing).
    Update(Event),
    /// List events matching a filter.
    List(ListFilter),
    /// Get summary status.
    Status,
    /// Generate a report.
    Report { period: ReportPeriod },
    /// Shutdown the daemon.
    Shutdown,
    /// Ping (health check).
    Ping,
}

/// Response from daemon → client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum Response {
    Ok(ResponseData),
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseData {
    Event(Event),
    Events(Vec<Event>),
    Summary(StatusSummary),
    Report(ReportData),
    Pong,
    Ack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilter {
    pub event_type: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPeriod {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSummary {
    pub today_events: u32,
    pub today_calories: f64,
    pub today_active_minutes: f64,
    pub week_events: u32,
    pub streak_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub period: ReportPeriod,
    pub total_events: u32,
    pub total_calories: f64,
    pub total_active_minutes: f64,
    pub avg_daily_calories: f64,
    pub avg_daily_active_minutes: f64,
}

/// Get the path to the IPC socket.
pub fn socket_path() -> std::path::PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("healthctl.sock")
    } else {
        // Fallback: /tmp
        std::path::PathBuf::from("/tmp").join(format!("healthctl-{}.sock", whoami()))
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".into())
}
