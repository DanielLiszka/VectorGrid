use crate::asr::eval_asr_metrics_from_desired;
use crate::composite_cpu::{compute_macz_hist, compute_ott_line};
use crate::double_ma_backend::build_strategy_capability_report_for_request;
use crate::double_ma_strategy::validate_strategy_cfg;
use crate::double_ma::{
    compute_halftrend_trend, compute_kaufman_stop_series, compute_log_returns_f64,
    eval_metrics_f64_with_exit_mode, strategy_cfg_for_optimization,
    validate_strategy_cfg_for_optimization, KaufmanStopSeries,
};
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cpu, DynamicIndicatorComputeOutput,
};
#[cfg(feature = "cuda")]
use crate::dynamic_indicator_adapter::{
    compute_dynamic_indicator_cuda_with_inputs, PreparedCudaIndicatorInputs,
};
use crate::strategy_ir::{build_static_strategy_ir, StaticStrategyIrInput, StrategyProfileKind};
use crate::strategy_plan::{
    compile_strategy_plan, CompiledConditionGroup, CompiledConditionOp, CompiledStrategyPlan,
    CompiledValueRef, StrategyPlanBackend,
};
use crate::strategy_gpu_runtime::{StrategyCapabilityReport, StrategyHandle};
use crate::CompositeSignalMode;
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicDoubleMaStrategyIrPreview {
    pub capability: StrategyCapabilityReport,
    pub requested_backend: StrategyPlanBackend,
    pub resolved_backend: StrategyPlanBackend,
    pub plan_fingerprint_hex: String,
    pub indicator_count: usize,
    pub condition_count: usize,
    pub group_count: usize,
}

pub struct RunnableDynamicDoubleMaPlan {
    pub ir: crate::strategy_ir::StrategyIr,
    pub preview: DynamicDoubleMaStrategyIrPreview,
    pub cpu_plan: CompiledStrategyPlan,
    pub cpu_context: PreparedDynamicCpuContext,
    pub gpu_handle: Option<StrategyHandle>,
}

const DYNAMIC_V1_MAX_INDICATORS: usize = 12;
const DYNAMIC_V1_MAX_CONDITIONS: usize = 24;
const DYNAMIC_V1_MAX_GROUPS: usize = 8;
const DYNAMIC_V1_MAX_GROUP_CONDITIONS: usize = 8;

fn enforce_dynamic_double_ma_v1_output_refs(
    ir: &crate::strategy_ir::StrategyIr,
) -> Result<(), String> {
    use crate::indicator_catalog::{find_dynamic_indicator_descriptor, IndicatorValueType};

    for indicator in &ir.program.indicators {
        let Some(source_indicator_id) = indicator.indicator_id.as_deref() else {
            continue;
        };
        let descriptor = find_dynamic_indicator_descriptor(source_indicator_id).ok_or_else(|| {
            format!(
                "unsupported_indicator: '{}' is not in dynamic indicator registry",
                source_indicator_id
            )
        })?;
        let output_id = crate::indicator_catalog::normalize_and_validate_indicator_output_ref(
            descriptor.id.as_str(),
            indicator.output_id.as_deref(),
        )?;
        let output = descriptor
            .outputs
            .iter()
            .find(|candidate| candidate.id.eq_ignore_ascii_case(output_id.as_str()))
            .ok_or_else(|| {
                format!(
                    "indicator '{}' output '{}' is not in the registry descriptor",
                    descriptor.id, output_id
                )
            })?;
        if !matches!(output.value_type, IndicatorValueType::F64 | IndicatorValueType::F32) {
            return Err(format!(
                "dynamic gpu v1 supports only floating scalar outputs; indicator '{}' output '{}' is {:?}",
                descriptor.id, output.id, output.value_type
            ));
        }
    }

    Ok(())
}

fn enforce_dynamic_double_ma_product_caps(ir: &crate::strategy_ir::StrategyIr) -> Result<(), String> {
    let indicators = ir.program.indicators.len();
    if indicators > DYNAMIC_V1_MAX_INDICATORS {
        return Err(format!(
            "dynamic gpu v1 supports at most {DYNAMIC_V1_MAX_INDICATORS} indicators, got {indicators}"
        ));
    }
    let conditions = ir.program.conditions.len();
    if conditions > DYNAMIC_V1_MAX_CONDITIONS {
        return Err(format!(
            "dynamic gpu v1 supports at most {DYNAMIC_V1_MAX_CONDITIONS} conditions, got {conditions}"
        ));
    }
    let groups = ir.program.groups.len();
    if groups > DYNAMIC_V1_MAX_GROUPS {
        return Err(format!(
            "dynamic gpu v1 supports at most {DYNAMIC_V1_MAX_GROUPS} groups, got {groups}"
        ));
    }
    for group in &ir.program.groups {
        let group_conditions = group.condition_ids.len();
        if group_conditions > DYNAMIC_V1_MAX_GROUP_CONDITIONS {
            return Err(format!(
                "dynamic gpu v1 group '{}' exceeds max condition count ({group_conditions} > {DYNAMIC_V1_MAX_GROUP_CONDITIONS})",
                group.id
            ));
        }
    }

    if !ir.execution.strategy.long_only && ir.program.signals.short_entry_group.is_none() {
        return Err(
            "dynamic gpu v1 bidirectional plans must bind short_entry_group explicitly"
                .to_string(),
        );
    }

    enforce_dynamic_double_ma_v1_output_refs(ir)?;

    Ok(())
}

