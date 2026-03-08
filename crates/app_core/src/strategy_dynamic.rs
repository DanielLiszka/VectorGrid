use crate::composite_cpu::{compute_macz_hist, compute_ott_line};
use crate::double_ma::{
    compute_halftrend_trend, compute_kaufman_stop_series, compute_log_returns_f64,
    eval_metrics_f64_with_exit_mode, strategy_cfg_for_optimization, KaufmanStopSeries,
};
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_with_inputs, PreparedCudaIndicatorInputs,
};
use crate::strategy_ir::{build_static_strategy_ir, StaticStrategyIrInput, StrategyProfileKind};
use crate::strategy_plan::{compile_strategy_plan, CompiledStrategyPlan, StrategyPlanBackend};
use crate::CompositeSignalMode;
use std::collections::HashMap;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use ta_strategies::double_ma::Metrics;
use vector_ta::utilities::data_loader::{source_type, Candles};
use vector_ta::utilities::enums::Kernel;

pub struct PreparedDynamicCpuContext {
    pub lr: Vec<f64>,
    pub close: Vec<f64>,
    pub ma_source_prices: Vec<f64>,
    pub halftrend_filter: Option<Vec<f64>>,
    pub composite_halftrend: Option<Vec<f64>>,
    pub macz_hist: Option<Vec<f64>>,
    pub ott_line: Option<Vec<f64>>,
    pub dynamic_slot_series: HashMap<String, DynamicIndicatorComputeOutput>,
    kaufman_stops: Option<KaufmanStopSeries>,
}

pub type DynamicIndicatorStagingProgressFn<'a> =
    dyn FnMut(usize, usize, &str, crate::strategy_plan::IndicatorRouteKind, bool) + 'a;

fn env_flag_enabled(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let t = v.trim().to_ascii_lowercase();
            t == "1" || t == "true" || t == "yes" || t == "on"
        }
        Err(_) => false,
    }
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
}

pub fn dynamic_strategy_opt_experimental_enabled() -> bool {
    env_flag_enabled("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL")
}

pub fn dynamic_strategy_opt_shadow_enabled() -> bool {
    env_flag_enabled("VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW")
}

pub fn dynamic_strategy_opt_shadow_every() -> usize {
    env_usize("VECTORBT_DYNAMIC_STRATEGY_OPT_SHADOW_EVERY")
        .filter(|v| *v > 0)
        .unwrap_or(256)
}

fn dynamic_strategy_opt_artifact_path() -> Option<String> {
    std::env::var("VECTORBT_DYNAMIC_STRATEGY_OPT_ARTIFACT_PATH")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[derive(Debug, Clone, Copy)]
pub struct DynamicStrategyArtifactRecord<'a> {
    pub scope: &'a str,
    pub profile: &'a str,
    pub backend: &'a str,
    pub dynamic_enabled: bool,
    pub shadow_enabled: bool,
    pub shadow_every: usize,
    pub shadow_sampled: usize,
    pub shadow_mismatches: usize,
    pub runtime_ms: u64,
    pub work_items: usize,
}

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

