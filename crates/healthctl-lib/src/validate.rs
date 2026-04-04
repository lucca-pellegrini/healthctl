use anyhow::{bail, Result};

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
            if event.duration_secs.is_none()
                && (event.start_time.is_none() || event.end_time.is_none())
            {
                bail!("sleep event requires at least --duration or both --start and --end");
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
                || event.duration_secs.is_some()
                || event.start_time.is_some()
                || event.end_time.is_some()
                || !event.exercises.is_empty();
            if !has_data {
                bail!(
                    "event must have at least one of: time info, duration, metrics, or exercises"
                );
            }
        }
    }

    // Cross-field: if start > end, that's invalid.
    if let (Some(start), Some(end)) = (event.start_time, event.end_time) {
        if start > end {
            bail!("start time ({start}) is after end time ({end})");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventType};

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
    fn test_sleep_valid_with_duration() {
        let mut event = Event::new(EventType::Sleep);
        event.duration_secs = Some(28800.0);
        assert!(validate_event(&event).is_ok());
    }
}
