use crate::alloc_policy::{alloc_f64_with_prefix_uninit_tail, resize_uninit_i32};
use crate::double_ma::BackendUsed;
use crate::dynamic_indicator_adapter::{compute_dynamic_indicator_cpu, DynamicIndicatorSeries};
use crate::indicator_catalog::{IndicatorParamKV, IndicatorParamValue};
use crate::iterative_sim::{simulate_iterative_from_signals, IterativeSimConfig, OhlcSlice};
use crate::validation_pipeline::{
    apply_candidate_hard_gates, build_run_verdict, compute_probability_metrics, mark_robust_pareto,
    robust_score_from_metrics, sanitize_hard_gates, ValidationCandidateVerdict,
    ValidationPostCandidate, ValidationRunVerdict,
};
use crate::validation_shared::{canonical_json_key_bytes, ValidationArtifactContext};
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use libloading::Library;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use ta_strategies::double_ma::{Metrics, StrategyConfig};
use vector_ta::utilities::data_loader::Candles;
use vector_ta::utilities::enums::Kernel;

const ML_SIGNAL_FEATURE_SCHEMA_VERSION: u32 = 1;
const ML_SIGNAL_SCORE_CACHE_VERSION: u32 = 1;
const ML_SIGNAL_EXPORT_MANIFEST_VERSION: u32 = 1;
const ML_SIGNAL_SPLIT_PLAN_VERSION: u32 = 1;
const ML_SIGNAL_EXPORTER_VERSION: &str = "ml_signal_export_v2";
const ML_SIGNAL_SCORE_CONTRACT_VERSION: u32 = 1;

fn default_horizon_bars() -> usize {
    10
}

fn default_ma_source() -> String {
    "close".to_string()
}

fn default_max_hold_bars() -> usize {
    10
}

fn default_threshold() -> f64 {
    0.55
}

fn default_delta() -> f64 {
    0.05
}

fn default_true() -> bool {
    true
}

fn default_split_step_bars() -> usize {
    63
}

fn default_split_gap_bars() -> usize {
    10
}

fn default_top_k() -> usize {
    25
}

fn default_ml_signal_score_space() -> MlSignalScoreSpace {
    MlSignalScoreSpace::Probability
}

fn default_ml_signal_score_scope() -> MlSignalScoreScope {
    MlSignalScoreScope::CustomPartial
}

fn default_ml_signal_exporter_version() -> String {
    ML_SIGNAL_EXPORTER_VERSION.to_string()
}

fn default_ml_signal_score_contract_version() -> u32 {
    ML_SIGNAL_SCORE_CONTRACT_VERSION
}

