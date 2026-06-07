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
    active_mins: i32,
    workouts: i32,
}

/// Full dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DashboardData {
    recent_activities: Vec<ActivityItem>,
    week_summary: WeeklySummary,
    weekly: WeeklyStats,
    connection_status: String,
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
                let _ = Command::new("cargo")
                    .args(["run", "-p", "healthctl-daemon"])
                    .spawn();
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

#[tauri::command]
async fn get_dashboard_data(
    _state: State<'_, AppState>,
    week_start_date: Option<String>,
) -> Result<DashboardData, String> {
    use chrono::Datelike;

    let now = chrono::Utc::now();
    let today = now.date_naive();

    // Calculate week start (Sunday) and end (Saturday)
    let week_start = if let Some(ref date_str) = week_start_date {
        chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format: {}", e))?
    } else {
        let days_since_sunday = today.weekday().num_days_from_sunday() as i64;
        today - chrono::Duration::days(days_since_sunday)
    };
    let week_end = week_start + chrono::Duration::days(6);

    // Fetch events for the selected week only
    let filter = healthctl_lib::ipc::ListFilter {
        event_type: None,
        from: Some(chrono::DateTime::from_naive_utc_and_offset(
            week_start.and_hms_opt(0, 0, 0).unwrap(),
            chrono::Utc,
        )),
        to: Some(chrono::DateTime::from_naive_utc_and_offset(
            (week_end + chrono::Duration::days(1))
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            chrono::Utc,
        )),
        tags: vec![],
        limit: Some(500),
    };

    let response = send_request(&Request::List(filter))?;

    let events = match response {
        Response::Ok(ResponseData::Events(events)) => events,
        Response::Ok(_) => return Err("Unexpected response type".to_string()),
        Response::Error { message } => return Err(message),
    };

    // Recent activities (within selected week, most recent first)
    let mut sorted_events: Vec<_> = events
        .iter()
        .filter(|e| {
            // Filter to events within the selected week
            e.start_time
                .map(|t| {
                    let date = t.date_naive();
                    date >= week_start && date <= week_end
                })
                .unwrap_or(false)
        })
        .collect();
    sorted_events.sort_by(|a, b| b.start_time.cmp(&a.start_time));
    let recent_activities: Vec<ActivityItem> = sorted_events
        .iter()
        .take(50) // Show more since it's filtered to one week
        .map(|e| event_to_activity_item(e))
        .collect();

    // Weekly summary (Sunday to today)
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

    for event in &events {
        // Check if event falls within current week (Sunday to Saturday)
        let event_date = event.start_time.map(|t| t.date_naive());
        let is_in_week = event_date
            .map(|d| d >= week_start && d <= week_end)
            .unwrap_or(false);

        if is_in_week {
            week_summary.total_steps += get_steps(event);
            week_summary.total_calories += get_calories(event);
            week_summary.total_distance_km += get_distance_km(event);

            // Only count active minutes for non-sleep events
            if !matches!(event.event_type, EventType::Sleep) {
                week_summary.total_active_mins += get_duration_mins(event);
            }

            if is_workout(event) {
                week_summary.workout_count += 1;
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
                }
            }
        }
    }

    if sleep_count > 0 {
        week_summary.avg_sleep_hours = Some(sleep_hours_total / sleep_count as f64);
    }

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
            active_mins: 0,
            workouts: 0,
        };

        for event in &events {
            if let Some(start) = event.start_time {
                if start.date_naive() == date {
                    day_stats.steps += get_steps(event);
                    day_stats.calories += get_calories(event);
                    if !matches!(event.event_type, EventType::Sleep) {
                        day_stats.active_mins += get_duration_mins(event);
                    }
                    if is_workout(event) {
                        day_stats.workouts += 1;
                    }
                }
            }
        }

        weekly.days.push(day_stats);
    }

    Ok(DashboardData {
        recent_activities,
        week_summary,
        weekly,
        connection_status: format!("Connected ({} events)", events.len()),
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
