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
}
