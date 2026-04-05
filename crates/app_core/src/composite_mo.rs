use crate::composite_cpu::{
    evaluate_composite_cpu_trade22_metrics, CompositeCpuRequest, MaczGateParams, OttGateParams,
};
use crate::double_ma::{
    compute_halftrend_trend, compute_log_returns_f64, first_finite_idx, ma_matrix_cpu_f64,
    resolve_alma_params_for_side, run_double_ma_optimization_blocking_with_candles,
    strategy_cfg_for_optimization, validate_ma_kind, validate_strategy_cfg,
    validate_strategy_cfg_for_optimization, Backend, BackendUsed, DoubleMaMetricContract,
    DoubleMaRequest, HalfTrendRegimeFilter,
};
use crate::prescan_predictive::{predictive_prescan_seed_pairs, PredictivePrescanParams};
use crate::progress::ProgressSink;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ta_optimizer::mo::{
    EvalOutcome, Nsga2Config, ParamDomain, ParamValue, QmcMoConfig, RandomMoConfig, SearchSpace,
    TpeMoConfig,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{DoubleMaParams, Metrics, StrategyConfig};

fn emit_progress(
    progress: Option<&dyn ProgressSink>,
    processed: usize,
    total: usize,
    phase: &'static str,
) {
    crate::progress::emit_double_ma_progress(progress, processed, total, phase);
}

fn maybe_emit_progress_throttled(
    progress: Option<&dyn ProgressSink>,
    done: usize,
    total: usize,
    phase: &'static str,
    start: std::time::Instant,
    last_emit_ms: &AtomicU64,
) {
    let Some(progress) = progress else { return };
    if total == 0 {
        return;
    }

    // Keep overhead low: only consider emitting occasionally, and throttle by wall time.
    let is_final = done >= total;
    if !is_final && (done & 0x3f) != 0 {
        return;
    }

    let now_ms = start.elapsed().as_millis() as u64;
    let prev = last_emit_ms.load(Ordering::Relaxed);
    if !is_final && now_ms.saturating_sub(prev) < 150 {
        return;
    }

    if last_emit_ms
        .compare_exchange(prev, now_ms, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        emit_progress(Some(progress), done.min(total), total, phase);
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeConstraints {
    pub min_trades: Option<u32>,
    pub max_drawdown: Option<f64>,
    pub max_abs_exposure: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "sampler")]
pub enum CompositeMoSampler {
    Nsga2 { config: Nsga2Config },
    Random { config: RandomMoConfig },
    Tpe { config: TpeMoConfig },
    Qmc { config: QmcMoConfig },
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeMoObjectives {
    pub maximize: ObjectiveKind,
    pub minimize: ObjectiveKind,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositeGpuPrescanObjectives {
    Maximize,
    Minimize,
    Both,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositePrescanMaPair {
    pub fast_ma_type: String,
    pub slow_ma_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeGpuPrescanConfig {
    #[serde(default = "default_prescan_top_k")]
    pub top_k: usize,
    #[serde(default = "default_prescan_window")]
    pub window: u16,
    /// Optional predictive rescue pass: add top-K MA pairs by directional edge on a coarse MA grid.
    /// Set to 0 to disable and keep objective-only prescan behavior.
    #[serde(default = "default_prescan_predictive_top_k")]
    pub predictive_top_k: usize,
    /// Coarsening stride (in period units) for predictive rescue sampling.
    #[serde(default = "default_prescan_predictive_stride")]
    pub predictive_stride: u16,
    #[serde(default)]
    pub device_id: Option<u32>,
    #[serde(default)]
    pub require_gpu: bool,
    #[serde(default = "default_prescan_objectives")]
    pub objectives: CompositeGpuPrescanObjectives,
    #[serde(default)]
    pub ma_pairs: Option<Vec<CompositePrescanMaPair>>,
    #[serde(default = "default_prescan_mode")]
    pub mode: OptimizationMode,
}

fn default_prescan_top_k() -> usize {
    128
}

fn default_prescan_window() -> u16 {
    8
}

fn default_prescan_predictive_top_k() -> usize {
    12
}

fn default_prescan_predictive_stride() -> u16 {
    8
}

fn default_prescan_objectives() -> CompositeGpuPrescanObjectives {
    CompositeGpuPrescanObjectives::Both
}

fn default_prescan_mode() -> OptimizationMode {
    OptimizationMode::Auto
}

#[derive(Clone, Debug, Deserialize)]
pub struct CompositeMoRequest {
    pub data_id: String,

    // MA config (v1: MA types and MA params are fixed; lengths are optimized)
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,

    /// Base strategy config. Individual fields can be overridden by search-space params.
    #[serde(default)]
    pub base_strategy: StrategyConfig,
    #[serde(default)]
    pub signal_mode: CompositeSignalMode,

    /// Search space for optimization. If a param is omitted, the corresponding base/default value
    /// is used.
    pub search_space: SearchSpace,

    /// Multiobjective spec (maximize one, minimize one).
    pub objectives: CompositeMoObjectives,

    /// Sampler configuration.
    pub sampler: CompositeMoSampler,

    /// Optional constraints. Violations are handled via constrained-domination.
    pub constraints: Option<CompositeConstraints>,

    /// Optional stage-0 prescan to narrow MA length domains before multiobjective optimization.
    pub gpu_prescan: Option<CompositeGpuPrescanConfig>,

    /// Currently unused for multiobjective; present to keep request shape similar.
    #[serde(default)]
    pub mode: Option<OptimizationMode>,
}

fn default_ma_source() -> String {
    "close".to_string()
}

fn period_window_slice(periods: &[u16], center: u16, window: u16) -> &[u16] {
    let lo = center.saturating_sub(window);
    let hi = center.saturating_add(window);
    let start = periods.partition_point(|&x| x < lo);
    let end = periods.partition_point(|&x| x <= hi);
    &periods[start..end]
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct IndicatorSeriesKey {
    halftrend_amplitude: u16,
    halftrend_atr_period: u16,
    halftrend_channel_deviation_bits: u64,
    macz_fast_length: u16,
    macz_slow_length: u16,
    macz_signal_length: u16,
    macz_lengthz: u16,
    macz_length_stdev: u16,
    macz_a_bits: u64,
    macz_b_bits: u64,
    macz_gamma_bits: u64,
    macz_use_lag: bool,
    macz_use_volume: bool,
    ott_period: u16,
    ott_percent_bits: u64,
    ott_ma_type: String,
}

impl IndicatorSeriesKey {
    fn from_params(
        halftrend: &HalfTrendRegimeFilter,
        macz: &MaczGateParams,
        ott: &OttGateParams,
    ) -> Self {
        Self {
            halftrend_amplitude: halftrend.amplitude,
            halftrend_atr_period: halftrend.atr_period,
            halftrend_channel_deviation_bits: halftrend.channel_deviation.to_bits(),
            macz_fast_length: macz.fast_length,
            macz_slow_length: macz.slow_length,
            macz_signal_length: macz.signal_length,
            macz_lengthz: macz.lengthz,
            macz_length_stdev: macz.length_stdev,
            macz_a_bits: macz.a.to_bits(),
            macz_b_bits: macz.b.to_bits(),
            macz_gamma_bits: macz.gamma.to_bits(),
            macz_use_lag: macz.use_lag,
            macz_use_volume: macz.use_volume,
            ott_period: ott.period,
            ott_percent_bits: ott.percent.to_bits(),
            ott_ma_type: ott.ma_type.trim().to_ascii_lowercase(),
        }
    }
}

#[derive(Clone, Debug)]
struct IndicatorSeriesBundle {
    halftrend_trend: Vec<f64>,
    macz_hist: Vec<f64>,
    ott_line: Vec<f64>,
    first_valid: usize,
}

#[derive(Debug)]
struct IndicatorSeriesCache {
    cap: usize,
    map: HashMap<IndicatorSeriesKey, Arc<IndicatorSeriesBundle>>,
    order: VecDeque<IndicatorSeriesKey>,
}

impl IndicatorSeriesCache {
    fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &IndicatorSeriesKey) -> Option<Arc<IndicatorSeriesBundle>> {
        let hit = self.map.get(key).cloned();
        if hit.is_some() {
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                let _ = self.order.remove(pos);
            }
            self.order.push_back(key.clone());
        }
        hit
    }

    fn insert(
        &mut self,
        key: IndicatorSeriesKey,
        value: Arc<IndicatorSeriesBundle>,
    ) -> Arc<IndicatorSeriesBundle> {
        if let Some(existing) = self.map.get(&key) {
            return Arc::clone(existing);
        }
        self.map.insert(key.clone(), Arc::clone(&value));
        self.order.push_back(key);
        while self.map.len() > self.cap {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
        value
    }
}

fn composite_signal_mode_tag(mode: CompositeSignalMode) -> u8 {
    match mode {
        CompositeSignalMode::HardGate => 0,
        CompositeSignalMode::EntryCoreExitOverlay => 1,
        CompositeSignalMode::TrendPullbackReversion => 2,
    }
}

fn dynamic_plan_cache_key(
    fast_ma_type: &str,
    slow_ma_type: &str,
    indicator_key: &IndicatorSeriesKey,
    strategy: &StrategyConfig,
    signal_mode: CompositeSignalMode,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    fast_ma_type.hash(&mut hasher);
    slow_ma_type.hash(&mut hasher);
    indicator_key.hash(&mut hasher);
    strategy.commission.to_bits().hash(&mut hasher);
    strategy.eps_rel.to_bits().hash(&mut hasher);
    strategy.long_only.hash(&mut hasher);
    strategy.allow_flip.hash(&mut hasher);
    strategy.trade_on_next_bar.hash(&mut hasher);
    composite_signal_mode_tag(signal_mode).hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
struct DynamicCompositePlanCache {
    cap: usize,
    map: HashMap<u64, Arc<crate::strategy_plan::CompiledStrategyPlan>>,
    order: VecDeque<u64>,
}

impl DynamicCompositePlanCache {
    fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&mut self, key: u64) -> Option<Arc<crate::strategy_plan::CompiledStrategyPlan>> {
        let hit = self.map.get(&key).cloned();
        if hit.is_some() {
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                let _ = self.order.remove(pos);
            }
            self.order.push_back(key);
        }
        hit
    }

    fn insert(
        &mut self,
        key: u64,
        value: Arc<crate::strategy_plan::CompiledStrategyPlan>,
    ) -> Arc<crate::strategy_plan::CompiledStrategyPlan> {
        if let Some(existing) = self.map.get(&key) {
            return Arc::clone(existing);
        }
        self.map.insert(key.clone(), Arc::clone(&value));
        self.order.push_back(key);
        while self.map.len() > self.cap {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
        value
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoParams {
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    pub fast_len: u16,
    pub slow_len: u16,

    pub halftrend: HalfTrendRegimeFilter,
    pub macz: MaczGateParams,
    pub ott: OttGateParams,
    pub strategy: StrategyConfig,
    pub signal_mode: CompositeSignalMode,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoPoint {
    pub params: CompositeMoParams,
    pub metrics: Metrics,
    pub maximize_value: f64,
    pub minimize_value: f64,
    pub violation: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoPrescanTypePairTelemetry {
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    pub candidate_fast_periods: usize,
    pub candidate_slow_periods: usize,
    pub candidate_pairs: usize,
    pub pair_density: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoPrescanTelemetry {
    pub requested_device_id: Option<u32>,
    pub resolved_device_id: Option<u32>,
    pub backend_used: String,
    pub backend_route: Option<String>,
    pub backend_fallback_reason: Option<String>,
    pub mode: String,
    pub require_gpu: bool,
    pub runtime_ms: u64,
    pub requested_scan_pairs: usize,
    pub candidate_fast_periods: usize,
    pub candidate_slow_periods: usize,
    pub candidate_pairs: usize,
    pub pair_density: f64,
    pub type_pairs: Vec<CompositeMoPrescanTypePairTelemetry>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoSearchTelemetry {
    pub sampler: String,
    pub requested_evaluations: usize,
    pub actual_evaluations: usize,
    pub unique_evaluations: usize,
    pub duplicate_evaluations_avoided: usize,
    pub feasible_points: usize,
    pub rejected_decode_or_order: usize,
    pub rejected_prescan_pair: usize,
    pub rejected_other_constraints: usize,
    pub exact_grid_used: bool,
    pub exact_grid_total_combinations: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoTelemetry {
    pub prescan: Option<CompositeMoPrescanTelemetry>,
    pub search: CompositeMoSearchTelemetry,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoResult {
    pub data_id: String,
    pub pareto: Vec<CompositeMoPoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_surface_kind: Option<String>,
    pub total_evaluations: usize,
    pub runtime_ms: u64,
    pub telemetry: CompositeMoTelemetry,
}

fn composite_mo_trade22_request(
    req: &CompositeMoRequest,
    params: &CompositeMoParams,
) -> CompositeCpuRequest {
    CompositeCpuRequest {
        backend: Backend::CpuOnly,
        data_id: req.data_id.clone(),
        fast_range: (params.fast_len as u32, params.fast_len as u32, 1),
        slow_range: (params.slow_len as u32, params.slow_len as u32, 1),
        fast_ma_types: vec![params.fast_ma_type.clone()],
        slow_ma_types: vec![params.slow_ma_type.clone()],
        ma_source: req.ma_source.clone(),
        fast_ma_params: req.fast_ma_params.clone(),
        slow_ma_params: req.slow_ma_params.clone(),
        strategy: params.strategy.clone(),
        halftrend: params.halftrend.clone(),
        macz: params.macz.clone(),
        ott: params.ott.clone(),
        signal_mode: params.signal_mode,
        asr: None,
        vcc: None,
        vemr: None,
        rocb: None,
        vcet: None,
        mkcs: None,
        kqsr: None,
        carm: None,
        kvdf: None,
        cdmc: None,
        objective: req.objectives.maximize,
        mode: OptimizationMode::Grid,
        gpu_prescan: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        top_k: Some(1),
        include_all: Some(false),
        heatmap_bins: None,
        include_trade_metrics: true,
        metric_contract: Some(DoubleMaMetricContract::Trade22),
    }
}

fn composite_mo_trade22_params(params: &CompositeMoParams) -> DoubleMaParams {
    DoubleMaParams {
        fast_len: params.fast_len,
        slow_len: params.slow_len,
        fast_ma_id: 0,
        slow_ma_id: 0,
    }
}

struct ParamIdx {
    fast_ma_type: Option<usize>,
    slow_ma_type: Option<usize>,
    fast_len: usize,
    slow_len: usize,

    halftrend_amplitude: Option<usize>,
    halftrend_channel_deviation: Option<usize>,
    halftrend_atr_period: Option<usize>,

    macz_fast_length: Option<usize>,
    macz_slow_length: Option<usize>,
    macz_signal_length: Option<usize>,
    macz_lengthz: Option<usize>,
    macz_length_stdev: Option<usize>,
    macz_a: Option<usize>,
    macz_b: Option<usize>,
    macz_gamma: Option<usize>,
    macz_use_lag: Option<usize>,
    macz_use_volume: Option<usize>,

    ott_period: Option<usize>,
    ott_percent: Option<usize>,
    ott_ma_type: Option<usize>,

    strategy_commission: Option<usize>,
    strategy_eps_rel: Option<usize>,
    strategy_long_only: Option<usize>,
    strategy_allow_flip: Option<usize>,
}

type MaTypePairKey = (String, String);
type AllowedPairsByType = HashMap<MaTypePairKey, HashSet<(u16, u16)>>;

fn find_param(space: &SearchSpace, name: &str) -> Option<usize> {
    space
        .params
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(name))
}

const MAX_PRESCAN_PAIRS_PER_TYPE: usize = 4096;
const EXACT_GRID_MAX_AXIS_VALUES: usize = 256;
const EXACT_GRID_MAX_COMBOS: usize = 4096;

fn composite_mo_supported_param_names() -> &'static [&'static str] {
    &[
        "fast_ma_type",
        "slow_ma_type",
        "fast_len",
        "slow_len",
        "halftrend_amplitude",
        "halftrend_channel_deviation",
        "halftrend_atr_period",
        "macz_fast_length",
        "macz_slow_length",
        "macz_signal_length",
        "macz_lengthz",
        "macz_length_stdev",
        "macz_a",
        "macz_b",
        "macz_gamma",
        "macz_use_lag",
        "macz_use_volume",
        "ott_period",
        "ott_percent",
        "ott_ma_type",
        "strategy_commission",
        "strategy_eps_rel",
        "strategy_long_only",
        "strategy_allow_flip",
    ]
}

fn validate_composite_mo_search_space(space: &SearchSpace) -> Result<(), String> {
    if find_param(space, "strategy_trade_on_next_bar").is_some() {
        return Err(
            "search_space param 'strategy_trade_on_next_bar' is not permitted because same-bar execution is disabled."
                .to_string(),
        );
    }

    for param in &space.params {
        let name = param.name.trim();
        if !composite_mo_supported_param_names()
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(name))
        {
            return Err(format!(
                "unsupported CompositeMo search-space param '{name}'"
            ));
        }
    }

    Ok(())
}

fn parse_u16_from_choice(s: &str) -> Result<u16, String> {
    let v = s
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("invalid period choice '{s}'"))?;
    if v == 0 || v > u16::MAX as u32 {
        return Err(format!("period choice out of range: {v}"));
    }
    Ok(v as u16)
}

fn allowed_periods_from_domain(domain: &ParamDomain) -> Result<Vec<u16>, String> {
    match domain {
        ParamDomain::Categorical { choices } => {
            // For categorical domains, we return the unique period set used for MA precompute.
            // NOTE: do not use this for decoding ParamValue::Cat indices.
            let mut out: Vec<u16> = choices
                .iter()
                .map(|c| parse_u16_from_choice(c))
                .collect::<Result<Vec<_>, _>>()?;
            out.sort_unstable();
            out.dedup();
            if out.is_empty() {
                Err("empty categorical period choices".to_string())
            } else {
                Ok(out)
            }
        }
        ParamDomain::IntRange {
            low,
            high,
            step,
            log: _,
        } => {
            if *step <= 0 {
                return Err("period int range step must be > 0".to_string());
            }
            let lo = (*low).min(*high);
            let hi = (*low).max(*high);
            if lo <= 0 {
                return Err("period int range must be > 0".to_string());
            }
            let mut out: Vec<u16> = Vec::new();
            let mut v = lo;
            while v <= hi {
                if v > u16::MAX as i64 {
                    break;
                }
                out.push(v as u16);
                match v.checked_add(*step) {
                    Some(n) if n > v => v = n,
                    _ => break,
                }
            }
            if out.is_empty() {
                return Err("empty period range".to_string());
            }
            Ok(out)
        }
        other => Err(format!(
            "period domain must be categorical or int range, got {other:?}"
        )),
    }
}

fn period_choice_map(domain: &ParamDomain) -> Result<Option<Vec<u16>>, String> {
    match domain {
        ParamDomain::Categorical { choices } => Ok(Some(
            choices
                .iter()
                .map(|c| parse_u16_from_choice(c))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ParamDomain::IntRange { .. } => Ok(None),
        other => Err(format!(
            "period domain must be categorical or int range, got {other:?}"
        )),
    }
}

fn numeric_choice_map_u16(domain: &ParamDomain) -> Result<Option<Vec<u16>>, String> {
    match domain {
        ParamDomain::Categorical { choices } => Ok(Some(
            choices
                .iter()
                .map(|choice| {
                    let raw = choice.trim();
                    let parsed = raw
                        .parse::<u32>()
                        .map_err(|_| format!("invalid numeric choice '{choice}'"))?;
                    if parsed == 0 || parsed > u16::MAX as u32 {
                        return Err(format!("numeric choice out of range: {parsed}"));
                    }
                    Ok(parsed as u16)
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ParamDomain::IntRange { .. } => Ok(None),
        ParamDomain::FloatRange { .. } => Ok(None),
        ParamDomain::Bool => Ok(None),
    }
}

fn categorical_choice<'a>(
    space: &'a SearchSpace,
    values: &[ParamValue],
    idx: usize,
) -> Result<Option<&'a str>, String> {
    let Some(ParamValue::Cat(choice_idx)) = values.get(idx) else {
        return Ok(None);
    };
    let Some(def) = space.params.get(idx) else {
        return Err("param idx invalid".to_string());
    };
    let ParamDomain::Categorical { choices } = &def.domain else {
        return Err(format!("param '{}' must be categorical", def.name));
    };
    let Some(choice) = choices.get(*choice_idx) else {
        return Err(format!(
            "param '{}' categorical index out of range",
            def.name
        ));
    };
    Ok(Some(choice.as_str()))
}

fn parse_bool_choice(choice: &str) -> Result<bool, String> {
    let normalized = choice.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid bool choice '{choice}'")),
    }
}

fn decode_u16_param(space: &SearchSpace, values: &[ParamValue], idx: usize) -> Result<u16, String> {
    match values.get(idx) {
        Some(ParamValue::Cat(_)) => {
            let choice = categorical_choice(space, values, idx)?
                .ok_or_else(|| "categorical choice missing".to_string())?;
            let parsed = choice
                .trim()
                .parse::<u32>()
                .map_err(|_| format!("invalid numeric choice '{choice}'"))?;
            if parsed == 0 || parsed > u16::MAX as u32 {
                return Err(format!("numeric choice out of range: {parsed}"));
            }
            Ok(parsed as u16)
        }
        _ => get_u16(values, idx),
    }
}

fn set_u16_param_value(
    values: &mut [ParamValue],
    idx: usize,
    space: &SearchSpace,
    choice_idx_by_period: Option<&HashMap<u16, usize>>,
    value: u16,
) {
    if idx >= values.len() {
        return;
    }
    let Some(def) = space.params.get(idx) else {
        return;
    };
    match &def.domain {
        ParamDomain::Categorical { .. } => {
            if let Some(choice_idx) =
                choice_idx_by_period.and_then(|choices| choices.get(&value).copied())
            {
                values[idx] = ParamValue::Cat(choice_idx);
            }
        }
        ParamDomain::IntRange {
            low, high, step, ..
        } => {
            let lo = (*low).min(*high).max(1);
            let hi = (*low).max(*high).max(lo);
            let step = (*step).max(1);
            let clamped = (value as i64).clamp(lo, hi);
            let aligned = lo + (((clamped - lo) / step) * step);
            values[idx] = ParamValue::Int(aligned.clamp(lo, hi));
        }
        _ => {}
    }
}

fn decode_f64_param(space: &SearchSpace, values: &[ParamValue], idx: usize) -> Result<f64, String> {
    match values.get(idx) {
        Some(ParamValue::Cat(_)) => {
            let choice = categorical_choice(space, values, idx)?
                .ok_or_else(|| "categorical choice missing".to_string())?;
            choice
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("invalid float choice '{choice}'"))
        }
        _ => get_f64(values, idx),
    }
}

fn decode_bool_param(
    space: &SearchSpace,
    values: &[ParamValue],
    idx: usize,
) -> Result<bool, String> {
    match values.get(idx) {
        Some(ParamValue::Cat(_)) => {
            let choice = categorical_choice(space, values, idx)?
                .ok_or_else(|| "categorical choice missing".to_string())?;
            parse_bool_choice(choice)
        }
        _ => get_bool(values, idx),
    }
}

fn get_u16(values: &[ParamValue], idx: usize) -> Result<u16, String> {
    match values.get(idx) {
        Some(ParamValue::Int(v)) => {
            let v = *v;
            if v <= 0 || v > u16::MAX as i64 {
                return Err(format!("int out of u16 range: {v}"));
            }
            Ok(v as u16)
        }
        Some(ParamValue::Float(v)) => {
            let v = *v;
            if !v.is_finite() {
                return Err("float is not finite".to_string());
            }
            let vi = v.round() as i64;
            if vi <= 0 || vi > u16::MAX as i64 {
                return Err(format!("float out of u16 range: {v}"));
            }
            Ok(vi as u16)
        }
        Some(ParamValue::Cat(i)) => {
            if *i > u16::MAX as usize {
                return Err("categorical index out of u16 range".to_string());
            }
            Ok((*i) as u16)
        }
        Some(ParamValue::Bool(b)) => Ok(if *b { 1 } else { 0 }),
        None => Err("param missing".to_string()),
    }
}

fn get_f64(values: &[ParamValue], idx: usize) -> Result<f64, String> {
    match values.get(idx) {
        Some(ParamValue::Float(v)) => Ok(*v),
        Some(ParamValue::Int(v)) => Ok(*v as f64),
        Some(ParamValue::Bool(b)) => Ok(if *b { 1.0 } else { 0.0 }),
        Some(ParamValue::Cat(v)) => Ok(*v as f64),
        None => Err("param missing".to_string()),
    }
}

fn get_bool(values: &[ParamValue], idx: usize) -> Result<bool, String> {
    match values.get(idx) {
        Some(ParamValue::Bool(b)) => Ok(*b),
        Some(ParamValue::Int(v)) => Ok(*v != 0),
        Some(ParamValue::Cat(v)) => Ok(*v != 0),
        Some(ParamValue::Float(v)) => Ok(v.is_finite() && *v != 0.0),
        None => Err("param missing".to_string()),
    }
}

fn metric_value(m: &Metrics, k: ObjectiveKind) -> f64 {
    match k {
        ObjectiveKind::Pnl => m.pnl,
        ObjectiveKind::Sharpe => m.sharpe,
        ObjectiveKind::Sortino => m.sortino,
        ObjectiveKind::Calmar => m.calmar,
        ObjectiveKind::Omega => m.omega,
        ObjectiveKind::Volatility => m.volatility,
        ObjectiveKind::MaxDrawdown => m.max_dd,
    }
}

fn objective_needs_return_stats(k: ObjectiveKind) -> bool {
    matches!(
        k,
        ObjectiveKind::Sharpe
            | ObjectiveKind::Sortino
            | ObjectiveKind::Omega
            | ObjectiveKind::Volatility
    )
}

fn validate_objectives(objs: &CompositeMoObjectives) -> Result<(), String> {
    if objs.maximize == objs.minimize {
        return Err("maximize and minimize objectives must be different".to_string());
    }
    if matches!(
        objs.maximize,
        ObjectiveKind::MaxDrawdown | ObjectiveKind::Volatility
    ) {
        return Err(format!(
            "maximize objective {:?} is a risk metric; choose Sharpe/Sortino/Calmar/Omega/Pnl",
            objs.maximize
        ));
    }
    Ok(())
}

fn decode_len(
    domain: &ParamDomain,
    values: &[ParamValue],
    idx: usize,
    choice_map: Option<&[u16]>,
) -> Result<u16, String> {
    match (domain, values.get(idx)) {
        (ParamDomain::Categorical { .. }, Some(ParamValue::Cat(c))) => choice_map
            .and_then(|m| m.get(*c).copied())
            .ok_or_else(|| "categorical period index out of range".to_string()),
        _ => get_u16(values, idx),
    }
}

fn decode_ma_type(
    space: &SearchSpace,
    values: &[ParamValue],
    param_idx: Option<usize>,
    fallback: &str,
) -> Result<String, String> {
    let Some(i) = param_idx else {
        return Ok(fallback.to_string());
    };
    let Some(def) = space.params.get(i) else {
        return Ok(fallback.to_string());
    };
    let ParamDomain::Categorical { choices } = &def.domain else {
        return Err(format!("param '{}' must be categorical", def.name));
    };

    match values.get(i) {
        Some(ParamValue::Cat(ci)) => {
            let Some(choice) = choices.get(*ci) else {
                return Err("categorical index out of range".to_string());
            };
            let ma = choice.trim().to_ascii_lowercase();
            if ma.is_empty() {
                return Err("ma type choice cannot be empty".to_string());
            }
            Ok(ma)
        }
        _ => Ok(fallback.to_string()),
    }
}

fn set_len_value(
    values: &mut [ParamValue],
    idx: usize,
    domain: &ParamDomain,
    choice_idx_by_period: Option<&HashMap<u16, usize>>,
    v: u16,
) {
    if idx >= values.len() {
        return;
    }
    match domain {
        ParamDomain::Categorical { .. } => {
            if let Some(map) = choice_idx_by_period {
                if let Some(&ci) = map.get(&v) {
                    values[idx] = ParamValue::Cat(ci);
                }
            }
        }
        ParamDomain::IntRange { .. } => {
            values[idx] = ParamValue::Int(v as i64);
        }
        _ => {}
    }
}

fn backend_used_label(backend: &BackendUsed) -> &'static str {
    match backend {
        BackendUsed::Cpu => "Cpu",
        BackendUsed::Gpu => "Gpu",
    }
}

fn arithmetic_period_runs(periods: &[u16]) -> Vec<&[u16]> {
    if periods.len() <= 2 {
        return vec![periods];
    }

    let mut runs: Vec<&[u16]> = Vec::new();
    let mut start = 0usize;
    while start < periods.len() {
        if start + 2 >= periods.len() {
            runs.push(&periods[start..]);
            break;
        }

        let step = periods[start + 1].saturating_sub(periods[start]);
        let mut end = start + 2;
        while end < periods.len() {
            let prev = periods[end - 1];
            let cur = periods[end];
            if cur.saturating_sub(prev) != step {
                break;
            }
            end += 1;
        }
        runs.push(&periods[start..end]);
        start = end;
    }
    runs
}

fn default_halftrend() -> HalfTrendRegimeFilter {
    HalfTrendRegimeFilter {
        amplitude: 2,
        channel_deviation: 2.0,
        atr_period: 100,
    }
}

fn default_macz() -> MaczGateParams {
    MaczGateParams::default()
}

fn default_ott() -> OttGateParams {
    OttGateParams::default()
}

fn build_param_indices(space: &SearchSpace) -> Result<ParamIdx, String> {
    let Some(fast_len) = find_param(space, "fast_len") else {
        return Err("search_space is missing required param 'fast_len'".to_string());
    };
    let Some(slow_len) = find_param(space, "slow_len") else {
        return Err("search_space is missing required param 'slow_len'".to_string());
    };

    Ok(ParamIdx {
        fast_ma_type: find_param(space, "fast_ma_type"),
        slow_ma_type: find_param(space, "slow_ma_type"),
        fast_len,
        slow_len,

        halftrend_amplitude: find_param(space, "halftrend_amplitude"),
        halftrend_channel_deviation: find_param(space, "halftrend_channel_deviation"),
        halftrend_atr_period: find_param(space, "halftrend_atr_period"),

        macz_fast_length: find_param(space, "macz_fast_length"),
        macz_slow_length: find_param(space, "macz_slow_length"),
        macz_signal_length: find_param(space, "macz_signal_length"),
        macz_lengthz: find_param(space, "macz_lengthz"),
        macz_length_stdev: find_param(space, "macz_length_stdev"),
        macz_a: find_param(space, "macz_a"),
        macz_b: find_param(space, "macz_b"),
        macz_gamma: find_param(space, "macz_gamma"),
        macz_use_lag: find_param(space, "macz_use_lag"),
        macz_use_volume: find_param(space, "macz_use_volume"),

        ott_period: find_param(space, "ott_period"),
        ott_percent: find_param(space, "ott_percent"),
        ott_ma_type: find_param(space, "ott_ma_type"),

        strategy_commission: find_param(space, "strategy_commission"),
        strategy_eps_rel: find_param(space, "strategy_eps_rel"),
        strategy_long_only: find_param(space, "strategy_long_only"),
        strategy_allow_flip: find_param(space, "strategy_allow_flip"),
    })
}

fn count_valid_period_pairs(fast_periods: &BTreeSet<u16>, slow_periods: &BTreeSet<u16>) -> usize {
    let mut total = 0usize;
    for &fast in fast_periods {
        for &slow in slow_periods {
            if fast < slow {
                total = total.saturating_add(1);
            }
        }
    }
    total
}

fn pair_density(candidate_pairs: usize, possible_pairs: usize) -> f64 {
    if possible_pairs == 0 {
        0.0
    } else {
        (candidate_pairs as f64) / (possible_pairs as f64)
    }
}

fn nearest_allowed_pair(
    pairs: &HashSet<(u16, u16)>,
    target_fast: u16,
    target_slow: u16,
) -> Option<(u16, u16)> {
    pairs.iter().copied().min_by_key(|&(fast, slow)| {
        let fast_delta = (fast as i64 - target_fast as i64).unsigned_abs();
        let slow_delta = (slow as i64 - target_slow as i64).unsigned_abs();
        let order_penalty = if fast < slow { 0u64 } else { 1u64 };
        (
            fast_delta.saturating_add(slow_delta),
            order_penalty,
            slow as u64,
            fast as u64,
        )
    })
}

fn resolve_dynamic_artifact_backend(prescan: Option<&CompositeMoPrescanTelemetry>) -> String {
    match prescan {
        Some(telemetry) if !telemetry.backend_used.is_empty() => {
            format!("Cpu+Prescan({})", telemetry.backend_used)
        }
        _ => "Cpu".to_string(),
    }
}

pub fn run_composite_mo_optimization_blocking_with_candles(
    mut req: CompositeMoRequest,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    progress: Option<&dyn ProgressSink>,
) -> Result<CompositeMoResult, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".to_string());
    }

    let start = std::time::Instant::now();
    emit_progress(progress, 0, 1, "composite_mo_prep");

    // Multiobjective mode currently ignores OptimizationMode; this is a placeholder for later.
    let _ = req.mode;

    validate_strategy_cfg_for_optimization(&req.base_strategy)?;
    req.base_strategy = strategy_cfg_for_optimization(req.base_strategy.clone());
    validate_objectives(&req.objectives)?;
    req.search_space.validate()?;
    validate_composite_mo_search_space(&req.search_space)?;
    let signal_mode = req.signal_mode;

    let base_fast_ma = req.fast_ma_type.trim().to_ascii_lowercase();
    let base_slow_ma = req.slow_ma_type.trim().to_ascii_lowercase();
    if base_fast_ma.is_empty() || base_slow_ma.is_empty() {
        return Err("fast_ma_type and slow_ma_type cannot be empty.".to_string());
    }
    validate_ma_kind(&base_fast_ma)?;
    validate_ma_kind(&base_slow_ma)?;

    let idx = build_param_indices(&req.search_space)?;

    let mut fast_domain = &req
        .search_space
        .params
        .get(idx.fast_len)
        .ok_or_else(|| "fast_len param idx invalid".to_string())?
        .domain;
    let mut slow_domain = &req
        .search_space
        .params
        .get(idx.slow_len)
        .ok_or_else(|| "slow_len param idx invalid".to_string())?
        .domain;

    let mut fast_choice_map = period_choice_map(fast_domain)?;
    let mut slow_choice_map = period_choice_map(slow_domain)?;

    let mut fast_periods = allowed_periods_from_domain(fast_domain)?;
    let mut slow_periods = allowed_periods_from_domain(slow_domain)?;
    let mut allowed_pairs_by_type: Option<AllowedPairsByType> = None;
    let mut prescan_telemetry: Option<CompositeMoPrescanTelemetry> = None;

    // Safety limits: precomputing full MA matrices is expensive in RAM.
    let max_periods = 256usize;
    let to_u32_range =
        |low: i64, high: i64, step: i64, name: &str| -> Result<(u32, u32, u32), String> {
            if step <= 0 {
                return Err(format!("{name} int range step must be > 0"));
            }
            if step > u32::MAX as i64 {
                return Err(format!("{name} int range step too large"));
            }
            let lo = low.min(high);
            let hi = low.max(high);
            if lo <= 0 || hi <= 0 {
                return Err(format!("{name} int range must be > 0"));
            }
            if lo > u16::MAX as i64 {
                return Err(format!("{name} range low exceeds u16::MAX"));
            }
            let hi = hi.min(u16::MAX as i64);
            Ok((lo as u32, hi as u32, step as u32))
        };

    if let Some(cfg) = req.gpu_prescan.clone() {
        let fast_range = match fast_domain {
            ParamDomain::IntRange {
                low,
                high,
                step,
                log: _,
            } => Some(to_u32_range(*low, *high, *step, "fast_len")?),
            _ => None,
        };
        let slow_range = match slow_domain {
            ParamDomain::IntRange {
                low,
                high,
                step,
                log: _,
            } => Some(to_u32_range(*low, *high, *step, "slow_len")?),
            _ => None,
        };

        if let (Some(fast_range), Some(slow_range)) = (fast_range, slow_range) {
            let resolved_device_id = match cfg.device_id {
                Some(device_id) => Some(device_id),
                None => crate::preferred_cuda_device()
                    .map_err(|err| format!("failed to resolve preferred CUDA device: {err}"))?
                    .map(|device| device.device_id),
            };
            let device_id = resolved_device_id.unwrap_or(0);
            let prescan_started = std::time::Instant::now();
            let mut backend_used_summary: Option<String> = None;
            let mut backend_route_summary: Option<String> = None;
            let mut backend_fallback_reason_summary: Option<String> = None;

            let mut scan_objectives: Vec<ObjectiveKind> = Vec::new();
            match cfg.objectives {
                CompositeGpuPrescanObjectives::Maximize => {
                    scan_objectives.push(req.objectives.maximize)
                }
                CompositeGpuPrescanObjectives::Minimize => {
                    scan_objectives.push(req.objectives.minimize)
                }
                CompositeGpuPrescanObjectives::Both => {
                    scan_objectives.push(req.objectives.maximize);
                    if req.objectives.minimize != req.objectives.maximize {
                        scan_objectives.push(req.objectives.minimize);
                    }
                }
            }

            let mut fast_set: BTreeSet<u16> = BTreeSet::new();
            let mut slow_set: BTreeSet<u16> = BTreeSet::new();
            let mut allowed_pairs_map: AllowedPairsByType = HashMap::new();

            let max_pairs = 8usize;
            let mut scan_pairs: Vec<(String, String)> = cfg
                .ma_pairs
                .as_ref()
                .map(|pairs| {
                    pairs
                        .iter()
                        .map(|p| {
                            (
                                p.fast_ma_type.trim().to_ascii_lowercase(),
                                p.slow_ma_type.trim().to_ascii_lowercase(),
                            )
                        })
                        .collect()
                })
                .unwrap_or_else(|| vec![(base_fast_ma.clone(), base_slow_ma.clone())]);

            scan_pairs.retain(|(f, s)| !f.is_empty() && !s.is_empty());
            scan_pairs.sort();
            scan_pairs.dedup();

            if scan_pairs.is_empty() {
                return Err("gpu_prescan ma_pairs is empty".to_string());
            }
            if scan_pairs.len() > max_pairs {
                return Err(format!(
                    "gpu_prescan ma_pairs too large ({} > {max_pairs})",
                    scan_pairs.len()
                ));
            }
            for (f, s) in &scan_pairs {
                validate_ma_kind(f)?;
                validate_ma_kind(s)?;
            }

            for (fast_ma_type, slow_ma_type) in scan_pairs.iter().cloned() {
                let type_key = (fast_ma_type.clone(), slow_ma_type.clone());
                let (fast_alma_offset, fast_alma_sigma) = resolve_alma_params_for_side(
                    "fast",
                    &fast_ma_type,
                    req.fast_ma_params.as_ref(),
                )?;
                let (slow_alma_offset, slow_alma_sigma) = resolve_alma_params_for_side(
                    "slow",
                    &slow_ma_type,
                    req.slow_ma_params.as_ref(),
                )?;

                for objective in scan_objectives.iter().copied() {
                    if cancel.load(Ordering::Relaxed) {
                        return Err("cancelled".to_string());
                    }

                    let dm_req = DoubleMaRequest {
                        backend: Backend::Auto { device_id },
                        data_id: req.data_id.clone(),
                        fast_range,
                        slow_range,
                        fast_ma_types: vec![fast_ma_type.clone()],
                        slow_ma_types: vec![slow_ma_type.clone()],
                        ma_source: req.ma_source.clone(),
                        export_all_csv_path: None,
                        fast_ma_params: req.fast_ma_params.clone(),
                        slow_ma_params: req.slow_ma_params.clone(),
                        fast_ma_param_sweeps: None,
                        slow_ma_param_sweeps: None,
                        strategy: req.base_strategy.clone(),
                        halftrend_filter: None,
                        objective,
                        score_mode: None,
                        extra_shortlist_objectives: None,
                        shortlist_confidence_mode: None,
                        shortlist_union_mode: None,
                        shortlist_slice_mode: None,
                        shortlist_slice_benchmark: None,
                        shortlist_neighborhood_radius: None,
                        shortlist_neighborhood_step: None,
                        mode: cfg.mode,
                        top_k: Some(cfg.top_k),
                        include_all: Some(false),
                        heatmap_bins: Some(0),
                        constraints: None,
                        vdps: None,
                        dynamic_strategy_ir: None,
                        gpu_execution: crate::double_ma::GpuExecutionPolicy::BestEffort,
                        persist_followup_cache: false,
                        include_trade_metrics: false,
                        metric_contract: None,
                        hurdle_tau: None,
                    };

                    let dm_res = run_double_ma_optimization_blocking_with_candles(
                        dm_req, candles, cancel, progress,
                    )?;
                    let backend_label = backend_used_label(&dm_res.backend_used).to_string();
                    backend_used_summary = Some(match backend_used_summary.take() {
                        None => backend_label,
                        Some(prev) if prev == backend_label => prev,
                        Some(_) => "Mixed".to_string(),
                    });
                    if let Some(route) = dm_res.backend_route.as_ref() {
                        backend_route_summary = Some(match backend_route_summary.take() {
                            None => route.clone(),
                            Some(prev) if prev == *route => prev,
                            Some(_) => "Mixed".to_string(),
                        });
                    }
                    if let Some(reason) = dm_res.backend_fallback_reason.as_ref() {
                        backend_fallback_reason_summary =
                            Some(match backend_fallback_reason_summary.take() {
                                None => reason.clone(),
                                Some(prev) if prev == *reason => prev,
                                Some(_) => "Mixed".to_string(),
                            });
                    }
                    if cfg.require_gpu && matches!(dm_res.backend_used.clone(), BackendUsed::Cpu) {
                        return Err(
                            "gpu_prescan require_gpu=true but GPU backend was not available (CPU used)"
                                .to_string(),
                        );
                    }

                    let mut pairs: Vec<(u16, u16)> =
                        Vec::with_capacity(dm_res.top.len().saturating_add(1));
                    pairs.push((dm_res.best_params.fast_len, dm_res.best_params.slow_len));
                    for (p, _) in &dm_res.top {
                        pairs.push((p.fast_len, p.slow_len));
                    }

                    let allowed_entry = allowed_pairs_map.entry(type_key.clone()).or_default();
                    for (f, s) in pairs {
                        let fast_window = period_window_slice(&fast_periods, f, cfg.window);
                        let slow_window = period_window_slice(&slow_periods, s, cfg.window);
                        for &fast in fast_window {
                            for &slow in slow_window {
                                if fast >= slow {
                                    continue;
                                }
                                fast_set.insert(fast);
                                slow_set.insert(slow);
                                allowed_entry.insert((fast, slow));
                            }
                        }
                    }
                }

                if cfg.predictive_top_k > 0 {
                    let rescue = predictive_prescan_seed_pairs(PredictivePrescanParams {
                        candles,
                        ma_source: &req.ma_source,
                        fast_periods: &fast_periods,
                        slow_periods: &slow_periods,
                        fast_ma_type: &fast_ma_type,
                        slow_ma_type: &slow_ma_type,
                        fast_ma_params: req.fast_ma_params.as_ref(),
                        slow_ma_params: req.slow_ma_params.as_ref(),
                        fast_alma_offset,
                        fast_alma_sigma,
                        slow_alma_offset,
                        slow_alma_sigma,
                        long_only: req.base_strategy.long_only,
                        top_k: cfg.predictive_top_k,
                        period_stride: cfg.predictive_stride,
                    })?;

                    for (f, s) in rescue {
                        let allowed_entry = allowed_pairs_map.entry(type_key.clone()).or_default();
                        let fast_window = period_window_slice(&fast_periods, f, cfg.window);
                        let slow_window = period_window_slice(&slow_periods, s, cfg.window);
                        for &fast in fast_window {
                            for &slow in slow_window {
                                if fast >= slow {
                                    continue;
                                }
                                fast_set.insert(fast);
                                slow_set.insert(slow);
                                allowed_entry.insert((fast, slow));
                            }
                        }
                    }
                }
            }

            let fast_candidates: Vec<u16> = fast_set.into_iter().collect();
            let slow_candidates: Vec<u16> = slow_set.into_iter().collect();
            if fast_candidates.is_empty()
                || slow_candidates.is_empty()
                || allowed_pairs_map.values().all(HashSet::is_empty)
            {
                return Err("gpu_prescan produced empty candidate set".to_string());
            }
            if fast_candidates.len() > max_periods || slow_candidates.len() > max_periods {
                return Err(format!(
                    "gpu_prescan produced too many MA periods (fast={}, slow={}); lower window/top_k or increase step",
                    fast_candidates.len(),
                    slow_candidates.len()
                ));
            }

            // Rewrite domains to discrete candidate sets.
            if let Some(p) = req.search_space.params.get_mut(idx.fast_len) {
                p.domain = ParamDomain::Categorical {
                    choices: fast_candidates.iter().map(|p| p.to_string()).collect(),
                };
            }
            if let Some(p) = req.search_space.params.get_mut(idx.slow_len) {
                p.domain = ParamDomain::Categorical {
                    choices: slow_candidates.iter().map(|p| p.to_string()).collect(),
                };
            }

            // Re-bind domains + allowed sets after rewrite.
            fast_domain = &req
                .search_space
                .params
                .get(idx.fast_len)
                .ok_or_else(|| "fast_len param idx invalid".to_string())?
                .domain;
            slow_domain = &req
                .search_space
                .params
                .get(idx.slow_len)
                .ok_or_else(|| "slow_len param idx invalid".to_string())?
                .domain;
            fast_choice_map = period_choice_map(fast_domain)?;
            slow_choice_map = period_choice_map(slow_domain)?;
            fast_periods = allowed_periods_from_domain(fast_domain)?;
            slow_periods = allowed_periods_from_domain(slow_domain)?;
            let fast_allowed: HashSet<u16> = fast_periods.iter().copied().collect();
            let slow_allowed: HashSet<u16> = slow_periods.iter().copied().collect();
            for pairs in allowed_pairs_map.values_mut() {
                pairs
                    .retain(|(f, s)| fast_allowed.contains(f) && slow_allowed.contains(s) && f < s);
            }
            allowed_pairs_map.retain(|_, pairs| !pairs.is_empty());
            if allowed_pairs_map.is_empty() {
                return Err("gpu_prescan produced no valid parameter combinations".to_string());
            }

            let mut type_pairs: Vec<CompositeMoPrescanTypePairTelemetry> = Vec::new();
            let mut candidate_pair_total = 0usize;
            let mut possible_pair_total = 0usize;
            let mut type_keys: Vec<_> = allowed_pairs_map.keys().cloned().collect();
            type_keys.sort();
            for (fast_ma_type, slow_ma_type) in type_keys {
                let Some(pairs) =
                    allowed_pairs_map.get(&(fast_ma_type.clone(), slow_ma_type.clone()))
                else {
                    continue;
                };
                if pairs.len() > MAX_PRESCAN_PAIRS_PER_TYPE {
                    return Err(format!(
                        "gpu_prescan produced too many candidate pairs for ({fast_ma_type}, {slow_ma_type}) ({} > {})",
                        pairs.len(),
                        MAX_PRESCAN_PAIRS_PER_TYPE
                    ));
                }
                let mut fast_candidates_for_type: BTreeSet<u16> = BTreeSet::new();
                let mut slow_candidates_for_type: BTreeSet<u16> = BTreeSet::new();
                for &(fast, slow) in pairs {
                    fast_candidates_for_type.insert(fast);
                    slow_candidates_for_type.insert(slow);
                }
                let possible_pairs =
                    count_valid_period_pairs(&fast_candidates_for_type, &slow_candidates_for_type);
                candidate_pair_total = candidate_pair_total.saturating_add(pairs.len());
                possible_pair_total = possible_pair_total.saturating_add(possible_pairs);
                type_pairs.push(CompositeMoPrescanTypePairTelemetry {
                    fast_ma_type,
                    slow_ma_type,
                    candidate_fast_periods: fast_candidates_for_type.len(),
                    candidate_slow_periods: slow_candidates_for_type.len(),
                    candidate_pairs: pairs.len(),
                    pair_density: pair_density(pairs.len(), possible_pairs),
                });
            }

            prescan_telemetry = Some(CompositeMoPrescanTelemetry {
                requested_device_id: cfg.device_id,
                resolved_device_id,
                backend_used: backend_used_summary.unwrap_or_else(|| "Unknown".to_string()),
                backend_route: backend_route_summary,
                backend_fallback_reason: backend_fallback_reason_summary,
                mode: format!("{:?}", cfg.mode),
                require_gpu: cfg.require_gpu,
                runtime_ms: prescan_started.elapsed().as_millis() as u64,
                requested_scan_pairs: scan_pairs.len(),
                candidate_fast_periods: fast_periods.len(),
                candidate_slow_periods: slow_periods.len(),
                candidate_pairs: candidate_pair_total,
                pair_density: pair_density(candidate_pair_total, possible_pair_total),
                type_pairs,
            });
            allowed_pairs_by_type = Some(allowed_pairs_map);
        } else if fast_periods.len() > max_periods || slow_periods.len() > max_periods {
            return Err(
                "gpu_prescan requires fast_len/slow_len to use IntRange domains".to_string(),
            );
        }
    } else if fast_periods.len() > max_periods || slow_periods.len() > max_periods {
        return Err(format!(
            "too many MA periods (fast={}, slow={}); narrow the domain, increase step, or enable gpu_prescan",
            fast_periods.len(),
            slow_periods.len()
        ));
    }

    if fast_periods.len() > max_periods || slow_periods.len() > max_periods {
        return Err(format!(
            "too many MA periods (fast={}, slow={}); narrow the domain, increase step, or tune gpu_prescan",
            fast_periods.len(),
            slow_periods.len()
        ));
    }

    let n = candles.close.len();
    if n < 2 {
        return Err("not enough candles (need at least 2)".to_string());
    }

    let close = vector_ta::utilities::data_loader::source_type(candles, "close");
    let ma_prices = vector_ta::utilities::data_loader::source_type(candles, &req.ma_source);
    if close.len() != ma_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let fv_close =
        first_finite_idx(close).ok_or_else(|| "close series all non-finite".to_string())?;
    let fv_ma =
        first_finite_idx(ma_prices).ok_or_else(|| "ma source series all non-finite".to_string())?;
    let base_first_valid = fv_close.max(fv_ma);
    let lr = compute_log_returns_f64(close);
    emit_progress(progress, 0, 1, "composite_mo_prep");

    // We only need per-bar return stats if any objective uses them.
    let need_sharpe = objective_needs_return_stats(req.objectives.maximize)
        || objective_needs_return_stats(req.objectives.minimize);

    // Precompute MA matrices for the allowed period sets.
    let build_ma_bank = |ma_type: &str,
                         periods: &[u16],
                         ma_params: Option<&HashMap<String, f64>>,
                         alma_offset: f64,
                         alma_sigma: f64|
     -> Result<(Vec<f64>, HashMap<u16, usize>), String> {
        if periods.is_empty() {
            return Err("empty period set".to_string());
        }
        let mut map: HashMap<u16, usize> = HashMap::with_capacity(periods.len());
        for (i, &p) in periods.iter().enumerate() {
            map.insert(p, i);
        }

        let uniform = periods.len() <= 2
            || periods
                .windows(2)
                .map(|w| w[1] as i32 - w[0] as i32)
                .all(|d| d == (periods[1] as i32 - periods[0] as i32));

        let cols = n;
        if uniform {
            let (mat, cols2) = ma_matrix_cpu_f64(
                candles,
                ma_type,
                &req.ma_source,
                periods,
                ma_params,
                alma_offset,
                alma_sigma,
            )?;
            if cols2 != cols {
                return Err("MA matrix column mismatch".to_string());
            }
            return Ok((mat, map));
        }

        let total = periods.len().saturating_mul(cols);
        let mut values = crate::alloc_policy::alloc_f64_with_prefix_uninit_tail(total, 0, 0.0);

        let mut row_offset = 0usize;
        for run in arithmetic_period_runs(periods) {
            let (mat, cols2) = ma_matrix_cpu_f64(
                candles,
                ma_type,
                &req.ma_source,
                run,
                ma_params,
                alma_offset,
                alma_sigma,
            )?;
            if cols2 != cols {
                return Err("MA matrix column mismatch".to_string());
            }
            let rows = run.len();
            let dst = row_offset.saturating_mul(cols);
            let src_len = rows.saturating_mul(cols);
            values[dst..dst + src_len].copy_from_slice(&mat[..src_len]);
            row_offset = row_offset.saturating_add(rows);
        }

        Ok((values, map))
    };

    #[derive(Clone)]
    struct Bank {
        values: Vec<f64>,
        idx_map: HashMap<u16, usize>,
    }

    let max_ma_types = 8usize;

    let mut fast_type_choices: Vec<String> = match idx.fast_ma_type {
        None => vec![base_fast_ma.clone()],
        Some(i) => match req.search_space.params.get(i).map(|p| &p.domain) {
            Some(ParamDomain::Categorical { choices }) => choices
                .iter()
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => {
                return Err("fast_ma_type param must be categorical".to_string());
            }
        },
    };
    if !fast_type_choices.iter().any(|t| t == &base_fast_ma) {
        fast_type_choices.push(base_fast_ma.clone());
    }

    let mut slow_type_choices: Vec<String> = match idx.slow_ma_type {
        None => vec![base_slow_ma.clone()],
        Some(i) => match req.search_space.params.get(i).map(|p| &p.domain) {
            Some(ParamDomain::Categorical { choices }) => choices
                .iter()
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => {
                return Err("slow_ma_type param must be categorical".to_string());
            }
        },
    };
    if !slow_type_choices.iter().any(|t| t == &base_slow_ma) {
        slow_type_choices.push(base_slow_ma.clone());
    }

    fast_type_choices.sort();
    fast_type_choices.dedup();
    slow_type_choices.sort();
    slow_type_choices.dedup();

    if fast_type_choices.is_empty() || slow_type_choices.is_empty() {
        return Err("ma type choices cannot be empty".to_string());
    }
    if fast_type_choices.len() > max_ma_types || slow_type_choices.len() > max_ma_types {
        return Err(format!(
            "too many MA types (fast={}, slow={}); narrow the domain",
            fast_type_choices.len(),
            slow_type_choices.len()
        ));
    }

    for t in &fast_type_choices {
        validate_ma_kind(t)?;
    }
    for t in &slow_type_choices {
        validate_ma_kind(t)?;
    }

    let fast_bank_periods_by_type: HashMap<String, Vec<u16>> =
        if let Some(allowed_pairs) = &allowed_pairs_by_type {
            fast_type_choices
                .iter()
                .filter_map(|t| {
                    let mut periods: BTreeSet<u16> = BTreeSet::new();
                    for ((fast_ma_type, _), pairs) in allowed_pairs {
                        if fast_ma_type == t {
                            for &(fast, _) in pairs {
                                periods.insert(fast);
                            }
                        }
                    }
                    if periods.is_empty() {
                        None
                    } else {
                        Some((t.clone(), periods.into_iter().collect()))
                    }
                })
                .collect()
        } else {
            fast_type_choices
                .iter()
                .map(|t| (t.clone(), fast_periods.clone()))
                .collect()
        };
    let slow_bank_periods_by_type: HashMap<String, Vec<u16>> =
        if let Some(allowed_pairs) = &allowed_pairs_by_type {
            slow_type_choices
                .iter()
                .filter_map(|t| {
                    let mut periods: BTreeSet<u16> = BTreeSet::new();
                    for ((_, slow_ma_type), pairs) in allowed_pairs {
                        if slow_ma_type == t {
                            for &(_, slow) in pairs {
                                periods.insert(slow);
                            }
                        }
                    }
                    if periods.is_empty() {
                        None
                    } else {
                        Some((t.clone(), periods.into_iter().collect()))
                    }
                })
                .collect()
        } else {
            slow_type_choices
                .iter()
                .map(|t| (t.clone(), slow_periods.clone()))
                .collect()
        };

    let mut fast_banks: HashMap<String, Bank> = HashMap::new();
    for t in &fast_type_choices {
        let Some(periods) = fast_bank_periods_by_type.get(t) else {
            continue;
        };
        let (alma_offset, alma_sigma) =
            resolve_alma_params_for_side("fast", t, req.fast_ma_params.as_ref())?;
        let (values, idx_map) = build_ma_bank(
            t,
            periods,
            req.fast_ma_params.as_ref(),
            alma_offset,
            alma_sigma,
        )?;
        fast_banks.insert(t.to_string(), Bank { values, idx_map });
    }

    let mut slow_banks: HashMap<String, Bank> = HashMap::new();
    for t in &slow_type_choices {
        let Some(periods) = slow_bank_periods_by_type.get(t) else {
            continue;
        };
        let (alma_offset, alma_sigma) =
            resolve_alma_params_for_side("slow", t, req.slow_ma_params.as_ref())?;
        let (values, idx_map) = build_ma_bank(
            t,
            periods,
            req.slow_ma_params.as_ref(),
            alma_offset,
            alma_sigma,
        )?;
        slow_banks.insert(t.to_string(), Bank { values, idx_map });
    }

    emit_progress(progress, 1, 1, "composite_mo_prep");

    let constraints_cfg = req.constraints.clone();
    let base_strategy = req.base_strategy.clone();
    let ma_source = req.ma_source.clone();
    let fast_ma_params = req.fast_ma_params.clone();
    let slow_ma_params = req.slow_ma_params.clone();
    let objectives = req.objectives.clone();
    let search_space = req.search_space.clone();
    let allowed_pairs_by_type = allowed_pairs_by_type.map(Arc::new);
    let indicator_cache = Arc::new(Mutex::new(IndicatorSeriesCache::new(8)));
    let dynamic_strategy_enabled =
        crate::strategy_dynamic::dynamic_strategy_opt_experimental_enabled();
    let dynamic_plan_cache = if dynamic_strategy_enabled {
        Some(Arc::new(Mutex::new(DynamicCompositePlanCache::new(512))))
    } else {
        None
    };
    let dynamic_shadow_enabled =
        dynamic_strategy_enabled && crate::strategy_dynamic::dynamic_strategy_opt_shadow_enabled();
    let dynamic_shadow_every = if dynamic_shadow_enabled {
        crate::strategy_dynamic::dynamic_strategy_opt_shadow_every().max(1)
    } else {
        0
    };
    let dynamic_shadow_sampled = AtomicUsize::new(0);
    let dynamic_shadow_mismatches = AtomicUsize::new(0);

    let fast_choice_idx_by_period: Option<HashMap<u16, usize>> =
        fast_choice_map.as_ref().map(|m| {
            m.iter()
                .enumerate()
                .map(|(i, &p)| (p, i))
                .collect::<HashMap<u16, usize>>()
        });
    let slow_choice_idx_by_period: Option<HashMap<u16, usize>> =
        slow_choice_map.as_ref().map(|m| {
            m.iter()
                .enumerate()
                .map(|(i, &p)| (p, i))
                .collect::<HashMap<u16, usize>>()
        });
    let macz_fast_domain = idx
        .macz_fast_length
        .and_then(|i| search_space.params.get(i).map(|p| p.domain.clone()));
    let macz_slow_domain = idx
        .macz_slow_length
        .and_then(|i| search_space.params.get(i).map(|p| p.domain.clone()));
    let macz_fast_choice_map = match macz_fast_domain.as_ref() {
        Some(domain) => numeric_choice_map_u16(domain)?,
        None => None,
    };
    let macz_slow_choice_map = match macz_slow_domain.as_ref() {
        Some(domain) => numeric_choice_map_u16(domain)?,
        None => None,
    };
    let macz_fast_choice_idx_by_period: Option<HashMap<u16, usize>> =
        macz_fast_choice_map.as_ref().map(|m| {
            m.iter()
                .enumerate()
                .map(|(i, &p)| (p, i))
                .collect::<HashMap<u16, usize>>()
        });
    let macz_slow_choice_idx_by_period: Option<HashMap<u16, usize>> =
        macz_slow_choice_map.as_ref().map(|m| {
            m.iter()
                .enumerate()
                .map(|(i, &p)| (p, i))
                .collect::<HashMap<u16, usize>>()
        });
    let _macz_fast_periods = match macz_fast_domain.as_ref() {
        Some(domain) => allowed_periods_from_domain(domain)?,
        None => Vec::new(),
    };
    let _macz_slow_periods = match macz_slow_domain.as_ref() {
        Some(domain) => allowed_periods_from_domain(domain)?,
        None => Vec::new(),
    };

    let feasible_points = AtomicUsize::new(0);
    let rejected_decode_or_order = AtomicUsize::new(0);
    let rejected_prescan_pair = AtomicUsize::new(0);
    let rejected_other_constraints = AtomicUsize::new(0);

    let eval_one = |values: &[ParamValue]| -> EvalOutcome<(Metrics, CompositeMoParams, f64, f64)> {
        // Decode lengths first (and short-circuit invalid combos without computing indicators).
        let mut constraints: Vec<f64> = Vec::new();

        let fast_len = decode_len(
            fast_domain,
            values,
            idx.fast_len,
            fast_choice_map.as_deref(),
        )
        .unwrap_or(0);
        let slow_len = decode_len(
            slow_domain,
            values,
            idx.slow_len,
            slow_choice_map.as_deref(),
        )
        .unwrap_or(0);

        if fast_len == 0 || slow_len == 0 || fast_len >= slow_len {
            rejected_decode_or_order.fetch_add(1, Ordering::Relaxed);
            let viol = if fast_len == 0 || slow_len == 0 {
                1.0
            } else {
                (fast_len as f64) - (slow_len as f64) + 1.0
            };
            constraints.push(viol);
            let metrics = Metrics::default();
            let params = CompositeMoParams {
                fast_ma_type: base_fast_ma.clone(),
                slow_ma_type: base_slow_ma.clone(),
                fast_len,
                slow_len,
                halftrend: default_halftrend(),
                macz: default_macz(),
                ott: default_ott(),
                strategy: base_strategy.clone(),
                signal_mode,
            };
            return EvalOutcome {
                objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                constraints,
                payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
            };
        }
        let fast_ma_type =
            match decode_ma_type(&search_space, values, idx.fast_ma_type, &base_fast_ma) {
                Ok(v) => v,
                Err(_) => {
                    rejected_decode_or_order.fetch_add(1, Ordering::Relaxed);
                    constraints.push(1.0);
                    let metrics = Metrics::default();
                    let params = CompositeMoParams {
                        fast_ma_type: base_fast_ma.clone(),
                        slow_ma_type: base_slow_ma.clone(),
                        fast_len,
                        slow_len,
                        halftrend: default_halftrend(),
                        macz: default_macz(),
                        ott: default_ott(),
                        strategy: base_strategy.clone(),
                        signal_mode,
                    };
                    return EvalOutcome {
                        objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                        constraints,
                        payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                    };
                }
            };
        let slow_ma_type =
            match decode_ma_type(&search_space, values, idx.slow_ma_type, &base_slow_ma) {
                Ok(v) => v,
                Err(_) => {
                    rejected_decode_or_order.fetch_add(1, Ordering::Relaxed);
                    constraints.push(1.0);
                    let metrics = Metrics::default();
                    let params = CompositeMoParams {
                        fast_ma_type: base_fast_ma.clone(),
                        slow_ma_type: base_slow_ma.clone(),
                        fast_len,
                        slow_len,
                        halftrend: default_halftrend(),
                        macz: default_macz(),
                        ott: default_ott(),
                        strategy: base_strategy.clone(),
                        signal_mode,
                    };
                    return EvalOutcome {
                        objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                        constraints,
                        payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                    };
                }
            };

        if let Some(pairs_by_type) = allowed_pairs_by_type.as_ref() {
            match pairs_by_type.get(&(fast_ma_type.clone(), slow_ma_type.clone())) {
                Some(pairs) if pairs.contains(&(fast_len, slow_len)) => {}
                _ => {
                    rejected_prescan_pair.fetch_add(1, Ordering::Relaxed);
                    constraints.push(1.0);
                    let metrics = Metrics::default();
                    let params = CompositeMoParams {
                        fast_ma_type: fast_ma_type.clone(),
                        slow_ma_type: slow_ma_type.clone(),
                        fast_len,
                        slow_len,
                        halftrend: default_halftrend(),
                        macz: default_macz(),
                        ott: default_ott(),
                        strategy: base_strategy.clone(),
                        signal_mode,
                    };
                    return EvalOutcome {
                        objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                        constraints,
                        payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                    };
                }
            }
        }

        let fast_bank = match fast_banks.get(&fast_ma_type) {
            Some(v) => v,
            None => {
                constraints.push(1.0);
                let metrics = Metrics::default();
                let params = CompositeMoParams {
                    fast_ma_type: base_fast_ma.clone(),
                    slow_ma_type: base_slow_ma.clone(),
                    fast_len,
                    slow_len,
                    halftrend: default_halftrend(),
                    macz: default_macz(),
                    ott: default_ott(),
                    strategy: base_strategy.clone(),
                    signal_mode,
                };
                return EvalOutcome {
                    objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                    constraints,
                    payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                };
            }
        };
        let slow_bank = match slow_banks.get(&slow_ma_type) {
            Some(v) => v,
            None => {
                constraints.push(1.0);
                let metrics = Metrics::default();
                let params = CompositeMoParams {
                    fast_ma_type: base_fast_ma.clone(),
                    slow_ma_type: base_slow_ma.clone(),
                    fast_len,
                    slow_len,
                    halftrend: default_halftrend(),
                    macz: default_macz(),
                    ott: default_ott(),
                    strategy: base_strategy.clone(),
                    signal_mode,
                };
                return EvalOutcome {
                    objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                    constraints,
                    payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                };
            }
        };

        let fast_row = match fast_bank.idx_map.get(&fast_len) {
            Some(&row) => {
                let off = row.saturating_mul(n);
                &fast_bank.values[off..off + n]
            }
            None => {
                constraints.push(1.0);
                let metrics = Metrics::default();
                let params = CompositeMoParams {
                    fast_ma_type: base_fast_ma.clone(),
                    slow_ma_type: base_slow_ma.clone(),
                    fast_len,
                    slow_len,
                    halftrend: default_halftrend(),
                    macz: default_macz(),
                    ott: default_ott(),
                    strategy: base_strategy.clone(),
                    signal_mode,
                };
                return EvalOutcome {
                    objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                    constraints,
                    payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                };
            }
        };

        let slow_row = match slow_bank.idx_map.get(&slow_len) {
            Some(&row) => {
                let off = row.saturating_mul(n);
                &slow_bank.values[off..off + n]
            }
            None => {
                constraints.push(1.0);
                let metrics = Metrics::default();
                let params = CompositeMoParams {
                    fast_ma_type: base_fast_ma.clone(),
                    slow_ma_type: base_slow_ma.clone(),
                    fast_len,
                    slow_len,
                    halftrend: default_halftrend(),
                    macz: default_macz(),
                    ott: default_ott(),
                    strategy: base_strategy.clone(),
                    signal_mode,
                };
                return EvalOutcome {
                    objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                    constraints,
                    payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                };
            }
        };

        // HalfTrend (defaults overridden by optional params).
        let mut halftrend = default_halftrend();
        if let Some(i) = idx.halftrend_amplitude {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                halftrend.amplitude = v;
            }
        }
        if let Some(i) = idx.halftrend_channel_deviation {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                halftrend.channel_deviation = v;
            }
        }
        if let Some(i) = idx.halftrend_atr_period {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                halftrend.atr_period = v;
            }
        }
        if halftrend.amplitude == 0
            || halftrend.atr_period == 0
            || !halftrend.channel_deviation.is_finite()
            || halftrend.channel_deviation <= 0.0
        {
            constraints.push(1.0);
        }

        // MACZ (defaults overridden).
        let mut macz = default_macz();
        if let Some(i) = idx.macz_fast_length {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                macz.fast_length = v;
            }
        }
        if let Some(i) = idx.macz_slow_length {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                macz.slow_length = v;
            }
        }
        if let Some(i) = idx.macz_signal_length {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                macz.signal_length = v;
            }
        }
        if let Some(i) = idx.macz_lengthz {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                macz.lengthz = v;
            }
        }
        if let Some(i) = idx.macz_length_stdev {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                macz.length_stdev = v;
            }
        }
        if let Some(i) = idx.macz_a {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                macz.a = v;
            }
        }
        if let Some(i) = idx.macz_b {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                macz.b = v;
            }
        }
        if let Some(i) = idx.macz_gamma {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                macz.gamma = v;
            }
        }
        if let Some(i) = idx.macz_use_lag {
            if let Ok(v) = decode_bool_param(&search_space, values, i) {
                macz.use_lag = v;
            }
        }
        if let Some(i) = idx.macz_use_volume {
            if let Ok(v) = decode_bool_param(&search_space, values, i) {
                macz.use_volume = v;
            }
        }
        if macz.fast_length == 0
            || macz.slow_length == 0
            || macz.signal_length == 0
            || macz.lengthz == 0
            || macz.length_stdev == 0
            || macz.fast_length >= macz.slow_length
            || !macz.a.is_finite()
            || !macz.b.is_finite()
            || !macz.gamma.is_finite()
            || macz.gamma < 0.0
            || macz.gamma >= 1.0
        {
            constraints.push(1.0);
        }

        // OTT (defaults overridden).
        let mut ott = default_ott();
        if let Some(i) = idx.ott_period {
            if let Ok(v) = decode_u16_param(&search_space, values, i) {
                ott.period = v;
            }
        }
        if let Some(i) = idx.ott_percent {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                ott.percent = v;
            }
        }
        if let Some(i) = idx.ott_ma_type {
            if let Some(ParamValue::Cat(ci)) = values.get(i) {
                if let Some(ParamDomain::Categorical { choices }) =
                    search_space.params.get(i).map(|p| &p.domain)
                {
                    if let Some(s) = choices.get(*ci) {
                        ott.ma_type = s.to_string();
                    }
                }
            }
        }
        if ott.period == 0
            || !ott.percent.is_finite()
            || ott.percent <= 0.0
            || ott.ma_type.trim().is_empty()
        {
            constraints.push(1.0);
        }

        // Strategy config (base overridden).
        let mut strategy = base_strategy.clone();
        if let Some(i) = idx.strategy_commission {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                strategy.commission = v;
            }
        }
        if let Some(i) = idx.strategy_eps_rel {
            if let Ok(v) = decode_f64_param(&search_space, values, i) {
                strategy.eps_rel = v;
            }
        }
        if let Some(i) = idx.strategy_long_only {
            if let Ok(v) = decode_bool_param(&search_space, values, i) {
                strategy.long_only = v;
            }
        }
        if let Some(i) = idx.strategy_allow_flip {
            if let Ok(v) = decode_bool_param(&search_space, values, i) {
                strategy.allow_flip = v;
            }
        }
        if validate_strategy_cfg(&strategy).is_err() {
            constraints.push(1.0);
        }

        // If structural constraints already violated, avoid expensive indicator work.
        if constraints.iter().any(|&v| v > 0.0) {
            let metrics = Metrics::default();
            let params = CompositeMoParams {
                fast_ma_type: fast_ma_type.clone(),
                slow_ma_type: slow_ma_type.clone(),
                fast_len,
                slow_len,
                halftrend,
                macz,
                ott,
                strategy,
                signal_mode,
            };
            return EvalOutcome {
                objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                constraints,
                payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
            };
        }

        let indicator_key = IndicatorSeriesKey::from_params(&halftrend, &macz, &ott);
        let cached_bundle = if let Ok(mut cache) = indicator_cache.lock() {
            cache.get(&indicator_key)
        } else {
            None
        };
        let indicator_bundle = if let Some(hit) = cached_bundle {
            hit
        } else {
            let halftrend_trend = match compute_halftrend_trend(candles, &halftrend) {
                Ok(v) => v,
                Err(_) => {
                    constraints.push(1.0);
                    let metrics = Metrics::default();
                    let params = CompositeMoParams {
                        fast_ma_type: fast_ma_type.clone(),
                        slow_ma_type: slow_ma_type.clone(),
                        fast_len,
                        slow_len,
                        halftrend,
                        macz,
                        ott,
                        strategy,
                        signal_mode,
                    };
                    return EvalOutcome {
                        objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                        constraints,
                        payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                    };
                }
            };
            let fv_ht = first_finite_idx(&halftrend_trend).unwrap_or(n);

            let macz_hist =
                match crate::composite_cpu::compute_macz_hist(candles, &ma_source, &macz) {
                    Ok(v) => v,
                    Err(_) => {
                        constraints.push(1.0);
                        let metrics = Metrics::default();
                        let params = CompositeMoParams {
                            fast_ma_type: fast_ma_type.clone(),
                            slow_ma_type: slow_ma_type.clone(),
                            fast_len,
                            slow_len,
                            halftrend,
                            macz,
                            ott,
                            strategy,
                            signal_mode,
                        };
                        return EvalOutcome {
                            objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                            constraints,
                            payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                        };
                    }
                };
            let fv_macz = first_finite_idx(&macz_hist).unwrap_or(n);

            let ott_line = match crate::composite_cpu::compute_ott_line(candles, &ma_source, &ott) {
                Ok(v) => v,
                Err(_) => {
                    constraints.push(1.0);
                    let metrics = Metrics::default();
                    let params = CompositeMoParams {
                        fast_ma_type: fast_ma_type.clone(),
                        slow_ma_type: slow_ma_type.clone(),
                        fast_len,
                        slow_len,
                        halftrend,
                        macz,
                        ott,
                        strategy,
                        signal_mode,
                    };
                    return EvalOutcome {
                        objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                        constraints,
                        payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
                    };
                }
            };
            let fv_ott = first_finite_idx(&ott_line).unwrap_or(n);

            let bundle = Arc::new(IndicatorSeriesBundle {
                halftrend_trend,
                macz_hist,
                ott_line,
                first_valid: base_first_valid.max(fv_ht).max(fv_macz).max(fv_ott),
            });

            if let Ok(mut cache) = indicator_cache.lock() {
                if let Some(existing) = cache.get(&indicator_key) {
                    existing
                } else {
                    cache.insert(indicator_key.clone(), Arc::clone(&bundle))
                }
            } else {
                bundle
            }
        };

        let first_valid = indicator_bundle.first_valid;
        let trade_offset = if strategy.trade_on_next_bar {
            1usize
        } else {
            0usize
        };
        let max_p = (fast_len as usize).max(slow_len as usize).max(1);
        let t_valid = first_valid.saturating_add(max_p.saturating_sub(1));
        let start_t = t_valid.saturating_add(trade_offset);
        if start_t >= n {
            constraints.push(1.0);
        }
        if cancel.load(Ordering::Relaxed) {
            constraints.push(1.0);
        }

        let invalid_outcome = |constraints: Vec<f64>| {
            let metrics = Metrics::default();
            let params = CompositeMoParams {
                fast_ma_type: fast_ma_type.clone(),
                slow_ma_type: slow_ma_type.clone(),
                fast_len,
                slow_len,
                halftrend: halftrend.clone(),
                macz: macz.clone(),
                ott: ott.clone(),
                strategy: strategy.clone(),
                signal_mode,
            };
            EvalOutcome {
                objectives: [f64::NEG_INFINITY, f64::NEG_INFINITY],
                constraints,
                payload: (metrics, params, f64::NEG_INFINITY, f64::NEG_INFINITY),
            }
        };

        let eval_static_metrics = || {
            if need_sharpe {
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                    &lr,
                    ma_prices,
                    fast_row,
                    slow_row,
                    &indicator_bundle.halftrend_trend,
                    &indicator_bundle.macz_hist,
                    &indicator_bundle.ott_line,
                    start_t,
                    &strategy,
                    signal_mode,
                )
            } else {
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_fast_with_mode(
                    &lr,
                    ma_prices,
                    fast_row,
                    slow_row,
                    &indicator_bundle.halftrend_trend,
                    &indicator_bundle.macz_hist,
                    &indicator_bundle.ott_line,
                    start_t,
                    &strategy,
                    signal_mode,
                )
            }
        };

        let metrics = if dynamic_strategy_enabled {
            let plan_key = dynamic_plan_cache_key(
                &fast_ma_type,
                &slow_ma_type,
                &indicator_key,
                &strategy,
                signal_mode,
            );

            let cached_plan = dynamic_plan_cache.as_ref().and_then(|cache| {
                if let Ok(mut cache) = cache.lock() {
                    cache.get(plan_key)
                } else {
                    None
                }
            });
            let compiled_plan = if let Some(plan) = cached_plan {
                plan
            } else {
                let compiled =
                    match crate::strategy_dynamic::compile_dynamic_plan_for_composite_cpu_config(
                        &ma_source,
                        &fast_ma_type,
                        &slow_ma_type,
                        fast_ma_params.as_ref(),
                        slow_ma_params.as_ref(),
                        &strategy,
                        &halftrend,
                        &macz,
                        &ott,
                        signal_mode,
                        crate::strategy_plan::StrategyPlanBackend::Cpu,
                    ) {
                        Ok(v) => v,
                        Err(_) => {
                            constraints.push(1.0);
                            return invalid_outcome(constraints);
                        }
                    };
                let plan = Arc::new(compiled);
                if let Some(cache) = dynamic_plan_cache.as_ref() {
                    if let Ok(mut cache) = cache.lock() {
                        let _ = cache.insert(plan_key, Arc::clone(&plan));
                    }
                }
                plan
            };

            let dynamic_metrics =
                match crate::strategy_dynamic::eval_dynamic_plan_metrics_cpu_precomputed(
                    &compiled_plan,
                    &lr,
                    close,
                    ma_prices,
                    None,
                    Some(&indicator_bundle.halftrend_trend),
                    Some(&indicator_bundle.macz_hist),
                    Some(&indicator_bundle.ott_line),
                    None,
                    fast_row,
                    slow_row,
                    start_t,
                    need_sharpe,
                ) {
                    Ok(v) => v,
                    Err(_) => {
                        constraints.push(1.0);
                        return invalid_outcome(constraints);
                    }
                };

            if dynamic_shadow_enabled {
                let sampled = dynamic_shadow_sampled
                    .fetch_add(1, Ordering::Relaxed)
                    .saturating_add(1);
                if sampled % dynamic_shadow_every == 0 {
                    let static_metrics = eval_static_metrics();
                    if !crate::strategy_dynamic::metrics_shadow_equivalent(
                        &dynamic_metrics,
                        &static_metrics,
                    ) {
                        dynamic_shadow_mismatches.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            dynamic_metrics
        } else {
            eval_static_metrics()
        };

        // User constraints derived from metrics.
        if let Some(cfg) = constraints_cfg.as_ref() {
            if let Some(min_trades) = cfg.min_trades {
                if (metrics.trades as u32) < min_trades {
                    constraints.push((min_trades - metrics.trades as u32) as f64);
                }
            }
            if let Some(max_dd) = cfg.max_drawdown {
                if max_dd.is_finite() && metrics.max_dd.is_finite() && metrics.max_dd > max_dd {
                    constraints.push(metrics.max_dd - max_dd);
                }
            }
            if let Some(max_abs) = cfg.max_abs_exposure {
                let v = metrics.exposure.abs();
                if max_abs.is_finite() && v.is_finite() && v > max_abs {
                    constraints.push(v - max_abs);
                }
            }
        }

        let max_v = metric_value(&metrics, objectives.maximize);
        let min_v = metric_value(&metrics, objectives.minimize);
        let objectives = [max_v, -min_v];

        let params = CompositeMoParams {
            fast_ma_type,
            slow_ma_type,
            fast_len,
            slow_len,
            halftrend,
            macz,
            ott,
            strategy,
            signal_mode,
        };

        if constraints.iter().any(|&value| value > 0.0) {
            rejected_other_constraints.fetch_add(1, Ordering::Relaxed);
        } else {
            feasible_points.fetch_add(1, Ordering::Relaxed);
        }

        EvalOutcome {
            objectives,
            constraints,
            payload: (metrics, params, max_v, min_v),
        }
    };

    let repair = |values: &mut [ParamValue]| {
        let fast_ma_type = decode_ma_type(&search_space, values, idx.fast_ma_type, &base_fast_ma)
            .unwrap_or_else(|_| base_fast_ma.clone());
        let slow_ma_type = decode_ma_type(&search_space, values, idx.slow_ma_type, &base_slow_ma)
            .unwrap_or_else(|_| base_slow_ma.clone());
        let fast_len = decode_len(
            fast_domain,
            values,
            idx.fast_len,
            fast_choice_map.as_deref(),
        )
        .unwrap_or(0);
        let slow_len = decode_len(
            slow_domain,
            values,
            idx.slow_len,
            slow_choice_map.as_deref(),
        )
        .unwrap_or(0);
        let target_fast = fast_len.max(1);
        let target_slow = slow_len.max(1);

        if let Some(type_pairs) = allowed_pairs_by_type
            .as_ref()
            .and_then(|pairs| pairs.get(&(fast_ma_type.clone(), slow_ma_type.clone())))
        {
            if (fast_len == 0 || slow_len == 0 || fast_len >= slow_len)
                || !type_pairs.contains(&(fast_len, slow_len))
            {
                if let Some((new_fast, new_slow)) =
                    nearest_allowed_pair(type_pairs, target_fast, target_slow)
                {
                    set_len_value(
                        values,
                        idx.fast_len,
                        fast_domain,
                        fast_choice_idx_by_period.as_ref(),
                        new_fast,
                    );
                    set_len_value(
                        values,
                        idx.slow_len,
                        slow_domain,
                        slow_choice_idx_by_period.as_ref(),
                        new_slow,
                    );
                }
            }
        } else if fast_len == 0 || slow_len == 0 || fast_len >= slow_len {
            let max_less_than = |sorted: &[u16], x: u16| -> Option<u16> {
                let i = sorted.partition_point(|&v| v < x);
                if i == 0 {
                    None
                } else {
                    Some(sorted[i - 1])
                }
            };
            let min_greater_than = |sorted: &[u16], x: u16| -> Option<u16> {
                let i = sorted.partition_point(|&v| v <= x);
                sorted.get(i).copied()
            };

            if let Some(new_fast) = max_less_than(&fast_periods, target_slow) {
                set_len_value(
                    values,
                    idx.fast_len,
                    fast_domain,
                    fast_choice_idx_by_period.as_ref(),
                    new_fast,
                );
                let fast2 = decode_len(
                    fast_domain,
                    values,
                    idx.fast_len,
                    fast_choice_map.as_deref(),
                )
                .unwrap_or(0);
                let slow2 = decode_len(
                    slow_domain,
                    values,
                    idx.slow_len,
                    slow_choice_map.as_deref(),
                )
                .unwrap_or(0);
                if fast2 != 0 && slow2 != 0 && fast2 < slow2 {
                    // repaired
                } else if let Some(new_slow) = min_greater_than(&slow_periods, target_fast) {
                    set_len_value(
                        values,
                        idx.slow_len,
                        slow_domain,
                        slow_choice_idx_by_period.as_ref(),
                        new_slow,
                    );
                } else if let (Some(&min_fast), Some(&max_slow)) =
                    (fast_periods.first(), slow_periods.last())
                {
                    if min_fast < max_slow {
                        set_len_value(
                            values,
                            idx.fast_len,
                            fast_domain,
                            fast_choice_idx_by_period.as_ref(),
                            min_fast,
                        );
                        set_len_value(
                            values,
                            idx.slow_len,
                            slow_domain,
                            slow_choice_idx_by_period.as_ref(),
                            max_slow,
                        );
                    }
                }
            }
        }

        if let (Some(fast_idx), Some(slow_idx)) = (idx.macz_fast_length, idx.macz_slow_length) {
            let macz_fast = decode_u16_param(&search_space, values, fast_idx).unwrap_or(0);
            let macz_slow = decode_u16_param(&search_space, values, slow_idx).unwrap_or(0);
            if macz_fast != 0 && macz_slow != 0 && macz_fast >= macz_slow {
                if let Some(param) = search_space.params.get(fast_idx) {
                    match &param.domain {
                        ParamDomain::Categorical { .. } | ParamDomain::IntRange { .. } => {
                            let repaired_fast = macz_fast_choice_idx_by_period
                                .as_ref()
                                .and_then(|choices| {
                                    choices
                                        .keys()
                                        .copied()
                                        .filter(|candidate| *candidate < macz_slow)
                                        .max()
                                })
                                .unwrap_or_else(|| macz_slow.saturating_sub(1).max(1));
                            set_u16_param_value(
                                values,
                                fast_idx,
                                &search_space,
                                macz_fast_choice_idx_by_period.as_ref(),
                                repaired_fast,
                            );
                        }
                        _ => {}
                    }
                }

                let repaired_fast = decode_u16_param(&search_space, values, fast_idx).unwrap_or(0);
                if repaired_fast == 0 || repaired_fast >= macz_slow {
                    if let Some(param) = search_space.params.get(slow_idx) {
                        match &param.domain {
                            ParamDomain::Categorical { .. } | ParamDomain::IntRange { .. } => {
                                let repaired_slow = macz_slow_choice_idx_by_period
                                    .as_ref()
                                    .and_then(|choices| {
                                        choices
                                            .keys()
                                            .copied()
                                            .filter(|candidate| *candidate > repaired_fast.max(1))
                                            .min()
                                    })
                                    .unwrap_or_else(|| repaired_fast.saturating_add(1).max(2));
                                set_u16_param_value(
                                    values,
                                    slow_idx,
                                    &search_space,
                                    macz_slow_choice_idx_by_period.as_ref(),
                                    repaired_slow,
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    };

    let requested_eval_budget = match req.sampler.clone() {
        CompositeMoSampler::Nsga2 { config } => config
            .population_size
            .saturating_mul(config.generations.saturating_add(1)),
        CompositeMoSampler::Random { config } => config.trials,
        CompositeMoSampler::Tpe { config } => config.trials,
        CompositeMoSampler::Qmc { config } => config.trials,
    };
    enum ExactGridPlan {
        FullSpace {
            total_combinations: usize,
        },
        ExplicitCandidates {
            total_combinations: usize,
            candidates: Vec<Vec<ParamValue>>,
        },
    }

    let max_exact_grid_combos = EXACT_GRID_MAX_COMBOS.min(requested_eval_budget.max(1));
    let prescan_exact_grid_plan = if let Some(pairs_by_type) =
        allowed_pairs_by_type.as_ref().map(|pairs| &**pairs)
    {
        let mut reduced_params = Vec::with_capacity(search_space.params.len().saturating_sub(2));
        let mut reduced_to_original: Vec<usize> =
            Vec::with_capacity(search_space.params.len().saturating_sub(2));
        for (original_idx, param) in search_space.params.iter().cloned().enumerate() {
            if original_idx == idx.fast_len || original_idx == idx.slow_len {
                continue;
            }
            reduced_params.push(param);
            reduced_to_original.push(original_idx);
        }

        let base_values_grid: Option<Vec<Vec<ParamValue>>> = if reduced_params.is_empty() {
            Some(vec![Vec::new()])
        } else {
            ta_optimizer::mo::bounded_exact_grid(
                &SearchSpace {
                    params: reduced_params,
                },
                EXACT_GRID_MAX_AXIS_VALUES,
                max_exact_grid_combos,
            )?
            .map(|grid| grid.iter().collect())
        };

        if let Some(base_values_grid) = base_values_grid {
            let mut candidates: Vec<Vec<ParamValue>> = Vec::new();
            let mut overflow = false;

            for base_values in base_values_grid {
                let mut template = vec![ParamValue::Int(0); search_space.params.len()];
                for (reduced_idx, original_idx) in reduced_to_original.iter().copied().enumerate() {
                    if let Some(value) = base_values.get(reduced_idx) {
                        template[original_idx] = value.clone();
                    }
                }

                let fast_ma_type =
                    decode_ma_type(&search_space, &template, idx.fast_ma_type, &base_fast_ma)?;
                let slow_ma_type =
                    decode_ma_type(&search_space, &template, idx.slow_ma_type, &base_slow_ma)?;
                let Some(allowed_pairs) =
                    pairs_by_type.get(&(fast_ma_type.clone(), slow_ma_type.clone()))
                else {
                    continue;
                };

                let mut sorted_pairs: Vec<(u16, u16)> = allowed_pairs.iter().copied().collect();
                sorted_pairs.sort_unstable();
                for (fast_len, slow_len) in sorted_pairs {
                    let mut candidate = template.clone();
                    set_len_value(
                        &mut candidate,
                        idx.fast_len,
                        fast_domain,
                        fast_choice_idx_by_period.as_ref(),
                        fast_len,
                    );
                    set_len_value(
                        &mut candidate,
                        idx.slow_len,
                        slow_domain,
                        slow_choice_idx_by_period.as_ref(),
                        slow_len,
                    );
                    candidates.push(candidate);
                    if candidates.len() > max_exact_grid_combos {
                        overflow = true;
                        break;
                    }
                }
                if overflow {
                    break;
                }
            }

            if !overflow && !candidates.is_empty() {
                Some(ExactGridPlan::ExplicitCandidates {
                    total_combinations: candidates.len(),
                    candidates,
                })
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let full_space_exact_grid_total = req
        .search_space
        .grid(EXACT_GRID_MAX_AXIS_VALUES, max_exact_grid_combos)
        .ok()
        .map(|grid| grid.len());
    let exact_grid_plan = prescan_exact_grid_plan.or_else(|| {
        full_space_exact_grid_total
            .map(|total_combinations| ExactGridPlan::FullSpace { total_combinations })
    });
    let exact_grid_total_combinations = exact_grid_plan.as_ref().map(|plan| match plan {
        ExactGridPlan::FullSpace { total_combinations }
        | ExactGridPlan::ExplicitCandidates {
            total_combinations, ..
        } => *total_combinations,
    });
    let exact_grid_used = exact_grid_plan.is_some();
    let (progress_phase, progress_total) = if let Some(total) = exact_grid_total_combinations {
        ("composite_mo_exact_grid", total)
    } else {
        match req.sampler.clone() {
            CompositeMoSampler::Nsga2 { config } => (
                "composite_mo_nsga2",
                config
                    .population_size
                    .saturating_mul(config.generations.saturating_add(1)),
            ),
            CompositeMoSampler::Random { config } => ("composite_mo_random", config.trials),
            CompositeMoSampler::Tpe { config } => ("composite_mo_tpe", config.trials),
            CompositeMoSampler::Qmc { config } => ("composite_mo_qmc", config.trials),
        }
    };

    let done_counter = AtomicUsize::new(0);
    let last_emit_ms = AtomicU64::new(0);
    emit_progress(progress, 0, progress_total, progress_phase);

    let eval_with_progress =
        |values: &[ParamValue]| -> EvalOutcome<(Metrics, CompositeMoParams, f64, f64)> {
            let out = eval_one(values);
            let done = done_counter
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            maybe_emit_progress_throttled(
                progress,
                done,
                progress_total,
                progress_phase,
                start,
                &last_emit_ms,
            );
            out
        };

    let (pareto_points, search_stats, sampler_label) = if let Some(plan) = exact_grid_plan {
        let res = match plan {
            ExactGridPlan::FullSpace { .. } => ta_optimizer::mo::exact_grid_mo_search_with_stats(
                &req.search_space,
                EXACT_GRID_MAX_AXIS_VALUES,
                max_exact_grid_combos,
                256,
                repair,
                eval_with_progress,
            )?,
            ExactGridPlan::ExplicitCandidates { candidates, .. } => {
                ta_optimizer::mo::candidate_set_mo_search_with_stats(
                    candidates,
                    256,
                    repair,
                    eval_with_progress,
                )?
            }
        };
        (res.pareto, res.stats, "ExactGrid".to_string())
    } else {
        match req.sampler.clone() {
            CompositeMoSampler::Nsga2 { config } => {
                let res = ta_optimizer::mo::nsga2_optimize(
                    &req.search_space,
                    config.clone(),
                    repair,
                    eval_with_progress,
                )?;
                (
                    res.pareto,
                    ta_optimizer::mo::MoSearchStats {
                        requested_evaluations: config
                            .population_size
                            .saturating_mul(config.generations.saturating_add(1)),
                        actual_evaluations: res.total_evaluations,
                        unique_evaluations: res.unique_evaluations,
                        duplicate_evaluations_avoided: res.duplicate_evaluations_avoided,
                        exact_grid_used: false,
                    },
                    "Nsga2".to_string(),
                )
            }
            CompositeMoSampler::Random { config } => {
                let res = ta_optimizer::mo::random_mo_search_with_stats(
                    &req.search_space,
                    config,
                    repair,
                    eval_with_progress,
                )?;
                (res.pareto, res.stats, "Random".to_string())
            }
            CompositeMoSampler::Tpe { config } => {
                let res = ta_optimizer::mo::tpe_mo_search_with_stats(
                    &req.search_space,
                    config,
                    repair,
                    eval_with_progress,
                )?;
                (res.pareto, res.stats, "Tpe".to_string())
            }
            CompositeMoSampler::Qmc { config } => {
                let res = ta_optimizer::mo::qmc_mo_search_with_stats(
                    &req.search_space,
                    config,
                    repair,
                    eval_with_progress,
                )?;
                (res.pareto, res.stats, "Qmc".to_string())
            }
        }
    };

    emit_progress(progress, progress_total, progress_total, progress_phase);

    let mut pareto = pareto_points
        .into_iter()
        .map(|p| {
            let (metrics, params, max_v, min_v) = p.payload;
            CompositeMoPoint {
                params,
                metrics,
                maximize_value: max_v,
                minimize_value: min_v,
                violation: p.violation,
            }
        })
        .collect::<Vec<_>>();

    for point in pareto.iter_mut() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }
        let trade22_req = composite_mo_trade22_request(&req, &point.params);
        let trade22_params = composite_mo_trade22_params(&point.params);
        point.metrics =
            evaluate_composite_cpu_trade22_metrics(&trade22_req, candles, &trade22_params)?;
    }

    let runtime_ms = start.elapsed().as_millis() as u64;
    let actual_evaluations = done_counter.load(Ordering::Relaxed);
    let telemetry = CompositeMoTelemetry {
        prescan: prescan_telemetry.clone(),
        search: CompositeMoSearchTelemetry {
            sampler: sampler_label,
            requested_evaluations: if exact_grid_used {
                requested_eval_budget
            } else {
                search_stats.requested_evaluations
            },
            actual_evaluations,
            unique_evaluations: search_stats.unique_evaluations,
            duplicate_evaluations_avoided: search_stats.duplicate_evaluations_avoided,
            feasible_points: feasible_points.load(Ordering::Relaxed),
            rejected_decode_or_order: rejected_decode_or_order.load(Ordering::Relaxed),
            rejected_prescan_pair: rejected_prescan_pair.load(Ordering::Relaxed),
            rejected_other_constraints: rejected_other_constraints.load(Ordering::Relaxed),
            exact_grid_used: search_stats.exact_grid_used,
            exact_grid_total_combinations,
        },
    };
    let shadow_sampled = if dynamic_shadow_enabled {
        dynamic_shadow_sampled.load(Ordering::Relaxed)
    } else {
        0
    };
    let shadow_mismatches = if dynamic_shadow_enabled {
        dynamic_shadow_mismatches.load(Ordering::Relaxed)
    } else {
        0
    };
    if dynamic_shadow_enabled && shadow_mismatches > 0 {
        eprintln!(
            "[dynamic-shadow][composite_mo] sampled={} mismatches={} every={}",
            shadow_sampled, shadow_mismatches, dynamic_shadow_every
        );
    }
    let artifact_backend = resolve_dynamic_artifact_backend(prescan_telemetry.as_ref());
    crate::strategy_dynamic::maybe_append_dynamic_strategy_artifact(
        crate::strategy_dynamic::DynamicStrategyArtifactRecord {
            scope: "composite_mo",
            profile: "CompositeMo",
            backend: &artifact_backend,
            dynamic_enabled: dynamic_strategy_enabled,
            shadow_enabled: dynamic_shadow_enabled,
            shadow_every: dynamic_shadow_every,
            shadow_sampled,
            shadow_mismatches,
            runtime_ms,
            work_items: actual_evaluations,
        },
    );
    Ok(CompositeMoResult {
        data_id: req.data_id,
        pareto,
        metric_surface_kind: Some("trade22".to_string()),
        total_evaluations: actual_evaluations,
        runtime_ms,
        telemetry,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEnvGuard {
        _env: std::sync::MutexGuard<'static, ()>,
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        _gpu: crate::gpu_backtest_kernel::GpuTestLockGuard,
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn lock_env() -> TestEnvGuard {
        let env = crate::test_support::lock_test_env();
        let gpu = crate::gpu_backtest_kernel::lock_gpu_test();
        crate::gpu_backtest_kernel::reset_test_runtime();
        crate::dynamic_indicator_adapter::reset_test_cuda_input_cache();
        TestEnvGuard {
            _env: env,
            _gpu: gpu,
        }
    }

    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    fn lock_env() -> TestEnvGuard {
        let env = crate::test_support::lock_test_env();
        TestEnvGuard { _env: env }
    }

    fn with_dynamic_strategy_opt_env<T>(enabled: bool, f: impl FnOnce() -> T) -> T {
        let _guard = lock_env();
        let key = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let prev = std::env::var(key).ok();
        if enabled {
            std::env::set_var(key, "1");
        } else {
            std::env::remove_var(key);
        }
        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        match out {
            Ok(v) => v,
            Err(panic_payload) => std::panic::resume_unwind(panic_payload),
        }
    }

    fn with_dynamic_strategy_shadow_env<T>(every: usize, f: impl FnOnce() -> T) -> T {
        let _guard = lock_env();
        let key_exp = "VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL";
        let key_shadow = "VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW";
        let key_every = "VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW_EVERY";
        let prev_exp = std::env::var(key_exp).ok();
        let prev_shadow = std::env::var(key_shadow).ok();
        let prev_every = std::env::var(key_every).ok();
        std::env::set_var(key_exp, "1");
        std::env::set_var(key_shadow, "1");
        std::env::set_var(key_every, every.max(1).to_string());

        let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        match prev_exp {
            Some(v) => std::env::set_var(key_exp, v),
            None => std::env::remove_var(key_exp),
        }
        match prev_shadow {
            Some(v) => std::env::set_var(key_shadow, v),
            None => std::env::remove_var(key_shadow),
        }
        match prev_every {
            Some(v) => std::env::set_var(key_every, v),
            None => std::env::remove_var(key_every),
        }

        match out {
            Ok(v) => v,
            Err(panic_payload) => std::panic::resume_unwind(panic_payload),
        }
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9
    }

    fn make_trending_candles(len: usize) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    #[test]
    fn composite_mo_smoke_random_returns_pareto() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 7,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 12,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 123,
                    trials: 50,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 9);
        assert!(!res.pareto.is_empty());
        assert_eq!(res.telemetry.search.requested_evaluations, 50);
        assert_eq!(res.telemetry.search.actual_evaluations, 9);
        assert!(res.telemetry.search.exact_grid_used);
        assert_eq!(res.telemetry.search.exact_grid_total_combinations, Some(9));
        assert!(res
            .pareto
            .iter()
            .any(|p| p.params.fast_len < p.params.slow_len));
    }

    #[test]
    fn composite_mo_fast_objectives_emit_trade22_metric_surface() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 7,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 12,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Pnl,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 321,
                    trials: 50,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(
            req.clone(),
            &candles,
            &cancel,
            None,
        )
        .expect("composite mo trade22 run");

        assert_eq!(res.metric_surface_kind.as_deref(), Some("trade22"));
        let point = res.pareto.first().expect("pareto point");
        let expected = evaluate_composite_cpu_trade22_metrics(
            &composite_mo_trade22_request(&req, &point.params),
            &candles,
            &composite_mo_trade22_params(&point.params),
        )
        .expect("expected trade22 metrics");

        assert!(point.metrics.downside_frequency.is_finite());
        assert!(point.metrics.underwater_fraction.is_finite());
        assert!(point.metrics.gross_profit >= 0.0);
        assert!(point.metrics.gross_loss >= 0.0);
        assert_eq!(point.metrics.closed_trades, expected.closed_trades);
        assert_eq!(point.metrics.winning_trades, expected.winning_trades);
        assert_eq!(point.metrics.losing_trades, expected.losing_trades);
        assert!((point.metrics.gross_profit - expected.gross_profit).abs() <= 1.0e-9);
        assert!((point.metrics.gross_loss - expected.gross_loss).abs() <= 1.0e-9);
    }

    #[test]
    fn composite_mo_dynamic_strategy_env_path_matches_legacy_results() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 8,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 14,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 999,
                    trials: 40,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_composite_mo_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("legacy composite mo run")
        });
        let dynamic = with_dynamic_strategy_opt_env(true, || {
            run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic composite mo run")
        });

        assert_eq!(legacy.total_evaluations, dynamic.total_evaluations);
        assert_eq!(legacy.pareto.len(), dynamic.pareto.len());

        let mut legacy_points = legacy.pareto.clone();
        let mut dynamic_points = dynamic.pareto.clone();
        let sort_key = |p: &CompositeMoPoint| {
            (
                p.params.fast_ma_type.clone(),
                p.params.slow_ma_type.clone(),
                p.params.fast_len,
                p.params.slow_len,
            )
        };
        legacy_points.sort_by_key(sort_key);
        dynamic_points.sort_by_key(sort_key);

        for (a, b) in legacy_points.iter().zip(dynamic_points.iter()) {
            assert_eq!(sort_key(a), sort_key(b));
            assert!(approx_eq(a.maximize_value, b.maximize_value));
            assert!(approx_eq(a.minimize_value, b.minimize_value));
            assert!(approx_eq(a.metrics.pnl, b.metrics.pnl));
            assert!(approx_eq(a.metrics.sharpe, b.metrics.sharpe));
            assert!(approx_eq(a.metrics.max_dd, b.metrics.max_dd));
            assert_eq!(a.metrics.trades, b.metrics.trades);
        }
    }

    #[test]
    fn composite_mo_dynamic_strategy_shadow_mode_matches_legacy_results() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 8,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 14,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 999,
                    trials: 40,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let legacy = with_dynamic_strategy_opt_env(false, || {
            run_composite_mo_optimization_blocking_with_candles(
                req.clone(),
                &candles,
                &cancel,
                None,
            )
            .expect("legacy composite mo run")
        });
        let dynamic = with_dynamic_strategy_shadow_env(1, || {
            run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
                .expect("dynamic shadow composite mo run")
        });

        assert_eq!(legacy.total_evaluations, dynamic.total_evaluations);
        assert_eq!(legacy.pareto.len(), dynamic.pareto.len());
    }

    #[test]
    fn composite_mo_supports_unsorted_categorical_len_domains() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["7".to_string(), "5".to_string(), "6".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["12".to_string(), "10".to_string(), "11".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 7,
                    trials: 30,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");
        assert_eq!(res.total_evaluations, 9);
        assert!(!res.pareto.is_empty());
        assert_eq!(res.telemetry.search.requested_evaluations, 30);
        assert_eq!(res.telemetry.search.actual_evaluations, 9);
        assert!(res.telemetry.search.exact_grid_used);
    }

    #[test]
    fn composite_mo_rejects_unknown_search_space_param() {
        let candles = make_trending_candles(200);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 6,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 11,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "not_supported".to_string(),
                        domain: ParamDomain::Bool,
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig { seed: 1, trials: 4 },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let err = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect_err("unknown params should be rejected");
        assert!(err.contains("unsupported CompositeMo search-space param 'not_supported'"));
    }

    #[test]
    fn composite_mo_decodes_categorical_numeric_and_bool_params() {
        let candles = make_trending_candles(500);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["5".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["10".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "halftrend_channel_deviation".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["1.25".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_period".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["6".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_percent".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["1.2".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["ema".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "macz_gamma".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["0.1".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "macz_use_lag".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["true".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "strategy_long_only".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["false".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig { seed: 7, trials: 6 },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("categorical decode run");

        assert_eq!(res.total_evaluations, 1);
        assert_eq!(res.telemetry.search.requested_evaluations, 6);
        assert_eq!(res.telemetry.search.actual_evaluations, 1);
        assert!(res.telemetry.search.exact_grid_used);
        assert!(!res.pareto.is_empty());
        for point in &res.pareto {
            assert!((point.params.halftrend.channel_deviation - 1.25).abs() < 1e-12);
            assert_eq!(point.params.ott.period, 6);
            assert!((point.params.ott.percent - 1.2).abs() < 1e-12);
            assert_eq!(point.params.ott.ma_type.to_ascii_lowercase(), "ema");
            assert!((point.params.macz.gamma - 0.1).abs() < 1e-12);
            assert!(point.params.macz.use_lag);
            assert!(!point.params.strategy.long_only);
        }
    }

    #[test]
    fn composite_mo_repairs_macz_order_before_exact_grid_eval() {
        let candles = make_trending_candles(500);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["5".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["10".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "macz_fast_length".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["12".to_string(), "14".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "macz_slow_length".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["12".to_string(), "15".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig { seed: 9, trials: 8 },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("macz repair run");

        assert!(res.telemetry.search.exact_grid_used);
        assert!(res.total_evaluations <= 4);
        for point in &res.pareto {
            assert!(point.params.macz.fast_length < point.params.macz.slow_length);
        }
    }

    #[test]
    fn composite_mo_pair_aware_repair_chooses_nearest_allowed_pair() {
        let allowed = HashSet::from([(5u16, 20u16), (8u16, 24u16), (13u16, 34u16)]);

        assert_eq!(nearest_allowed_pair(&allowed, 7, 22), Some((8, 24)));
        assert_eq!(nearest_allowed_pair(&allowed, 12, 33), Some((13, 34)));
    }

    #[test]
    fn composite_mo_smoke_qmc_sobol_returns_pareto() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 8,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 14,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Qmc {
                config: QmcMoConfig {
                    seed: 7,
                    trials: 40,
                    sequence: ta_optimizer::mo::QmcSequence::Sobol,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 20);
        assert!(!res.pareto.is_empty());
        assert_eq!(res.telemetry.search.requested_evaluations, 40);
        assert_eq!(res.telemetry.search.actual_evaluations, 20);
        assert!(res.telemetry.search.exact_grid_used);
    }

    #[test]
    fn composite_mo_smoke_tpe_returns_pareto() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 8,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 14,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "ott_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Tpe {
                config: TpeMoConfig {
                    seed: 3,
                    trials: 64,
                    startup_trials: 10,
                    n_ei_candidates: 24,
                    gamma: 0.25,
                    batch_size: 16,
                    max_model_points: 256,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 20);
        assert!(!res.pareto.is_empty());
        assert_eq!(res.telemetry.search.requested_evaluations, 64);
        assert_eq!(res.telemetry.search.actual_evaluations, 20);
        assert!(res.telemetry.search.exact_grid_used);
    }

    #[test]
    fn composite_mo_can_optimize_ma_types() {
        let candles = make_trending_candles(2000);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string(), "ema".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_ma_type".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["sma".to_string(), "ema".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 7,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 12,
                            step: 1,
                            log: false,
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 11,
                    trials: 40,
                },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 36);
        assert!(!res.pareto.is_empty());
        assert_eq!(res.telemetry.search.requested_evaluations, 40);
        assert_eq!(res.telemetry.search.actual_evaluations, 36);
        assert!(res.telemetry.search.exact_grid_used);
        for p in &res.pareto {
            assert!(matches!(p.params.fast_ma_type.as_str(), "sma" | "ema"));
            assert!(matches!(p.params.slow_ma_type.as_str(), "sma" | "ema"));
        }
    }

    #[test]
    fn composite_mo_gpu_prescan_narrows_large_len_ranges() {
        // Force the prescan path by exceeding the max cached MA periods.
        let candles = make_trending_candles(800);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 270,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 300,
                            step: 1,
                            log: false,
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 1,
                    trials: 20,
                },
            },
            constraints: None,
            gpu_prescan: Some(CompositeGpuPrescanConfig {
                top_k: 8,
                window: 2,
                predictive_top_k: 0,
                predictive_stride: 8,
                device_id: Some(7),
                require_gpu: false,
                objectives: CompositeGpuPrescanObjectives::Both,
                ma_pairs: None,
                mode: OptimizationMode::Grid,
            }),
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 16);
        assert!(!res.pareto.is_empty());
        assert_eq!(res.telemetry.search.requested_evaluations, 20);
        assert_eq!(res.telemetry.search.actual_evaluations, 16);
        assert_eq!(res.telemetry.search.rejected_prescan_pair, 0);
        let prescan = res.telemetry.prescan.expect("prescan telemetry");
        assert_eq!(prescan.requested_device_id, Some(7));
        assert_eq!(prescan.resolved_device_id, Some(7));
        assert!(prescan.candidate_pairs > 0);
        assert!(prescan.pair_density > 0.0);
    }

    #[test]
    fn composite_mo_gpu_prescan_uses_exact_grid_for_small_candidate_sets() {
        let candles = make_trending_candles(800);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 270,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 300,
                            step: 1,
                            log: false,
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig { seed: 3, trials: 8 },
            },
            constraints: None,
            gpu_prescan: Some(CompositeGpuPrescanConfig {
                top_k: 1,
                window: 0,
                predictive_top_k: 0,
                predictive_stride: 8,
                device_id: Some(5),
                require_gpu: false,
                objectives: CompositeGpuPrescanObjectives::Both,
                ma_pairs: Some(vec![CompositePrescanMaPair {
                    fast_ma_type: "sma".to_string(),
                    slow_ma_type: "sma".to_string(),
                }]),
                mode: OptimizationMode::Grid,
            }),
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert!(res.total_evaluations >= 1);
        assert!(res.total_evaluations <= 8);
        assert_eq!(res.telemetry.search.requested_evaluations, 8);
        assert_eq!(
            res.telemetry.search.actual_evaluations,
            res.total_evaluations
        );
        assert!(res.telemetry.search.exact_grid_used);
        assert_eq!(
            res.telemetry.search.exact_grid_total_combinations,
            Some(res.total_evaluations)
        );
        let prescan = res.telemetry.prescan.expect("prescan telemetry");
        assert_eq!(prescan.requested_device_id, Some(5));
        assert_eq!(prescan.resolved_device_id, Some(5));
        assert_eq!(prescan.candidate_pairs, res.total_evaluations);
        assert!(prescan.pair_density > 0.0);
    }

    #[test]
    fn composite_mo_gpu_prescan_supports_multiple_ma_pairs() {
        let candles = make_trending_candles(800);
        let cancel = AtomicBool::new(false);

        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 80,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 100,
                            high: 180,
                            step: 1,
                            log: false,
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig {
                    seed: 2,
                    trials: 10,
                },
            },
            constraints: None,
            gpu_prescan: Some(CompositeGpuPrescanConfig {
                top_k: 4,
                window: 2,
                predictive_top_k: 4,
                predictive_stride: 8,
                device_id: None,
                require_gpu: false,
                objectives: CompositeGpuPrescanObjectives::Both,
                ma_pairs: Some(vec![
                    CompositePrescanMaPair {
                        fast_ma_type: "sma".to_string(),
                        slow_ma_type: "sma".to_string(),
                    },
                    CompositePrescanMaPair {
                        fast_ma_type: "ema".to_string(),
                        slow_ma_type: "ema".to_string(),
                    },
                ]),
                mode: OptimizationMode::Grid,
            }),
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 10);
        assert!(!res.pareto.is_empty());
        let prescan = res.telemetry.prescan.expect("prescan telemetry");
        assert_eq!(prescan.type_pairs.len(), 2);
        assert!(prescan
            .type_pairs
            .iter()
            .any(|row| row.fast_ma_type == "sma"));
        assert!(prescan
            .type_pairs
            .iter()
            .any(|row| row.fast_ma_type == "ema"));
    }

    #[test]
    fn composite_mo_rejects_unknown_search_space_params() {
        let candles = make_trending_candles(128);
        let cancel = AtomicBool::new(false);
        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 5,
                            high: 6,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::IntRange {
                            low: 10,
                            high: 11,
                            step: 1,
                            log: false,
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "not_supported".to_string(),
                        domain: ParamDomain::Bool,
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig { seed: 1, trials: 4 },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let err = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .unwrap_err();
        assert!(err.contains("unsupported CompositeMo search-space param 'not_supported'"));
    }

    #[test]
    fn composite_mo_decodes_stringified_categorical_numeric_and_bool_params() {
        let candles = make_trending_candles(512);
        let cancel = AtomicBool::new(false);
        let req = CompositeMoRequest {
            data_id: "test".to_string(),
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            base_strategy: StrategyConfig::default(),
            signal_mode: CompositeSignalMode::HardGate,
            search_space: SearchSpace {
                params: vec![
                    ta_optimizer::mo::ParamDef {
                        name: "fast_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["5".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "slow_len".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["20".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "halftrend_channel_deviation".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["1.5".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "macz_use_lag".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["true".to_string()],
                        },
                    },
                    ta_optimizer::mo::ParamDef {
                        name: "strategy_allow_flip".to_string(),
                        domain: ParamDomain::Categorical {
                            choices: vec!["false".to_string()],
                        },
                    },
                ],
            },
            objectives: CompositeMoObjectives {
                maximize: ObjectiveKind::Sharpe,
                minimize: ObjectiveKind::MaxDrawdown,
            },
            sampler: CompositeMoSampler::Random {
                config: RandomMoConfig { seed: 1, trials: 8 },
            },
            constraints: None,
            gpu_prescan: None,
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("categorical numeric/bool decode run");
        let point = res.pareto.first().expect("pareto point");
        assert!(approx_eq(point.params.halftrend.channel_deviation, 1.5));
        assert!(point.params.macz.use_lag);
        assert!(!point.params.strategy.allow_flip);
        assert_eq!(res.total_evaluations, 1);
        assert_eq!(res.telemetry.search.requested_evaluations, 8);
        assert_eq!(res.telemetry.search.actual_evaluations, 1);
    }
}
