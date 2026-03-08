use crate::math::{mat_mul, quantile, ridge_fit, stddev};
use crate::rng::{stable_seed, SplitMix64};
use crate::signature::StrategyProfile;
use crate::types::ExecutionStage;

#[derive(Clone, Debug, Default)]
pub struct ScreenPrediction {
    pub mean: Vec<[f32; 2]>,
    pub half_width: Vec<[f32; 2]>,
    pub optimistic: Vec<[f32; 2]>,
    pub pessimistic: Vec<[f32; 2]>,
}

#[derive(Clone, Debug)]
pub struct ConformalScreenModel {
    profile: StrategyProfile,
    _stage: ExecutionStage,
    sketch_dim: usize,
    min_train: usize,
    calibration_frac: f32,
    conformal_alpha: f32,
    x: Vec<Vec<f32>>,
    y: Vec<[f32; 2]>,
    proj: Option<Vec<Vec<f32>>>,
    w: Option<Vec<Vec<f32>>>,
    resid_scale: [f32; 2],
    q_joint: f32,
    q_obj: [f32; 2],
    rng: SplitMix64,
    pub coverage_total: usize,
    pub coverage_joint_hits: usize,
    pub coverage_obj_hits: [usize; 2],
}

impl ConformalScreenModel {
    pub fn new(profile: StrategyProfile, stage: ExecutionStage) -> Self {
        let seed = stable_seed(&[&profile.signature.key(), stage.name, "screen"]);
        Self {
            profile,
            _stage: stage,
            sketch_dim: 48,
            min_train: 18,
            calibration_frac: 0.25,
            conformal_alpha: 0.10,
            x: Vec::new(),
            y: Vec::new(),
            proj: None,
            w: None,
            resid_scale: [0.25, 0.25],
            q_joint: 1.0,
            q_obj: [0.25, 0.25],
            rng: SplitMix64::new(seed),
            coverage_total: 0,
            coverage_joint_hits: 0,
            coverage_obj_hits: [0, 0],
        }
    }

    pub fn ready(&self) -> bool {
        self.x.len() >= self.min_train && self.w.is_some()
    }

    fn phi(&mut self, u: &[Vec<f32>]) -> Vec<Vec<f32>> {
        if u.is_empty() {
            return Vec::new();
        }
        let groups = self.profile.latent_groups();
        let raw_dim = 1 + u[0].len() * 3 + groups.len() * 2;
        let mut raw = vec![vec![0.0; raw_dim]; u.len()];
        for (r, row) in u.iter().enumerate() {
            let mut c = 0usize;
            raw[r][c] = 1.0;
            c += 1;
            for &x in row {
                raw[r][c] = x;
                c += 1;
            }
            for &x in row {
                raw[r][c] = x * x;
                c += 1;
            }
            for &x in row {
                raw[r][c] = (core::f32::consts::PI * x).sin();
                c += 1;
            }
            for idxs in groups.values() {
                let mut vals = Vec::with_capacity(idxs.len());
                for &i in idxs {
                    vals.push(row[i]);
                }
                raw[r][c] = vals.iter().sum::<f32>() / vals.len().max(1) as f32;
                c += 1;
                raw[r][c] = stddev(&vals);
                c += 1;
            }
        }
        if raw_dim <= self.sketch_dim {
            return raw;
        }
        if self.proj.is_none() {
            let mut proj = vec![vec![0.0; self.sketch_dim]; raw_dim];
            let scale = (raw_dim as f32).sqrt();
            for i in 0..raw_dim {
                for j in 0..self.sketch_dim {
                    proj[i][j] = self.rng.standard_normal() / scale;
                }
            }
            self.proj = Some(proj);
        }
        mat_mul(&raw, self.proj.as_ref().unwrap())
    }

    pub fn add_exact_batch(&mut self, u: &[Vec<f32>], y: &[[f32; 2]]) {
        for row in u {
            self.x.push(row.clone());
        }
        for yy in y {
            self.y.push(*yy);
        }
        if self.x.len() >= self.min_train {
            self.refit();
        }
    }

