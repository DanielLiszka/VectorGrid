/// Returns the rounded-index quantile from an already-sorted ascending slice.
///
/// The input must already be sorted in the caller's intended order. `q` is
/// clamped into `[0.0, 1.0]`, empty input returns `0.0`, and the selection rule
/// is rounded-index rather than interpolated.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return if sorted[0].is_finite() {
            sorted[0]
        } else {
            0.0
        };
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    let value = sorted[idx.min(sorted.len() - 1)];
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// Returns the median from an already-sorted ascending slice.
///
/// For even-length slices, the current policy resolves to the upper middle
/// element because it delegates to the rounded-index `quantile(..., 0.5)` rule.
pub fn median(sorted: &[f64]) -> f64 {
    quantile(sorted, 0.5)
}

fn quantile_index(len: usize, q: f64) -> usize {
    let qq = q.clamp(0.0, 1.0);
    (qq * (len.saturating_sub(1) as f64)).round() as usize
}

/// Returns the rounded-index quantile from an unsorted slice in place.
///
/// This mutates the input slice but preserves the same selection rule as
/// [`quantile`], which means callers can avoid a full sort when they only need
/// one quantile.
pub fn quantile_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return if values[0].is_finite() {
            values[0]
        } else {
            0.0
        };
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, nth, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    if nth.is_finite() {
        *nth
    } else {
        0.0
    }
}

/// Returns the median from an unsorted slice in place.
pub fn median_in_place(values: &mut [f64]) -> f64 {
    quantile_in_place(values, 0.5)
}

