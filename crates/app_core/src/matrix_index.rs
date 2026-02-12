use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MatrixOrder {
    /// Iterate `fast_periods` in ascending order; for each fast, iterate `slow_periods` in ascending
    /// order, emitting only pairs where `fast < slow`.
    FastMajor,
}

/// Stable column indexing for the common “fast < slow” Double-MA grid.
///
/// This provides a deterministic mapping:
/// - `(fast_len, slow_len)` → `col_id`
/// - `col_id` → `(fast_len, slow_len)`
///
/// Column ordering is **FastMajor**:
/// `for fast in fast_periods { for slow in slow_periods { if fast < slow { emit }}}`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoubleMaFastLtSlowIndex {
    pub order: MatrixOrder,
    pub fast_periods: Vec<u16>,
    pub slow_periods: Vec<u16>,
    /// For each fast index `i`, the first slow index `j` such that `slow_periods[j] > fast_periods[i]`.
    pub slow_start_idx: Vec<u32>,
    /// For each fast index `i`, the starting column id for that fast period.
    pub fast_offsets: Vec<u32>,
    /// Total number of columns (valid fast<slow pairs).
    pub cols: u32,
}

impl DoubleMaFastLtSlowIndex {
    pub fn new(fast_periods: Vec<u16>, slow_periods: Vec<u16>) -> Result<Self, String> {
        validate_strictly_increasing_u16("fast_periods", &fast_periods)?;
        validate_strictly_increasing_u16("slow_periods", &slow_periods)?;

        let mut slow_start_idx: Vec<u32> = Vec::with_capacity(fast_periods.len());
        let mut fast_offsets: Vec<u32> = Vec::with_capacity(fast_periods.len());

        let slow_len_u32: u32 = slow_periods
            .len()
            .try_into()
            .map_err(|_| "slow_periods too long".to_string())?;

        let mut offset: u32 = 0;
        for &f in &fast_periods {
            fast_offsets.push(offset);
            let start = slow_periods.partition_point(|&s| s <= f);
            let start_u32: u32 = start
                .try_into()
                .map_err(|_| "slow_periods too long".to_string())?;
            slow_start_idx.push(start_u32);

            let count = slow_len_u32.saturating_sub(start_u32);
            offset = offset
                .checked_add(count)
                .ok_or_else(|| "cols overflow u32".to_string())?;
        }

        Ok(Self {
            order: MatrixOrder::FastMajor,
            fast_periods,
            slow_periods,
            slow_start_idx,
            fast_offsets,
            cols: offset,
        })
    }

    pub fn col_id(&self, fast_len: u16, slow_len: u16) -> Option<u32> {
        if fast_len >= slow_len {
            return None;
        }
        let i = self.fast_periods.binary_search(&fast_len).ok()?;
        let j = self.slow_periods.binary_search(&slow_len).ok()?;

        let start = *self.slow_start_idx.get(i)? as usize;
        if j < start {
            return None;
        }
        let within = j - start;
        let col = self.fast_offsets.get(i)?.saturating_add(within as u32);
        if col < self.cols {
            Some(col)
        } else {
            None
        }
    }

    pub fn params_for_col(&self, col: u32) -> Option<(u16, u16)> {
        if col >= self.cols {
            return None;
        }
        let i = self.fast_offsets.partition_point(|&off| off <= col);
        let fast_i = i.saturating_sub(1);
        let fast_len = *self.fast_periods.get(fast_i)?;
        let start = *self.slow_start_idx.get(fast_i)? as usize;

        let within = col.saturating_sub(*self.fast_offsets.get(fast_i)?);
        let slow_i = start.saturating_add(within as usize);
        let slow_len = *self.slow_periods.get(slow_i)?;
        if fast_len < slow_len {
            Some((fast_len, slow_len))
        } else {
            None
        }
    }
}

fn validate_strictly_increasing_u16(name: &str, xs: &[u16]) -> Result<(), String> {
    if xs.is_empty() {
        return Err(format!("{name} cannot be empty"));
    }
    for w in xs.windows(2) {
        if w[1] <= w[0] {
            return Err(format!("{name} must be strictly increasing"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_ma_index_maps_both_directions() {
        let idx = DoubleMaFastLtSlowIndex::new(vec![1, 2, 3], vec![1, 2, 3, 4]).unwrap();
        assert_eq!(idx.cols, 6);

        assert_eq!(idx.col_id(1, 2), Some(0));
        assert_eq!(idx.col_id(1, 3), Some(1));
        assert_eq!(idx.col_id(1, 4), Some(2));
        assert_eq!(idx.col_id(2, 3), Some(3));
        assert_eq!(idx.col_id(2, 4), Some(4));
        assert_eq!(idx.col_id(3, 4), Some(5));

        assert_eq!(idx.params_for_col(0), Some((1, 2)));
        assert_eq!(idx.params_for_col(2), Some((1, 4)));
        assert_eq!(idx.params_for_col(3), Some((2, 3)));
        assert_eq!(idx.params_for_col(5), Some((3, 4)));
    }

    #[test]
    fn double_ma_index_rejects_unsorted_axes() {
        let err = DoubleMaFastLtSlowIndex::new(vec![2, 1], vec![1, 2]).unwrap_err();
        assert!(err.contains("fast_periods") && err.contains("increasing"));
    }
}
