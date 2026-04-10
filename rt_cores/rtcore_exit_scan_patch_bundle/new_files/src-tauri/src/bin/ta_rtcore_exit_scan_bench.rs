use std::time::Instant;

use app_core::rtcore_exit_scan::{evaluate_exit_bars_rtcore, ExitHit, RtCoreBarrierScene};

fn next_u32(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 32) as u32
}

fn next_f32(state: &mut u64) -> f32 {
    (next_u32(state) as f32) / (u32::MAX as f32)
}

fn build_synthetic_candles(n_bars: usize) -> (Vec<f32>, Vec<f32>) {
    let mut lows = Vec::with_capacity(n_bars);
    let mut highs = Vec::with_capacity(n_bars);
    let mut rng = 0xDEC0DED_u64;
    let mut price = 100.0f32;

    for i in 0..n_bars {
        let drift = ((i as f32) * 0.0005).sin() * 0.4;
        let noise = (next_f32(&mut rng) - 0.5) * 0.75;
        let width = 0.2 + next_f32(&mut rng) * 1.8;
        price += drift + noise * 0.05;
        let mid = price + noise;
        lows.push(mid - width);
        highs.push(mid + width);
    }

    (lows, highs)
}

fn build_queries(lows: &[f32], highs: &[f32], n_queries: usize) -> (Vec<i32>, Vec<f32>, Vec<f32>) {
    let mut rng = 0xBAD5EED_u64;
    let n_bars = lows.len();
    let mut entry_bars = Vec::with_capacity(n_queries);
    let mut stops = Vec::with_capacity(n_queries);
    let mut targets = Vec::with_capacity(n_queries);

    for _ in 0..n_queries {
        let entry = 8 + (next_u32(&mut rng) as usize % (n_bars.saturating_sub(32)));
        let anchor = 0.5 * (lows[entry] + highs[entry]);
        let stop = anchor - (0.25 + next_f32(&mut rng) * 1.5);
        let target = anchor + (0.25 + next_f32(&mut rng) * 1.5);
        entry_bars.push(entry as i32);
        stops.push(stop);
        targets.push(target);
    }

    (entry_bars, stops, targets)
}

fn cpu_first_hit(lows: &[f32], highs: &[f32], entry_bar: i32, barrier_price: f32) -> Option<i32> {
    let start = (entry_bar + 1).max(0) as usize;
    for i in start..lows.len() {
        if lows[i] <= barrier_price && highs[i] >= barrier_price {
            return Some(i as i32);
        }
    }
    None
}

fn sample_match_rate(
    lows: &[f32],
    highs: &[f32],
    entry_bars: &[i32],
    stops: &[f32],
    targets: &[f32],
    rt_hits: &[ExitHit],
    sample: usize,
) -> f32 {
    let n = sample.min(entry_bars.len());
    if n == 0 {
        return 1.0;
    }
    let mut ok = 0usize;
    for i in 0..n {
        let stop_cpu = cpu_first_hit(lows, highs, entry_bars[i], stops[i]);
        let target_cpu = cpu_first_hit(lows, highs, entry_bars[i], targets[i]);
        if stop_cpu == rt_hits[i].stop_hit_bar && target_cpu == rt_hits[i].target_hit_bar {
            ok += 1;
        }
    }
    ok as f32 / n as f32
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let n_bars: usize = std::env::var("RTCORE_BARS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let n_queries: usize = std::env::var("RTCORE_QUERIES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000);

    println!("rtcore_exit_scan_bench: bars={n_bars}, queries={n_queries}");
    println!("compiled_in={}", RtCoreBarrierScene::compiled());
    println!("rtcore_available_gpu0={}", RtCoreBarrierScene::available(0));

    let (lows, highs) = build_synthetic_candles(n_bars);
    let (entry_bars, stops, targets) = build_queries(&lows, &highs, n_queries);

    let t0 = Instant::now();
    let rt_hits = evaluate_exit_bars_rtcore(&lows, &highs, &entry_bars, &stops, &targets)?;
    let total = t0.elapsed();

    let sample = 10_000usize.min(n_queries);
    let t1 = Instant::now();
    let match_rate = sample_match_rate(&lows, &highs, &entry_bars, &stops, &targets, &rt_hits, sample);
    let validation = t1.elapsed();

    let stop_hits = rt_hits.iter().filter(|h| h.stop_hit_bar.is_some()).count();
    let target_hits = rt_hits.iter().filter(|h| h.target_hit_bar.is_some()).count();

    println!("rt_total_ms={:.3}", total.as_secs_f64() * 1000.0);
    println!("rt_queries_per_sec={:.3}", n_queries as f64 / total.as_secs_f64());
    println!("stop_hit_rate={:.3}", stop_hits as f64 / n_queries as f64);
    println!("target_hit_rate={:.3}", target_hits as f64 / n_queries as f64);
    println!("sample_validation_n={sample}");
    println!("sample_match_rate={:.6}", match_rate);
    println!("sample_validation_ms={:.3}", validation.as_secs_f64() * 1000.0);

    Ok(())
}