/// Returns the mean of the lower `q` tail from an unsorted slice in place.
pub fn lower_tail_mean_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, _, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    let mut sum = 0.0;
    let mut count = 0usize;
    for value in values.iter().take(idx.saturating_add(1)) {
        if value.is_finite() {
            sum += *value;
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Returns the mean of the upper `q` tail from an unsorted slice in place.
pub fn upper_tail_mean_in_place(values: &mut [f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = quantile_index(values.len(), q).min(values.len() - 1);
    let (_, _, _) = values.select_nth_unstable_by(idx, |a, b| a.total_cmp(b));
    let mut sum = 0.0;
    let mut count = 0usize;
    for value in values.iter().skip(idx) {
        if value.is_finite() {
            sum += *value;
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Returns the minimum finite value, or `0.0` when the selected extreme is not finite.
pub fn worst_min(xs: &[f64]) -> f64 {
    let v = xs
        .iter()
        .copied()
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a });
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Returns the maximum finite value, or `0.0` when the selected extreme is not finite.
pub fn worst_max(xs: &[f64]) -> f64 {
    let v = xs
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a });
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

/// Returns the median of an unsorted `u32` slice.
///
/// For even-length slices, the current policy resolves to the upper middle
/// element after sorting.
pub fn median_u32(xs: &[u32]) -> u32 {
    if xs.is_empty() {
        return 0;
    }
    let mut ys = xs.to_vec();
    ys.sort_unstable();
    ys[ys.len() / 2]
}

/// Returns the median from an unsorted `u32` slice in place.
pub fn median_u32_in_place(values: &mut [u32]) -> u32 {
    if values.is_empty() {
        return 0;
    }
    let idx = values.len() / 2;
    let (_, nth, _) = values.select_nth_unstable(idx);
    *nth
}

/// Returns the minimum value from a `u32` slice, or `0` for empty input.
pub fn worst_min_u32(xs: &[u32]) -> u32 {
    xs.iter().copied().min().unwrap_or(0)
}

/// Returns the maximum value from a `u32` slice, or `0` for empty input.
pub fn worst_max_u32(xs: &[u32]) -> u32 {
    xs.iter().copied().max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_handles_bounds() {
        let xs = vec![1.0, 2.0, 3.0];
        assert_eq!(quantile(&xs, -1.0), 1.0);
        assert_eq!(quantile(&xs, 0.0), 1.0);
        assert_eq!(quantile(&xs, 0.5), 2.0);
        assert_eq!(quantile(&xs, 1.0), 3.0);
        assert_eq!(quantile(&xs, 2.0), 3.0);
        assert_eq!(quantile(&xs, f64::NAN), 1.0);
    }

    #[test]
    fn quantile_zeroes_non_finite_selected_values() {
        assert_eq!(quantile(&[f64::NAN], 0.5), 0.0);
        assert_eq!(quantile(&[f64::INFINITY], 0.5), 0.0);
        assert_eq!(quantile(&[1.0, f64::NAN, 3.0], 0.5), 0.0);
        assert_eq!(quantile(&[1.0, f64::INFINITY, 3.0], 0.5), 0.0);
    }

    #[test]
    fn median_even_length_uses_upper_middle_policy() {
        let xs = vec![1.0, 2.0, 3.0, 4.0];
        assert_eq!(median(&xs), 3.0);
    }

    #[test]
    fn quantile_in_place_matches_sorted_quantile() {
        let values: [f64; 5] = [4.0, 1.0, 3.0, 2.0, 5.0];
        for q in [0.0, 0.1, 0.5, 0.9, 1.0, f64::NAN] {
            let mut unsorted = values;
            let mut sorted = values;
            sorted.sort_by(|a, b| a.total_cmp(b));
            assert_eq!(quantile_in_place(&mut unsorted, q), quantile(&sorted, q));
        }
    }

    #[test]
    fn median_in_place_matches_sorted_median() {
        let values: [f64; 4] = [8.0, 2.0, 6.0, 4.0];
        let mut unsorted = values;
        let mut sorted = values;
        sorted.sort_by(|a, b| a.total_cmp(b));
        assert_eq!(median_in_place(&mut unsorted), median(&sorted));
    }

    #[test]
    fn tail_mean_helpers_match_expected_lower_and_upper_means() {
        let mut lower = [4.0, 1.0, 3.0, 2.0, 5.0];
        assert!((lower_tail_mean_in_place(&mut lower, 0.25) - 1.5).abs() <= 1.0e-12);

        let mut upper = [4.0, 1.0, 3.0, 2.0, 5.0];
        assert!((upper_tail_mean_in_place(&mut upper, 0.75) - 4.5).abs() <= 1.0e-12);
    }

    #[test]
    fn median_u32_even_length_uses_upper_middle_policy() {
        assert_eq!(median_u32(&[8, 2, 6, 4]), 6);
    }

    #[test]
    fn median_u32_in_place_matches_sorted_policy() {
        let mut values = [8, 2, 6, 4];
        assert_eq!(median_u32_in_place(&mut values), median_u32(&[8, 2, 6, 4]));
    }

    #[test]
    fn worst_max_u32_returns_maximum_or_zero() {
        assert_eq!(worst_max_u32(&[8, 2, 6, 4]), 8);
        assert_eq!(worst_max_u32(&[]), 0);
    }

    #[test]
    fn worst_helpers_handle_non_finite() {
        assert_eq!(worst_min(&[f64::NAN]), 0.0);
        assert_eq!(worst_max(&[f64::NAN]), 0.0);
    }

    #[test]
    fn worst_helpers_prefer_finite_extremes_over_nan_entries() {
        let xs = [1.5, f64::NAN, -2.0, 4.25];
        assert_eq!(worst_min(&xs), -2.0);
        assert_eq!(worst_max(&xs), 4.25);
    }

    #[test]
    fn worst_helpers_zero_on_non_finite_selected_extreme() {
        assert_eq!(worst_min(&[1.0, f64::NEG_INFINITY]), 0.0);
        assert_eq!(worst_max(&[1.0, f64::INFINITY]), 0.0);
    }

    #[test]
    fn worst_helpers_ignore_opposite_direction_infinity() {
        assert_eq!(worst_min(&[1.0, f64::INFINITY]), 1.0);
        assert_eq!(worst_max(&[1.0, f64::NEG_INFINITY]), 1.0);
    }
}
