//! Pretty-printing utilities with ANSI colors, emojis, and Unicode formatting.

use healthctl_lib::event::{ActivityKind, Event, EventType, MentalKind};
use healthctl_lib::ipc::{Breakdown, ReportData, ReportPeriod, StatusSummary};
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

/// Display width of a string, accounting for East Asian Width (emojis count as 2).
/// Note: this operates on the *plain* (un-colored) text — never pass ANSI codes here.
pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Truncate a plain string to at most `max` display columns, appending `…` if cut.
/// The ellipsis itself counts toward the width budget.
pub fn truncate_to_width(s: &str, max: usize) -> String {
    if display_width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    // Reserve one column for the ellipsis.
    let budget = max.saturating_sub(1);
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        if w + cw > budget {
            break;
        }
        out.push(ch);
        w += cw;
    }
    out.push('…');
    out
}

/// Right-pad a *colored* cell to a target display width, given its plain text
/// (used only to measure width, since ANSI codes have zero display width).
pub fn pad_right(colored: &str, plain: &str, width: usize) -> String {
    let pad = width.saturating_sub(display_width(plain));
    format!("{}{}", colored, " ".repeat(pad))
}

/// Left-pad a *colored* cell to a target display width, given its plain text.
pub fn pad_left(colored: &str, plain: &str, width: usize) -> String {
    let pad = width.saturating_sub(display_width(plain));
    format!("{}{}", " ".repeat(pad), colored)
}

/// Get emoji for event type.
///
/// Thin wrapper over [`EventType::emoji`] so existing call sites keep working.
pub fn event_emoji(event_type: &EventType) -> &'static str {
    event_type.emoji()
}

/// Format event type as a colored, pretty string.
pub fn format_event_type(event_type: &EventType) -> String {
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
    }
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

/// A small palette for colored progress bars. Not all variants are used by
/// every view; they form a complete, reusable palette.
#[derive(Clone, Copy)]
#[allow(dead_code)]
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

/// Identifies one of the six report cards (mirrors the dashboard stat cards).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReportCard {
    Steps,
    Calories,
    Distance,
    Active,
    Sleep,
    Workouts,
}

/// Print a report: the six-card overview, followed by any requested detail
/// cards (in `cards` order). With no detail cards, only the overview prints.
pub fn print_report(report: &ReportData, cards: &[ReportCard]) {
    let period_str = match report.period {
        ReportPeriod::Day => "Daily",
        ReportPeriod::Week => "Weekly",
        ReportPeriod::Month => "Monthly",
        ReportPeriod::Year => "Yearly",
    };

    println!();
    println!(
        " 📈 {} {}   {}",
        period_str.bold(),
        "Report".bold(),
        report.range_label.dimmed()
    );
    println!(" {}", "═".repeat(56).dimmed());

    print_overview(report);

    for card in cards {
        match card {
            ReportCard::Steps => print_steps_detail(report),
            ReportCard::Calories => print_calories_detail(report),
            ReportCard::Distance => print_distance_detail(report),
            ReportCard::Active => print_active_detail(report),
            ReportCard::Sleep => print_sleep_detail(report),
            ReportCard::Workouts => print_workouts_detail(report),
        }
    }

    if cards.is_empty() {
        println!(
            "\n {}",
            "Tip: add -S/-c/-d/-A/-s/-w for details, or -a for all.".dimmed()
        );
    }
    println!();
}

/// The six-card overview grid (two columns of three rows).
fn print_overview(report: &ReportData) {
    let sleep_str = report
        .sleep
        .avg_hours
        .map(|h| format!("{h:.1}"))
        .unwrap_or_else(|| "—".into());

    // (emoji, value, label) for each card.
    let cells: [(&str, String, &str); 6] = [
        ("👣", fmt_int(report.steps.total), "Steps"),
        ("🔥", fmt_int(report.calories.total), "Calories"),
        ("📏", format!("{:.1}", report.distance.total_km), "km"),
        ("⏱️", fmt_int(report.active.total_minutes), "Active min"),
        ("😴", sleep_str, "Avg Sleep"),
        ("💪", report.workouts.count.to_string(), "Workouts"),
    ];

    println!();
    // Render three rows of two cards.
    for row in cells.chunks(2) {
        let mut line = String::from("  ");
        for (emoji, value, label) in row {
            let cell_plain = format!("{emoji} {}  {}", value, label);
            let colored = format!(
                "{} {}  {}",
                emoji,
                value.bold().bright_white(),
                label.dimmed()
            );
            line.push_str(&pad_right(&colored, &cell_plain, 28));
        }
        println!("{}", line.trim_end());
    }
}