fn append_dynamic_strategy_artifact(
    path: &str,
    record: &DynamicStrategyArtifactRecord<'_>,
) -> Result<(), String> {
    let file_path = Path::new(path);
    if let Some(parent) = file_path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)
        .map_err(|e| e.to_string())?;
    let need_header = file.metadata().map_err(|e| e.to_string())?.len() == 0;
    if need_header {
        writeln!(
            file,
            "timestamp_unix_s,scope,profile,backend,dynamic_enabled,shadow_enabled,shadow_every,shadow_sampled,shadow_mismatches,runtime_ms,work_items"
        )
        .map_err(|e| e.to_string())?;
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    writeln!(
        file,
        "{},{},{},{},{},{},{},{},{},{},{}",
        ts,
        csv_escape(record.scope),
        csv_escape(record.profile),
        csv_escape(record.backend),
        u8::from(record.dynamic_enabled),
        u8::from(record.shadow_enabled),
        record.shadow_every,
        record.shadow_sampled,
        record.shadow_mismatches,
        record.runtime_ms,
        record.work_items
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn maybe_append_dynamic_strategy_artifact(record: DynamicStrategyArtifactRecord<'_>) {
    let Some(path) = dynamic_strategy_opt_artifact_path() else {
        return;
    };
    if let Err(e) = append_dynamic_strategy_artifact(&path, &record) {
        eprintln!(
            "[dynamic-shadow][artifact] failed to append report '{}': {}",
            path, e
        );
    }
}

fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

pub(crate) fn metrics_shadow_equivalent(a: &Metrics, b: &Metrics) -> bool {
    // Keep this strict enough to catch semantic drift while tolerant to tiny fp noise.
    let tol = 1e-9;
    a.trades == b.trades
        && approx_eq(a.pnl, b.pnl, tol)
        && approx_eq(a.sharpe, b.sharpe, tol)
        && approx_eq(a.sortino, b.sortino, tol)
        && approx_eq(a.calmar, b.calmar, tol)
        && approx_eq(a.omega, b.omega, tol)
        && approx_eq(a.volatility, b.volatility, tol)
        && approx_eq(a.max_dd, b.max_dd, tol)
        && approx_eq(a.exposure, b.exposure, tol)
        && approx_eq(a.net_exposure, b.net_exposure, tol)
}

fn composite_profile_from_strategy_flags(
    strategy: &ta_strategies::double_ma::StrategyConfig,
) -> StrategyProfileKind {
    if strategy.breakout_momentum_enable {
        StrategyProfileKind::BreakoutMomentum
    } else if strategy.regime_hybrid_enable {
        StrategyProfileKind::RegimeHybrid
    } else if strategy.trend_quality_enable {
        StrategyProfileKind::CompositeTrend
    } else {
        StrategyProfileKind::CompositeCpu
    }
}

pub fn compile_dynamic_plan_for_double_ma_config(
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &ta_strategies::double_ma::StrategyConfig,
    halftrend_filter: Option<&crate::double_ma::HalfTrendRegimeFilter>,
    backend: StrategyPlanBackend,
) -> Result<CompiledStrategyPlan, String> {
    let ir = build_static_strategy_ir(
        StrategyProfileKind::DoubleMa,
        StaticStrategyIrInput::from_double_ma(
            ma_source.to_string(),
            fast_ma_type.to_string(),
            slow_ma_type.to_string(),
            fast_ma_params.cloned(),
            slow_ma_params.cloned(),
            strategy_cfg_for_optimization(strategy.clone()),
            halftrend_filter.cloned(),
        ),
    )?;
    compile_strategy_plan(&ir, backend, None)
}

pub fn compile_dynamic_plan_for_composite_cpu_config(
    ma_source: &str,
    fast_ma_type: &str,
    slow_ma_type: &str,
    fast_ma_params: Option<&HashMap<String, f64>>,
    slow_ma_params: Option<&HashMap<String, f64>>,
    strategy: &ta_strategies::double_ma::StrategyConfig,
    halftrend: &crate::double_ma::HalfTrendRegimeFilter,
    macz: &crate::composite_cpu::MaczGateParams,
    ott: &crate::composite_cpu::OttGateParams,
    signal_mode: CompositeSignalMode,
    backend: StrategyPlanBackend,
) -> Result<CompiledStrategyPlan, String> {
    let strategy = strategy_cfg_for_optimization(strategy.clone());
    let profile = composite_profile_from_strategy_flags(&strategy);
    let ir = build_static_strategy_ir(
        profile,
        StaticStrategyIrInput {
            ma_source: ma_source.to_string(),
            fast_ma_type: fast_ma_type.to_string(),
            slow_ma_type: slow_ma_type.to_string(),
            fast_ma_params: fast_ma_params.cloned(),
            slow_ma_params: slow_ma_params.cloned(),
            strategy,
            halftrend_filter: None,
            halftrend: Some(halftrend.clone()),
            macz: Some(macz.clone()),
            ott: Some(ott.clone()),
            signal_mode: Some(signal_mode),
        },
    )?;
    compile_strategy_plan(&ir, backend, None)
}

pub(crate) fn build_dynamic_cpu_context_from_precomputed(
    lr: &[f64],
    close: &[f64],
    ma_source_prices: &[f64],
    halftrend_filter: Option<&[f64]>,
    composite_halftrend: Option<&[f64]>,
    macz_hist: Option<&[f64]>,
    ott_line: Option<&[f64]>,
    kaufman_stops: Option<&KaufmanStopSeries>,
) -> PreparedDynamicCpuContext {
    PreparedDynamicCpuContext {
        lr: lr.to_vec(),
        close: close.to_vec(),
        ma_source_prices: ma_source_prices.to_vec(),
        halftrend_filter: halftrend_filter.map(|v| v.to_vec()),
        composite_halftrend: composite_halftrend.map(|v| v.to_vec()),
        macz_hist: macz_hist.map(|v| v.to_vec()),
        ott_line: ott_line.map(|v| v.to_vec()),
        dynamic_slot_series: HashMap::new(),
        kaufman_stops: kaufman_stops.cloned(),
    }
}

pub fn prepare_dynamic_cpu_context(
    plan: &CompiledStrategyPlan,
    candles: &Candles,
    ma_source: &str,
) -> Result<PreparedDynamicCpuContext, String> {
    let close = source_type(candles, "close");
    let ma_source_prices = source_type(candles, ma_source);
    if close.len() != ma_source_prices.len() {
        return Err("ma source length mismatch".to_string());
    }
    let close = close.to_vec();
    let ma_source_prices = ma_source_prices.to_vec();
    let lr = compute_log_returns_f64(&close);

    let halftrend_filter = match plan.halftrend_filter.as_ref() {
        Some(filter) => Some(compute_halftrend_trend(candles, filter)?),
        None => None,
    };

    let kaufman_stops = compute_kaufman_stop_series(candles, &plan.strategy)?;

    let (composite_halftrend, macz_hist, ott_line) = match plan.profile {
        StrategyProfileKind::DoubleMa => (None, None, None),
        StrategyProfileKind::CompositeCpu
        | StrategyProfileKind::CompositeTrend
        | StrategyProfileKind::RegimeHybrid
        | StrategyProfileKind::BreakoutMomentum => {
            let halftrend_cfg = plan
                .halftrend
                .as_ref()
                .ok_or_else(|| "compiled plan missing composite halftrend config".to_string())?;
            let macz_cfg = plan
                .macz
                .as_ref()
                .ok_or_else(|| "compiled plan missing composite macz config".to_string())?;
            let ott_cfg = plan
                .ott
                .as_ref()
                .ok_or_else(|| "compiled plan missing composite ott config".to_string())?;

            (
                Some(compute_halftrend_trend(candles, halftrend_cfg)?),
                Some(compute_macz_hist(candles, ma_source, macz_cfg)?),
                Some(compute_ott_line(candles, ma_source, ott_cfg)?),
            )
        }
    };
    let dynamic_slot_series = prepare_dynamic_indicator_slot_series(plan, candles, ma_source)?;

    Ok(PreparedDynamicCpuContext {
        lr,
        close,
        ma_source_prices,
        halftrend_filter,
        composite_halftrend,
        macz_hist,
        ott_line,
        dynamic_slot_series,
        kaufman_stops,
    })
}

fn dynamic_indicator_cache_key(
    dataset_signature: &str,
    route: crate::strategy_plan::IndicatorRouteKind,
    source_indicator_id: &str,
    output_id: Option<&str>,
    params: &[crate::indicator_catalog::IndicatorParamKV],
) -> Result<String, String> {
    let params_json = serde_json::to_string(params)
        .map_err(|e| format!("serialize indicator params for cache key: {e}"))?;
    Ok(format!(
        "{}|{:?}|{}|{}|{}",
        dataset_signature,
        route,
        source_indicator_id.trim().to_ascii_lowercase(),
        output_id.unwrap_or("").trim().to_ascii_lowercase(),
        params_json
    ))
}

fn dataset_cache_signature(candles: &Candles, ma_source: &str) -> String {
    let len = candles.close.len();
    let first_ts = candles.timestamp.first().copied().unwrap_or_default();
    let last_ts = candles.timestamp.last().copied().unwrap_or_default();
    let first_close = candles.close.first().copied().unwrap_or_default();
    let last_close = candles.close.last().copied().unwrap_or_default();
    let ma_slice = source_type(candles, ma_source);
    let first_src = ma_slice.first().copied().unwrap_or_default();
    let last_src = ma_slice.last().copied().unwrap_or_default();
    format!(
        "n={len}|ts={first_ts}:{last_ts}|close={first_close:.8}:{last_close:.8}|src={first_src:.8}:{last_src:.8}|ma={}",
        ma_source.trim().to_ascii_lowercase()
    )
}

fn dynamic_indicator_staging_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|flag| flag.load(Ordering::Relaxed))
}

