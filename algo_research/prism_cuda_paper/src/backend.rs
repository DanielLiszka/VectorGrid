use std::collections::HashMap;

use crate::rng::{stable_seed, SplitMix64};
use crate::signature::{EntryModule, RegimeGate, RiskOverlay, StrategyProfile, StrategyRegistry};
use crate::types::{CandidateState, ExactEvalRequest, ExactEvalResult, ExecutionStage, GraphPlan};

pub trait BacktestGpuBackend {
    fn evaluate_batch(
        &mut self,
        registry: &StrategyRegistry,
        stage: &ExecutionStage,
        requests: &[ExactEvalRequest],
        plan: &GraphPlan,
    ) -> Vec<ExactEvalResult>;

    fn total_runtime_ms(&self) -> f32;
    fn total_host_device_bytes(&self) -> f32;
    fn total_bytes_avoided(&self) -> f32;
    fn full_evals(&self) -> usize;
    fn partial_reuses(&self) -> usize;
}

#[derive(Clone, Debug, Default)]
pub struct PtxLaunchManifest {
    pub screen_encode_kernel: &'static str,
    pub screen_predict_kernel: &'static str,
    pub set_gate_kernel: &'static str,
    pub vtrs_gate_kernel: &'static str,
    pub vcc_entry_kernel: &'static str,
    pub vemr_entry_kernel: &'static str,
    pub atr_risk_kernel: &'static str,
    pub devstop_risk_kernel: &'static str,
    pub chandelier_risk_kernel: &'static str,
    pub compose_signal_kernel: &'static str,
    pub backtest_metrics_kernel: &'static str,
}

impl PtxLaunchManifest {
    pub fn for_4090() -> Self {
        Self {
            screen_encode_kernel: "prism_encode_latent_features_kernel",
            screen_predict_kernel: "prism_tc_linear_predict_kernel",
            set_gate_kernel: "prism_compute_set_gate_kernel",
            vtrs_gate_kernel: "prism_compute_vtrs_gate_kernel",
            vcc_entry_kernel: "prism_compute_vcc_entry_kernel",
            vemr_entry_kernel: "prism_compute_vemr_entry_kernel",
            atr_risk_kernel: "prism_compute_atr_risk_kernel",
            devstop_risk_kernel: "prism_compute_devstop_risk_kernel",
            chandelier_risk_kernel: "prism_compute_chandelier_risk_kernel",
            compose_signal_kernel: "prism_compose_signal_kernel",
            backtest_metrics_kernel: "prism_backtest_metrics_kernel",
        }
    }
}

#[derive(Clone, Debug)]
pub struct MockBackend {
    rng: SplitMix64,
    candle_count: usize,
    asset_count: usize,
    fold_count: usize,
    pub total_runtime_ms_acc: f32,
    pub total_host_device_bytes_acc: f32,
    pub total_bytes_avoided_acc: f32,
    pub full_evals_acc: usize,
    pub partial_reuses_acc: usize,
}

impl MockBackend {
    pub fn new(seed: u64, candle_count: usize, asset_count: usize, fold_count: usize) -> Self {
        Self {
            rng: SplitMix64::new(seed),
            candle_count,
            asset_count,
            fold_count,
            total_runtime_ms_acc: 0.0,
            total_host_device_bytes_acc: 0.0,
            total_bytes_avoided_acc: 0.0,
            full_evals_acc: 0,
            partial_reuses_acc: 0,
        }
    }

    fn module_target(&self, signature_key: &str, module: &str, dim: usize) -> Vec<f32> {
        let mut rr = SplitMix64::new(stable_seed(&[signature_key, module]));
        (0..dim).map(|_| rr.gen_range_f32(0.2, 0.8)).collect()
    }

