use crate::composite_cpu::{MaczGateParams, OttGateParams};
use crate::double_ma::{
    compute_halftrend_trend, compute_log_returns_f64, first_finite_idx, ma_matrix_cpu_f64,
    resolve_alma_params_for_side, run_double_ma_optimization_blocking_with_candles,
    validate_ma_kind, validate_strategy_cfg, Backend, BackendUsed, DoubleMaRequest,
    HalfTrendRegimeFilter,
};
use crate::progress::ProgressSink;
use crate::prescan_predictive::{predictive_prescan_seed_pairs, PredictivePrescanParams};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ta_optimizer::mo::{
    EvalOutcome, Nsga2Config, ParamDomain, ParamValue, QmcMoConfig, RandomMoConfig, SearchSpace,
    TpeMoConfig,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::{Metrics, StrategyConfig};

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
pub struct CompositeMoResult {
    pub data_id: String,
    pub pareto: Vec<CompositeMoPoint>,
    pub total_evaluations: usize,
    pub runtime_ms: u64,
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
    strategy_trade_on_next_bar: Option<usize>,
}

fn find_param(space: &SearchSpace, name: &str) -> Option<usize> {
    space
        .params
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(name))
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
        strategy_trade_on_next_bar: find_param(space, "strategy_trade_on_next_bar"),
    })
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

    validate_strategy_cfg(&req.base_strategy)?;
    validate_objectives(&req.objectives)?;
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
    let mut allowed_pairs: Option<HashSet<(u16, u16)>> = None;

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
            let device_id = cfg.device_id.unwrap_or(0);

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
            let mut pair_set: HashSet<(u16, u16)> = HashSet::new();

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

            for (fast_ma_type, slow_ma_type) in scan_pairs {
                let (fast_alma_offset, fast_alma_sigma) =
                    resolve_alma_params_for_side("fast", &fast_ma_type, req.fast_ma_params.as_ref())?;
                let (slow_alma_offset, slow_alma_sigma) =
                    resolve_alma_params_for_side("slow", &slow_ma_type, req.slow_ma_params.as_ref())?;

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
                        mode: cfg.mode,
                        top_k: Some(cfg.top_k),
                        include_all: Some(false),
                        heatmap_bins: Some(0),
                    };

                    let dm_res = run_double_ma_optimization_blocking_with_candles(
                        dm_req, candles, cancel, progress,
                    )?;
                    if cfg.require_gpu && matches!(dm_res.backend_used, BackendUsed::Cpu) {
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
                                pair_set.insert((fast, slow));
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
                        let fast_window = period_window_slice(&fast_periods, f, cfg.window);
                        let slow_window = period_window_slice(&slow_periods, s, cfg.window);
                        for &fast in fast_window {
                            for &slow in slow_window {
                                if fast >= slow {
                                    continue;
                                }
                                fast_set.insert(fast);
                                slow_set.insert(slow);
                                pair_set.insert((fast, slow));
                            }
                        }
                    }
                }
            }

            let fast_candidates: Vec<u16> = fast_set.into_iter().collect();
            let slow_candidates: Vec<u16> = slow_set.into_iter().collect();
            if fast_candidates.is_empty() || slow_candidates.is_empty() || pair_set.is_empty() {
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
            pair_set.retain(|(f, s)| fast_allowed.contains(f) && slow_allowed.contains(s) && f < s);
            if pair_set.is_empty() {
                return Err("gpu_prescan produced no valid parameter combinations".to_string());
            }
            allowed_pairs = Some(pair_set);
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
        let mut values: Vec<f64> = vec![f64::NAN; periods.len().saturating_mul(cols)];

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

        // Non-uniform period sets: compute each row individually.
        for (row, &period) in periods.iter().enumerate() {
            let (mat, cols2) = ma_matrix_cpu_f64(
                candles,
                ma_type,
                &req.ma_source,
                &[period],
                ma_params,
                alma_offset,
                alma_sigma,
            )?;
            if cols2 != cols {
                return Err("MA matrix column mismatch".to_string());
            }
            let dst = row.saturating_mul(cols);
            values[dst..dst + cols].copy_from_slice(&mat[0..cols]);
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

    let mut fast_banks: HashMap<String, Bank> = HashMap::new();
    for t in &fast_type_choices {
        let (alma_offset, alma_sigma) =
            resolve_alma_params_for_side("fast", t, req.fast_ma_params.as_ref())?;
        let (values, idx_map) = build_ma_bank(
            t,
            &fast_periods,
            req.fast_ma_params.as_ref(),
            alma_offset,
            alma_sigma,
        )?;
        fast_banks.insert(t.to_string(), Bank { values, idx_map });
    }

    let mut slow_banks: HashMap<String, Bank> = HashMap::new();
    for t in &slow_type_choices {
        let (alma_offset, alma_sigma) =
            resolve_alma_params_for_side("slow", t, req.slow_ma_params.as_ref())?;
        let (values, idx_map) = build_ma_bank(
            t,
            &slow_periods,
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
    let objectives = req.objectives.clone();
    let search_space = req.search_space.clone();
    let allowed_pairs = allowed_pairs.map(Arc::new);
    let indicator_cache = Arc::new(Mutex::new(IndicatorSeriesCache::new(8)));

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
        if allowed_pairs
            .as_ref()
            .is_some_and(|pairs| !pairs.contains(&(fast_len, slow_len)))
        {
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

        let fast_ma_type =
            match decode_ma_type(&search_space, values, idx.fast_ma_type, &base_fast_ma) {
                Ok(v) => v,
                Err(_) => {
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
            if let Ok(v) = get_u16(values, i) {
                halftrend.amplitude = v;
            }
        }
        if let Some(i) = idx.halftrend_channel_deviation {
            if let Ok(v) = get_f64(values, i) {
                halftrend.channel_deviation = v;
            }
        }
        if let Some(i) = idx.halftrend_atr_period {
            if let Ok(v) = get_u16(values, i) {
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
            if let Ok(v) = get_u16(values, i) {
                macz.fast_length = v;
            }
        }
        if let Some(i) = idx.macz_slow_length {
            if let Ok(v) = get_u16(values, i) {
                macz.slow_length = v;
            }
        }
        if let Some(i) = idx.macz_signal_length {
            if let Ok(v) = get_u16(values, i) {
                macz.signal_length = v;
            }
        }
        if let Some(i) = idx.macz_lengthz {
            if let Ok(v) = get_u16(values, i) {
                macz.lengthz = v;
            }
        }
        if let Some(i) = idx.macz_length_stdev {
            if let Ok(v) = get_u16(values, i) {
                macz.length_stdev = v;
            }
        }
        if let Some(i) = idx.macz_a {
            if let Ok(v) = get_f64(values, i) {
                macz.a = v;
            }
        }
        if let Some(i) = idx.macz_b {
            if let Ok(v) = get_f64(values, i) {
                macz.b = v;
            }
        }
        if let Some(i) = idx.macz_gamma {
            if let Ok(v) = get_f64(values, i) {
                macz.gamma = v;
            }
        }
        if let Some(i) = idx.macz_use_lag {
            if let Ok(v) = get_bool(values, i) {
                macz.use_lag = v;
            }
        }
        if let Some(i) = idx.macz_use_volume {
            if let Ok(v) = get_bool(values, i) {
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
            if let Ok(v) = get_u16(values, i) {
                ott.period = v;
            }
        }
        if let Some(i) = idx.ott_percent {
            if let Ok(v) = get_f64(values, i) {
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
            if let Ok(v) = get_f64(values, i) {
                strategy.commission = v;
            }
        }
        if let Some(i) = idx.strategy_eps_rel {
            if let Ok(v) = get_f64(values, i) {
                strategy.eps_rel = v;
            }
        }
        if let Some(i) = idx.strategy_long_only {
            if let Ok(v) = get_bool(values, i) {
                strategy.long_only = v;
            }
        }
        if let Some(i) = idx.strategy_allow_flip {
            if let Ok(v) = get_bool(values, i) {
                strategy.allow_flip = v;
            }
        }
        if let Some(i) = idx.strategy_trade_on_next_bar {
            if let Ok(v) = get_bool(values, i) {
                strategy.trade_on_next_bar = v;
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

        let metrics = if need_sharpe {
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

        EvalOutcome {
            objectives,
            constraints,
            payload: (metrics, params, max_v, min_v),
        }
    };

    let repair = |values: &mut [ParamValue]| {
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
        if fast_len == 0 || slow_len == 0 || fast_len < slow_len {
            return;
        }

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

        if let Some(new_fast) = max_less_than(&fast_periods, slow_len) {
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
                return;
            }
        }

        if let Some(new_slow) = min_greater_than(&slow_periods, fast_len) {
            set_len_value(
                values,
                idx.slow_len,
                slow_domain,
                slow_choice_idx_by_period.as_ref(),
                new_slow,
            );
            return;
        }

        if let (Some(&min_fast), Some(&max_slow)) = (fast_periods.first(), slow_periods.last()) {
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
    };

    let (progress_phase, progress_total) = match req.sampler.clone() {
        CompositeMoSampler::Nsga2 { config } => (
            "composite_mo_nsga2",
            config
                .population_size
                .saturating_mul(config.generations.saturating_add(1)),
        ),
        CompositeMoSampler::Random { config } => ("composite_mo_random", config.trials),
        CompositeMoSampler::Tpe { config } => ("composite_mo_tpe", config.trials),
        CompositeMoSampler::Qmc { config } => ("composite_mo_qmc", config.trials),
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

    let (pareto_points, total_evals) = match req.sampler.clone() {
        CompositeMoSampler::Nsga2 { config } => {
            let res = ta_optimizer::mo::nsga2_optimize(
                &req.search_space,
                config,
                repair,
                eval_with_progress,
            )?;
            (res.pareto, res.total_evaluations)
        }
        CompositeMoSampler::Random { config } => {
            let pareto = ta_optimizer::mo::random_mo_search(
                &req.search_space,
                config.clone(),
                repair,
                eval_with_progress,
            )?;
            (pareto, config.trials)
        }
        CompositeMoSampler::Tpe { config } => {
            let pareto = ta_optimizer::mo::tpe_mo_search(
                &req.search_space,
                config.clone(),
                repair,
                eval_with_progress,
            )?;
            (pareto, config.trials)
        }
        CompositeMoSampler::Qmc { config } => {
            let pareto = ta_optimizer::mo::qmc_mo_search(
                &req.search_space,
                config.clone(),
                repair,
                eval_with_progress,
            )?;
            (pareto, config.trials)
        }
    };

    emit_progress(progress, progress_total, progress_total, progress_phase);

    let pareto = pareto_points
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

    let runtime_ms = start.elapsed().as_millis() as u64;
    Ok(CompositeMoResult {
        data_id: req.data_id,
        pareto,
        total_evaluations: total_evals,
        runtime_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

        assert_eq!(res.total_evaluations, 50);
        assert!(!res.pareto.is_empty());
        assert!(res
            .pareto
            .iter()
            .any(|p| p.params.fast_len < p.params.slow_len));
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
        assert_eq!(res.total_evaluations, 30);
        assert!(!res.pareto.is_empty());
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

        assert_eq!(res.total_evaluations, 40);
        assert!(!res.pareto.is_empty());
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

        assert_eq!(res.total_evaluations, 64);
        assert!(!res.pareto.is_empty());
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

        assert_eq!(res.total_evaluations, 40);
        assert!(!res.pareto.is_empty());
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
                device_id: None,
                require_gpu: false,
                objectives: CompositeGpuPrescanObjectives::Both,
                ma_pairs: None,
                mode: OptimizationMode::Grid,
            }),
            mode: None,
        };

        let res = run_composite_mo_optimization_blocking_with_candles(req, &candles, &cancel, None)
            .expect("composite mo run");

        assert_eq!(res.total_evaluations, 20);
        assert!(!res.pareto.is_empty());
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
    }
}
