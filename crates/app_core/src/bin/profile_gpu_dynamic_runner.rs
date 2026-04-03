use std::sync::atomic::AtomicBool;
use std::time::Instant;

use serde::Serialize;
use serde_json::json;
use ta_app_core::double_ma::{DoubleMaMetricContract, DoubleMaShortlistSliceMode};
use ta_app_core::strategy_presets::{
    breakout_strategy_for_kernel, regime_hybrid_strategy_for_kernel,
    trend_quality_strategy_for_kernel,
};
use ta_app_core::SpaBenchmarkKind;
use ta_app_core::{
    run_double_ma_optimization_blocking_with_candles_and_eval_range, Backend, BackendUsed,
    DoubleMaRequest, StrategyExecutableCacheStats,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy)]
enum StrategyKind {
    DoubleMa,
    TrendQuality,
    RegimeHybrid,
    BreakoutMomentum,
}

#[derive(Clone, Copy)]
enum WorkloadKind {
    Small,
    Medium,
    Large,
    Xlarge,
}

#[derive(Clone, Copy)]
enum AdxMode {
    Off,
    On,
}

#[derive(Clone, Copy)]
enum CandleShape {
    Auto,
    Synth,
    Trending,
    Choppy,
    Breakout,
}

#[derive(Clone, Copy)]
enum ToggleMode {
    Inherit,
    Off,
    On,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct ExecutableCacheStatsWindow {
    before: StrategyExecutableCacheStats,
    after: StrategyExecutableCacheStats,
    delta: StrategyExecutableCacheStats,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct PreparedDeviceDispatchStats {
    device_path_hits: u64,
    device_path_fallbacks: u64,
}

fn parse_arg_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == flag {
            return args.next();
        }
    }
    None
}

fn parse_strategy_kind() -> Result<StrategyKind, String> {
    let raw = parse_arg_value("--strategy").unwrap_or_else(|| "breakout_momentum".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "double_ma" => Ok(StrategyKind::DoubleMa),
        "trend_quality" => Ok(StrategyKind::TrendQuality),
        "regime_hybrid" => Ok(StrategyKind::RegimeHybrid),
        "breakout_momentum" => Ok(StrategyKind::BreakoutMomentum),
        other => Err(format!(
            "unsupported --strategy '{other}' (expected: double_ma|trend_quality|regime_hybrid|breakout_momentum)"
        )),
    }
}

fn parse_workload_kind() -> Result<WorkloadKind, String> {
    let raw = parse_arg_value("--workload").unwrap_or_else(|| "xlarge".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "small" => Ok(WorkloadKind::Small),
        "medium" => Ok(WorkloadKind::Medium),
        "large" => Ok(WorkloadKind::Large),
        "xlarge" => Ok(WorkloadKind::Xlarge),
        other => Err(format!(
            "unsupported --workload '{other}' (expected: small|medium|large|xlarge)"
        )),
    }
}

fn parse_ma_type(flag: &str, default: &str) -> String {
    parse_arg_value(flag).unwrap_or_else(|| default.to_string())
}

fn parse_iterations() -> Result<usize, String> {
    let raw = parse_arg_value("--measure-iterations")
        .or_else(|| parse_arg_value("--iterations"))
        .unwrap_or_else(|| "1".to_string());
    let v = raw
        .trim()
        .parse::<usize>()
        .map_err(|e| format!("invalid --iterations '{}': {e}", raw.trim()))?;
    if v == 0 {
        return Err("--iterations must be >= 1".to_string());
    }
    Ok(v)
}

fn parse_warmup_iterations() -> Result<usize, String> {
    let raw = parse_arg_value("--warmup-iterations").unwrap_or_else(|| "0".to_string());
    raw.trim()
        .parse::<usize>()
        .map_err(|e| format!("invalid --warmup-iterations '{}': {e}", raw.trim()))
}

fn parse_heatmap_bins() -> Result<u16, String> {
    let raw = parse_arg_value("--heatmap-bins").unwrap_or_else(|| "64".to_string());
    raw.trim()
        .parse::<u16>()
        .map_err(|e| format!("invalid --heatmap-bins '{}': {e}", raw.trim()))
}

fn parse_optional_usize_flag(flag: &str) -> Result<Option<usize>, String> {
    let Some(raw) = parse_arg_value(flag) else {
        return Ok(None);
    };
    raw.trim()
        .parse::<usize>()
        .map(Some)
        .map_err(|e| format!("invalid {flag} '{}': {e}", raw.trim()))
}

