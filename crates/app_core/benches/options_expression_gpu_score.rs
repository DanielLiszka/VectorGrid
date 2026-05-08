#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[macro_use]
mod bench_support;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
mod options_expression_bench_support;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use criterion::{criterion_group, Criterion};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use std::time::Duration;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use ta_app_core::{
    preferred_cuda_device_id, run_options_expression_blocking, OptionsExpressionBackendPreference,
};

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use options_expression_bench_support::{
    print_bench_metadata, synthetic_dataset, synthetic_manifest, synthetic_request,
};

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn bench_options_expression_gpu_score(c: &mut Criterion) {
    let Some(device_id) = preferred_cuda_device_id() else {
        print_bench_metadata(
            "options_expression/gpu_score",
            &[
                ("surface", "gpu_compiled_tensor".to_string()),
                ("status", "skipped_no_cuda_device".to_string()),
            ],
        );
        return;
    };

    let dataset = synthetic_dataset(1_440);
    let manifest = synthetic_manifest(256, 1_440);
    let mut cpu_request = synthetic_request();
    cpu_request.backend_preference = OptionsExpressionBackendPreference::CpuReference;
    cpu_request.top_k = 32;
    let mut gpu_request = synthetic_request();
    gpu_request.backend_preference = OptionsExpressionBackendPreference::GpuCompiledTensor;
    gpu_request.top_k = 32;

    let _ = run_options_expression_blocking(cpu_request.clone(), &manifest, &dataset)
        .expect("warm cpu run");
    let _ = run_options_expression_blocking(gpu_request.clone(), &manifest, &dataset)
        .expect("warm gpu run");

    let mut group = c.benchmark_group("options_expression/gpu_score");
    group.measurement_time(Duration::from_secs(8));
    group.sample_size(10);
    print_bench_metadata(
        "options_expression/gpu_score",
        &[
            ("events", manifest.events.len().to_string()),
            ("contracts", dataset.contracts.len().to_string()),
            ("rows", dataset.manifest.row_count.to_string()),
            ("device_id", device_id.to_string()),
            ("surface", "cpu_vs_gpu_compiled_tensor".to_string()),
        ],
    );
    group.bench_function("cpu_reference", |b| {
        b.iter(|| {
            let result = run_options_expression_blocking(cpu_request.clone(), &manifest, &dataset)
                .expect("cpu bench run");
            criterion::black_box(result.ranked_results.first().map(|row| row.robust_score));
        })
    });
    group.bench_function("gpu_compiled_tensor", |b| {
        b.iter(|| {
            let result = run_options_expression_blocking(gpu_request.clone(), &manifest, &dataset)
                .expect("gpu bench run");
            criterion::black_box(result.ranked_results.first().map(|row| row.robust_score));
        })
    });
    group.finish();
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
criterion_group!(benches, bench_options_expression_gpu_score);
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
serialized_criterion_main!(benches);

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn main() {}
