// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
struct DashboardMetrics {
    heart_rate: Option<f64>,
    steps: Option<i32>,
    calories: Option<f64>,
    distance: Option<f64>,
    active_minutes: Option<i32>,
    sleep_hours: Option<f64>,
}

struct AppState {
    metrics: Mutex<DashboardMetrics>,
}

#[tauri::command]
fn get_metrics(state: State<AppState>) -> DashboardMetrics {
    let metrics = state.metrics.lock().unwrap();
    DashboardMetrics {
        heart_rate: metrics.heart_rate,
        steps: metrics.steps,
        calories: metrics.calories,
        distance: metrics.distance,
        active_minutes: metrics.active_minutes,
        sleep_hours: metrics.sleep_hours,
    }
}

#[tauri::command]
fn update_metrics(
    heart_rate: Option<f64>,
    steps: Option<i32>,
    calories: Option<f64>,
    distance: Option<f64>,
    active_minutes: Option<i32>,
    sleep_hours: Option<f64>,
    state: State<AppState>,
) {
    let mut metrics = state.metrics.lock().unwrap();
    if heart_rate.is_some() {
        metrics.heart_rate = heart_rate;
    }
    if steps.is_some() {
        metrics.steps = steps;
    }
    if calories.is_some() {
        metrics.calories = calories;
    }
    if distance.is_some() {
        metrics.distance = distance;
    }
    if active_minutes.is_some() {
        metrics.active_minutes = active_minutes;
    }
    if sleep_hours.is_some() {
        metrics.sleep_hours = sleep_hours;
    }
}

fn main() {
    let app_state = AppState {
        metrics: Mutex::new(DashboardMetrics {
            heart_rate: Some(72.0),
            steps: Some(8432),
            calories: Some(320.5),
            distance: Some(6.2),
            active_minutes: Some(45),
            sleep_hours: Some(7.5),
        }),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![get_metrics, update_metrics])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
