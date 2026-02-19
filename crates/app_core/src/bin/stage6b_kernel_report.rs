use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use ta_app_core::{run_double_ma_optimization_blocking_with_candles, Backend, DoubleMaRequest};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;
use vector_ta::utilities::data_loader::Candles;

const FAST_RANGE: (u32, u32, u32) = (5, 120, 1);
const SLOW_RANGE: (u32, u32, u32) = (10, 260, 2);
const TOP_K: usize = 20;
const CANDLE_LEN: usize = 120_000;

#[derive(Clone, Copy)]
enum CandleKind {
    Trending,
    Choppy,
    Breakout,
}

#[derive(Clone)]
struct StrategyCase {
    label: &'static str,
    strategy: StrategyConfig,
    candles: CandleKind,
}

#[derive(Serialize)]
struct Report {
    generated_unix_secs: u64,
    candle_len: usize,
    fast_range: [u32; 3],
    slow_range: [u32; 3],
    top_k: usize,
    cuda_available: bool,
    kernel_build_enabled: bool,
    strategy_reports: Vec<StrategyReport>,
}

#[derive(Serialize)]
struct StrategyReport {
    strategy: String,
    backend_runs: Vec<BackendRunReport>,
    overlap_kernel_vs_cpu: Option<f64>,
    overlap_kernel_vs_gpu_sweep: Option<f64>,
}

#[derive(Serialize)]
struct BackendRunReport {
    backend: String,
    ok: bool,
    runtime_ms: Option<u64>,
    best: Option<BestResult>,
    top_k_count: Option<usize>,
    error: Option<String>,
}

