#[macro_use]
mod bench_support;

mod options_expression_bench_support;

use criterion::{criterion_group, Criterion};
use std::time::Duration;
use ta_app_core::analyze_options_expression_feasibility_blocking;

use options_expression_bench_support::{
    print_bench_metadata, synthetic_dataset, synthetic_manifest, synthetic_request,
};

fn bench_options_expression_summary_compile(c: &mut Criterion) {
    let dataset = synthetic_dataset(960);
    let manifest = synthetic_manifest(84, 960);
    let mut request = synthetic_request();
    request.hold_minutes = vec![5, 15, 30, 60, 120, 240];
    request.target_return_buckets = vec![0.15, 0.25, 0.50, 0.75, 1.0];
    request.stop_return_buckets = vec![0.10, 0.25, 0.50];
    request.top_k = 16;

    let mut group = c.benchmark_group("options_expression/summary_compile");
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);
    print_bench_metadata(
        "options_expression/summary_compile",
        &[
            ("events", manifest.events.len().to_string()),
            ("contracts", dataset.contracts.len().to_string()),
            (
                "surface",
                "feasibility_preview_summary_heavy_configuration".to_string(),
            ),
        ],
    );
    group.bench_function("analyze_options_expression_feasibility_blocking", |b| {
        b.iter(|| {
            let report = analyze_options_expression_feasibility_blocking(
                request.clone(),
                &manifest,
                &dataset,
            )
            .expect("feasibility");
            criterion::black_box(report.baseline_exit_summary_count);
            criterion::black_box(report.conservative_exit_summary_count);
        })
    });
    group.finish();
}

criterion_group!(benches, bench_options_expression_summary_compile);
serialized_criterion_main!(benches);
