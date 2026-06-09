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
    /// Delete an event by full ID.
    Delete { id: Uuid },
    /// Delete an event by ID prefix (short IDs).
    DeleteByPrefix { prefix: String },
    /// Update an event (after editing).
    Update(Event),
    /// List events matching a filter.
    List(ListFilter),
    /// Get summary status.
    Status,
    /// Generate a report.
    Report { period: ReportPeriod },
    /// List event candidates for shell completion (short id + label),
    /// most-recent first, optionally filtered by an id prefix.
    CompleteEvents {
        /// Optional short-id prefix to filter on.
        #[serde(default)]
        prefix: Option<String>,
        /// Maximum number of candidates to return.
        #[serde(default)]
        limit: Option<u32>,
    },
    /// List recently-used tags for shell completion, most-recent first.
    CompleteTags {
        /// Maximum number of tags to return.
        #[serde(default)]
        limit: Option<u32>,
    },
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
    /// Shell-completion event candidates (most-recent first).
    Completions(Vec<EventCompletion>),
    /// Shell-completion tag candidates (most-recent first).
    Tags(Vec<String>),
    /// Simple acknowledgment responses use wrapper structs to ensure unique
    /// serialization with `#[serde(untagged)]`. Unit variants would all
    /// serialize to `null` and become indistinguishable.
    Pong(PongMarker),
    Ack(AckMarker),
}

/// Marker struct for Pong response (serializes distinctly from Ack).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PongMarker {
    pong: bool,
}

impl Default for PongMarker {
    fn default() -> Self {
        Self { pong: true }
    }
}

/// Marker struct for Ack response (serializes distinctly from Pong).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AckMarker {
    ack: bool,
}

impl Default for AckMarker {
    fn default() -> Self {
        Self { ack: true }
    }
}

/// A single event candidate for shell completion.
///
/// `short_id` is the 8-character id prefix shown by `healthctl list`. The
/// `description` is a human-readable annotation (type, date and tags) used to
/// inform the user what kind of event the id refers to — analogous to how
/// `git show <TAB>` annotates commit hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCompletion {
    /// Short 8-char id prefix (the completion candidate itself).
    pub short_id: String,
    /// Human-readable description (e.g. "🏃 Run · Jun 06 07:30 · morning").
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListFilter {
    pub event_type: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
    pub limit: Option<u32>,
    /// If true, return most recent first; otherwise chronological (oldest first).
    #[serde(default)]
    pub reverse: bool,
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

/// A single named breakdown slice (e.g. ("walk", 20.0)), value semantics depend
/// on the card: count for workouts, kcal for calories, km for distance, minutes
/// for active time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakdown {
    pub label: String,
    pub value: f64,
}

/// One night of sleep for the sleep log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepNight {
    /// Formatted wake date, e.g. "Mon 06/09".
    pub date: String,
    pub hours: f64,
    pub quality: Option<i32>,
}

/// Rich report data mirroring the dashboard's stat cards and detail modals.
///
/// All six "cards" are always populated for the requested period. Comparisons
/// are against the immediately-preceding period of the same length. Projections
/// are only present when the period is the *current* (in-progress) one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub period: ReportPeriod,
    /// Number of days in the period (1/7/30/365), used for daily averages.
    pub period_days: i64,
    /// Human-readable date range, e.g. "May 31 – Jun 06".
    pub range_label: String,
    /// Whether this period is the current, in-progress one (enables projections).
    pub is_current: bool,
    /// Days elapsed so far in the period (== period_days unless current).
    pub days_elapsed: i64,

    pub total_events: u32,

    pub steps: StepsReport,
    pub calories: CaloriesReport,
    pub distance: DistanceReport,
    pub active: ActiveReport,
    pub sleep: SleepReport,
    pub workouts: WorkoutsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepsReport {
    pub total: f64,
    pub daily_avg: f64,
    pub best_day: Option<String>,
    pub best_day_value: f64,
    /// Percent change vs previous period.
    pub vs_previous: f64,
    /// Percent change vs the average of the longer baseline window.
    pub vs_average: f64,
    /// Projected total for the full period (current period only).
    pub projection: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaloriesReport {
    pub total: f64,
    pub by_activity: Vec<Breakdown>,
    pub daily_avg: f64,
    pub best_day: Option<String>,
    pub best_day_value: f64,
    pub vs_previous: f64,
    pub projection: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceReport {
    /// Total distance in km.
    pub total_km: f64,
    pub by_activity: Vec<Breakdown>,
    pub best_day: Option<String>,
    pub best_day_value: f64,
    pub vs_previous: f64,
    pub projection: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveReport {
    /// Total active minutes (excludes sleep).
    pub total_minutes: f64,
    pub by_activity: Vec<Breakdown>,
    pub daily_avg: f64,
    pub most_active_day: Option<String>,
    pub vs_previous: f64,
    pub projection: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepReport {
    /// Average hours per night.
    pub avg_hours: Option<f64>,
    pub nights: Vec<SleepNight>,
    pub best_night: Option<String>,
    pub best_night_hours: f64,
    pub worst_night: Option<String>,
    pub worst_night_hours: f64,
    pub avg_quality: f64,
    /// Percent change of average per-night hours vs previous period.
    pub vs_previous: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutsReport {
    pub count: u32,
    pub by_type: Vec<Breakdown>,
    /// Total workout duration in minutes.
    pub total_duration: f64,
    /// Average workout duration in minutes.
    pub avg_duration: f64,
    pub vs_previous: f64,
    pub muscle_groups: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_roundtrip() {
        let response = Response::Ok(ResponseData::Ack(AckMarker::default()));
        let json = serde_json::to_string(&response).unwrap();
        println!("Serialized Ack: {}", json);

        let parsed: Response = serde_json::from_str(&json).unwrap();
        println!("Parsed: {:?}", parsed);

        match parsed {
            Response::Ok(ResponseData::Ack(_)) => {}
            other => panic!("Expected Ack, got {:?}", other),
        }
    }

    #[test]
    fn test_pong_roundtrip() {
        let response = Response::Ok(ResponseData::Pong(PongMarker::default()));
        let json = serde_json::to_string(&response).unwrap();
        println!("Serialized Pong: {}", json);

        let parsed: Response = serde_json::from_str(&json).unwrap();
        println!("Parsed: {:?}", parsed);

        match parsed {
            Response::Ok(ResponseData::Pong(_)) => {}
            other => panic!("Expected Pong, got {:?}", other),
        }
    }

    #[test]
    fn test_ack_and_pong_are_distinct() {
        let ack = Response::Ok(ResponseData::Ack(AckMarker::default()));
        let pong = Response::Ok(ResponseData::Pong(PongMarker::default()));

        let ack_json = serde_json::to_string(&ack).unwrap();
        let pong_json = serde_json::to_string(&pong).unwrap();

        // They should serialize to different JSON
        assert_ne!(ack_json, pong_json);

        // Each should roundtrip correctly
        let parsed_ack: Response = serde_json::from_str(&ack_json).unwrap();
        let parsed_pong: Response = serde_json::from_str(&pong_json).unwrap();

        assert!(matches!(parsed_ack, Response::Ok(ResponseData::Ack(_))));
        assert!(matches!(parsed_pong, Response::Ok(ResponseData::Pong(_))));
    }
}
