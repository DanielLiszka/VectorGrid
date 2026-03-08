pub mod rng;
pub mod math;
pub mod types;
pub mod signature;
pub mod pareto;
pub mod cost;
pub mod conformal;
pub mod scheduler;
pub mod backend;
pub mod optimizer;

pub use backend::{BacktestGpuBackend, MockBackend, PtxLaunchManifest};
pub use conformal::{ConformalScreenModel, ScreenPrediction};
pub use cost::{CostPrediction, RuntimeCostModel};
pub use optimizer::{PrismOptimizer, PrismOptimizerConfig, PrismRunSummary};
pub use pareto::ParetoArchive2D;
pub use scheduler::GraphBatchPlanner;
pub use signature::{
    build_modular_triad_registry, EntryModule, LatentParamSpec, MaType, RegimeGate,
    RiskOverlay, StrategyProfile, StrategyRegistry, StrategySignature,
};
pub use types::{
    CandidateState, ExactEvalRequest, ExactEvalResult, ExecutionStage, GraphPlan,
    ObjectiveSpec, SearchCell, SignatureCandidate, StageArchivePoints,
};
