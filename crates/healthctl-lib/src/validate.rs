use anyhow::{Result, bail};

use crate::event::{Event, EventType};

/// Validate an event before insertion.
/// Returns Ok(()) if valid, Err with explanation otherwise.
pub fn validate_event(event: &Event) -> Result<()> {
    match &event.event_type {
        EventType::Hydration => {
            if !event.metrics.contains_key("volume_ml") {
                bail!("hydration event requires an amount (e.g. 'healthctl add hydration 500ml')");
            }
        }
        EventType::Substance => {
            if !event.metrics.contains_key("amount_kg") {
                bail!(
                    "substance event requires an amount (e.g. 'healthctl add substance caffeine 200mg')"
                );
            }
        }
        EventType::Sleep => {
            // Sleep requires time information (start+end gives us duration).
            if event.start_time.is_none() && event.end_time.is_none() {
                bail!(
                    "sleep event requires at least --duration, --start, or --end with enough info to resolve times"
                );
            }
        }
        EventType::Nutrition => {
            if event.metrics.is_empty() {
                bail!("nutrition event requires at least one metric (e.g. --calories=650)");
            }
        }
        _ => {
            // For activity, strength, mental: at least one piece of data must exist.
            let has_data = !event.metrics.is_empty()
                || event.start_time.is_some()
                || event.end_time.is_some()
                || !event.exercises.is_empty();
            if !has_data {
                bail!("event must have at least one of: time info, metrics, or exercises");
            }
        }
    }

    // Cross-field: if start > end, that's invalid.
    if let (Some(start), Some(end)) = (event.start_time, event.end_time)
        && start > end
    {
        bail!("start time ({start}) is after end time ({end})");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventType};
    use chrono::Utc;

    #[test]
    fn test_hydration_requires_amount() {
        let event = Event::new(EventType::Hydration);
        assert!(validate_event(&event).is_err());
    }

    #[test]
    fn test_hydration_valid() {
        let mut event = Event::new(EventType::Hydration);
        event.metrics.insert("volume_ml".into(), 500.0);
        assert!(validate_event(&event).is_ok());
    }

    #[test]
    fn test_sleep_requires_time_info() {
        let event = Event::new(EventType::Sleep);
        assert!(validate_event(&event).is_err());
    }

    #[test]
    fn test_sleep_valid_with_times() {
        let mut event = Event::new(EventType::Sleep);
        let now = Utc::now();
        event.end_time = Some(now);
        event.start_time = Some(now - chrono::Duration::hours(8));
        assert!(validate_event(&event).is_ok());
    }

    #[test]
    fn test_sleep_valid_with_end_only() {
        // If user gave --duration + no explicit times, resolve_times sets both.
        // But if only --end was given (duration implicit from context), at least end exists.
        let mut event = Event::new(EventType::Sleep);
        event.end_time = Some(Utc::now());
        assert!(validate_event(&event).is_ok());
    }
}