    pub fn refit(&mut self) {
        if self.x.len() < self.min_train {
            return;
        }
        let n = self.x.len();
        let mut idx: Vec<usize> = (0..n).collect();
        self.rng.shuffle(&mut idx);
        let mut n_cal = ((n as f32) * self.calibration_frac).round() as usize;
        n_cal = n_cal.max(8).min(n / 3).max(1);
        let n_tr = n.saturating_sub(n_cal).max(1);
        let tr_idx = &idx[..n_tr];
        let cal_idx = &idx[n_tr..];

        let xtr: Vec<Vec<f32>> = tr_idx.iter().map(|&i| self.x[i].clone()).collect();
        let ytr: Vec<Vec<f32>> = tr_idx.iter().map(|&i| vec![self.y[i][0], self.y[i][1]]).collect();
        let phi_tr = self.phi(&xtr);
        let Some(w) = ridge_fit(&phi_tr, &ytr, 1e-3) else { return; };
        self.w = Some(w);

        let yhat_tr = mat_mul(&phi_tr, self.w.as_ref().unwrap());
        let resid0: Vec<f32> = ytr.iter().zip(yhat_tr.iter()).map(|(y, p)| (y[0] - p[0]).abs()).collect();
        let resid1: Vec<f32> = ytr.iter().zip(yhat_tr.iter()).map(|(y, p)| (y[1] - p[1]).abs()).collect();
        self.resid_scale = [stddev(&resid0).max(1e-4), stddev(&resid1).max(1e-4)];

        if !cal_idx.is_empty() {
            let xcal: Vec<Vec<f32>> = cal_idx.iter().map(|&i| self.x[i].clone()).collect();
            let ycal: Vec<[f32; 2]> = cal_idx.iter().map(|&i| self.y[i]).collect();
            let phi_cal = self.phi(&xcal);
            let yhat = mat_mul(&phi_cal, self.w.as_ref().unwrap());
            let mut scores_joint = Vec::new();
            let mut scores0 = Vec::new();
            let mut scores1 = Vec::new();
            for (yy, pp) in ycal.iter().zip(yhat.iter()) {
                let a0 = (yy[0] - pp[0]).abs();
                let a1 = (yy[1] - pp[1]).abs();
                scores0.push(a0);
                scores1.push(a1);
                scores_joint.push((a0 / self.resid_scale[0]).max(a1 / self.resid_scale[1]));
            }
            self.q_obj = [
                quantile(scores0, 1.0 - self.conformal_alpha),
                quantile(scores1, 1.0 - self.conformal_alpha),
            ];
            self.q_joint = quantile(scores_joint, 1.0 - self.conformal_alpha).max(1.0);
        }
    }

    pub fn predict(&mut self, u: &[Vec<f32>], cell_radius: f32) -> ScreenPrediction {
        if u.is_empty() {
            return ScreenPrediction::default();
        }
        if !self.ready() {
            let n = u.len();
            return ScreenPrediction {
                mean: vec![[0.5, 0.5]; n],
                half_width: vec![[0.5, 0.5]; n],
                optimistic: vec![[1.0, 1.0]; n],
                pessimistic: vec![[0.0, 0.0]; n],
            };
        }
        let phi = self.phi(u);
        let mean_mat = mat_mul(&phi, self.w.as_ref().unwrap());
        let mut pred = ScreenPrediction::default();
        for row in mean_mat {
            let mut half0 = (self.resid_scale[0] * self.q_joint).max(self.q_obj[0]);
            let mut half1 = (self.resid_scale[1] * self.q_joint).max(self.q_obj[1]);
            half0 += 0.20 * cell_radius;
            half1 += 0.20 * cell_radius;
            let mean = [row[0].clamp(0.0, 1.0), row[1].clamp(0.0, 1.0)];
            let half = [half0.max(1e-4), half1.max(1e-4)];
            pred.mean.push(mean);
            pred.half_width.push(half);
            pred.optimistic.push([(mean[0] + half[0]).clamp(0.0, 1.0), (mean[1] + half[1]).clamp(0.0, 1.0)]);
            pred.pessimistic.push([(mean[0] - half[0]).clamp(0.0, 1.0), (mean[1] - half[1]).clamp(0.0, 1.0)]);
        }
        pred
    }

    pub fn update_coverage(&mut self, pred: &ScreenPrediction, exact: &[[f32; 2]]) {
        for (pp, yy) in pred.mean.iter().zip(exact.iter()) {
            let half = pred.half_width[pred.mean.iter().position(|m| std::ptr::eq(m, pp)).unwrap_or(0)];
            let o0 = yy[0] >= pp[0] - half[0] && yy[0] <= pp[0] + half[0];
            let o1 = yy[1] >= pp[1] - half[1] && yy[1] <= pp[1] + half[1];
            self.coverage_total += 1;
            self.coverage_obj_hits[0] += o0 as usize;
            self.coverage_obj_hits[1] += o1 as usize;
            self.coverage_joint_hits += (o0 && o1) as usize;
        }
    }
}
