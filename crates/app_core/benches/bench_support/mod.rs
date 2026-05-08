#[path = "../../src/bin_support/benchmark_lock.rs"]
mod benchmark_lock;

pub fn acquire_benchmark_lock(context: &str) -> benchmark_lock::BenchmarkRunGuard {
    benchmark_lock::acquire(context).unwrap_or_else(|error| {
        eprintln!("failed to acquire benchmark lock: {error}");
        std::process::exit(2);
    })
}

macro_rules! serialized_criterion_main {
    ( $( $group:path ),+ $(,)* ) => {
        fn main() {
            let _benchmark_guard =
                bench_support::acquire_benchmark_lock("app-core Criterion benchmark");

            $(
                $group();
            )+

            ::criterion::Criterion::default()
                .configure_from_args()
                .final_summary();
        }
    }
}
