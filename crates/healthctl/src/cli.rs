use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use healthctl_lib::event::{ActivityKind, Event, EventType, MentalKind};
use healthctl_lib::ipc::{ListFilter, ReportPeriod};
use healthctl_lib::parse::{parse_datetime, parse_duration};
use healthctl_lib::units::{parse_mass, parse_metric, parse_volume};
use healthctl_lib::validate::validate_event;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "healthctl", about = "Health metrics tracking CLI")]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Add a new health event.
    Add(AddCommand),
    /// Clone an existing event with overrides.
    Clone(CloneCommand),
    /// List events.
    List(ListCommand),
    /// Edit an event in $EDITOR.
    Edit {
        /// Event ID to edit.
        event_id: Uuid,
    },
    /// Show today's status summary.
    Status,
    /// Generate a report.
    Report {
        /// Period: day, week, month.
        #[arg(default_value = "week")]
        period: String,
    },
    /// Manage the daemon.
    Daemon(DaemonCommand),
}

#[derive(Parser)]
pub struct AddCommand {
    /// Event category: activity, strength, sleep, nutrition, hydration, substance, mental.
    pub category: String,
    /// Subtype (e.g. "run", "walk", "meditation", substance name).
    pub subtype: Option<String>,
    /// Positional amount (for hydration/substance: "500ml", "200mg").
    pub amount: Option<String>,

    /// Start time.
    #[arg(long)]
    pub start: Option<String>,
    /// End time.
    #[arg(long)]
    pub end: Option<String>,
    /// Duration (e.g. "45m", "1h15m", "00:45").
    #[arg(long)]
    pub duration: Option<String>,

    // Common metrics
    #[arg(long)]
    pub distance: Option<String>,
    #[arg(long)]
    pub elevation: Option<String>,
    #[arg(long)]
    pub calories: Option<String>,
    #[arg(long)]
    pub steps: Option<String>,
    #[arg(long)]
    pub sets: Option<String>,
    #[arg(long)]
    pub reps: Option<String>,
    #[arg(long)]
    pub weight: Option<String>,
    #[arg(long)]
    pub volume: Option<String>,

    // Nutrition-specific
    #[arg(long)]
    pub protein: Option<String>,
    #[arg(long)]
    pub carbs: Option<String>,
    #[arg(long)]
    pub fat: Option<String>,

    // Tags (repeatable).
    #[arg(long = "tag", num_args = 1)]
    pub tags: Vec<String>,

    // Exercises (repeatable): each --exercise starts a new exercise context.
    #[arg(long = "exercise", num_args = 1)]
    pub exercises: Vec<String>,
}

#[derive(Parser)]
pub struct CloneCommand {
    /// Event ID to clone.
    pub event_id: Uuid,

    #[arg(long)]
    pub start: Option<String>,
    #[arg(long)]
    pub end: Option<String>,
    #[arg(long)]
    pub duration: Option<String>,
    #[arg(long)]
    pub calories: Option<String>,

    #[arg(long = "tag", num_args = 1)]
    pub tags: Vec<String>,
}

#[derive(Parser)]
pub struct ListCommand {
    /// Event type filter (activity, sleep, nutrition, etc).
    pub event_type: Option<String>,

    #[arg(long)]
    pub day: Option<String>,
    #[arg(long)]
    pub week: bool,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,

    #[arg(long = "tag", num_args = 1)]
    pub tags: Vec<String>,
}

#[derive(Parser)]
pub struct DaemonCommand {
    #[arg(long)]
    pub stop: bool,
    #[arg(long)]
    pub restart: bool,
    #[arg(long)]
    pub status: bool,
}

pub fn parse() -> Args {
    Args::parse()
}

/// Build an Event from the AddCommand, parsing all fields.
pub fn build_event(cmd: AddCommand) -> Result<Event> {
    let event_type = parse_event_type(&cmd.category, cmd.subtype.as_deref())?;
    let mut event = Event::new(event_type.clone());

    // Parse time fields.
    if let Some(ref s) = cmd.start {
        event.start_time = Some(parse_datetime(s)?);
    }
    if let Some(ref s) = cmd.end {
        event.end_time = Some(parse_datetime(s)?);
    }
    if let Some(ref s) = cmd.duration {
        event.duration_secs = Some(parse_duration(s)?);
    }

    // If duration set but no start, and end is set → derive start.
    // If duration set but no end → end = now, derive start.
    if event.duration_secs.is_some() && event.end_time.is_none() && event.start_time.is_none() {
        event.end_time = Some(chrono::Utc::now());
    }
    event.derive_start();
    event.derive_duration();

    // Parse metrics.
    if let Some(ref v) = cmd.distance {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("distance_m".into(), val);
    }
    if let Some(ref v) = cmd.elevation {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("elevation_m".into(), val);
    }
    if let Some(ref v) = cmd.calories {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("calories_kcal".into(), val);
    }
    if let Some(ref v) = cmd.steps {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("steps".into(), val);
    }
    if let Some(ref v) = cmd.sets {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("sets".into(), val);
    }
    if let Some(ref v) = cmd.reps {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("reps".into(), val);
    }
    if let Some(ref v) = cmd.weight {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("weight_kg".into(), val);
    }
    if let Some(ref v) = cmd.volume {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("volume_kg".into(), val);
    }
    if let Some(ref v) = cmd.protein {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("protein_kg".into(), val);
    }
    if let Some(ref v) = cmd.carbs {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("carbs_kg".into(), val);
    }
    if let Some(ref v) = cmd.fat {
        let (val, _) = parse_metric(v)?;
        event.metrics.insert("fat_kg".into(), val);
    }

    // Handle positional amount for hydration/substance.
    match &event_type {
        EventType::Hydration => {
            if let Some(ref amt) = cmd.amount {
                let vol = parse_volume(amt)?;
                event.metrics.insert("volume_ml".into(), vol);
            }
        }
        EventType::Substance => {
            if let Some(ref amt) = cmd.amount {
                let mass = parse_mass(amt)?;
                event.metrics.insert("amount_kg".into(), mass);
            }
            // Store substance name as a tag.
            if let Some(ref sub) = cmd.subtype {
                event.tags.push(sub.clone());
            }
        }
        _ => {}
    }

    // Tags.
    event.tags.extend(cmd.tags);
    event.dedup_tags();

    // TODO: parse --exercise contexts properly (requires raw arg parsing).
    // For now, exercises are not supported via CLI flags (use edit).

    // Validate.
    validate_event(&event)?;

    Ok(event)
}

