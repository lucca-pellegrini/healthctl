//! Report aggregation: turn a slice of events into rich [`ReportData`].
//!
//! This mirrors the dashboard's client-side computation (see
//! `healthctl-dashboard/src/main.rs::get_dashboard_data`) but generalized over
//! an arbitrary period instead of a fixed Sunday–Saturday week. The same metric
//! extraction rules are used so the CLI and dashboard agree:
//!
//! - Steps:    sum of the `steps` metric.
//! - Calories: sum of `calories` / `calories_kcal`.
//! - Distance: sum of `distance_m` (m→km) / `distance`.
//! - Active:   sum of durations for all non-sleep events.
//! - Sleep:    average hours per night, attributed to the *wake* (end) day.
//! - Workouts: count of Strength/Activity events that have an end time.

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};

use crate::event::{Event, EventType};
use crate::ipc::{
    ActiveReport, Breakdown, CaloriesReport, DistanceReport, ReportData, ReportPeriod, SleepNight,
    SleepReport, StepsReport, WorkoutsReport,
};

/// Muscle groups recognized in workout tags, matched against the dashboard's
/// fixed allow-list (case-insensitive).
const MUSCLE_GROUPS: &[&str] = &[
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
];

// ---- metric extraction helpers (mirror the dashboard) ----

fn get_calories(e: &Event) -> f64 {
    e.metrics
        .get("calories")
        .or_else(|| e.metrics.get("calories_kcal"))
        .copied()
        .unwrap_or(0.0)
}

fn get_distance_km(e: &Event) -> f64 {
    if let Some(m) = e.metrics.get("distance_m") {
        m / 1000.0
    } else if let Some(km) = e.metrics.get("distance") {
        *km
    } else {
        0.0
    }
}

fn get_steps(e: &Event) -> f64 {
    e.metrics.get("steps").copied().unwrap_or(0.0)
}

fn get_duration_mins(e: &Event) -> f64 {
    e.duration_secs().map(|s| s / 60.0).unwrap_or(0.0)
}

fn is_sleep(e: &Event) -> bool {
    matches!(e.event_type, EventType::Sleep)
}

fn is_workout(e: &Event) -> bool {
    matches!(e.event_type, EventType::Strength | EventType::Activity(_)) && e.end_time.is_some()
}

/// Lowercased activity-type label used for breakdowns, e.g. "walk", "run",
/// "strength", "sleep", "nutrition".
fn activity_label(e: &Event) -> String {
    match &e.event_type {
        EventType::Activity(kind) => format!("{kind:?}").to_lowercase(),
        EventType::Strength => "strength".into(),
        EventType::Sleep => "sleep".into(),
        EventType::Nutrition => "nutrition".into(),
        EventType::Hydration => "hydration".into(),
        EventType::Substance => "substance".into(),
        EventType::Mental(kind) => format!("{kind:?}").to_lowercase(),
    }
}

fn get_sleep_quality(e: &Event) -> Option<i32> {
    for tag in &e.tags {
        if let Some(rest) = tag.strip_prefix("quality:") {
            if let Ok(n) = rest.trim().parse::<i32>() {
                return Some(n);
            }
        }
    }
    None
}

/// The reference time an event belongs to. Sleep is attributed to its wake
/// (end) time; everything else to its start (falling back to end/created).
fn event_anchor(e: &Event) -> Option<DateTime<Utc>> {
    if is_sleep(e) {
        e.end_time.or(e.start_time)
    } else {
        e.start_time.or(e.end_time)
    }
}

fn in_range(t: DateTime<Utc>, from: DateTime<Utc>, to: DateTime<Utc>) -> bool {
    t >= from && t < to
}

/// Percent change of `current` relative to `previous`, matching the dashboard:
/// if previous is zero, returns 100 when current is positive else 0.
fn pct_change(current: f64, previous: f64) -> f64 {
    if previous == 0.0 {
        if current > 0.0 { 100.0 } else { 0.0 }
    } else {
        ((current - previous) / previous) * 100.0
    }
}

fn period_days(period: &ReportPeriod) -> i64 {
    match period {
        ReportPeriod::Day => 1,
        ReportPeriod::Week => 7,
        ReportPeriod::Month => 30,
        ReportPeriod::Year => 365,
    }
}

/// Aggregate of a metric set over a slice of events. Used for the previous
/// period (where we only need scalar totals) and the baseline window.
struct Totals {
    steps: f64,
    calories: f64,
    distance_km: f64,
    active_minutes: f64,
    sleep_hours: f64,
    sleep_nights: u32,
    workouts: u32,
}

