use std::time::{Duration, Instant};

use app_core::tensorcore_alma::{
    device_synchronize, AlmaSpec, CudaF32Buffer, TcPrecision, TensorCoreAlmaPlan,
};

fn lcg(seed: &mut u64) -> f32 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let x = ((*seed >> 33) as u32) as f32 / (u32::MAX as f32);
    x
}

fn build_prices(n: usize) -> Vec<f32> {
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut out = Vec::with_capacity(n);
    let mut px = 100.0f32;
    for _ in 0..n {
        let step = (lcg(&mut seed) - 0.5) * 0.8;
        px = (px + step).max(1.0);
        out.push(px);
    }
    out
}

fn build_specs() -> Vec<AlmaSpec> {
    let mut specs = Vec::new();
    for period in (10u16..=259u16).step_by(1) {
        specs.push(AlmaSpec {
            period,
            offset: 0.85,
            sigma: 6.0,
        });
    }
    specs
}

fn cpu_alma_at(prices: &[f32], spec: AlmaSpec, end_idx: usize) -> f32 {
    let period = spec.period as usize;
    let start = end_idx + 1 - period;
    let mean = spec.offset * ((period - 1) as f32);
    let stdev = (period as f32) / spec.sigma;
    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for i in 0..period {
        let x = i as f32;
        let w = (-(x - mean).powi(2) / (2.0 * stdev * stdev)).exp();
        num += prices[start + i] * w;
        den += w;
    }
    num / den
}

fn median(mut xs: Vec<Duration>) -> Duration {
    xs.sort();
    xs[xs.len() / 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n_prices = 200_000usize;
    let prices = build_prices(n_prices);
    let specs = build_specs();

    let plan = TensorCoreAlmaPlan::from_alma_specs(&specs, TcPrecision::Tf32Fast)?;
    let rows = plan.output_rows(prices.len())?;
    let cols = plan.n_windows();

    println!(
        "tensorcore-alma bench: prices={} max_period={} params={} rows={} valid_from={}",
        n_prices,
        plan.max_period(),
        cols,
        rows,
        plan.valid_from()
    );

    let dev_prices = CudaF32Buffer::from_slice(&prices)?;
    let mut dev_out = CudaF32Buffer::zeros(rows * cols)?;

    // Warmup
    unsafe {
        plan.run_on_device_raw(
            dev_prices.as_device_ptr(),
            prices.len(),
            dev_out.as_mut_device_ptr(),
            8192,
            std::ptr::null_mut(),
        )?;
    }
    device_synchronize()?;

    let mut samples = Vec::new();
    for _ in 0..10 {
        let start = Instant::now();
        unsafe {
            plan.run_on_device_raw(
                dev_prices.as_device_ptr(),
                prices.len(),
                dev_out.as_mut_device_ptr(),
                8192,
                std::ptr::null_mut(),
            )?;
        }
        device_synchronize()?;
        samples.push(start.elapsed());
    }

    let med = median(samples);
    println!(
        "median runtime: {:.3} ms for {} ALMA configs over {} rows",
        med.as_secs_f64() * 1e3,
        cols,
        rows
    );

    let mut host_out = vec![0.0f32; rows * cols];
    dev_out.copy_to(&mut host_out)?;

    // Spot-check a few entries against a CPU reference.
    let mut max_abs_err = 0.0f32;
    let check_cols = [0usize, cols / 2, cols - 1];
    let check_rows = [0usize, rows / 3, rows - 1];
    for &r in &check_rows {
        let end_idx = plan.valid_from() + r;
        for &c in &check_cols {
            let got = host_out[r * cols + c];
            let want = cpu_alma_at(&prices, specs[c], end_idx);
            let err = (got - want).abs();
            max_abs_err = max_abs_err.max(err);
            println!(
                "check row={} col={} end_idx={} got={:.6} want={:.6} abs_err={:.6}",
                r, c, end_idx, got, want, err
            );
        }
    }

    println!("max sampled absolute error: {:.6}", max_abs_err);
    Ok(())
}
