use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ValidationArtifactContext {
    #[serde(default)]
    pub strategy_kind: String,
    #[serde(default)]
    pub csv_path: Option<String>,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationAdmissionPolicy {
    #[default]
    LegacyTopk,
    ScalarUnion,
    ParetoFrontier,
    Auto,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ValidationAdmissionHeadProvenance {
    #[serde(default)]
    pub head: String,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub candidate_count: usize,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ValidationAdmissionProvenance {
    #[serde(default)]
    pub requested_policy: ValidationAdmissionPolicy,
    #[serde(default)]
    pub applied_policy: ValidationAdmissionPolicy,
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub source_candidate_count: usize,
    #[serde(default)]
    pub pre_dedupe_count: usize,
    #[serde(default)]
    pub post_dedupe_count: usize,
    #[serde(default)]
    pub post_cap_count: usize,
    #[serde(default)]
    pub candidate_cap_limit: usize,
    #[serde(default)]
    pub trimmed_count: usize,
    #[serde(default)]
    pub trim_ratio: f64,
    #[serde(default)]
    pub cap_applied: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heads: Vec<ValidationAdmissionHeadProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_bias_note: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfig {
    pub window_bars: usize,
    pub step_bars: usize,
}

fn default_walk_forward_window_bars() -> usize {
    20_000
}

fn default_walk_forward_step_bars() -> usize {
    20_000
}

fn default_reopt_walk_forward_top_n() -> usize {
    8
}

fn default_spa_slices() -> usize {
    16
}

fn default_spa_bootstrap_samples() -> usize {
    500
}

fn default_spa_block_length() -> usize {
    4
}

fn default_spa_benchmark() -> SpaBenchmarkKind {
    SpaBenchmarkKind::BuyAndHold
}

fn default_mc_sims() -> usize {
    500
}

fn default_cross_max_datasets() -> usize {
    8
}

fn default_cross_gate_enabled() -> bool {
    true
}

fn default_cross_gate_require_pareto() -> bool {
    true
}

fn default_cross_gate_min_score() -> f64 {
    0.0
}

fn default_cross_gate_min_pnl_positive_frac() -> f64 {
    0.5
}

fn default_cross_gate_min_sharpe_median() -> f64 {
    0.0
}

fn default_cross_gate_min_datasets() -> usize {
    1
}

fn default_cross_ticker_mode() -> CrossTickerMode {
    CrossTickerMode::Lightweight
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WalkForwardConfigOptional {
    #[serde(default = "default_walk_forward_window_bars")]
    pub window_bars: usize,
    #[serde(default = "default_walk_forward_step_bars")]
    pub step_bars: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReoptWalkForwardConfigOptional {
    #[serde(default = "default_walk_forward_window_bars")]
    pub window_bars: usize,
    #[serde(default = "default_walk_forward_step_bars")]
    pub step_bars: usize,
    #[serde(default = "default_reopt_walk_forward_top_n")]
    pub top_n_candidates: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaBenchmarkKind {
    Flat,
    BuyAndHold,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaConfigOptional {
    #[serde(default = "default_spa_slices")]
    pub slices: usize,
    #[serde(default = "default_spa_bootstrap_samples")]
    pub bootstrap_samples: usize,
    #[serde(default = "default_spa_block_length")]
    pub block_length: usize,
    #[serde(default = "default_spa_benchmark")]
    pub benchmark: SpaBenchmarkKind,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalkForwardStats {
    pub windows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_windows_upper_bound: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bars_covered: Option<usize>,
    pub sharpe_median: f64,
    pub sharpe_worst: f64,
    pub max_dd_median: f64,
    pub max_dd_worst: f64,
    pub max_dd_tail_mean_p95: f64,
    pub pnl_median: f64,
    pub pnl_worst: f64,
    pub pnl_tail_mean_p05: f64,
    pub downside_vol_ratio_median: f64,
    pub downside_vol_ratio_worst: f64,
    pub ulcer_index_median: f64,
    pub ulcer_index_worst: f64,
    pub martin_ratio_median: f64,
    pub martin_ratio_worst: f64,
    pub underwater_fraction_median: f64,
    pub underwater_fraction_worst: f64,
    pub max_underwater_duration_bars_median: u32,
    pub max_underwater_duration_bars_worst: u32,
    pub trades_median: u32,
    pub trades_worst: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReoptWalkForwardWindowStats {
    pub train_start: usize,
    pub train_end: usize,
    pub test_end: usize,
    pub winner_key: String,
    pub train_score: f64,
    pub train_pnl: f64,
    pub train_sharpe: f64,
    pub train_ulcer_index: f64,
    pub train_martin_ratio: f64,
    pub forward_pnl: f64,
    pub forward_sharpe: f64,
    pub forward_max_dd: f64,
    pub forward_ulcer_index: f64,
    pub forward_martin_ratio: f64,
    pub forward_trades: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pnl_efficiency: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe_efficiency: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReoptWalkForwardStats {
    pub windows: usize,
    pub top_n_candidates: usize,
    pub unique_winners: usize,
    pub winner_changes: usize,
    pub dominant_winner_frac: f64,
    pub winner_param_drift_mean: f64,
    pub winner_param_drift_max: f64,
    pub train_score_median: f64,
    pub forward_positive_pnl_frac: f64,
    pub forward_sharpe_median: f64,
    pub forward_sharpe_worst: f64,
    pub forward_max_dd_median: f64,
    pub forward_max_dd_worst: f64,
    pub forward_pnl_median: f64,
    pub forward_pnl_worst: f64,
    pub forward_ulcer_index_median: f64,
    pub forward_ulcer_index_worst: f64,
    pub forward_martin_ratio_median: f64,
    pub forward_martin_ratio_worst: f64,
    pub forward_trades_median: u32,
    pub forward_trades_worst: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_pnl_efficiency_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_pnl_efficiency_worst: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_sharpe_efficiency_median: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_sharpe_efficiency_worst: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_sharpe_efficiency_std: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub winner_sequence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_stats: Vec<ReoptWalkForwardWindowStats>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SpaValidationStats {
    pub scope: String,
    pub candidate_count: usize,
    pub slice_count: usize,
    pub bootstrap_samples: usize,
    pub block_length: usize,
    pub benchmark: SpaBenchmarkKind,
    pub best_candidate_key: String,
    pub best_candidate_mean_diff: f64,
    pub best_candidate_t_stat: f64,
    pub p_value: f64,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
}

/// Parameter perturbation config.
///
/// A fully omitted or zeroed config is treated as disabled work by the validation pipeline.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PerturbConfig {
    #[serde(default)]
    pub iterations: usize,
    #[serde(default)]
    pub pct_min: f64,
    #[serde(default)]
    pub pct_max: f64,
    #[serde(default)]
    pub flip_bool_prob: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PerturbStats {
    pub iterations: usize,
    pub pct_min: f64,
    pub pct_max: f64,
    pub sharpe_mean: f64,
    pub sharpe_worst: f64,
    pub max_dd_mean: f64,
    pub max_dd_worst: f64,
    pub pnl_mean: f64,
    pub pnl_worst: f64,
    pub pnl_positive_frac: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TradeBootstrapConfig {
    #[serde(default = "default_mc_sims")]
    pub simulations: usize,
    /// `Some(0)` is a sentinel for IID-only mode in the validation pipeline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stationary_mean_block_len: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TradeBootstrapStats {
    pub simulations: usize,
    pub trade_count: usize,
    pub prob_pnl_positive: f64,
    pub pnl_p05: f64,
    pub pnl_tail_mean_p05: f64,
    pub pnl_p50: f64,
    pub pnl_p95: f64,
    pub max_dd_p50: f64,
    pub max_dd_p95: f64,
    pub max_dd_tail_mean_p95: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_mean_block_len: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_prob_pnl_positive: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_p05: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_tail_mean_p05: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_pnl_p95: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_max_dd_p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_max_dd_p95: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stationary_max_dd_tail_mean_p95: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CrossTickerDataset {
    pub csv_path: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CrossTickerMode {
    Lightweight,
    PreservedFinalists,
    FullStack,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CrossTickerGateConfig {
    #[serde(default = "default_cross_gate_enabled")]
    pub enabled: bool,
    #[serde(default = "default_cross_gate_require_pareto")]
    pub require_pareto: bool,
    #[serde(default = "default_cross_gate_min_score")]
    pub min_score: f64,
    #[serde(default = "default_cross_gate_min_pnl_positive_frac")]
    pub min_pnl_positive_frac: f64,
    #[serde(default = "default_cross_gate_min_sharpe_median")]
    pub min_sharpe_median: f64,
    #[serde(default)]
    pub max_dd_worst: Option<f64>,
    #[serde(default = "default_cross_gate_min_datasets")]
    pub min_datasets: usize,
}

impl Default for CrossTickerGateConfig {
    fn default() -> Self {
        Self {
            enabled: default_cross_gate_enabled(),
            require_pareto: default_cross_gate_require_pareto(),
            min_score: default_cross_gate_min_score(),
            min_pnl_positive_frac: default_cross_gate_min_pnl_positive_frac(),
            min_sharpe_median: default_cross_gate_min_sharpe_median(),
            max_dd_worst: None,
            min_datasets: default_cross_gate_min_datasets(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CrossTickerConfig {
    #[serde(default)]
    pub datasets: Vec<CrossTickerDataset>,
    #[serde(default = "default_cross_max_datasets")]
    pub max_datasets: usize,
    #[serde(default = "default_cross_ticker_mode")]
    pub mode: CrossTickerMode,
    #[serde(default)]
    pub gate: CrossTickerGateConfig,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerDatasetMeta {
    pub label: String,
    pub csv_path: String,
    pub bars: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerValidationSummary {
    pub mode: CrossTickerMode,
    pub datasets: Vec<CrossTickerDatasetMeta>,
}

/// Additive semantics/provenance contract for validation artifacts.
///
/// This is intentionally descriptive metadata layered on top of the existing
/// validation outputs. It does not change ranking, gating, or approval logic.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ValidationSemanticsProvenance {
    /// Stage order for the current validation stack.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stage_sequence: Vec<String>,
    /// Metric channel used for the candidate-local score basis.
    #[serde(default)]
    pub score_channel: String,
    /// Source contract for PSR/DSR.
    #[serde(default)]
    pub probability_metric_source: String,
    /// Source contract for the stored track-record bars.
    #[serde(default)]
    pub track_record_source: String,
    /// CSCV coverage state at the run level.
    #[serde(default)]
    pub cscv_coverage_state: String,
    /// Candidate-stat coverage shape for CSCV outputs.
    #[serde(default)]
    pub cscv_candidate_stat_coverage: String,
    /// Whether evaluated CSCV selected any candidates.
    #[serde(default)]
    pub cscv_selection_state: String,
    #[serde(default)]
    pub cscv_selected_candidate_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cscv_skipped_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_mode: Option<CrossTickerMode>,
    /// Normalized evidence-depth label for cross-ticker reruns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_evidence_depth: Option<String>,
    /// Validation modules intentionally preserved inside cross-ticker reruns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_modules_preserved: Vec<String>,
    /// Validation modules intentionally omitted from cross-ticker reruns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_modules_omitted: Vec<String>,
    /// Current cross-ticker aggregation scope. Success-only summaries do not
    /// imply full-panel robustness over failed reruns.
    #[serde(default)]
    pub cross_summary_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_summary_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_summary_role: Option<String>,
    #[serde(default)]
    pub passed_semantics: String,
    #[serde(default)]
    pub approved_semantics: String,
    #[serde(default)]
    pub passed_can_diverge_from_approved: bool,
    #[serde(default)]
    pub passed_vs_approved_contract: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_summary_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_summary_semantics: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_summary_descriptive_only: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CrossTickerStats {
    pub datasets: usize,
    pub sharpe_median: f64,
    pub sharpe_worst: f64,
    pub max_dd_median: f64,
    pub max_dd_worst: f64,
    pub pnl_median: f64,
    pub pnl_worst: f64,
    pub trades_median: u32,
    pub trades_worst: u32,
    pub pnl_positive_frac: f64,
    pub score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationAuxWorkMask {
    pub regime_slice_robustness: bool,
    pub friction_stress: bool,
    pub entry_margin_fragility: bool,
    pub underwater_duration: bool,
    pub checkpoint_robustness: bool,
    pub contiguous_segment_robustness: bool,
    pub benchmark_slice_summary: bool,
}

impl ValidationAuxWorkMask {
    pub const fn full() -> Self {
        Self {
            regime_slice_robustness: true,
            friction_stress: true,
            entry_margin_fragility: true,
            underwater_duration: true,
            checkpoint_robustness: true,
            contiguous_segment_robustness: true,
            benchmark_slice_summary: true,
        }
    }

    pub const fn preserved_finalists() -> Self {
        Self {
            regime_slice_robustness: false,
            friction_stress: false,
            entry_margin_fragility: false,
            underwater_duration: false,
            checkpoint_robustness: false,
            contiguous_segment_robustness: false,
            benchmark_slice_summary: false,
        }
    }
}

struct CanonicalJsonValue<'a>(&'a JsonValue);

impl Serialize for CanonicalJsonValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0 {
            JsonValue::Null => serializer.serialize_unit(),
            JsonValue::Bool(value) => serializer.serialize_bool(*value),
            JsonValue::Number(value) => value.serialize(serializer),
            JsonValue::String(value) => serializer.serialize_str(value),
            JsonValue::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&CanonicalJsonValue(item))?;
                }
                seq.end()
            }
            JsonValue::Object(map) => {
                let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                keys.sort_unstable();

                let mut out = serializer.serialize_map(Some(map.len()))?;
                for key in keys {
                    if let Some(value) = map.get(key) {
                        out.serialize_entry(key, &CanonicalJsonValue(value))?;
                    }
                }
                out.end()
            }
        }
    }
}

pub(crate) fn canonical_json_value_key(value: &JsonValue) -> Result<String, String> {
    serde_json::to_string(&CanonicalJsonValue(value)).map_err(|e| e.to_string())
}

pub(crate) fn canonical_json_value_key_bytes(value: &JsonValue) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&CanonicalJsonValue(value)).map_err(|e| e.to_string())
}

/// Canonicalize JSON object-key ordering for deterministic internal key generation.
///
/// Arrays intentionally preserve their existing order.
pub(crate) fn canonical_json_key<T: Serialize>(value: &T) -> Result<String, String> {
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    canonical_json_value_key(&json)
}

pub(crate) fn canonical_json_key_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    canonical_json_value_key_bytes(&json)
}

#[cfg(any(test, all(feature = "cuda", feature = "cuda-backtest-kernel")))]
pub(crate) fn canonical_json_eq<T: Serialize>(a: &T, b: &T) -> Result<bool, String> {
    Ok(canonical_json_key(a)? == canonical_json_key(b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::json;
    use std::collections::HashMap;

    fn canonicalize_json_value_legacy(value: &JsonValue) -> JsonValue {
        match value {
            JsonValue::Array(items) => {
                JsonValue::Array(items.iter().map(canonicalize_json_value_legacy).collect())
            }
            JsonValue::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_unstable();
                let mut out = serde_json::Map::with_capacity(map.len());
                for key in keys {
                    if let Some(next) = map.get(key) {
                        out.insert(key.clone(), canonicalize_json_value_legacy(next));
                    }
                }
                JsonValue::Object(out)
            }
            _ => value.clone(),
        }
    }

    #[derive(Serialize)]
    struct WrappedMap {
        inner: HashMap<String, f64>,
    }

    #[test]
    fn canonical_json_key_ignores_hash_map_insertion_order() {
        let mut left = HashMap::new();
        left.insert("b".to_string(), 2.0);
        left.insert("a".to_string(), 1.0);

        let mut right = HashMap::new();
        right.insert("a".to_string(), 1.0);
        right.insert("b".to_string(), 2.0);

        let left = WrappedMap { inner: left };
        let right = WrappedMap { inner: right };

        assert_eq!(
            canonical_json_key(&left).expect("left key"),
            canonical_json_key(&right).expect("right key")
        );
        assert!(canonical_json_eq(&left, &right).expect("canonical eq"));
    }

    #[test]
    fn preserved_finalists_mask_disables_auxiliary_work() {
        let mask = ValidationAuxWorkMask::preserved_finalists();
        assert!(!mask.regime_slice_robustness);
        assert!(!mask.friction_stress);
        assert!(!mask.entry_margin_fragility);
        assert!(!mask.underwater_duration);
        assert!(!mask.checkpoint_robustness);
        assert!(!mask.contiguous_segment_robustness);
        assert!(!mask.benchmark_slice_summary);
    }

    #[test]
    fn cross_ticker_gate_defaults_match_missing_and_empty_gate_forms() {
        let missing_gate: CrossTickerConfig =
            serde_json::from_str("{}").expect("deserialize config without gate");
        let empty_gate: CrossTickerConfig =
            serde_json::from_str(r#"{"gate":{}}"#).expect("deserialize config with empty gate");
        let defaults = CrossTickerGateConfig::default();

        assert_eq!(missing_gate.gate.enabled, defaults.enabled);
        assert_eq!(missing_gate.gate.require_pareto, defaults.require_pareto);
        assert_eq!(missing_gate.gate.min_score, defaults.min_score);
        assert_eq!(
            missing_gate.gate.min_pnl_positive_frac,
            defaults.min_pnl_positive_frac
        );
        assert_eq!(
            missing_gate.gate.min_sharpe_median,
            defaults.min_sharpe_median
        );
        assert_eq!(missing_gate.gate.max_dd_worst, defaults.max_dd_worst);
        assert_eq!(missing_gate.gate.min_datasets, defaults.min_datasets);

        assert_eq!(missing_gate.gate.enabled, empty_gate.gate.enabled);
        assert_eq!(
            missing_gate.gate.require_pareto,
            empty_gate.gate.require_pareto
        );
        assert_eq!(missing_gate.gate.min_score, empty_gate.gate.min_score);
        assert_eq!(
            missing_gate.gate.min_pnl_positive_frac,
            empty_gate.gate.min_pnl_positive_frac
        );
        assert_eq!(
            missing_gate.gate.min_sharpe_median,
            empty_gate.gate.min_sharpe_median
        );
        assert_eq!(missing_gate.gate.max_dd_worst, empty_gate.gate.max_dd_worst);
        assert_eq!(missing_gate.gate.min_datasets, empty_gate.gate.min_datasets);
    }

    #[test]
    fn perturb_config_omission_defaults_to_disabled_zeroed_shape() {
        let cfg: PerturbConfig =
            serde_json::from_str("{}").expect("deserialize perturb config defaults");

        assert_eq!(cfg.iterations, 0);
        assert_eq!(cfg.pct_min, 0.0);
        assert_eq!(cfg.pct_max, 0.0);
        assert_eq!(cfg.flip_bool_prob, 0.0);
    }

    #[test]
    fn canonical_json_key_preserves_array_order_and_matches_bytes_output() {
        let left = json!({
            "nested": {
                "b": 2,
                "a": 1
            },
            "items": [3, 2, 1]
        });
        let right = json!({
            "items": [1, 2, 3],
            "nested": {
                "a": 1,
                "b": 2
            }
        });

        let left_key = canonical_json_key(&left).expect("left key");
        let left_bytes = canonical_json_key_bytes(&left).expect("left bytes");
        let right_key = canonical_json_key(&right).expect("right key");

        assert_eq!(left_key.as_bytes(), left_bytes.as_slice());
        assert_ne!(left_key, right_key, "array order should remain significant");
    }

    #[test]
    fn direct_canonical_json_value_serializer_matches_legacy_output() {
        let value = json!({
            "z": [
                {"b": 2, "a": 1},
                true,
                null,
                "line\nbreak",
                1.25
            ],
            "a": {
                "nested": {"d": 4, "c": 3},
                "items": [3, 1, 2]
            }
        });

        let legacy = serde_json::to_string(&canonicalize_json_value_legacy(&value))
            .expect("legacy canonical string");
        let direct = canonical_json_value_key(&value).expect("direct canonical string");
        let direct_bytes = canonical_json_value_key_bytes(&value).expect("direct canonical bytes");

        assert_eq!(direct, legacy);
        assert_eq!(direct.as_bytes(), direct_bytes.as_slice());
    }
}