fn totals_for<'a>(
    events: impl Iterator<Item = &'a Event>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Totals {
    let mut t = Totals {
        steps: 0.0,
        calories: 0.0,
        distance_km: 0.0,
        active_minutes: 0.0,
        sleep_hours: 0.0,
        sleep_nights: 0,
        workouts: 0,
    };
    for e in events {
        let Some(anchor) = event_anchor(e) else {
            continue;
        };
        if !in_range(anchor, from, to) {
            continue;
        }
        t.steps += get_steps(e);
        t.calories += get_calories(e);
        t.distance_km += get_distance_km(e);
        if is_sleep(e) {
            let h = get_duration_mins(e) / 60.0;
            if h > 0.0 {
                t.sleep_hours += h;
                t.sleep_nights += 1;
            }
        } else {
            t.active_minutes += get_duration_mins(e);
        }
        if is_workout(e) {
            t.workouts += 1;
        }
    }
    t
}

/// Sort a label→value map into a descending top-N breakdown.
fn top_breakdowns(map: std::collections::HashMap<String, f64>, n: usize) -> Vec<Breakdown> {
    let mut v: Vec<Breakdown> = map
        .into_iter()
        .map(|(label, value)| Breakdown { label, value })
        .collect();
    v.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v.truncate(n);
    v
}