#[derive(Serialize)]
struct BestResult {
    fast_len: u16,
    slow_len: u16,
    pnl: f64,
    sharpe: f64,
    max_dd: f64,
    trades: u32,
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
        let base = 100.0 + i as f64 * 0.02;
        open.push(base);
        high.push(base + 0.8);
        low.push(base - 0.8);
        close.push(base + 0.3);
        vol.push(1_000.0 + (i % 11) as f64 * 20.0);
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
        let c = base + if (i & 1) == 0 { 0.2 } else { -0.2 };
        open.push(base);
        close.push(c);
        high.push(base.max(c) + 0.5);
        low.push(base.min(c) - 0.5);
        vol.push(1_100.0 + (i % 13) as f64 * 17.0);
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
            100.0 + i as f64 * 0.015
        } else if i < down_start {
            102.0 + (i - up_start) as f64 * 0.22
        } else {
            102.0 + (down_start - up_start) as f64 * 0.22 - (i - down_start) as f64 * 0.28
        };
        let n = ((i as f64) / 7.0).sin() * 0.06;
        let o = base + n;
        let c = if i < down_start {
            base + 0.25 + n
        } else {
            base - 0.25 + n
        };
        open.push(o);
        close.push(c);
        high.push(o.max(c) + 0.20);
        low.push(o.min(c) - 0.20);
        let burst = if i < up_start {
            900.0
        } else if i < down_start {
            2_150.0
        } else {
            2_350.0
        };
        vol.push(burst + ((i % 11) as f64) * 30.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn build_case_candles(kind: CandleKind) -> Candles {
    match kind {
        CandleKind::Trending => make_trending_candles(CANDLE_LEN),
        CandleKind::Choppy => make_choppy_candles(CANDLE_LEN),
        CandleKind::Breakout => make_breakout_candles(CANDLE_LEN),
    }
}

fn strategy_cases() -> Vec<StrategyCase> {
    let mut trend_quality = StrategyConfig::default();
    trend_quality.trend_quality_enable = true;
    trend_quality.trend_quality_use_adx = false;

    let mut regime_hybrid = StrategyConfig::default();
    regime_hybrid.long_only = false;
    regime_hybrid.regime_hybrid_enable = true;

    let mut breakout = StrategyConfig::default();
    breakout.long_only = false;
    breakout.breakout_momentum_enable = true;
    breakout.breakout_use_adx = false;
    breakout.breakout_bbands_mult = 0.8;
    breakout.breakout_cmo_min_abs = 0.0;
    breakout.breakout_macd_fast_period = 5;
    breakout.breakout_macd_slow_period = 13;
    breakout.breakout_macd_signal_period = 4;
    breakout.breakout_adosc_short_period = 2;
    breakout.breakout_adosc_long_period = 4;

    vec![
        StrategyCase {
            label: "double_ma_baseline",
            strategy: StrategyConfig::default(),
            candles: CandleKind::Trending,
        },
        StrategyCase {
            label: "composite_trend",
            strategy: trend_quality,
            candles: CandleKind::Trending,
        },
        StrategyCase {
            label: "regime_hybrid",
            strategy: regime_hybrid,
            candles: CandleKind::Choppy,
        },
        StrategyCase {
            label: "breakout_momentum",
            strategy: breakout,
            candles: CandleKind::Breakout,
        },
    ]
}

fn make_request(backend: Backend, strategy: StrategyConfig) -> DoubleMaRequest {
    DoubleMaRequest {
        backend,
        data_id: "stage6b-report".to_string(),
        fast_range: FAST_RANGE,
        slow_range: SLOW_RANGE,
        fast_ma_types: vec!["sma".to_string()],
        slow_ma_types: vec!["sma".to_string()],
        ma_source: "close".to_string(),
        export_all_csv_path: None,
        fast_ma_params: None,
        slow_ma_params: None,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter: None,
        objective: ObjectiveKind::Sharpe,
        mode: OptimizationMode::Grid,
        top_k: Some(TOP_K),
        include_all: Some(false),
        heatmap_bins: Some(0),
        constraints: None,
        vdps: None,
    }
}

fn run_backend(
    backend_name: &str,
    backend: Backend,
    candles: &Candles,
    strategy: StrategyConfig,
    cancel: &AtomicBool,
) -> (BackendRunReport, Option<ta_app_core::BackendOptimizationResult>) {
    let req = make_request(backend, strategy);
    let start = Instant::now();
    match run_double_ma_optimization_blocking_with_candles(req, candles, cancel, None) {
        Ok(res) => {
            let best = BestResult {
                fast_len: res.best_params.fast_len,
                slow_len: res.best_params.slow_len,
                pnl: res.best_metrics.pnl,
                sharpe: res.best_metrics.sharpe,
                max_dd: res.best_metrics.max_dd,
                trades: res.best_metrics.trades,
            };
            (
                BackendRunReport {
                    backend: backend_name.to_string(),
                    ok: true,
                    runtime_ms: Some(start.elapsed().as_millis() as u64),
                    best: Some(best),
                    top_k_count: Some(res.top.len()),
                    error: None,
                },
                Some(res),
            )
        }
        Err(e) => (
            BackendRunReport {
                backend: backend_name.to_string(),
                ok: false,
                runtime_ms: Some(start.elapsed().as_millis() as u64),
                best: None,
                top_k_count: None,
                error: Some(e),
            },
            None,
        ),
    }
}

fn topk_keys(
    r: &ta_app_core::BackendOptimizationResult,
    k: usize,
) -> Vec<(u16, u16, String, String)> {
    r.top
        .iter()
        .take(k)
        .map(|(p, _)| {
            (
                p.fast_len,
                p.slow_len,
                p.fast_ma_type.clone(),
                p.slow_ma_type.clone(),
            )
        })
        .collect()
}

fn topk_overlap(
    lhs: &ta_app_core::BackendOptimizationResult,
    rhs: &ta_app_core::BackendOptimizationResult,
    k: usize,
) -> Option<f64> {
    let left = topk_keys(lhs, k);
    if left.is_empty() {
        return None;
    }
    let right: HashSet<(u16, u16, String, String)> = topk_keys(rhs, k).into_iter().collect();
    if right.is_empty() {
        return None;
    }
    let hit = left.iter().filter(|key| right.contains(*key)).count();
    Some(hit as f64 / left.len() as f64)
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
    let output_path = std::env::args().nth(1);
    let cancel = AtomicBool::new(false);
    let cuda_ok = cuda_available();
    let mut strategy_reports: Vec<StrategyReport> = Vec::new();

    for case in strategy_cases() {
        let candles = build_case_candles(case.candles);
        let mut backend_runs: Vec<BackendRunReport> = Vec::new();
        let (cpu_row, cpu_res) = run_backend(
            "cpu_only",
            Backend::CpuOnly,
            &candles,
            case.strategy.clone(),
            &cancel,
        );
        backend_runs.push(cpu_row);

        let mut gpu_sweep_res: Option<ta_app_core::BackendOptimizationResult> = None;
        #[cfg_attr(not(feature = "cuda-backtest-kernel"), allow(unused_mut))]
        let mut gpu_kernel_res: Option<ta_app_core::BackendOptimizationResult> = None;
        if cuda_ok {
            let (gpu_sweep_row, r) = run_backend(
                "gpu_sweep",
                Backend::GpuOnly { device_id: 0 },
                &candles,
                case.strategy.clone(),
                &cancel,
            );
            backend_runs.push(gpu_sweep_row);
            gpu_sweep_res = r;

            #[cfg(feature = "cuda-backtest-kernel")]
            {
                let (gpu_kernel_row, r) = run_backend(
                    "gpu_kernel",
                    Backend::GpuKernel { device_id: 0 },
                    &candles,
                    case.strategy.clone(),
                    &cancel,
                );
                backend_runs.push(gpu_kernel_row);
                gpu_kernel_res = r;
            }
        }

        let overlap_kernel_vs_cpu = match (&gpu_kernel_res, &cpu_res) {
            (Some(k), Some(c)) => topk_overlap(k, c, TOP_K),
            _ => None,
        };
        let overlap_kernel_vs_gpu_sweep = match (&gpu_kernel_res, &gpu_sweep_res) {
            (Some(k), Some(g)) => topk_overlap(k, g, TOP_K),
            _ => None,
        };

        strategy_reports.push(StrategyReport {
            strategy: case.label.to_string(),
            backend_runs,
            overlap_kernel_vs_cpu,
            overlap_kernel_vs_gpu_sweep,
        });
    }

    let report = Report {
        generated_unix_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        candle_len: CANDLE_LEN,
        fast_range: [FAST_RANGE.0, FAST_RANGE.1, FAST_RANGE.2],
        slow_range: [SLOW_RANGE.0, SLOW_RANGE.1, SLOW_RANGE.2],
        top_k: TOP_K,
        cuda_available: cuda_ok,
        kernel_build_enabled: cfg!(feature = "cuda-backtest-kernel"),
        strategy_reports,
    };

    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Some(path) = output_path {
        if let Err(e) = std::fs::write(&path, json.as_bytes()) {
            eprintln!("failed to write report to '{}': {}", path, e);
            std::process::exit(1);
        }
        println!("wrote stage6b report to {}", path);
    } else {
        println!("{}", json);
    }
}