pub fn parse_report_period(s: &str) -> Result<ReportPeriod> {
    match s.to_lowercase().as_str() {
        "day" | "today" | "d" => Ok(ReportPeriod::Day),
        "week" | "w" => Ok(ReportPeriod::Week),
        "month" | "m" => Ok(ReportPeriod::Month),
        other => bail!("unknown report period: '{other}'. Valid: day, week, month"),
    }
}

fn parse_event_type(category: &str, subtype: Option<&str>) -> Result<EventType> {
    match category.to_lowercase().as_str() {
        "activity" => {
            let kind = match subtype {
                Some("run") => ActivityKind::Run,
                Some("walk") => ActivityKind::Walk,
                Some("cycle") | Some("cycling") | Some("bike") => ActivityKind::Cycle,
                Some("swim") | Some("swimming") => ActivityKind::Swim,
                Some("hike") | Some("hiking") => ActivityKind::Hike,
                Some(other) => ActivityKind::Other(other.to_string()),
                None => bail!("activity requires a subtype (e.g. 'run', 'walk', 'cycle')"),
            };
            Ok(EventType::Activity(kind))
        }
        "strength" => Ok(EventType::Strength),
        "sleep" => Ok(EventType::Sleep),
        "nutrition" | "food" | "meal" => Ok(EventType::Nutrition),
        "hydration" | "water" | "drink" => Ok(EventType::Hydration),
        "substance" | "supplement" => Ok(EventType::Substance),
        "mental" => {
            let kind = match subtype {
                Some("meditation") | Some("meditate") => MentalKind::Meditation,
                Some("relaxation") | Some("relax") => MentalKind::Relaxation,
                Some("prayer") | Some("pray") => MentalKind::Prayer,
                Some("journaling") | Some("journal") => MentalKind::Journaling,
                Some(other) => MentalKind::Other(other.to_string()),
                None => bail!("mental requires a subtype (e.g. 'meditation', 'relaxation')"),
            };
            Ok(EventType::Mental(kind))
        }
        other => bail!("unknown event category: '{other}'. Valid: activity, strength, sleep, nutrition, hydration, substance, mental"),
    }
}

/// Open an event in $EDITOR as TOML, return the parsed updated event.
pub fn edit_event(event: Event) -> Result<Event> {
    let toml_str = toml::to_string_pretty(&event)?;
    let tmp_path = std::env::temp_dir().join(format!("healthctl-{}.toml", event.id));
    std::fs::write(&tmp_path, &toml_str)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor)
        .arg(&tmp_path)
        .status()?;

    if !status.success() {
        bail!("editor exited with non-zero status");
    }

    let edited = std::fs::read_to_string(&tmp_path)?;
    std::fs::remove_file(&tmp_path).ok();

    let updated: Event = toml::from_str(&edited)?;
    validate_event(&updated)?;

    Ok(updated)
}

impl CloneCommand {
    pub fn to_overrides(&self) -> Result<serde_json::Value> {
        let mut map = serde_json::Map::new();

        if let Some(ref s) = self.start {
            let dt = parse_datetime(s)?;
            map.insert("start_time".into(), serde_json::to_value(dt)?);
        }
        if let Some(ref s) = self.end {
            let dt = parse_datetime(s)?;
            map.insert("end_time".into(), serde_json::to_value(dt)?);
        }
        if let Some(ref s) = self.duration {
            let secs = parse_duration(s)?;
            map.insert("duration_secs".into(), serde_json::Value::from(secs));
        }
        if let Some(ref v) = self.calories {
            let (val, _) = parse_metric(v)?;
            let mut metrics = serde_json::Map::new();
            metrics.insert("calories_kcal".into(), serde_json::Value::from(val));
            map.insert("metrics".into(), serde_json::Value::Object(metrics));
        }
        if !self.tags.is_empty() {
            map.insert("tags".into(), serde_json::to_value(&self.tags)?);
        }

        Ok(serde_json::Value::Object(map))
    }
}

impl ListCommand {
    pub fn to_filter(&self) -> Result<ListFilter> {
        let (from, to) = if self.week {
            let now = chrono::Utc::now();
            let week_ago = now - chrono::Duration::days(7);
            (Some(week_ago), Some(now))
        } else if let Some(ref day) = self.day {
            let (f, t) = healthctl_lib::parse::parse_date_range(day)?;
            (Some(f), Some(t))
        } else if let Some(ref from_str) = self.from {
            let (f, _) = healthctl_lib::parse::parse_date_range(from_str)?;
            let to = if let Some(ref to_str) = self.to {
                let (_, t) = healthctl_lib::parse::parse_date_range(to_str)?;
                Some(t)
            } else {
                Some(chrono::Utc::now())
            };
            (Some(f), to)
        } else {
            (None, None)
        };

        Ok(ListFilter {
            event_type: self.event_type.clone(),
            from,
            to,
            tags: self.tags.clone(),
            limit: self.limit,
        })
    }
}