fn default_ml_signal_inference_backend_preference() -> MlSignalInferenceBackendPreference {
    MlSignalInferenceBackendPreference::Auto
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlSignalScoreSpace {
    Probability,
    RawMargin,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlSignalScoreScope {
    FullDataset,
    CalibrationSlice,
    WalkforwardTest,
    CustomPartial,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlSignalInferenceBackendPreference {
    Auto,
    NativeCpu,
    NativeCuda,
    ScoreCacheOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MlSignalInferenceBackendUsed {
    ScoreCacheOnly,
    NativeCpu,
    NativeCuda,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalScoreCacheSummary {
    pub matched_rows: usize,
    pub skipped_rows: usize,
    pub bars_scored: usize,
    pub duplicate_timestamps: usize,
    pub coverage_ratio: f64,
    pub score_space: MlSignalScoreSpace,
    pub score_scope: MlSignalScoreScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_schema_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_contract_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_bundle_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_backend_requested: Option<MlSignalInferenceBackendPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_backend_used: Option<MlSignalInferenceBackendUsed>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_library_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_gpu_device_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalIndicatorConfig {
    pub ma_source: String,
    pub horizon_bars: usize,
    pub alma_periods: Vec<usize>,
    pub adx_period: usize,
    pub atr_period: usize,
    pub pattern_semantics: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalWarmupPolicy {
    pub kind: String,
    pub first_valid_bar: usize,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalLabelPolicy {
    pub kind: String,
    pub horizon_bars: usize,
    pub round_trip_cost_bps: f64,
    pub deadband_kind: String,
    pub long_rule: String,
    pub short_rule: String,
    pub keep_rule: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalSampleWeightPolicy {
    pub kind: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalFeatureSchema {
    pub schema_version: u32,
    pub strategy_kind: String,
    pub horizon_bars: usize,
    pub ma_source: String,
    pub first_valid_bar: usize,
    pub feature_names: Vec<String>,
    pub pattern_ids: Vec<String>,
    #[serde(default)]
    pub schema_hash: String,
    #[serde(default = "default_ml_signal_exporter_version")]
    pub exporter_version: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub feature_block_versions: HashMap<String, u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicator_config: Option<MlSignalIndicatorConfig>,
    #[serde(default)]
    pub label_horizon_bars: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_valid_reasoning: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalSplitWindow {
    pub split_id: String,
    pub train_start: usize,
    pub train_end: usize,
    pub calibration_start: usize,
    pub calibration_end: usize,
    pub test_start: usize,
    pub test_end: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalSplitPlan {
    pub version: u32,
    pub train_bars: usize,
    pub calibration_bars: usize,
    pub test_bars: usize,
    #[serde(default = "default_split_gap_bars")]
    pub gap_bars: usize,
    #[serde(default = "default_split_step_bars")]
    pub step_bars: usize,
    #[serde(default)]
    pub windows: Vec<MlSignalSplitWindow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalTickerHoldoutConfig {
    #[serde(default)]
    pub modulus: usize,
    #[serde(default)]
    pub residue: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalDatasetExportManifest {
    pub version: u32,
    pub strategy_kind: String,
    pub data_id: String,
    pub output_path: String,
    pub row_count: usize,
    pub feature_schema_path: String,
    pub feature_schema_version: u32,
    pub split_manifest_path: Option<String>,
    pub split_count: usize,
    pub horizon_bars: usize,
    pub first_valid_bar: usize,
    pub pattern_count: usize,
    pub feature_count: usize,
    pub first_timestamp: Option<i64>,
    pub last_timestamp: Option<i64>,
    pub ticker_holdout: Option<bool>,
    #[serde(default)]
    pub feature_schema_hash: String,
    #[serde(default = "default_ml_signal_exporter_version")]
    pub exporter_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicator_config: Option<MlSignalIndicatorConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warmup_policy: Option<MlSignalWarmupPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_policy: Option<MlSignalLabelPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_weight_policy: Option<MlSignalSampleWeightPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalDatasetExportRequest {
    pub data_id: String,
    pub output_path: String,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    #[serde(default = "default_horizon_bars")]
    pub horizon_bars: usize,
    #[serde(default)]
    pub round_trip_cost_bps: f64,
    #[serde(default)]
    pub split_plan: Option<MlSignalSplitPlan>,
    #[serde(default)]
    pub ticker_holdout: Option<MlSignalTickerHoldoutConfig>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalDatasetExportResult {
    pub data_id: String,
    pub output_path: String,
    pub feature_schema_path: String,
    pub export_manifest_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_manifest_path: Option<String>,
    pub row_count: usize,
    pub feature_count: usize,
    pub feature_schema_version: u32,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub feature_schema_hash: String,
    pub pattern_count: usize,
    pub first_valid_bar: usize,
    pub horizon_bars: usize,
    pub split_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticker_holdout: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalScoreCacheBuildRequest {
    pub data_id: String,
    pub input_path: String,
    pub output_path: String,
    #[serde(default)]
    pub model_bundle_id: Option<String>,
    #[serde(default)]
    pub feature_schema_path: Option<String>,
    #[serde(default)]
    pub feature_schema_hash: Option<String>,
    #[serde(default)]
    pub export_manifest_path: Option<String>,
    #[serde(default)]
    pub source_bundle_manifest_path: Option<String>,
    #[serde(default)]
    pub calibration_state: Option<String>,
    #[serde(default)]
    pub threshold_policy: Option<String>,
    #[serde(default)]
    pub producer_kind: Option<String>,
    #[serde(default)]
    pub score_contract_version: Option<u32>,
    #[serde(default = "default_ml_signal_inference_backend_preference")]
    pub native_backend_preference: MlSignalInferenceBackendPreference,
    #[serde(default)]
    pub native_library_path: Option<String>,
    #[serde(default)]
    pub native_gpu_device_id: Option<i32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalScoreCacheBuildResult {
    pub data_id: String,
    pub input_path: String,
    pub output_path: String,
    pub manifest_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_schema_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_manifest_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_bundle_manifest_path: Option<String>,
    pub matched_rows: usize,
    pub skipped_rows: usize,
    pub bars_scored: usize,
    pub duplicate_timestamps: usize,
    pub coverage_ratio: f64,
    pub score_space: MlSignalScoreSpace,
    pub score_scope: MlSignalScoreScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_timestamp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<i64>,
    pub model_bundle_id: Option<String>,
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_schema_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_contract_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_bundle_manifest_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_manifest_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_backend_requested: Option<MlSignalInferenceBackendPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_backend_used: Option<MlSignalInferenceBackendUsed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_library_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_gpu_device_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalPolicyParams {
    #[serde(default = "default_threshold")]
    pub t_long: f64,
    #[serde(default = "default_threshold")]
    pub t_short: f64,
    #[serde(default = "default_delta")]
    pub delta: f64,
    #[serde(default = "default_max_hold_bars")]
    pub max_hold_bars: usize,
    #[serde(default)]
    pub exit_on_flat: bool,
    #[serde(default = "default_true")]
    pub exit_on_opposite: bool,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub strategy: StrategyConfig,
}

impl Default for MlSignalPolicyParams {
    fn default() -> Self {
        Self {
            t_long: default_threshold(),
            t_short: default_threshold(),
            delta: default_delta(),
            max_hold_bars: default_max_hold_bars(),
            exit_on_flat: false,
            exit_on_opposite: true,
            label: None,
            strategy: StrategyConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalValidationRequest {
    pub data_id: String,
    pub score_cache_path: String,
    pub candidates: Vec<MlSignalPolicyParams>,
    #[serde(default)]
    pub iterative: IterativeSimConfig,
    #[serde(default)]
    pub min_psr: Option<f64>,
    #[serde(default)]
    pub min_dsr: Option<f64>,
    #[serde(default)]
    pub validation_context: Option<ValidationArtifactContext>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalSweepAxisF64 {
    pub start: f64,
    pub end: f64,
    pub step: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalSweepAxisUsize {
    pub start: usize,
    pub end: usize,
    pub step: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalPolicySweepRequest {
    pub data_id: String,
    pub score_cache_path: String,
    #[serde(default)]
    pub t_long: Option<MlSignalSweepAxisF64>,
    #[serde(default)]
    pub t_short: Option<MlSignalSweepAxisF64>,
    #[serde(default)]
    pub delta: Option<MlSignalSweepAxisF64>,
    #[serde(default)]
    pub max_hold_bars: Option<MlSignalSweepAxisUsize>,
    #[serde(default)]
    pub exit_on_flat_values: Option<Vec<bool>>,
    #[serde(default)]
    pub exit_on_opposite_values: Option<Vec<bool>>,
    #[serde(default)]
    pub base_strategy: StrategyConfig,
    #[serde(default)]
    pub iterative: IterativeSimConfig,
    #[serde(default)]
    pub min_psr: Option<f64>,
    #[serde(default)]
    pub min_dsr: Option<f64>,
    #[serde(default)]
    pub validation_context: Option<ValidationArtifactContext>,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalPolicySweepResult {
    pub sweep_size: usize,
    pub top_k: usize,
    pub validation: MlSignalValidationResult,
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalValidationCandidateResult {
    pub params: MlSignalPolicyParams,
    pub in_sample: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterative_in_sample: Option<Metrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsr: Option<f64>,
    pub track_record_bars: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_track_record_length_bars: Option<usize>,
    #[serde(default)]
    pub validation_verdict: ValidationCandidateVerdict,
    pub robust_score: f64,
    pub robust_pareto: bool,
}

impl ValidationPostCandidate for MlSignalValidationCandidateResult {
    fn in_sample_metrics(&self) -> &Metrics {
        &self.in_sample
    }

    fn iterative_in_sample_metrics(&self) -> Option<&Metrics> {
        self.iterative_in_sample.as_ref()
    }

    fn walk_forward_stats(&self) -> Option<&crate::validation_shared::WalkForwardStats> {
        None
    }

    fn cscv_oos_metric_p50(&self) -> Option<f64> {
        None
    }

    fn cscv_oos_rank_frac_p50(&self) -> Option<f64> {
        None
    }

    fn psr(&self) -> Option<f64> {
        self.psr
    }

    fn dsr(&self) -> Option<f64> {
        self.dsr
    }

    fn set_psr(&mut self, v: Option<f64>) {
        self.psr = v;
    }

    fn set_dsr(&mut self, v: Option<f64>) {
        self.dsr = v;
    }

    fn track_record_bars(&self) -> usize {
        self.track_record_bars
    }

    fn min_track_record_length_bars(&self) -> Option<usize> {
        self.min_track_record_length_bars
    }

    fn set_track_record_bars(&mut self, v: usize) {
        self.track_record_bars = v;
    }

    fn set_min_track_record_length_bars(&mut self, v: Option<usize>) {
        self.min_track_record_length_bars = v;
    }

    fn robust_score(&self) -> f64 {
        self.robust_score
    }

    fn set_robust_score(&mut self, v: f64) {
        self.robust_score = v;
    }

    fn set_robust_pareto(&mut self, v: bool) {
        self.robust_pareto = v;
    }

    fn set_cscv_pbo(&mut self, _v: Option<crate::cscv_pbo::CscvCandidatePboStats>) {}

    fn validation_verdict(&self) -> &ValidationCandidateVerdict {
        &self.validation_verdict
    }

    fn set_validation_verdict(&mut self, v: ValidationCandidateVerdict) {
        self.validation_verdict = v;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalValidationResult {
    pub data_id: String,
    pub runtime_ms: u64,
    pub backend_used: BackendUsed,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    pub score_cache_path: String,
    pub score_summary: MlSignalScoreCacheSummary,
    pub candidates: Vec<MlSignalValidationCandidateResult>,
    pub run_verdict: ValidationRunVerdict,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalScoreCacheManifest {
    pub version: u32,
    pub kind: String,
    pub strategy_kind: String,
    pub data_id: String,
    pub input_path: String,
    pub feature_schema_version: u32,
    #[serde(default = "default_ml_signal_score_space")]
    pub score_space: MlSignalScoreSpace,
    #[serde(default = "default_ml_signal_score_scope")]
    pub score_scope: MlSignalScoreScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_schema_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_schema_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_bundle_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_bundle_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_kind: Option<String>,
    #[serde(default = "default_ml_signal_score_contract_version")]
    pub score_contract_version: u32,
    pub matched_rows: usize,
    pub skipped_rows: usize,
    pub bars_scored: usize,
    #[serde(default)]
    pub duplicate_timestamps: usize,
    #[serde(default)]
    pub coverage_ratio: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_backend_requested: Option<MlSignalInferenceBackendPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_backend_used: Option<MlSignalInferenceBackendUsed>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_library_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_gpu_device_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_fallback_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MlSignalModelBundleManifest {
    pub bundle_id: String,
    pub schema_version: u32,
    pub feature_schema_version: u32,
    pub model_family: String,
    pub model_format: String,
    #[serde(default)]
    pub long_model_path: Option<String>,
    #[serde(default)]
    pub short_model_path: Option<String>,
    #[serde(default)]
    pub calibration_path: Option<String>,
    #[serde(default)]
    pub feature_schema_path: Option<String>,
    #[serde(default)]
    pub export_manifest_path: Option<String>,
    #[serde(default)]
    pub split_manifest_path: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub source_export_manifest_hash: Option<String>,
    #[serde(default)]
    pub source_feature_schema_hash: Option<String>,
    #[serde(default)]
    pub training_config: Option<JsonValue>,
    #[serde(default)]
    pub training_windows_summary: Option<JsonValue>,
    #[serde(default)]
    pub calibration_summary: Option<JsonValue>,
    #[serde(default)]
    pub class_balance_policy: Option<String>,
    #[serde(default)]
    pub threshold_policy_default: Option<JsonValue>,
    #[serde(default)]
    pub explainability_artifacts: Option<JsonValue>,
    #[serde(default)]
    pub bundle_parent_id: Option<String>,
    #[serde(default)]
    pub score_contract: Option<JsonValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalModelBundleSummary {
    pub bundle_id: String,
    pub manifest_path: String,
    pub state: String,
    pub model_family: String,
    pub model_format: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MlSignalModelBundleStatus {
    pub bundle_id: String,
    pub manifest_path: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<MlSignalModelBundleManifest>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub issues: Vec<String>,
}

#[derive(Clone, Debug)]
struct DatasetFeatureFrame {
    pattern_ids: Vec<String>,
    pattern_hits: Vec<Vec<u8>>,
    alma_h: Vec<f64>,
    alma_3h: Vec<f64>,
    alma_10h: Vec<f64>,
    adx_2h: Vec<f64>,
    atr_h: Vec<f64>,
    first_valid_bar: usize,
}

#[derive(Clone, Debug)]
struct MlSignalInferenceFeatureRows {
    timestamps: Vec<i64>,
    float_features_transposed: Vec<Vec<f32>>,
}

#[derive(Clone, Debug)]
struct MlSignalNativeBuildMetadata {
    backend_requested: MlSignalInferenceBackendPreference,
    backend_used: MlSignalInferenceBackendUsed,
    library_path: Option<String>,
    gpu_device_id: Option<i32>,
    fallback_reason: Option<String>,
}

#[derive(Clone, Debug)]
struct MlSignalNativeScoreBuild {
    rows: Vec<MlSignalScoreCacheRow>,
    feature_schema_path: Option<String>,
    feature_schema_hash: Option<String>,
    export_manifest_path: Option<String>,
    source_bundle_manifest_path: Option<String>,
    source_bundle_manifest_hash: Option<String>,
    export_manifest_hash: Option<String>,
    calibration_state: Option<String>,
    threshold_policy: Option<String>,
    producer_kind: Option<String>,
    score_contract_version: Option<u32>,
    native: MlSignalNativeBuildMetadata,
}

#[derive(Clone, Copy, Debug)]
struct ScorePoint {
    p_long: Option<f64>,
    p_short: Option<f64>,
    raw_long: Option<f64>,
    raw_short: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct ParsedScoreRow {
    timestamp: i64,
    p_long: Option<f64>,
    p_short: Option<f64>,
    raw_long: Option<f64>,
    raw_short: Option<f64>,
}

#[derive(Clone, Debug)]
struct ParsedScoreCsv {
    rows: Vec<ParsedScoreRow>,
    score_space: MlSignalScoreSpace,
    score_scope: Option<MlSignalScoreScope>,
}

fn log_returns_from_close(close: &[f64]) -> Vec<f64> {
    let mut out = alloc_f64_with_prefix_uninit_tail(close.len(), 1, 0.0);
    for i in 1..close.len() {
        let prev = close[i - 1];
        let next = close[i];
        if prev.is_finite() && next.is_finite() && prev > 0.0 && next > 0.0 {
            out[i] = (next / prev).ln();
        } else {
            out[i] = 0.0;
        }
    }
    out
}

fn default_alma_params(period: usize) -> Vec<IndicatorParamKV> {
    vec![
        IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Int(period as i64),
        },
        IndicatorParamKV {
            key: "offset".to_string(),
            value: IndicatorParamValue::Float(0.85),
        },
        IndicatorParamKV {
            key: "sigma".to_string(),
            value: IndicatorParamValue::Float(6.0),
        },
    ]
}

fn period_params(period: usize) -> Vec<IndicatorParamKV> {
    vec![IndicatorParamKV {
        key: "period".to_string(),
        value: IndicatorParamValue::Int(period as i64),
    }]
}

fn length_params(length: usize) -> Vec<IndicatorParamKV> {
    vec![IndicatorParamKV {
        key: "length".to_string(),
        value: IndicatorParamValue::Int(length as i64),
    }]
}

fn extract_pattern_matrix(
    output: &crate::dynamic_indicator_adapter::DynamicIndicatorComputeOutput,
    pattern_ids: &[String],
) -> Result<Vec<Vec<u8>>, String> {
    let rows = output.rows;
    let cols = output.cols;
    if rows == 0 || cols == 0 {
        return Ok(Vec::new());
    }
    if !pattern_ids.is_empty() && rows != pattern_ids.len() {
        return Err(format!(
            "pattern_recognition rows {} do not match pattern_ids {}",
            rows,
            pattern_ids.len()
        ));
    }
    let mut matrix: Vec<Vec<u8>> = Vec::with_capacity(rows);
    match &output.series {
        DynamicIndicatorSeries::I32(values) => {
            if values.len() != rows.saturating_mul(cols) {
                return Err("pattern_recognition matrix length mismatch".to_string());
            }
            for row in 0..rows {
                let offset = row * cols;
                let src = &values[offset..offset + cols];
                let mut out_row = Vec::with_capacity(cols);
                for &value in src {
                    out_row.push(u8::from(value != 0));
                }
                matrix.push(out_row);
            }
        }
        DynamicIndicatorSeries::Bool(values) => {
            if values.len() != rows.saturating_mul(cols) {
                return Err("pattern_recognition matrix length mismatch".to_string());
            }
            for row in 0..rows {
                let offset = row * cols;
                let src = &values[offset..offset + cols];
                let mut out_row = Vec::with_capacity(cols);
                for &value in src {
                    out_row.push(u8::from(value));
                }
                matrix.push(out_row);
            }
        }
        _ => {
            return Err("pattern_recognition produced unsupported output type".to_string());
        }
    }
    Ok(matrix)
}

fn extract_f64_series(
    output: crate::dynamic_indicator_adapter::DynamicIndicatorComputeOutput,
) -> Result<Vec<f64>, String> {
    match output.series {
        DynamicIndicatorSeries::F64(values) => Ok(values),
        DynamicIndicatorSeries::I32(values) => Ok(values.into_iter().map(|v| v as f64).collect()),
        DynamicIndicatorSeries::Bool(values) => Ok(values
            .into_iter()
            .map(|value| if value { 1.0 } else { 0.0 })
            .collect()),
    }
}

fn first_finite_idx(values: &[f64]) -> usize {
    values
        .iter()
        .position(|v| v.is_finite())
        .unwrap_or(values.len())
}

fn rolling_count(prefix: &[usize], start: usize, end: usize) -> usize {
    if prefix.is_empty() || end >= prefix.len() {
        return 0;
    }
    prefix[end].saturating_sub(prefix[start])
}

fn sample_deadband(round_trip_cost_bps: f64, atr_h: f64, open_next: f64) -> f64 {
    let cost_frac = if round_trip_cost_bps.is_finite() && round_trip_cost_bps > 0.0 {
        round_trip_cost_bps / 10_000.0
    } else {
        0.0
    };
    let atr_frac = if atr_h.is_finite() && open_next.is_finite() && open_next > 0.0 {
        0.25 * (atr_h / open_next).max(0.0)
    } else {
        0.0
    };
    (2.0 * cost_frac).max(atr_frac)
}

fn feature_frame(
    candles: &Candles,
    ma_source: &str,
    horizon_bars: usize,
) -> Result<DatasetFeatureFrame, String> {
    let h = horizon_bars.max(2);
    let pattern_output = compute_dynamic_indicator_cpu(
        candles,
        ma_source,
        "pattern_recognition",
        None,
        &[],
        Kernel::Auto,
    )
    .map_err(|e| format!("pattern_recognition failed: {e}"))?;
    let pattern_ids = pattern_output.pattern_ids.clone().unwrap_or_default();
    let pattern_hits = extract_pattern_matrix(&pattern_output, &pattern_ids)?;

    let alma_h = extract_f64_series(
        compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            "alma",
            None,
            default_alma_params(h).as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| format!("alma(H) failed: {e}"))?,
    )?;
    let alma_3h = extract_f64_series(
        compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            "alma",
            None,
            default_alma_params(h.saturating_mul(3)).as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| format!("alma(3H) failed: {e}"))?,
    )?;
    let alma_10h = extract_f64_series(
        compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            "alma",
            None,
            default_alma_params(h.saturating_mul(10)).as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| format!("alma(10H) failed: {e}"))?,
    )?;
    let adx_2h = extract_f64_series(
        compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            "adx",
            None,
            period_params(h.saturating_mul(2)).as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| format!("adx(2H) failed: {e}"))?,
    )?;
    let atr_h = extract_f64_series(
        compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            "atr",
            None,
            length_params(h).as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| format!("atr(H) failed: {e}"))?,
    )?;

    let mut first_valid_bar = 1usize;
    first_valid_bar = first_valid_bar.max(first_finite_idx(&alma_h));
    first_valid_bar = first_valid_bar.max(first_finite_idx(&alma_3h));
    first_valid_bar = first_valid_bar.max(first_finite_idx(&alma_10h));
    first_valid_bar = first_valid_bar.max(first_finite_idx(&adx_2h));
    first_valid_bar = first_valid_bar.max(first_finite_idx(&atr_h));
    first_valid_bar = first_valid_bar.max(h.saturating_mul(3));

    Ok(DatasetFeatureFrame {
        pattern_ids,
        pattern_hits,
        alma_h,
        alma_3h,
        alma_10h,
        adx_2h,
        atr_h,
        first_valid_bar,
    })
}

fn format_f64(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        String::new()
    }
}

fn stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().copied().sum::<f64>() / values.len() as f64;
    let var = values
        .iter()
        .map(|value| {
            let diff = *value - mean;
            diff * diff
        })
        .sum::<f64>()
        / values.len() as f64;
    var.sqrt()
}

fn prefix_sum(values: &[f64]) -> Vec<f64> {
    let mut prefix = alloc_f64_with_prefix_uninit_tail(values.len() + 1, 1, 0.0);
    for (idx, value) in values.iter().enumerate() {
        prefix[idx + 1] = prefix[idx] + *value;
    }
    prefix
}

fn prefix_sum_squares(values: &[f64]) -> Vec<f64> {
    let mut prefix = alloc_f64_with_prefix_uninit_tail(values.len() + 1, 1, 0.0);
    for (idx, value) in values.iter().enumerate() {
        prefix[idx + 1] = prefix[idx] + (*value * *value);
    }
    prefix
}

fn rolling_sum(prefix: &[f64], start: usize, end_exclusive: usize) -> f64 {
    if prefix.is_empty() || end_exclusive >= prefix.len() || start >= end_exclusive {
        0.0
    } else {
        prefix[end_exclusive] - prefix[start]
    }
}

fn rolling_stddev(prefix: &[f64], prefix_sq: &[f64], start: usize, end_exclusive: usize) -> f64 {
    if end_exclusive <= start || end_exclusive >= prefix.len() || end_exclusive >= prefix_sq.len() {
        return 0.0;
    }
    let count = (end_exclusive - start) as f64;
    if count <= 0.0 {
        return 0.0;
    }
    let sum = rolling_sum(prefix, start, end_exclusive);
    let sum_sq = rolling_sum(prefix_sq, start, end_exclusive);
    let mean = sum / count;
    ((sum_sq / count) - (mean * mean)).max(0.0).sqrt()
}

fn feature_names_for_schema(pattern_ids: &[String]) -> Vec<String> {
    let mut feature_names = vec![
        "body_pct".to_string(),
        "abs_body_frac".to_string(),
        "upper_wick_frac".to_string(),
        "lower_wick_frac".to_string(),
        "close_location".to_string(),
        "gap_pct".to_string(),
        "true_range_pct".to_string(),
    ];
    for pattern_id in pattern_ids {
        feature_names.push(format!("pr_{pattern_id}"));
        feature_names.push(format!("pr_{pattern_id}_count_3"));
        feature_names.push(format!("pr_{pattern_id}_count_5"));
        feature_names.push(format!("pr_{pattern_id}_count_h"));
        feature_names.push(format!("pr_{pattern_id}_count_2h"));
        feature_names.push(format!("pr_{pattern_id}_bars_since"));
    }
    feature_names.extend([
        "alma_h".to_string(),
        "alma_3h".to_string(),
        "alma_10h".to_string(),
        "alma_spread_h_3h_atr".to_string(),
        "alma_spread_3h_10h_atr".to_string(),
        "alma_slope_h_half_atr".to_string(),
        "alma_slope_3h_h_atr".to_string(),
        "close_minus_alma_3h_atr".to_string(),
        "adx_2h".to_string(),
        "log_ret_1".to_string(),
        "log_ret_2".to_string(),
        "log_ret_3".to_string(),
        "log_ret_5".to_string(),
        "log_ret_h".to_string(),
        "realized_vol_h".to_string(),
        "realized_vol_3h".to_string(),
        "atr_h_over_close".to_string(),
    ]);
    feature_names
}

fn build_ml_signal_inference_feature_rows(
    candles: &Candles,
    schema: &MlSignalFeatureSchema,
) -> Result<MlSignalInferenceFeatureRows, String> {
    let n = candles.close.len();
    let h = schema.horizon_bars.max(2);
    if n < 64 {
        return Err("not enough candles for native MlSignal inference".to_string());
    }
    let frame = feature_frame(candles, &schema.ma_source, h)?;
    let expected_feature_names = feature_names_for_schema(&frame.pattern_ids);
    if schema.feature_names != expected_feature_names {
        return Err(
            "feature schema feature_names do not match runtime MlSignal feature frame".to_string(),
        );
    }
    let last_label_bar = n.saturating_sub(h.saturating_add(2));
    if frame.first_valid_bar > last_label_bar {
        return Err("not enough candles after warmup for native MlSignal inference".to_string());
    }

    let log_returns = log_returns_from_close(&candles.close);
    let log_return_prefix = prefix_sum(&log_returns);
    let log_return_sq_prefix = prefix_sum_squares(&log_returns);
    let mut pattern_prefix: Vec<Vec<usize>> = Vec::with_capacity(frame.pattern_hits.len());
    for row in &frame.pattern_hits {
        let mut prefix = vec![0usize; row.len() + 1];
        for (idx, value) in row.iter().enumerate() {
            prefix[idx + 1] = prefix[idx] + usize::from(*value != 0);
        }
        pattern_prefix.push(prefix);
    }
    let mut last_seen: Vec<Option<usize>> = vec![None; frame.pattern_hits.len()];
    for idx in 0..frame.first_valid_bar {
        for (row_idx, row) in frame.pattern_hits.iter().enumerate() {
            if row.get(idx).copied().unwrap_or(0) != 0 {
                last_seen[row_idx] = Some(idx);
            }
        }
    }

    let row_count = last_label_bar
        .saturating_sub(frame.first_valid_bar)
        .saturating_add(1);
    let mut timestamps = Vec::with_capacity(row_count);
    let mut float_features_transposed: Vec<Vec<f32>> = (0..schema.feature_names.len())
        .map(|_| Vec::with_capacity(row_count))
        .collect();

    for t in frame.first_valid_bar..=last_label_bar {
        let prev_close = candles.close[t.saturating_sub(1)];
        let open = candles.open[t];
        let high = candles.high[t];
        let low = candles.low[t];
        let close = candles.close[t];
        let range = (high - low).abs();
        let eps = if close.is_finite() && close.abs() > 1.0e-12 {
            close.abs() * 1.0e-12
        } else {
            1.0e-12
        };
        let true_range = if prev_close.is_finite() {
            (high - low)
                .abs()
                .max((high - prev_close).abs())
                .max((low - prev_close).abs())
        } else {
            (high - low).abs()
        };
        let body_pct = if open.is_finite() && open != 0.0 {
            (close - open) / open
        } else {
            0.0
        };
        let abs_body_frac = (close - open).abs() / (range + eps);
        let upper_wick_frac = (high - open.max(close)).max(0.0) / (range + eps);
        let lower_wick_frac = (open.min(close) - low).max(0.0) / (range + eps);
        let close_location = (close - low) / (range + eps);
        let gap_pct = if prev_close.is_finite() && prev_close != 0.0 {
            (open - prev_close) / prev_close
        } else {
            0.0
        };
        let true_range_pct = if prev_close.is_finite() && prev_close != 0.0 {
            true_range / prev_close
        } else {
            0.0
        };

        let atr_h = frame.atr_h[t];
        let alma_h = frame.alma_h[t];
        let alma_3h = frame.alma_3h[t];
        let alma_10h = frame.alma_10h[t];
        let adx_2h = frame.adx_2h[t];
        let half_h = (h / 2).max(1);
        let alma_slope_h_half_atr = if t >= half_h && atr_h.is_finite() && atr_h > 0.0 {
            (frame.alma_h[t] - frame.alma_h[t - half_h]) / atr_h
        } else {
            0.0
        };
        let alma_slope_3h_h_atr = if t >= h && atr_h.is_finite() && atr_h > 0.0 {
            (frame.alma_3h[t] - frame.alma_3h[t - h]) / atr_h
        } else {
            0.0
        };
        let alma_spread_h_3h_atr = if atr_h.is_finite() && atr_h > 0.0 {
            (alma_h - alma_3h) / atr_h
        } else {
            0.0
        };
        let alma_spread_3h_10h_atr = if atr_h.is_finite() && atr_h > 0.0 {
            (alma_3h - alma_10h) / atr_h
        } else {
            0.0
        };
        let close_minus_alma_3h_atr = if atr_h.is_finite() && atr_h > 0.0 {
            (close - alma_3h) / atr_h
        } else {
            0.0
        };

        let log_ret_1 = log_returns[t];
        let log_ret_2 = rolling_sum(&log_return_prefix, t.saturating_sub(1), t + 1);
        let log_ret_3 = rolling_sum(&log_return_prefix, t.saturating_sub(2), t + 1);
        let log_ret_5 = rolling_sum(&log_return_prefix, t.saturating_sub(4), t + 1);
        let log_ret_h = rolling_sum(&log_return_prefix, t + 1 - h.min(t + 1), t + 1);
        let realized_vol_h = rolling_stddev(
            &log_return_prefix,
            &log_return_sq_prefix,
            t + 1 - h.min(t + 1),
            t + 1,
        );
        let three_h = h.saturating_mul(3);
        let realized_vol_3h = rolling_stddev(
            &log_return_prefix,
            &log_return_sq_prefix,
            t + 1 - three_h.min(t + 1),
            t + 1,
        );
        let atr_h_over_close = if close.is_finite() && close != 0.0 {
            atr_h / close
        } else {
            0.0
        };

        timestamps.push(candles.timestamp[t]);
        let mut column_idx = 0usize;
        let push_value =
            |columns: &mut Vec<Vec<f32>>, idx: &mut usize, value: f64| -> Result<(), String> {
                let out = if value.is_finite() { value as f32 } else { 0.0 };
                columns
                    .get_mut(*idx)
                    .ok_or_else(|| "feature column index overflow".to_string())?
                    .push(out);
                *idx = idx.saturating_add(1);
                Ok(())
            };

        push_value(&mut float_features_transposed, &mut column_idx, body_pct)?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            abs_body_frac,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            upper_wick_frac,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            lower_wick_frac,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            close_location,
        )?;
        push_value(&mut float_features_transposed, &mut column_idx, gap_pct)?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            true_range_pct,
        )?;

        for (row_idx, row) in frame.pattern_hits.iter().enumerate() {
            let hit = row[t];
            if hit != 0 {
                last_seen[row_idx] = Some(t);
            }
            let prefix = &pattern_prefix[row_idx];
            push_value(
                &mut float_features_transposed,
                &mut column_idx,
                f64::from(hit),
            )?;
            push_value(
                &mut float_features_transposed,
                &mut column_idx,
                rolling_count(prefix, t.saturating_sub(2), t + 1) as f64,
            )?;
            push_value(
                &mut float_features_transposed,
                &mut column_idx,
                rolling_count(prefix, t.saturating_sub(4), t + 1) as f64,
            )?;
            push_value(
                &mut float_features_transposed,
                &mut column_idx,
                rolling_count(prefix, t + 1 - h.min(t + 1), t + 1) as f64,
            )?;
            let two_h = h.saturating_mul(2);
            push_value(
                &mut float_features_transposed,
                &mut column_idx,
                rolling_count(prefix, t + 1 - two_h.min(t + 1), t + 1) as f64,
            )?;
            let bars_since = last_seen[row_idx]
                .map(|last| t.saturating_sub(last))
                .unwrap_or(h.saturating_mul(4));
            push_value(
                &mut float_features_transposed,
                &mut column_idx,
                bars_since.min(h.saturating_mul(4)) as f64,
            )?;
        }

        push_value(&mut float_features_transposed, &mut column_idx, alma_h)?;
        push_value(&mut float_features_transposed, &mut column_idx, alma_3h)?;
        push_value(&mut float_features_transposed, &mut column_idx, alma_10h)?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            alma_spread_h_3h_atr,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            alma_spread_3h_10h_atr,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            alma_slope_h_half_atr,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            alma_slope_3h_h_atr,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            close_minus_alma_3h_atr,
        )?;
        push_value(&mut float_features_transposed, &mut column_idx, adx_2h)?;
        push_value(&mut float_features_transposed, &mut column_idx, log_ret_1)?;
        push_value(&mut float_features_transposed, &mut column_idx, log_ret_2)?;
        push_value(&mut float_features_transposed, &mut column_idx, log_ret_3)?;
        push_value(&mut float_features_transposed, &mut column_idx, log_ret_5)?;
        push_value(&mut float_features_transposed, &mut column_idx, log_ret_h)?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            realized_vol_h,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            realized_vol_3h,
        )?;
        push_value(
            &mut float_features_transposed,
            &mut column_idx,
            atr_h_over_close,
        )?;

        if column_idx != float_features_transposed.len() {
            return Err(format!(
                "feature frame emitted {column_idx} columns but schema expects {}",
                float_features_transposed.len()
            ));
        }
    }

    Ok(MlSignalInferenceFeatureRows {
        timestamps,
        float_features_transposed,
    })
}

fn base_path_without_extension(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let raw_path = Path::new(trimmed);
    let stem = raw_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("ml_signal");
    let parent = raw_path.parent().unwrap_or_else(|| Path::new(""));
    parent.join(stem).to_string_lossy().to_string()
}

fn feature_schema_path_for_export(output_path: &str) -> String {
    format!(
        "{}.feature_schema.json",
        base_path_without_extension(output_path)
    )
}

fn export_manifest_path_for_export(output_path: &str) -> String {
    format!(
        "{}.export_manifest.json",
        base_path_without_extension(output_path)
    )
}

fn split_manifest_path_for_export(output_path: &str) -> String {
    format!(
        "{}.split_manifest.json",
        base_path_without_extension(output_path)
    )
}

fn score_manifest_path_for_cache(output_path: &str) -> String {
    format!("{}.manifest.json", base_path_without_extension(output_path))
}

fn ensure_parent_dir(path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("create parent dir: {e}"))?;
        }
    }
    Ok(())
}

fn stable_hash_mod(text: &str, modulus: usize) -> Option<usize> {
    if modulus == 0 {
        return None;
    }
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Some((hash % modulus as u64) as usize)
}

pub fn assign_ml_signal_ticker_holdout(
    ticker_id: &str,
    cfg: &MlSignalTickerHoldoutConfig,
) -> Option<bool> {
    if cfg.modulus == 0 {
        return None;
    }
    stable_hash_mod(ticker_id.trim(), cfg.modulus)
        .map(|value| value == (cfg.residue % cfg.modulus.max(1)))
}

pub fn build_ml_signal_split_windows(
    total_rows: usize,
    plan: &MlSignalSplitPlan,
) -> Result<Vec<MlSignalSplitWindow>, String> {
    if total_rows == 0 {
        return Ok(Vec::new());
    }
    if plan.train_bars == 0 || plan.calibration_bars == 0 || plan.test_bars == 0 {
        return Err("split plan requires positive train/calibration/test bars".to_string());
    }
    if plan.step_bars == 0 {
        return Err("split plan step_bars must be positive".to_string());
    }
    let window_size = plan
        .train_bars
        .saturating_add(plan.gap_bars)
        .saturating_add(plan.calibration_bars)
        .saturating_add(plan.gap_bars)
        .saturating_add(plan.test_bars);
    if total_rows < window_size {
        return Ok(Vec::new());
    }

    let mut windows = Vec::new();
    let mut start = 0usize;
    let mut split_idx = 0usize;
    while start + window_size <= total_rows {
        let train_start = start;
        let train_end = train_start + plan.train_bars;
        let calibration_start = train_end + plan.gap_bars;
        let calibration_end = calibration_start + plan.calibration_bars;
        let test_start = calibration_end + plan.gap_bars;
        let test_end = test_start + plan.test_bars;
        windows.push(MlSignalSplitWindow {
            split_id: format!("roll_{split_idx:03}"),
            train_start,
            train_end,
            calibration_start,
            calibration_end,
            test_start,
            test_end,
        });
        split_idx = split_idx.saturating_add(1);
        start = start.saturating_add(plan.step_bars);
    }
    Ok(windows)
}

fn write_json_file<T: Serialize>(path: &str, value: &T, context: &str) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let file = File::create(path).map_err(|e| format!("create {context}: {e}"))?;
    serde_json::to_writer_pretty(BufWriter::new(file), value)
        .map_err(|e| format!("write {context}: {e}"))
}

fn current_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64-{hash:016x}")
}

fn ml_signal_feature_schema_hash(schema: &MlSignalFeatureSchema) -> Result<String, String> {
    let mut hash_input = schema.clone();
    hash_input.schema_hash.clear();
    let bytes = canonical_json_key_bytes(&hash_input)?;
    Ok(fnv1a64_hex(&bytes))
}

fn json_file_fingerprint(path: &str) -> Result<Option<String>, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path_ref = Path::new(trimmed);
    if !path_ref.exists() {
        return Ok(None);
    }
    let file =
        File::open(path_ref).map_err(|e| format!("open json for fingerprint '{trimmed}': {e}"))?;
    let value: JsonValue = serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse json for fingerprint '{trimmed}': {e}"))?;
    let bytes = canonical_json_key_bytes(&value)?;
    Ok(Some(fnv1a64_hex(&bytes)))
}

#[cfg(target_os = "windows")]
const DEFAULT_CATBOOST_LIBRARY_CANDIDATES: &[&str] = &["catboostmodel.dll"];
#[cfg(target_os = "linux")]
const DEFAULT_CATBOOST_LIBRARY_CANDIDATES: &[&str] = &["libcatboostmodel.so"];
#[cfg(target_os = "macos")]
const DEFAULT_CATBOOST_LIBRARY_CANDIDATES: &[&str] = &["libcatboostmodel.dylib"];

type ModelCalcerHandle = c_void;
type ModelCalcerCreateFn = unsafe extern "C" fn() -> *mut ModelCalcerHandle;
type ModelCalcerDeleteFn = unsafe extern "C" fn(*mut ModelCalcerHandle);
type GetErrorStringFn = unsafe extern "C" fn() -> *const c_char;
type LoadFullModelFromFileFn = unsafe extern "C" fn(*mut ModelCalcerHandle, *const c_char) -> bool;
type EnableGPUEvaluationFn = unsafe extern "C" fn(*mut ModelCalcerHandle, c_int) -> bool;
type SetPredictionTypeFn = unsafe extern "C" fn(*mut ModelCalcerHandle, c_int) -> bool;
type CalcModelPredictionFlatTransposedFn = unsafe extern "C" fn(
    *mut ModelCalcerHandle,
    usize,
    *const *const f32,
    usize,
    *mut f64,
    usize,
) -> bool;
type GetFloatFeaturesCountFn = unsafe extern "C" fn(*mut ModelCalcerHandle) -> usize;

const CATBOOST_PREDICTION_RAW_FORMULA_VAL: c_int = 0;

struct CatBoostApi {
    _library: Library,
    model_calcer_create: ModelCalcerCreateFn,
    model_calcer_delete: ModelCalcerDeleteFn,
    get_error_string: GetErrorStringFn,
    load_full_model_from_file: LoadFullModelFromFileFn,
    enable_gpu_evaluation: EnableGPUEvaluationFn,
    set_prediction_type: SetPredictionTypeFn,
    calc_model_prediction_flat_transposed: CalcModelPredictionFlatTransposedFn,
    get_float_features_count: GetFloatFeaturesCountFn,
}

struct CatBoostModelHandle<'a> {
    api: &'a CatBoostApi,
    handle: *mut ModelCalcerHandle,
}

impl CatBoostApi {
    fn load(explicit_path: Option<&str>) -> Result<(Self, String), String> {
        let mut candidates: Vec<String> = Vec::new();
        if let Some(path) = explicit_path
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            candidates.push(path.to_string());
        }
        if let Ok(path) = std::env::var("CATBOOST_MODEL_INTERFACE_LIB") {
            let trimmed = path.trim();
            if !trimmed.is_empty() && !candidates.iter().any(|value| value == trimmed) {
                candidates.push(trimmed.to_string());
            }
        }
        for candidate in DEFAULT_CATBOOST_LIBRARY_CANDIDATES {
            if !candidates.iter().any(|value| value == candidate) {
                candidates.push((*candidate).to_string());
            }
        }

        let mut load_errors = Vec::new();
        for candidate in candidates {
            let library = match unsafe { Library::new(&candidate) } {
                Ok(value) => value,
                Err(err) => {
                    load_errors.push(format!("{candidate}: {err}"));
                    continue;
                }
            };
            let api =
                unsafe { Self::from_library(library) }.map_err(|err| format!("{candidate}: {err}"));
            match api {
                Ok(value) => return Ok((value, candidate)),
                Err(err) => load_errors.push(err),
            }
        }

        Err(format!(
            "failed to load CatBoost native applier library: {}",
            load_errors.join(" | ")
        ))
    }

    unsafe fn from_library(library: Library) -> Result<Self, String> {
        fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> Result<T, String> {
            let symbol = unsafe { library.get::<T>(name) }.map_err(|err| {
                format!(
                    "missing symbol {}: {err}",
                    String::from_utf8_lossy(name).trim_end_matches('\0')
                )
            })?;
            Ok(*symbol)
        }

        Ok(Self {
            model_calcer_create: load_symbol(&library, b"ModelCalcerCreate\0")?,
            model_calcer_delete: load_symbol(&library, b"ModelCalcerDelete\0")?,
            get_error_string: load_symbol(&library, b"GetErrorString\0")?,
            load_full_model_from_file: load_symbol(&library, b"LoadFullModelFromFile\0")?,
            enable_gpu_evaluation: load_symbol(&library, b"EnableGPUEvaluation\0")?,
            set_prediction_type: load_symbol(&library, b"SetPredictionType\0")?,
            calc_model_prediction_flat_transposed: load_symbol(
                &library,
                b"CalcModelPredictionFlatTransposed\0",
            )?,
            get_float_features_count: load_symbol(&library, b"GetFloatFeaturesCount\0")?,
            _library: library,
        })
    }

    fn last_error_string(&self) -> String {
        let ptr = unsafe { (self.get_error_string)() };
        if ptr.is_null() {
            "unknown CatBoost native error".to_string()
        } else {
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .trim()
                .to_string()
        }
    }
}

impl<'a> CatBoostModelHandle<'a> {
    fn new(api: &'a CatBoostApi) -> Result<Self, String> {
        let handle = unsafe { (api.model_calcer_create)() };
        if handle.is_null() {
            return Err(api.last_error_string());
        }
        Ok(Self { api, handle })
    }

    fn load_model_from_file(&self, path: &Path) -> Result<(), String> {
        let path_str = path.to_string_lossy();
        let c_path =
            CString::new(path_str.as_bytes()).map_err(|_| "model path contains NUL".to_string())?;
        let ok = unsafe { (self.api.load_full_model_from_file)(self.handle, c_path.as_ptr()) };
        if ok {
            Ok(())
        } else {
            Err(self.api.last_error_string())
        }
    }

    fn try_enable_gpu(&self, device_id: i32) -> Result<(), String> {
        let ok = unsafe { (self.api.enable_gpu_evaluation)(self.handle, device_id as c_int) };
        if ok {
            Ok(())
        } else {
            Err(self.api.last_error_string())
        }
    }

    fn float_feature_count(&self) -> usize {
        unsafe { (self.api.get_float_features_count)(self.handle) }
    }

    fn calc_raw_transposed(
        &self,
        float_features_transposed: &[Vec<f32>],
        doc_count: usize,
    ) -> Result<Vec<f64>, String> {
        let expected_features = self.float_feature_count();
        if expected_features != float_features_transposed.len() {
            return Err(format!(
                "model expects {expected_features} float features but inference frame provides {}",
                float_features_transposed.len()
            ));
        }
        if doc_count == 0 {
            return Ok(Vec::new());
        }
        for (idx, feature) in float_features_transposed.iter().enumerate() {
            if feature.len() != doc_count {
                return Err(format!(
                    "feature column {idx} length {} does not match doc_count {doc_count}",
                    feature.len()
                ));
            }
        }
        let ok = unsafe {
            (self.api.set_prediction_type)(self.handle, CATBOOST_PREDICTION_RAW_FORMULA_VAL)
        };
        if !ok {
            return Err(self.api.last_error_string());
        }
        let pointers: Vec<*const f32> = float_features_transposed
            .iter()
            .map(|column| column.as_ptr())
            .collect();
        let mut out = vec![0.0_f64; doc_count];
        let ok = unsafe {
            (self.api.calc_model_prediction_flat_transposed)(
                self.handle,
                doc_count,
                pointers.as_ptr(),
                pointers.len(),
                out.as_mut_ptr(),
                out.len(),
            )
        };
        if ok {
            Ok(out)
        } else {
            Err(self.api.last_error_string())
        }
    }
}

impl Drop for CatBoostModelHandle<'_> {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.api.model_calcer_delete)(self.handle);
            }
        }
    }
}

fn resolve_bundle_child_path(
    manifest_path: &Path,
    manifest_rel: Option<&str>,
    default_name: &str,
) -> PathBuf {
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    manifest_rel
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| base_dir.join(value))
        .unwrap_or_else(|| base_dir.join(default_name))
}

fn read_feature_schema_from_path(path: &Path) -> Result<MlSignalFeatureSchema, String> {
    let file =
        File::open(path).map_err(|e| format!("open feature schema '{}': {e}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse feature schema '{}': {e}", path.display()))
}

fn read_json_value_file(path: &Path, context: &str) -> Result<JsonValue, String> {
    let file = File::open(path).map_err(|e| format!("open {context} '{}': {e}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse {context} '{}': {e}", path.display()))
}

fn clip_probability(value: f64) -> f64 {
    value.clamp(1.0e-6, 1.0 - 1.0e-6)
}

fn raw_margin_probability(raw: f64) -> f64 {
    let clipped = raw.clamp(-40.0, 40.0);
    clip_probability(1.0 / (1.0 + (-clipped).exp()))
}

fn apply_sigmoid_calibration(raw: f64, coef: f64, intercept: f64) -> f64 {
    let margin = ((raw * coef) + intercept).clamp(-40.0, 40.0);
    clip_probability(1.0 / (1.0 + (-margin).exp()))
}

fn apply_isotonic_calibration(raw: f64, xs: &[f64], ys: &[f64]) -> Option<f64> {
    if xs.is_empty() || ys.is_empty() || xs.len() != ys.len() {
        return None;
    }
    if raw <= xs[0] {
        return Some(clip_probability(ys[0]));
    }
    if raw >= xs[xs.len() - 1] {
        return Some(clip_probability(ys[ys.len() - 1]));
    }
    match xs.binary_search_by(|probe| probe.partial_cmp(&raw).unwrap_or(std::cmp::Ordering::Less)) {
        Ok(idx) => Some(clip_probability(ys[idx])),
        Err(idx) => {
            if idx == 0 || idx >= xs.len() {
                return None;
            }
            let x0 = xs[idx - 1];
            let x1 = xs[idx];
            let y0 = ys[idx - 1];
            let y1 = ys[idx];
            let span = x1 - x0;
            if !span.is_finite() || span.abs() <= f64::EPSILON {
                return Some(clip_probability(y0));
            }
            let ratio = ((raw - x0) / span).clamp(0.0, 1.0);
            Some(clip_probability(y0 + ((y1 - y0) * ratio)))
        }
    }
}

fn apply_calibration_probability(raw: f64, config: Option<&JsonValue>) -> f64 {
    let Some(config) = config.and_then(JsonValue::as_object) else {
        return raw_margin_probability(raw);
    };
    let method = config
        .get("method")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if method == "sigmoid" || (config.contains_key("coef") && config.contains_key("intercept")) {
        let coef = config
            .get("coef")
            .and_then(JsonValue::as_f64)
            .unwrap_or(1.0);
        let intercept = config
            .get("intercept")
            .and_then(JsonValue::as_f64)
            .unwrap_or(0.0);
        return apply_sigmoid_calibration(raw, coef, intercept);
    }
    if method == "isotonic"
        || (config.contains_key("x_thresholds") && config.contains_key("y_thresholds"))
    {
        let xs: Vec<f64> = config
            .get("x_thresholds")
            .and_then(JsonValue::as_array)
            .map(|values| values.iter().filter_map(JsonValue::as_f64).collect())
            .unwrap_or_default();
        let ys: Vec<f64> = config
            .get("y_thresholds")
            .and_then(JsonValue::as_array)
            .map(|values| values.iter().filter_map(JsonValue::as_f64).collect())
            .unwrap_or_default();
        if let Some(value) = apply_isotonic_calibration(raw, &xs, &ys) {
            return value;
        }
    }
    raw_margin_probability(raw)
}

fn build_ml_signal_indicator_config(
    ma_source: &str,
    horizon_bars: usize,
) -> MlSignalIndicatorConfig {
    MlSignalIndicatorConfig {
        ma_source: ma_source.trim().to_string(),
        horizon_bars,
        alma_periods: vec![
            horizon_bars.max(1),
            horizon_bars.saturating_mul(3).max(1),
            horizon_bars.saturating_mul(10).max(1),
        ],
        adx_period: horizon_bars.saturating_mul(2).max(1),
        atr_period: horizon_bars.max(1),
        pattern_semantics: "binary_hit_plus_signed_raw_indicator_columns".to_string(),
    }
}

fn build_ml_signal_label_policy(
    round_trip_cost_bps: f64,
    horizon_bars: usize,
) -> MlSignalLabelPolicy {
    MlSignalLabelPolicy {
        kind: "binary_return_above_deadband".to_string(),
        horizon_bars,
        round_trip_cost_bps,
        deadband_kind: "round_trip_cost_plus_atr_scaled_floor".to_string(),
        long_rule: "y_long = 1 when r_long > deadband".to_string(),
        short_rule: "y_short = 1 when r_short > deadband".to_string(),
        keep_rule: "keep_side = false when side_return in [0, deadband]".to_string(),
    }
}

fn build_ml_signal_warmup_policy(first_valid_bar: usize) -> MlSignalWarmupPolicy {
    MlSignalWarmupPolicy {
        kind: "feature_frame_warmup".to_string(),
        first_valid_bar,
        reason: "pattern-recognition, ALMA, ADX, ATR, and rolling-return features require warmup before labels are valid".to_string(),
    }
}

fn build_ml_signal_sample_weight_policy() -> MlSignalSampleWeightPolicy {
    MlSignalSampleWeightPolicy {
        kind: "constant_1_0_placeholder".to_string(),
        description: "export currently emits sample_weight=1.0 for every row; weighting policy is reserved for future revisions".to_string(),
    }
}

pub fn export_ml_signal_dataset_with_candles(
    req: MlSignalDatasetExportRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) -> Result<MlSignalDatasetExportResult, String> {
    if req.data_id.trim().is_empty() {
        return Err("data_id is required".to_string());
    }
    if req.output_path.trim().is_empty() {
        return Err("output_path is required".to_string());
    }
    let n = candles.close.len();
    if n < 64 {
        return Err("not enough candles for ML dataset export".to_string());
    }
    let h = req.horizon_bars.max(2);
    let frame = feature_frame(candles, &req.ma_source, h)?;
    let last_label_bar = n.saturating_sub(h.saturating_add(2));
    if frame.first_valid_bar >= last_label_bar {
        return Err("not enough candles after warmup for ML dataset export".to_string());
    }

    let feature_names = feature_names_for_schema(&frame.pattern_ids);
    let mut feature_block_versions = HashMap::new();
    feature_block_versions.insert("candlestick_context".to_string(), 1);
    feature_block_versions.insert("pattern_recognition".to_string(), 1);
    feature_block_versions.insert("trend_regime".to_string(), 1);
    feature_block_versions.insert("return_volatility".to_string(), 1);
    let mut feature_schema = MlSignalFeatureSchema {
        schema_version: ML_SIGNAL_FEATURE_SCHEMA_VERSION,
        strategy_kind: "MlSignal".to_string(),
        horizon_bars: h,
        ma_source: req.ma_source.clone(),
        first_valid_bar: frame.first_valid_bar,
        feature_names: feature_names.clone(),
        pattern_ids: frame.pattern_ids.clone(),
        schema_hash: String::new(),
        exporter_version: ML_SIGNAL_EXPORTER_VERSION.to_string(),
        feature_block_versions,
        indicator_config: Some(build_ml_signal_indicator_config(&req.ma_source, h)),
        label_horizon_bars: h,
        first_valid_reasoning: Some(
            "first_valid_bar is set by feature_frame warmup requirements before label generation begins"
                .to_string(),
        ),
    };
    feature_schema.schema_hash = ml_signal_feature_schema_hash(&feature_schema)?;
    let schema_path = feature_schema_path_for_export(&req.output_path);
    write_json_file(&schema_path, &feature_schema, "feature schema")?;

    let split_windows = if let Some(plan) = req.split_plan.as_ref() {
        build_ml_signal_split_windows(last_label_bar + 1, plan)?
    } else {
        Vec::new()
    };
    let split_manifest_path = if let Some(plan) = req.split_plan.as_ref() {
        let mut split_manifest = plan.clone();
        split_manifest.version = ML_SIGNAL_SPLIT_PLAN_VERSION;
        split_manifest.windows = split_windows.clone();
        let path = split_manifest_path_for_export(&req.output_path);
        write_json_file(&path, &split_manifest, "split manifest")?;
        Some(path)
    } else {
        None
    };
    let ticker_holdout = req
        .ticker_holdout
        .as_ref()
        .and_then(|cfg| assign_ml_signal_ticker_holdout(&req.data_id, cfg));

    ensure_parent_dir(&req.output_path)?;
    let file = File::create(&req.output_path).map_err(|e| format!("create dataset csv: {e}"))?;
    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_writer(BufWriter::new(file));

    let mut header: Vec<String> = vec![
        "timestamp".to_string(),
        "ticker_id".to_string(),
        "split_id".to_string(),
        "sample_weight".to_string(),
    ];
    header.extend(feature_names.clone());
    header.extend([
        "r_long".to_string(),
        "r_short".to_string(),
        "y_long".to_string(),
        "y_short".to_string(),
        "keep_long".to_string(),
        "keep_short".to_string(),
    ]);
    writer
        .write_record(header.iter().map(String::as_str))
        .map_err(|e| format!("write dataset header: {e}"))?;

    let log_returns = log_returns_from_close(&candles.close);
    let log_return_prefix = prefix_sum(&log_returns);
    let log_return_sq_prefix = prefix_sum_squares(&log_returns);
    let mut pattern_prefix: Vec<Vec<usize>> = Vec::with_capacity(frame.pattern_hits.len());
    for row in &frame.pattern_hits {
        let mut prefix = vec![0usize; row.len() + 1];
        for (idx, value) in row.iter().enumerate() {
            prefix[idx + 1] = prefix[idx] + usize::from(*value != 0);
        }
        pattern_prefix.push(prefix);
    }
    let mut last_seen: Vec<Option<usize>> = vec![None; frame.pattern_hits.len()];
    for idx in 0..frame.first_valid_bar {
        for (row_idx, row) in frame.pattern_hits.iter().enumerate() {
            if row.get(idx).copied().unwrap_or(0) != 0 {
                last_seen[row_idx] = Some(idx);
            }
        }
    }

    let mut row_count = 0usize;
    let mut record: Vec<String> = Vec::with_capacity(4 + feature_names.len() + 6);
    for t in frame.first_valid_bar..=last_label_bar {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let prev_close = candles.close[t.saturating_sub(1)];
        let open = candles.open[t];
        let high = candles.high[t];
        let low = candles.low[t];
        let close = candles.close[t];
        let range = (high - low).abs();
        let eps = if close.is_finite() && close.abs() > 1.0e-12 {
            close.abs() * 1.0e-12
        } else {
            1.0e-12
        };
        let true_range = if prev_close.is_finite() {
            (high - low)
                .abs()
                .max((high - prev_close).abs())
                .max((low - prev_close).abs())
        } else {
            (high - low).abs()
        };
        let body_pct = if open.is_finite() && open != 0.0 {
            (close - open) / open
        } else {
            0.0
        };
        let abs_body_frac = (close - open).abs() / (range + eps);
        let upper_wick_frac = (high - open.max(close)).max(0.0) / (range + eps);
        let lower_wick_frac = (open.min(close) - low).max(0.0) / (range + eps);
        let close_location = (close - low) / (range + eps);
        let gap_pct = if prev_close.is_finite() && prev_close != 0.0 {
            (open - prev_close) / prev_close
        } else {
            0.0
        };
        let true_range_pct = if prev_close.is_finite() && prev_close != 0.0 {
            true_range / prev_close
        } else {
            0.0
        };

        let atr_h = frame.atr_h[t];
        let alma_h = frame.alma_h[t];
        let alma_3h = frame.alma_3h[t];
        let alma_10h = frame.alma_10h[t];
        let adx_2h = frame.adx_2h[t];
        let half_h = (h / 2).max(1);
        let alma_slope_h_half_atr = if t >= half_h && atr_h.is_finite() && atr_h > 0.0 {
            (frame.alma_h[t] - frame.alma_h[t - half_h]) / atr_h
        } else {
            0.0
        };
        let alma_slope_3h_h_atr = if t >= h && atr_h.is_finite() && atr_h > 0.0 {
            (frame.alma_3h[t] - frame.alma_3h[t - h]) / atr_h
        } else {
            0.0
        };
        let alma_spread_h_3h_atr = if atr_h.is_finite() && atr_h > 0.0 {
            (alma_h - alma_3h) / atr_h
        } else {
            0.0
        };
        let alma_spread_3h_10h_atr = if atr_h.is_finite() && atr_h > 0.0 {
            (alma_3h - alma_10h) / atr_h
        } else {
            0.0
        };
        let close_minus_alma_3h_atr = if atr_h.is_finite() && atr_h > 0.0 {
            (close - alma_3h) / atr_h
        } else {
            0.0
        };

        let log_ret_1 = log_returns[t];
        let log_ret_2 = rolling_sum(&log_return_prefix, t.saturating_sub(1), t + 1);
        let log_ret_3 = rolling_sum(&log_return_prefix, t.saturating_sub(2), t + 1);
        let log_ret_5 = rolling_sum(&log_return_prefix, t.saturating_sub(4), t + 1);
        let log_ret_h = rolling_sum(&log_return_prefix, t + 1 - h.min(t + 1), t + 1);
        let realized_vol_h = rolling_stddev(
            &log_return_prefix,
            &log_return_sq_prefix,
            t + 1 - h.min(t + 1),
            t + 1,
        );
        let three_h = h.saturating_mul(3);
        let realized_vol_3h = rolling_stddev(
            &log_return_prefix,
            &log_return_sq_prefix,
            t + 1 - three_h.min(t + 1),
            t + 1,
        );
        let atr_h_over_close = if close.is_finite() && close != 0.0 {
            atr_h / close
        } else {
            0.0
        };

        let open_next = candles.open[t + 1];
        let open_exit = candles.open[t + h + 1];
        let r_long = if open_next.is_finite() && open_exit.is_finite() && open_next > 0.0 {
            (open_exit - open_next) / open_next
        } else {
            0.0
        };
        let r_short = if open_next.is_finite() && open_exit.is_finite() && open_next > 0.0 {
            (open_next - open_exit) / open_next
        } else {
            0.0
        };
        let deadband = sample_deadband(req.round_trip_cost_bps, atr_h, open_next);
        let y_long = if r_long > deadband { 1 } else { 0 };
        let y_short = if r_short > deadband { 1 } else { 0 };
        let keep_long = !((0.0..=deadband).contains(&r_long));
        let keep_short = !((0.0..=deadband).contains(&r_short));

        let split_id = split_windows
            .iter()
            .find(|window| t >= window.train_start && t < window.test_end)
            .map(|window| window.split_id.clone())
            .unwrap_or_default();

        record.clear();
        record.push(candles.timestamp[t].to_string());
        record.push(req.data_id.clone());
        record.push(split_id);
        record.push("1.0".to_string());
        record.push(format_f64(body_pct));
        record.push(format_f64(abs_body_frac));
        record.push(format_f64(upper_wick_frac));
        record.push(format_f64(lower_wick_frac));
        record.push(format_f64(close_location));
        record.push(format_f64(gap_pct));
        record.push(format_f64(true_range_pct));

        for (row_idx, row) in frame.pattern_hits.iter().enumerate() {
            let hit = row[t];
            if hit != 0 {
                last_seen[row_idx] = Some(t);
            }
            let prefix = &pattern_prefix[row_idx];
            record.push(hit.to_string());
            record.push(rolling_count(prefix, t.saturating_sub(2), t + 1).to_string());
            record.push(rolling_count(prefix, t.saturating_sub(4), t + 1).to_string());
            record.push(rolling_count(prefix, t + 1 - h.min(t + 1), t + 1).to_string());
            let two_h = h.saturating_mul(2);
            record.push(rolling_count(prefix, t + 1 - two_h.min(t + 1), t + 1).to_string());
            let bars_since = last_seen[row_idx]
                .map(|last| t.saturating_sub(last))
                .unwrap_or(h.saturating_mul(4));
            record.push(bars_since.min(h.saturating_mul(4)).to_string());
        }

        record.extend([
            format_f64(alma_h),
            format_f64(alma_3h),
            format_f64(alma_10h),
            format_f64(alma_spread_h_3h_atr),
            format_f64(alma_spread_3h_10h_atr),
            format_f64(alma_slope_h_half_atr),
            format_f64(alma_slope_3h_h_atr),
            format_f64(close_minus_alma_3h_atr),
            format_f64(adx_2h),
            format_f64(log_ret_1),
            format_f64(log_ret_2),
            format_f64(log_ret_3),
            format_f64(log_ret_5),
            format_f64(log_ret_h),
            format_f64(realized_vol_h),
            format_f64(realized_vol_3h),
            format_f64(atr_h_over_close),
            format_f64(r_long),
            format_f64(r_short),
            y_long.to_string(),
            y_short.to_string(),
            if keep_long { "1" } else { "0" }.to_string(),
            if keep_short { "1" } else { "0" }.to_string(),
        ]);

        writer
            .write_record(record.iter().map(String::as_str))
            .map_err(|e| format!("write dataset row: {e}"))?;
        row_count += 1;
    }
    writer
        .flush()
        .map_err(|e| format!("flush dataset csv: {e}"))?;

    let export_manifest = MlSignalDatasetExportManifest {
        version: ML_SIGNAL_EXPORT_MANIFEST_VERSION,
        strategy_kind: "MlSignal".to_string(),
        data_id: req.data_id.clone(),
        output_path: req.output_path.clone(),
        row_count,
        feature_schema_path: schema_path.clone(),
        feature_schema_version: ML_SIGNAL_FEATURE_SCHEMA_VERSION,
        split_manifest_path: split_manifest_path.clone(),
        split_count: split_windows.len(),
        horizon_bars: h,
        first_valid_bar: frame.first_valid_bar,
        pattern_count: frame.pattern_ids.len(),
        feature_count: feature_names.len(),
        first_timestamp: candles.timestamp.first().copied(),
        last_timestamp: candles.timestamp.last().copied(),
        ticker_holdout,
        feature_schema_hash: feature_schema.schema_hash.clone(),
        exporter_version: ML_SIGNAL_EXPORTER_VERSION.to_string(),
        indicator_config: feature_schema.indicator_config.clone(),
        warmup_policy: Some(build_ml_signal_warmup_policy(frame.first_valid_bar)),
        label_policy: Some(build_ml_signal_label_policy(req.round_trip_cost_bps, h)),
        sample_weight_policy: Some(build_ml_signal_sample_weight_policy()),
        created_at_unix_ms: Some(current_unix_ms()),
    };
    let export_manifest_path = export_manifest_path_for_export(&req.output_path);
    write_json_file(&export_manifest_path, &export_manifest, "export manifest")?;

    Ok(MlSignalDatasetExportResult {
        data_id: req.data_id,
        output_path: req.output_path,
        feature_schema_path: schema_path,
        export_manifest_path,
        split_manifest_path,
        row_count,
        feature_count: feature_names.len(),
        feature_schema_version: ML_SIGNAL_FEATURE_SCHEMA_VERSION,
        feature_schema_hash: feature_schema.schema_hash,
        pattern_count: frame.pattern_ids.len(),
        first_valid_bar: frame.first_valid_bar,
        horizon_bars: h,
        split_count: split_windows.len(),
        ticker_holdout,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MlSignalScoreCacheRow {
    timestamp: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    p_long: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    p_short: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw_long: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw_short: Option<f64>,
}

fn manifest_threshold_policy_kind(manifest: &MlSignalModelBundleManifest) -> Option<String> {
    manifest
        .threshold_policy_default
        .as_ref()
        .and_then(JsonValue::as_object)
        .and_then(|value| value.get("kind").or_else(|| value.get("threshold_policy")))
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn manifest_score_contract_version(manifest: &MlSignalModelBundleManifest) -> Option<u32> {
    manifest
        .score_contract
        .as_ref()
        .and_then(JsonValue::as_object)
        .and_then(|value| {
            value
                .get("score_contract_version")
                .or_else(|| value.get("scoreContractVersion"))
        })
        .and_then(JsonValue::as_u64)
        .map(|value| value as u32)
}

fn manifest_calibration_state(manifest: &MlSignalModelBundleManifest) -> Option<String> {
    manifest
        .calibration_summary
        .as_ref()
        .and_then(JsonValue::as_object)
        .and_then(|value| {
            value
                .get("calibration_state")
                .or_else(|| value.get("calibrationState"))
                .or_else(|| value.get("status"))
                .or_else(|| value.get("kind"))
        })
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            manifest
                .calibration_path
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|_| "bundle_calibration_attached".to_string())
        })
}

fn build_ml_signal_native_score_cache(
    req: &MlSignalScoreCacheBuildRequest,
    candles: &Candles,
    _cancel: &AtomicBool,
) -> Result<MlSignalNativeScoreBuild, String> {
    let manifest_path = req
        .source_bundle_manifest_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            "native score-cache generation requires a resolved source bundle manifest path"
                .to_string()
        })?;
    let manifest = read_ml_signal_manifest(&manifest_path)?;
    if manifest.model_family.trim() != "catboost_binary_pair" {
        return Err(format!(
            "native MlSignal inference supports model_family 'catboost_binary_pair', got '{}'",
            manifest.model_family
        ));
    }
    if manifest.model_format.trim() != "cbm" {
        return Err(format!(
            "native MlSignal inference supports model_format 'cbm', got '{}'",
            manifest.model_format
        ));
    }

    let feature_schema_path = resolve_bundle_child_path(
        &manifest_path,
        manifest.feature_schema_path.as_deref(),
        "feature_schema.json",
    );
    let feature_schema = read_feature_schema_from_path(&feature_schema_path)?;
    let export_manifest_path = resolve_bundle_child_path(
        &manifest_path,
        manifest.export_manifest_path.as_deref(),
        "export_manifest.json",
    );
    let calibration_path = manifest
        .calibration_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| resolve_bundle_child_path(&manifest_path, Some(value), value));
    let long_model_path = resolve_bundle_child_path(
        &manifest_path,
        manifest.long_model_path.as_deref(),
        "model_long.cbm",
    );
    let short_model_path = resolve_bundle_child_path(
        &manifest_path,
        manifest.short_model_path.as_deref(),
        "model_short.cbm",
    );

    let inference_rows = build_ml_signal_inference_feature_rows(candles, &feature_schema)?;
    let calibration_json = calibration_path
        .as_ref()
        .filter(|path| path.exists())
        .map(|path| read_json_value_file(path, "calibration"))
        .transpose()?;
    let long_calibration = calibration_json
        .as_ref()
        .and_then(|value| value.get("long"));
    let short_calibration = calibration_json
        .as_ref()
        .and_then(|value| value.get("short"));

    let backend_requested = req.native_backend_preference;
    let (api, library_path) = CatBoostApi::load(req.native_library_path.as_deref())?;
    let long_model = CatBoostModelHandle::new(&api)?;
    long_model.load_model_from_file(&long_model_path)?;
    let short_model = CatBoostModelHandle::new(&api)?;
    short_model.load_model_from_file(&short_model_path)?;

    let requested_gpu_id = req.native_gpu_device_id.unwrap_or(0);
    let mut backend_used = MlSignalInferenceBackendUsed::NativeCpu;
    let mut fallback_reason = None;
    match backend_requested {
        MlSignalInferenceBackendPreference::NativeCuda
        | MlSignalInferenceBackendPreference::Auto => {
            let long_gpu = long_model.try_enable_gpu(requested_gpu_id);
            let short_gpu = short_model.try_enable_gpu(requested_gpu_id);
            match (long_gpu, short_gpu) {
                (Ok(()), Ok(())) => {
                    backend_used = MlSignalInferenceBackendUsed::NativeCuda;
                }
                (left, right) => {
                    if backend_requested == MlSignalInferenceBackendPreference::NativeCuda {
                        fallback_reason = Some(format!(
                            "native CUDA evaluation unavailable; long={:?} short={:?}; falling back to CPU",
                            left.err(),
                            right.err()
                        ));
                    } else if left.is_err() || right.is_err() {
                        fallback_reason = Some(
                            "native CUDA evaluation unavailable; falling back to CPU".to_string(),
                        );
                    }
                }
            }
        }
        MlSignalInferenceBackendPreference::NativeCpu
        | MlSignalInferenceBackendPreference::ScoreCacheOnly => {}
    }

    let raw_long = long_model.calc_raw_transposed(
        &inference_rows.float_features_transposed,
        inference_rows.timestamps.len(),
    )?;
    let raw_short = short_model.calc_raw_transposed(
        &inference_rows.float_features_transposed,
        inference_rows.timestamps.len(),
    )?;
    if raw_long.len() != inference_rows.timestamps.len()
        || raw_short.len() != inference_rows.timestamps.len()
    {
        return Err(
            "native CatBoost inference returned row counts that do not match timestamps"
                .to_string(),
        );
    }

    let rows: Vec<MlSignalScoreCacheRow> = inference_rows
        .timestamps
        .iter()
        .enumerate()
        .map(|(idx, timestamp)| {
            let long_raw = raw_long[idx];
            let short_raw = raw_short[idx];
            MlSignalScoreCacheRow {
                timestamp: *timestamp,
                p_long: Some(apply_calibration_probability(long_raw, long_calibration)),
                p_short: Some(apply_calibration_probability(short_raw, short_calibration)),
                raw_long: Some(long_raw),
                raw_short: Some(short_raw),
            }
        })
        .collect();

    Ok(MlSignalNativeScoreBuild {
        rows,
        feature_schema_path: Some(feature_schema_path.to_string_lossy().to_string()),
        feature_schema_hash: if feature_schema.schema_hash.trim().is_empty() {
            manifest.source_feature_schema_hash.clone()
        } else {
            Some(feature_schema.schema_hash.clone())
        },
        export_manifest_path: export_manifest_path
            .exists()
            .then(|| export_manifest_path.to_string_lossy().to_string()),
        source_bundle_manifest_path: Some(manifest_path.to_string_lossy().to_string()),
        source_bundle_manifest_hash: json_file_fingerprint(&manifest_path.to_string_lossy())?,
        export_manifest_hash: if export_manifest_path.exists() {
            json_file_fingerprint(&export_manifest_path.to_string_lossy())?
        } else {
            None
        },
        calibration_state: manifest_calibration_state(&manifest),
        threshold_policy: manifest_threshold_policy_kind(&manifest),
        producer_kind: Some(
            match backend_used {
                MlSignalInferenceBackendUsed::NativeCpu => "catboost_native_cpu",
                MlSignalInferenceBackendUsed::NativeCuda => "catboost_native_cuda",
                MlSignalInferenceBackendUsed::ScoreCacheOnly => "score_cache_only",
            }
            .to_string(),
        ),
        score_contract_version: manifest_score_contract_version(&manifest),
        native: MlSignalNativeBuildMetadata {
            backend_requested,
            backend_used,
            library_path: Some(library_path),
            gpu_device_id: (backend_used == MlSignalInferenceBackendUsed::NativeCuda)
                .then_some(requested_gpu_id),
            fallback_reason,
        },
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MlSignalScoreCacheFile {
    version: u32,
    kind: String,
    strategy_kind: String,
    feature_schema_version: u32,
    data_id: String,
    input_path: String,
    #[serde(default = "default_ml_signal_score_space")]
    score_space: MlSignalScoreSpace,
    #[serde(default = "default_ml_signal_score_scope")]
    score_scope: MlSignalScoreScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_bundle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    feature_schema_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    export_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_bundle_manifest_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_bundle_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    export_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    calibration_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    threshold_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    producer_kind: Option<String>,
    #[serde(default = "default_ml_signal_score_contract_version")]
    score_contract_version: u32,
    matched_rows: usize,
    skipped_rows: usize,
    bars_scored: usize,
    #[serde(default)]
    duplicate_timestamps: usize,
    #[serde(default)]
    coverage_ratio: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    first_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_backend_requested: Option<MlSignalInferenceBackendPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_backend_used: Option<MlSignalInferenceBackendUsed>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_library_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_gpu_device_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    native_fallback_reason: Option<String>,
    scores: Vec<MlSignalScoreCacheRow>,
}

#[derive(Clone, Debug)]
struct LoadedScoreCache {
    points: Vec<Option<ScorePoint>>,
    summary: MlSignalScoreCacheSummary,
    has_probability_long: bool,
    has_probability_short: bool,
    rows_with_probability: usize,
    rows_missing_long_probability: usize,
    rows_missing_short_probability: usize,
}

fn find_header_index(headers: &StringRecord, names: &[&str]) -> Option<usize> {
    headers.iter().enumerate().find_map(|(idx, header)| {
        let normalized = header.trim().to_ascii_lowercase();
        if names.iter().any(|name| normalized == *name) {
            Some(idx)
        } else {
            None
        }
    })
}

fn find_preferred_header(headers: &StringRecord, names: &[&str]) -> Option<(usize, String)> {
    for name in names {
        if let Some(idx) = find_header_index(headers, &[*name]) {
            return Some((idx, (*name).to_string()));
        }
    }
    None
}

fn parse_i64_cell(record: &StringRecord, idx: Option<usize>) -> Result<Option<i64>, String> {
    let Some(idx) = idx else {
        return Ok(None);
    };
    let Some(raw) = record.get(idx) else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i64>()
        .map(Some)
        .map_err(|e| format!("invalid i64 '{trimmed}': {e}"))
}

fn parse_f64_cell(record: &StringRecord, idx: Option<usize>) -> Result<Option<f64>, String> {
    let Some(idx) = idx else {
        return Ok(None);
    };
    let Some(raw) = record.get(idx) else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = trimmed
        .parse::<f64>()
        .map_err(|e| format!("invalid f64 '{trimmed}': {e}"))?;
    if parsed.is_finite() {
        Ok(Some(parsed))
    } else {
        Ok(None)
    }
}

fn parse_text_cell(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    let idx = idx?;
    let raw = record.get(idx)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn normalize_score_scope_text(raw: &str) -> Option<MlSignalScoreScope> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "full_dataset" | "full-dataset" | "fulldataset" | "full dataset" => {
            Some(MlSignalScoreScope::FullDataset)
        }
        "calibration_slice" | "calibration-slice" | "calibrationslice" | "calibration slice" => {
            Some(MlSignalScoreScope::CalibrationSlice)
        }
        "walkforward_test" | "walkforward-test" | "walkforwardtest" | "walkforward test"
        | "walk_forward_test" | "walk-forward-test" => Some(MlSignalScoreScope::WalkforwardTest),
        "custom_partial" | "custom-partial" | "custompartial" | "custom partial" => {
            Some(MlSignalScoreScope::CustomPartial)
        }
        _ => None,
    }
}

fn infer_score_scope(
    input_path: &str,
    requested_scope: Option<MlSignalScoreScope>,
    matched_rows: usize,
    total_bars: usize,
) -> MlSignalScoreScope {
    if let Some(scope) = requested_scope {
        return scope;
    }
    let lower = Path::new(input_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(input_path)
        .to_ascii_lowercase();
    if lower.contains("validation_scores") {
        return MlSignalScoreScope::CalibrationSlice;
    }
    if lower.contains("walkforward_test_scores") {
        return MlSignalScoreScope::WalkforwardTest;
    }
    if total_bars > 0 && matched_rows == total_bars {
        return MlSignalScoreScope::FullDataset;
    }
    MlSignalScoreScope::CustomPartial
}

fn read_score_rows_csv(input_path: &str) -> Result<ParsedScoreCsv, String> {
    let file = File::open(input_path).map_err(|e| format!("open score csv: {e}"))?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(file));
    let headers = reader
        .headers()
        .map_err(|e| format!("read score csv headers: {e}"))?
        .clone();

    let ts_idx = find_header_index(&headers, &["timestamp", "ts", "time"])
        .ok_or_else(|| "score csv requires a timestamp column".to_string())?;
    let p_long_match = find_preferred_header(
        &headers,
        &["p_long_cal", "p_long", "prob_long", "long_probability"],
    );
    let p_short_match = find_preferred_header(
        &headers,
        &["p_short_cal", "p_short", "prob_short", "short_probability"],
    );
    let p_long_idx = p_long_match.as_ref().map(|(idx, _)| *idx);
    let p_short_idx = p_short_match.as_ref().map(|(idx, _)| *idx);
    let raw_long_idx = find_header_index(&headers, &["raw_long", "score_long", "margin_long"]);
    let raw_short_idx = find_header_index(&headers, &["raw_short", "score_short", "margin_short"]);
    let scope_idx = find_header_index(&headers, &["score_scope", "scope"]);

    if p_long_idx.is_none()
        && p_short_idx.is_none()
        && raw_long_idx.is_none()
        && raw_short_idx.is_none()
    {
        return Err(
            "score csv requires at least one of p_long, p_short, raw_long, raw_short".to_string(),
        );
    }

    let score_space = if p_long_idx.is_some() || p_short_idx.is_some() {
        MlSignalScoreSpace::Probability
    } else {
        MlSignalScoreSpace::RawMargin
    };
    let mut rows = Vec::new();
    let mut score_scope = None;
    for (line_idx, row) in reader.records().enumerate() {
        let record = row.map_err(|e| format!("read score csv row {}: {e}", line_idx + 2))?;
        let timestamp = parse_i64_cell(&record, Some(ts_idx))
            .map_err(|e| format!("score csv row {} timestamp: {e}", line_idx + 2))?
            .ok_or_else(|| format!("score csv row {} missing timestamp", line_idx + 2))?;
        let p_long = parse_f64_cell(&record, p_long_idx)
            .map_err(|e| format!("score csv row {} p_long: {e}", line_idx + 2))?;
        let p_short = parse_f64_cell(&record, p_short_idx)
            .map_err(|e| format!("score csv row {} p_short: {e}", line_idx + 2))?;
        let raw_long = parse_f64_cell(&record, raw_long_idx)
            .map_err(|e| format!("score csv row {} raw_long: {e}", line_idx + 2))?;
        let raw_short = parse_f64_cell(&record, raw_short_idx)
            .map_err(|e| format!("score csv row {} raw_short: {e}", line_idx + 2))?;
        if score_scope.is_none() {
            score_scope = parse_text_cell(&record, scope_idx)
                .as_deref()
                .and_then(normalize_score_scope_text);
        }

        if p_long.is_none() && p_short.is_none() && raw_long.is_none() && raw_short.is_none() {
            continue;
        }

        rows.push(ParsedScoreRow {
            timestamp,
            p_long,
            p_short,
            raw_long,
            raw_short,
        });
    }
    Ok(ParsedScoreCsv {
        rows,
        score_space,
        score_scope,
    })
}

fn write_ml_signal_score_cache(
    req: MlSignalScoreCacheBuildRequest,
    candles: &Candles,
    score_space: MlSignalScoreSpace,
    requested_scope: Option<MlSignalScoreScope>,
    rows: Vec<MlSignalScoreCacheRow>,
    feature_schema_path: Option<String>,
    feature_schema_hash: Option<String>,
    export_manifest_path: Option<String>,
    source_bundle_manifest_path: Option<String>,
    source_bundle_manifest_hash: Option<String>,
    export_manifest_hash: Option<String>,
    calibration_state: Option<String>,
    threshold_policy: Option<String>,
    producer_kind: Option<String>,
    score_contract_version: Option<u32>,
    native_metadata: Option<MlSignalNativeBuildMetadata>,
) -> Result<MlSignalScoreCacheBuildResult, String> {
    let timestamp_to_index: HashMap<i64, usize> = candles
        .timestamp
        .iter()
        .enumerate()
        .map(|(idx, ts)| (*ts, idx))
        .collect();
    let mut matched: HashMap<i64, MlSignalScoreCacheRow> = HashMap::new();
    let mut skipped_rows = 0usize;
    let mut duplicate_timestamps = 0usize;
    let mut seen_timestamps = HashSet::new();

    for row in rows {
        if !seen_timestamps.insert(row.timestamp) {
            duplicate_timestamps = duplicate_timestamps.saturating_add(1);
        }
        if !timestamp_to_index.contains_key(&row.timestamp) {
            skipped_rows = skipped_rows.saturating_add(1);
            continue;
        }
        matched.insert(row.timestamp, row);
    }

    let mut scores: Vec<MlSignalScoreCacheRow> = matched.into_values().collect();
    scores.sort_by_key(|row| row.timestamp);
    let bars_scored = scores.len();
    let coverage_ratio = if candles.close.is_empty() {
        0.0
    } else {
        (bars_scored as f64 / candles.close.len() as f64).clamp(0.0, 1.0)
    };
    let score_scope = infer_score_scope(
        &req.input_path,
        requested_scope,
        bars_scored,
        candles.close.len(),
    );
    let native_backend_requested = native_metadata.as_ref().map(|meta| meta.backend_requested);
    let native_backend_used = native_metadata.as_ref().map(|meta| meta.backend_used);
    let native_library_path = native_metadata
        .as_ref()
        .and_then(|meta| meta.library_path.clone());
    let native_gpu_device_id = native_metadata.as_ref().and_then(|meta| meta.gpu_device_id);
    let native_fallback_reason = native_metadata
        .as_ref()
        .and_then(|meta| meta.fallback_reason.clone());

    let cache = MlSignalScoreCacheFile {
        version: ML_SIGNAL_SCORE_CACHE_VERSION,
        kind: "vectorgrid_ml_signal_score_cache".to_string(),
        strategy_kind: "MlSignal".to_string(),
        feature_schema_version: ML_SIGNAL_FEATURE_SCHEMA_VERSION,
        data_id: req.data_id.clone(),
        input_path: req.input_path.clone(),
        score_space,
        score_scope,
        model_bundle_id: req.model_bundle_id.clone(),
        feature_schema_hash: feature_schema_hash.clone(),
        export_manifest_path: export_manifest_path.clone(),
        source_bundle_manifest_path: source_bundle_manifest_path.clone(),
        source_bundle_manifest_hash: source_bundle_manifest_hash.clone(),
        export_manifest_hash: export_manifest_hash.clone(),
        calibration_state: calibration_state.clone(),
        threshold_policy: threshold_policy.clone(),
        producer_kind: producer_kind.clone(),
        score_contract_version: score_contract_version.unwrap_or(ML_SIGNAL_SCORE_CONTRACT_VERSION),
        matched_rows: bars_scored,
        skipped_rows,
        bars_scored,
        duplicate_timestamps,
        coverage_ratio,
        first_timestamp: scores.first().map(|row| row.timestamp),
        last_timestamp: scores.last().map(|row| row.timestamp),
        native_backend_requested,
        native_backend_used,
        native_library_path: native_library_path.clone(),
        native_gpu_device_id,
        native_fallback_reason: native_fallback_reason.clone(),
        scores,
    };
    ensure_parent_dir(&req.output_path)?;
    let file = File::create(&req.output_path).map_err(|e| format!("create score cache: {e}"))?;
    serde_json::to_writer_pretty(BufWriter::new(file), &cache)
        .map_err(|e| format!("write score cache json: {e}"))?;

    let manifest = MlSignalScoreCacheManifest {
        version: cache.version,
        kind: cache.kind.clone(),
        strategy_kind: cache.strategy_kind.clone(),
        data_id: cache.data_id.clone(),
        input_path: cache.input_path.clone(),
        feature_schema_version: cache.feature_schema_version,
        score_space: cache.score_space,
        score_scope: cache.score_scope,
        feature_schema_path: feature_schema_path.clone(),
        model_bundle_id: cache.model_bundle_id.clone(),
        feature_schema_hash: cache.feature_schema_hash.clone(),
        export_manifest_path: cache.export_manifest_path.clone(),
        source_bundle_manifest_path: cache.source_bundle_manifest_path.clone(),
        source_bundle_manifest_hash: cache.source_bundle_manifest_hash.clone(),
        export_manifest_hash: cache.export_manifest_hash.clone(),
        calibration_state: cache.calibration_state.clone(),
        threshold_policy: cache.threshold_policy.clone(),
        producer_kind: cache.producer_kind.clone(),
        score_contract_version: cache.score_contract_version,
        matched_rows: cache.matched_rows,
        skipped_rows: cache.skipped_rows,
        bars_scored: cache.bars_scored,
        duplicate_timestamps: cache.duplicate_timestamps,
        coverage_ratio: cache.coverage_ratio,
        first_timestamp: cache.first_timestamp,
        last_timestamp: cache.last_timestamp,
        native_backend_requested,
        native_backend_used,
        native_library_path: native_library_path.clone(),
        native_gpu_device_id,
        native_fallback_reason: native_fallback_reason.clone(),
    };
    let manifest_path = score_manifest_path_for_cache(&req.output_path);
    write_json_file(&manifest_path, &manifest, "score cache manifest")?;

    Ok(MlSignalScoreCacheBuildResult {
        data_id: req.data_id,
        input_path: req.input_path,
        output_path: req.output_path,
        manifest_path,
        feature_schema_path,
        export_manifest_path: cache.export_manifest_path.clone(),
        source_bundle_manifest_path: cache.source_bundle_manifest_path.clone(),
        matched_rows: cache.matched_rows,
        skipped_rows: cache.skipped_rows,
        bars_scored: cache.bars_scored,
        duplicate_timestamps: cache.duplicate_timestamps,
        coverage_ratio: cache.coverage_ratio,
        score_space: cache.score_space,
        score_scope: cache.score_scope,
        first_timestamp: cache.first_timestamp,
        last_timestamp: cache.last_timestamp,
        model_bundle_id: req.model_bundle_id,
        version: cache.version,
        feature_schema_hash: cache.feature_schema_hash,
        calibration_state: cache.calibration_state,
        threshold_policy: cache.threshold_policy,
        producer_kind: cache.producer_kind,
        score_contract_version: Some(cache.score_contract_version),
        source_bundle_manifest_hash: cache.source_bundle_manifest_hash,
        export_manifest_hash: cache.export_manifest_hash,
        native_backend_requested,
        native_backend_used,
        native_library_path,
        native_gpu_device_id,
        native_fallback_reason,
    })
}

pub fn build_ml_signal_score_cache_from_csv(
    req: MlSignalScoreCacheBuildRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) -> Result<MlSignalScoreCacheBuildResult, String> {
    if req.data_id.trim().is_empty() {
        return Err("data_id is required".to_string());
    }
    if req.input_path.trim().is_empty()
        && req.native_backend_preference == MlSignalInferenceBackendPreference::ScoreCacheOnly
    {
        return Err("input_path is required".to_string());
    }
    if req.output_path.trim().is_empty() {
        return Err("output_path is required".to_string());
    }

    if req.input_path.trim().is_empty() {
        let native = build_ml_signal_native_score_cache(&req, candles, cancel)?;
        return write_ml_signal_score_cache(
            req,
            candles,
            MlSignalScoreSpace::Probability,
            Some(MlSignalScoreScope::CustomPartial),
            native.rows,
            native.feature_schema_path,
            native.feature_schema_hash,
            native.export_manifest_path,
            native.source_bundle_manifest_path,
            native.source_bundle_manifest_hash,
            native.export_manifest_hash,
            native.calibration_state,
            native.threshold_policy,
            native.producer_kind,
            native.score_contract_version,
            Some(native.native),
        );
    }

    let feature_schema_hash = if let Some(hash) = req
        .feature_schema_hash
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        Some(hash.to_string())
    } else if let Some(path) = req
        .feature_schema_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        json_file_fingerprint(path)?
    } else {
        None
    };
    let source_bundle_manifest_hash = if let Some(path) = req
        .source_bundle_manifest_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        json_file_fingerprint(path)?
    } else {
        None
    };
    let export_manifest_hash = if let Some(path) = req
        .export_manifest_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        json_file_fingerprint(path)?
    } else {
        None
    };
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }
    let parsed = read_score_rows_csv(&req.input_path)?;
    let rows: Vec<MlSignalScoreCacheRow> = parsed
        .rows
        .into_iter()
        .map(|row| MlSignalScoreCacheRow {
            timestamp: row.timestamp,
            p_long: row.p_long,
            p_short: row.p_short,
            raw_long: row.raw_long,
            raw_short: row.raw_short,
        })
        .collect();
    let feature_schema_path = req.feature_schema_path.clone();
    let export_manifest_path = req.export_manifest_path.clone();
    let source_bundle_manifest_path = req.source_bundle_manifest_path.clone();
    let calibration_state = req.calibration_state.clone();
    let threshold_policy = req.threshold_policy.clone();
    let producer_kind = req.producer_kind.clone();
    let score_contract_version = req.score_contract_version;
    write_ml_signal_score_cache(
        req,
        candles,
        parsed.score_space,
        parsed.score_scope,
        rows,
        feature_schema_path,
        feature_schema_hash,
        export_manifest_path,
        source_bundle_manifest_path,
        source_bundle_manifest_hash,
        export_manifest_hash,
        calibration_state,
        threshold_policy,
        producer_kind,
        score_contract_version,
        None,
    )
}

fn load_score_cache(
    score_cache_path: &str,
    expected_data_id: &str,
    candles: &Candles,
) -> Result<LoadedScoreCache, String> {
    let file = File::open(score_cache_path).map_err(|e| format!("open score cache: {e}"))?;
    let cache: MlSignalScoreCacheFile = serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse score cache json: {e}"))?;

    if cache.version != ML_SIGNAL_SCORE_CACHE_VERSION {
        return Err(format!(
            "unsupported score cache version {} (expected {})",
            cache.version, ML_SIGNAL_SCORE_CACHE_VERSION
        ));
    }
    if cache.kind.trim() != "vectorgrid_ml_signal_score_cache" {
        return Err(format!("unsupported score cache kind '{}'", cache.kind));
    }
    if cache.strategy_kind.trim() != "MlSignal" {
        return Err(format!(
            "score cache strategy_kind '{}' does not match MlSignal",
            cache.strategy_kind
        ));
    }
    if cache.feature_schema_version != ML_SIGNAL_FEATURE_SCHEMA_VERSION {
        return Err(format!(
            "score cache feature schema version {} does not match expected {}",
            cache.feature_schema_version, ML_SIGNAL_FEATURE_SCHEMA_VERSION
        ));
    }
    if !expected_data_id.trim().is_empty() && cache.data_id.trim() != expected_data_id.trim() {
        return Err(format!(
            "score cache data_id '{}' does not match requested '{}'",
            cache.data_id, expected_data_id
        ));
    }
    if cache.duplicate_timestamps > 0 {
        return Err(format!(
            "score cache contains {} duplicate timestamps; rebuild from deduplicated scores",
            cache.duplicate_timestamps
        ));
    }

    let mut out = vec![None; candles.close.len()];
    let mut has_probability_long = false;
    let mut has_probability_short = false;
    let mut rows_with_probability = 0usize;
    let mut rows_missing_long_probability = 0usize;
    let mut rows_missing_short_probability = 0usize;
    let mut candle_idx = 0usize;
    for row in cache.scores {
        let has_long = row.p_long.filter(|value| value.is_finite()).is_some();
        let has_short = row.p_short.filter(|value| value.is_finite()).is_some();
        if has_long {
            has_probability_long = true;
        }
        if has_short {
            has_probability_short = true;
        }
        if has_long || has_short {
            rows_with_probability = rows_with_probability.saturating_add(1);
            if !has_long {
                rows_missing_long_probability = rows_missing_long_probability.saturating_add(1);
            }
            if !has_short {
                rows_missing_short_probability = rows_missing_short_probability.saturating_add(1);
            }
        }
        while candle_idx < candles.timestamp.len() && candles.timestamp[candle_idx] < row.timestamp
        {
            candle_idx = candle_idx.saturating_add(1);
        }
        if candle_idx < candles.timestamp.len() && candles.timestamp[candle_idx] == row.timestamp {
            out[candle_idx] = Some(ScorePoint {
                p_long: row.p_long,
                p_short: row.p_short,
                raw_long: row.raw_long,
                raw_short: row.raw_short,
            });
        }
    }
    let coverage_ratio = if cache.coverage_ratio.is_finite() && cache.coverage_ratio > 0.0 {
        cache.coverage_ratio
    } else if candles.close.is_empty() {
        0.0
    } else {
        (cache.bars_scored as f64 / candles.close.len() as f64).clamp(0.0, 1.0)
    };
    if cache.score_scope == MlSignalScoreScope::FullDataset && !candles.timestamp.is_empty() {
        let expected_first = candles.timestamp.first().copied();
        let expected_last = candles.timestamp.last().copied();
        if cache.first_timestamp != expected_first || cache.last_timestamp != expected_last {
            return Err(
                "full_dataset score cache timestamp span does not match the loaded candles"
                    .to_string(),
            );
        }
        if coverage_ratio < 0.95 {
            return Err(format!(
                "full_dataset score cache coverage_ratio {:.4} is too low for full-dataset validation",
                coverage_ratio
            ));
        }
    }
    Ok(LoadedScoreCache {
        points: out,
        summary: MlSignalScoreCacheSummary {
            matched_rows: cache.matched_rows,
            skipped_rows: cache.skipped_rows,
            bars_scored: cache.bars_scored,
            duplicate_timestamps: cache.duplicate_timestamps,
            coverage_ratio,
            score_space: cache.score_space,
            score_scope: cache.score_scope,
            first_timestamp: cache.first_timestamp,
            last_timestamp: cache.last_timestamp,
            feature_schema_hash: cache.feature_schema_hash,
            calibration_state: cache.calibration_state,
            threshold_policy: cache.threshold_policy,
            producer_kind: cache.producer_kind,
            score_contract_version: Some(cache.score_contract_version),
            source_bundle_manifest_hash: cache.source_bundle_manifest_hash,
            export_manifest_hash: cache.export_manifest_hash,
            native_backend_requested: cache.native_backend_requested,
            native_backend_used: cache.native_backend_used,
            native_library_path: cache.native_library_path,
            native_gpu_device_id: cache.native_gpu_device_id,
            native_fallback_reason: cache.native_fallback_reason,
        },
        has_probability_long,
        has_probability_short,
        rows_with_probability,
        rows_missing_long_probability,
        rows_missing_short_probability,
    })
}

fn score_target(point: Option<ScorePoint>, params: &MlSignalPolicyParams) -> Option<i32> {
    let point = point?;
    let p_long = point.p_long.filter(|v| v.is_finite())?;
    let p_short = point.p_short.filter(|v| v.is_finite());

    let p_short_cmp = p_short.unwrap_or(0.0);
    if p_long >= params.t_long && (p_long - p_short_cmp) >= params.delta {
        return Some(1);
    }
    if !params.strategy.long_only {
        let p_short = p_short?;
        if p_short >= params.t_short && (p_short - p_long) >= params.delta {
            return Some(-1);
        }
    }
    Some(0)
}

const ML_SIGNAL_NO_REQUEST: i32 = i32::MIN;

#[derive(Clone, Copy, Default)]
struct MlSignalOverlayState {
    cooldown_left: usize,
    pending_sign: i32,
    pending_count: usize,
}

#[inline]
fn ml_signal_overlay_on_bar_start(state: &mut MlSignalOverlayState) {
    if state.cooldown_left > 0 {
        state.cooldown_left = state.cooldown_left.saturating_sub(1);
    }
}

#[inline]
fn ml_signal_overlay_reset_pending(state: &mut MlSignalOverlayState) {
    state.pending_sign = 0;
    state.pending_count = 0;
}

#[inline]
fn ml_signal_apply_signal_persistence(
    raw_sign: i32,
    current_pos: i32,
    persistence_bars: usize,
    state: &mut MlSignalOverlayState,
) -> i32 {
    if persistence_bars <= 1 {
        if raw_sign == 0 {
            ml_signal_overlay_reset_pending(state);
        } else {
            state.pending_sign = raw_sign;
            state.pending_count = 1;
        }
        return raw_sign;
    }
    if raw_sign == 0 {
        ml_signal_overlay_reset_pending(state);
        return 0;
    }
    if raw_sign == state.pending_sign {
        state.pending_count = state.pending_count.saturating_add(1);
    } else {
        state.pending_sign = raw_sign;
        state.pending_count = 1;
    }
    if state.pending_count >= persistence_bars {
        raw_sign
    } else if current_pos != 0 {
        current_pos
    } else {
        0
    }
}

#[inline]
fn fill_requested_targets(
    score_points: &[Option<ScorePoint>],
    params: &MlSignalPolicyParams,
    requested: &mut [i32],
) {
    requested.fill(ML_SIGNAL_NO_REQUEST);
    for (idx, point) in score_points.iter().copied().enumerate() {
        let Some(target) = score_target(point, params) else {
            continue;
        };
        let apply_idx = if params.strategy.trade_on_next_bar {
            idx.saturating_add(1)
        } else {
            idx
        };
        if apply_idx < requested.len() {
            requested[apply_idx] = target;
        }
    }
}

fn fill_desired_positions(requested: &[i32], params: &MlSignalPolicyParams, desired: &mut [i32]) {
    desired.fill(0);
    let mut current = 0i32;
    let mut held_bars = 0usize;
    let mut overlays = MlSignalOverlayState::default();
    let cooldown_bars = params.strategy.cooldown_bars();
    let persistence_bars = params.strategy.signal_persistence_bars().max(1);

    for (bar, desired_slot) in desired.iter_mut().enumerate() {
        ml_signal_overlay_on_bar_start(&mut overlays);
        if current != 0 && params.max_hold_bars > 0 && held_bars >= params.max_hold_bars {
            current = 0;
            held_bars = 0;
            if cooldown_bars > 0 {
                overlays.cooldown_left = cooldown_bars;
            }
        }

        if requested[bar] != ML_SIGNAL_NO_REQUEST {
            let mut target = requested[bar].clamp(-1, 1);
            if params.strategy.long_only && target < 0 {
                target = 0;
            }
            target = ml_signal_apply_signal_persistence(
                target,
                current,
                persistence_bars,
                &mut overlays,
            );
            if current == 0 && target != 0 && overlays.cooldown_left > 0 {
                target = 0;
            }
            if current == 0 {
                if target != 0 {
                    current = target;
                    held_bars = 0;
                }
            } else if target == 0 {
                if params.exit_on_flat {
                    current = 0;
                    held_bars = 0;
                    if cooldown_bars > 0 {
                        overlays.cooldown_left = cooldown_bars;
                    }
                }
            } else if target != current && params.exit_on_opposite {
                if params.strategy.allow_flip {
                    current = target;
                    held_bars = 0;
                } else {
                    current = 0;
                    held_bars = 0;
                    if cooldown_bars > 0 {
                        overlays.cooldown_left = cooldown_bars;
                    }
                }
            }
        } else {
            ml_signal_overlay_reset_pending(&mut overlays);
        }

        *desired_slot = current;
        if current != 0 {
            held_bars = held_bars.saturating_add(1);
        }
    }
}

#[cfg(test)]
fn build_desired_positions(
    score_points: &[Option<ScorePoint>],
    params: &MlSignalPolicyParams,
) -> Vec<i32> {
    let mut requested = vec![ML_SIGNAL_NO_REQUEST; score_points.len()];
    fill_requested_targets(score_points, params, &mut requested);
    let mut desired = vec![0i32; score_points.len()];
    fill_desired_positions(&requested, params, &mut desired);
    desired
}

fn neutralize_ml_signal_execution_overlays(strategy: &StrategyConfig) -> StrategyConfig {
    let mut normalized = strategy.clone();
    normalized.allow_flip = true;
    normalized.cooldown_bars = 0;
    normalized.signal_persistence_bars = 1;
    normalized
}

fn validate_ml_signal_candidates(
    data_id: String,
    score_cache_path: String,
    score_summary: MlSignalScoreCacheSummary,
    score_points: &[Option<ScorePoint>],
    candidates: Vec<MlSignalPolicyParams>,
    candles: &Candles,
    iterative: &IterativeSimConfig,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    cancel: &AtomicBool,
) -> Result<MlSignalValidationResult, String> {
    let started = std::time::Instant::now();
    let score_obs = score_points
        .iter()
        .filter(|point| point.is_some())
        .count()
        .max(2);
    let gates = sanitize_hard_gates(min_psr, min_dsr, None, None, None);
    let lr = log_returns_from_close(&candles.close);
    let mut requested = Vec::new();
    resize_uninit_i32(&mut requested, score_points.len());
    let mut desired = Vec::new();
    resize_uninit_i32(&mut desired, score_points.len());

    let mut out = Vec::with_capacity(candidates.len());
    for params in candidates {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        fill_requested_targets(score_points, &params, &mut requested);
        fill_desired_positions(&requested, &params, &mut desired);
        let strategy = neutralize_ml_signal_execution_overlays(&params.strategy);
        let metrics = simulate_iterative_from_signals(
            &lr,
            OhlcSlice {
                open: &candles.open,
                high: &candles.high,
                low: &candles.low,
                close: &candles.close,
            },
            &desired,
            0,
            &strategy,
            iterative,
        );
        let robust_score = robust_score_from_metrics(&metrics, None);
        out.push(MlSignalValidationCandidateResult {
            params,
            in_sample: metrics,
            iterative_in_sample: None,
            psr: None,
            dsr: None,
            track_record_bars: score_obs,
            min_track_record_length_bars: None,
            validation_verdict: ValidationCandidateVerdict::default(),
            robust_score,
            robust_pareto: false,
        });
    }

    let sample_obs = vec![score_obs; out.len()];
    compute_probability_metrics(&mut out, &sample_obs);
    mark_robust_pareto(&mut out);
    apply_candidate_hard_gates(&mut out, &gates, None);
    let mut run_verdict = build_run_verdict(&out, out.len(), &gates, None, None);
    append_score_scope_warning_flag(&mut run_verdict, &score_summary);

    Ok(MlSignalValidationResult {
        data_id,
        runtime_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        backend_used: BackendUsed::Cpu,
        backend_route: Some("ml_signal_score_cache_iterative".to_string()),
        backend_fallback_reason: None,
        score_cache_path,
        score_summary,
        candidates: out,
        run_verdict,
    })
}

fn expand_axis_f64(axis: Option<&MlSignalSweepAxisF64>, fallback: f64) -> Result<Vec<f64>, String> {
    let Some(axis) = axis else {
        return Ok(vec![fallback]);
    };
    if !axis.start.is_finite()
        || !axis.end.is_finite()
        || !axis.step.is_finite()
        || axis.step <= 0.0
    {
        return Err("f64 sweep axis requires finite start/end and positive step".to_string());
    }
    if axis.end < axis.start {
        return Err("f64 sweep axis end must be >= start".to_string());
    }
    let mut values = Vec::new();
    let mut current = axis.start;
    while current <= axis.end + (axis.step * 1.0e-9) {
        values.push(((current * 1_000_000.0).round()) / 1_000_000.0);
        current += axis.step;
        if values.len() > 4096 {
            return Err("f64 sweep axis expanded beyond 4096 values".to_string());
        }
    }
    if values.is_empty() {
        values.push(fallback);
    }
    Ok(values)
}

fn expand_axis_usize(
    axis: Option<&MlSignalSweepAxisUsize>,
    fallback: usize,
) -> Result<Vec<usize>, String> {
    let Some(axis) = axis else {
        return Ok(vec![fallback.max(1)]);
    };
    if axis.step == 0 {
        return Err("usize sweep axis step must be positive".to_string());
    }
    if axis.end < axis.start {
        return Err("usize sweep axis end must be >= start".to_string());
    }
    let mut values = Vec::new();
    let mut current = axis.start;
    while current <= axis.end {
        values.push(current.max(1));
        current = current.saturating_add(axis.step);
        if values.len() > 4096 {
            return Err("usize sweep axis expanded beyond 4096 values".to_string());
        }
        if axis.step == 0 {
            break;
        }
    }
    if values.is_empty() {
        values.push(fallback.max(1));
    }
    Ok(values)
}

fn validate_score_cache_contract(
    cache: &LoadedScoreCache,
    candidates: &[MlSignalPolicyParams],
) -> Result<(), String> {
    if cache.summary.score_space != MlSignalScoreSpace::Probability {
        return Err(
            "score cache only contains raw margin columns; probability-threshold validation requires p_long/p_short or calibrated probability columns"
                .to_string(),
        );
    }
    let needs_short = candidates
        .iter()
        .any(|candidate| !candidate.strategy.long_only);
    if !cache.has_probability_long {
        return Err("score cache is missing long probability scores".to_string());
    }
    if needs_short && !cache.has_probability_short {
        return Err(
            "score cache is missing short probability scores required for non-long-only validation"
                .to_string(),
        );
    }
    if cache.rows_with_probability == 0 {
        return Err("score cache does not contain any usable probability rows".to_string());
    }
    if cache.rows_missing_long_probability > 0 {
        return Err(format!(
            "score cache has {} probability rows missing long scores",
            cache.rows_missing_long_probability
        ));
    }
    if needs_short && cache.rows_missing_short_probability > 0 {
        return Err(format!(
            "score cache has {} probability rows missing short scores required for non-long-only validation",
            cache.rows_missing_short_probability
        ));
    }
    Ok(())
}

fn score_scope_warning_flag(scope: MlSignalScoreScope) -> Option<&'static str> {
    match scope {
        MlSignalScoreScope::WalkforwardTest => None,
        MlSignalScoreScope::CalibrationSlice => Some("score_scope_calibration_slice"),
        MlSignalScoreScope::FullDataset => Some("score_scope_full_dataset"),
        MlSignalScoreScope::CustomPartial => Some("score_scope_custom_partial"),
    }
}

fn append_score_scope_warning_flag(
    run_verdict: &mut ValidationRunVerdict,
    score_summary: &MlSignalScoreCacheSummary,
) {
    let Some(flag) = score_scope_warning_flag(score_summary.score_scope) else {
        return;
    };
    if !run_verdict
        .warning_flags
        .iter()
        .any(|existing| existing == flag)
    {
        run_verdict.warning_flags.push(flag.to_string());
    }
}

pub fn run_ml_signal_validation_blocking_with_candles(
    req: MlSignalValidationRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) -> Result<MlSignalValidationResult, String> {
    if req.data_id.trim().is_empty() {
        return Err("data_id is required".to_string());
    }
    if req.score_cache_path.trim().is_empty() {
        return Err("score_cache_path is required".to_string());
    }
    if req.candidates.is_empty() {
        return Err("at least one candidate is required".to_string());
    }
    if candles.close.len() < 3 {
        return Err("not enough candles for ML validation".to_string());
    }

    let score_cache = load_score_cache(&req.score_cache_path, &req.data_id, candles)?;
    validate_score_cache_contract(&score_cache, &req.candidates)?;
    validate_ml_signal_candidates(
        req.data_id,
        req.score_cache_path,
        score_cache.summary,
        &score_cache.points,
        req.candidates,
        candles,
        &req.iterative,
        req.min_psr,
        req.min_dsr,
        cancel,
    )
}

pub fn run_ml_signal_policy_sweep_blocking_with_candles(
    req: MlSignalPolicySweepRequest,
    candles: &Candles,
    cancel: &AtomicBool,
) -> Result<MlSignalPolicySweepResult, String> {
    if req.data_id.trim().is_empty() {
        return Err("data_id is required".to_string());
    }
    if req.score_cache_path.trim().is_empty() {
        return Err("score_cache_path is required".to_string());
    }
    let t_long_values = expand_axis_f64(req.t_long.as_ref(), default_threshold())?;
    let t_short_values = expand_axis_f64(req.t_short.as_ref(), default_threshold())?;
    let delta_values = expand_axis_f64(req.delta.as_ref(), default_delta())?;
    let max_hold_values = expand_axis_usize(req.max_hold_bars.as_ref(), default_max_hold_bars())?;
    let exit_on_flat_values = req
        .exit_on_flat_values
        .clone()
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| vec![false]);
    let exit_on_opposite_values = req
        .exit_on_opposite_values
        .clone()
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| vec![true]);

    let candidate_budget = t_long_values
        .len()
        .saturating_mul(t_short_values.len())
        .saturating_mul(delta_values.len())
        .saturating_mul(max_hold_values.len())
        .saturating_mul(exit_on_flat_values.len())
        .saturating_mul(exit_on_opposite_values.len());
    if candidate_budget > 20_000 {
        return Err("policy sweep expanded beyond 20,000 candidates".to_string());
    }

    let mut candidates = Vec::with_capacity(candidate_budget);
    for t_long in &t_long_values {
        for t_short in &t_short_values {
            for delta in &delta_values {
                for max_hold_bars in &max_hold_values {
                    for exit_on_flat in &exit_on_flat_values {
                        for exit_on_opposite in &exit_on_opposite_values {
                            candidates.push(MlSignalPolicyParams {
                                t_long: *t_long,
                                t_short: *t_short,
                                delta: *delta,
                                max_hold_bars: *max_hold_bars,
                                exit_on_flat: *exit_on_flat,
                                exit_on_opposite: *exit_on_opposite,
                                label: Some(format!(
                                    "tl_{:.3}_ts_{:.3}_d_{:.3}_hold_{}_flat_{}_opp_{}",
                                    t_long,
                                    t_short,
                                    delta,
                                    max_hold_bars,
                                    exit_on_flat,
                                    exit_on_opposite
                                )),
                                strategy: req.base_strategy.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    let sweep_size = candidates.len();
    let score_cache = load_score_cache(&req.score_cache_path, &req.data_id, candles)?;
    validate_score_cache_contract(&score_cache, &candidates)?;
    let mut validation = validate_ml_signal_candidates(
        req.data_id,
        req.score_cache_path,
        score_cache.summary.clone(),
        &score_cache.points,
        candidates,
        candles,
        &req.iterative,
        req.min_psr,
        req.min_dsr,
        cancel,
    )?;
    validation
        .candidates
        .sort_by(|a, b| b.robust_score.total_cmp(&a.robust_score));
    if req.top_k > 0 && validation.candidates.len() > req.top_k {
        validation.candidates.truncate(req.top_k);
    }
    validation.run_verdict = build_run_verdict(
        &validation.candidates,
        sweep_size,
        &sanitize_hard_gates(req.min_psr, req.min_dsr, None, None, None),
        None,
        None,
    );
    append_score_scope_warning_flag(&mut validation.run_verdict, &validation.score_summary);
    Ok(MlSignalPolicySweepResult {
        sweep_size,
        top_k: req.top_k,
        validation,
    })
}

pub fn read_ml_signal_manifest(
    manifest_path: &Path,
) -> Result<MlSignalModelBundleManifest, String> {
    let file = File::open(manifest_path).map_err(|e| format!("open ml_signal manifest: {e}"))?;
    serde_json::from_reader(BufReader::new(file))
        .map_err(|e| format!("parse ml_signal manifest json: {e}"))
}

fn manifest_child_exists(base_dir: &Path, rel_path: Option<&String>) -> bool {
    rel_path
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| base_dir.join(value).exists())
        .unwrap_or(false)
}

fn manifest_json_field_present(value: Option<&JsonValue>, key: &str) -> bool {
    value
        .and_then(JsonValue::as_object)
        .and_then(|row| row.get(key))
        .is_some()
}

pub fn classify_ml_signal_manifest(
    manifest_path: &Path,
) -> Result<MlSignalModelBundleStatus, String> {
    let manifest = read_ml_signal_manifest(manifest_path)?;
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut issues = Vec::new();

    if manifest.schema_version == 0 {
        issues.push("manifest schema_version must be positive".to_string());
    }
    if manifest.feature_schema_version != ML_SIGNAL_FEATURE_SCHEMA_VERSION {
        issues.push(format!(
            "feature_schema_version {} does not match expected {}",
            manifest.feature_schema_version, ML_SIGNAL_FEATURE_SCHEMA_VERSION
        ));
    }
    if !manifest_child_exists(base_dir, manifest.long_model_path.as_ref()) {
        issues.push("long model file missing".to_string());
    }
    if !manifest_child_exists(base_dir, manifest.short_model_path.as_ref()) {
        issues.push("short model file missing".to_string());
    }
    if let Some(path) = manifest.calibration_path.as_ref() {
        if !path.trim().is_empty() && !base_dir.join(path).exists() {
            issues.push("calibration file missing".to_string());
        }
    }
    if manifest
        .source_feature_schema_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issues.push("recommended source_feature_schema_hash missing".to_string());
    }
    if manifest
        .source_export_manifest_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issues.push("recommended source_export_manifest_hash missing".to_string());
    }
    if manifest.training_config.is_none() {
        issues.push("recommended training_config missing".to_string());
    }
    if manifest
        .feature_schema_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issues.push("recommended feature_schema_path missing".to_string());
    } else if !manifest_child_exists(base_dir, manifest.feature_schema_path.as_ref()) {
        issues.push("feature schema sidecar missing".to_string());
    }
    if manifest
        .export_manifest_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issues.push("recommended export_manifest_path missing".to_string());
    } else if !manifest_child_exists(base_dir, manifest.export_manifest_path.as_ref()) {
        issues.push("export manifest sidecar missing".to_string());
    }
    if manifest
        .split_manifest_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issues.push("recommended split_manifest_path missing".to_string());
    } else if !manifest_child_exists(base_dir, manifest.split_manifest_path.as_ref()) {
        issues.push("split manifest sidecar missing".to_string());
    }
    if manifest.score_contract.is_none() {
        issues.push("recommended score_contract missing".to_string());
    } else {
        if !manifest_json_field_present(manifest.score_contract.as_ref(), "score_contract_version")
            && !manifest_json_field_present(
                manifest.score_contract.as_ref(),
                "scoreContractVersion",
            )
        {
            issues.push("recommended score_contract_version missing".to_string());
        }
        if !manifest_json_field_present(
            manifest.score_contract.as_ref(),
            "walkforward_recommended_mode",
        ) && !manifest_json_field_present(
            manifest.score_contract.as_ref(),
            "walkforwardRecommendedMode",
        ) {
            issues.push("recommended walkforward_recommended_mode missing".to_string());
        }
    }
    if manifest.training_windows_summary.is_none() {
        issues.push("recommended training_windows_summary missing".to_string());
    }
    if manifest.class_balance_policy.is_none() {
        issues.push("recommended class_balance_policy missing".to_string());
    }
    if manifest.explainability_artifacts.is_none() {
        issues.push("recommended explainability_artifacts missing".to_string());
    }
    if manifest
        .calibration_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        && manifest.calibration_summary.is_none()
    {
        issues.push("recommended calibration_summary missing".to_string());
    }

    let state = if issues.is_empty() {
        "healthy"
    } else {
        "warning"
    }
    .to_string();
    Ok(MlSignalModelBundleStatus {
        bundle_id: manifest.bundle_id.clone(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        state,
        manifest: Some(manifest),
        issues,
    })
}

pub fn list_ml_signal_model_bundle_summaries(
    root_dir: &Path,
) -> Result<Vec<MlSignalModelBundleSummary>, String> {
    if !root_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(root_dir).map_err(|e| format!("read ml_signal models dir: {e}"))? {
        let entry = entry.map_err(|e| format!("read ml_signal model entry: {e}"))?;
        let path = entry.path();
        let manifest_path = if path.is_dir() {
            let candidate = path.join("manifest.json");
            if candidate.exists() {
                candidate
            } else {
                continue;
            }
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("manifest.json"))
            .unwrap_or(false)
        {
            path
        } else {
            continue;
        };

        let status = classify_ml_signal_manifest(&manifest_path)?;
        let manifest = status
            .manifest
            .clone()
            .ok_or_else(|| "manifest unexpectedly missing".to_string())?;
        out.push(MlSignalModelBundleSummary {
            bundle_id: status.bundle_id,
            manifest_path: status.manifest_path,
            state: status.state,
            model_family: manifest.model_family,
            model_format: manifest.model_format,
            issues: status.issues,
        });
    }
    out.sort_by(|a, b| a.bundle_id.cmp(&b.bundle_id));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::lock_test_env;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

    fn temp_file_path(name: &str, ext: &str) -> String {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("ta_ml_signal_{name}_{nonce}.{ext}"))
            .to_string_lossy()
            .to_string()
    }

    fn temp_dir_path(name: &str) -> String {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("ta_ml_signal_{name}_{nonce}"))
            .to_string_lossy()
            .to_string()
    }

    fn score_cache_build_request(
        data_id: &str,
        input_path: &str,
        output_path: &str,
        model_bundle_id: Option<&str>,
    ) -> MlSignalScoreCacheBuildRequest {
        MlSignalScoreCacheBuildRequest {
            data_id: data_id.to_string(),
            input_path: input_path.to_string(),
            output_path: output_path.to_string(),
            model_bundle_id: model_bundle_id.map(str::to_string),
            feature_schema_path: None,
            feature_schema_hash: None,
            export_manifest_path: None,
            source_bundle_manifest_path: None,
            calibration_state: None,
            threshold_policy: None,
            producer_kind: None,
            score_contract_version: None,
            native_backend_preference: MlSignalInferenceBackendPreference::ScoreCacheOnly,
            native_library_path: None,
            native_gpu_device_id: None,
        }
    }

    fn sample_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut volume = Vec::with_capacity(len);
        for i in 0..len {
            let base = 100.0 + (i as f64 * 0.35) + ((i % 7) as f64 * 0.12);
            let o = base;
            let c = base + if i % 9 < 5 { 0.45 } else { -0.20 };
            ts.push(1_700_000_000_000_i64 + (i as i64 * 86_400_000));
            open.push(o);
            high.push(o.max(c) + 0.35);
            low.push(o.min(c) - 0.30);
            close.push(c);
            volume.push(1_000.0 + (i % 11) as f64 * 25.0);
        }
        Candles::new_with_fields(
            ts,
            open,
            high,
            low,
            close,
            volume,
            CandleFieldFlags {
                open: true,
                high: true,
                low: true,
                close: true,
                volume: true,
            },
        )
    }

    #[test]
    fn score_target_respects_thresholds_and_delta() {
        let mut params = MlSignalPolicyParams::default();
        params.strategy.long_only = false;
        assert_eq!(
            score_target(
                Some(ScorePoint {
                    p_long: Some(0.68),
                    p_short: Some(0.45),
                    raw_long: None,
                    raw_short: None,
                }),
                &params
            ),
            Some(1)
        );
        assert_eq!(
            score_target(
                Some(ScorePoint {
                    p_long: Some(0.44),
                    p_short: Some(0.69),
                    raw_long: None,
                    raw_short: None,
                }),
                &params
            ),
            Some(-1)
        );
        assert_eq!(
            score_target(
                Some(ScorePoint {
                    p_long: Some(0.58),
                    p_short: Some(0.56),
                    raw_long: None,
                    raw_short: None,
                }),
                &params
            ),
            Some(0)
        );
    }

    #[test]
    fn log_returns_from_close_preserves_zero_prefix_and_invalid_rows() {
        assert!(log_returns_from_close(&[]).is_empty());
        assert_eq!(log_returns_from_close(&[100.0]), vec![0.0]);

        let out = log_returns_from_close(&[100.0, 110.0, f64::NAN, 90.0, 0.0, 95.0]);
        assert_eq!(out[0], 0.0);
        assert!((out[1] - (110.0f64 / 100.0).ln()).abs() <= f64::EPSILON);
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.0);
        assert_eq!(out[4], 0.0);
        assert_eq!(out[5], 0.0);
    }

    #[test]
    fn prefix_sum_helpers_match_reference() {
        let values = [1.0, -2.0, 3.5, 0.5];

        assert_eq!(prefix_sum(&values), vec![0.0, 1.0, -1.0, 2.5, 3.0]);
        assert_eq!(
            prefix_sum_squares(&values),
            vec![0.0, 1.0, 5.0, 17.25, 17.5]
        );
    }

    #[test]
    fn build_desired_positions_respects_allow_flip_cooldown_and_persistence() {
        let mut params = MlSignalPolicyParams::default();
        params.strategy.long_only = false;
        params.strategy.trade_on_next_bar = false;
        params.exit_on_opposite = true;
        params.strategy.allow_flip = false;
        params.strategy.cooldown_bars = 2;
        let score_points = vec![
            Some(ScorePoint {
                p_long: Some(0.80),
                p_short: Some(0.10),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.10),
                p_short: Some(0.85),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.10),
                p_short: Some(0.85),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.10),
                p_short: Some(0.85),
                raw_long: None,
                raw_short: None,
            }),
        ];
        assert_eq!(
            build_desired_positions(&score_points, &params),
            vec![1, 0, 0, -1]
        );

        params.strategy.allow_flip = true;
        params.strategy.cooldown_bars = 0;
        params.strategy.signal_persistence_bars = 2;
        let persistent = vec![
            Some(ScorePoint {
                p_long: Some(0.80),
                p_short: Some(0.10),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.80),
                p_short: Some(0.10),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.10),
                p_short: Some(0.10),
                raw_long: None,
                raw_short: None,
            }),
        ];
        assert_eq!(build_desired_positions(&persistent, &params), vec![0, 1, 1]);
    }

    #[test]
    fn build_desired_positions_respects_opposite_persistence_before_close_and_reentry() {
        let mut params = MlSignalPolicyParams::default();
        params.strategy.long_only = false;
        params.strategy.trade_on_next_bar = false;
        params.strategy.allow_flip = false;
        params.strategy.cooldown_bars = 2;
        params.strategy.signal_persistence_bars = 2;
        params.max_hold_bars = 0;
        params.exit_on_flat = false;
        params.exit_on_opposite = true;

        let score_points = vec![
            Some(ScorePoint {
                p_long: Some(0.80),
                p_short: Some(0.10),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.82),
                p_short: Some(0.10),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.12),
                p_short: Some(0.81),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.10),
                p_short: Some(0.84),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.09),
                p_short: Some(0.86),
                raw_long: None,
                raw_short: None,
            }),
            Some(ScorePoint {
                p_long: Some(0.08),
                p_short: Some(0.88),
                raw_long: None,
                raw_short: None,
            }),
        ];

        assert_eq!(
            build_desired_positions(&score_points, &params),
            vec![0, 1, 1, 0, 0, -1]
        );
    }

    #[test]
    fn score_csv_prefers_calibrated_probability_columns() {
        let _guard = lock_test_env();
        let input_path = temp_file_path("scores_prefers_cal", "csv");
        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record([
                "timestamp",
                "p_long",
                "p_long_cal",
                "p_short",
                "p_short_cal",
            ])
            .expect("header");
        writer
            .write_record(["1", "0.15", "0.81", "0.85", "0.19"])
            .expect("row");
        writer.flush().expect("flush score csv");

        let parsed = read_score_rows_csv(&input_path).expect("parse score csv");
        assert_eq!(parsed.score_space, MlSignalScoreSpace::Probability);
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].p_long, Some(0.81));
        assert_eq!(parsed.rows[0].p_short, Some(0.19));

        let _ = fs::remove_file(&input_path);
    }

    #[test]
    fn score_cache_build_persists_probability_metadata() {
        let _guard = lock_test_env();
        let candles = sample_candles(20);
        let input_path = temp_file_path("scores_meta_in", "csv");
        let cache_path = temp_file_path("scores_meta_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long_cal", "score_scope"])
            .expect("header");
        writer
            .write_record([
                candles.timestamp[3].to_string(),
                "0.75".to_string(),
                "custom_partial".to_string(),
            ])
            .expect("row 1");
        writer
            .write_record([
                candles.timestamp[4].to_string(),
                "0.74".to_string(),
                "custom_partial".to_string(),
            ])
            .expect("row 2");
        writer
            .write_record([
                candles.timestamp[4].to_string(),
                "0.73".to_string(),
                "custom_partial".to_string(),
            ])
            .expect("row 3");
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, Some("bundle-a")),
            &candles,
            &cancel,
        )
        .expect("build score cache");

        assert_eq!(build.score_space, MlSignalScoreSpace::Probability);
        assert_eq!(build.score_scope, MlSignalScoreScope::CustomPartial);
        assert_eq!(build.duplicate_timestamps, 1);
        assert!((build.coverage_ratio - 0.10).abs() < 1.0e-9);

        let manifest: MlSignalScoreCacheManifest = serde_json::from_reader(BufReader::new(
            File::open(&build.manifest_path).expect("open manifest"),
        ))
        .expect("parse manifest");
        assert_eq!(manifest.score_space, MlSignalScoreSpace::Probability);
        assert_eq!(manifest.score_scope, MlSignalScoreScope::CustomPartial);
        assert_eq!(manifest.duplicate_timestamps, 1);
        assert!((manifest.coverage_ratio - 0.10).abs() < 1.0e-9);

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn validation_rejects_raw_only_score_cache() {
        let _guard = lock_test_env();
        let candles = sample_candles(24);
        let input_path = temp_file_path("scores_raw_only_in", "csv");
        let cache_path = temp_file_path("scores_raw_only_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "raw_long", "raw_short"])
            .expect("header");
        for ts in candles.timestamp.iter().skip(2).take(8) {
            writer
                .write_record([ts.to_string(), "0.25".to_string(), "-0.25".to_string()])
                .expect("row");
        }
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build raw-only cache");

        let err = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams::default()],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect_err("raw-only cache should be rejected");
        assert!(err.contains("raw margin"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn validation_allows_long_only_one_sided_probability_cache() {
        let _guard = lock_test_env();
        let candles = sample_candles(30);
        let input_path = temp_file_path("scores_long_only_in", "csv");
        let cache_path = temp_file_path("scores_long_only_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long_cal"])
            .expect("header");
        for (idx, ts) in candles.timestamp.iter().enumerate().skip(3).take(12) {
            let p_long = if idx % 4 < 2 { "0.78" } else { "0.41" };
            writer
                .write_record([ts.to_string(), p_long.to_string()])
                .expect("row");
        }
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, Some("bundle-a")),
            &candles,
            &cancel,
        )
        .expect("build score cache");

        let mut strategy = StrategyConfig::default();
        strategy.long_only = true;
        let validation = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams {
                    strategy,
                    ..MlSignalPolicyParams::default()
                }],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect("long-only validation should succeed");

        assert_eq!(
            validation.score_summary.score_space,
            MlSignalScoreSpace::Probability
        );
        assert_eq!(validation.candidates.len(), 1);

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn validation_rejects_two_sided_cache_with_missing_short_probability_rows() {
        let _guard = lock_test_env();
        let candles = sample_candles(24);
        let input_path = temp_file_path("scores_missing_short_in", "csv");
        let cache_path = temp_file_path("scores_missing_short_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short"])
            .expect("header");
        writer
            .write_record([
                candles.timestamp[2].to_string(),
                "0.72".to_string(),
                "0.18".to_string(),
            ])
            .expect("row 1");
        writer
            .write_record([
                candles.timestamp[3].to_string(),
                "0.74".to_string(),
                "".to_string(),
            ])
            .expect("row 2");
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build score cache");

        let mut strategy = StrategyConfig::default();
        strategy.long_only = false;
        let err = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams {
                    strategy,
                    ..MlSignalPolicyParams::default()
                }],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect_err("two-sided validation should reject incomplete short scores");
        assert!(err.contains("missing short scores"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn score_cache_build_and_validation_smoke() {
        let _guard = lock_test_env();
        let candles = sample_candles(80);
        let input_path = temp_file_path("scores_in", "csv");
        let cache_path = temp_file_path("scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short"])
            .expect("header");
        for (idx, ts) in candles.timestamp.iter().enumerate().skip(5).take(50) {
            let (p_long, p_short) = if idx % 6 < 3 {
                (0.72, 0.18)
            } else {
                (0.20, 0.74)
            };
            writer
                .write_record([ts.to_string(), p_long.to_string(), p_short.to_string()])
                .expect("row");
        }
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, Some("bundle-a")),
            &candles,
            &cancel,
        )
        .expect("build score cache");
        assert_eq!(build.matched_rows, 50);
        assert_eq!(build.skipped_rows, 0);
        assert!(Path::new(&build.manifest_path).exists());

        let mut strategy = StrategyConfig::default();
        strategy.long_only = false;
        let validation = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams {
                    t_long: 0.60,
                    t_short: 0.60,
                    delta: 0.10,
                    max_hold_bars: 8,
                    exit_on_flat: true,
                    exit_on_opposite: true,
                    label: Some("smoke".to_string()),
                    strategy,
                }],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect("validation should succeed");

        assert_eq!(validation.candidates.len(), 1);
        assert!(!validation.backend_route.unwrap_or_default().is_empty());
        assert!(validation.candidates[0].track_record_bars >= 2);

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn score_cache_prefers_calibrated_columns_and_persists_metadata() {
        let _guard = lock_test_env();
        let candles = sample_candles(24);
        let input_path = temp_file_path("validation_scores_input", "csv");
        let cache_path = temp_file_path("validation_scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record([
                "timestamp",
                "p_long",
                "p_short",
                "p_long_cal",
                "p_short_cal",
            ])
            .expect("header");
        for ts in candles.timestamp.iter().skip(3).take(10) {
            writer
                .write_record([
                    ts.to_string(),
                    "0.11".to_string(),
                    "0.89".to_string(),
                    "0.77".to_string(),
                    "0.23".to_string(),
                ])
                .expect("row");
        }
        writer
            .write_record([
                candles.timestamp[5].to_string(),
                "0.12".to_string(),
                "0.88".to_string(),
                "0.91".to_string(),
                "0.09".to_string(),
            ])
            .expect("duplicate row");
        writer
            .write_record([
                "9999999999999".to_string(),
                "0.12".to_string(),
                "0.88".to_string(),
                "0.66".to_string(),
                "0.34".to_string(),
            ])
            .expect("unmatched row");
        writer.flush().expect("flush score csv");

        let bundle_manifest_path = temp_file_path("ml_bundle_manifest", "json");
        std::fs::write(
            &bundle_manifest_path,
            r#"{"bundle_id":"bundle-a","schema_version":1}"#,
        )
        .expect("write bundle manifest");
        let export_manifest_path = temp_file_path("ml_export_manifest", "json");
        std::fs::write(
            &export_manifest_path,
            r#"{"data_id":"data-1","schema_hash":"fnv1a64-feature-schema"}"#,
        )
        .expect("write export manifest");
        let mut req =
            score_cache_build_request("data-1", &input_path, &cache_path, Some("bundle-a"));
        req.feature_schema_hash = Some("fnv1a64-feature-schema".to_string());
        req.export_manifest_path = Some(export_manifest_path.clone());
        req.source_bundle_manifest_path = Some(bundle_manifest_path.clone());
        req.calibration_state = Some("calibrated_probability".to_string());
        req.threshold_policy = Some("desktop_probability_threshold_only".to_string());
        req.producer_kind = Some("python_research_harness".to_string());
        req.score_contract_version = Some(3);
        let build = build_ml_signal_score_cache_from_csv(req, &candles, &cancel)
            .expect("build score cache");
        assert_eq!(build.score_space, MlSignalScoreSpace::Probability);
        assert_eq!(build.score_scope, MlSignalScoreScope::CalibrationSlice);
        assert_eq!(build.duplicate_timestamps, 1);
        assert_eq!(build.skipped_rows, 1);
        assert!(build.coverage_ratio > 0.0);

        let cache: MlSignalScoreCacheFile =
            serde_json::from_reader(BufReader::new(File::open(&cache_path).expect("open cache")))
                .expect("parse cache");
        assert_eq!(cache.score_space, MlSignalScoreSpace::Probability);
        assert_eq!(cache.score_scope, MlSignalScoreScope::CalibrationSlice);
        assert_eq!(cache.duplicate_timestamps, 1);
        assert_eq!(cache.scores[0].p_long, Some(0.77));
        assert_eq!(cache.scores[0].p_short, Some(0.23));

        let manifest: MlSignalScoreCacheManifest = serde_json::from_reader(BufReader::new(
            File::open(&build.manifest_path).expect("open manifest"),
        ))
        .expect("parse manifest");
        assert_eq!(manifest.score_space, MlSignalScoreSpace::Probability);
        assert_eq!(manifest.score_scope, MlSignalScoreScope::CalibrationSlice);
        assert_eq!(manifest.duplicate_timestamps, 1);
        assert_eq!(
            manifest.feature_schema_hash.as_deref(),
            Some("fnv1a64-feature-schema")
        );
        assert_eq!(
            manifest.calibration_state.as_deref(),
            Some("calibrated_probability")
        );
        assert_eq!(
            manifest.threshold_policy.as_deref(),
            Some("desktop_probability_threshold_only")
        );
        assert_eq!(
            manifest.producer_kind.as_deref(),
            Some("python_research_harness")
        );
        assert_eq!(manifest.score_contract_version, 3);
        assert!(manifest.source_bundle_manifest_hash.is_some());
        assert!(manifest.export_manifest_hash.is_some());

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
        let _ = fs::remove_file(&bundle_manifest_path);
        let _ = fs::remove_file(&export_manifest_path);
    }

    #[test]
    fn dataset_export_writes_schema_hash_and_provenance_metadata() {
        let _guard = lock_test_env();
        let candles = sample_candles(160);
        let output_path = temp_file_path("ml_dataset_export", "csv");
        let cancel = AtomicBool::new(false);

        let export = export_ml_signal_dataset_with_candles(
            MlSignalDatasetExportRequest {
                data_id: "data-ml".to_string(),
                output_path: output_path.clone(),
                ma_source: "close".to_string(),
                horizon_bars: 10,
                round_trip_cost_bps: 12.5,
                split_plan: None,
                ticker_holdout: None,
            },
            &candles,
            &cancel,
        )
        .expect("export dataset");

        assert!(!export.feature_schema_hash.trim().is_empty());
        let schema: MlSignalFeatureSchema = serde_json::from_reader(BufReader::new(
            File::open(&export.feature_schema_path).expect("open feature schema"),
        ))
        .expect("parse feature schema");
        assert_eq!(schema.schema_hash, export.feature_schema_hash);
        assert_eq!(schema.exporter_version, ML_SIGNAL_EXPORTER_VERSION);
        assert!(schema.indicator_config.is_some());

        let manifest: MlSignalDatasetExportManifest = serde_json::from_reader(BufReader::new(
            File::open(&export.export_manifest_path).expect("open export manifest"),
        ))
        .expect("parse export manifest");
        assert_eq!(manifest.feature_schema_hash, export.feature_schema_hash);
        assert_eq!(manifest.exporter_version, ML_SIGNAL_EXPORTER_VERSION);
        assert!(manifest.label_policy.is_some());
        assert!(manifest.sample_weight_policy.is_some());
        assert!(manifest.created_at_unix_ms.is_some());

        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(&export.feature_schema_path);
        let _ = fs::remove_file(&export.export_manifest_path);
    }

    #[test]
    fn validation_rejects_raw_only_cache_and_missing_short_side() {
        let _guard = lock_test_env();
        let candles = sample_candles(24);
        let input_path = temp_file_path("raw_scores_in", "csv");
        let cache_path = temp_file_path("raw_scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "raw_long", "raw_short"])
            .expect("header");
        for ts in candles.timestamp.iter().skip(2).take(8) {
            writer
                .write_record([ts.to_string(), "1.5".to_string(), "-1.2".to_string()])
                .expect("row");
        }
        writer.flush().expect("flush score csv");
        build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build raw cache");
        let mut strategy = StrategyConfig::default();
        strategy.long_only = false;
        let err = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams {
                    strategy,
                    ..MlSignalPolicyParams::default()
                }],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect_err("raw-only cache should fail");
        assert!(err.contains("raw margin columns"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(score_manifest_path_for_cache(&cache_path));
    }

    #[test]
    fn validation_allows_long_only_one_sided_scores_but_requires_short_side_for_two_sided() {
        let _guard = lock_test_env();
        let candles = sample_candles(32);
        let input_path = temp_file_path("long_only_scores_in", "csv");
        let cache_path = temp_file_path("long_only_scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long"])
            .expect("header");
        for ts in candles.timestamp.iter().skip(3).take(12) {
            writer
                .write_record([ts.to_string(), "0.81".to_string()])
                .expect("row");
        }
        writer.flush().expect("flush score csv");
        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build one-sided cache");

        let mut long_only_strategy = StrategyConfig::default();
        long_only_strategy.long_only = true;
        let ok = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams {
                    strategy: long_only_strategy,
                    ..MlSignalPolicyParams::default()
                }],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect("long-only validation should succeed");
        assert_eq!(
            ok.score_summary.score_space,
            MlSignalScoreSpace::Probability
        );
        assert_eq!(ok.score_summary.bars_scored, build.bars_scored);

        let mut long_short_strategy = StrategyConfig::default();
        long_short_strategy.long_only = false;
        let err = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams {
                    strategy: long_short_strategy,
                    ..MlSignalPolicyParams::default()
                }],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect_err("two-sided validation should reject missing short scores");
        assert!(err.contains("short probability"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn validation_adds_score_scope_warning_flags_without_rejecting_probability_cache() {
        let _guard = lock_test_env();
        let candles = sample_candles(32);
        let input_path = temp_file_path("scope_warning_scores_in", "csv");
        let cache_path = temp_file_path("scope_warning_scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short", "score_scope"])
            .expect("header");
        for ts in candles.timestamp.iter().skip(3).take(12) {
            writer
                .write_record([
                    ts.to_string(),
                    "0.81".to_string(),
                    "0.19".to_string(),
                    "CalibrationSlice".to_string(),
                ])
                .expect("row");
        }
        writer.flush().expect("flush score csv");
        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build cache");
        assert_eq!(build.score_scope, MlSignalScoreScope::CalibrationSlice);

        let validation = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams::default()],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect("validation should succeed");
        assert!(validation
            .run_verdict
            .warning_flags
            .iter()
            .any(|flag| flag == "score_scope_calibration_slice"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn walkforward_scope_does_not_add_extra_warning_flag() {
        let _guard = lock_test_env();
        let candles = sample_candles(32);
        let input_path = temp_file_path("scope_walkforward_scores_in", "csv");
        let cache_path = temp_file_path("scope_walkforward_scores_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short", "score_scope"])
            .expect("header");
        for ts in candles.timestamp.iter().skip(3).take(12) {
            writer
                .write_record([
                    ts.to_string(),
                    "0.81".to_string(),
                    "0.19".to_string(),
                    "WalkforwardTest".to_string(),
                ])
                .expect("row");
        }
        writer.flush().expect("flush score csv");
        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build cache");
        assert_eq!(build.score_scope, MlSignalScoreScope::WalkforwardTest);

        let validation = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams::default()],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect("validation should succeed");
        assert!(!validation
            .run_verdict
            .warning_flags
            .iter()
            .any(|flag| flag.starts_with("score_scope_")));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn classify_manifest_reports_missing_recommended_metadata() {
        let _guard = lock_test_env();
        let bundle_dir = Path::new(&temp_dir_path("mlsig_manifest_status")).to_path_buf();
        fs::create_dir_all(&bundle_dir).expect("create bundle dir");
        fs::write(bundle_dir.join("model_long.cbm"), b"long").expect("write long model");
        fs::write(bundle_dir.join("model_short.cbm"), b"short").expect("write short model");
        fs::write(bundle_dir.join("calibration.json"), b"{}").expect("write calibration");
        let manifest_path = bundle_dir.join("manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "bundle_id": "bundle_status_test",
                "schema_version": 1,
                "feature_schema_version": 1,
                "model_family": "catboost_binary_pair",
                "model_format": "cbm",
                "long_model_path": "model_long.cbm",
                "short_model_path": "model_short.cbm",
                "calibration_path": "calibration.json",
                "source_feature_schema_hash": "schemahash-ok",
                "source_export_manifest_hash": "exporthash-ok",
                "training_config": { "iterations": 100 },
                "score_contract": {
                    "validation_scores_scope": "calibration_slice"
                }
            }))
            .expect("serialize manifest"),
        )
        .expect("write manifest");

        let status = classify_ml_signal_manifest(&manifest_path).expect("classify manifest");
        assert_eq!(status.state, "warning");
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended feature_schema_path missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended export_manifest_path missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended split_manifest_path missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended score_contract_version missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended walkforward_recommended_mode missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended training_windows_summary missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended class_balance_policy missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended explainability_artifacts missing"));
        assert!(status
            .issues
            .iter()
            .any(|issue| issue == "recommended calibration_summary missing"));

        let _ = fs::remove_file(bundle_dir.join("model_long.cbm"));
        let _ = fs::remove_file(bundle_dir.join("model_short.cbm"));
        let _ = fs::remove_file(bundle_dir.join("calibration.json"));
        let _ = fs::remove_file(&manifest_path);
        let _ = fs::remove_dir(&bundle_dir);
    }

    #[test]
    fn build_split_windows_generates_rolls() {
        let windows = build_ml_signal_split_windows(
            300,
            &MlSignalSplitPlan {
                version: 1,
                train_bars: 120,
                calibration_bars: 40,
                test_bars: 30,
                gap_bars: 10,
                step_bars: 50,
                windows: Vec::new(),
            },
        )
        .expect("split windows");
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].split_id, "roll_000");
        assert_eq!(windows[0].train_start, 0);
        assert_eq!(windows[0].test_end, 210);
        assert_eq!(windows[1].split_id, "roll_001");
        assert_eq!(windows[1].train_start, 50);
    }

    #[test]
    fn ticker_holdout_assignment_is_stable() {
        let cfg = MlSignalTickerHoldoutConfig {
            modulus: 5,
            residue: 2,
        };
        let a = assign_ml_signal_ticker_holdout("AAPL", &cfg);
        let b = assign_ml_signal_ticker_holdout("AAPL", &cfg);
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn policy_sweep_returns_ranked_candidates() {
        let _guard = lock_test_env();
        let candles = sample_candles(80);
        let input_path = temp_file_path("scores_sweep_in", "csv");
        let cache_path = temp_file_path("scores_sweep_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short"])
            .expect("header");
        for (idx, ts) in candles.timestamp.iter().enumerate().skip(5).take(50) {
            let (p_long, p_short) = if idx % 5 < 3 {
                (0.75, 0.15)
            } else {
                (0.20, 0.70)
            };
            writer
                .write_record([ts.to_string(), p_long.to_string(), p_short.to_string()])
                .expect("row");
        }
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, Some("bundle-a")),
            &candles,
            &cancel,
        )
        .expect("build score cache");
        let mut strategy = StrategyConfig::default();
        strategy.long_only = false;
        let sweep = run_ml_signal_policy_sweep_blocking_with_candles(
            MlSignalPolicySweepRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                t_long: Some(MlSignalSweepAxisF64 {
                    start: 0.55,
                    end: 0.65,
                    step: 0.10,
                }),
                t_short: Some(MlSignalSweepAxisF64 {
                    start: 0.55,
                    end: 0.65,
                    step: 0.10,
                }),
                delta: Some(MlSignalSweepAxisF64 {
                    start: 0.05,
                    end: 0.10,
                    step: 0.05,
                }),
                max_hold_bars: Some(MlSignalSweepAxisUsize {
                    start: 5,
                    end: 6,
                    step: 1,
                }),
                exit_on_flat_values: Some(vec![false, true]),
                exit_on_opposite_values: Some(vec![true]),
                base_strategy: strategy,
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
                top_k: 3,
            },
            &candles,
            &cancel,
        )
        .expect("policy sweep");
        assert_eq!(sweep.top_k, 3);
        assert!(sweep.sweep_size >= 8);
        assert_eq!(sweep.validation.candidates.len(), 3);

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn validation_rejects_duplicate_timestamp_score_cache() {
        let _guard = lock_test_env();
        let candles = sample_candles(24);
        let input_path = temp_file_path("scores_duplicate_in", "csv");
        let cache_path = temp_file_path("scores_duplicate_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short"])
            .expect("header");
        let ts = candles.timestamp[5].to_string();
        writer
            .write_record([ts.clone(), "0.80".to_string(), "0.20".to_string()])
            .expect("row a");
        writer
            .write_record([ts, "0.81".to_string(), "0.19".to_string()])
            .expect("row b");
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build cache");
        let err = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams::default()],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect_err("duplicate cache should fail validation");
        assert!(err.contains("duplicate timestamps"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }

    #[test]
    fn validation_rejects_full_dataset_scope_with_partial_coverage() {
        let _guard = lock_test_env();
        let candles = sample_candles(24);
        let input_path = temp_file_path("scores_partial_full_dataset_in", "csv");
        let cache_path = temp_file_path("scores_partial_full_dataset_cache", "json");
        let cancel = AtomicBool::new(false);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&input_path)
            .expect("create score csv");
        writer
            .write_record(["timestamp", "p_long", "p_short", "score_scope"])
            .expect("header");
        for ts in candles.timestamp.iter().skip(3).take(10) {
            writer
                .write_record([
                    ts.to_string(),
                    "0.80".to_string(),
                    "0.20".to_string(),
                    "FullDataset".to_string(),
                ])
                .expect("row");
        }
        writer.flush().expect("flush score csv");

        let build = build_ml_signal_score_cache_from_csv(
            score_cache_build_request("data-1", &input_path, &cache_path, None),
            &candles,
            &cancel,
        )
        .expect("build cache");
        let err = run_ml_signal_validation_blocking_with_candles(
            MlSignalValidationRequest {
                data_id: "data-1".to_string(),
                score_cache_path: cache_path.clone(),
                candidates: vec![MlSignalPolicyParams::default()],
                iterative: IterativeSimConfig::default(),
                min_psr: None,
                min_dsr: None,
                validation_context: None,
            },
            &candles,
            &cancel,
        )
        .expect_err("partial full-dataset cache should fail validation");
        assert!(err.contains("full_dataset"));

        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&cache_path);
        let _ = fs::remove_file(&build.manifest_path);
    }
}
