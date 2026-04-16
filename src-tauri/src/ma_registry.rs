//! Projection helpers for the frontend moving-average catalog.

use serde::Serialize;

use vector_ta::indicators::moving_averages::param_schema::MaParamInfo;

/// Frontend catalog row projected from the upstream moving-average registry.
#[derive(Debug, Clone, Serialize)]
pub struct MaInfo {
    pub id: String,
    pub label: String,
    /// Upstream single-call CPU support. Batch-only CPU support may still exist when this is false.
    pub supports_cpu: bool,
    /// Upstream single-call CUDA support. Sweep/batch CUDA support may still exist when this is false.
    pub supports_cuda: bool,
    /// Upstream CUDA sweep/batch support.
    pub supports_cuda_ma_sweep: bool,
    /// App-local GPU-kernel policy overlay.
    pub supports_cuda_kernel: bool,
    /// App-local preferred GPU-kernel policy overlay.
    pub prefers_cuda_kernel: bool,
    pub requires_candles: bool,
    pub period_based: bool,
    pub single_output: bool,
    /// Numeric parameter schema only. Additional enum/output semantics may still live in `notes`.
    pub params: Vec<MaParamInfo>,
    pub notes: Option<String>,
}

pub fn list_mas() -> Vec<MaInfo> {
    vector_ta::indicators::moving_averages::registry::list_moving_averages()
        .iter()
        .map(|m| {
            let supports_cuda_kernel = ta_app_core::ma_policy::supports_gpu_kernel_ma(m.id);
            MaInfo {
                id: m.id.to_string(),
                label: m.label.to_string(),
                supports_cpu: m.supports_cpu_single,
                supports_cuda: m.supports_cuda_single,
                supports_cuda_ma_sweep: m.supports_cuda_sweep,
                supports_cuda_kernel,
                prefers_cuda_kernel: supports_cuda_kernel
                    && ta_app_core::ma_policy::prefers_gpu_kernel_ma(m.id),
                requires_candles: m.requires_candles,
                period_based: m.period_based,
                single_output: m.single_output,
                params: vector_ta::indicators::moving_averages::param_schema::ma_param_schema(m.id)
                    .to_vec(),
                notes: m.notes.map(|s| s.to_string()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{list_mas, MaInfo};

    fn row_by_id<'a>(rows: &'a [MaInfo], id: &str) -> &'a MaInfo {
        rows.iter()
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("missing moving average row for {id}"))
    }

    #[test]
    fn projection_preserves_single_vs_batch_support_distinctions() {
        let rows = list_mas();
        let buff = row_by_id(&rows, "buff_averages");
        let vwap = row_by_id(&rows, "vwap");

        assert!(!buff.supports_cpu);
        assert!(!buff.supports_cuda);
        assert!(buff.supports_cuda_ma_sweep);

        assert!(vwap.supports_cpu);
        assert!(!vwap.supports_cuda);
        assert!(vwap.supports_cuda_ma_sweep);
    }

    #[test]
    fn projection_preserves_requires_candles_and_param_notes() {
        let rows = list_mas();
        let frama = row_by_id(&rows, "frama");
        let n_order_ema = row_by_id(&rows, "n_order_ema");

        assert!(frama.requires_candles);
        assert!(frama
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("source is ignored"));

        assert_eq!(n_order_ema.params.len(), 1);
        assert!(n_order_ema.params[0]
            .notes
            .unwrap_or_default()
            .contains("Additional enum params"));
    }

    #[test]
    fn prefers_cuda_kernel_implies_kernel_support() {
        for row in list_mas() {
            assert!(
                !row.prefers_cuda_kernel || row.supports_cuda_kernel,
                "preferred kernel MA must also be kernel-supported: {}",
                row.id
            );
        }
    }

    #[test]
    fn projection_clamps_advisory_speedup_preference_to_kernel_supported_rows() {
        let rows = list_mas();
        let gaussian = row_by_id(&rows, "gaussian");

        assert!(ta_app_core::ma_policy::prefers_gpu_kernel_ma("gaussian"));
        assert!(!gaussian.supports_cuda_kernel);
        assert!(!gaussian.prefers_cuda_kernel);
    }
}