fn emit_dynamic_indicator_staging_progress(
    progress: &mut Option<&mut DynamicIndicatorStagingProgressFn<'_>>,
    completed: usize,
    total: usize,
    indicator_id: &str,
    route: crate::strategy_plan::IndicatorRouteKind,
    cache_hit: bool,
) {
    if let Some(cb) = progress.as_deref_mut() {
        cb(completed, total, indicator_id, route, cache_hit);
    }
}

pub fn prepare_dynamic_indicator_slot_series_cpu(
    plan: &CompiledStrategyPlan,
    candles: &Candles,
    ma_source: &str,
) -> Result<HashMap<String, DynamicIndicatorComputeOutput>, String> {
    prepare_dynamic_indicator_slot_series_cpu_with_controls(plan, candles, ma_source, None, None)
}

pub fn prepare_dynamic_indicator_slot_series_cpu_with_controls(
    plan: &CompiledStrategyPlan,
    candles: &Candles,
    ma_source: &str,
    cancel: Option<&AtomicBool>,
    mut progress: Option<&mut DynamicIndicatorStagingProgressFn<'_>>,
) -> Result<HashMap<String, DynamicIndicatorComputeOutput>, String> {
    let mut slot_outputs: HashMap<String, DynamicIndicatorComputeOutput> = HashMap::new();
    let mut cache: HashMap<String, DynamicIndicatorComputeOutput> = HashMap::new();
    let dataset_sig = dataset_cache_signature(candles, ma_source);
    let total = plan
        .indicator_plan
        .iter()
        .filter(|slot| slot.source_indicator_id.is_some())
        .count();
    let mut completed = 0usize;

    for slot in &plan.indicator_plan {
        if dynamic_indicator_staging_cancelled(cancel) {
            return Err("cancelled".to_string());
        }
        let Some(source_indicator_id) = slot.source_indicator_id.as_deref() else {
            continue;
        };
        let cache_key = dynamic_indicator_cache_key(
            dataset_sig.as_str(),
            slot.route,
            source_indicator_id,
            slot.output_id.as_deref(),
            slot.params.as_slice(),
        )?;
        if let Some(cached) = cache.get(cache_key.as_str()) {
            slot_outputs.insert(slot.indicator_id.clone(), cached.clone());
            completed = completed.saturating_add(1);
            emit_dynamic_indicator_staging_progress(
                &mut progress,
                completed,
                total,
                slot.indicator_id.as_str(),
                slot.route,
                true,
            );
            continue;
        }

        if dynamic_indicator_staging_cancelled(cancel) {
            return Err("cancelled".to_string());
        }
        let computed = compute_dynamic_indicator_cpu(
            candles,
            ma_source,
            source_indicator_id,
            slot.output_id.as_deref(),
            slot.params.as_slice(),
            Kernel::Auto,
        )
        .map_err(|e| e.to_string())?;
        cache.insert(cache_key, computed.clone());
        slot_outputs.insert(slot.indicator_id.clone(), computed);
        completed = completed.saturating_add(1);
        emit_dynamic_indicator_staging_progress(
            &mut progress,
            completed,
            total,
            slot.indicator_id.as_str(),
            slot.route,
            false,
        );
    }

    Ok(slot_outputs)
}

