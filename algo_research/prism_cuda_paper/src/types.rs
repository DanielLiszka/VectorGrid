use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ObjectiveSpec {
    pub name: &'static str,
    pub maximize: bool,
}

#[derive(Clone, Debug)]
pub struct ExecutionStage {
    pub name: &'static str,
    pub candle_fraction: f32,
    pub asset_fraction: f32,
    pub fold_fraction: f32,
    pub resource: f32,
    pub graph_stage_id: &'static str,
}

impl ExecutionStage {
    pub fn scale(&self) -> f32 {
        self.candle_fraction * self.asset_fraction * self.fold_fraction
    }
}

pub fn default_execution_stages() -> Vec<ExecutionStage> {
    vec![
        ExecutionStage {
            name: "lf0_fast",
            candle_fraction: 0.15,
            asset_fraction: 0.25,
            fold_fraction: 0.34,
            resource: 0.10,
            graph_stage_id: "lf0",
        },
        ExecutionStage {
            name: "lf1_mid",
            candle_fraction: 0.35,
            asset_fraction: 0.50,
            fold_fraction: 0.67,
            resource: 0.35,
            graph_stage_id: "lf1",
        },
        ExecutionStage {
            name: "hf2_full_assets",
            candle_fraction: 0.70,
            asset_fraction: 1.0,
            fold_fraction: 1.0,
            resource: 0.70,
            graph_stage_id: "hf2",
        },
        ExecutionStage {
            name: "xf3_full",
            candle_fraction: 1.0,
            asset_fraction: 1.0,
            fold_fraction: 1.0,
            resource: 1.0,
            graph_stage_id: "xf3",
        },
    ]
}

#[derive(Clone, Debug, Default)]
pub struct CandidateState {
    pub module_scores: HashMap<String, [f32; 2]>,
    pub cache_handles: HashMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct SignatureCandidate {
    pub cid: String,
    pub profile_key: String,
    pub latent: Vec<f32>,
    pub decoded_params: HashMap<String, f32>,
    pub origin: String,
    pub parent_id: Option<String>,
    pub changed_group_signature: String,
    pub objectives_by_stage: HashMap<usize, [f32; 2]>,
    pub state_by_stage: HashMap<usize, CandidateState>,
    pub meta_by_stage: HashMap<usize, HashMap<String, f32>>,
    pub last_expected_hv_gain_per_ms: f32,
}

impl SignatureCandidate {
    pub fn new(
        cid: String,
        profile_key: String,
        latent: Vec<f32>,
        decoded_params: HashMap<String, f32>,
        origin: &str,
        parent_id: Option<String>,
        changed_group_signature: &str,
    ) -> Self {
        Self {
            cid,
            profile_key,
            latent,
            decoded_params,
            origin: origin.to_string(),
            parent_id,
            changed_group_signature: changed_group_signature.to_string(),
            objectives_by_stage: HashMap::new(),
            state_by_stage: HashMap::new(),
            meta_by_stage: HashMap::new(),
            last_expected_hv_gain_per_ms: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExactEvalRequest {
    pub candidate_id: String,
    pub profile_key: String,
    pub latent: Vec<f32>,
    pub decoded_params: HashMap<String, f32>,
    pub stage_idx: usize,
    pub parent_state: Option<CandidateState>,
    pub changed_group_signature: String,
}

#[derive(Clone, Debug)]
pub struct ExactEvalResult {
    pub objectives_max: [f32; 2],
    pub state: Option<CandidateState>,
    pub runtime_ms: f32,
    pub host_device_bytes: f32,
    pub bytes_avoided: f32,
    pub partial_reuse: bool,
    pub graph_key: String,
    pub graph_hit: bool,
    pub metadata: HashMap<String, f32>,
}

#[derive(Clone, Debug)]
pub struct GraphPlan {
    pub graph_key: String,
    pub batch_shape: usize,
    pub request_indices: Vec<usize>,
    pub active_count: usize,
    pub graph_hit: bool,
}

#[derive(Clone, Debug)]
pub struct SearchCell {
    pub cell_id: String,
    pub profile_key: String,
    pub stage_idx: usize,
    pub low_u: Vec<f32>,
    pub high_u: Vec<f32>,
    pub depth: usize,
    pub exact_count: usize,
    pub screened_count: usize,
    pub best_conservative_hv_ms: f32,
    pub last_uncertainty: f32,
    pub anchor: Option<SignatureCandidate>,
    pub bank: Vec<SignatureCandidate>,
}

impl SearchCell {
    pub fn width(&self) -> Vec<f32> {
        self.low_u
            .iter()
            .zip(self.high_u.iter())
            .map(|(l, h)| h - l)
            .collect()
    }

    pub fn radius(&self) -> f32 {
        let w = self.width();
        if w.is_empty() {
            0.0
        } else {
            w.iter().sum::<f32>() / w.len() as f32
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StageArchivePoints {
    pub points: Vec<Vec<f32>>,
}
