use crate::math::{crowding_select, pareto_front_indices};
use crate::types::SignatureCandidate;

#[derive(Clone, Debug)]
pub struct ParetoArchive2D {
    pub max_size: usize,
    pub members: Vec<SignatureCandidate>,
}

impl ParetoArchive2D {
    pub fn new(max_size: usize) -> Self {
        Self { max_size, members: Vec::new() }
    }

    pub fn points(&self, stage_idx: usize) -> Vec<Vec<f32>> {
        self.members
            .iter()
            .filter_map(|c| c.objectives_by_stage.get(&stage_idx).map(|o| vec![o[0], o[1]]))
            .collect()
    }

    pub fn update(&mut self, mut candidates: Vec<SignatureCandidate>, stage_idx: usize) {
        let mut pool = Vec::new();
        pool.append(&mut self.members.clone());
        pool.append(&mut candidates);
        let points: Vec<Vec<f32>> = pool
            .iter()
            .filter_map(|c| c.objectives_by_stage.get(&stage_idx).map(|o| vec![o[0], o[1]]))
            .collect();
        if points.is_empty() {
            return;
        }
        let keep_idx = pareto_front_indices(&points);
        let mut keep: Vec<SignatureCandidate> = keep_idx.into_iter().map(|i| pool[i].clone()).collect();
        if keep.len() > self.max_size {
            let pts: Vec<Vec<f32>> = keep
                .iter()
                .filter_map(|c| c.objectives_by_stage.get(&stage_idx).map(|o| vec![o[0], o[1]]))
                .collect();
            let subset = crowding_select(&pts, self.max_size);
            keep = subset.into_iter().map(|i| keep[i].clone()).collect();
        }
        self.members = keep;
    }
}
