use anyhow::{bail, Result};
use chrono::{DateTime, Local, NaiveTime, TimeZone, Utc};

/// Parse a duration string into seconds.
/// Accepted formats:
///   - humantime style: "1h15m", "45m", "2h30m5s", "50m"
///   - HH:MM or HH:MM:SS: "00:15", "01:30:00"
///   - float with unit: "0.25h", "1.5h", "90m"
///   - bare number is REJECTED (must have unit)
pub fn parse_duration(input: &str) -> Result<f64> {
    let input = input.trim();

    if input.is_empty() {
        bail!("empty duration string");
    }

    // Reject bare numbers with no unit.
    if input.parse::<f64>().is_ok() {
        bail!("duration must include a unit (e.g. '15m', '1h', '00:15'), got '{input}'");
    }

    // Try HH:MM or HH:MM:SS format.
    if input.contains(':') {
        let parts: Vec<&str> = input.split(':').collect();
        match parts.len() {
            2 => {
                let h: f64 = parts[0].parse()?;
                let m: f64 = parts[1].parse()?;
                return Ok(h * 3600.0 + m * 60.0);
            }
            3 => {
                let h: f64 = parts[0].parse()?;
                let m: f64 = parts[1].parse()?;
                let s: f64 = parts[2].parse()?;
                return Ok(h * 3600.0 + m * 60.0 + s);
            }
            _ => bail!("invalid duration format: '{input}'"),
        }
    }

    // Try humantime parsing.
    if let Ok(std_dur) = humantime::parse_duration(input) {
        return Ok(std_dur.as_secs_f64());
    }

    // Try float with single unit suffix: "0.25h", "1.5m"
    if let Some(stripped) = input.strip_suffix('h') {
        let val: f64 = stripped.parse()?;
        return Ok(val * 3600.0);
    }
    if let Some(stripped) = input.strip_suffix('m') {
        let val: f64 = stripped.parse()?;
        return Ok(val * 60.0);
    }
    if let Some(stripped) = input.strip_suffix('s') {
        let val: f64 = stripped.parse()?;
        return Ok(val);
    }

    bail!("cannot parse duration: '{input}'")
}

/// Parse a time/datetime string into a UTC DateTime.
/// Accepted formats:
///   - "now" → current time
///   - "HH:MM" or "H:MMAM/PM" → most recent occurrence (today or yesterday, never future)
///   - ISO 8601: "2026-05-10T08:00" etc.
///   - Date + time: "2026-05-10 08:00"
pub fn parse_datetime(input: &str) -> Result<DateTime<Utc>> {
    let input = input.trim();

    if input.eq_ignore_ascii_case("now") {
        return Ok(Utc::now());
    }

    // Try full ISO 8601 / RFC 3339.
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt.with_timezone(&Utc));
    }

    // Try "YYYY-MM-DDTHH:MM" (no timezone, assume local).
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M") {
        return Ok(local_naive_to_utc(dt));
    }

    // Try "YYYY-MM-DD HH:MM".
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M") {
        return Ok(local_naive_to_utc(dt));
    }

    // Try time-only formats → resolve to most recent occurrence.
    if let Some(time) = try_parse_time(input) {
        let dt = resolve_most_recent(time);
        return Ok(dt);
    }

    bail!("cannot parse datetime: '{input}'")
}

/// Try parsing a time-only string.
/// Supports: "17:30", "5:30PM", "5:30pm", "05:30", "5:30 PM"
fn try_parse_time(input: &str) -> Option<NaiveTime> {
    // 24h format: HH:MM
    if let Ok(t) = NaiveTime::parse_from_str(input, "%H:%M") {
        return Some(t);
    }
    // 12h format: "5:30PM" / "5:30 PM"
    let normalized = input.replace(' ', "").to_uppercase();
    if let Ok(t) = NaiveTime::parse_from_str(&normalized, "%I:%M%p") {
        return Some(t);
    }
    // HH:MM:SS
    if let Ok(t) = NaiveTime::parse_from_str(input, "%H:%M:%S") {
        return Some(t);
    }
    None
}

/// Given a time, find the most recent occurrence (today or yesterday, never future).
fn resolve_most_recent(time: NaiveTime) -> DateTime<Utc> {
    let local_now = Local::now();
    let today = local_now.date_naive();

    let candidate = today.and_time(time);
    let candidate_local = Local.from_local_datetime(&candidate).single();

    if let Some(local_dt) = candidate_local {
        if local_dt <= local_now {
            return local_dt.with_timezone(&Utc);
        }
    }

    // Use yesterday.
    let yesterday = today - chrono::Duration::days(1);
    let candidate = yesterday.and_time(time);
    Local
        .from_local_datetime(&candidate)
        .single()
        .expect("valid local time")
        .with_timezone(&Utc)
}

fn local_naive_to_utc(dt: chrono::NaiveDateTime) -> DateTime<Utc> {
    Local
        .from_local_datetime(&dt)
        .single()
        .expect("valid local time")
        .with_timezone(&Utc)
}

/// Parse relative date expressions for queries: "today", "yesterday", "7 days", etc.
/// Returns a (from, to) UTC range.
pub fn parse_date_range(input: &str) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let input = input.trim().to_lowercase();
    let now = Local::now();
    let today_start = now
        .date_naive()
        .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let today_start_utc = Local
        .from_local_datetime(&today_start)
        .single()
        .unwrap()
        .with_timezone(&Utc);

    match input.as_str() {
        "today" => Ok((today_start_utc, Utc::now())),
        "yesterday" => {
            let yesterday = today_start_utc - chrono::Duration::days(1);
            Ok((yesterday, today_start_utc))
        }
        _ => {
            // Try "N days" pattern.
            if let Some(rest) = input.strip_suffix("days").or(input.strip_suffix("day")) {
                let n: i64 = rest.trim().parse()?;
                let from = today_start_utc - chrono::Duration::days(n);
                return Ok((from, Utc::now()));
            }
            bail!("cannot parse date range: '{input}'")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_humantime() {
        assert!((parse_duration("1h15m").unwrap() - 4500.0).abs() < 0.01);
        assert!((parse_duration("45m").unwrap() - 2700.0).abs() < 0.01);
        assert!((parse_duration("2h30m5s").unwrap() - 9005.0).abs() < 0.01);
    }

    #[test]
    fn test_duration_colon() {
        assert!((parse_duration("00:15").unwrap() - 900.0).abs() < 0.01);
        assert!((parse_duration("01:30:00").unwrap() - 5400.0).abs() < 0.01);
    }

    #[test]
    fn test_duration_float() {
        assert!((parse_duration("0.25h").unwrap() - 900.0).abs() < 0.01);
        assert!((parse_duration("1.5h").unwrap() - 5400.0).abs() < 0.01);
    }

    #[test]
    fn test_duration_bare_number_rejected() {
        assert!(parse_duration("15").is_err());
    }

    #[test]
    fn test_datetime_now() {
        let dt = parse_datetime("now").unwrap();
        let diff = (Utc::now() - dt).num_seconds().abs();
        assert!(diff < 2);
    }

    #[test]
    fn test_datetime_iso() {
        let dt = parse_datetime("2026-05-10T08:00").unwrap();
        assert_eq!(
            dt.date_naive(),
            chrono::NaiveDate::from_ymd_opt(2026, 5, 10).unwrap()
        );
    }
}