#[cfg(feature = "cuda")]
fn cast_cuda_output_to_cpu_series(
    out: crate::dynamic_indicator_adapter::DynamicIndicatorCudaComputeOutput,
) -> Result<DynamicIndicatorComputeOutput, String> {
    use crate::dynamic_indicator_adapter::DynamicIndicatorCudaSeries;

    let series = match out.series {
        DynamicIndicatorCudaSeries::HostF32(values) => {
            crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(
                values.into_iter().map(|v| v as f64).collect(),
            )
        }
        DynamicIndicatorCudaSeries::DeviceF32(_) => {
            return Err(
                "cuda adapter returned device output where host output was requested".to_string(),
            )
        }
    };
    Ok(DynamicIndicatorComputeOutput {
        output_id: out.output_id,
        series,
        warmup: out.warmup,
        rows: out.rows,
        cols: out.cols,
        pattern_ids: out.pattern_ids,
    })
}

pub fn prepare_dynamic_indicator_slot_series(
    plan: &CompiledStrategyPlan,
    candles: &Candles,
    ma_source: &str,
) -> Result<HashMap<String, DynamicIndicatorComputeOutput>, String> {
    prepare_dynamic_indicator_slot_series_with_controls(plan, candles, ma_source, None, None)
}

pub fn prepare_dynamic_indicator_slot_series_with_controls(
    plan: &CompiledStrategyPlan,
    candles: &Candles,
    ma_source: &str,
    cancel: Option<&AtomicBool>,
    progress: Option<&mut DynamicIndicatorStagingProgressFn<'_>>,
) -> Result<HashMap<String, DynamicIndicatorComputeOutput>, String> {
    #[cfg(not(feature = "cuda"))]
    {
        return prepare_dynamic_indicator_slot_series_cpu_with_controls(
            plan, candles, ma_source, cancel, progress,
        );
    }
    #[cfg(feature = "cuda")]
    {
        let mut progress = progress;
        let mut slot_outputs: HashMap<String, DynamicIndicatorComputeOutput> = HashMap::new();
        let mut cache: HashMap<String, DynamicIndicatorComputeOutput> = HashMap::new();
        let dataset_sig = dataset_cache_signature(candles, ma_source);
        let prepared_cuda_inputs = PreparedCudaIndicatorInputs::from_candles(candles, ma_source);
        let total = plan
            .indicator_plan
            .iter()
            .filter(|slot| slot.source_indicator_id.is_some())
            .count();
        let mut completed = 0usize;

        for slot in &plan.indicator_plan {
            if dynamic_indicator_staging_cancelled(cancel) {
                return Err("cancelled".to_string());
            }
            let Some(source_indicator_id) = slot.source_indicator_id.as_deref() else {
                continue;
            };
            let cache_key = dynamic_indicator_cache_key(
                dataset_sig.as_str(),
                slot.route,
                source_indicator_id,
                slot.output_id.as_deref(),
                slot.params.as_slice(),
            )?;
            if let Some(cached) = cache.get(cache_key.as_str()) {
                slot_outputs.insert(slot.indicator_id.clone(), cached.clone());
                completed = completed.saturating_add(1);
                emit_dynamic_indicator_staging_progress(
                    &mut progress,
                    completed,
                    total,
                    slot.indicator_id.as_str(),
                    slot.route,
                    true,
                );
                continue;
            }

            if dynamic_indicator_staging_cancelled(cancel) {
                return Err("cancelled".to_string());
            }
            let computed = match slot.route {
                crate::strategy_plan::IndicatorRouteKind::Cpu => compute_dynamic_indicator_cpu(
                    candles,
                    ma_source,
                    source_indicator_id,
                    slot.output_id.as_deref(),
                    slot.params.as_slice(),
                    Kernel::Auto,
                )
                .map_err(|e| e.to_string())?,
                crate::strategy_plan::IndicatorRouteKind::GpuSingle
                | crate::strategy_plan::IndicatorRouteKind::GpuKernel => {
                    let cuda_attempt = compute_dynamic_indicator_cuda_with_inputs(
                        &prepared_cuda_inputs,
                        source_indicator_id,
                        slot.output_id.as_deref(),
                        slot.params.as_slice(),
                        Kernel::Auto,
                        vector_ta::indicators::dispatch::CudaOutputTarget::HostF32,
                    );
                    match cuda_attempt {
                        Ok(out) => cast_cuda_output_to_cpu_series(out)?,
                        Err(_) => compute_dynamic_indicator_cpu(
                            candles,
                            ma_source,
                            source_indicator_id,
                            slot.output_id.as_deref(),
                            slot.params.as_slice(),
                            Kernel::Auto,
                        )
                        .map_err(|e| e.to_string())?,
                    }
                }
            };

            cache.insert(cache_key, computed.clone());
            slot_outputs.insert(slot.indicator_id.clone(), computed);
            completed = completed.saturating_add(1);
            emit_dynamic_indicator_staging_progress(
                &mut progress,
                completed,
                total,
                slot.indicator_id.as_str(),
                slot.route,
                false,
            );
        }
        Ok(slot_outputs)
    }
}

