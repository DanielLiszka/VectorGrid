use std::collections::{HashMap, HashSet};

use crate::signature::StrategyRegistry;
use crate::types::{ExactEvalRequest, ExecutionStage, GraphPlan};

#[derive(Clone, Debug)]
pub struct GraphBatchPlanner {
    pub stable_shapes: Vec<usize>,
    cache: HashSet<String>,
    pub hits: usize,
    pub misses: usize,
    pub plans: usize,
}

impl Default for GraphBatchPlanner {
    fn default() -> Self {
        Self::new(vec![16, 32, 64, 128])
    }
}

impl GraphBatchPlanner {
    pub fn new(stable_shapes: Vec<usize>) -> Self {
        Self {
            stable_shapes,
            cache: HashSet::new(),
            hits: 0,
            misses: 0,
            plans: 0,
        }
    }

    pub fn select_shape(&self, n: usize) -> usize {
        for &s in &self.stable_shapes {
            if n <= s {
                return s;
            }
        }
        *self.stable_shapes.last().unwrap_or(&128)
    }

    pub fn make_plans(
        &mut self,
        requests: &[ExactEvalRequest],
        registry: &StrategyRegistry,
        stages: &[ExecutionStage],
    ) -> Vec<GraphPlan> {
        let mut buckets: HashMap<(String, String, String), Vec<usize>> = HashMap::new();
        for (i, req) in requests.iter().enumerate() {
            let prof = registry.get(&req.profile_key);
            let stage = &stages[req.stage_idx];
            let key = (
                prof.signature.graph_shape(),
                stage.graph_stage_id.to_string(),
                req.changed_group_signature.clone(),
            );
            buckets.entry(key).or_default().push(i);
        }
        let mut out = Vec::new();
        for ((shape, stage_id, changed_sig), idxs) in buckets {
            let max_shape = *self.stable_shapes.last().unwrap_or(&128);
            for chunk in idxs.chunks(max_shape) {
                let batch_shape = self.select_shape(chunk.len());
                let graph_key = format!("{}|{}|{}|B{}", shape, stage_id, changed_sig, batch_shape);
                let hit = self.cache.contains(&graph_key);
                if hit {
                    self.hits += 1;
                } else {
                    self.cache.insert(graph_key.clone());
                    self.misses += 1;
                }
                self.plans += 1;
                out.push(GraphPlan {
                    graph_key,
                    batch_shape,
                    request_indices: chunk.to_vec(),
                    active_count: chunk.len(),
                    graph_hit: hit,
                });
            }
        }
        out
    }

    pub fn replay_hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f32 / total as f32 }
    }
}
