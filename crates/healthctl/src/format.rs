//! Pretty-printing utilities with ANSI colors, emojis, and Unicode formatting.

use healthctl_lib::event::{ActivityKind, Event, EventType, MentalKind};
use healthctl_lib::ipc::{ReportData, ReportPeriod, StatusSummary};
use owo_colors::OwoColorize;

/// Get emoji for event type.
pub fn event_emoji(event_type: &EventType) -> &'static str {
    match event_type {
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

/// Format event type as a colored, pretty string.
pub fn format_event_type(event_type: &EventType) -> String {
    let emoji = event_emoji(event_type);
    let name = match event_type {
        EventType::Activity(kind) => {
            let kind_str = match kind {
                ActivityKind::Run => "Run",
                ActivityKind::Walk => "Walk",
                ActivityKind::Cycle => "Cycle",
                ActivityKind::Swim => "Swim",
                ActivityKind::Hike => "Hike",
                ActivityKind::Other(s) => s.as_str(),
            };
            format!("{} {}", emoji, kind_str.cyan())
        }
        EventType::Strength => format!("{} {}", emoji, "Strength".magenta()),
        EventType::Sleep => format!("{} {}", emoji, "Sleep".blue()),
        EventType::Nutrition => format!("{} {}", emoji, "Nutrition".yellow()),
        EventType::Hydration => format!("{} {}", emoji, "Hydration".cyan()),
        EventType::Substance => format!("{} {}", emoji, "Substance".white()),
        EventType::Mental(kind) => {
            let kind_str = match kind {
                MentalKind::Meditation => "Meditation",
                MentalKind::Relaxation => "Relaxation",
                MentalKind::Prayer => "Prayer",
                MentalKind::Journaling => "Journaling",
                MentalKind::Other(s) => s.as_str(),
            };
            format!("{} {}", emoji, kind_str.green())
        }
    };
    name
}

/// Format event type as plain string (no color codes) for width calculation.
pub fn format_event_type_plain(event_type: &EventType) -> String {
    let emoji = event_emoji(event_type);
    match event_type {
        EventType::Activity(kind) => {
            let kind_str = match kind {
                ActivityKind::Run => "Run",
                ActivityKind::Walk => "Walk",
                ActivityKind::Cycle => "Cycle",
                ActivityKind::Swim => "Swim",
                ActivityKind::Hike => "Hike",
                ActivityKind::Other(s) => s.as_str(),
            };
            format!("{} {}", emoji, kind_str)
        }
        EventType::Strength => format!("{} Strength", emoji),
        EventType::Sleep => format!("{} Sleep", emoji),
        EventType::Nutrition => format!("{} Nutrition", emoji),
        EventType::Hydration => format!("{} Hydration", emoji),
        EventType::Substance => format!("{} Substance", emoji),
        EventType::Mental(kind) => {
            let kind_str = match kind {
                MentalKind::Meditation => "Meditation",
                MentalKind::Relaxation => "Relaxation",
                MentalKind::Prayer => "Prayer",
                MentalKind::Journaling => "Journaling",
                MentalKind::Other(s) => s.as_str(),
            };
            format!("{} {}", emoji, kind_str)
        }
    }
}

/// Format duration in seconds to human-readable string.
pub fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    if h > 0 {
        format!("{}h {:02}m", h, m)
    } else {
        format!("{}m", m)
    }
}

/// Format duration with color.
pub fn format_duration_colored(secs: f64) -> String {
    format_duration(secs).bright_white().to_string()
}

/// Format a percentage change with appropriate color.
pub fn format_percentage(value: f64) -> String {
    if value > 0.0 {
        format!("+{:.1}%", value).green().to_string()
    } else if value < 0.0 {
        format!("{:.1}%", value).red().to_string()
    } else {
        "0.0%".dimmed().to_string()
    }
}

