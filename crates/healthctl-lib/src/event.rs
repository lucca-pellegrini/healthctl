use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The core event type stored in the database.
/// All units are normalized to SI: kg, meters, seconds, kcal.
/// Only start_time and end_time are persisted; duration is always derived.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    /// Arbitrary key-value metrics. Values are always stored as f64 in SI units.
    pub metrics: HashMap<String, f64>,
    /// Freeform tags (deduplicated).
    pub tags: Vec<String>,
    /// Per-exercise breakdown for strength training.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exercises: Vec<Exercise>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Activity(ActivityKind),
    Strength,
    Sleep,
    Nutrition,
    Hydration,
    Substance,
    Mental(MentalKind),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Run,
    Walk,
    Cycle,
    Swim,
    Hike,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MentalKind {
    Meditation,
    Relaxation,
    Prayer,
    Journaling,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub name: String,
    pub sets: Option<u32>,
    pub reps: Option<u32>,
    /// Weight in kg.
    pub weight_kg: Option<f64>,
}

impl Event {
    pub fn new(event_type: EventType) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            start_time: None,
            end_time: None,
            metrics: HashMap::new(),
            tags: Vec::new(),
            exercises: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Compute duration in seconds from start and end times.
    /// Returns None if either time is missing.
    pub fn duration_secs(&self) -> Option<f64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => {
                let dur = (end - start).num_milliseconds() as f64 / 1000.0;
                if dur > 0.0 { Some(dur) } else { None }
            }
            _ => None,
        }
    }

    /// Resolve start and end times from whatever combination the user provided.
    /// Call this after setting start_time, end_time, and passing in the parsed duration.
    /// Logic:
    ///   - start + end → done (duration is derived)
    ///   - start + duration → end = start + duration
    ///   - end + duration → start = end - duration
    ///   - duration only → end = now, start = now - duration
    ///   - start only → fine (no end, no duration)
    ///   - end only → fine (no start, no duration)
    pub fn resolve_times(&mut self, duration_secs: Option<f64>) {
        match (self.start_time, self.end_time, duration_secs) {
            // Both times already set — nothing to do.
            (Some(_), Some(_), _) => {}
            // Start + duration → compute end.
            (Some(start), None, Some(dur)) => {
                self.end_time = Some(start + chrono::Duration::milliseconds((dur * 1000.0) as i64));
            }
            // End + duration → compute start.
            (None, Some(end), Some(dur)) => {
                self.start_time = Some(end - chrono::Duration::milliseconds((dur * 1000.0) as i64));
            }
            // Duration only → end = now, start = now - duration.
            (None, None, Some(dur)) => {
                let now = Utc::now();
                self.end_time = Some(now);
                self.start_time = Some(now - chrono::Duration::milliseconds((dur * 1000.0) as i64));
            }
            // No duration and at most one time — leave as-is.
            _ => {}
        }
    }

    /// Deduplicate tags.
    pub fn dedup_tags(&mut self) {
        self.tags.sort();
        self.tags.dedup();
    }

    /// The event's anchor time for ordering: prefer start, then end, then
    /// creation. Used by listings, reports and shell completion.
    pub fn anchor_time(&self) -> DateTime<Utc> {
        self.start_time.or(self.end_time).unwrap_or(self.created_at)
    }
}

impl EventType {
    /// An emoji glyph for this event type. Mirrors the CLI's pretty-printer but
    /// lives in the shared library so non-CLI consumers (e.g. the daemon's
    /// shell-completion endpoint) can annotate events without depending on the
    /// CLI crate.
    pub fn emoji(&self) -> &'static str {
        match self {
            EventType::Activity(kind) => match kind {
                ActivityKind::Run => "🏃",
                ActivityKind::Walk => "🚶",
                ActivityKind::Cycle => "🚴",
                ActivityKind::Swim => "🏊",
                ActivityKind::Hike => "🥾",
                ActivityKind::Other(_) => "🏋️",
            },
            EventType::Strength => "💪",
            EventType::Sleep => "😴",
            EventType::Nutrition => "🍽️",
            EventType::Hydration => "💧",
            EventType::Substance => "💊",
            EventType::Mental(kind) => match kind {
                MentalKind::Meditation => "🧘",
                MentalKind::Relaxation => "🌿",
                MentalKind::Prayer => "🙏",
                MentalKind::Journaling => "📝",
                MentalKind::Other(_) => "🧠",
            },
        }
    }

    /// A short plain-text label for this event type, e.g. "Run" or "Sleep".
    pub fn label(&self) -> String {
        match self {
            EventType::Activity(kind) => match kind {
                ActivityKind::Run => "Run".into(),
                ActivityKind::Walk => "Walk".into(),
                ActivityKind::Cycle => "Cycle".into(),
                ActivityKind::Swim => "Swim".into(),
                ActivityKind::Hike => "Hike".into(),
                ActivityKind::Other(s) => s.clone(),
            },
            EventType::Strength => "Strength".into(),
            EventType::Sleep => "Sleep".into(),
            EventType::Nutrition => "Nutrition".into(),
            EventType::Hydration => "Hydration".into(),
            EventType::Substance => "Substance".into(),
            EventType::Mental(kind) => match kind {
                MentalKind::Meditation => "Meditation".into(),
                MentalKind::Relaxation => "Relaxation".into(),
                MentalKind::Prayer => "Prayer".into(),
                MentalKind::Journaling => "Journaling".into(),
                MentalKind::Other(s) => s.clone(),
            },
        }
    }
}
