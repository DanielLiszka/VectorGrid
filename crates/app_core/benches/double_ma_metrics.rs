use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use ta_app_core::{Backend, DoubleMaRequest};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;

use vector_ta::utilities::data_loader::Candles;

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

fn expand_range_u16((start, end, step): (u32, u32, u32)) -> Vec<u16> {
    let push_checked = |out: &mut Vec<u16>, v: u32| {
        if v <= u16::MAX as u32 {
            out.push(v as u16);
        }
    };

    if step == 0 || start == end {
        let mut out = Vec::with_capacity(1);
        push_checked(&mut out, start);
        return out;
    }

    let step = step.max(1);
    let (lo, hi) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };

    let mut out = Vec::new();
    let mut v = lo;
    loop {
        push_checked(&mut out, v);
        if v == hi {
            break;
        }
        match v.checked_add(step) {
            Some(next) if next > v && next <= hi => v = next,
            _ => break,
        }
    }
    out
}

fn count_pairs_fast_lt_slow(fast_periods: &[u16], slow_periods: &[u16]) -> usize {
    if fast_periods.is_empty() || slow_periods.is_empty() {
        return 0;
    }
    let mut slow_sorted: Vec<u16> = slow_periods.to_vec();
    slow_sorted.sort_unstable();

    let s_len = slow_sorted.len();
    let mut total: usize = 0;
    for &f in fast_periods {
        let idx = slow_sorted.partition_point(|&s| s <= f);
        total = total.saturating_add(s_len.saturating_sub(idx));
    }
    total
}

fn bench_ma_profile() -> (
    &'static str,
    &'static str,
    &'static str,
    Option<HashMap<String, f64>>,
) {
    let profile = std::env::var("VECTORBT_BENCH_KERNEL_MA")
        .unwrap_or_else(|_| "sma".to_string())
        .to_ascii_lowercase();
    match profile.as_str() {
        "alma" => {
            let mut alma_p: HashMap<String, f64> = HashMap::new();
            alma_p.insert("offset".to_string(), 0.85);
            alma_p.insert("sigma".to_string(), 6.0);
            ("alma_alma", "alma", "alma", Some(alma_p))
        }
        _ => ("sma_sma", "sma", "sma", None),
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_gpu_kernel_pnl_full_metrics(c: &mut Criterion) {
    if !vector_ta::cuda::cuda_available() {
        return;
    }

    let candles = make_synth_candles(200_000);
    let cancel = AtomicBool::new(false);

    let prev_budget = std::env::var("VECTORBT_KERNEL_VRAM_BUDGET_MB").ok();

    let (ma_label, fast_ma, slow_ma, ma_params) = bench_ma_profile();

    let cases: &[(&str, (u32, u32, u32), (u32, u32, u32), Option<&'static str>)] = &[
        ("small", (5, 50, 1), (20, 200, 5), None),
        ("medium", (5, 100, 1), (10, 200, 1), None),
        ("large", (5, 200, 1), (10, 400, 1), None),
        ("large_batched_1gb", (5, 200, 1), (10, 400, 1), Some("1024")),
    ];

    let mut group = c.benchmark_group(format!(
        "double_ma/gpu_kernel_pnl_full_metrics/{ma_label}/200k"
    ));
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);

    for (case_label, fast_range, slow_range, budget_mb) in cases.iter().copied() {
        match budget_mb {
            Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
            None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
        }

        let fast_periods = expand_range_u16(fast_range);
        let slow_periods = expand_range_u16(slow_range);
        let pairs = count_pairs_fast_lt_slow(&fast_periods, &slow_periods);
        let req = DoubleMaRequest {
            backend: Backend::GpuKernel { device_id: 0 },
            data_id: "bench".to_string(),
            fast_range,
            slow_range,
            fast_ma_types: vec![fast_ma.to_string()],
            slow_ma_types: vec![slow_ma.to_string()],
            ma_source: "close".to_string(),
            export_all_csv_path: None,
            fast_ma_params: ma_params.clone(),
            slow_ma_params: ma_params.clone(),
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Pnl,
            mode: OptimizationMode::Grid,
            top_k: Some(50),
            include_all: Some(false),
            heatmap_bins: Some(64),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: Default::default(),
            persist_followup_cache: false,
        };

        let name = format!("{case_label}/{pairs}pairs");
        group.bench_function(&name, |b| {
            b.iter(|| {
                let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                    req.clone(),
                    &candles,
                    &cancel,
                    None,
                )
                .expect("gpu kernel run");
            })
        });
    }

    match prev_budget {
        Some(v) => std::env::set_var("VECTORBT_KERNEL_VRAM_BUDGET_MB", v),
        None => std::env::remove_var("VECTORBT_KERNEL_VRAM_BUDGET_MB"),
    }
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn bench_gpu_kernel_pnl_full_metrics(_c: &mut Criterion) {}

criterion_group!(benches, bench_gpu_kernel_pnl_full_metrics);
criterion_main!(benches);