/// Print a labeled detail section header (a card title).
fn print_card_title(emoji: &str, title: &str) {
    println!("\n {} {}", emoji, title.bold());
    println!(" {}", "─".repeat(48).dimmed());
}

/// Print a comparison row like "vs Last Week  +7.7%".
fn print_comparison(label: &str, pct: f64) {
    print_row(label, &format_percentage(pct));
}

/// Render a "By Activity" breakdown with proportional Unicode bars.
fn print_breakdown(items: &[Breakdown], color: BarColor, fmt_value: impl Fn(f64) -> String) {
    if items.is_empty() {
        return;
    }
    print_section("By Activity");
    let max = items
        .iter()
        .map(|b| b.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    // Align labels and values by display width.
    let label_w = items
        .iter()
        .map(|b| display_width(&b.label))
        .max()
        .unwrap_or(0)
        .max(8);
    let val_strs: Vec<String> = items.iter().map(|b| fmt_value(b.value)).collect();
    let val_w = val_strs.iter().map(|s| display_width(s)).max().unwrap_or(0);

    for (b, val) in items.iter().zip(val_strs.iter()) {
        let ratio = b.value / max;
        let bar = progress_bar_colored(ratio, 18, color);
        let label_cell = pad_right(&b.label.dimmed().to_string(), &b.label, label_w);
        let val_cell = pad_left(&val.bright_white().to_string(), val, val_w);
        println!("   {label_cell}  {bar}  {val_cell}");
    }
}

fn print_steps_detail(report: &ReportData) {
    let s = &report.steps;
    print_card_title("👣", "Steps");
    print_section("Statistics");
    print_row("Total", &fmt_int(s.total).bright_white().to_string());
    print_row("Daily Average", &fmt_int(s.daily_avg).cyan().to_string());
    if let Some(ref day) = s.best_day {
        print_row(
            "Best Day",
            &format!("{} ({})", day, fmt_int(s.best_day_value)),
        );
    }
    print_section("Comparisons");
    print_comparison("vs Previous", s.vs_previous);
    print_comparison("vs Average", s.vs_average);
    if let Some(p) = s.projection {
        print_section("Projection");
        print_row("End of Period", &fmt_int(p).yellow().to_string());
    }
}

fn print_calories_detail(report: &ReportData) {
    let c = &report.calories;
    print_card_title("🔥", "Calories");
    print_breakdown(&c.by_activity, BarColor::Yellow, |v| format!("{v:.0} kcal"));
    print_section("Statistics");
    print_row(
        "Total",
        &format!("{:.0} kcal", c.total).bright_white().to_string(),
    );
    print_row(
        "Daily Average",
        &format!("{:.0} kcal", c.daily_avg).cyan().to_string(),
    );
    if let Some(ref day) = c.best_day {
        print_row(
            "Best Day",
            &format!("{} ({:.0} kcal)", day, c.best_day_value),
        );
    }
    print_section("Comparisons");
    print_comparison("vs Previous", c.vs_previous);
    if let Some(p) = c.projection {
        print_section("Projection");
        print_row(
            "End of Period",
            &format!("{p:.0} kcal").yellow().to_string(),
        );
    }
}

fn print_distance_detail(report: &ReportData) {
    let d = &report.distance;
    print_card_title("📏", "Distance");
    print_breakdown(&d.by_activity, BarColor::Cyan, |v| format!("{v:.1} km"));
    print_section("Statistics");
    print_row(
        "Total",
        &format!("{:.1} km", d.total_km).bright_white().to_string(),
    );
    if let Some(ref day) = d.best_day {
        print_row("Best Day", &format!("{} ({:.1} km)", day, d.best_day_value));
    }
    print_section("Comparisons");
    print_comparison("vs Previous", d.vs_previous);
    if let Some(p) = d.projection {
        print_section("Projection");
        print_row("End of Period", &format!("{p:.1} km").yellow().to_string());
    }
}

fn print_active_detail(report: &ReportData) {
    let a = &report.active;
    print_card_title("⏱️", "Active Time");
    print_breakdown(&a.by_activity, BarColor::Green, |v| {
        format_duration(v * 60.0)
    });
    print_section("Statistics");
    print_row(
        "Total",
        &format_duration(a.total_minutes * 60.0)
            .bright_white()
            .to_string(),
    );
    print_row(
        "Daily Average",
        &format!("{:.0} min", a.daily_avg).cyan().to_string(),
    );
    if let Some(ref day) = a.most_active_day {
        print_row("Most Active Day", day);
    }
    print_section("Comparisons");
    print_comparison("vs Previous", a.vs_previous);
    if let Some(p) = a.projection {
        print_section("Projection");
        print_row(
            "End of Period",
            &format_duration(p * 60.0).yellow().to_string(),
        );
    }
}

fn print_sleep_detail(report: &ReportData) {
    let s = &report.sleep;
    print_card_title("😴", "Sleep");
    if !s.nights.is_empty() {
        print_section("Sleep Log");
        // Show up to 7 most-recent nights with a small bar (scaled to 10h).
        for night in s.nights.iter().take(7) {
            let ratio = (night.hours / 10.0).min(1.0);
            let bar = progress_bar_colored(ratio, 14, BarColor::Blue);
            let q = night.quality.map(|q| format!(" q{q}")).unwrap_or_default();
            let date_cell = pad_right(&night.date, &night.date, 9);
            println!(
                "   {date_cell}  {bar}  {}{}",
                format!("{:.1}h", night.hours).bright_white(),
                q.dimmed()
            );
        }
    }
    print_section("Statistics");
    if let Some(avg) = s.avg_hours {
        print_row(
            "Avg / Night",
            &format!("{avg:.1}h").bright_white().to_string(),
        );
    }
    if let Some(ref n) = s.best_night {
        print_row("Best Night", &format!("{} ({:.1}h)", n, s.best_night_hours));
    }
    if let Some(ref n) = s.worst_night {
        print_row(
            "Worst Night",
            &format!("{} ({:.1}h)", n, s.worst_night_hours),
        );
    }
    if s.avg_quality > 0.0 {
        print_row("Avg Quality", &format!("{:.1}/10", s.avg_quality));
    }
    print_section("Comparisons");
    print_comparison("vs Previous", s.vs_previous);
}

fn print_workouts_detail(report: &ReportData) {
    let w = &report.workouts;
    print_card_title("💪", "Workouts");
    print_breakdown(&w.by_type, BarColor::Purple, |v| format!("{v:.0}x"));
    print_section("Statistics");
    print_row("Count", &w.count.to_string().bright_white().to_string());
    print_row(
        "Total Duration",
        &format_duration(w.total_duration * 60.0).cyan().to_string(),
    );
    print_row(
        "Average Duration",
        &format!("{:.0} min", w.avg_duration).cyan().to_string(),
    );
    print_section("Comparisons");
    print_comparison("vs Previous", w.vs_previous);
    if !w.muscle_groups.is_empty() {
        print_section("Muscle Groups Worked");
        print!("   ");
        let badges: Vec<String> = w
            .muscle_groups
            .iter()
            .map(|m| {
                let label = capitalize(m);
                format!(" {} ", label)
                    .black()
                    .on_bright_magenta()
                    .to_string()
            })
            .collect();
        println!("{}", badges.join(" "));
    }
}

/// Format a floating count as a thousands-grouped integer, e.g. 56904 → "56,904".
fn fmt_int(v: f64) -> String {
    let n = v.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// Capitalize the first letter of a word.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

/// Print events as a formatted table.
pub fn print_events_table(events: &[Event]) {
    if events.is_empty() {
        println!("\n {} {}\n", "ℹ️".dimmed(), "No events found".dimmed());
        return;
    }

    // Column display widths.
    let widths = [8usize, 14, 14, 8, 30]; // ID, Date, Type, Duration, Tags

    // Header
    println!();
    print_table_border(&widths, '┌', '┬', '┐');
    let headers = ["ID", "Date", "Type", "Duration", "Tags"];
    let header_cells: Vec<String> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            // Duration is right-aligned, the rest left-aligned.
            if i == 3 {
                pad_left(&h.bold().to_string(), h, widths[i])
            } else {
                pad_right(&h.bold().to_string(), h, widths[i])
            }
        })
        .collect();
    print_table_row(&header_cells);
    print_table_border(&widths, '├', '┼', '┤');

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
            .map(format_duration)
            .unwrap_or_else(|| "—".into());

        // Tags: truncate by display width so the cell never exceeds the column.
        let (tags_colored, tags_plain) = if event.tags.is_empty() {
            ("—".dimmed().to_string(), "—".to_string())
        } else {
            let tag_str = truncate_to_width(&event.tags.join(", "), widths[4]);
            (tag_str.dimmed().to_string(), tag_str)
        };

        // Every column is padded by *display width* (emojis count as 2 columns),
        // measuring against the plain (un-colored) text.
        let cells = [
            pad_right(&id.bright_black().to_string(), id, widths[0]),
            pad_right(&date, &date, widths[1]),
            pad_right(&type_colored, &type_plain, widths[2]),
            pad_left(&dur.bright_white().to_string(), &dur, widths[3]),
            pad_right(&tags_colored, &tags_plain, widths[4]),
        ];
        print_table_row(&cells);
    }

    // Footer
    print_table_border(&widths, '└', '┴', '┘');

    println!(
        " {} {}\n",
        "📋".dimmed(),
        format!("{} event(s)", events.len()).dimmed()
    );
}