fn parse_optional_f64_flag(flag: &str) -> Result<Option<f64>, String> {
    let Some(raw) = parse_arg_value(flag) else {
        return Ok(None);
    };
    raw.trim()
        .parse::<f64>()
        .map(Some)
        .map_err(|e| format!("invalid {flag} '{}': {e}", raw.trim()))
}

fn parse_eval_range(total_bars: usize) -> Result<Option<(usize, usize)>, String> {
    let eval_start = parse_optional_usize_flag("--eval-start")?;
    let eval_end = parse_optional_usize_flag("--eval-end")?;
    match (eval_start, eval_end) {
        (None, None) => Ok(None),
        (Some(start), Some(end)) => {
            if start >= end {
                return Err(format!(
                    "--eval-start must be < --eval-end, got [{start}, {end})"
                ));
            }
            if end > total_bars {
                return Err(format!(
                    "--eval-end {end} exceeds candle length {total_bars}"
                ));
            }
            Ok(Some((start, end)))
        }
        _ => Err("--eval-start and --eval-end must be provided together".to_string()),
    }
}

fn parse_metric_contract() -> Result<Option<DoubleMaMetricContract>, String> {
    let Some(raw) = parse_arg_value("--metric-contract") else {
        return Ok(None);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" | "none" => Ok(None),
        "full17" | "full_17" | "full-17" => Ok(Some(DoubleMaMetricContract::Full17)),
        "trade22" | "trade_22" | "trade-22" => Ok(Some(DoubleMaMetricContract::Trade22)),
        other => Err(format!(
            "unsupported --metric-contract '{other}' (expected: auto|full17|trade22)"
        )),
    }
}

fn parse_shortlist_slice_mode() -> Result<Option<DoubleMaShortlistSliceMode>, String> {
    let Some(raw) = parse_arg_value("--shortlist-slice-mode") else {
        return Ok(None);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(None),
        "halves" => Ok(Some(DoubleMaShortlistSliceMode::Halves)),
        "quartiles" => Ok(Some(DoubleMaShortlistSliceMode::Quartiles)),
        other => Err(format!(
            "unsupported --shortlist-slice-mode '{other}' (expected: none|halves|quartiles)"
        )),
    }
}

fn parse_shortlist_slice_benchmark() -> Result<Option<SpaBenchmarkKind>, String> {
    let Some(raw) = parse_arg_value("--shortlist-slice-benchmark") else {
        return Ok(None);
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(None),
        "flat" => Ok(Some(SpaBenchmarkKind::Flat)),
        "buy_and_hold" | "buy-and-hold" | "buyandhold" => Ok(Some(SpaBenchmarkKind::BuyAndHold)),
        other => Err(format!(
            "unsupported --shortlist-slice-benchmark '{other}' (expected: none|flat|buy_and_hold)"
        )),
    }
}

fn parse_objective_kind() -> Result<ObjectiveKind, String> {
    let raw = parse_arg_value("--objective").unwrap_or_else(|| "sharpe".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "sharpe" => Ok(ObjectiveKind::Sharpe),
        "pnl" => Ok(ObjectiveKind::Pnl),
        "sortino" => Ok(ObjectiveKind::Sortino),
        "omega" => Ok(ObjectiveKind::Omega),
        "calmar" => Ok(ObjectiveKind::Calmar),
        "max_drawdown" | "maxdd" | "drawdown" => Ok(ObjectiveKind::MaxDrawdown),
        "volatility" | "vol" => Ok(ObjectiveKind::Volatility),
        other => Err(format!(
            "unsupported --objective '{other}' (expected: sharpe|pnl|sortino|omega|calmar|max_drawdown|volatility)"
        )),
    }
}

fn parse_dynamic_mode() -> Result<bool, String> {
    let raw = parse_arg_value("--dynamic-mode").unwrap_or_else(|| "off".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!(
            "unsupported --dynamic-mode '{other}' (expected: on|off)"
        )),
    }
}

fn parse_toggle_mode(flag: &str, default: &str) -> Result<ToggleMode, String> {
    let raw = parse_arg_value(flag).unwrap_or_else(|| default.to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "inherit" => Ok(ToggleMode::Inherit),
        "off" => Ok(ToggleMode::Off),
        "on" => Ok(ToggleMode::On),
        other => Err(format!(
            "unsupported {flag} '{other}' (expected: inherit|off|on)"
        )),
    }
}

