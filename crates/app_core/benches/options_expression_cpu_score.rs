#[macro_use]
mod bench_support;

mod options_expression_bench_support;

use criterion::{criterion_group, Criterion};
use std::time::Duration;
use ta_app_core::run_options_expression_blocking;

use options_expression_bench_support::{
    print_bench_metadata, synthetic_dataset, synthetic_manifest, synthetic_request,
};

fn bench_options_expression_cpu_score(c: &mut Criterion) {
    let dataset = synthetic_dataset(720);
    let manifest = synthetic_manifest(72, 720);
    let request = synthetic_request();

    let mut group = c.benchmark_group("options_expression/cpu_score");
    group.measurement_time(Duration::from_secs(6));
    group.sample_size(10);
    print_bench_metadata(
        "options_expression/cpu_score",
        &[
            ("events", manifest.events.len().to_string()),
            ("contracts", dataset.contracts.len().to_string()),
            ("surface", "cpu_reference".to_string()),
        ],
    );
    group.bench_function("run_options_expression_blocking", |b| {
        b.iter(|| {
            let result =
                run_options_expression_blocking(request.clone(), &manifest, &dataset).expect("run");
            criterion::black_box(result.ranked_results.len());
        })
    });
    group.finish();
}

criterion_group!(benches, bench_options_expression_cpu_score);
serialized_criterion_main!(benches);
