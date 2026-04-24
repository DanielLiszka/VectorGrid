pub const EXECUTION_QTY_SCALE: u64 = 10_000;
const DECIMAL_ROUNDING_PLACES: usize = 12;

pub fn normalize_execution_quantity(value: f64) -> Result<f64, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err("qty must be finite and > 0".to_string());
    }
    let units = execution_quantity_units(value)?;
    if units == 0 {
        return Err("qty is too small after normalization".to_string());
    }
    Ok((units as f64) / (EXECUTION_QTY_SCALE as f64))
}

pub fn execution_quantity_units(value: f64) -> Result<u64, String> {
    if !value.is_finite() || value <= 0.0 {
        return Err("qty must be finite and > 0".to_string());
    }
    let text = format!("{value:.precision$}", precision = DECIMAL_ROUNDING_PLACES);
    decimal_text_to_scaled_units(&text, EXECUTION_QTY_SCALE)
}

fn decimal_text_to_scaled_units(text: &str, scale: u64) -> Result<u64, String> {
    let scale_places = scale.ilog10() as usize;
    let trimmed = text.trim();
    let unsigned = trimmed
        .strip_prefix('+')
        .unwrap_or(trimmed)
        .strip_prefix('-')
        .unwrap_or_else(|| trimmed.strip_prefix('+').unwrap_or(trimmed));
    let (integer_part, fractional_part) = unsigned
        .split_once('.')
        .map_or((unsigned, ""), |(integer, fractional)| {
            (integer, fractional)
        });

    let integer_units = if integer_part.is_empty() {
        0
    } else {
        integer_part
            .parse::<u64>()
            .map_err(|_| "qty is too large after normalization".to_string())?
            .checked_mul(scale)
            .ok_or_else(|| "qty is too large after normalization".to_string())?
    };

    let mut fractional_units = 0_u64;
    let mut digits = fractional_part.bytes().filter(|b| b.is_ascii_digit());
    for _ in 0..scale_places {
        fractional_units = fractional_units
            .checked_mul(10)
            .ok_or_else(|| "qty is too large after normalization".to_string())?;
        if let Some(digit) = digits.next() {
            fractional_units += u64::from(digit - b'0');
        }
    }
    if digits.next().is_some_and(|digit| digit >= b'5') {
        fractional_units = fractional_units
            .checked_add(1)
            .ok_or_else(|| "qty is too large after normalization".to_string())?;
    }

    integer_units
        .checked_add(fractional_units)
        .ok_or_else(|| "qty is too large after normalization".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_midpoints_at_one_ten_thousandth_boundaries() {
        assert_eq!(normalize_execution_quantity(0.00005).unwrap(), 0.0001);
        assert_eq!(normalize_execution_quantity(0.000149).unwrap(), 0.0001);
        assert_eq!(normalize_execution_quantity(0.00015).unwrap(), 0.0002);
        assert_eq!(normalize_execution_quantity(1.23455).unwrap(), 1.2346);
    }

    #[test]
    fn rejects_invalid_and_too_small_quantities() {
        assert_eq!(
            normalize_execution_quantity(0.0).unwrap_err(),
            "qty must be finite and > 0"
        );
        assert_eq!(
            normalize_execution_quantity(f64::NAN).unwrap_err(),
            "qty must be finite and > 0"
        );
        assert_eq!(
            normalize_execution_quantity(0.00001).unwrap_err(),
            "qty is too small after normalization"
        );
    }
}
