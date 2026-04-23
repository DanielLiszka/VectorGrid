use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndicatorMatrixF64 {
    pub rows: usize,
    pub cols: usize,
    pub values: Vec<f64>,
}

impl IndicatorMatrixF64 {
    pub fn try_new(rows: usize, cols: usize, values: Vec<f64>) -> Result<Self, String> {
        let expected = rows
            .checked_mul(cols)
            .ok_or_else(|| "rows*cols overflow".to_string())?;
        if values.len() != expected {
            return Err(format!(
                "matrix values length {} != expected {}",
                values.len(),
                expected
            ));
        }
        Ok(Self { rows, cols, values })
    }

    pub fn row(&self, row: usize) -> Option<&[f64]> {
        if row >= self.rows || self.cols == 0 {
            return None;
        }
        let off = row.saturating_mul(self.cols);
        self.values.get(off..off.saturating_add(self.cols))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovingAverageBatchRequest {
    pub ma_type: String,
    pub ma_source: String,
    pub periods: Vec<u16>,
    pub ma_params: Option<HashMap<String, f64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovingAverageBatchResponse {
    pub ma_type: String,
    pub ma_source: String,
    pub periods: Vec<u16>,
    pub matrix: IndicatorMatrixF64,
}

fn validate_uniform_step(periods: &[u16]) -> Result<(), String> {
    if periods.is_empty() {
        return Err("periods cannot be empty".to_string());
    }
    if periods.len() < 2 {
        return Ok(());
    }

    for w in periods.windows(2) {
        if w[1] <= w[0] {
            return Err("periods must be strictly increasing".to_string());
        }
    }

    let step = periods[1].saturating_sub(periods[0]);
    if step == 0 {
        return Err("periods must be strictly increasing".to_string());
    }
    for i in 2..periods.len() {
        if periods[i].saturating_sub(periods[i - 1]) != step {
            return Err("periods must have a uniform step".to_string());
        }
    }
    Ok(())
}

/// Minimal “IndicatorFactory-like” entrypoint for VectorTA moving averages:
/// compute an MA matrix for a (uniform-step) period grid.
pub fn compute_moving_average_batch_cpu(
    candles: &Candles,
    req: MovingAverageBatchRequest,
) -> Result<MovingAverageBatchResponse, String> {
    let ma_type = req.ma_type.trim().to_ascii_lowercase();
    let ma_source = req.ma_source.trim().to_string();
    if ma_type.is_empty() {
        return Err("ma_type cannot be empty".to_string());
    }
    if ma_source.is_empty() {
        return Err("ma_source cannot be empty".to_string());
    }

    validate_uniform_step(&req.periods)?;

    let (alma_offset, alma_sigma) =
        crate::double_ma::resolve_alma_params_for_side("ma", &ma_type, req.ma_params.as_ref())?;

    let (values, cols) = crate::double_ma::ma_matrix_cpu_f64(
        candles,
        &ma_type,
        &ma_source,
        &req.periods,
        req.ma_params.as_ref(),
        alma_offset,
        alma_sigma,
    )?;

    let matrix = IndicatorMatrixF64::try_new(req.periods.len(), cols, values)?;
    Ok(MovingAverageBatchResponse {
        ma_type,
        ma_source,
        periods: req.periods,
        matrix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trending_candles(len: usize) -> Candles {
        let len = len.max(2);
        let ts: Vec<i64> = (0..len as i64).collect();
        let mut close: Vec<f64> = Vec::with_capacity(len);
        for i in 0..len {
            close.push(100.0 + i as f64);
        }
        let open = close.clone();
        let high = close.clone();
        let low = close.clone();
        let vol = vec![0.0; len];
        Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn compute_moving_average_batch_cpu_smoke() {
        let candles = make_trending_candles(16);
        let req = MovingAverageBatchRequest {
            ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            periods: vec![2, 3, 4],
            ma_params: None,
        };
        let out = compute_moving_average_batch_cpu(&candles, req).unwrap();
        assert_eq!(out.matrix.rows, 3);
        assert_eq!(out.matrix.cols, 16);
        assert_eq!(out.matrix.values.len(), 3 * 16);
        assert!(out.matrix.row(0).unwrap()[10].is_finite());
        assert!(out.matrix.row(2).unwrap()[10].is_finite());
    }

    #[test]
    fn moving_average_batch_rejects_non_uniform_periods() {
        let candles = make_trending_candles(16);
        let req = MovingAverageBatchRequest {
            ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            periods: vec![2, 4, 5],
            ma_params: None,
        };
        let err = compute_moving_average_batch_cpu(&candles, req).unwrap_err();
        assert!(err.contains("uniform"));
    }
}