/// Create a Unicode progress bar.
pub fn progress_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Create a colored progress bar.
pub fn progress_bar_colored(ratio: f64, width: usize, color: BarColor) -> String {
    let bar = progress_bar(ratio, width);
    match color {
        BarColor::Cyan => bar.cyan().to_string(),
        BarColor::Green => bar.green().to_string(),
        BarColor::Purple => bar.purple().to_string(),
        BarColor::Yellow => bar.yellow().to_string(),
        BarColor::Blue => bar.blue().to_string(),
        BarColor::Red => bar.red().to_string(),
    }
}

#[derive(Clone, Copy)]
pub enum BarColor {
    Cyan,
    Green,
    Purple,
    Yellow,
    Blue,
    Red,
}

/// Format tags as colored badges.
pub fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    tags.iter()
        .map(|t| format!("[{}]", t.dimmed()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format tags for table display (comma-separated, dimmed).
pub fn format_tags_compact(tags: &[String]) -> String {
    if tags.is_empty() {
        return "—".dimmed().to_string();
    }
    tags.join(", ").dimmed().to_string()
}

/// Print a section header.
pub fn print_section(title: &str) {
    println!("\n {} {}", "▸".cyan(), title.bold());
    println!(" {}", "─".repeat(40).dimmed());
}

/// Print a key-value row.
pub fn print_row(label: &str, value: &str) {
    println!("   {:<20} {}", label.dimmed(), value);
}

/// Print a detailed event view.
pub fn print_event_detail(event: &Event) {
    let type_str = format_event_type(&event.event_type);

    println!();
    let short_id = &event.id.to_string()[..8];
    println!(" {} {}", "Event".bold(), short_id.bright_black());
    println!(" {}", "═".repeat(50).dimmed());

    print_row("Type", &type_str);

    if let Some(start) = event.start_time {
        print_row("Start", &start.format("%Y-%m-%d %H:%M:%S %Z").to_string());
    }
    if let Some(end) = event.end_time {
        print_row("End", &end.format("%Y-%m-%d %H:%M:%S %Z").to_string());
    }
    if let Some(dur) = event.duration_secs() {
        print_row("Duration", &format_duration_colored(dur));
    }

    if !event.metrics.is_empty() {
        print_section("Metrics");
        let mut keys: Vec<&String> = event.metrics.keys().collect();
        keys.sort();
        for key in keys {
            let val = event.metrics[key];
            let name = pretty_metric_name(key);
            let value = format_metric_value(key, val);
            print_row(&name, &value.bright_white().to_string());
        }
    }

    if !event.tags.is_empty() {
        print_row("Tags", &format_tags(&event.tags));
    }

    if !event.exercises.is_empty() {
        print_section("Exercises");
        for ex in &event.exercises {
            let mut parts = vec![ex.name.clone().cyan().to_string()];
            if let Some(s) = ex.sets {
                parts.push(format!("{}×", s));
            }
            if let Some(r) = ex.reps {
                parts.push(format!("{} reps", r));
            }
            if let Some(w) = ex.weight_kg {
                parts.push(format!("{:.1}kg", w).yellow().to_string());
            }
            println!("   {}", parts.join(" "));
        }
    }

    println!(
        "\n   {} {}",
        "Created:".dimmed(),
        event
            .created_at
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string()
            .dimmed()
    );
    println!();
}

/// Print status summary with colors and emojis.
pub fn print_status(summary: &StatusSummary) {
    println!();
    println!(" {} {}", "📊".bold(), "Today's Status".bold());
    println!(" {}", "═".repeat(40).dimmed());

    print_row(
        "Events",
        &summary.today_events.to_string().bright_white().to_string(),
    );
    print_row(
        "Calories",
        &format!("{:.0} kcal", summary.today_calories)
            .yellow()
            .to_string(),
    );
    print_row(
        "Active Time",
        &format!("{:.0} min", summary.today_active_minutes)
            .cyan()
            .to_string(),
    );

    println!();
    print_row(
        "This Week",
        &format!("{} events", summary.week_events)
            .dimmed()
            .to_string(),
    );

    // Streak with fire emoji
    let streak_str = if summary.streak_days > 0 {
        format!("🔥 {} days", summary.streak_days)
            .bright_red()
            .bold()
            .to_string()
    } else {
        "0 days".dimmed().to_string()
    };
    print_row("Streak", &streak_str);
    println!();
}

/// Print report with bars and colors.
pub fn print_report(report: &ReportData, breakdown: Option<&[(String, u32)]>) {
    let period_str = match report.period {
        ReportPeriod::Day => "Daily",
        ReportPeriod::Week => "Weekly",
        ReportPeriod::Month => "Monthly",
        ReportPeriod::Year => "Yearly",
    };

    println!();
    println!(" {} {} {}", "📈", period_str.bold(), "Report".bold());
    println!(" {}", "═".repeat(50).dimmed());

    // By Activity breakdown if available
    if let Some(breakdown) = breakdown {
        if !breakdown.is_empty() {
            print_section("By Activity");
            let max_count = breakdown.iter().map(|(_, c)| *c).max().unwrap_or(1);
            for (activity, count) in breakdown.iter().take(5) {
                let ratio = *count as f64 / max_count as f64;
                let bar = progress_bar_colored(ratio, 20, BarColor::Cyan);
                println!("   {:<12} {} {:>3}x", activity.dimmed(), bar, count);
            }
        }
    }

    print_section("Totals");
    print_row(
        "Events",
        &report.total_events.to_string().bright_white().to_string(),
    );
    print_row(
        "Calories",
        &format!("{:.0} kcal", report.total_calories)
            .yellow()
            .to_string(),
    );
    print_row(
        "Active Time",
        &format_duration(report.total_active_minutes * 60.0)
            .cyan()
            .to_string(),
    );

    print_section("Daily Averages");
    print_row(
        "Calories",
        &format!("{:.0} kcal", report.avg_daily_calories)
            .yellow()
            .to_string(),
    );
    print_row(
        "Active Time",
        &format!("{:.0} min", report.avg_daily_active_minutes)
            .cyan()
            .to_string(),
    );

    println!();
}

/// Print events as a formatted table.
pub fn print_events_table(events: &[Event]) {
    if events.is_empty() {
        println!("\n {} {}\n", "ℹ️".dimmed(), "No events found".dimmed());
        return;
    }

    // Table constants
    let id_w = 8;
    let date_w = 14;
    let type_w = 14;
    let dur_w = 8;
    let tags_w = 30;

    // Header
    println!();
    println!(
        " {}{}{}{}{}{}{}{}{}{}",
        "┌".dimmed(),
        "─".repeat(id_w + 2).dimmed(),
        "┬".dimmed(),
        "─".repeat(date_w + 2).dimmed(),
        "┬".dimmed(),
        "─".repeat(type_w + 2).dimmed(),
        "┬".dimmed(),
        "─".repeat(dur_w + 2).dimmed(),
        "┬".dimmed(),
        "─".repeat(tags_w + 2).dimmed(),
    );
    println!(
        " {} {:<id_w$} {} {:<date_w$} {} {:<type_w$} {} {:>dur_w$} {} {:<tags_w$} {}",
        "│".dimmed(),
        "ID".bold(),
        "│".dimmed(),
        "Date".bold(),
        "│".dimmed(),
        "Type".bold(),
        "│".dimmed(),
        "Duration".bold(),
        "│".dimmed(),
        "Tags".bold(),
        "│".dimmed(),
    );
    println!(
        " {}{}{}{}{}{}{}{}{}{}",
        "├".dimmed(),
        "─".repeat(id_w + 2).dimmed(),
        "┼".dimmed(),
        "─".repeat(date_w + 2).dimmed(),
        "┼".dimmed(),
        "─".repeat(type_w + 2).dimmed(),
        "┼".dimmed(),
        "─".repeat(dur_w + 2).dimmed(),
        "┼".dimmed(),
        "─".repeat(tags_w + 2).dimmed(),
    );

    // Rows
    for event in events {
        let id = &event.id.to_string()[..8];

        let date = event
            .start_time
            .or(event.end_time)
            .map(|t| t.format("%b %d %H:%M").to_string())
            .unwrap_or_else(|| "—".into());

        let type_colored = format_event_type(&event.event_type);
        let type_plain = format_event_type_plain(&event.event_type);

        let dur = event
            .duration_secs()
            .map(|d| format_duration(d))
            .unwrap_or_else(|| "—".into());

        let tags = if event.tags.is_empty() {
            "—".dimmed().to_string()
        } else {
            let tag_str = event.tags.join(", ");
            if tag_str.len() > tags_w {
                format!("{}…", &tag_str[..tags_w - 1]).dimmed().to_string()
            } else {
                tag_str.dimmed().to_string()
            }
        };

        // Calculate padding for type column (accounting for ANSI codes)
        let type_padding = type_w.saturating_sub(type_plain.chars().count());
        let type_display = format!("{}{}", type_colored, " ".repeat(type_padding));

        println!(
            " {} {:<id_w$} {} {:<date_w$} {} {} {} {:>dur_w$} {} {:<tags_w$} {}",
            "│".dimmed(),
            id.bright_black(),
            "│".dimmed(),
            date,
            "│".dimmed(),
            type_display,
            "│".dimmed(),
            dur.bright_white(),
            "│".dimmed(),
            tags,
            "│".dimmed(),
        );
    }

    // Footer
    println!(
        " {}{}{}{}{}{}{}{}{}{}",
        "└".dimmed(),
        "─".repeat(id_w + 2).dimmed(),
        "┴".dimmed(),
        "─".repeat(date_w + 2).dimmed(),
        "┴".dimmed(),
        "─".repeat(type_w + 2).dimmed(),
        "┴".dimmed(),
        "─".repeat(dur_w + 2).dimmed(),
        "┴".dimmed(),
        "─".repeat(tags_w + 2).dimmed(),
    );

    println!(
        " {} {}\n",
        "📋".dimmed(),
        format!("{} event(s)", events.len()).dimmed()
    );
}

/// Print event summary for delete confirmation.
pub fn print_event_delete_confirm(event: &Event) {
    println!();
    println!(" {} {}", "⚠️".yellow(), "Event to delete:".yellow().bold());
    println!(" {}", "─".repeat(40).dimmed());

    print_row("ID", &event.id.to_string().bright_black().to_string());
    print_row("Type", &format_event_type(&event.event_type));

    if let Some(start) = event.start_time {
        print_row("Time", &start.format("%Y-%m-%d %H:%M").to_string());
    }
    if let Some(dur) = event.duration_secs() {
        print_row("Duration", &format_duration(dur));
    }
    if !event.tags.is_empty() {
        print_row("Tags", &event.tags.join(", "));
    }
    println!();
}

/// Pretty-print a metric key for display.
fn pretty_metric_name(key: &str) -> String {
    let base = key
        .strip_suffix("_m")
        .or_else(|| key.strip_suffix("_kg"))
        .or_else(|| key.strip_suffix("_ml"))
        .or_else(|| key.strip_suffix("_kcal"))
        .unwrap_or(key);

    base.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a metric value with appropriate display unit.
fn format_metric_value(key: &str, val: f64) -> String {
    if key.ends_with("_m") {
        if val.abs() >= 1000.0 {
            format!("{:.2} km", val / 1000.0)
        } else {
            format!("{:.0} m", val)
        }
    } else if key.ends_with("_kg") {
        if val.abs() < 0.001 {
            format!("{:.0} mg", val * 1_000_000.0)
        } else if val.abs() < 1.0 {
            format!("{:.1} g", val * 1000.0)
        } else {
            format!("{:.1} kg", val)
        }
    } else if key.ends_with("_ml") {
        if val.abs() >= 1000.0 {
            format!("{:.2} l", val / 1000.0)
        } else {
            format!("{:.0} ml", val)
        }
    } else if key.ends_with("_kcal") {
        format!("{:.0} kcal", val)
    } else {
        if val.fract() == 0.0 {
            format!("{:.0}", val)
        } else {
            format!("{val}")
        }
    }
}
