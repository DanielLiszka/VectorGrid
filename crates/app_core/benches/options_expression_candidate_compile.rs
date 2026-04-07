mod options_expression_bench_support;

use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use ta_app_core::analyze_options_expression_feasibility_blocking;

use options_expression_bench_support::{
    print_bench_metadata, synthetic_dataset, synthetic_manifest, synthetic_request,
};

fn bench_options_expression_candidate_compile(c: &mut Criterion) {
    let dataset = synthetic_dataset(720);
    let manifest = synthetic_manifest(72, 720);
    let request = synthetic_request();

    let mut group = c.benchmark_group("options_expression/candidate_compile");
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);
    print_bench_metadata(
        "options_expression/candidate_compile",
        &[
            ("events", manifest.events.len().to_string()),
            ("contracts", dataset.contracts.len().to_string()),
            ("surface", "feasibility_preview".to_string()),
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
            criterion::black_box(report.compiled_candidate_count);
        })
    });
    group.finish();
}

criterion_group!(benches, bench_options_expression_candidate_compile);
criterion_main!(benches);