fn parse_toggle_mode_alias(
    primary: &str,
    secondary: &str,
    default: &str,
) -> Result<ToggleMode, String> {
    let raw = parse_arg_value(primary)
        .or_else(|| parse_arg_value(secondary))
        .unwrap_or_else(|| default.to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "inherit" => Ok(ToggleMode::Inherit),
        "off" => Ok(ToggleMode::Off),
        "on" => Ok(ToggleMode::On),
        other => Err(format!(
            "unsupported {primary}/{secondary} '{other}' (expected: inherit|off|on)"
        )),
    }
}

fn parse_emit_json() -> bool {
    std::env::args().skip(1).any(|arg| arg == "--emit-json")
}

fn metric_contract_label(metric_contract: Option<DoubleMaMetricContract>) -> &'static str {
    match metric_contract {
        Some(DoubleMaMetricContract::Full17) => "full17",
        Some(DoubleMaMetricContract::Trade22) => "trade22",
        None => "auto",
    }
}

fn shortlist_slice_mode_label(mode: Option<DoubleMaShortlistSliceMode>) -> &'static str {
    match mode {
        Some(DoubleMaShortlistSliceMode::Halves) => "halves",
        Some(DoubleMaShortlistSliceMode::Quartiles) => "quartiles",
        None => "none",
    }
}

fn shortlist_slice_benchmark_label(benchmark: Option<&SpaBenchmarkKind>) -> &'static str {
    match benchmark {
        Some(SpaBenchmarkKind::Flat) => "flat",
        Some(SpaBenchmarkKind::BuyAndHold) => "buy_and_hold",
        None => "none",
    }
}

fn parse_candle_shape() -> Result<CandleShape, String> {
    let raw = parse_arg_value("--candle-shape").unwrap_or_else(|| "auto".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(CandleShape::Auto),
        "synth" => Ok(CandleShape::Synth),
        "trend" | "trending" => Ok(CandleShape::Trending),
        "choppy" => Ok(CandleShape::Choppy),
        "breakout" => Ok(CandleShape::Breakout),
        other => Err(format!(
            "unsupported --candle-shape '{other}' (expected: auto|synth|trending|choppy|breakout)"
        )),
    }
}

fn parse_device_id() -> Result<u32, String> {
    let raw = parse_arg_value("--device-id").unwrap_or_else(|| "0".to_string());
    raw.trim()
        .parse::<u32>()
        .map_err(|e| format!("invalid --device-id '{}': {e}", raw.trim()))
}

fn parse_adx_mode() -> Result<AdxMode, String> {
    let raw = parse_arg_value("--adx-mode").unwrap_or_else(|| "off".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => Ok(AdxMode::Off),
        "on" => Ok(AdxMode::On),
        other => Err(format!(
            "unsupported --adx-mode '{other}' (expected: on|off)"
        )),
    }
}

fn apply_env_toggle(var_name: &str, mode: ToggleMode) {
    match mode {
        ToggleMode::Inherit => {}
        ToggleMode::Off => std::env::set_var(var_name, "0"),
        ToggleMode::On => std::env::set_var(var_name, "1"),
    }
}

fn apply_nvtx_phase(mode: ToggleMode, is_warmup: bool) {
    match mode {
        ToggleMode::Inherit => {}
        ToggleMode::Off => std::env::set_var("VECTORBT_ENABLE_NVTX", "0"),
        ToggleMode::On => {
            std::env::set_var("VECTORBT_ENABLE_NVTX", if is_warmup { "0" } else { "1" })
        }
    }
}