    fn module_score(
        &self,
        profile: &StrategyProfile,
        signature_key: &str,
        module: &str,
        latent: &[f32],
    ) -> [f32; 2] {
        let groups = profile.latent_groups();
        let idxs = groups.get(module).cloned().unwrap_or_default();
        if idxs.is_empty() {
            return [0.5, 0.5];
        }
        let target = self.module_target(signature_key, module, idxs.len());
        let mut reward = 0.0;
        let mut robust = 0.0;
        for (k, i) in idxs.iter().enumerate() {
            let d = (latent[*i] - target[k]).abs();
            reward += (1.0 - d / 0.28).max(0.0);
            robust += (-(d * d) / 0.05).exp();
            if profile.numeric_specs[*i].quantile_normalized {
                robust += 0.015;
            }
        }
        let n = idxs.len() as f32;
        [reward / n, (robust / n).min(1.0)]
    }

    fn interaction_bonus(&self, profile: &StrategyProfile, scores: &HashMap<String, [f32; 2]>) -> [f32; 2] {
        let rg = scores.get("regime").copied().unwrap_or([0.5, 0.5]);
        let eg = scores.get("entry").copied().unwrap_or([0.5, 0.5]);
        let kg = scores.get("risk").copied().unwrap_or([0.5, 0.5]);
        let mut synergy = 0.08 * rg[0].min(eg[0]) + 0.05 * eg[0].min(kg[0]);
        let mut stability = 0.10 * rg[1].min(eg[1]) + 0.06 * eg[1].min(kg[1]);
        match profile.signature.entry_module {
            EntryModule::Abmt => {
                synergy += 0.03;
                stability += 0.02;
            }
            EntryModule::Tdb => {
                synergy += 0.09;
                stability -= 0.06;
            }
            EntryModule::Vcc => {
                synergy += 0.08;
                stability -= 0.05;
            }
            EntryModule::Vemr => {
                synergy += 0.02;
                stability += 0.03;
            }
        }
        match profile.signature.regime_gate {
            RegimeGate::None => {
                synergy += 0.05;
                stability -= 0.05;
            }
            RegimeGate::SetGate => {
                synergy += 0.02;
                stability += 0.01;
            }
            RegimeGate::VtrsGate => {
                synergy -= 0.03;
                stability += 0.07;
            }
        }
        match profile.signature.risk_overlay {
            RiskOverlay::Atr => {
                synergy += 0.01;
                stability += 0.02;
            }
            RiskOverlay::DevStop => {
                synergy -= 0.03;
                stability += 0.07;
            }
            RiskOverlay::Chandelier => {
                synergy -= 0.01;
                stability += 0.05;
            }
        }
        [synergy, stability]
    }

    fn materialize_objectives(
        &mut self,
        profile: &StrategyProfile,
        stage: &ExecutionStage,
        module_scores: &HashMap<String, [f32; 2]>,
    ) -> [f32; 2] {
        let n = module_scores.len().max(1) as f32;
        let reward = module_scores.values().map(|v| v[0]).sum::<f32>() / n;
        let robust = module_scores.values().map(|v| v[1]).sum::<f32>() / n;
        let bonus = self.interaction_bonus(profile, module_scores);
        let bias = 0.04 * (1.0 - stage.resource);
        let noise = 0.015 * (1.0 - stage.resource);
        let obj0 = (reward + bonus[0] - 0.02 * bias + self.rng.standard_normal() * noise).clamp(0.0, 1.0);
        let obj1 = (robust + bonus[1] - 0.03 * bias + self.rng.standard_normal() * noise).clamp(0.0, 1.0);
        [obj0, obj1]
    }

    fn runtime_baseline(&self, profile: &StrategyProfile, stage: &ExecutionStage, batch_shape: usize) -> (f32, f32) {
        let candle_eff = self.candle_count as f32 * stage.candle_fraction;
        let asset_eff = self.asset_count as f32 * stage.asset_fraction;
        let fold_eff = self.fold_count as f32 * stage.fold_fraction;
        let kernel_factor = 1.0 + 0.12 * profile.kernel_families.len() as f32;
        let graph_factor = 1.0 + 0.02 * (batch_shape.max(1) as f32).log2();
        let runtime_ms = (0.0015 * profile.indicator_count as f32 * candle_eff / 1_000.0
            + 0.0045 * asset_eff * fold_eff
            + 0.018 * profile.kernel_families.len() as f32)
            * kernel_factor
            * graph_factor;
        let baseline_bytes = batch_shape as f32 * (profile.numeric_specs.len() as f32 * 4.0)
            + profile.indicator_count as f32 * candle_eff * 4.0 * 0.04
            + batch_shape as f32 * 2.0 * 4.0;
        (runtime_ms, baseline_bytes)
    }
}

