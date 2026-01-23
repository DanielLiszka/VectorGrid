pub mod composite_cpu;
pub mod double_ma;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub mod gpu_backtest_kernel;
pub mod progress;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
pub(crate) mod vram_ma;

pub use ta_strategies::double_ma::DoubleMaCurves;

pub use double_ma::{
    compute_double_ma_drilldown_blocking_with_candles,
    run_double_ma_optimization_blocking_with_candles, Backend, BackendOptimizationResult,
    BackendUsed, DoubleMaDrilldownRequest, DoubleMaParamsResolved, DoubleMaRequest,
    OptimizationModeResolved,
};

pub use composite_cpu::{
    compute_composite_cpu_drilldown_blocking_with_candles,
    run_composite_cpu_optimization_blocking_with_candles, CompositeCpuDrilldownRequest,
    CompositeCpuRequest,
};