fn format_optional_ms(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn executable_cache_stats_delta(
    before: StrategyExecutableCacheStats,
    after: StrategyExecutableCacheStats,
) -> StrategyExecutableCacheStats {
    StrategyExecutableCacheStats {
        compiled_bundle_hits: after
            .compiled_bundle_hits
            .saturating_sub(before.compiled_bundle_hits),
        compiled_bundle_misses: after
            .compiled_bundle_misses
            .saturating_sub(before.compiled_bundle_misses),
        compiled_bundle_entries: after
            .compiled_bundle_entries
            .saturating_sub(before.compiled_bundle_entries),
        hits: after.hits.saturating_sub(before.hits),
        misses: after.misses.saturating_sub(before.misses),
        entries: after.entries.saturating_sub(before.entries),
        uploaded_thread_entries: after
            .uploaded_thread_entries
            .saturating_sub(before.uploaded_thread_entries),
    }
}

fn executable_cache_stats_window(
    before: StrategyExecutableCacheStats,
    after: StrategyExecutableCacheStats,
) -> ExecutableCacheStatsWindow {
    ExecutableCacheStatsWindow {
        before,
        after,
        delta: executable_cache_stats_delta(before, after),
    }
}

#[cfg(feature = "cuda")]
fn drain_prepared_device_dispatch_stats() -> PreparedDeviceDispatchStats {
    let stats = ta_app_core::dynamic_indicator_adapter::take_prepared_cuda_dispatch_stats();
    PreparedDeviceDispatchStats {
        device_path_hits: stats.device_path_hits,
        device_path_fallbacks: stats.device_path_fallbacks,
    }
}

#[cfg(not(feature = "cuda"))]
fn drain_prepared_device_dispatch_stats() -> PreparedDeviceDispatchStats {
    PreparedDeviceDispatchStats::default()
}

fn make_trending_candles(len: usize) -> Candles {
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

    Candles::new(ts, open, high, low, close, vol)
}

fn make_synth_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);
        let drift = 0.00005;
        let noise = (i as f64 * 0.017).sin() * 0.001;
        px *= 1.0 + drift + noise;

        open.push(px);
        high.push(px * 1.001);
        low.push(px * 0.999);
        close.push(px);
        vol.push(1_000.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_choppy_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    for i in 0..len {
        ts.push(i as i64);
        let base = 100.0 + ((i as f64) / 9.0).sin() * 2.0 + ((i as f64) / 37.0).sin() * 0.7;
        let c = base + if (i & 1) == 0 { 0.15 } else { -0.15 };
        let o = base;
        open.push(o);
        close.push(c);
        high.push(o.max(c) + 0.45);
        low.push(o.min(c) - 0.45);
        vol.push(1_000.0 + (i % 5) as f64 * 25.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn make_breakout_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let up_start = len / 3;
    let down_start = (len * 2) / 3;
    for i in 0..len {
        ts.push(i as i64);
        let base = if i < up_start {
            100.0 + i as f64 * 0.02
        } else if i < down_start {
            102.0 + (i - up_start) as f64 * 0.35
        } else {
            102.0 + (down_start - up_start) as f64 * 0.35 - (i - down_start) as f64 * 0.38
        };
        let n = ((i as f64) / 7.0).sin() * 0.08;
        let o = base + n;
        let c = if i < down_start {
            base + 0.30 + n
        } else {
            base - 0.30 + n
        };
        open.push(o);
        close.push(c);
        high.push(o.max(c) + 0.22);
        low.push(o.min(c) - 0.22);
        let burst = if i < up_start {
            950.0
        } else if i < down_start {
            2_200.0
        } else {
            2_450.0
        };
        vol.push(burst + ((i % 11) as f64) * 35.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn workload_ranges(kind: WorkloadKind) -> ((u32, u32, u32), (u32, u32, u32)) {
    match kind {
        WorkloadKind::Small => ((5, 50, 1), (20, 200, 5)),
        WorkloadKind::Medium => ((5, 100, 1), (10, 200, 1)),
        WorkloadKind::Large => ((5, 200, 1), (10, 400, 1)),
        WorkloadKind::Xlarge => ((5, 500, 1), (10, 1000, 1)),
    }
}

fn strategy_config(kind: StrategyKind, adx_mode: AdxMode) -> StrategyConfig {
    match kind {
        StrategyKind::DoubleMa => StrategyConfig::default(),
        StrategyKind::TrendQuality => {
            let mut s = trend_quality_strategy_for_kernel();
            s.trend_quality_use_adx = matches!(adx_mode, AdxMode::On);
            s
        }
        StrategyKind::RegimeHybrid => regime_hybrid_strategy_for_kernel(),
        StrategyKind::BreakoutMomentum => {
            let mut s = breakout_strategy_for_kernel();
            s.breakout_use_adx = matches!(adx_mode, AdxMode::On);
            s
        }
    }
}

fn apply_risk_overrides(
    strategy: &mut StrategyConfig,
    stop_loss_pct: Option<f64>,
    take_profit_pct: Option<f64>,
) {
    if let Some(stop_loss_pct) = stop_loss_pct {
        strategy.stop_loss_pct = Some(stop_loss_pct);
    }
    if let Some(take_profit_pct) = take_profit_pct {
        strategy.take_profit_pct = Some(take_profit_pct);
    }
}

fn candles_for_strategy(kind: StrategyKind, len: usize) -> Candles {
    match kind {
        StrategyKind::DoubleMa | StrategyKind::TrendQuality => make_trending_candles(len),
        StrategyKind::RegimeHybrid => make_choppy_candles(len),
        StrategyKind::BreakoutMomentum => make_breakout_candles(len),
    }
}

fn candles_for_run(kind: StrategyKind, shape: CandleShape, len: usize) -> Candles {
    match shape {
        CandleShape::Auto => candles_for_strategy(kind, len),
        CandleShape::Synth => make_synth_candles(len),
        CandleShape::Trending => make_trending_candles(len),
        CandleShape::Choppy => make_choppy_candles(len),
        CandleShape::Breakout => make_breakout_candles(len),
    }
}

fn backend_for_strategy(kind: StrategyKind, adx_mode: AdxMode, device_id: u32) -> Backend {
    match kind {
        StrategyKind::TrendQuality if matches!(adx_mode, AdxMode::On) => {
            Backend::GpuHybridKernel { device_id }
        }
        StrategyKind::BreakoutMomentum if matches!(adx_mode, AdxMode::On) => {
            Backend::GpuHybridKernel { device_id }
        }
        _ => Backend::GpuKernel { device_id },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        backend_for_strategy, executable_cache_stats_window, AdxMode, Backend,
        PreparedDeviceDispatchStats, StrategyExecutableCacheStats, StrategyKind,
    };
    use serde_json::json;

    #[test]
    fn backend_for_strategy_keeps_regime_hybrid_on_exact_gpu_kernel_surface() {
        assert!(matches!(
            backend_for_strategy(StrategyKind::RegimeHybrid, AdxMode::Off, 7),
            Backend::GpuKernel { device_id: 7 }
        ));
    }

    #[test]
    fn backend_for_strategy_keeps_adx_bearing_profiles_on_hybrid_route() {
        assert!(matches!(
            backend_for_strategy(StrategyKind::TrendQuality, AdxMode::On, 3),
            Backend::GpuHybridKernel { device_id: 3 }
        ));
        assert!(matches!(
            backend_for_strategy(StrategyKind::BreakoutMomentum, AdxMode::On, 5),
            Backend::GpuHybridKernel { device_id: 5 }
        ));
    }

    #[test]
    fn executable_cache_stats_window_reports_saturating_delta() {
        let before = StrategyExecutableCacheStats {
            compiled_bundle_hits: 2,
            compiled_bundle_misses: 1,
            compiled_bundle_entries: 1,
            hits: 4,
            misses: 2,
            entries: 3,
            uploaded_thread_entries: 1,
        };
        let after = StrategyExecutableCacheStats {
            compiled_bundle_hits: 6,
            compiled_bundle_misses: 3,
            compiled_bundle_entries: 2,
            hits: 9,
            misses: 3,
            entries: 5,
            uploaded_thread_entries: 2,
        };
        let window = executable_cache_stats_window(before, after);

        assert_eq!(window.before, before);
        assert_eq!(window.after, after);
        assert_eq!(
            window.delta,
            StrategyExecutableCacheStats {
                compiled_bundle_hits: 4,
                compiled_bundle_misses: 2,
                compiled_bundle_entries: 1,
                hits: 5,
                misses: 1,
                entries: 2,
                uploaded_thread_entries: 1,
            }
        );
    }

    #[test]
    fn stats_artifacts_serialize_with_expected_json_keys() {
        let payload = json!({
            "gpu_strategy_executable_cache_stats": executable_cache_stats_window(
                StrategyExecutableCacheStats {
                    compiled_bundle_hits: 10,
                    compiled_bundle_misses: 4,
                    compiled_bundle_entries: 2,
                    hits: 1,
                    misses: 2,
                    entries: 3,
                    uploaded_thread_entries: 1,
                },
                StrategyExecutableCacheStats {
                    compiled_bundle_hits: 12,
                    compiled_bundle_misses: 5,
                    compiled_bundle_entries: 3,
                    hits: 5,
                    misses: 3,
                    entries: 4,
                    uploaded_thread_entries: 2,
                },
            ),
            "prepared_device_dispatch_stats": PreparedDeviceDispatchStats {
                device_path_hits: 7,
                device_path_fallbacks: 1,
            },
        });

        assert_eq!(
            payload["gpu_strategy_executable_cache_stats"]["before"]["hits"],
            1
        );
        assert_eq!(
            payload["gpu_strategy_executable_cache_stats"]["before"]["compiled_bundle_hits"],
            10
        );
        assert_eq!(
            payload["gpu_strategy_executable_cache_stats"]["after"]["misses"],
            3
        );
        assert_eq!(
            payload["gpu_strategy_executable_cache_stats"]["delta"]["entries"],
            1
        );
        assert_eq!(
            payload["gpu_strategy_executable_cache_stats"]["delta"]["uploaded_thread_entries"],
            1
        );
        assert_eq!(
            payload["prepared_device_dispatch_stats"]["device_path_hits"],
            7
        );
        assert_eq!(
            payload["prepared_device_dispatch_stats"]["device_path_fallbacks"],
            1
        );
    }
}

#[cfg(feature = "cuda")]
fn cuda_available() -> bool {
    vector_ta::cuda::cuda_available()
}

#[cfg(not(feature = "cuda"))]
fn cuda_available() -> bool {
    false
}

fn main() {
    let strategy_kind = parse_strategy_kind().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let workload_kind = parse_workload_kind().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let iterations = parse_iterations().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let warmup_iterations = parse_warmup_iterations().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let heatmap_bins = parse_heatmap_bins().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let metric_contract = parse_metric_contract().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let shortlist_slice_mode = parse_shortlist_slice_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let shortlist_slice_benchmark = parse_shortlist_slice_benchmark().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let objective = parse_objective_kind().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let include_trade_metrics_mode = parse_toggle_mode("--include-trade-metrics", "inherit")
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
    let dynamic_enabled = parse_dynamic_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let cuda_graphs_mode = parse_toggle_mode_alias("--cuda-graphs", "--graphs", "inherit")
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2);
        });
    let nvtx_mode = parse_toggle_mode("--nvtx", "inherit").unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let device_id = parse_device_id().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let fast_ma_type = parse_ma_type("--fast-ma", "sma");
    let slow_ma_type = parse_ma_type("--slow-ma", "sma");
    let adx_mode = parse_adx_mode().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let candle_shape = parse_candle_shape().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let emit_json = parse_emit_json();
    let stop_loss_pct = parse_optional_f64_flag("--stop-loss-pct").unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let take_profit_pct = parse_optional_f64_flag("--take-profit-pct").unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });

    if !cuda_available() {
        eprintln!("CUDA device not available");
        std::process::exit(1);
    }

    if dynamic_enabled {
        std::env::set_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", "1");
    } else {
        std::env::remove_var("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL");
    }
    apply_env_toggle("VECTORBT_KERNEL_USE_CUDA_GRAPHS", cuda_graphs_mode);
    apply_nvtx_phase(nvtx_mode, warmup_iterations > 0);

    let mut strategy = strategy_config(strategy_kind, adx_mode);
    apply_risk_overrides(&mut strategy, stop_loss_pct, take_profit_pct);
    let candles = candles_for_run(strategy_kind, candle_shape, 200_000);
    let eval_range = parse_eval_range(candles.close.len()).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2);
    });
    let (fast_range, slow_range) = workload_ranges(workload_kind);
    let cancel = AtomicBool::new(false);
    let include_trade_metrics = match include_trade_metrics_mode {
        ToggleMode::On => true,
        ToggleMode::Off => false,
        ToggleMode::Inherit => matches!(metric_contract, Some(DoubleMaMetricContract::Trade22)),
    };
    if matches!(metric_contract, Some(DoubleMaMetricContract::Full17)) && include_trade_metrics {
        eprintln!("--metric-contract full17 cannot be combined with --include-trade-metrics on");
        std::process::exit(2);
    }
    if matches!(metric_contract, Some(DoubleMaMetricContract::Trade22)) && !include_trade_metrics {
        eprintln!(
            "--metric-contract trade22 requires trade metrics; omit --include-trade-metrics or set it to on"
        );
        std::process::exit(2);
    }
    if shortlist_slice_benchmark.is_some() && shortlist_slice_mode.is_none() {
        eprintln!("--shortlist-slice-benchmark requires --shortlist-slice-mode");
        std::process::exit(2);
    }

    let backend = backend_for_strategy(strategy_kind, adx_mode, device_id);
    if shortlist_slice_mode.is_some() && !matches!(backend, Backend::GpuKernel { .. }) {
        eprintln!(
            "--shortlist-slice-mode currently requires an exact GpuKernel route; resolved backend was {:?}",
            backend
        );
        std::process::exit(2);
    }

    let req = DoubleMaRequest {
        backend,
        data_id: "profile-gpu-dynamic-runner".to_string(),
        fast_range,
        slow_range,
        fast_ma_types: vec![fast_ma_type.clone()],
        slow_ma_types: vec![slow_ma_type.clone()],
        ma_source: "close".to_string(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter: None,
        objective,
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode,
        shortlist_slice_benchmark: shortlist_slice_benchmark.clone(),
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(50),
        include_all: Some(false),
        heatmap_bins: Some(heatmap_bins),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics,
        metric_contract,

        hurdle_tau: None,
    };

    for i in 0..(warmup_iterations + iterations) {
        let is_warmup = i < warmup_iterations;
        let iter_index = if is_warmup {
            i + 1
        } else {
            i + 1 - warmup_iterations
        };
        apply_nvtx_phase(nvtx_mode, is_warmup);
        let executable_cache_before = if emit_json {
            Some(ta_app_core::gpu_strategy_executable_cache_stats())
        } else {
            None
        };
        if emit_json {
            let _ = drain_prepared_device_dispatch_stats();
        }
        let t0 = Instant::now();
        let res = run_double_ma_optimization_blocking_with_candles_and_eval_range(
            req.clone(),
            &candles,
            &cancel,
            None,
            eval_range,
        )
        .unwrap_or_else(|e| {
            eprintln!("run failed (iter {}): {}", i + 1, e);
            std::process::exit(1);
        });
        if !matches!(res.backend_used, BackendUsed::Gpu) {
            eprintln!(
                "unexpected backend_used={:?}; expected GPU",
                res.backend_used
            );
            std::process::exit(1);
        }
        let wall_ms = t0.elapsed().as_millis() as u64;
        if emit_json {
            let executable_cache_after = ta_app_core::gpu_strategy_executable_cache_stats();
            let prepared_device_dispatch_stats = drain_prepared_device_dispatch_stats();
            let payload = json!({
                "phase": if is_warmup { "warmup" } else { "measure" },
                "iter": iter_index,
                "backend_used": format!("{:?}", res.backend_used),
                "backend_route": &res.backend_route,
                "runtime_ms": res.runtime_ms,
                "wall_ms": wall_ms,
                "objective": format!("{:?}", objective),
                "metric_contract": metric_contract_label(metric_contract),
                "shortlist_slice_mode": shortlist_slice_mode_label(shortlist_slice_mode),
                "shortlist_slice_benchmark": shortlist_slice_benchmark_label(shortlist_slice_benchmark.as_ref()),
                "eval_range": eval_range.map(|(start, end)| json!({ "start": start, "end": end })),
                "include_trade_metrics": include_trade_metrics,
                "stop_loss_pct": stop_loss_pct,
                "take_profit_pct": take_profit_pct,
                "fast_ma": &fast_ma_type,
                "slow_ma": &slow_ma_type,
                "heatmap_bins": heatmap_bins,
                "cuda_graphs": match cuda_graphs_mode {
                    ToggleMode::Inherit => "inherit",
                    ToggleMode::Off => "off",
                    ToggleMode::On => "on",
                },
                "nvtx": match nvtx_mode {
                    ToggleMode::Inherit => "inherit",
                    ToggleMode::Off => "off",
                    ToggleMode::On => if is_warmup { "warmup_off" } else { "measure_on" },
                },
                "best_params": {
                    "fast_len": res.best_params.fast_len,
                    "slow_len": res.best_params.slow_len,
                },
                "best_metrics": {
                    "sharpe": res.best_metrics.sharpe,
                    "pnl": res.best_metrics.pnl,
                },
                "gpu_kernel_timing_ms": &res.gpu_kernel_timing_ms,
                "hybrid_timing_ms": &res.hybrid_timing_ms,
                "indicator_route_plan": &res.indicator_route_plan,
                "backend_fallback_reason": res.backend_fallback_reason.as_deref(),
                "route_diagnostics": &res.route_diagnostics,
                "gpu_strategy_executable_cache_stats": executable_cache_stats_window(
                    executable_cache_before.expect("json mode captures executable cache baseline"),
                    executable_cache_after,
                ),
                "prepared_device_dispatch_stats": prepared_device_dispatch_stats,
            });
            println!(
                "{}",
                serde_json::to_string(&payload).expect("serialize runner payload")
            );
            continue;
        }
        println!(
            "phase={} iter={} fast_ma={} slow_ma={} backend={:?} route={} objective={:?} metric_contract={} shortlist_slice_mode={} shortlist_slice_benchmark={} eval_range={} include_trade_metrics={} stop_loss_pct={} take_profit_pct={} runtime_ms={} wall_ms={} heatmap_bins={} cuda_graphs={} best=({}, {}) sharpe={:.6} pnl={:.6}",
            if is_warmup { "warmup" } else { "measure" },
            iter_index,
            fast_ma_type,
            slow_ma_type,
            res.backend_used,
            res.backend_route.as_deref().unwrap_or("<none>"),
            objective,
            metric_contract_label(metric_contract),
            shortlist_slice_mode_label(shortlist_slice_mode),
            shortlist_slice_benchmark_label(shortlist_slice_benchmark.as_ref()),
            eval_range
                .map(|(start, end)| format!("[{start}, {end})"))
                .unwrap_or_else(|| "full".to_string()),
            include_trade_metrics,
            stop_loss_pct
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "none".to_string()),
            take_profit_pct
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "none".to_string()),
            res.runtime_ms,
            wall_ms,
            heatmap_bins,
            match cuda_graphs_mode {
                ToggleMode::Inherit => "inherit",
                ToggleMode::Off => "off",
                ToggleMode::On => "on",
            },
            res.best_params.fast_len,
            res.best_params.slow_len,
            res.best_metrics.sharpe,
            res.best_metrics.pnl
        );
        if let Some(timing) = res.gpu_kernel_timing_ms.as_ref() {
            println!(
                "gpu_kernel_timing_ms=route_wall:{} runtime_init:{} set_current:{} gpu_indicator_prep:{} h2d_upload:{} launch_submit_us:{} d2h_copy:{} host_decode:{} explicit_sync:{} host_shortlist_materialize:{} host_finalize:{} host_finalize_agg_finalize:{} host_finalize_finalist_recompute:{} host_finalize_shortlist_union:{} host_finalize_shortlist_confidence:{} host_finalize_shortlist_slice:{} host_finalize_shortlist_slice_replay:{} host_finalize_shortlist_slice_record_assembly:{} host_finalize_shortlist_slice_sort:{} host_finalize_shortlist_slice_diagnostics:{} host_finalize_export_flush:{} host_finalize_cleanup:{} graph_capture:{} graph_replay_count:{}",
                timing.route_wall_ms,
                format_optional_ms(timing.runtime_init_ms),
                format_optional_ms(timing.set_current_ms),
                format_optional_ms(timing.gpu_indicator_prep_ms),
                format_optional_ms(timing.h2d_upload_ms),
                timing.launch_submit_us
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                format_optional_ms(timing.d2h_copy_ms),
                format_optional_ms(timing.host_decode_ms),
                format_optional_ms(timing.explicit_sync_ms),
                format_optional_ms(timing.host_shortlist_materialize_ms),
                format_optional_ms(timing.host_finalize_ms),
                format_optional_ms(timing.host_finalize_agg_finalize_ms),
                format_optional_ms(timing.host_finalize_finalist_recompute_ms),
                format_optional_ms(timing.host_finalize_shortlist_union_ms),
                format_optional_ms(timing.host_finalize_shortlist_confidence_ms),
                format_optional_ms(timing.host_finalize_shortlist_slice_ms),
                format_optional_ms(timing.host_finalize_shortlist_slice_replay_ms),
                format_optional_ms(timing.host_finalize_shortlist_slice_record_assembly_ms),
                format_optional_ms(timing.host_finalize_shortlist_slice_sort_ms),
                format_optional_ms(timing.host_finalize_shortlist_slice_diagnostics_ms),
                format_optional_ms(timing.host_finalize_export_flush_ms),
                format_optional_ms(timing.host_finalize_cleanup_ms),
                format_optional_ms(timing.graph_capture_ms),
                timing.graph_replay_count.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())
            );
        }
        if let Some(timing) = res.hybrid_timing_ms.as_ref() {
            println!(
                "hybrid_timing_ms=planner:{} cpu_indicator_prep:{} gpu_indicator_prep:{} h2d_upload:{} kernel_eval:{}",
                timing.planner_ms,
                timing.cpu_indicator_prep_ms,
                timing.gpu_indicator_prep_ms,
                timing.h2d_upload_ms,
                timing.kernel_eval_ms
            );
        }
        if let Some(plan) = res.indicator_route_plan.as_ref() {
            println!(
                "indicator_route_plan=fast:{}->{} slow:{}->{}",
                plan.fast.indicator_id, plan.fast.route, plan.slow.indicator_id, plan.slow.route
            );
        }
        if let Some(reason) = res.backend_fallback_reason.as_deref() {
            println!("fallback_reason={reason}");
        }
        if let Some(diags) = res.route_diagnostics.as_ref() {
            for diag in diags {
                println!("route_diagnostic={diag}");
            }
        }
    }
}