pub fn eval_dynamic_plan_metrics_cpu(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    compute_sharpe: bool,
) -> Result<Metrics, String> {
    if fast.len() != context.lr.len() || slow.len() != context.lr.len() {
        return Err("fast/slow series length mismatch".to_string());
    }
    if start_t >= context.lr.len() {
        return Ok(Metrics::default());
    }

    eval_dynamic_plan_metrics_cpu_precomputed(
        plan,
        &context.lr,
        &context.close,
        &context.ma_source_prices,
        context.halftrend_filter.as_deref(),
        context.composite_halftrend.as_deref(),
        context.macz_hist.as_deref(),
        context.ott_line.as_deref(),
        context.kaufman_stops.as_ref(),
        fast,
        slow,
        start_t,
        compute_sharpe,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn eval_dynamic_plan_metrics_cpu_precomputed(
    plan: &CompiledStrategyPlan,
    lr: &[f64],
    close: &[f64],
    ma_source_prices: &[f64],
    halftrend_filter: Option<&[f64]>,
    composite_halftrend: Option<&[f64]>,
    macz_hist: Option<&[f64]>,
    ott_line: Option<&[f64]>,
    kaufman_stops: Option<&KaufmanStopSeries>,
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    compute_sharpe: bool,
) -> Result<Metrics, String> {
    let n = lr.len();
    if close.len() != n || ma_source_prices.len() != n || fast.len() != n || slow.len() != n {
        return Err("series length mismatch".to_string());
    }
    if start_t >= n {
        return Ok(Metrics::default());
    }

    match plan.profile {
        StrategyProfileKind::DoubleMa => Ok(eval_metrics_f64_with_exit_mode(
            lr,
            close,
            fast,
            slow,
            halftrend_filter,
            start_t,
            &plan.strategy,
            kaufman_stops,
            compute_sharpe,
        )),
        StrategyProfileKind::CompositeCpu
        | StrategyProfileKind::CompositeTrend
        | StrategyProfileKind::RegimeHybrid
        | StrategyProfileKind::BreakoutMomentum => {
            let halftrend = composite_halftrend
                .ok_or_else(|| "composite halftrend series missing in context".to_string())?;
            let macz_hist =
                macz_hist.ok_or_else(|| "composite macz series missing in context".to_string())?;
            let ott_line =
                ott_line.ok_or_else(|| "composite ott series missing in context".to_string())?;
            let signal_mode = plan.signal_mode.unwrap_or(CompositeSignalMode::HardGate);

            let metrics = if compute_sharpe {
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                    lr,
                    ma_source_prices,
                    fast,
                    slow,
                    halftrend,
                    macz_hist,
                    ott_line,
                    start_t,
                    &plan.strategy,
                    signal_mode,
                )
            } else {
                ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_fast_with_mode(
                    lr,
                    ma_source_prices,
                    fast,
                    slow,
                    halftrend,
                    macz_hist,
                    ott_line,
                    start_t,
                    &plan.strategy,
                    signal_mode,
                )
            };
            Ok(metrics)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite_cpu::{CompositeCpuRequest, MaczGateParams, OttGateParams};
    use crate::double_ma::{
        first_finite_idx, ma_matrix_cpu_f64, resolve_alma_params_for_side, Backend,
        DoubleMaRequest, HalfTrendRegimeFilter,
    };
    use crate::indicator_catalog::{IndicatorParamKV, IndicatorParamValue};
    use crate::strategy_ir::{
        ConditionGroup, ConditionGroupMode, ConditionNode, ConditionOp, ExecutionPolicy,
        IndicatorNode, SignalBindings, SignalProgramGraph, StrategyIr, ValueRef,
        STRATEGY_IR_SCHEMA_VERSION,
    };
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.05;
            open.push(base);
            high.push(base + 0.75);
            low.push(base - 0.75);
            close.push(base + ((i % 11) as f64 - 5.0) * 0.01);
            vol.push(10_000.0 + ((i % 17) as f64) * 10.0);
        }
        Candles::new(ts, open, high, low, close, vol)
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-10
    }

    fn make_dynamic_indicator_plan_for_slot_prep() -> (CompiledStrategyPlan, Candles) {
        let candles = make_trending_candles(2048);
        let ir = StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION,
            profile: StrategyProfileKind::DoubleMa,
            ma_source: "close".to_string(),
            fast_ma: crate::strategy_ir::MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: HashMap::new(),
            },
            slow_ma: crate::strategy_ir::MovingAverageSpec {
                ma_type: "alma".to_string(),
                params: HashMap::new(),
            },
            program: SignalProgramGraph {
                indicators: vec![
                    IndicatorNode {
                        id: "sma_20_a".to_string(),
                        kind: None,
                        indicator_id: Some("sma".to_string()),
                        params: vec![IndicatorParamKV {
                            key: "period".to_string(),
                            value: IndicatorParamValue::Int(20),
                        }],
                        output_id: Some("value".to_string()),
                    },
                    // Duplicate compute shape to exercise cache reuse.
                    IndicatorNode {
                        id: "sma_20_b".to_string(),
                        kind: None,
                        indicator_id: Some("sma".to_string()),
                        params: vec![IndicatorParamKV {
                            key: "period".to_string(),
                            value: IndicatorParamValue::Int(20),
                        }],
                        output_id: Some("value".to_string()),
                    },
                ],
                conditions: vec![ConditionNode {
                    id: "c".to_string(),
                    op: ConditionOp::Gt,
                    lhs: ValueRef::Indicator {
                        id: "sma_20_a".to_string(),
                    },
                    rhs: ValueRef::Indicator {
                        id: "sma_20_b".to_string(),
                    },
                }],
                groups: vec![ConditionGroup {
                    id: "g".to_string(),
                    mode: ConditionGroupMode::All,
                    condition_ids: vec!["c".to_string()],
                }],
                signals: SignalBindings {
                    long_entry_group: Some("g".to_string()),
                    long_exit_group: None,
                    short_entry_group: None,
                    short_exit_group: None,
                },
            },
            execution: ExecutionPolicy {
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                halftrend: None,
                macz: None,
                ott: None,
                signal_mode: None,
            },
        };
        let plan = crate::strategy_plan::compile_strategy_plan(&ir, StrategyPlanBackend::Cpu, None)
            .expect("plan should compile");
        (plan, candles)
    }

    #[test]
    fn dynamic_double_ma_cpu_metrics_match_static_eval() {
        let candles = make_trending_candles(4096);
        let strategy = StrategyConfig::default();
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (10, 10, 1),
            slow_range: (30, 30, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: strategy.clone(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
        };

        let plan = compile_dynamic_plan_for_double_ma_config(
            &req.ma_source,
            "sma",
            "sma",
            req.fast_ma_params.as_ref(),
            req.slow_ma_params.as_ref(),
            &req.strategy,
            req.halftrend_filter.as_ref(),
            StrategyPlanBackend::Cpu,
        )
        .expect("plan should compile");
        let context =
            prepare_dynamic_cpu_context(&plan, &candles, "close").expect("context should prepare");

        let periods_fast = [10_u16];
        let periods_slow = [30_u16];
        let (fast_mat, cols_fast) =
            ma_matrix_cpu_f64(&candles, "sma", "close", &periods_fast, None, 0.85, 6.0)
                .expect("fast matrix");
        let (slow_mat, cols_slow) =
            ma_matrix_cpu_f64(&candles, "sma", "close", &periods_slow, None, 0.85, 6.0)
                .expect("slow matrix");
        assert_eq!(cols_fast, cols_slow);

        let n = cols_fast;
        let fast = &fast_mat[..n];
        let slow = &slow_mat[..n];
        let start_t = 128;

        let static_metrics = eval_metrics_f64_with_exit_mode(
            &context.lr,
            &context.close,
            fast,
            slow,
            None,
            start_t,
            &strategy_cfg_for_optimization(strategy.clone()),
            None,
            true,
        );
        let dynamic_metrics =
            eval_dynamic_plan_metrics_cpu(&plan, &context, fast, slow, start_t, true)
                .expect("dynamic eval should succeed");

        assert!(approx_eq(static_metrics.sharpe, dynamic_metrics.sharpe));
        assert!(approx_eq(static_metrics.pnl, dynamic_metrics.pnl));
        assert!(approx_eq(static_metrics.max_dd, dynamic_metrics.max_dd));
        assert_eq!(static_metrics.trades, dynamic_metrics.trades);
    }

    #[test]
    fn dynamic_composite_cpu_metrics_match_static_eval() {
        let candles = make_trending_candles(4096);
        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            data_id: "test".to_string(),
            fast_range: (10, 10, 1),
            slow_range: (30, 30, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let plan = compile_dynamic_plan_for_composite_cpu_config(
            &req.ma_source,
            "sma",
            "sma",
            req.fast_ma_params.as_ref(),
            req.slow_ma_params.as_ref(),
            &req.strategy,
            &req.halftrend,
            &req.macz,
            &req.ott,
            req.signal_mode,
            StrategyPlanBackend::Cpu,
        )
        .expect("plan should compile");
        let context =
            prepare_dynamic_cpu_context(&plan, &candles, "close").expect("context should prepare");

        let periods_fast = [10_u16];
        let periods_slow = [30_u16];
        let (fast_mat, cols_fast) =
            ma_matrix_cpu_f64(&candles, "sma", "close", &periods_fast, None, 0.85, 6.0)
                .expect("fast matrix");
        let (slow_mat, cols_slow) =
            ma_matrix_cpu_f64(&candles, "sma", "close", &periods_slow, None, 0.85, 6.0)
                .expect("slow matrix");
        assert_eq!(cols_fast, cols_slow);

        let n = cols_fast;
        let fast = &fast_mat[..n];
        let slow = &slow_mat[..n];
        let start_t = 128;

        let static_metrics =
            ta_strategies::composite_cpu::eval_composite_metrics_from_log_returns_and_mas_f64_with_mode(
                &context.lr,
                &context.ma_source_prices,
                fast,
                slow,
                context
                    .composite_halftrend
                    .as_deref()
                    .expect("halftrend should exist"),
                context.macz_hist.as_deref().expect("macz should exist"),
                context.ott_line.as_deref().expect("ott should exist"),
                start_t,
                &strategy_cfg_for_optimization(req.strategy.clone()),
                req.signal_mode,
            );

        let dynamic_metrics =
            eval_dynamic_plan_metrics_cpu(&plan, &context, fast, slow, start_t, true)
                .expect("dynamic eval should succeed");

        assert!(approx_eq(static_metrics.sharpe, dynamic_metrics.sharpe));
        assert!(approx_eq(static_metrics.pnl, dynamic_metrics.pnl));
        assert!(approx_eq(static_metrics.max_dd, dynamic_metrics.max_dd));
        assert_eq!(static_metrics.trades, dynamic_metrics.trades);
    }

    #[test]
    fn prepare_dynamic_indicator_slot_series_cpu_materializes_dynamic_slots() {
        let (plan, candles) = make_dynamic_indicator_plan_for_slot_prep();
        let slot_outputs = prepare_dynamic_indicator_slot_series_cpu(&plan, &candles, "close")
            .expect("slot prep should succeed");
        assert_eq!(slot_outputs.len(), 2);
        let first = slot_outputs
            .get("sma_20_a")
            .expect("first slot output must exist");
        assert_eq!(first.cols, candles.close.len());
        let second = slot_outputs
            .get("sma_20_b")
            .expect("second slot output must exist");
        assert_eq!(second.cols, candles.close.len());
    }

    #[test]
    fn prepare_dynamic_indicator_slot_series_cpu_reuses_cached_compute_shapes() {
        let (plan, candles) = make_dynamic_indicator_plan_for_slot_prep();
        let slot_outputs = prepare_dynamic_indicator_slot_series_cpu(&plan, &candles, "close")
            .expect("slot prep should succeed");
        let first = slot_outputs
            .get("sma_20_a")
            .expect("first slot output must exist");
        let second = slot_outputs
            .get("sma_20_b")
            .expect("second slot output must exist");
        assert_eq!(first.output_id, second.output_id);
        assert_eq!(first.rows, second.rows);
        assert_eq!(first.cols, second.cols);
        match (&first.series, &second.series) {
            (
                crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(lhs),
                crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(rhs),
            ) => {
                assert_eq!(lhs.len(), rhs.len());
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    if l.is_nan() || r.is_nan() {
                        assert!(l.is_nan() && r.is_nan());
                    } else {
                        assert!((l - r).abs() <= 1e-12);
                    }
                }
            }
            _ => panic!("expected f64 series for sma output"),
        }
    }

    #[test]
    fn prepare_dynamic_indicator_slot_series_cpu_reports_cache_hits_for_duplicate_shapes() {
        let (plan, candles) = make_dynamic_indicator_plan_for_slot_prep();
        let mut events: Vec<(String, bool)> = Vec::new();
        {
            let mut progress = |_: usize,
                                _: usize,
                                indicator_id: &str,
                                _: crate::strategy_plan::IndicatorRouteKind,
                                cache_hit: bool| {
                events.push((indicator_id.to_string(), cache_hit));
            };
            let _ = prepare_dynamic_indicator_slot_series_cpu_with_controls(
                &plan,
                &candles,
                "close",
                None,
                Some(&mut progress),
            )
            .expect("slot prep should succeed");
        }
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "sma_20_a");
        assert!(!events[0].1);
        assert_eq!(events[1].0, "sma_20_b");
        assert!(events[1].1);
    }

    #[test]
    fn prepare_dynamic_indicator_slot_series_cpu_cache_key_includes_route() {
        let (mut plan, candles) = make_dynamic_indicator_plan_for_slot_prep();
        plan.indicator_plan[0].route = crate::strategy_plan::IndicatorRouteKind::Cpu;
        plan.indicator_plan[1].route = crate::strategy_plan::IndicatorRouteKind::GpuSingle;

        let mut events: Vec<(String, bool)> = Vec::new();
        {
            let mut progress = |_: usize,
                                _: usize,
                                indicator_id: &str,
                                _: crate::strategy_plan::IndicatorRouteKind,
                                cache_hit: bool| {
                events.push((indicator_id.to_string(), cache_hit));
            };
            let _ = prepare_dynamic_indicator_slot_series_cpu_with_controls(
                &plan,
                &candles,
                "close",
                None,
                Some(&mut progress),
            )
            .expect("slot prep should succeed");
        }
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "sma_20_a");
        assert!(!events[0].1);
        assert_eq!(events[1].0, "sma_20_b");
        assert!(!events[1].1);
    }

    #[test]
    fn prepare_dynamic_indicator_slot_series_cpu_honors_cancellation() {
        let (plan, candles) = make_dynamic_indicator_plan_for_slot_prep();
        let cancel = AtomicBool::new(true);
        let err = prepare_dynamic_indicator_slot_series_cpu_with_controls(
            &plan,
            &candles,
            "close",
            Some(&cancel),
            None,
        )
        .expect_err("cancel flag should short-circuit slot preparation");
        assert!(err.contains("cancelled"));
    }

    #[test]
    fn compile_composite_plan_maps_profile_from_strategy_flags() {
        let mut strategy = StrategyConfig::default();
        strategy.breakout_momentum_enable = true;
        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            data_id: "test".to_string(),
            fast_range: (10, 10, 1),
            slow_range: (30, 30, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy,
            halftrend: HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            macz: MaczGateParams::default(),
            ott: OttGateParams::default(),
            signal_mode: CompositeSignalMode::HardGate,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            gpu_prescan: None,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
        };

        let plan = compile_dynamic_plan_for_composite_cpu_config(
            &req.ma_source,
            "sma",
            "sma",
            req.fast_ma_params.as_ref(),
            req.slow_ma_params.as_ref(),
            &req.strategy,
            &req.halftrend,
            &req.macz,
            &req.ott,
            req.signal_mode,
            StrategyPlanBackend::Cpu,
        )
        .expect("plan should compile");
        assert_eq!(plan.profile, StrategyProfileKind::BreakoutMomentum);
    }

    #[test]
    fn prepare_context_accepts_valid_ma_source() {
        let candles = make_trending_candles(128);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (10, 10, 1),
            slow_range: (30, 30, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
        };
        let plan = compile_dynamic_plan_for_double_ma_config(
            &req.ma_source,
            "sma",
            "sma",
            req.fast_ma_params.as_ref(),
            req.slow_ma_params.as_ref(),
            &req.strategy,
            req.halftrend_filter.as_ref(),
            StrategyPlanBackend::Cpu,
        )
        .expect("plan should compile");
        let ctx = prepare_dynamic_cpu_context(&plan, &candles, "close")
            .expect("known source should prepare");
        assert_eq!(ctx.lr.len(), candles.close.len());
    }

    #[test]
    fn eval_dynamic_plan_rejects_series_len_mismatch() {
        let candles = make_trending_candles(128);
        let req = DoubleMaRequest {
            backend: Backend::CpuOnly,
            data_id: "test".to_string(),
            fast_range: (10, 10, 1),
            slow_range: (30, 30, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["sma".to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            mode: OptimizationMode::Grid,
            top_k: Some(5),
            include_all: Some(false),
            heatmap_bins: Some(0),
            constraints: None,
            vdps: None,
        };
        let plan = compile_dynamic_plan_for_double_ma_config(
            &req.ma_source,
            "sma",
            "sma",
            req.fast_ma_params.as_ref(),
            req.slow_ma_params.as_ref(),
            &req.strategy,
            req.halftrend_filter.as_ref(),
            StrategyPlanBackend::Cpu,
        )
        .expect("plan should compile");
        let context =
            prepare_dynamic_cpu_context(&plan, &candles, "close").expect("context should prepare");
        let err = eval_dynamic_plan_metrics_cpu(&plan, &context, &[1.0, 2.0], &[1.0], 1, true)
            .expect_err("mismatch should fail");
        assert!(err.contains("length mismatch"));
    }

    #[test]
    fn ma_matrix_smoke_for_helper_imports() {
        let candles = make_trending_candles(256);
        let (_offset, _sigma) =
            resolve_alma_params_for_side("fast", "sma", None).expect("resolve helper");
        let (mat, cols) = ma_matrix_cpu_f64(&candles, "sma", "close", &[8, 16], None, 0.85, 6.0)
            .expect("matrix helper");
        assert_eq!(cols, candles.close.len());
        assert_eq!(mat.len(), cols * 2);
        assert!(first_finite_idx(&mat).is_some());
    }

    #[test]
    fn dynamic_strategy_artifact_is_appended_when_enabled() {
        let path = std::env::temp_dir().join(format!(
            "vectorbt_dynamic_strategy_artifact_{}_{}.csv",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        append_dynamic_strategy_artifact(
            path.to_string_lossy().as_ref(),
            &DynamicStrategyArtifactRecord {
                scope: "double_ma_cpu",
                profile: "DoubleMa",
                backend: "Cpu",
                dynamic_enabled: true,
                shadow_enabled: true,
                shadow_every: 8,
                shadow_sampled: 64,
                shadow_mismatches: 0,
                runtime_ms: 12,
                work_items: 100,
            },
        )
        .expect("artifact append should succeed");

        let content = std::fs::read_to_string(&path).expect("artifact file should exist");
        assert!(content.contains("timestamp_unix_s,scope,profile,backend"));
        assert!(content.contains("double_ma_cpu"));
        assert!(content.contains("DoubleMa"));
        let mut lines = content.lines();
        let _header = lines.next().expect("csv header should exist");
        let row = lines.next().expect("csv row should exist");
        let cols: Vec<&str> = row.split(',').collect();
        assert_eq!(cols.len(), 11);
        assert_eq!(cols[1], "double_ma_cpu");
        assert_eq!(cols[2], "DoubleMa");
        assert_eq!(cols[3], "Cpu");
        assert_eq!(cols[4], "1");
        assert_eq!(cols[5], "1");
        assert_eq!(cols[6], "8");
        assert_eq!(cols[7], "64");
        assert_eq!(cols[8], "0");
        assert_eq!(cols[9], "12");
        assert_eq!(cols[10], "100");

        let _ = std::fs::remove_file(path);
    }
}