fn validate_dynamic_indicator_output_for_gpu_v1(
    indicator_id: &str,
    output: &DynamicIndicatorComputeOutput,
) -> Result<(), String> {
    match &output.series {
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(_) => {}
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::I32(_) => {
            return Err(format!(
                "dynamic gpu v1 supports only floating scalar outputs; indicator '{indicator_id}' materialized i32 output '{}'",
                output.output_id
            ));
        }
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::Bool(_) => {
            return Err(format!(
                "dynamic gpu v1 supports only floating scalar outputs; indicator '{indicator_id}' materialized bool output '{}'",
                output.output_id
            ));
        }
    }
    if output.rows != 1 {
        return Err(format!(
            "dynamic gpu v1 supports only single-series scalar outputs; indicator '{indicator_id}' materialized {} row(s) for output '{}'",
            output.rows, output.output_id
        ));
    }
    if output
        .pattern_ids
        .as_ref()
        .is_some_and(|pattern_ids| !pattern_ids.is_empty())
    {
        return Err(format!(
            "dynamic gpu v1 does not support pattern or matrix-style outputs; indicator '{indicator_id}' output '{}' returned pattern ids",
            output.output_id
        ));
    }
    Ok(())
}

pub(crate) fn validate_dynamic_double_ma_gpu_handle_v1(
    handle: &StrategyHandle,
) -> Result<(), String> {
    if handle.plan.backend != StrategyPlanBackend::GpuUnified {
        return Err("dynamic gpu kernel requires a GpuUnified strategy plan".to_string());
    }
    if handle.plan.profile != crate::strategy_ir::StrategyProfileKind::DoubleMa {
        return Err("dynamic gpu kernel currently supports only the DoubleMa profile".to_string());
    }
    if handle.plan.halftrend_filter.is_some() {
        return Err(
            "dynamic gpu kernel v1 does not support halftrend/regime-gated DoubleMa plans"
                .to_string(),
        );
    }
    if handle.plan.strategy.exit_mode != ta_strategies::double_ma::ExitMode::Cross {
        return Err(
            "dynamic gpu kernel v1 supports only plain crossover exit mode for DoubleMa"
                .to_string(),
        );
    }
    let program = handle
        .gpu_program
        .as_ref()
        .ok_or_else(|| "strategy handle is missing a compiled gpu predicate program".to_string())?;
    if program.indicator_slots.len() != 2 {
        return Err(format!(
            "dynamic gpu kernel v1 expects exactly 2 indicator slots, got {}",
            program.indicator_slots.len()
        ));
    }
    for slot in &program.indicator_slots {
        if slot.indicator_index > 1 {
            return Err(format!(
                "dynamic gpu kernel v1 only supports fast/slow slots, found indicator slot index {}",
                slot.indicator_index
            ));
        }
    }
    if let Some(abi) = handle.gpu_program_abi.as_ref() {
        for condition in &abi.conditions {
            let lhs_supported = matches!(condition.lhs.source, 0 | 3);
            let rhs_supported = matches!(condition.rhs.source, 0 | 3);
            if !lhs_supported || !rhs_supported {
                return Err(
                    "dynamic gpu kernel v1 supports only indicator and constant operands"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

pub fn preview_dynamic_double_ma_strategy_ir(
    ir: &crate::strategy_ir::StrategyIr,
    requested_backend: StrategyPlanBackend,
    objective: ta_optimizer::ObjectiveKind,
    top_k: usize,
    include_all: bool,
) -> Result<DynamicDoubleMaStrategyIrPreview, String> {
    if ir.profile != StrategyProfileKind::DoubleMa {
        return Err("dynamic strategy preview currently supports only profile=DoubleMa".to_string());
    }

    validate_strategy_cfg(&ir.execution.strategy)?;
    enforce_dynamic_double_ma_product_caps(ir)?;

    let regime_filter_present =
        ir.execution.halftrend_filter.is_some() || ir.execution.halftrend.is_some();
    let mut capability = build_strategy_capability_report_for_request(
        ir.fast_ma.ma_type.as_str(),
        ir.slow_ma.ma_type.as_str(),
        &ir.execution.strategy,
        regime_filter_present,
        objective,
        top_k,
        include_all,
    );

    let mut compiled_backend = requested_backend;
    let mut plan = match compile_strategy_plan(ir, requested_backend, None) {
        Ok(plan) => plan,
        Err(err) if requested_backend == StrategyPlanBackend::GpuUnified && capability.cpu_dynamic_supported => {
            capability.gpu_static_supported = false;
            capability.gpu_dynamic_supported = false;
            capability.reasons.push(err);
            compiled_backend = StrategyPlanBackend::Cpu;
            compile_strategy_plan(ir, compiled_backend, None)?
        }
        Err(err) => return Err(err),
    };

    if compiled_backend == StrategyPlanBackend::GpuUnified
        && !capability.gpu_dynamic_supported
        && capability.cpu_dynamic_supported
    {
        compiled_backend = StrategyPlanBackend::Cpu;
        plan = compile_strategy_plan(ir, compiled_backend, None)?;
    }

    if compiled_backend == StrategyPlanBackend::GpuUnified && capability.gpu_dynamic_supported {
        let handle = StrategyHandle::from_plan(plan.clone(), capability.clone())?;
        if let Err(reason) = validate_dynamic_double_ma_gpu_handle_v1(&handle) {
            capability.gpu_static_supported = false;
            capability.gpu_dynamic_supported = false;
            capability.reasons.push(reason);
            compiled_backend = StrategyPlanBackend::Cpu;
            plan = compile_strategy_plan(ir, compiled_backend, None)?;
        }
    }

    Ok(DynamicDoubleMaStrategyIrPreview {
        capability,
        requested_backend,
        resolved_backend: compiled_backend,
        plan_fingerprint_hex: plan.ir_fingerprint_hex.clone(),
        indicator_count: plan.indicator_plan.len(),
        condition_count: plan.conditions.len(),
        group_count: plan.groups.len(),
    })
}

pub fn prepare_runnable_dynamic_double_ma_plan(
    ir: crate::strategy_ir::StrategyIr,
    requested_backend: StrategyPlanBackend,
    objective: ta_optimizer::ObjectiveKind,
    top_k: usize,
    include_all: bool,
    candles: &Candles,
) -> Result<RunnableDynamicDoubleMaPlan, String> {
    let preview = preview_dynamic_double_ma_strategy_ir(
        &ir,
        requested_backend,
        objective,
        top_k,
        include_all,
    )?;
    let cpu_plan = compile_strategy_plan(&ir, StrategyPlanBackend::Cpu, None)?;
    let cpu_context = prepare_dynamic_cpu_context(&cpu_plan, candles, ir.ma_source.as_str())?;
    let gpu_handle = if preview.resolved_backend == StrategyPlanBackend::GpuUnified {
        let gpu_plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)?;
        Some(StrategyHandle::from_plan(
            gpu_plan,
            preview.capability.clone(),
        )?)
    } else {
        None
    };
    Ok(RunnableDynamicDoubleMaPlan {
        ir,
        preview,
        cpu_plan,
        cpu_context,
        gpu_handle,
    })
}

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
    validate_strategy_cfg_for_optimization(strategy)?;
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
    validate_strategy_cfg_for_optimization(strategy)?;
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
        validate_dynamic_indicator_output_for_gpu_v1(slot.indicator_id.as_str(), &computed)?;
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
        let prepared_cuda_inputs =
            match PreparedCudaIndicatorInputs::from_candles(candles, ma_source) {
                Ok(v) => v,
                Err(_) => {
                    return prepare_dynamic_indicator_slot_series_cpu_with_controls(
                        plan, candles, ma_source, cancel, progress,
                    );
                }
            };
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
            validate_dynamic_indicator_output_for_gpu_v1(slot.indicator_id.as_str(), &computed)?;

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

fn dynamic_indicator_series_f64<'a>(
    output: &'a DynamicIndicatorComputeOutput,
    indicator_id: &str,
) -> Result<&'a [f64], String> {
    match &output.series {
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(values) => Ok(values.as_slice()),
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::I32(_) => Err(format!(
            "dynamic indicator '{}' materialized i32 output at runtime",
            indicator_id
        )),
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::Bool(_) => Err(format!(
            "dynamic indicator '{}' materialized bool output at runtime",
            indicator_id
        )),
    }
}

fn load_double_ma_plan_indicator_value(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    index: u32,
    t: usize,
) -> Result<f64, String> {
    let slot = plan
        .indicator_plan
        .get(index as usize)
        .ok_or_else(|| format!("compiled indicator index {} out of range", index))?;
    if t >= fast.len() || t >= slow.len() || t >= context.close.len() || t >= context.ma_source_prices.len()
    {
        return Err(format!("bar index {} out of range for dynamic plan load", t));
    }

    match slot.indicator_kind {
        Some(crate::strategy_ir::IndicatorKind::FastMa) => Ok(fast[t]),
        Some(crate::strategy_ir::IndicatorKind::SlowMa) => Ok(slow[t]),
        Some(crate::strategy_ir::IndicatorKind::HalfTrendTrend) => context
            .halftrend_filter
            .as_deref()
            .or(context.composite_halftrend.as_deref())
            .and_then(|series| series.get(t).copied())
            .ok_or_else(|| "halftrend series missing in dynamic cpu context".to_string()),
        Some(crate::strategy_ir::IndicatorKind::MaczHist) => context
            .macz_hist
            .as_deref()
            .and_then(|series| series.get(t).copied())
            .ok_or_else(|| "macz series missing in dynamic cpu context".to_string()),
        Some(crate::strategy_ir::IndicatorKind::OttLine) => context
            .ott_line
            .as_deref()
            .and_then(|series| series.get(t).copied())
            .ok_or_else(|| "ott series missing in dynamic cpu context".to_string()),
        Some(crate::strategy_ir::IndicatorKind::TrendQualityRegime)
        | Some(crate::strategy_ir::IndicatorKind::BreakoutRegime) => Err(format!(
            "indicator '{}' is not supported in the DoubleMa dynamic cpu evaluator",
            slot.indicator_id
        )),
        None => {
            let output = context
                .dynamic_slot_series
                .get(slot.indicator_id.as_str())
                .ok_or_else(|| format!("dynamic indicator slot '{}' missing from cpu context", slot.indicator_id))?;
            let series = dynamic_indicator_series_f64(output, slot.indicator_id.as_str())?;
            series
                .get(t)
                .copied()
                .ok_or_else(|| format!("dynamic indicator '{}' missing bar {}", slot.indicator_id, t))
        }
    }
}

fn load_double_ma_value_ref(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    value: &CompiledValueRef,
    t: usize,
) -> Result<f64, String> {
    match value {
        CompiledValueRef::IndicatorIndex { index } => {
            load_double_ma_plan_indicator_value(plan, context, fast, slow, *index, t)
        }
        CompiledValueRef::MaSource => context
            .ma_source_prices
            .get(t)
            .copied()
            .ok_or_else(|| format!("ma_source missing bar {}", t)),
        CompiledValueRef::Close => context
            .close
            .get(t)
            .copied()
            .ok_or_else(|| format!("close missing bar {}", t)),
        CompiledValueRef::ConstantBits { bits } => Ok(f64::from_bits(*bits)),
    }
}

fn eval_double_ma_compiled_condition(
    condition: &CompiledConditionOp,
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    sig_t: usize,
    prev_t: usize,
) -> Result<bool, String> {
    let lhs_now = load_double_ma_value_ref(plan, context, fast, slow, &condition.lhs, sig_t)?;
    let rhs_now = load_double_ma_value_ref(plan, context, fast, slow, &condition.rhs, sig_t)?;
    let lhs_prev = load_double_ma_value_ref(plan, context, fast, slow, &condition.lhs, prev_t)?;
    let rhs_prev = load_double_ma_value_ref(plan, context, fast, slow, &condition.rhs, prev_t)?;

    let finite_now = lhs_now.is_finite() && rhs_now.is_finite();
    let finite_prev = lhs_prev.is_finite() && rhs_prev.is_finite();

    Ok(match condition.op {
        crate::strategy_ir::ConditionOp::Gt => finite_now && lhs_now > rhs_now,
        crate::strategy_ir::ConditionOp::Ge => finite_now && lhs_now >= rhs_now,
        crate::strategy_ir::ConditionOp::Lt => finite_now && lhs_now < rhs_now,
        crate::strategy_ir::ConditionOp::Le => finite_now && lhs_now <= rhs_now,
        crate::strategy_ir::ConditionOp::CrossUp => {
            finite_now && finite_prev && lhs_prev <= rhs_prev && lhs_now > rhs_now
        }
        crate::strategy_ir::ConditionOp::CrossDown => {
            finite_now && finite_prev && lhs_prev >= rhs_prev && lhs_now < rhs_now
        }
    })
}

fn eval_double_ma_compiled_group(
    group_index: Option<u32>,
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    sig_t: usize,
    prev_t: usize,
) -> Result<bool, String> {
    let Some(group_index) = group_index else {
        return Ok(false);
    };
    let group: &CompiledConditionGroup = plan
        .groups
        .get(group_index as usize)
        .ok_or_else(|| format!("compiled group index {} out of range", group_index))?;
    match group.mode {
        crate::strategy_ir::ConditionGroupMode::All => {
            for condition_index in &group.condition_indexes {
                let condition = plan
                    .conditions
                    .get(*condition_index as usize)
                    .ok_or_else(|| format!("compiled condition index {} out of range", condition_index))?;
                if !eval_double_ma_compiled_condition(
                    condition, plan, context, fast, slow, sig_t, prev_t,
                )? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        crate::strategy_ir::ConditionGroupMode::Any => {
            for condition_index in &group.condition_indexes {
                let condition = plan
                    .conditions
                    .get(*condition_index as usize)
                    .ok_or_else(|| format!("compiled condition index {} out of range", condition_index))?;
                if eval_double_ma_compiled_condition(
                    condition, plan, context, fast, slow, sig_t, prev_t,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn build_dynamic_double_ma_desired_positions(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
) -> Result<Vec<i32>, String> {
    let n = fast.len();
    let mut out = vec![0; n];
    if n == 0 || start_t >= n || slow.len() != n || context.close.len() != n || context.ma_source_prices.len() != n {
        return Ok(out);
    }

    let trade_offset = if plan.strategy.trade_on_next_bar { 1usize } else { 0usize };
    let no_flip = !plan.strategy.allow_flip;
    let long_only = plan.strategy.long_only;
    let mut pos: i32 = 0;

    for t in start_t..n {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let long_entry = eval_double_ma_compiled_group(
            plan.signals.long_entry_group,
            plan,
            context,
            fast,
            slow,
            sig_t,
            prev_t,
        )?;
        let long_exit = eval_double_ma_compiled_group(
            plan.signals.long_exit_group,
            plan,
            context,
            fast,
            slow,
            sig_t,
            prev_t,
        )?;
        let short_entry = eval_double_ma_compiled_group(
            plan.signals.short_entry_group,
            plan,
            context,
            fast,
            slow,
            sig_t,
            prev_t,
        )?;
        let short_exit = eval_double_ma_compiled_group(
            plan.signals.short_exit_group,
            plan,
            context,
            fast,
            slow,
            sig_t,
            prev_t,
        )?;

        let mut seeded_entry = false;
        let mut seeded_short = false;
        if t == start_t && pos == 0 && plan.indicator_plan.len() >= 2 {
            let fast_now =
                load_double_ma_plan_indicator_value(plan, context, fast, slow, 0, sig_t)?;
            let slow_now =
                load_double_ma_plan_indicator_value(plan, context, fast, slow, 1, sig_t)?;
            if fast_now.is_finite() && slow_now.is_finite() {
                let eps = if plan.strategy.eps_rel > 0.0 {
                    plan.strategy.eps_rel * slow_now.abs().max(1.0)
                } else {
                    0.0
                };
                seeded_entry = fast_now > (slow_now + eps);
                seeded_short = fast_now < (slow_now - eps);
            }
        }

        let mut desired = pos;
        if pos == 0 {
            if long_entry || seeded_entry {
                desired = 1;
            } else if !long_only && (short_entry || seeded_short) {
                desired = -1;
            }
        } else if pos > 0 {
            if !long_only && short_entry {
                desired = if no_flip { 0 } else { -1 };
            } else if long_exit {
                desired = 0;
            }
        } else if long_entry {
            desired = if no_flip { 0 } else { 1 };
        } else if short_exit {
            desired = 0;
        }

        if long_only && desired < 0 {
            desired = 0;
        }
        out[t] = desired;

        if desired != pos {
            if pos == 0 || desired == 0 {
                pos = desired;
            } else if no_flip {
                pos = 0;
            } else {
                pos = desired;
            }
        }
    }

    Ok(out)
}

pub fn eval_dynamic_plan_metrics_cpu_with_candles(
    plan: &CompiledStrategyPlan,
    candles: &Candles,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    compute_sharpe: bool,
) -> Result<Metrics, String> {
    match plan.profile {
        StrategyProfileKind::DoubleMa => {
            let desired =
                build_dynamic_double_ma_desired_positions(plan, context, fast, slow, start_t)?;
            let metrics = eval_asr_metrics_from_desired(
                candles,
                context.lr.as_slice(),
                desired.as_slice(),
                start_t,
                &plan.strategy,
            );
            if compute_sharpe {
                Ok(metrics)
            } else {
                let mut fast_metrics = metrics;
                fast_metrics.sharpe = 0.0;
                Ok(fast_metrics)
            }
        }
        _ => eval_dynamic_plan_metrics_cpu(
            plan,
            context,
            fast,
            slow,
            start_t,
            compute_sharpe,
        ),
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

    if plan.profile == StrategyProfileKind::DoubleMa {
        return eval_dynamic_double_ma_metrics_cpu_generic(
            plan,
            context,
            fast,
            slow,
            start_t,
            compute_sharpe,
        );
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

#[inline]
fn load_dynamic_indicator_slot_value(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    slot_index: u32,
    t: usize,
) -> f64 {
    let Some(slot) = plan.indicator_plan.get(slot_index as usize) else {
        return f64::NAN;
    };
    if matches!(
        slot.indicator_kind,
        Some(crate::strategy_ir::IndicatorKind::FastMa)
    ) || slot.indicator_id.eq_ignore_ascii_case("fast_ma")
    {
        return fast.get(t).copied().unwrap_or(f64::NAN);
    }
    if matches!(
        slot.indicator_kind,
        Some(crate::strategy_ir::IndicatorKind::SlowMa)
    ) || slot.indicator_id.eq_ignore_ascii_case("slow_ma")
    {
        return slow.get(t).copied().unwrap_or(f64::NAN);
    }
    let Some(output) = context.dynamic_slot_series.get(slot.indicator_id.as_str()) else {
        return f64::NAN;
    };
    match &output.series {
        crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(values) => {
            values.get(t).copied().unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    }
}

#[inline]
fn load_compiled_value_for_dynamic_double_ma(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    value: &crate::strategy_plan::CompiledValueRef,
    t: usize,
) -> f64 {
    match value {
        crate::strategy_plan::CompiledValueRef::IndicatorIndex { index } => {
            load_dynamic_indicator_slot_value(plan, context, fast, slow, *index, t)
        }
        crate::strategy_plan::CompiledValueRef::MaSource => {
            context.ma_source_prices.get(t).copied().unwrap_or(f64::NAN)
        }
        crate::strategy_plan::CompiledValueRef::Close => {
            context.close.get(t).copied().unwrap_or(f64::NAN)
        }
        crate::strategy_plan::CompiledValueRef::ConstantBits { bits } => f64::from_bits(*bits),
    }
}

#[inline]
fn eval_dynamic_condition_for_double_ma(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    condition: &crate::strategy_plan::CompiledConditionOp,
    sig_t: usize,
    prev_t: usize,
) -> bool {
    let lhs_now =
        load_compiled_value_for_dynamic_double_ma(plan, context, fast, slow, &condition.lhs, sig_t);
    let rhs_now =
        load_compiled_value_for_dynamic_double_ma(plan, context, fast, slow, &condition.rhs, sig_t);
    if !lhs_now.is_finite() || !rhs_now.is_finite() {
        return false;
    }
    match condition.op {
        crate::strategy_ir::ConditionOp::Gt => lhs_now > rhs_now,
        crate::strategy_ir::ConditionOp::Ge => lhs_now >= rhs_now,
        crate::strategy_ir::ConditionOp::Lt => lhs_now < rhs_now,
        crate::strategy_ir::ConditionOp::Le => lhs_now <= rhs_now,
        crate::strategy_ir::ConditionOp::CrossUp => {
            let lhs_prev = load_compiled_value_for_dynamic_double_ma(
                plan,
                context,
                fast,
                slow,
                &condition.lhs,
                prev_t,
            );
            let rhs_prev = load_compiled_value_for_dynamic_double_ma(
                plan,
                context,
                fast,
                slow,
                &condition.rhs,
                prev_t,
            );
            lhs_prev.is_finite() && rhs_prev.is_finite() && lhs_prev <= rhs_prev && lhs_now > rhs_now
        }
        crate::strategy_ir::ConditionOp::CrossDown => {
            let lhs_prev = load_compiled_value_for_dynamic_double_ma(
                plan,
                context,
                fast,
                slow,
                &condition.lhs,
                prev_t,
            );
            let rhs_prev = load_compiled_value_for_dynamic_double_ma(
                plan,
                context,
                fast,
                slow,
                &condition.rhs,
                prev_t,
            );
            lhs_prev.is_finite() && rhs_prev.is_finite() && lhs_prev >= rhs_prev && lhs_now < rhs_now
        }
    }
}

#[inline]
fn eval_dynamic_group_for_double_ma(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    group_index: Option<u32>,
    sig_t: usize,
    prev_t: usize,
) -> bool {
    let Some(group_index) = group_index else {
        return false;
    };
    let Some(group) = plan.groups.get(group_index as usize) else {
        return false;
    };
    let is_all = matches!(group.mode, crate::strategy_ir::ConditionGroupMode::All);
    let mut aggregate = is_all;
    for condition_index in &group.condition_indexes {
        let Some(condition) = plan.conditions.get(*condition_index as usize) else {
            return false;
        };
        let ok = eval_dynamic_condition_for_double_ma(
            plan,
            context,
            fast,
            slow,
            condition,
            sig_t,
            prev_t,
        );
        if is_all {
            aggregate &= ok;
            if !aggregate {
                break;
            }
        } else {
            aggregate |= ok;
            if aggregate {
                break;
            }
        }
    }
    aggregate
}

fn eval_dynamic_double_ma_metrics_cpu_generic(
    plan: &CompiledStrategyPlan,
    context: &PreparedDynamicCpuContext,
    fast: &[f64],
    slow: &[f64],
    start_t: usize,
    compute_sharpe: bool,
) -> Result<Metrics, String> {
    let n_total = context.lr.len();
    if n_total < 2
        || fast.len() != n_total
        || slow.len() != n_total
        || context.close.len() != n_total
        || context.ma_source_prices.len() != n_total
        || start_t >= n_total
    {
        return Ok(Metrics::default());
    }

    let cfg = &plan.strategy;
    let commission = cfg.effective_commission();
    if (!(0.0..1.0).contains(&commission)) && commission != 0.0 {
        return Ok(Metrics::default());
    }
    let log_cost = if commission > 0.0 {
        (1.0 - commission).ln()
    } else {
        0.0
    };
    let stop_log = cfg.stop_loss_pct().map(|pct| (1.0 - pct).ln());
    let take_log = cfg.take_profit_pct().map(|pct| (1.0 + pct).ln());
    let trade_offset = if cfg.trade_on_next_bar { 1usize } else { 0usize };

    let mut pos: i32 = 0;
    let mut log_eq = 0.0_f64;
    let mut log_peak = 0.0_f64;
    let mut max_log_dd = 0.0_f64;
    let mut trades: u32 = 0;
    let mut trade_log_ret = 0.0_f64;

    let mut n_obs: u64 = 0;
    let mut mean = 0.0_f64;
    let mut m2 = 0.0_f64;
    let mut sum_neg_sq = 0.0_f64;
    let mut sum_pos_r = 0.0_f64;
    let mut sum_neg_abs_r = 0.0_f64;
    let mut sum_abs_pos: i64 = 0;
    let mut sum_pos: i64 = 0;

    for t in start_t..n_total {
        let sig_t = t.saturating_sub(trade_offset);
        let prev_t = sig_t.saturating_sub(1);

        let long_entry = eval_dynamic_group_for_double_ma(
            plan,
            context,
            fast,
            slow,
            plan.signals.long_entry_group,
            sig_t,
            prev_t,
        );
        let long_exit = eval_dynamic_group_for_double_ma(
            plan,
            context,
            fast,
            slow,
            plan.signals.long_exit_group,
            sig_t,
            prev_t,
        );
        let short_entry = eval_dynamic_group_for_double_ma(
            plan,
            context,
            fast,
            slow,
            plan.signals.short_entry_group,
            sig_t,
            prev_t,
        );
        let short_exit = eval_dynamic_group_for_double_ma(
            plan,
            context,
            fast,
            slow,
            plan.signals.short_exit_group,
            sig_t,
            prev_t,
        );

        let mut seeded_entry = false;
        let mut seeded_short = false;
        if t == start_t && pos == 0 && plan.indicator_plan.len() >= 2 {
            let fast_now = load_dynamic_indicator_slot_value(plan, context, fast, slow, 0, sig_t);
            let slow_now = load_dynamic_indicator_slot_value(plan, context, fast, slow, 1, sig_t);
            if fast_now.is_finite() && slow_now.is_finite() {
                let eps = if cfg.eps_rel > 0.0 {
                    cfg.eps_rel * 1.0_f64.max(slow_now.abs())
                } else {
                    0.0
                };
                seeded_entry = fast_now > (slow_now + eps);
                seeded_short = fast_now < (slow_now - eps);
            }
        }

        let mut desired = pos;
        if pos == 0 {
            if long_entry || seeded_entry {
                desired = 1;
            } else if !cfg.long_only && (short_entry || seeded_short) {
                desired = -1;
            }
        } else if pos > 0 {
            if !cfg.long_only && short_entry {
                desired = if cfg.allow_flip { -1 } else { 0 };
            } else if long_exit {
                desired = 0;
            }
        } else if long_entry {
            desired = if cfg.allow_flip { 1 } else { 0 };
        } else if short_exit {
            desired = 0;
        }
        if cfg.long_only && desired < 0 {
            desired = 0;
        }

        if desired != pos {
            if pos == 0 || desired == 0 {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades = trades.saturating_add(1);
                pos = desired;
                trade_log_ret = if pos != 0 && log_cost != 0.0 {
                    log_cost
                } else {
                    0.0
                };
            } else if !cfg.allow_flip {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
            } else {
                if log_cost != 0.0 {
                    log_eq += 2.0 * log_cost;
                }
                trades = trades.saturating_add(2);
                pos = desired;
                trade_log_ret = if log_cost != 0.0 { log_cost } else { 0.0 };
            }
        }

        let pos_bar = pos;
        let lr_t = context.lr[t];
        let step_log = (pos_bar as f64) * lr_t;
        log_eq += step_log;

        if pos_bar != 0 {
            trade_log_ret += step_log;
            let hit_stop = stop_log.is_some_and(|th| trade_log_ret <= th);
            let hit_take = take_log.is_some_and(|th| trade_log_ret >= th);
            if hit_stop || hit_take {
                if log_cost != 0.0 {
                    log_eq += log_cost;
                }
                trades = trades.saturating_add(1);
                pos = 0;
                trade_log_ret = 0.0;
            }
        }

        if log_eq > log_peak {
            log_peak = log_eq;
        }
        let dd_log = log_peak - log_eq;
        if dd_log > max_log_dd {
            max_log_dd = dd_log;
        }

        let step_r = if pos_bar == 0 {
            0.0
        } else {
            (pos_bar as f64) * lr_t.exp_m1()
        };

        if compute_sharpe {
            n_obs = n_obs.saturating_add(1);
            let delta = step_r - mean;
            mean += delta / n_obs as f64;
            let delta2 = step_r - mean;
            m2 += delta * delta2;
            if step_r < 0.0 {
                sum_neg_sq += step_r * step_r;
                sum_neg_abs_r += -step_r;
            } else {
                sum_pos_r += step_r;
            }
        } else {
            n_obs = n_obs.saturating_add(1);
        }
        sum_abs_pos += i64::from(pos_bar.abs());
        sum_pos += i64::from(pos_bar);
    }

    let pnl = log_eq.exp() - 1.0;
    let max_dd = if max_log_dd > 0.0 {
        -(-max_log_dd).exp_m1()
    } else {
        0.0
    };
    let denom = n_obs.max(1) as f64;
    let exposure = sum_abs_pos as f64 / denom;
    let net_exposure = sum_pos as f64 / denom;

    if !compute_sharpe {
        let calmar = if max_dd > 1.0e-12 {
            pnl / max_dd
        } else if pnl > 0.0 {
            1.0e6
        } else if pnl < 0.0 {
            -1.0e6
        } else {
            0.0
        }
        .clamp(-1.0e6, 1.0e6);
        return Ok(Metrics {
            pnl,
            sharpe: 0.0,
            sortino: 0.0,
            calmar,
            omega: 0.0,
            volatility: 0.0,
            max_dd,
            trades,
            exposure,
            net_exposure,
        });
    }

    let variance = if n_obs > 1 { m2 / (n_obs - 1) as f64 } else { 0.0 };
    let volatility = variance.max(0.0).sqrt();
    let sharpe = if volatility > 0.0 { mean / volatility } else { 0.0 };
    let downside_dev = (sum_neg_sq / denom).max(0.0).sqrt();
    let sortino = if downside_dev > 0.0 {
        mean / downside_dev
    } else if mean > 0.0 {
        1.0e6
    } else if mean < 0.0 {
        -1.0e6
    } else {
        0.0
    }
    .clamp(-1.0e6, 1.0e6);
    let omega = if sum_neg_abs_r > 1.0e-12 {
        (sum_pos_r / sum_neg_abs_r).clamp(0.0, 1.0e6)
    } else if sum_pos_r > 0.0 {
        1.0e6
    } else {
        0.0
    };
    let calmar = if max_dd > 1.0e-12 {
        pnl / max_dd
    } else if pnl > 0.0 {
        1.0e6
    } else if pnl < 0.0 {
        -1.0e6
    } else {
        0.0
    }
    .clamp(-1.0e6, 1.0e6);

    Ok(Metrics {
        pnl,
        sharpe,
        sortino,
        calmar,
        omega,
        volatility,
        max_dd,
        trades,
        exposure,
        net_exposure,
    })
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
        build_static_strategy_ir, ConditionGroup, ConditionGroupMode, ConditionNode, ConditionOp,
        ExecutionPolicy, IndicatorNode, SignalBindings, SignalProgramGraph, StaticStrategyIrInput,
        StrategyIr, ValueRef,
        STRATEGY_IR_SCHEMA_VERSION,
    };
    use crate::MovingAverageSpec;
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
        
            dynamic_strategy_ir: None,
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
    fn dynamic_double_ma_cpu_metrics_support_constant_and_close_operands() {
        let candles = make_trending_candles(1024);
        let strategy = StrategyConfig::default();
        let ir = StrategyIr {
            schema_version: crate::strategy_ir::STRATEGY_IR_SCHEMA_VERSION,
            profile: StrategyProfileKind::DoubleMa,
            ma_source: "close".to_string(),
            fast_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: HashMap::new(),
            },
            slow_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: HashMap::new(),
            },
            program: SignalProgramGraph {
                indicators: vec![
                    IndicatorNode {
                        id: "fast_ma".to_string(),
                        kind: Some(crate::strategy_ir::IndicatorKind::FastMa),
                        indicator_id: None,
                        params: Vec::new(),
                        output_id: None,
                    },
                    IndicatorNode {
                        id: "slow_ma".to_string(),
                        kind: Some(crate::strategy_ir::IndicatorKind::SlowMa),
                        indicator_id: None,
                        params: Vec::new(),
                        output_id: None,
                    },
                ],
                conditions: vec![
                    ConditionNode {
                        id: "long_entry".to_string(),
                        op: ConditionOp::Gt,
                        lhs: ValueRef::Indicator {
                            id: "fast_ma".to_string(),
                        },
                        rhs: ValueRef::constant(0.0),
                    },
                    ConditionNode {
                        id: "long_exit".to_string(),
                        op: ConditionOp::Lt,
                        lhs: ValueRef::Close,
                        rhs: ValueRef::constant(0.0),
                    },
                ],
                groups: vec![
                    ConditionGroup {
                        id: "long_entry_group".to_string(),
                        mode: ConditionGroupMode::All,
                        condition_ids: vec!["long_entry".to_string()],
                    },
                    ConditionGroup {
                        id: "long_exit_group".to_string(),
                        mode: ConditionGroupMode::All,
                        condition_ids: vec!["long_exit".to_string()],
                    },
                ],
                signals: SignalBindings {
                    long_entry_group: Some("long_entry_group".to_string()),
                    long_exit_group: Some("long_exit_group".to_string()),
                    short_entry_group: None,
                    short_exit_group: None,
                },
            },
            execution: ExecutionPolicy {
                strategy: strategy.clone(),
                halftrend_filter: None,
                halftrend: None,
                macz: None,
                ott: None,
                signal_mode: None,
            },
        };
        let plan = crate::strategy_plan::compile_strategy_plan(&ir, StrategyPlanBackend::Cpu, None)
            .expect("plan should compile");
        let context =
            prepare_dynamic_cpu_context(&plan, &candles, "close").expect("context should prepare");
        let (fast_mat, cols_fast) =
            ma_matrix_cpu_f64(&candles, "sma", "close", &[10_u16], None, 0.85, 6.0)
                .expect("fast matrix");
        let (slow_mat, cols_slow) =
            ma_matrix_cpu_f64(&candles, "sma", "close", &[30_u16], None, 0.85, 6.0)
                .expect("slow matrix");
        assert_eq!(cols_fast, cols_slow);

        let dynamic_metrics =
            eval_dynamic_plan_metrics_cpu(&plan, &context, &fast_mat[..cols_fast], &slow_mat[..cols_fast], 64, true)
                .expect("dynamic eval should succeed");

        assert_eq!(dynamic_metrics.trades, 1);
        assert!(dynamic_metrics.pnl.is_finite());
        assert!(dynamic_metrics.pnl > 0.0);
        assert!(dynamic_metrics.exposure > 0.8);
    }

    #[test]
    fn dynamic_composite_cpu_metrics_match_static_eval() {
        let candles = make_trending_candles(4096);
        let req = CompositeCpuRequest {
            backend: crate::double_ma::Backend::CpuOnly,
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
    fn validate_dynamic_indicator_output_for_gpu_v1_rejects_non_float_series() {
        let output = DynamicIndicatorComputeOutput {
            output_id: "signal".to_string(),
            series: crate::dynamic_indicator_adapter::DynamicIndicatorSeries::Bool(vec![
                true, false,
            ]),
            warmup: None,
            rows: 1,
            cols: 2,
            pattern_ids: None,
        };
        let err = validate_dynamic_indicator_output_for_gpu_v1("bool_slot", &output)
            .expect_err("bool output should be rejected");
        assert!(err.contains("floating scalar outputs"));
    }

    #[test]
    fn validate_dynamic_indicator_output_for_gpu_v1_rejects_multi_row_output() {
        let output = DynamicIndicatorComputeOutput {
            output_id: "value".to_string(),
            series: crate::dynamic_indicator_adapter::DynamicIndicatorSeries::F64(vec![
                1.0, 2.0, 3.0, 4.0,
            ]),
            warmup: None,
            rows: 2,
            cols: 2,
            pattern_ids: None,
        };
        let err = validate_dynamic_indicator_output_for_gpu_v1("matrix_slot", &output)
            .expect_err("multi-row output should be rejected");
        assert!(err.contains("single-series scalar outputs"));
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
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            vcet: None,
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
        
            dynamic_strategy_ir: None,
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
        
            dynamic_strategy_ir: None,
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

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_keeps_gpu_backend_for_plain_v1_shape() {
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");

        let preview = preview_dynamic_double_ma_strategy_ir(
            &ir,
            StrategyPlanBackend::GpuUnified,
            ObjectiveKind::Sharpe,
            32,
            false,
        )
        .expect("preview should succeed");

        assert!(preview.capability.gpu_dynamic_supported);
        assert_eq!(preview.resolved_backend, StrategyPlanBackend::GpuUnified);
        assert!(!preview.plan_fingerprint_hex.is_empty());
    }

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_halftrend_shape() {
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                Some(HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 14,
                }),
            ),
        )
        .expect("static ir should build");

        let preview = preview_dynamic_double_ma_strategy_ir(
            &ir,
            StrategyPlanBackend::GpuUnified,
            ObjectiveKind::Sharpe,
            32,
            false,
        )
        .expect("preview should still succeed via cpu fallback");

        assert!(!preview.capability.gpu_dynamic_supported);
        assert_eq!(preview.resolved_backend, StrategyPlanBackend::Cpu);
        assert!(preview
            .capability
            .reasons
            .iter()
            .any(|reason| reason.contains("halftrend")));
    }

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_non_cross_exit_mode() {
        let mut strategy = StrategyConfig::default();
        strategy.exit_mode = ta_strategies::double_ma::ExitMode::KaufmanStop;
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                strategy,
                None,
            ),
        )
        .expect("static ir should build");

        let preview = preview_dynamic_double_ma_strategy_ir(
            &ir,
            StrategyPlanBackend::GpuUnified,
            ObjectiveKind::Sharpe,
            32,
            false,
        )
        .expect("preview should still succeed via cpu fallback");

        assert!(!preview.capability.gpu_dynamic_supported);
        assert_eq!(preview.resolved_backend, StrategyPlanBackend::Cpu);
        assert!(preview
            .capability
            .reasons
            .iter()
            .any(|reason| reason.contains("kaufman-stop exit")));
    }

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_rejects_non_float_dynamic_output() {
        use crate::indicator_catalog::{list_dynamic_indicator_descriptors_cached, IndicatorValueType};
        use crate::strategy_ir::{
            ConditionGroup, ConditionGroupMode, ConditionNode, ExecutionPolicy, MovingAverageSpec,
            SignalBindings, SignalProgramGraph, StrategyIr, STRATEGY_IR_SCHEMA_VERSION,
        };

        let Some((indicator_id, output_id)) = list_dynamic_indicator_descriptors_cached()
            .iter()
            .find_map(|descriptor| {
                descriptor.outputs.iter().find_map(|output| {
                    if matches!(output.value_type, IndicatorValueType::I32 | IndicatorValueType::Bool)
                    {
                        Some((descriptor.id.clone(), output.id.clone()))
                    } else {
                        None
                    }
                })
            })
        else {
            return;
        };

        let ir = StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION,
            profile: StrategyProfileKind::DoubleMa,
            ma_source: "close".to_string(),
            fast_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: HashMap::new(),
            },
            slow_ma: MovingAverageSpec {
                ma_type: "alma".to_string(),
                params: HashMap::new(),
            },
            program: SignalProgramGraph {
                indicators: vec![IndicatorNode {
                    id: "non_float".to_string(),
                    kind: None,
                    indicator_id: Some(indicator_id.clone()),
                    params: Vec::new(),
                    output_id: Some(output_id.clone()),
                }],
                conditions: vec![ConditionNode {
                    id: "entry".to_string(),
                    op: ConditionOp::Gt,
                    lhs: ValueRef::Indicator {
                        id: "non_float".to_string(),
                    },
                    rhs: ValueRef::constant(0.0),
                }],
                groups: vec![ConditionGroup {
                    id: "entry_group".to_string(),
                    mode: ConditionGroupMode::All,
                    condition_ids: vec!["entry".to_string()],
                }],
                signals: SignalBindings {
                    long_entry_group: Some("entry_group".to_string()),
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

        let err = preview_dynamic_double_ma_strategy_ir(
            &ir,
            StrategyPlanBackend::GpuUnified,
            ObjectiveKind::Sharpe,
            32,
            false,
        )
        .expect_err("preview should reject unsupported non-float output refs");
        assert!(err.contains("floating scalar outputs"));
    }

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_rejects_product_cap_overflow() {
        let mut ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("static ir should build");

        for idx in 0..13 {
            ir.program.indicators.push(IndicatorNode {
                id: format!("extra_{idx}"),
                kind: None,
                indicator_id: Some("sma".to_string()),
                params: vec![crate::indicator_catalog::IndicatorParamKV {
                    key: "period".to_string(),
                    value: crate::indicator_catalog::IndicatorParamValue::Int(5 + idx as i64),
                }],
                output_id: Some("value".to_string()),
            });
        }

        let err = preview_dynamic_double_ma_strategy_ir(
            &ir,
            StrategyPlanBackend::GpuUnified,
            ObjectiveKind::Sharpe,
            32,
            false,
        )
        .expect_err("product cap overflow should fail preview");
        assert!(err.contains("at most 12 indicators"), "unexpected error: {err}");
    }

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_rejects_bidirectional_missing_short_entry() {
        let mut ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig {
                    long_only: false,
                    ..StrategyConfig::default()
                },
                None,
            ),
        )
        .expect("static ir should build");
        ir.program.signals.short_entry_group = None;

        let err = preview_dynamic_double_ma_strategy_ir(
            &ir,
            StrategyPlanBackend::GpuUnified,
            ObjectiveKind::Sharpe,
            32,
            false,
        )
        .expect_err("bidirectional missing short entry should fail preview");
        assert!(err.contains("bidirectional plans must bind short_entry_group"), "unexpected error: {err}");
    }
}
