use anyhow::{bail, Result};

/// Parse a value with a unit suffix and return the normalized SI value.
/// Supported dimensions:
///   - Distance: km, m, mi, miles, ft, yards → meters
///   - Weight/mass: kg, g, lb, lbs, oz → kg
///   - Volume: ml, l, floz, cups → ml
///   - Energy: kcal, cal, kj → kcal
///   - Generic counts: steps, reps, sets → raw number
///
/// Returns (normalized_value, canonical_unit_name).
pub fn parse_metric(input: &str) -> Result<(f64, &'static str)> {
    let input = input.trim();

    // Try to split into numeric part and unit suffix.
    let (num_str, unit) = split_number_unit(input)?;
    let value: f64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid number: '{num_str}'"))?;

    match unit {
        // Distance → meters
        "km" => Ok((value * 1000.0, "meters")),
        "m" => Ok((value, "meters")),
        "mi" | "miles" | "mile" => Ok((value * 1609.344, "meters")),
        "ft" | "feet" => Ok((value * 0.3048, "meters")),
        "yd" | "yards" => Ok((value * 0.9144, "meters")),

        // Weight → kg
        "kg" => Ok((value, "kg")),
        "g" => Ok((value / 1000.0, "kg")),
        "lb" | "lbs" => Ok((value * 0.453592, "kg")),
        "oz" => Ok((value * 0.0283495, "kg")),

        // Volume → ml
        "ml" => Ok((value, "ml")),
        "l" | "L" => Ok((value * 1000.0, "ml")),
        "floz" | "fl_oz" => Ok((value * 29.5735, "ml")),
        "cups" | "cup" => Ok((value * 236.588, "ml")),

        // Energy → kcal
        "kcal" | "cal" => Ok((value, "kcal")),
        "kj" => Ok((value / 4.184, "kcal")),

        // Unitless metrics (user just provides a number with implicit unit from context)
        "" => Ok((value, "raw")),

        _ => bail!("unknown unit: '{unit}'"),
    }
}

/// Parse a volume amount (e.g. "500ml", "1.5l") → milliliters.
pub fn parse_volume(input: &str) -> Result<f64> {
    let (val, unit) = parse_metric(input)?;
    if unit != "ml" {
        bail!("expected a volume unit (ml, l, floz, cups), got '{input}'");
    }
    Ok(val)
}

/// Parse a weight/mass (e.g. "200mg", "1g", "52kg") → kg.
pub fn parse_mass(input: &str) -> Result<f64> {
    let input = input.trim();
    let (num_str, unit) = split_number_unit(input)?;
    let value: f64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid number: '{num_str}'"))?;

    match unit {
        "kg" => Ok(value),
        "g" => Ok(value / 1000.0),
        "mg" => Ok(value / 1_000_000.0),
        "lb" | "lbs" => Ok(value * 0.453592),
        "oz" => Ok(value * 0.0283495),
        _ => bail!("expected a mass unit (kg, g, mg, lb, oz), got '{input}'"),
    }
}

/// Split "7.2km" into ("7.2", "km").
fn split_number_unit(input: &str) -> Result<(&str, &str)> {
    // Find where digits/dots/minus end and letters begin.
    let unit_start = input
        .find(|c: char| c.is_alphabetic() || c == '_')
        .unwrap_or(input.len());

    let num_str = &input[..unit_start];
    let unit = &input[unit_start..];

    if num_str.is_empty() {
        bail!("no numeric value found in '{input}'");
    }

    Ok((num_str, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_km() {
        let (val, unit) = parse_metric("5.2km").unwrap();
        assert_eq!(unit, "meters");
        assert!((val - 5200.0).abs() < 0.01);
    }

    #[test]
    fn test_weight_lb() {
        let (val, unit) = parse_metric("150lbs").unwrap();
        assert_eq!(unit, "kg");
        assert!((val - 68.039).abs() < 0.01);
    }

    #[test]
    fn test_volume_ml() {
        let val = parse_volume("500ml").unwrap();
        assert!((val - 500.0).abs() < 0.01);
    }

    #[test]
    fn test_volume_liters() {
        let val = parse_volume("1.5l").unwrap();
        assert!((val - 1500.0).abs() < 0.01);
    }

    #[test]
    fn test_mass_mg() {
        let val = parse_mass("200mg").unwrap();
        assert!((val - 0.0002).abs() < 0.0000001);
    }

    #[test]
    fn test_unitless_rejected_for_duration() {
        // "15" alone with no unit should parse as raw but context should reject it
        let (val, unit) = parse_metric("15").unwrap();
        assert_eq!(unit, "raw");
        assert!((val - 15.0).abs() < 0.01);
    }
}