impl BacktestGpuBackend for MockBackend {
    fn evaluate_batch(
        &mut self,
        registry: &StrategyRegistry,
        stage: &ExecutionStage,
        requests: &[ExactEvalRequest],
        plan: &GraphPlan,
    ) -> Vec<ExactEvalResult> {
        if requests.is_empty() {
            return Vec::new();
        }
        let prof = registry.get(&requests[0].profile_key);
        let (base_ms, base_bytes) = self.runtime_baseline(prof, stage, plan.batch_shape);
        let mut out = Vec::with_capacity(requests.len());
        for req in requests {
            let partial = req.parent_state.is_some() && req.changed_group_signature != "full";
            let mut module_scores = req.parent_state.as_ref().map(|s| s.module_scores.clone()).unwrap_or_default();
            let groups: Vec<String> = if partial {
                req.changed_group_signature.split('+').map(|s| s.to_string()).collect()
            } else {
                prof.latent_groups().keys().cloned().collect()
            };
            if partial { self.partial_reuses_acc += 1; } else { self.full_evals_acc += 1; }
            for g in groups.iter() {
                module_scores.insert(g.clone(), self.module_score(prof, &req.profile_key, g, &req.latent));
            }
            let obj = self.materialize_objectives(prof, stage, &module_scores);
            let mut runtime_ms = base_ms / plan.batch_shape.max(1) as f32;
            let mut host_bytes = base_bytes / plan.batch_shape.max(1) as f32;
            let mut bytes_avoided = 0.0f32;
            if partial {
                let touched = req.changed_group_signature.split('+').count().max(1) as f32;
                let total = prof.latent_groups().len().max(1) as f32;
                let reuse_frac = 1.0 - (0.15 + 0.70 * (touched / total)).min(0.90);
                runtime_ms *= 1.0 - 0.55 * reuse_frac;
                bytes_avoided += host_bytes * 0.70 * reuse_frac;
                host_bytes *= 1.0 - 0.70 * reuse_frac;
            }
            if plan.graph_hit {
                runtime_ms *= 0.92;
                bytes_avoided += host_bytes * 0.08;
                host_bytes *= 0.92;
            }
            let active_adjust = (plan.batch_shape as f32 / plan.active_count.max(1) as f32).max(1.0);
            runtime_ms *= active_adjust;
            host_bytes *= active_adjust;
            self.total_runtime_ms_acc += runtime_ms;
            self.total_host_device_bytes_acc += host_bytes;
            self.total_bytes_avoided_acc += bytes_avoided;
            let mut metadata = HashMap::new();
            metadata.insert("runtime_ms".to_string(), runtime_ms);
            metadata.insert("host_device_bytes".to_string(), host_bytes);
            metadata.insert("bytes_avoided".to_string(), bytes_avoided);
            let mut state = CandidateState::default();
            state.module_scores = module_scores;
            for (i, name) in ["regime", "entry", "risk"].iter().enumerate() {
                state.cache_handles.insert((*name).to_string(), (stable_seed(&[&req.profile_key, name]) + i as u64) as u64);
            }
            out.push(ExactEvalResult {
                objectives_max: obj,
                state: Some(state),
                runtime_ms,
                host_device_bytes: host_bytes,
                bytes_avoided,
                partial_reuse: partial,
                graph_key: plan.graph_key.clone(),
                graph_hit: plan.graph_hit,
                metadata,
            });
        }
        out
    }

    fn total_runtime_ms(&self) -> f32 { self.total_runtime_ms_acc }
    fn total_host_device_bytes(&self) -> f32 { self.total_host_device_bytes_acc }
    fn total_bytes_avoided(&self) -> f32 { self.total_bytes_avoided_acc }
    fn full_evals(&self) -> usize { self.full_evals_acc }
    fn partial_reuses(&self) -> usize { self.partial_reuses_acc }
}
