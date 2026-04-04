use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The core event type stored in the database.
/// All units are normalized to SI: kg, meters, seconds, kcal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    /// Duration in seconds (derived or explicit).
    pub duration_secs: Option<f64>,
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
            duration_secs: None,
            metrics: HashMap::new(),
            tags: Vec::new(),
            exercises: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Derive duration from start/end if not explicitly set.
    pub fn derive_duration(&mut self) {
        if self.duration_secs.is_none() {
            if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
                let dur = end - start;
                if dur.num_seconds() > 0 {
                    self.duration_secs = Some(dur.num_seconds() as f64);
                }
            }
        }
    }

    /// If duration is set and end_time is set but start_time is not, derive start.
    pub fn derive_start(&mut self) {
        if self.start_time.is_none() {
            if let (Some(end), Some(dur)) = (self.end_time, self.duration_secs) {
                self.start_time = Some(end - chrono::Duration::milliseconds((dur * 1000.0) as i64));
            }
        }
    }

    /// Deduplicate tags.
    pub fn dedup_tags(&mut self) {
        self.tags.sort();
        self.tags.dedup();
    }
}
