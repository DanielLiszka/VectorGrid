use crate::math::ridge_fit;
use crate::signature::StrategyProfile;
use crate::types::ExecutionStage;

#[derive(Clone, Copy, Debug, Default)]
pub struct CostPrediction {
    pub runtime_ms: f32,
    pub host_device_bytes: f32,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeCostModel {
    x: Vec<Vec<f32>>,
    y_ms: Vec<Vec<f32>>,
    y_bytes: Vec<Vec<f32>>,
    w_ms: Option<Vec<f32>>,
    w_bytes: Option<Vec<f32>>,
}

impl RuntimeCostModel {
    fn features(
        profile: &StrategyProfile,
        stage: &ExecutionStage,
        batch_shape: usize,
        changed_group_signature: &str,
        graph_hit: bool,
    ) -> Vec<f32> {
        let touched = if changed_group_signature == "full" {
            3.0
        } else {
            changed_group_signature.split('+').count() as f32
        };
        let partial = if changed_group_signature == "full" { 0.0 } else { 1.0 };
        vec![
            1.0,
            profile.indicator_count as f32,
            profile.kernel_families.len() as f32,
            stage.candle_fraction,
            stage.asset_fraction,
            stage.fold_fraction,
            batch_shape as f32,
            touched,
            partial,
            if graph_hit { 1.0 } else { 0.0 },
        ]
    }

    pub fn predict(
        &self,
        profile: &StrategyProfile,
        stage: &ExecutionStage,
        batch_shape: usize,
        changed_group_signature: &str,
    ) -> CostPrediction {
        let cold = self.x.len() < 24 || self.w_ms.is_none() || self.w_bytes.is_none();
        if cold {
            let partial_bonus = if changed_group_signature == "full" { 1.0 } else { 0.50 };
            let runtime_ms = (0.010
                + 0.004 * profile.kernel_families.len() as f32
                + 0.0008 * profile.indicator_count as f32
                + 0.030 * stage.scale())
                * partial_bonus
                * 0.94;
            let host_device_bytes = batch_shape as f32 * ((profile.numeric_specs.len() * 4 + 16) as f32) * partial_bonus;
            return CostPrediction { runtime_ms, host_device_bytes };
        }
        let x = Self::features(profile, stage, batch_shape, changed_group_signature, true);
        let ms = dot(self.w_ms.as_ref().unwrap(), &x).max(1e-4);
        let by = dot(self.w_bytes.as_ref().unwrap(), &x).max(1.0);
        CostPrediction { runtime_ms: ms, host_device_bytes: by }
    }

    pub fn update(
        &mut self,
        profile: &StrategyProfile,
        stage: &ExecutionStage,
        batch_shape: usize,
        changed_group_signature: &str,
        graph_hit: bool,
        observed_runtime_ms: f32,
        observed_bytes: f32,
    ) {
        self.x.push(Self::features(profile, stage, batch_shape, changed_group_signature, graph_hit));
        self.y_ms.push(vec![observed_runtime_ms]);
        self.y_bytes.push(vec![observed_bytes]);
        if self.x.len() >= 12 {
            self.refit();
        }
    }

    pub fn refit(&mut self) {
        if let Some(wms) = ridge_fit(&self.x, &self.y_ms, 1e-3) {
            self.w_ms = Some(wms.into_iter().map(|v| v[0]).collect());
        }
        if let Some(wb) = ridge_fit(&self.x, &self.y_bytes, 1e-3) {
            self.w_bytes = Some(wb.into_iter().map(|v| v[0]).collect());
        }
    }
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}
