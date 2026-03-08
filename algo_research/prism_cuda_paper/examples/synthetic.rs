use prism_cuda_paper::{
    build_modular_triad_registry, MockBackend, PrismOptimizer, PrismOptimizerConfig,
};

fn main() {
    let registry = build_modular_triad_registry();
    let backend = MockBackend::new(7, 200_000, 24, 3);
    let cfg = PrismOptimizerConfig {
        max_exact_evals: 384,
        ..Default::default()
    };
    let mut opt = PrismOptimizer::new(registry, backend, 7, cfg);
    let summary = opt.optimize();
    println!(
        "final_hv={:.6} stage={} archive={} evals={} partial_reuses={} graph_hit_rate={:.3} bytes_avoided={:.1}",
        summary.final_hv,
        summary.final_stage_idx,
        summary.archive_size,
        summary.exact_evals,
        summary.partial_reuses,
        summary.graph_hit_rate,
        summary.bytes_avoided
    );
}
