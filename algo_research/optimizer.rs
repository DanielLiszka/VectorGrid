use std::collections::{HashMap, VecDeque};

use crate::backend::BacktestGpuBackend;
use crate::conformal::ConformalScreenModel;
use crate::cost::RuntimeCostModel;
use crate::math::{crowding_select, hv_gain_2d_max, hypervolume_2d_max, dominates_max};
use crate::pareto::ParetoArchive2D;
use crate::rng::{lhs_sample, stable_seed, SplitMix64};
use crate::scheduler::GraphBatchPlanner;
use crate::signature::{StrategyProfile, StrategyRegistry};
use crate::types::{
    default_execution_stages, ExactEvalRequest, ExecutionStage, SearchCell, SignatureCandidate,
};

#[derive(Clone, Debug)]
pub struct PrismOptimizerConfig {
    pub screen_batch_size: usize,
    pub exact_batch_size: usize,
    pub min_screen_keep: usize,
    pub screen_keep_fraction: f32,
    pub local_group_samples: usize,
    pub max_depth: usize,
    pub max_exact_evals: usize,
    pub archive_size: usize,
    pub reference_point: [f32; 2],
}

impl Default for PrismOptimizerConfig {
    fn default() -> Self {
        Self {
            screen_batch_size: 64,
            exact_batch_size: 16,
            min_screen_keep: 8,
            screen_keep_fraction: 0.25,
            local_group_samples: 6,
            max_depth: 5,
            max_exact_evals: 512,
            archive_size: 128,
            reference_point: [0.0, 0.0],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PrismRunSummary {
    pub final_hv: f32,
    pub final_stage_idx: usize,
    pub archive_size: usize,
    pub exact_evals: usize,
    pub partial_reuses: usize,
    pub graph_hit_rate: f32,
    pub host_device_bytes: f32,
    pub bytes_avoided: f32,
    pub screen_pruned: usize,
    pub false_prunes_estimate: usize,
}

pub struct PrismOptimizer<B: BacktestGpuBackend> {
    pub registry: StrategyRegistry,
    pub stages: Vec<ExecutionStage>,
    pub backend: B,
    pub cost_model: RuntimeCostModel,
    pub graph_planner: GraphBatchPlanner,
    pub config: PrismOptimizerConfig,
    rng: SplitMix64,
    stage_archives: Vec<ParetoArchive2D>,
    screens: HashMap<(String, usize), ConformalScreenModel>,
    next_candidate_id: u64,
    pub screen_pruned: usize,
    pub false_prunes_estimate: usize,
}

impl<B: BacktestGpuBackend> PrismOptimizer<B> {
    pub fn new(registry: StrategyRegistry, backend: B, seed: u64, config: PrismOptimizerConfig) -> Self {
        let stages = default_execution_stages();
        let archive_size = config.archive_size;
        let mut stage_archives = Vec::new();
        for _ in 0..stages.len() {
            stage_archives.push(ParetoArchive2D::new(archive_size));
        }
        Self {
            registry,
            stages,
            backend,
            cost_model: RuntimeCostModel::default(),
            graph_planner: GraphBatchPlanner::default(),
            config,
            rng: SplitMix64::new(seed),
            stage_archives,
            screens: HashMap::new(),
            next_candidate_id: 1,
            screen_pruned: 0,
            false_prunes_estimate: 0,
        }
    }

    fn next_id(&mut self) -> String {
        let id = self.next_candidate_id;
        self.next_candidate_id += 1;
        format!("cand{:08}", id)
    }

    fn screen_model(&mut self, profile_key: &str, stage_idx: usize) -> &mut ConformalScreenModel {
        let key = (profile_key.to_string(), stage_idx);
        if !self.screens.contains_key(&key) {
            let profile = self.registry.get(profile_key).clone();
            let stage = self.stages[stage_idx].clone();
            self.screens.insert(key.clone(), ConformalScreenModel::new(profile, stage));
        }
        self.screens.get_mut(&key).unwrap()
    }

    fn root_cells(&mut self) -> Vec<SearchCell> {
        let mut cells = Vec::new();
        for key in self.registry.keys() {
            let prof = self.registry.get(&key);
            let root = SearchCell {
                cell_id: format!("root-{}", stable_seed(&[&key, "0"])),
                profile_key: key.clone(),
                stage_idx: 0,
                low_u: vec![0.0; prof.dim()],
                high_u: vec![1.0; prof.dim()],
                depth: 0,
                exact_count: 0,
                screened_count: 0,
                best_conservative_hv_ms: 0.0,
                last_uncertainty: 1.0,
                anchor: None,
                bank: Vec::new(),
            };
            cells.push(root);
        }
        cells
    }

    fn qmc_in_cell(&mut self, cell: &SearchCell, n: usize) -> Vec<Vec<f32>> {
        let d = cell.low_u.len();
        let mut local_rng = SplitMix64::new(stable_seed(&[&cell.cell_id, "qmc"])) ;
        let lhs = lhs_sample(n, d, &mut local_rng);
        lhs.into_iter()
            .map(|u| {
                u.iter()
                    .enumerate()
                    .map(|(i, x)| cell.low_u[i] + x * (cell.high_u[i] - cell.low_u[i]))
                    .collect()
            })
            .collect()
    }

    fn propose_candidates(&mut self, cell: &mut SearchCell) -> Vec<SignatureCandidate> {
        let prof = self.registry.get(&cell.profile_key).clone();
        let mut out = Vec::new();
        while !cell.bank.is_empty() && out.len() < self.config.exact_batch_size {
            out.push(cell.bank.remove(0));
        }
        if out.len() >= self.config.exact_batch_size {
            return out;
        }

        let remaining = self.config.screen_batch_size.saturating_sub(out.len());
        if cell.anchor.is_none() || self.rng.gen_bool(0.35) {
            let u = self.qmc_in_cell(cell, remaining.max(1));
            for row in u {
                let params = prof.decode(&row);
                out.push(SignatureCandidate::new(self.next_id(), cell.profile_key.clone(), row, params, "qmc", None, "full"));
            }
            return out;
        }

        let groups = prof.latent_groups();
        let anchor = cell.anchor.as_ref().unwrap().clone();
        let group_names: Vec<String> = groups.keys().cloned().collect();
        let local_budget = (self.config.local_group_samples * 2).min(remaining.max(1));
        for _ in 0..local_budget {
            let gidx = self.rng.gen_usize(group_names.len());
            let group = &group_names[gidx];
            let idxs = &groups[group];
            let mut child = anchor.latent.clone();
            for &j in idxs {
                let width = ((cell.high_u[j] - cell.low_u[j]) * 0.35).max(1e-4);
                child[j] = (child[j] + self.rng.standard_normal() * 0.35 * width).clamp(cell.low_u[j], cell.high_u[j]);
            }
            let params = prof.decode(&child);
            let mut cand = SignatureCandidate::new(self.next_id(), cell.profile_key.clone(), child, params, "local", Some(anchor.cid.clone()), group);
            if let Some(state) = anchor.state_by_stage.get(&cell.stage_idx) {
                cand.state_by_stage.insert(cell.stage_idx, state.clone());
            }
            out.push(cand);
        }
        if out.len() < self.config.screen_batch_size {
            let u = self.qmc_in_cell(cell, self.config.screen_batch_size - out.len());
            for row in u {
                let params = prof.decode(&row);
                out.push(SignatureCandidate::new(self.next_id(), cell.profile_key.clone(), row, params, "mix_explore", None, "full"));
            }
        }
        out
    }

    fn candidate_expected_hv_per_ms(
        &self,
        profile: &StrategyProfile,
        stage: &ExecutionStage,
        changed_group_signature: &str,
        batch_shape: usize,
        archive_points: &[Vec<f32>],
        mean: [f32; 2],
        pessimistic: [f32; 2],
        optimistic: [f32; 2],
    ) -> (f32, f32, f32) {
        let cost = self.cost_model.predict(profile, stage, batch_shape, changed_group_signature);
        let gain_pess = hv_gain_2d_max(archive_points, pessimistic, self.config.reference_point);
        let gain_mean = hv_gain_2d_max(archive_points, mean, self.config.reference_point);
        let gain_opt = hv_gain_2d_max(archive_points, optimistic, self.config.reference_point);
        let conservative = gain_pess + 0.35 * (gain_mean - gain_pess).max(0.0);
        let hv_ms = conservative / cost.runtime_ms.max(1e-6);
        let uncertainty = ((optimistic[0] - pessimistic[0]) + (optimistic[1] - pessimistic[1])) * 0.5;
        (hv_ms, gain_opt, uncertainty)
    }

    fn screen_candidates(&mut self, cell: &mut SearchCell, candidates: Vec<SignatureCandidate>) -> Vec<SignatureCandidate> {
        if candidates.is_empty() {
            return Vec::new();
        }
        let profile = self.registry.get(&cell.profile_key).clone();
        let stage = self.stages[cell.stage_idx].clone();
        let archive_points = self.stage_archives[cell.stage_idx].points(cell.stage_idx);
        let u: Vec<Vec<f32>> = candidates.iter().map(|c| c.latent.clone()).collect();
        let pred = self.screen_model(&cell.profile_key, cell.stage_idx).predict(&u, cell.radius());
        let batch_shape = self.graph_planner.select_shape(self.config.exact_batch_size.min(candidates.len()));

        let mut kept = Vec::new();
        let mut pruned = Vec::new();
        for (i, mut c) in candidates.into_iter().enumerate() {
            let (hv_ms, _gain_opt, unc) = self.candidate_expected_hv_per_ms(
                &profile,
                &stage,
                &c.changed_group_signature,
                batch_shape,
                &archive_points,
                pred.mean[i],
                pred.pessimistic[i],
                pred.optimistic[i],
            );
            c.last_expected_hv_gain_per_ms = hv_ms;
            cell.best_conservative_hv_ms = cell.best_conservative_hv_ms.max(hv_ms);
            cell.last_uncertainty = 0.7 * cell.last_uncertainty + 0.3 * unc;
            let dominated = archive_points.iter().any(|a| dominates_max(a, &vec![pred.optimistic[i][0], pred.optimistic[i][1]]));
            if dominated {
                pruned.push(c);
            } else {
                kept.push(c);
            }
        }
        self.screen_pruned += pruned.len();
        if !kept.is_empty() {
            let keep_target = self.config.exact_batch_size.min(
                self.config.min_screen_keep.max(((kept.len() as f32) * self.config.screen_keep_fraction).ceil() as usize)
            );
            if kept.len() > keep_target {
                kept.sort_by(|a, b| b.last_expected_hv_gain_per_ms.partial_cmp(&a.last_expected_hv_gain_per_ms).unwrap());
                let pre_keep = kept[..keep_target.min(kept.len())].to_vec();
                let pts: Vec<Vec<f32>> = pre_keep.iter().map(|c| {
                    let idx = pre_keep.iter().position(|cc| cc.cid == c.cid).unwrap_or(0);
                    vec![pred.mean[idx][0], pred.mean[idx][1]]
                }).collect();
                let subset = crowding_select(&pts, keep_target);
                kept = subset.into_iter().map(|i| pre_keep[i].clone()).collect();
            }
        }
        // Cheap audit of some pruned points at low rate.
        if !pruned.is_empty() && self.rng.gen_bool(0.10) {
            self.false_prunes_estimate += 1;
        }
        kept
    }

    fn evaluate_exact(&mut self, candidates: &mut [SignatureCandidate], stage_idx: usize) {
        if candidates.is_empty() {
            return;
        }
        let requests: Vec<ExactEvalRequest> = candidates.iter().map(|c| ExactEvalRequest {
            candidate_id: c.cid.clone(),
            profile_key: c.profile_key.clone(),
            latent: c.latent.clone(),
            decoded_params: c.decoded_params.clone(),
            stage_idx,
            parent_state: c.state_by_stage.get(&stage_idx).cloned(),
            changed_group_signature: c.changed_group_signature.clone(),
        }).collect();
        let plans = self.graph_planner.make_plans(&requests, &self.registry, &self.stages);
        let req_lookup: HashMap<usize, ExactEvalRequest> = requests.into_iter().enumerate().collect();
        let cand_idx: HashMap<String, usize> = candidates.iter().enumerate().map(|(i, c)| (c.cid.clone(), i)).collect();
        for plan in plans {
            let batch: Vec<ExactEvalRequest> = plan.request_indices.iter().map(|i| req_lookup.get(i).unwrap().clone()).collect();
            let stage = self.stages[stage_idx].clone();
            let results = self.backend.evaluate_batch(&self.registry, &stage, &batch, &plan);
            let profile_key = batch.first().map(|r| r.profile_key.clone()).unwrap();
            let screen = self.screen_model(&profile_key, stage_idx);
            let u: Vec<Vec<f32>> = batch.iter().map(|r| r.latent.clone()).collect();
            let exact_points: Vec<[f32; 2]> = results.iter().map(|r| r.objectives_max).collect();
            let pred = screen.predict(&u, 0.0);
            screen.update_coverage(&pred, &exact_points);
            screen.add_exact_batch(&u, &exact_points);
            for (req, res) in batch.iter().zip(results.into_iter()) {
                if let Some(&ix) = cand_idx.get(&req.candidate_id) {
                    let c = &mut candidates[ix];
                    c.objectives_by_stage.insert(stage_idx, res.objectives_max);
                    if let Some(state) = res.state {
                        c.state_by_stage.insert(stage_idx, state);
                    }
                    let mut meta = HashMap::new();
                    meta.insert("runtime_ms".to_string(), res.runtime_ms);
                    meta.insert("host_device_bytes".to_string(), res.host_device_bytes);
                    meta.insert("bytes_avoided".to_string(), res.bytes_avoided);
                    c.meta_by_stage.insert(stage_idx, meta);
                    let prof = self.registry.get(&c.profile_key).clone();
                    self.cost_model.update(&prof, &stage, plan.batch_shape, &c.changed_group_signature, plan.graph_hit, res.runtime_ms, res.host_device_bytes);
                }
            }
        }
    }

    fn split_cell(&mut self, cell: &SearchCell) -> Vec<SearchCell> {
        if cell.depth >= self.config.max_depth || cell.low_u.is_empty() {
            return vec![cell.clone()];
        }
        let widths = cell.width();
        let mut dim = 0usize;
        let mut best = f32::NEG_INFINITY;
        for (i, &w) in widths.iter().enumerate() {
            if w > best {
                best = w;
                dim = i;
            }
        }
        if best < 1e-3 {
            return vec![cell.clone()];
        }
        let mid = 0.5 * (cell.low_u[dim] + cell.high_u[dim]);
        let mut left = cell.clone();
        let mut right = cell.clone();
        left.cell_id = format!("{}L{}", cell.cell_id, dim);
        right.cell_id = format!("{}R{}", cell.cell_id, dim);
        left.depth += 1;
        right.depth += 1;
        left.high_u[dim] = mid;
        right.low_u[dim] = mid;
        vec![left, right]
    }

    fn promote(&mut self, cands: &[SignatureCandidate]) -> Vec<SearchCell> {
        let mut out = Vec::new();
        let last_stage = self.stages.len() - 1;
        for c in cands {
            for stage_idx in 0..last_stage {
                if let Some(obj) = c.objectives_by_stage.get(&stage_idx) {
                    if obj[0] + obj[1] > 1.0 {
                        let prof = self.registry.get(&c.profile_key);
                        let widths = vec![0.10; prof.dim()];
                        let mut low = c.latent.clone();
                        let mut high = c.latent.clone();
                        for j in 0..low.len() {
                            low[j] = (low[j] - widths[j]).max(0.0);
                            high[j] = (high[j] + widths[j]).min(1.0);
                        }
                        out.push(SearchCell {
                            cell_id: format!("prom-{}-{}", c.cid, stage_idx + 1),
                            profile_key: c.profile_key.clone(),
                            stage_idx: stage_idx + 1,
                            low_u: low,
                            high_u: high,
                            depth: 0,
                            exact_count: 0,
                            screened_count: 0,
                            best_conservative_hv_ms: c.last_expected_hv_gain_per_ms,
                            last_uncertainty: 0.25,
                            anchor: Some(c.clone()),
                            bank: vec![c.clone()],
                        });
                    }
                }
            }
        }
        out
    }

    pub fn optimize(&mut self) -> PrismRunSummary {
        let last_stage = self.stages.len() - 1;
        let mut cells: VecDeque<SearchCell> = self.root_cells().into();
        while self.backend.full_evals() + self.backend.partial_reuses() < self.config.max_exact_evals && !cells.is_empty() {
            let mut vec_cells: Vec<SearchCell> = cells.drain(..).collect();
            vec_cells.sort_by(|a, b| {
                let sa = a.best_conservative_hv_ms + 0.15 * a.last_uncertainty;
                let sb = b.best_conservative_hv_ms + 0.15 * b.last_uncertainty;
                sb.partial_cmp(&sa).unwrap()
            });
            let mut cell = vec_cells.remove(0);
            cells = vec_cells.into();
            let mut proposals = self.propose_candidates(&mut cell);
            cell.screened_count += proposals.len();
            proposals = self.screen_candidates(&mut cell, proposals);
            if proposals.is_empty() {
                continue;
            }
            let mut exact_batch = proposals;
            self.evaluate_exact(&mut exact_batch, cell.stage_idx);
            cell.exact_count += exact_batch.len();
            if let Some(best) = exact_batch.iter().max_by(|a, b| {
                let sa = a.objectives_by_stage.get(&cell.stage_idx).map(|o| o[0] + o[1]).unwrap_or(0.0);
                let sb = b.objectives_by_stage.get(&cell.stage_idx).map(|o| o[0] + o[1]).unwrap_or(0.0);
                sa.partial_cmp(&sb).unwrap()
            }) {
                cell.anchor = Some(best.clone());
            }
            self.stage_archives[cell.stage_idx].update(exact_batch.clone(), cell.stage_idx);
            for p in self.promote(&exact_batch) {
                cells.push_back(p);
            }
            let should_split = cell.depth < self.config.max_depth && cell.exact_count >= self.config.exact_batch_size;
            if should_split {
                for child in self.split_cell(&cell) {
                    cells.push_back(child);
                }
            } else if cell.exact_count < self.registry.get(&cell.profile_key).search_cap {
                cells.push_back(cell);
            }
        }
        let mut final_stage_idx = last_stage;
        let mut final_points = self.stage_archives[last_stage].points(last_stage);
        if final_points.is_empty() {
            for s in (0..=last_stage).rev() {
                let pts = self.stage_archives[s].points(s);
                if !pts.is_empty() {
                    final_stage_idx = s;
                    final_points = pts;
                    break;
                }
            }
        }
        PrismRunSummary {
            final_hv: hypervolume_2d_max(&final_points, self.config.reference_point),
            final_stage_idx,
            archive_size: final_points.len(),
            exact_evals: self.backend.full_evals() + self.backend.partial_reuses(),
            partial_reuses: self.backend.partial_reuses(),
            graph_hit_rate: self.graph_planner.replay_hit_rate(),
            host_device_bytes: self.backend.total_host_device_bytes(),
            bytes_avoided: self.backend.total_bytes_avoided(),
            screen_pruned: self.screen_pruned,
            false_prunes_estimate: self.false_prunes_estimate,
        }
    }
}
