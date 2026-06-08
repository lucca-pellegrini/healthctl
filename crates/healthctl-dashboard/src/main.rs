// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use healthctl_lib::event::{Event, EventType};
use healthctl_lib::ipc::{Request, Response, ResponseData};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::State;

/// A simplified event for the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivityItem {
    id: String,
    event_type: String,
    subtype: Option<String>,
    start_time: String,
    duration_mins: Option<i64>,
    metrics: std::collections::HashMap<String, f64>,
    tags: Vec<String>,
}

/// Weekly summary (Sunday to Saturday)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeeklySummary {
    total_steps: i32,
    total_calories: f64,
    total_distance_km: f64,
    total_active_mins: i32,
    avg_sleep_hours: Option<f64>,
    workout_count: i32,
    week_start: String,
    week_end: String,
}

/// Weekly stats for the chart
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeeklyStats {
    // Each entry is [date_label, steps, calories, active_mins]
    days: Vec<DayStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DayStats {
    date: String,
    day_name: String,
    steps: i32,
    calories: f64,
    distance_km: f64,
    active_mins: i32,
    sleep_hours: f64,
    workouts: i32,
}

/// Detailed card stats for flip cards
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CardDetails {
    steps: StepsDetails,
    calories: CaloriesDetails,
    distance: DistanceDetails,
    active: ActiveDetails,
    sleep: SleepDetails,
    workouts: WorkoutsDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StepsDetails {
    best_day: String,
    best_day_steps: i32,
    daily_avg: i32,
    vs_last_week: f64,       // percentage change
    vs_last_month: f64,      // percentage change
    projection: Option<i32>, // projected week total (current week only)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaloriesDetails {
    by_activity: Vec<(String, f64)>, // (activity type, calories)
    daily_avg: f64,
    best_day: String,
    best_day_calories: f64,
    vs_last_week: f64,
    projection: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DistanceDetails {
    by_activity: Vec<(String, f64)>, // (activity type, km)
    best_day: String,
    best_day_km: f64,
    total_elevation: f64,
    vs_last_week: f64,
    projection: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveDetails {
    by_activity: Vec<(String, i32)>, // (activity type, minutes)
    daily_avg: i32,
    most_active_day: String,
    vs_last_week: f64,
    projection: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SleepDetails {
    nights: Vec<SleepNight>,
    best_night: String,
    best_night_hours: f64,
    worst_night: String,
    worst_night_hours: f64,
    avg_quality: f64,
    vs_last_week: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SleepNight {
    date: String,
    hours: f64,
    quality: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkoutsDetails {
    by_type: Vec<(String, i32)>, // (workout type, count)
    total_duration: i32,
    avg_duration: i32,
    vs_last_week: f64,
    muscle_groups: Vec<String>,
}

/// Full dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DashboardData {
    recent_activities: Vec<ActivityItem>,
    week_summary: WeeklySummary,
    weekly: WeeklyStats,
    card_details: CardDetails,
    is_current_week: bool,
    connection_status: String,
    streak_days: u32,
}

struct AppState {}

fn format_event_type(event_type: &EventType) -> (String, Option<String>) {
    match event_type {
        EventType::Activity(kind) => {
            let subtype = format!("{:?}", kind).to_lowercase();
            ("activity".to_string(), Some(subtype))
        }
        EventType::Strength => ("strength".to_string(), None),
        EventType::Sleep => ("sleep".to_string(), None),
        EventType::Nutrition => ("nutrition".to_string(), None),
        EventType::Hydration => ("hydration".to_string(), None),
        EventType::Substance => ("substance".to_string(), None),
        EventType::Mental(kind) => {
            let subtype = format!("{:?}", kind).to_lowercase();
            ("mental".to_string(), Some(subtype))
        }
    }
}

fn connect_to_daemon() -> Result<std::os::unix::net::UnixStream, String> {
    use std::os::unix::net::UnixStream;

    let socket_path = healthctl_lib::ipc::socket_path();

    match UnixStream::connect(&socket_path) {
        Ok(s) => Ok(s),
        Err(_) => {
            // Try to start the daemon
            eprintln!("Daemon not running, attempting to start...");
            let daemon_path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("healthctl-daemon")))
                .unwrap_or_else(|| std::path::PathBuf::from("healthctl-daemon"));

            if let Err(e) = Command::new(&daemon_path).spawn() {
                eprintln!("Failed to start daemon: {}", e);
                let _ = Command::new("healthctl").args(["daemon"]).spawn();
            }

            std::thread::sleep(std::time::Duration::from_secs(2));

            UnixStream::connect(&socket_path)
                .map_err(|e| format!("Failed to connect to daemon: {}", e))
        }
    }
}

fn send_request(request: &Request) -> Result<Response, String> {
    use std::io::{Read, Write};

    let mut stream = connect_to_daemon()?;

    let request_json = serde_json::to_string(request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;

    stream
        .write_all(request_json.as_bytes())
        .map_err(|e| format!("Failed to send request: {}", e))?;
    stream
        .write_all(b"\n")
        .map_err(|e| format!("Failed to send newline: {}", e))?;

    let mut buffer = String::new();
    let mut byte_buffer = [0u8; 65536]; // Larger buffer for more events
    loop {
        match stream.read(&mut byte_buffer) {
            Ok(0) => break,
            Ok(n) => {
                buffer.push_str(&String::from_utf8_lossy(&byte_buffer[..n]));
                if buffer.contains('\n') {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(format!("Failed to read response: {}", e)),
        }
    }

    serde_json::from_str(buffer.trim()).map_err(|e| format!("Failed to parse response: {}", e))
}

fn event_to_activity_item(event: &Event) -> ActivityItem {
    let (event_type, subtype) = format_event_type(&event.event_type);

    let duration_mins = match (event.start_time, event.end_time) {
        (Some(s), Some(e)) => Some((e - s).num_minutes()),
        _ => None,
    };

    let start_time = event
        .start_time
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".to_string());

    ActivityItem {
        id: event.id.to_string()[..8].to_string(),
        event_type,
        subtype,
        start_time,
        duration_mins,
        metrics: event.metrics.clone(),
        tags: event.tags.clone(),
    }
}

fn get_calories(event: &Event) -> f64 {
    event
        .metrics
        .get("calories")
        .or_else(|| event.metrics.get("calories_kcal"))
        .copied()
        .unwrap_or(0.0)
}

fn get_distance_km(event: &Event) -> f64 {
    if let Some(dist) = event.metrics.get("distance_m") {
        dist / 1000.0
    } else if let Some(dist) = event.metrics.get("distance") {
        *dist
    } else {
        0.0
    }
}

fn get_steps(event: &Event) -> i32 {
    event.metrics.get("steps").copied().unwrap_or(0.0) as i32
}

fn get_duration_mins(event: &Event) -> i32 {
    match (event.start_time, event.end_time) {
        (Some(s), Some(e)) => (e - s).num_minutes() as i32,
        _ => 0,
    }
}

fn is_workout(event: &Event) -> bool {
    matches!(
        event.event_type,
        EventType::Strength | EventType::Activity(_)
    ) && event.end_time.is_some()
}

fn get_event_activity_type(event: &Event) -> String {
    match &event.event_type {
        EventType::Activity(kind) => format!("{:?}", kind).to_lowercase(),
        EventType::Strength => "strength".to_string(),
        EventType::Sleep => "sleep".to_string(),
        EventType::Nutrition => "nutrition".to_string(),
        EventType::Hydration => "hydration".to_string(),
        EventType::Substance => "substance".to_string(),
        EventType::Mental(kind) => format!("{:?}", kind).to_lowercase(),
    }
}

fn get_sleep_quality(event: &Event) -> Option<i32> {
    for tag in &event.tags {
        if tag.starts_with("quality:") {
            if let Ok(q) = tag[8..].parse::<i32>() {
                return Some(q);
            }
        }
    }
    None
}

#[tauri::command]
async fn get_dashboard_data(
    _state: State<'_, AppState>,
    week_start_date: Option<String>,
) -> Result<DashboardData, String> {
    use chrono::Datelike;
    use std::collections::HashMap;

    let now = chrono::Utc::now();
    let today = now.date_naive();

    // Calculate current week start for comparison
    let current_week_start = {
        let days_since_sunday = today.weekday().num_days_from_sunday() as i64;
        today - chrono::Duration::days(days_since_sunday)
    };

    // Calculate week start (Sunday) and end (Saturday)
    let week_start = if let Some(ref date_str) = week_start_date {
        chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format: {}", e))?
    } else {
        current_week_start
    };
    let week_end = week_start + chrono::Duration::days(6);
    let is_current_week = week_start == current_week_start;

    // Calculate previous week and month for comparisons
    let prev_week_start = week_start - chrono::Duration::days(7);
    let prev_week_end = prev_week_start + chrono::Duration::days(6);
    let month_start = week_start - chrono::Duration::days(28); // ~4 weeks

    // Fetch events for wider range (for comparisons)
    let filter = healthctl_lib::ipc::ListFilter {
        event_type: None,
        from: Some(chrono::DateTime::from_naive_utc_and_offset(
            month_start.and_hms_opt(0, 0, 0).unwrap(),
            chrono::Utc,
        )),
        to: Some(chrono::DateTime::from_naive_utc_and_offset(
            (week_end + chrono::Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            chrono::Utc,
        )),
        tags: vec![],
        limit: Some(2000),
    };

    let response = send_request(&Request::List(filter))?;

    let all_events = match response {
        Response::Ok(ResponseData::Events(events)) => events,
        Response::Ok(_) => return Err("Unexpected response type".to_string()),
        Response::Error { message } => return Err(message),
    };

    // Fetch status to get streak
    let streak_days = match send_request(&Request::Status) {
        Ok(Response::Ok(ResponseData::Summary(summary))) => summary.streak_days,
        _ => 0, // Default to 0 if we can't get the streak
    };

    // Filter events for different periods
    let events: Vec<_> = all_events
        .iter()
        .filter(|e| {
            e.start_time
                .map(|t| {
                    let d = t.date_naive();
                    d >= week_start && d <= week_end
                })
                .unwrap_or(false)
                || e.end_time
                    .map(|t| {
                        let d = t.date_naive();
                        d >= week_start && d <= week_end
                    })
                    .unwrap_or(false)
        })
        .collect();

    let prev_week_events: Vec<_> = all_events
        .iter()
        .filter(|e| {
            e.start_time
                .map(|t| {
                    let d = t.date_naive();
                    d >= prev_week_start && d <= prev_week_end
                })
                .unwrap_or(false)
        })
        .collect();

    let month_events: Vec<_> = all_events
        .iter()
        .filter(|e| {
            e.start_time
                .map(|t| {
                    let d = t.date_naive();
                    d >= month_start && d < week_start
                })
                .unwrap_or(false)
        })
        .collect();

    // Recent activities (within selected week, most recent first)
    let mut sorted_events = events.clone();
    sorted_events.sort_by(|a, b| b.start_time.cmp(&a.start_time));
    let recent_activities: Vec<ActivityItem> = sorted_events
        .iter()
        .take(50)
        .map(|e| event_to_activity_item(e))
        .collect();

    // Weekly summary (Sunday to Saturday)
    let mut week_summary = WeeklySummary {
        total_steps: 0,
        total_calories: 0.0,
        total_distance_km: 0.0,
        total_active_mins: 0,
        avg_sleep_hours: None,
        workout_count: 0,
        week_start: week_start.format("%b %d").to_string(),
        week_end: week_end.format("%b %d").to_string(),
    };

    let mut sleep_hours_total = 0.0;
    let mut sleep_count = 0;

    // Track detailed stats for cards
    let mut calories_by_type: HashMap<String, f64> = HashMap::new();
    let mut distance_by_type: HashMap<String, f64> = HashMap::new();
    let mut active_by_type: HashMap<String, i32> = HashMap::new();
    let mut workout_types: HashMap<String, i32> = HashMap::new();
    let mut muscle_groups: Vec<String> = Vec::new();
    let mut sleep_nights: Vec<SleepNight> = Vec::new();
    let mut total_workout_duration = 0;

    for event in &events {
        let event_date = event.start_time.map(|t| t.date_naive());
        let is_in_week = event_date
            .map(|d| d >= week_start && d <= week_end)
            .unwrap_or(false);

        if is_in_week {
            week_summary.total_steps += get_steps(event);
            week_summary.total_calories += get_calories(event);
            week_summary.total_distance_km += get_distance_km(event);

            let activity_type = get_event_activity_type(event);
            let calories = get_calories(event);
            let distance = get_distance_km(event);
            let duration = get_duration_mins(event);

            if calories > 0.0 {
                *calories_by_type.entry(activity_type.clone()).or_insert(0.0) += calories;
            }
            if distance > 0.0 {
                *distance_by_type.entry(activity_type.clone()).or_insert(0.0) += distance;
            }

            if !matches!(event.event_type, EventType::Sleep) {
                week_summary.total_active_mins += duration;
                if duration > 0 {
                    *active_by_type.entry(activity_type.clone()).or_insert(0) += duration;
                }
            }

            if is_workout(event) {
                week_summary.workout_count += 1;
                *workout_types.entry(activity_type.clone()).or_insert(0) += 1;
                total_workout_duration += duration;

                // Extract muscle groups from tags
                for tag in &event.tags {
                    let tag_lower = tag.to_lowercase();
                    if [
                        "chest",
                        "back",
                        "legs",
                        "shoulders",
                        "arms",
                        "biceps",
                        "triceps",
                        "quads",
                        "hamstrings",
                        "glutes",
                        "core",
                        "cardio",
                    ]
                    .contains(&tag_lower.as_str())
                    {
                        if !muscle_groups.contains(&tag_lower) {
                            muscle_groups.push(tag_lower);
                        }
                    }
                }
            }
        }

        // Sleep that ended within this week
        if matches!(event.event_type, EventType::Sleep) {
            if let Some(end) = event.end_time {
                let end_date = end.date_naive();
                if end_date >= week_start && end_date <= week_end {
                    let hours = get_duration_mins(event) as f64 / 60.0;
                    sleep_hours_total += hours;
                    sleep_count += 1;

                    sleep_nights.push(SleepNight {
                        date: end_date.format("%a %m/%d").to_string(),
                        hours,
                        quality: get_sleep_quality(event),
                    });
                }
            }
        }
    }

    if sleep_count > 0 {
        week_summary.avg_sleep_hours = Some(sleep_hours_total / sleep_count as f64);
    }

    // Calculate previous week totals for comparison
    let prev_week_steps: i32 = prev_week_events.iter().map(|e| get_steps(e)).sum();
    let prev_week_calories: f64 = prev_week_events.iter().map(|e| get_calories(e)).sum();
    let prev_week_distance: f64 = prev_week_events.iter().map(|e| get_distance_km(e)).sum();
    let prev_week_active: i32 = prev_week_events
        .iter()
        .filter(|e| !matches!(e.event_type, EventType::Sleep))
        .map(|e| get_duration_mins(e))
        .sum();
    let prev_week_sleep: f64 = prev_week_events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Sleep))
        .map(|e| get_duration_mins(e) as f64 / 60.0)
        .sum::<f64>();
    let prev_week_sleep_count = prev_week_events
        .iter()
        .filter(|e| matches!(e.event_type, EventType::Sleep))
        .count();
    let prev_week_workouts = prev_week_events.iter().filter(|e| is_workout(e)).count() as i32;

    // Calculate month average (per week)
    let month_weeks = 4.0;
    let month_steps: i32 = month_events.iter().map(|e| get_steps(e)).sum();
    let _month_calories: f64 = month_events.iter().map(|e| get_calories(e)).sum();
    let _month_avg_steps = month_steps as f64 / month_weeks;

    // Helper for percentage change
    let pct_change = |current: f64, previous: f64| -> f64 {
        if previous == 0.0 {
            if current > 0.0 { 100.0 } else { 0.0 }
        } else {
            ((current - previous) / previous) * 100.0
        }
    };

    // Weekly chart stats (Sunday to Saturday)
    let mut weekly = WeeklyStats { days: Vec::new() };

    for day_offset in 0..7 {
        let date = week_start + chrono::Duration::days(day_offset);
        let day_name = date.format("%a").to_string();
        let date_str = date.format("%m/%d").to_string();

        let mut day_stats = DayStats {
            date: date_str,
            day_name,
            steps: 0,
            calories: 0.0,
            distance_km: 0.0,
            active_mins: 0,
            sleep_hours: 0.0,
            workouts: 0,
        };

        for event in &events {
            // Regular events by start_time
            if let Some(start) = event.start_time {
                if start.date_naive() == date {
                    day_stats.steps += get_steps(event);
                    day_stats.calories += get_calories(event);
                    day_stats.distance_km += get_distance_km(event);
                    if !matches!(event.event_type, EventType::Sleep) {
                        day_stats.active_mins += get_duration_mins(event);
                    }
                    if is_workout(event) {
                        day_stats.workouts += 1;
                    }
                }
            }

            // Sleep by end_time (when you woke up)
            if matches!(event.event_type, EventType::Sleep) {
                if let Some(end) = event.end_time {
                    if end.date_naive() == date {
                        day_stats.sleep_hours += get_duration_mins(event) as f64 / 60.0;
                    }
                }
            }
        }

        weekly.days.push(day_stats);
    }

    // Find best days for each metric
    let best_steps_day = weekly.days.iter().max_by_key(|d| d.steps);
    let best_calories_day = weekly.days.iter().max_by(|a, b| {
        a.calories
            .partial_cmp(&b.calories)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best_distance_day = weekly.days.iter().max_by(|a, b| {
        a.distance_km
            .partial_cmp(&b.distance_km)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let most_active_day = weekly.days.iter().max_by_key(|d| d.active_mins);

    // Calculate days elapsed for projections (current week only)
    let days_elapsed = if is_current_week {
        (today.weekday().num_days_from_sunday() + 1) as f64
    } else {
        7.0
    };

    // Sort breakdowns by value descending
    let mut calories_breakdown: Vec<_> = calories_by_type.into_iter().collect();
    calories_breakdown.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut distance_breakdown: Vec<_> = distance_by_type.into_iter().collect();
    distance_breakdown.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut active_breakdown: Vec<_> = active_by_type.into_iter().collect();
    active_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

    let mut workout_breakdown: Vec<_> = workout_types.into_iter().collect();
    workout_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

    // Sleep stats - clone data we need before moving sleep_nights
    let best_sleep_night = sleep_nights
        .iter()
        .max_by(|a, b| {
            a.hours
                .partial_cmp(&b.hours)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    let worst_sleep_night = sleep_nights
        .iter()
        .filter(|n| n.hours > 0.0)
        .min_by(|a, b| {
            a.hours
                .partial_cmp(&b.hours)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    let avg_quality = {
        let qualities: Vec<_> = sleep_nights.iter().filter_map(|n| n.quality).collect();
        if qualities.is_empty() {
            0.0
        } else {
            qualities.iter().sum::<i32>() as f64 / qualities.len() as f64
        }
    };

    // Build card details
    let card_details = CardDetails {
        steps: StepsDetails {
            best_day: best_steps_day
                .map(|d| d.day_name.clone())
                .unwrap_or_default(),
            best_day_steps: best_steps_day.map(|d| d.steps).unwrap_or(0),
            daily_avg: if days_elapsed > 0.0 {
                (week_summary.total_steps as f64 / days_elapsed) as i32
            } else {
                0
            },
            vs_last_week: pct_change(week_summary.total_steps as f64, prev_week_steps as f64),
            vs_last_month: pct_change(
                week_summary.total_steps as f64,
                month_steps as f64 / month_weeks,
            ),
            projection: if is_current_week && days_elapsed > 0.0 {
                Some(((week_summary.total_steps as f64 / days_elapsed) * 7.0) as i32)
            } else {
                None
            },
        },
        calories: CaloriesDetails {
            by_activity: calories_breakdown.into_iter().take(5).collect(),
            daily_avg: if days_elapsed > 0.0 {
                week_summary.total_calories / days_elapsed
            } else {
                0.0
            },
            best_day: best_calories_day
                .map(|d| d.day_name.clone())
                .unwrap_or_default(),
            best_day_calories: best_calories_day.map(|d| d.calories).unwrap_or(0.0),
            vs_last_week: pct_change(week_summary.total_calories, prev_week_calories),
            projection: if is_current_week && days_elapsed > 0.0 {
                Some((week_summary.total_calories / days_elapsed) * 7.0)
            } else {
                None
            },
        },
        distance: DistanceDetails {
            by_activity: distance_breakdown.into_iter().take(5).collect(),
            best_day: best_distance_day
                .map(|d| d.day_name.clone())
                .unwrap_or_default(),
            best_day_km: best_distance_day.map(|d| d.distance_km).unwrap_or(0.0),
            total_elevation: 0.0, // Would need elevation data
            vs_last_week: pct_change(week_summary.total_distance_km, prev_week_distance),
            projection: if is_current_week && days_elapsed > 0.0 {
                Some((week_summary.total_distance_km / days_elapsed) * 7.0)
            } else {
                None
            },
        },
        active: ActiveDetails {
            by_activity: active_breakdown.into_iter().take(5).collect(),
            daily_avg: if days_elapsed > 0.0 {
                (week_summary.total_active_mins as f64 / days_elapsed) as i32
            } else {
                0
            },
            most_active_day: most_active_day
                .map(|d| d.day_name.clone())
                .unwrap_or_default(),
            vs_last_week: pct_change(
                week_summary.total_active_mins as f64,
                prev_week_active as f64,
            ),
            projection: if is_current_week && days_elapsed > 0.0 {
                Some(((week_summary.total_active_mins as f64 / days_elapsed) * 7.0) as i32)
            } else {
                None
            },
        },
        sleep: SleepDetails {
            nights: sleep_nights,
            best_night: best_sleep_night
                .as_ref()
                .map(|n| n.date.clone())
                .unwrap_or_default(),
            best_night_hours: best_sleep_night.as_ref().map(|n| n.hours).unwrap_or(0.0),
            worst_night: worst_sleep_night
                .as_ref()
                .map(|n| n.date.clone())
                .unwrap_or_default(),
            worst_night_hours: worst_sleep_night.as_ref().map(|n| n.hours).unwrap_or(0.0),
            avg_quality,
            vs_last_week: if prev_week_sleep_count > 0 {
                pct_change(
                    sleep_hours_total / sleep_count.max(1) as f64,
                    prev_week_sleep / prev_week_sleep_count as f64,
                )
            } else {
                0.0
            },
        },
        workouts: WorkoutsDetails {
            by_type: workout_breakdown.into_iter().take(5).collect(),
            total_duration: total_workout_duration,
            avg_duration: if week_summary.workout_count > 0 {
                total_workout_duration / week_summary.workout_count
            } else {
                0
            },
            vs_last_week: pct_change(week_summary.workout_count as f64, prev_week_workouts as f64),
            muscle_groups,
        },
    };

    Ok(DashboardData {
        recent_activities,
        week_summary,
        weekly,
        card_details,
        is_current_week,
        connection_status: format!("Connected ({} events)", events.len()),
        streak_days,
    })
}

fn main() {
    let app_state = AppState {};

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![get_dashboard_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