/// Print a horizontal table border given column widths and the corner/junction chars.
fn print_table_border(widths: &[usize], left: char, mid: char, right: char) {
    let mut line = String::new();
    line.push(left);
    for (i, w) in widths.iter().enumerate() {
        line.push_str(&"─".repeat(w + 2));
        if i + 1 < widths.len() {
            line.push(mid);
        }
    }
    line.push(right);
    println!(" {}", line.dimmed());
}

/// Print a single table row from pre-padded cells.
fn print_table_row(cells: &[String]) {
    let mut out = format!(" {}", "│".dimmed());
    for cell in cells {
        out.push_str(&format!(" {} {}", cell, "│".dimmed()));
    }
    println!("{out}");
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
    } else if val.fract() == 0.0 {
        format!("{:.0}", val)
    } else {
        format!("{val}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn test_display_width_emoji_double() {
        // Emojis render as two columns in most terminals.
        assert_eq!(display_width("🚶"), 2);
        assert_eq!(display_width("💪"), 2);
        // "💪 Strength" = 2 (emoji) + 1 (space) + 8 (Strength) = 11
        assert_eq!(display_width("💪 Strength"), 11);
    }

    #[test]
    fn test_truncate_to_width_no_cut() {
        assert_eq!(truncate_to_width("daily", 30), "daily");
    }

    #[test]
    fn test_truncate_to_width_cuts_with_ellipsis() {
        let s = "glutes, gym, hamstrings, legs, quads";
        let out = truncate_to_width(s, 30);
        // Must never exceed the column width.
        assert!(display_width(&out) <= 30);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn test_truncate_to_width_multibyte_safe() {
        // Should not panic on a multibyte boundary.
        let s = "área, ção, naïve, café";
        let out = truncate_to_width(s, 10);
        assert!(display_width(&out) <= 10);
    }

    #[test]
    fn test_pad_right_accounts_for_emoji() {
        // "🚶 Walk" has display width 7 (2 + 1 + 4); padding to 14 adds 7 spaces.
        let plain = "🚶 Walk";
        let padded = pad_right(plain, plain, 14);
        // The padded plain text should have display width exactly 14.
        assert_eq!(display_width(&padded), 14);
    }

    #[test]
    fn test_pad_left_accounts_for_width() {
        let plain = "1h 04m";
        let padded = pad_left(plain, plain, 8);
        assert_eq!(display_width(&padded), 8);
    }
}