/// Compute a [`ReportData`] for `period` ending at `now`, from `events`
/// (which must cover at least the period plus a baseline window before it).
///
/// - `from`/`to` define the current period `[from, to)`.
/// - The previous period is `[from - len, from)`.
/// - The baseline window for the "vs average" comparison is the four periods
///   before the current one, averaged per period.
/// - `is_current` enables projections.
pub fn compute_report(
    events: &[Event],
    period: ReportPeriod,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    now: DateTime<Utc>,
    is_current: bool,
) -> ReportData {
    let pdays = period_days(&period);
    let len = to - from;
    let prev_from = from - len;
    let baseline_from = from - len * 4;

    // Days elapsed (for averages/projection). For a current period, count from
    // the start up to `now`; otherwise the full period length.
    let days_elapsed = if is_current {
        let elapsed = (now - from).num_days() + 1;
        elapsed.clamp(1, pdays)
    } else {
        pdays
    };

    // ---- current-period accumulation ----
    let mut total_events = 0u32;
    let mut steps_total = 0.0;
    let mut calories_total = 0.0;
    let mut distance_total = 0.0;
    let mut active_total = 0.0;
    let mut workout_count = 0u32;
    let mut workout_duration = 0.0;

    let mut calories_by_type: std::collections::HashMap<String, f64> = Default::default();
    let mut distance_by_type: std::collections::HashMap<String, f64> = Default::default();
    let mut active_by_type: std::collections::HashMap<String, f64> = Default::default();
    let mut workout_types: std::collections::HashMap<String, f64> = Default::default();
    let mut muscle_groups: Vec<String> = Vec::new();

    // Per-day buckets keyed by day index for "best day" lookups.
    let mut day_steps: std::collections::HashMap<i64, f64> = Default::default();
    let mut day_calories: std::collections::HashMap<i64, f64> = Default::default();
    let mut day_distance: std::collections::HashMap<i64, f64> = Default::default();
    let mut day_active: std::collections::HashMap<i64, f64> = Default::default();

    let mut sleep_nights: Vec<SleepNight> = Vec::new();
    let mut sleep_hours_total = 0.0;
    let mut sleep_quality_sum = 0i64;
    let mut sleep_quality_count = 0u32;

    for e in events {
        let Some(anchor) = event_anchor(e) else {
            continue;
        };
        if !in_range(anchor, from, to) {
            continue;
        }
        total_events += 1;
        let label = activity_label(e);
        let day_idx = (anchor - from).num_days();

        let steps = get_steps(e);
        let calories = get_calories(e);
        let dist = get_distance_km(e);

        steps_total += steps;
        calories_total += calories;
        distance_total += dist;
        *day_steps.entry(day_idx).or_default() += steps;
        *day_calories.entry(day_idx).or_default() += calories;
        *day_distance.entry(day_idx).or_default() += dist;

        if calories > 0.0 {
            *calories_by_type.entry(label.clone()).or_default() += calories;
        }
        if dist > 0.0 {
            *distance_by_type.entry(label.clone()).or_default() += dist;
        }

        if is_sleep(e) {
            let hours = get_duration_mins(e) / 60.0;
            if hours > 0.0 {
                sleep_hours_total += hours;
                if let Some(q) = get_sleep_quality(e) {
                    sleep_quality_sum += q as i64;
                    sleep_quality_count += 1;
                }
                sleep_nights.push(SleepNight {
                    date: anchor.format("%a %m/%d").to_string(),
                    hours,
                    quality: get_sleep_quality(e),
                });
            }
        } else {
            let mins = get_duration_mins(e);
            active_total += mins;
            *day_active.entry(day_idx).or_default() += mins;
            if mins > 0.0 {
                *active_by_type.entry(label.clone()).or_default() += mins;
            }
        }

        if is_workout(e) {
            workout_count += 1;
            workout_duration += get_duration_mins(e);
            *workout_types.entry(label.clone()).or_default() += 1.0;
            for tag in &e.tags {
                let tl = tag.to_lowercase();
                if MUSCLE_GROUPS.contains(&tl.as_str()) && !muscle_groups.contains(&tl) {
                    muscle_groups.push(tl);
                }
            }
        }
    }

    // ---- previous period + baseline ----
    let prev = totals_for(events.iter(), prev_from, from);
    let baseline = totals_for(events.iter(), baseline_from, from);

    // "best day" label: format the day's date.
    let day_label = |idx: i64| -> String {
        let d = from + Duration::days(idx);
        d.format("%a %m/%d").to_string()
    };
    let best_of = |map: &std::collections::HashMap<i64, f64>| -> (Option<String>, f64) {
        map.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, v)| (Some(day_label(*idx)), *v))
            .unwrap_or((None, 0.0))
    };

    let (steps_best, steps_best_v) = best_of(&day_steps);
    let (cal_best, cal_best_v) = best_of(&day_calories);
    let (dist_best, dist_best_v) = best_of(&day_distance);
    let (active_best, _active_best_v) = best_of(&day_active);

    let proj = |total: f64| -> Option<f64> {
        if is_current && days_elapsed > 0 {
            Some((total / days_elapsed as f64) * pdays as f64)
        } else {
            None
        }
    };

    let steps = StepsReport {
        total: steps_total,
        daily_avg: steps_total / days_elapsed as f64,
        best_day: steps_best,
        best_day_value: steps_best_v,
        vs_previous: pct_change(steps_total, prev.steps),
        vs_average: pct_change(steps_total, baseline.steps / 4.0),
        projection: proj(steps_total),
    };

    let calories = CaloriesReport {
        total: calories_total,
        by_activity: top_breakdowns(calories_by_type, 5),
        daily_avg: calories_total / days_elapsed as f64,
        best_day: cal_best,
        best_day_value: cal_best_v,
        vs_previous: pct_change(calories_total, prev.calories),
        projection: proj(calories_total),
    };

    let distance = DistanceReport {
        total_km: distance_total,
        by_activity: top_breakdowns(distance_by_type, 5),
        best_day: dist_best,
        best_day_value: dist_best_v,
        vs_previous: pct_change(distance_total, prev.distance_km),
        projection: proj(distance_total),
    };

    let active = ActiveReport {
        total_minutes: active_total,
        by_activity: top_breakdowns(active_by_type, 5),
        daily_avg: active_total / days_elapsed as f64,
        most_active_day: active_best,
        vs_previous: pct_change(active_total, prev.active_minutes),
        projection: proj(active_total),
    };

    // Sleep statistics.
    let sleep_count = sleep_nights.len() as u32;
    let avg_hours = if sleep_count > 0 {
        Some(sleep_hours_total / sleep_count as f64)
    } else {
        None
    };
    let (best_night, best_night_hours) = sleep_nights
        .iter()
        .max_by(|a, b| {
            a.hours
                .partial_cmp(&b.hours)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|n| (Some(n.date.clone()), n.hours))
        .unwrap_or((None, 0.0));
    let (worst_night, worst_night_hours) = sleep_nights
        .iter()
        .filter(|n| n.hours > 0.0)
        .min_by(|a, b| {
            a.hours
                .partial_cmp(&b.hours)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|n| (Some(n.date.clone()), n.hours))
        .unwrap_or((None, 0.0));
    let avg_quality = if sleep_quality_count > 0 {
        sleep_quality_sum as f64 / sleep_quality_count as f64
    } else {
        0.0
    };
    let cur_avg_night = avg_hours.unwrap_or(0.0);
    let prev_avg_night = if prev.sleep_nights > 0 {
        prev.sleep_hours / prev.sleep_nights as f64
    } else {
        0.0
    };
    // Show newest nights first.
    sleep_nights.reverse();
    let sleep = SleepReport {
        avg_hours,
        nights: sleep_nights,
        best_night,
        best_night_hours,
        worst_night,
        worst_night_hours,
        avg_quality,
        vs_previous: if prev.sleep_nights > 0 {
            pct_change(cur_avg_night, prev_avg_night)
        } else {
            0.0
        },
    };

    let avg_duration = if workout_count > 0 {
        workout_duration / workout_count as f64
    } else {
        0.0
    };
    let workouts = WorkoutsReport {
        count: workout_count,
        by_type: top_breakdowns(workout_types, 5),
        total_duration: workout_duration,
        avg_duration,
        vs_previous: pct_change(workout_count as f64, prev.workouts as f64),
        muscle_groups,
    };

    let range_label = format_range(from, to);

    ReportData {
        period,
        period_days: pdays,
        range_label,
        is_current,
        days_elapsed,
        total_events,
        steps,
        calories,
        distance,
        active,
        sleep,
        workouts,
    }
}

/// Format a `[from, to)` range as e.g. "May 31 – Jun 06" (the displayed end is
/// the last *inclusive* day).
fn format_range(from: DateTime<Utc>, to: DateTime<Utc>) -> String {
    let from_local = chrono::Local.from_utc_datetime(&from.naive_utc());
    let last_local = chrono::Local.from_utc_datetime(&(to - Duration::seconds(1)).naive_utc());
    if from_local.year() == last_local.year()
        && from_local.month() == last_local.month()
        && from_local.day() == last_local.day()
    {
        from_local.format("%b %d, %Y").to_string()
    } else {
        format!(
            "{} – {}",
            from_local.format("%b %d"),
            last_local.format("%b %d, %Y")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ActivityKind, EventType};
    use chrono::TimeZone;
    use std::collections::HashMap;

    fn mk(
        kind: EventType,
        start: DateTime<Utc>,
        dur_mins: i64,
        metrics: &[(&str, f64)],
        tags: &[&str],
    ) -> Event {
        let mut m = HashMap::new();
        for (k, v) in metrics {
            m.insert((*k).to_string(), *v);
        }
        Event {
            id: uuid::Uuid::new_v4(),
            event_type: kind,
            start_time: Some(start),
            end_time: Some(start + Duration::minutes(dur_mins)),
            metrics: m,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            exercises: vec![],
            created_at: start,
        }
    }

    #[test]
    fn test_pct_change_basic() {
        assert_eq!(pct_change(110.0, 100.0), 10.0);
        assert_eq!(pct_change(90.0, 100.0), -10.0);
        assert_eq!(pct_change(5.0, 0.0), 100.0);
        assert_eq!(pct_change(0.0, 0.0), 0.0);
    }

    #[test]
    fn test_basic_aggregation() {
        let to = Utc.with_ymd_and_hms(2026, 6, 8, 0, 0, 0).unwrap();
        let from = to - Duration::days(7);
        // Two walks this week.
        let events = vec![
            mk(
                EventType::Activity(ActivityKind::Walk),
                from + Duration::days(1),
                25,
                &[
                    ("steps", 3000.0),
                    ("distance_m", 2000.0),
                    ("calories_kcal", 120.0),
                ],
                &["daily"],
            ),
            mk(
                EventType::Activity(ActivityKind::Walk),
                from + Duration::days(2),
                25,
                &[
                    ("steps", 3000.0),
                    ("distance_m", 2000.0),
                    ("calories_kcal", 120.0),
                ],
                &["daily"],
            ),
            mk(
                EventType::Strength,
                from + Duration::days(3),
                60,
                &[("calories_kcal", 300.0)],
                &["gym", "chest", "triceps"],
            ),
            mk(
                EventType::Sleep,
                from + Duration::days(3),
                480,
                &[],
                &["night", "quality:8"],
            ),
        ];

        let r = compute_report(&events, ReportPeriod::Week, from, to, to, false);
        assert_eq!(r.total_events, 4);
        assert_eq!(r.steps.total, 6000.0);
        assert_eq!(r.calories.total, 540.0);
        assert!((r.distance.total_km - 4.0).abs() < 1e-9);
        // Active excludes sleep: 25 + 25 + 60 = 110 minutes.
        assert!((r.active.total_minutes - 110.0).abs() < 1e-9);
        assert_eq!(r.workouts.count, 3); // 2 walks + 1 strength
        assert_eq!(r.sleep.nights.len(), 1);
        assert_eq!(r.sleep.avg_hours, Some(8.0));
        assert_eq!(r.sleep.avg_quality, 8.0);
        // Muscle groups from strength tags (chest, triceps), not "gym".
        assert!(r.workouts.muscle_groups.contains(&"chest".to_string()));
        assert!(r.workouts.muscle_groups.contains(&"triceps".to_string()));
        assert!(!r.workouts.muscle_groups.contains(&"gym".to_string()));
        // Not current → no projection.
        assert!(r.steps.projection.is_none());
    }

    #[test]
    fn test_projection_current_period() {
        let now = Utc.with_ymd_and_hms(2026, 6, 4, 12, 0, 0).unwrap();
        let from = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let to = from + Duration::days(7);
        let events = vec![mk(
            EventType::Activity(ActivityKind::Walk),
            from + Duration::days(1),
            25,
            &[("steps", 4000.0)],
            &[],
        )];
        let r = compute_report(&events, ReportPeriod::Week, from, to, now, true);
        assert!(r.is_current);
        // Projection present and >= the current total.
        let proj = r.steps.projection.unwrap();
        assert!(proj >= r.steps.total);
    }
}
