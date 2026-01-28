use rand::prelude::*;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sobol::{params::JoeKuoD6, Sobol};
use std::{cmp::Ordering, collections::HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ParamDomain {
    IntRange {
        low: i64,
        high: i64,
        #[serde(default = "default_step")]
        step: i64,
        #[serde(default)]
        log: bool,
    },
    FloatRange {
        low: f64,
        high: f64,
        #[serde(default)]
        log: bool,
    },
    Categorical {
        choices: Vec<String>,
    },
    Bool,
}

fn default_step() -> i64 {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub domain: ParamDomain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchSpace {
    pub params: Vec<ParamDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ParamValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Cat(usize),
}

fn values_hash(values: &[ParamValue]) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for v in values {
        let (tag, x): (u64, u64) = match v {
            ParamValue::Int(i) => (1, *i as u64),
            ParamValue::Float(f) => (2, f.to_bits()),
            ParamValue::Bool(b) => (3, if *b { 1 } else { 0 }),
            ParamValue::Cat(i) => (4, *i as u64),
        };
        let k = x
            .wrapping_add(tag.wrapping_mul(0xD6E8_FEB8_6659_FD93))
            .wrapping_add(0xA076_1D64_78BD_642F);
        h ^= k;
        h = h.rotate_left(27).wrapping_mul(0x3C79_AC49_2BA7_B653);
        h = h.wrapping_add(0x1C69_B3F7_4AC4_A3C1);
    }
    h
}

impl SearchSpace {
    pub fn validate(&self) -> Result<(), String> {
        if self.params.is_empty() {
            return Err("search space has no params".to_string());
        }
        for p in &self.params {
            let name = p.name.trim();
            if name.is_empty() {
                return Err("param name cannot be empty".to_string());
            }
            match &p.domain {
                ParamDomain::IntRange {
                    low,
                    high,
                    step,
                    log,
                } => {
                    if *step <= 0 {
                        return Err(format!("param '{name}': step must be > 0"));
                    }
                    let (lo, hi) = if low <= high {
                        (*low, *high)
                    } else {
                        (*high, *low)
                    };
                    if *log && lo <= 0 {
                        return Err(format!(
                            "param '{name}': log=true requires integer range > 0"
                        ));
                    }
                    if lo == hi {
                        continue;
                    }
                    let span = hi
                        .checked_sub(lo)
                        .ok_or_else(|| format!("param '{name}': range overflow"))?;
                    if span / step == 0 {
                        return Err(format!(
                            "param '{name}': step too large for range [{lo}, {hi}]"
                        ));
                    }
                }
                ParamDomain::FloatRange { low, high, log } => {
                    if !low.is_finite() || !high.is_finite() {
                        return Err(format!("param '{name}': float bounds must be finite"));
                    }
                    let (lo, hi) = if low <= high {
                        (*low, *high)
                    } else {
                        (*high, *low)
                    };
                    if lo == hi {
                        continue;
                    }
                    if *log && lo <= 0.0 {
                        return Err(format!("param '{name}': log=true requires float range > 0"));
                    }
                }
                ParamDomain::Categorical { choices } => {
                    if choices.is_empty() {
                        return Err(format!("param '{name}': categorical has no choices"));
                    }
                }
                ParamDomain::Bool => {}
            }
        }
        Ok(())
    }

    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<ParamValue> {
        self.params
            .iter()
            .map(|p| sample_one(&p.domain, rng))
            .collect()
    }

    pub fn sample_param<R: Rng + ?Sized>(&self, param_idx: usize, rng: &mut R) -> ParamValue {
        match self.params.get(param_idx) {
            Some(p) => sample_one(&p.domain, rng),
            None => ParamValue::Bool(rng.gen_bool(0.5)),
        }
    }

    /// Expand each parameter domain into a discrete list and iterate the full Cartesian product.
    ///
    /// This is intended for **deterministic grid runs** over small/medium discrete spaces (vectorbt-style
    /// "param_product"). It will refuse to enumerate:
    /// - any `FloatRange` with `low != high` (no step is defined in `ParamDomain`)
    /// - any axis with more than `max_axis_values` discrete values
    /// - any grid whose total combinations exceeds `max_combos`
    pub fn grid_iter(
        &self,
        max_axis_values: usize,
        max_combos: usize,
    ) -> Result<SearchSpaceGridIter, String> {
        self.validate()?;

        let mut axes: Vec<Vec<ParamValue>> = Vec::with_capacity(self.params.len());
        for p in &self.params {
            axes.push(domain_grid_values(&p.name, &p.domain, max_axis_values)?);
        }

        let len = grid_len_checked(&axes, max_combos)?;
        Ok(SearchSpaceGridIter::new(axes, len))
    }

    /// Expand each parameter domain into a discrete list and expose the full Cartesian product as
    /// a stable, indexable grid.
    pub fn grid(&self, max_axis_values: usize, max_combos: usize) -> Result<SearchSpaceGrid, String> {
        self.validate()?;

        let mut axes: Vec<Vec<ParamValue>> = Vec::with_capacity(self.params.len());
        for p in &self.params {
            axes.push(domain_grid_values(&p.name, &p.domain, max_axis_values)?);
        }

        let len = grid_len_checked(&axes, max_combos)?;
        SearchSpaceGrid::new(axes, len)
    }
}

fn domain_grid_values(
    name: &str,
    domain: &ParamDomain,
    max_axis_values: usize,
) -> Result<Vec<ParamValue>, String> {
    let max_axis_values = max_axis_values.max(1);
    match domain {
        ParamDomain::IntRange {
            low,
            high,
            step,
            log: _,
        } => {
            let step = *step;
            if step <= 0 {
                return Err(format!("param '{name}': step must be > 0"));
            }
            let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
            let span = hi
                .checked_sub(lo)
                .ok_or_else(|| format!("param '{name}': range overflow"))?;
            let steps = (span / step).saturating_add(1);
            if steps <= 0 {
                return Err(format!("param '{name}': empty integer grid"));
            }
            let steps_usize: usize = usize::try_from(steps)
                .map_err(|_| format!("param '{name}': integer grid too large"))?;
            if steps_usize > max_axis_values {
                return Err(format!(
                    "param '{name}': {steps_usize} grid values exceeds max_axis_values={max_axis_values}"
                ));
            }
            let mut out = Vec::with_capacity(steps_usize);
            let mut v = lo;
            while v <= hi {
                out.push(ParamValue::Int(v));
                v = v.saturating_add(step);
                if out.len() > max_axis_values {
                    break;
                }
            }
            if out.is_empty() {
                return Err(format!("param '{name}': empty integer grid"));
            }
            Ok(out)
        }
        ParamDomain::FloatRange { low, high, log: _ } => {
            if low == high {
                Ok(vec![ParamValue::Float(*low)])
            } else {
                Err(format!(
                    "param '{name}': FloatRange cannot be enumerated as a grid (no step); use sampling"
                ))
            }
        }
        ParamDomain::Categorical { choices } => {
            if choices.is_empty() {
                return Err(format!("param '{name}': categorical has no choices"));
            }
            if choices.len() > max_axis_values {
                return Err(format!(
                    "param '{name}': {} grid values exceeds max_axis_values={max_axis_values}",
                    choices.len()
                ));
            }
            Ok((0..choices.len()).map(|i| ParamValue::Cat(i)).collect())
        }
        ParamDomain::Bool => Ok(vec![ParamValue::Bool(false), ParamValue::Bool(true)]),
    }
}

fn grid_len_checked(axes: &[Vec<ParamValue>], max_combos: usize) -> Result<usize, String> {
    if axes.is_empty() {
        return Err("grid has no axes".to_string());
    }
    let max_combos = max_combos.max(1);
    let mut prod: usize = 1;
    for a in axes {
        if a.is_empty() {
            return Err("grid axis is empty".to_string());
        }
        prod = prod
            .checked_mul(a.len())
            .ok_or_else(|| "grid size overflow".to_string())?;
        if prod > max_combos {
            return Err(format!(
                "grid has {prod} combinations which exceeds max_combos={max_combos}"
            ));
        }
    }
    Ok(prod)
}

#[derive(Debug, Clone)]
pub struct SearchSpaceGrid {
    axes: Vec<Vec<ParamValue>>,
    strides: Vec<usize>,
    len: usize,
}

impl SearchSpaceGrid {
    fn new(axes: Vec<Vec<ParamValue>>, len: usize) -> Result<Self, String> {
        if axes.is_empty() {
            return Err("grid has no axes".to_string());
        }
        let mut strides: Vec<usize> = vec![1usize; axes.len()];
        let mut acc: usize = 1;
        for axis in (0..axes.len()).rev() {
            strides[axis] = acc;
            acc = acc
                .checked_mul(axes[axis].len())
                .ok_or_else(|| "grid size overflow".to_string())?;
        }
        if acc != len {
            return Err("grid stride mismatch".to_string());
        }
        Ok(Self { axes, strides, len })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, col: usize) -> Option<Vec<ParamValue>> {
        if col >= self.len {
            return None;
        }
        let mut out: Vec<ParamValue> = Vec::with_capacity(self.axes.len());
        for axis in 0..self.axes.len() {
            let stride = self.strides.get(axis).copied().unwrap_or(1);
            let axis_len = self.axes.get(axis)?.len();
            if axis_len == 0 {
                return None;
            }
            let idx = (col / stride) % axis_len;
            out.push(self.axes.get(axis)?.get(idx)?.clone());
        }
        Some(out)
    }

    pub fn iter(&self) -> SearchSpaceGridIterRef<'_> {
        SearchSpaceGridIterRef { grid: self, col: 0 }
    }

    pub fn chunks(&self, chunk_size: usize) -> SearchSpaceGridChunksIter<'_> {
        SearchSpaceGridChunksIter {
            grid: self,
            col: 0,
            chunk_size: chunk_size.max(1),
        }
    }
}

pub struct SearchSpaceGridIterRef<'a> {
    grid: &'a SearchSpaceGrid,
    col: usize,
}

impl<'a> Iterator for SearchSpaceGridIterRef<'a> {
    type Item = Vec<ParamValue>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.grid.get(self.col)?;
        self.col = self.col.saturating_add(1);
        Some(out)
    }
}

pub struct SearchSpaceGridChunksIter<'a> {
    grid: &'a SearchSpaceGrid,
    col: usize,
    chunk_size: usize,
}

impl<'a> Iterator for SearchSpaceGridChunksIter<'a> {
    type Item = Vec<Vec<ParamValue>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.col >= self.grid.len() {
            return None;
        }
        let end = (self.col.saturating_add(self.chunk_size)).min(self.grid.len());
        let mut out: Vec<Vec<ParamValue>> = Vec::with_capacity(end - self.col);
        for c in self.col..end {
            if let Some(v) = self.grid.get(c) {
                out.push(v);
            }
        }
        self.col = end;
        Some(out)
    }
}

#[derive(Debug)]
pub struct SearchSpaceGridIter {
    axes: Vec<Vec<ParamValue>>,
    idx: Vec<usize>,
    remaining: usize,
    done: bool,
}

impl SearchSpaceGridIter {
    fn new(axes: Vec<Vec<ParamValue>>, len: usize) -> Self {
        Self {
            idx: vec![0usize; axes.len()],
            axes,
            remaining: len,
            done: false,
        }
    }

    pub fn len(&self) -> usize {
        self.remaining
    }

    fn advance(&mut self) {
        if self.idx.is_empty() {
            self.done = true;
            return;
        }
        for axis in (0..self.idx.len()).rev() {
            let next = self.idx[axis].saturating_add(1);
            if next < self.axes[axis].len() {
                self.idx[axis] = next;
                return;
            }
            self.idx[axis] = 0;
            if axis == 0 {
                self.done = true;
                return;
            }
        }
    }
}

impl Iterator for SearchSpaceGridIter {
    type Item = Vec<ParamValue>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.remaining == 0 {
            return None;
        }

        let mut out: Vec<ParamValue> = Vec::with_capacity(self.axes.len());
        for (axis, values) in self.axes.iter().enumerate() {
            out.push(values[self.idx[axis]].clone());
        }

        self.remaining = self.remaining.saturating_sub(1);
        self.advance();
        Some(out)
    }
}

fn sample_one<R: Rng + ?Sized>(domain: &ParamDomain, rng: &mut R) -> ParamValue {
    match domain {
        ParamDomain::IntRange {
            low,
            high,
            step,
            log,
        } => {
            let (lo, hi) = if low <= high {
                (*low, *high)
            } else {
                (*high, *low)
            };
            if lo == hi {
                return ParamValue::Int(lo);
            }
            if *log {
                // Sample uniformly in log-space on the discrete grid.
                let lo_f = (lo as f64).ln();
                let hi_f = (hi as f64).ln();
                let v = rng.gen_range(lo_f..=hi_f).exp();
                let mut x = v.round() as i64;
                x = x.clamp(lo, hi);
                if *step > 1 {
                    x = lo + ((x - lo) / step).clamp(0, (hi - lo) / step) * step;
                }
                ParamValue::Int(x)
            } else {
                let span = (hi - lo) as u64;
                let steps = (span / (*step as u64)).max(1);
                let k = rng.gen_range(0..=steps);
                let x = lo.saturating_add((k as i64).saturating_mul(*step));
                ParamValue::Int(x.clamp(lo, hi))
            }
        }
        ParamDomain::FloatRange { low, high, log } => {
            let (lo, hi) = if low <= high {
                (*low, *high)
            } else {
                (*high, *low)
            };
            if lo == hi {
                return ParamValue::Float(lo);
            }
            if *log {
                let v = rng.gen_range(lo.ln()..=hi.ln()).exp();
                ParamValue::Float(v.clamp(lo, hi))
            } else {
                ParamValue::Float(rng.gen_range(lo..=hi))
            }
        }
        ParamDomain::Categorical { choices } => ParamValue::Cat(rng.gen_range(0..choices.len())),
        ParamDomain::Bool => ParamValue::Bool(rng.gen_bool(0.5)),
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Maximize,
    Minimize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Objectives2 {
    pub obj1: ObjectiveDirection,
    pub obj2: ObjectiveDirection,
}

impl Objectives2 {
    pub fn normalize(&self, v1: f64, v2: f64) -> [f64; 2] {
        let x1 = if v1.is_finite() {
            v1
        } else {
            f64::NEG_INFINITY
        };
        let x2 = if v2.is_finite() {
            v2
        } else {
            f64::NEG_INFINITY
        };
        [
            match self.obj1 {
                ObjectiveDirection::Maximize => x1,
                ObjectiveDirection::Minimize => -x1,
            },
            match self.obj2 {
                ObjectiveDirection::Maximize => x2,
                ObjectiveDirection::Minimize => -x2,
            },
        ]
    }
}

fn overall_violation(constraints: &[f64]) -> f64 {
    constraints
        .iter()
        .copied()
        .map(|v| if v.is_finite() { v } else { 0.0 })
        .map(|v| v.max(0.0))
        .sum::<f64>()
}

#[derive(Clone, Debug)]
pub struct EvalOutcome<T> {
    pub objectives: [f64; 2],  // should already be normalized to "maximize both"
    pub constraints: Vec<f64>, // <=0 is feasible; >0 is violation amount
    pub payload: T,
}

#[derive(Clone, Debug)]
pub struct MoPoint<T> {
    pub values: Vec<ParamValue>,
    pub objectives: [f64; 2], // normalized to maximize
    pub violation: f64,
    pub payload: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Nsga2Config {
    pub seed: u64,
    pub population_size: usize,
    pub generations: usize,
    #[serde(default = "default_crossover_prob")]
    pub crossover_prob: f64,
    #[serde(default = "default_swapping_prob")]
    pub swapping_prob: f64,
    pub mutation_prob: Option<f64>,
}

fn default_crossover_prob() -> f64 {
    0.9
}

fn default_swapping_prob() -> f64 {
    0.5
}

#[derive(Clone, Debug)]
pub struct Nsga2Result<T> {
    pub population: Vec<MoPoint<T>>,
    pub pareto: Vec<MoPoint<T>>,
    pub total_evaluations: usize,
}

fn constrained_dominates<T>(a: &MoPoint<T>, b: &MoPoint<T>) -> bool {
    let a_feas = a.violation <= 0.0;
    let b_feas = b.violation <= 0.0;

    if a_feas && !b_feas {
        return true;
    }
    if !a_feas && b_feas {
        return false;
    }
    if !a_feas && !b_feas {
        return a.violation < b.violation;
    }

    let a0 = a.objectives[0];
    let a1 = a.objectives[1];
    let b0 = b.objectives[0];
    let b1 = b.objectives[1];

    (a0 >= b0 && a1 >= b1) && (a0 > b0 || a1 > b1)
}

fn non_dominated_sort<T>(pop: &[MoPoint<T>]) -> Vec<Vec<usize>> {
    let n = pop.len();
    let mut dominates: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut dominated_count: Vec<usize> = vec![0; n];

    for i in 0..n {
        for j in (i + 1)..n {
            if constrained_dominates(&pop[i], &pop[j]) {
                dominates[i].push(j);
                dominated_count[j] = dominated_count[j].saturating_add(1);
            } else if constrained_dominates(&pop[j], &pop[i]) {
                dominates[j].push(i);
                dominated_count[i] = dominated_count[i].saturating_add(1);
            }
        }
    }

    let mut fronts: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = dominated_count
        .iter()
        .enumerate()
        .filter_map(|(idx, &c)| if c == 0 { Some(idx) } else { None })
        .collect();

    while !current.is_empty() {
        fronts.push(current.clone());
        let mut next: Vec<usize> = Vec::new();
        for &p in &current {
            for &q in &dominates[p] {
                dominated_count[q] = dominated_count[q].saturating_sub(1);
                if dominated_count[q] == 0 {
                    next.push(q);
                }
            }
        }
        current = next;
    }

    fronts
}

fn pareto_front_constrained_indices<T>(pop: &[MoPoint<T>]) -> Vec<usize> {
    if pop.is_empty() {
        return Vec::new();
    }

    let mut feasible: Vec<usize> = pop
        .iter()
        .enumerate()
        .filter_map(|(i, p)| if p.violation <= 0.0 { Some(i) } else { None })
        .collect();

    if feasible.is_empty() {
        let mut min_v = f64::INFINITY;
        for p in pop {
            if p.violation.is_finite() && p.violation < min_v {
                min_v = p.violation;
            }
        }
        if !min_v.is_finite() {
            min_v = f64::INFINITY;
        }
        let eps = 1e-12f64;
        return pop
            .iter()
            .enumerate()
            .filter_map(|(i, p)| if (p.violation - min_v).abs() <= eps { Some(i) } else { None })
            .collect();
    }

    // 2D Pareto front in O(n log n) for "maximize both":
    // sort by obj0 desc, then obj1 desc; keep points with strictly increasing obj1.
    feasible.sort_by(|&a, &b| {
        let pa = &pop[a];
        let pb = &pop[b];
        let o0 = pb.objectives[0]
            .partial_cmp(&pa.objectives[0])
            .unwrap_or(Ordering::Equal);
        if o0 != Ordering::Equal {
            return o0;
        }
        pb.objectives[1]
            .partial_cmp(&pa.objectives[1])
            .unwrap_or(Ordering::Equal)
    });

    let mut front: Vec<usize> = Vec::new();
    let mut best_obj1 = f64::NEG_INFINITY;
    for &idx in &feasible {
        let o1 = pop[idx].objectives[1];
        if !o1.is_finite() {
            continue;
        }
        if o1 > best_obj1 {
            best_obj1 = o1;
            front.push(idx);
        }
    }
    front
}

fn pareto_front_constrained<T: Clone>(pop: &[MoPoint<T>]) -> Vec<MoPoint<T>> {
    pareto_front_constrained_indices(pop)
        .into_iter()
        .filter_map(|idx| pop.get(idx).cloned())
        .collect()
}

fn crowding_distance<T>(front: &[usize], pop: &[MoPoint<T>]) -> Vec<f64> {
    if front.is_empty() {
        return Vec::new();
    }
    if front.len() <= 2 {
        return vec![f64::INFINITY; front.len()];
    }

    let mut out = vec![0.0f64; front.len()];
    for obj_idx in 0..2usize {
        let mut order: Vec<usize> = (0..front.len()).collect();
        order.sort_by(|&a, &b| {
            let va = pop[front[a]].objectives[obj_idx];
            let vb = pop[front[b]].objectives[obj_idx];
            va.partial_cmp(&vb).unwrap_or(Ordering::Equal)
        });

        let min_v = pop[front[order[0]]].objectives[obj_idx];
        let max_v = pop[front[order[order.len() - 1]]].objectives[obj_idx];
        if !min_v.is_finite() || !max_v.is_finite() {
            continue;
        }
        let denom = max_v - min_v;
        if denom == 0.0 {
            continue;
        }

        out[order[0]] = f64::INFINITY;
        out[order[order.len() - 1]] = f64::INFINITY;

        for w in 1..(order.len() - 1) {
            let prev = pop[front[order[w - 1]]].objectives[obj_idx];
            let next = pop[front[order[w + 1]]].objectives[obj_idx];
            if !prev.is_finite() || !next.is_finite() {
                continue;
            }
            out[order[w]] += (next - prev) / denom;
        }
    }
    out
}

fn elite_indices<T>(pop: &[MoPoint<T>], population_size: usize) -> Vec<usize> {
    let mut elites: Vec<usize> = Vec::with_capacity(population_size);
    let fronts = non_dominated_sort(pop);

    for front in fronts {
        if elites.len() >= population_size {
            break;
        }

        if elites.len() + front.len() <= population_size {
            elites.extend(front);
            continue;
        }

        // Partial fill by crowding distance.
        let crowd = crowding_distance(&front, pop);
        let mut idx: Vec<usize> = (0..front.len()).collect();
        idx.sort_by(|&a, &b| crowd[b].total_cmp(&crowd[a]));

        let need = population_size - elites.len();
        elites.extend(idx.into_iter().take(need).map(|i| front[i]));
        break;
    }

    elites
}

fn select_parent<T, R: Rng + ?Sized>(pop: &[MoPoint<T>], rng: &mut R) -> usize {
    debug_assert!(!pop.is_empty());
    if pop.len() == 1 {
        return 0;
    }
    let a = rng.gen_range(0..pop.len());
    let mut b = rng.gen_range(0..pop.len());
    if b == a {
        b = (b + 1) % pop.len();
    }
    if constrained_dominates(&pop[a], &pop[b]) {
        a
    } else {
        b
    }
}

pub fn nsga2_optimize<T, F, Repair>(
    space: &SearchSpace,
    cfg: Nsga2Config,
    repair: Repair,
    eval: F,
) -> Result<Nsga2Result<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.population_size == 0 {
        return Err("population_size must be > 0".to_string());
    }

    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let num_params = space.params.len();
    let mut_prob = cfg
        .mutation_prob
        .unwrap_or_else(|| 1.0 / (num_params as f64).max(1.0))
        .clamp(0.0, 1.0);

    let mut total_evaluations = 0usize;
    let mut all_seen: Vec<MoPoint<T>> = Vec::new();

    // Initial population (random).
    let mut population_values: Vec<Vec<ParamValue>> = (0..cfg.population_size)
        .map(|_| space.sample(&mut rng))
        .collect();
    for v in &mut population_values {
        repair(v);
    }

    let mut population: Vec<MoPoint<T>> = population_values
        .par_iter()
        .map(|values| {
            let out = eval(values);
            let violation = overall_violation(&out.constraints);
            MoPoint {
                values: values.clone(),
                objectives: out.objectives,
                violation,
                payload: out.payload,
            }
        })
        .collect();
    total_evaluations = total_evaluations.saturating_add(population.len());
    all_seen.extend(population.clone());

    for _gen in 0..cfg.generations {
        // Generate offspring.
        let mut offspring_values: Vec<Vec<ParamValue>> = Vec::with_capacity(cfg.population_size);
        while offspring_values.len() < cfg.population_size {
            let p0 = select_parent(&population, &mut rng);
            let p1 = if rng.gen_bool(cfg.crossover_prob.clamp(0.0, 1.0) as f64)
                && population.len() > 1
            {
                // Ensure distinct when possible.
                let mut idx = select_parent(&population, &mut rng);
                if idx == p0 {
                    idx = (idx + 1) % population.len();
                }
                idx
            } else {
                p0
            };

            let mut child = population[p0].values.clone();
            if p1 != p0 {
                let other = &population[p1].values;
                for i in 0..num_params.min(other.len()) {
                    if rng.gen_bool(cfg.swapping_prob.clamp(0.0, 1.0) as f64) {
                        child[i] = other[i].clone();
                    }
                }
            }

            for i in 0..num_params {
                if rng.gen_bool(mut_prob) {
                    child[i] = space.sample_param(i, &mut rng);
                }
            }

            repair(&mut child);
            offspring_values.push(child);
        }

        let offspring: Vec<MoPoint<T>> = offspring_values
            .par_iter()
            .map(|values| {
                let out = eval(values);
                let violation = overall_violation(&out.constraints);
                MoPoint {
                    values: values.clone(),
                    objectives: out.objectives,
                    violation,
                    payload: out.payload,
                }
            })
            .collect();
        total_evaluations = total_evaluations.saturating_add(offspring.len());
        all_seen.extend(offspring.clone());

        let mut combined = population;
        combined.extend(offspring);

        let elites = elite_indices(&combined, cfg.population_size);
        population = elites.into_iter().map(|i| combined[i].clone()).collect();
    }

    // Pareto front (rank-0) from all evaluated points.
    let fronts = non_dominated_sort(&all_seen);
    let pareto = fronts
        .first()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|idx| all_seen[idx].clone())
        .collect::<Vec<_>>();

    Ok(Nsga2Result {
        population,
        pareto,
        total_evaluations,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RandomMoConfig {
    pub seed: u64,
    pub trials: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QmcSequence {
    Halton,
    Sobol,
}

fn default_qmc_sequence() -> QmcSequence {
    QmcSequence::Halton
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QmcMoConfig {
    pub seed: u64,
    pub trials: usize,
    #[serde(default = "default_qmc_sequence")]
    pub sequence: QmcSequence,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TpeMoConfig {
    pub seed: u64,
    pub trials: usize,
    #[serde(default = "default_tpe_startup_trials")]
    pub startup_trials: usize,
    #[serde(default = "default_tpe_candidates")]
    pub n_ei_candidates: usize,
    #[serde(default = "default_tpe_gamma")]
    pub gamma: f64,
    #[serde(default = "default_tpe_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_tpe_max_model_points")]
    pub max_model_points: usize,
}

fn default_tpe_startup_trials() -> usize {
    10
}

fn default_tpe_candidates() -> usize {
    24
}

fn default_tpe_gamma() -> f64 {
    0.25
}

fn default_tpe_batch_size() -> usize {
    64
}

fn default_tpe_max_model_points() -> usize {
    2048
}

pub fn random_mo_search<T, F, Repair>(
    space: &SearchSpace,
    cfg: RandomMoConfig,
    repair: Repair,
    eval: F,
) -> Result<Vec<MoPoint<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(Vec::new());
    }

    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let mut values: Vec<Vec<ParamValue>> =
        (0..cfg.trials).map(|_| space.sample(&mut rng)).collect();
    for v in &mut values {
        repair(v);
    }

    let evaluated: Vec<MoPoint<T>> = values
        .par_iter()
        .map(|v| {
            let out = eval(v);
            let violation = overall_violation(&out.constraints);
            MoPoint {
                values: v.clone(),
                objectives: out.objectives,
                violation,
                payload: out.payload,
            }
        })
        .collect();

    Ok(pareto_front_constrained(&evaluated))
}

fn is_prime(x: u32) -> bool {
    if x < 2 {
        return false;
    }
    if x == 2 || x == 3 {
        return true;
    }
    if x % 2 == 0 {
        return false;
    }
    let mut d = 3u32;
    while (d as u64) * (d as u64) <= x as u64 {
        if x % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

fn first_primes(n: usize) -> Vec<u32> {
    let mut out: Vec<u32> = Vec::with_capacity(n);
    let mut x: u32 = 2;
    while out.len() < n {
        if is_prime(x) {
            out.push(x);
        }
        x = x.saturating_add(1);
    }
    out
}

fn radical_inverse(mut index: u64, base: u32) -> f64 {
    if base < 2 {
        return 0.0;
    }
    let b = base as u64;
    let inv_b = 1.0 / (base as f64);
    let mut f = inv_b;
    let mut result = 0.0f64;
    while index > 0 {
        let digit = (index % b) as f64;
        result += digit * f;
        index /= b;
        f *= inv_b;
        if !result.is_finite() {
            break;
        }
    }
    result
}

fn qmc_sample_one(domain: &ParamDomain, u: f64) -> ParamValue {
    let uu = if u.is_finite() { u.clamp(0.0, 0.999_999_999_999) } else { 0.5 };
    match domain {
        ParamDomain::IntRange {
            low,
            high,
            step,
            log,
        } => {
            let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
            if lo == hi {
                return ParamValue::Int(lo);
            }
            let st = (*step).max(1);
            let lo2 = lo;
            let hi2 = hi;
            if *log && lo2 > 0 && hi2 > 0 {
                let lo_t = (lo2 as f64).ln();
                let hi_t = (hi2 as f64).ln();
                let x = (lo_t + uu * (hi_t - lo_t)).exp();
                let mut xi = x.round() as i64;
                xi = xi.clamp(lo2, hi2);
                if st > 1 {
                    xi = lo2 + ((xi - lo2) / st) * st;
                }
                ParamValue::Int(xi.clamp(lo2, hi2))
            } else {
                let span = (hi2 - lo2) as u64;
                let steps = (span / (st as u64)).max(1);
                let mut k = (uu * ((steps + 1) as f64)).floor() as u64;
                if k > steps {
                    k = steps;
                }
                let x = lo2.saturating_add((k as i64).saturating_mul(st));
                ParamValue::Int(x.clamp(lo2, hi2))
            }
        }
        ParamDomain::FloatRange { low, high, log } => {
            let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
            if lo == hi {
                return ParamValue::Float(lo);
            }
            if *log && lo > 0.0 && hi > 0.0 {
                let x = (lo.ln() + uu * (hi.ln() - lo.ln())).exp();
                ParamValue::Float(x.clamp(lo, hi))
            } else {
                ParamValue::Float((lo + uu * (hi - lo)).clamp(lo, hi))
            }
        }
        ParamDomain::Categorical { choices } => {
            if choices.is_empty() {
                return ParamValue::Cat(0);
            }
            let k = choices.len();
            let mut idx = (uu * (k as f64)).floor() as usize;
            if idx >= k {
                idx = k.saturating_sub(1);
            }
            ParamValue::Cat(idx)
        }
        ParamDomain::Bool => ParamValue::Bool(uu < 0.5),
    }
}

pub fn qmc_mo_search<T, F, Repair>(
    space: &SearchSpace,
    cfg: QmcMoConfig,
    repair: Repair,
    eval: F,
) -> Result<Vec<MoPoint<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(Vec::new());
    }
    let dim = space.params.len();
    let mut values: Vec<Vec<ParamValue>> = Vec::with_capacity(cfg.trials);
    match cfg.sequence {
        QmcSequence::Halton => {
            let bases = first_primes(dim);
            for i in 0..cfg.trials {
                let idx = cfg.seed.wrapping_add(i as u64).wrapping_add(1);
                let mut v: Vec<ParamValue> = Vec::with_capacity(dim);
                for (d, p) in space.params.iter().enumerate() {
                    let u = radical_inverse(idx, bases[d]);
                    v.push(qmc_sample_one(&p.domain, u));
                }
                repair(&mut v);
                values.push(v);
            }
        }
        QmcSequence::Sobol => {
            let params = JoeKuoD6::minimal();
            let mut seq = Sobol::<f64>::new(dim, &params);

            let mut shift_rng = StdRng::seed_from_u64(cfg.seed);
            let shift: Vec<f64> = (0..dim)
                .map(|_| shift_rng.gen_range(0.0..1.0))
                .collect();

            for _ in 0..cfg.trials {
                let point = seq
                    .next()
                    .ok_or_else(|| "sobol sequence ended unexpectedly".to_string())?;
                let mut v: Vec<ParamValue> = Vec::with_capacity(dim);
                for (d, p) in space.params.iter().enumerate() {
                    let mut u = point.get(d).copied().unwrap_or(0.5);
                    if !u.is_finite() {
                        u = 0.5;
                    }
                    if let Some(s) = shift.get(d).copied() {
                        u = (u + s).fract();
                        if u < 0.0 {
                            u += 1.0;
                        }
                    }
                    v.push(qmc_sample_one(&p.domain, u));
                }
                repair(&mut v);
                values.push(v);
            }
        }
    }

    let evaluated: Vec<MoPoint<T>> = values
        .par_iter()
        .map(|v| {
            let out = eval(v);
            let violation = overall_violation(&out.constraints);
            MoPoint {
                values: v.clone(),
                objectives: out.objectives,
                violation,
                payload: out.payload,
            }
        })
        .collect();

    Ok(pareto_front_constrained(&evaluated))
}

fn log_sum_exp(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return f64::NEG_INFINITY;
    }
    let mut max_v = f64::NEG_INFINITY;
    for &x in xs {
        if x.is_finite() && x > max_v {
            max_v = x;
        }
    }
    if !max_v.is_finite() {
        return f64::NEG_INFINITY;
    }
    let mut sum = 0.0f64;
    for &x in xs {
        if !x.is_finite() {
            continue;
        }
        sum += (x - max_v).exp();
    }
    if sum <= 0.0 || !sum.is_finite() {
        return f64::NEG_INFINITY;
    }
    max_v + sum.ln()
}

#[derive(Clone, Debug)]
struct ParzenEstimator {
    mus: Vec<f64>,    // transformed space
    sigmas: Vec<f64>, // transformed space
    weights: Vec<f64>,
    low: f64,
    high: f64,
}

impl ParzenEstimator {
    fn from_samples(mus: Vec<f64>, low: f64, high: f64) -> Self {
        let samples: Vec<(f64, f64)> = mus.into_iter().map(|v| (v, 1.0)).collect();
        Self::from_weighted_samples(samples, low, high)
    }

    fn from_weighted_samples(samples: Vec<(f64, f64)>, low: f64, high: f64) -> Self {
        let mut pairs: Vec<(f64, f64)> = samples
            .into_iter()
            .filter(|(v, w)| v.is_finite() && w.is_finite() && *w > 0.0)
            .collect();
        if pairs.is_empty() {
            pairs.push(((low + high) * 0.5, 1.0));
        }
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        let n = pairs.len();
        let mus: Vec<f64> = pairs.iter().map(|(v, _)| *v).collect();
        let range = (high - low).abs().max(1e-12);
        let min_sigma = range * 1e-3;
        let max_sigma = range;

        let mut sigmas: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            let left = if i == 0 { mus[i] } else { mus[i - 1] };
            let right = if i + 1 >= n { mus[i] } else { mus[i + 1] };
            let dl = (mus[i] - left).abs();
            let dr = (right - mus[i]).abs();
            let mut s = dl.max(dr);
            if !s.is_finite() || s <= 0.0 {
                s = range * 0.5;
            }
            s = s.clamp(min_sigma, max_sigma);
            sigmas.push(s);
        }

        let mut weights: Vec<f64> = pairs.iter().map(|(_, w)| *w).collect();
        let sum_w: f64 = weights.iter().copied().sum();
        if !sum_w.is_finite() || sum_w <= 0.0 {
            let w = 1.0 / (n as f64).max(1.0);
            weights.clear();
            weights.resize(n, w);
        } else {
            for w in &mut weights {
                *w /= sum_w;
            }
        }

        ParzenEstimator {
            mus,
            sigmas,
            weights,
            low,
            high,
        }
    }

    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        let n = self.mus.len().max(1);
        let mut r = rng.gen_range(0.0..1.0);
        let mut idx = 0usize;
        for (i, w) in self.weights.iter().copied().enumerate().take(n) {
            r -= w;
            if r <= 0.0 {
                idx = i;
                break;
            }
        }
        idx = idx.min(self.mus.len().saturating_sub(1));
        let mu = self.mus[idx];
        let sigma = self
            .sigmas
            .get(idx)
            .copied()
            .unwrap_or(range_fallback_sigma(self.low, self.high))
            .max(1e-12);
        let normal = Normal::new(mu, sigma).unwrap_or_else(|_| Normal::new(0.0, 1.0).unwrap());
        normal.sample(rng).clamp(self.low, self.high)
    }

    fn log_pdf(&self, x: f64) -> f64 {
        if !x.is_finite() {
            return f64::NEG_INFINITY;
        }
        if self.mus.is_empty() {
            return f64::NEG_INFINITY;
        }
        let mut parts: Vec<f64> = Vec::with_capacity(self.mus.len());
        for ((mu, sigma0), w) in self
            .mus
            .iter()
            .copied()
            .zip(self.sigmas.iter().copied())
            .zip(self.weights.iter().copied())
        {
            let sigma = sigma0.max(1e-12);
            let z = (x - mu) / sigma;
            let lw = w.max(1e-12).ln();
            let lp = lw
                - 0.5 * (2.0 * std::f64::consts::PI).ln()
                - sigma.ln()
                - 0.5 * (z * z);
            parts.push(lp);
        }
        log_sum_exp(&parts)
    }
}

fn range_fallback_sigma(low: f64, high: f64) -> f64 {
    let range = (high - low).abs();
    if range.is_finite() && range > 0.0 {
        range * 0.5
    } else {
        1.0
    }
}

#[derive(Clone, Debug)]
enum TpeParamModel {
    Int {
        low: i64,
        high: i64,
        step: i64,
        log: bool,
        good: ParzenEstimator,
        bad: ParzenEstimator,
    },
    Float {
        low: f64,
        high: f64,
        log: bool,
        good: ParzenEstimator,
        bad: ParzenEstimator,
    },
    Categorical {
        good_p: Vec<f64>,
        bad_p: Vec<f64>,
    },
    Bool {
        good_p: [f64; 2],
        bad_p: [f64; 2],
    },
}

impl TpeParamModel {
    fn sample_candidate<R: Rng + ?Sized>(&self, rng: &mut R) -> ParamValue {
        match self {
            TpeParamModel::Int {
                low,
                high,
                step,
                log,
                good,
                ..
            } => {
                let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
                if lo == hi {
                    return ParamValue::Int(lo);
                }
                let lo_t = if *log && lo > 0 { (lo as f64).ln() } else { lo as f64 };
                let hi_t = if *log && hi > 0 { (hi as f64).ln() } else { hi as f64 };
                let x_t = good.sample(rng).clamp(lo_t.min(hi_t), lo_t.max(hi_t));
                let x = if *log { x_t.exp() } else { x_t };
                let mut xi = x.round() as i64;
                xi = xi.clamp(lo, hi);
                let st = (*step).max(1);
                if st > 1 {
                    xi = lo + ((xi - lo) / st) * st;
                }
                ParamValue::Int(xi.clamp(lo, hi))
            }
            TpeParamModel::Float { low, high, log, good, .. } => {
                let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
                if lo == hi {
                    return ParamValue::Float(lo);
                }
                let lo_t = if *log && lo > 0.0 { lo.ln() } else { lo };
                let hi_t = if *log && hi > 0.0 { hi.ln() } else { hi };
                let x_t = good.sample(rng).clamp(lo_t.min(hi_t), lo_t.max(hi_t));
                let x = if *log { x_t.exp() } else { x_t };
                ParamValue::Float(x.clamp(lo, hi))
            }
            TpeParamModel::Categorical { good_p, .. } => {
                if good_p.is_empty() {
                    return ParamValue::Cat(0);
                }
                let mut r = rng.gen_range(0.0..1.0);
                for (i, p) in good_p.iter().copied().enumerate() {
                    r -= p;
                    if r <= 0.0 {
                        return ParamValue::Cat(i);
                    }
                }
                ParamValue::Cat(good_p.len().saturating_sub(1))
            }
            TpeParamModel::Bool { good_p, .. } => ParamValue::Bool(rng.gen_bool(good_p[1].clamp(0.0, 1.0))),
        }
    }

    fn log_ratio(&self, v: &ParamValue) -> f64 {
        match (self, v) {
            (
                TpeParamModel::Int { low, high, log, good, bad, .. },
                ParamValue::Int(x),
            ) => {
                let lo = (*low).min(*high) as f64;
                let hi = (*low).max(*high) as f64;
                let xv = (*x as f64).clamp(lo, hi);
                let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                good.log_pdf(xt) - bad.log_pdf(xt)
            }
            (
                TpeParamModel::Float { low, high, log, good, bad, .. },
                ParamValue::Float(x),
            ) => {
                let lo = (*low).min(*high);
                let hi = (*low).max(*high);
                let xv = (*x).clamp(lo, hi);
                let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                good.log_pdf(xt) - bad.log_pdf(xt)
            }
            (TpeParamModel::Categorical { good_p, bad_p }, ParamValue::Cat(i)) => {
                let k = good_p.len().min(bad_p.len());
                if k == 0 {
                    return 0.0;
                }
                let idx = (*i).min(k.saturating_sub(1));
                let gp = good_p[idx].max(1e-12);
                let bp = bad_p[idx].max(1e-12);
                gp.ln() - bp.ln()
            }
            (TpeParamModel::Bool { good_p, bad_p }, ParamValue::Bool(b)) => {
                let idx = if *b { 1 } else { 0 };
                let gp = good_p[idx].max(1e-12);
                let bp = bad_p[idx].max(1e-12);
                gp.ln() - bp.ln()
            }
            _ => 0.0,
        }
    }
}

fn hypervolume_weights_for_good<T: Clone>(evaluated: &[MoPoint<T>], good_idx: &[usize]) -> Vec<f64> {
    if good_idx.is_empty() {
        return Vec::new();
    }

    let eps = 1e-12;
    let mut weights: Vec<f64> = vec![eps; good_idx.len()];

    let mut feas_points: Vec<MoPoint<()>> = Vec::new();
    let mut pos_map: Vec<usize> = Vec::new();
    for (pos, &idx) in good_idx.iter().enumerate() {
        let p = match evaluated.get(idx) {
            Some(p) => p,
            None => continue,
        };
        if p.violation <= 0.0 && p.objectives[0].is_finite() && p.objectives[1].is_finite() {
            pos_map.push(pos);
            feas_points.push(MoPoint {
                values: Vec::new(),
                objectives: p.objectives,
                violation: 0.0,
                payload: (),
            });
        }
    }

    if feas_points.len() < 2 {
        for w in &mut weights {
            *w = 1.0;
        }
        return weights;
    }

    let front = pareto_front_constrained_indices(&feas_points);
    if front.is_empty() {
        for w in &mut weights {
            *w = 1.0;
        }
        return weights;
    }

    let mut front_points: Vec<(usize, f64, f64)> = front
        .into_iter()
        .map(|i| {
            let pos = pos_map.get(i).copied().unwrap_or(0);
            let o = feas_points.get(i).map(|p| p.objectives).unwrap_or([0.0, 0.0]);
            (pos, o[0], o[1])
        })
        .collect();

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &(_, x, y) in &front_points {
        if x.is_finite() {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
        if y.is_finite() {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    if !min_x.is_finite() || !max_x.is_finite() || !min_y.is_finite() || !max_y.is_finite() {
        for w in &mut weights {
            *w = 1.0;
        }
        return weights;
    }

    let span_x = (max_x - min_x).abs().max(1e-9);
    let span_y = (max_y - min_y).abs().max(1e-9);
    let rx = min_x - 0.1 * span_x - 1e-12;
    let ry = min_y - 0.1 * span_y - 1e-12;

    front_points.sort_by(|a, b| {
        let ox = a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal);
        if ox != Ordering::Equal {
            return ox;
        }
        b.2.partial_cmp(&a.2).unwrap_or(Ordering::Equal)
    });

    let mut prev_x = rx;
    for i in 0..front_points.len() {
        let (pos, x, y) = front_points[i];
        let next_y = if i + 1 < front_points.len() {
            front_points[i + 1].2
        } else {
            ry
        };
        let dx = (x - prev_x).max(0.0);
        let dy = (y - next_y).max(0.0);
        let contrib = (dx * dy).max(eps);
        if let Some(w) = weights.get_mut(pos) {
            *w = contrib;
        }
        prev_x = x;
    }

    let sum_w: f64 = weights.iter().copied().sum();
    if !sum_w.is_finite() || sum_w <= 0.0 {
        for w in &mut weights {
            *w = 1.0;
        }
    }

    weights
}

fn build_tpe_models<T: Clone>(
    space: &SearchSpace,
    evaluated: &[MoPoint<T>],
    cfg: &TpeMoConfig,
) -> Vec<TpeParamModel> {
    let n = evaluated.len();
    let dim = space.params.len();
    if n < 2 || dim == 0 {
        return Vec::new();
    }

    let max_points = cfg.max_model_points.max(32);

    let mut idxs: Vec<usize> = (0..n).collect();
    if n > max_points {
        let mut feas: Vec<usize> = idxs.iter().copied().filter(|&i| evaluated[i].violation <= 0.0).collect();
        let mut infeas: Vec<usize> = idxs.iter().copied().filter(|&i| evaluated[i].violation > 0.0).collect();
        let score = |p: &MoPoint<T>| -> f64 { p.objectives[0] + p.objectives[1] };
        feas.sort_by(|&a, &b| score(&evaluated[b]).partial_cmp(&score(&evaluated[a])).unwrap_or(Ordering::Equal));
        infeas.sort_by(|&a, &b| {
            let va = evaluated[a].violation;
            let vb = evaluated[b].violation;
            let o = va.partial_cmp(&vb).unwrap_or(Ordering::Equal);
            if o != Ordering::Equal {
                return o;
            }
            score(&evaluated[b]).partial_cmp(&score(&evaluated[a])).unwrap_or(Ordering::Equal)
        });

        idxs.clear();
        for i in feas {
            if idxs.len() >= max_points {
                break;
            }
            idxs.push(i);
        }
        for i in infeas {
            if idxs.len() >= max_points {
                break;
            }
            idxs.push(i);
        }
    }

    // Rank points using constrained Pareto fronts on the subset (small enough for O(n^2)),
    // then prioritize diversity via crowding distance within each front.
    let subset: Vec<MoPoint<T>> = idxs.iter().map(|&i| evaluated[i].clone()).collect();
    let fronts = non_dominated_sort(&subset);
    let mut ranked: Vec<usize> = Vec::with_capacity(subset.len());
    for front in fronts {
        let cd = crowding_distance(&front, &subset);
        let mut tmp: Vec<(usize, f64)> = front
            .iter()
            .enumerate()
            .map(|(pos, &idx)| (idx, cd.get(pos).copied().unwrap_or(0.0)))
            .collect();
        tmp.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        ranked.extend(tmp.into_iter().map(|(idx, _)| idx));
    }

    let gamma = cfg.gamma.clamp(1e-6, 0.999_999);
    let mut good_n = ((gamma * (ranked.len() as f64)).ceil() as usize).max(1);
    if good_n >= ranked.len() {
        good_n = ranked.len().saturating_sub(1).max(1);
    }

    let mut good_idx: Vec<usize> = Vec::with_capacity(good_n);
    let mut bad_idx: Vec<usize> = Vec::with_capacity(ranked.len().saturating_sub(good_n));
    for (pos, idx_sub) in ranked.into_iter().enumerate() {
        let orig = idxs[idx_sub];
        if pos < good_n {
            good_idx.push(orig);
        } else {
            bad_idx.push(orig);
        }
    }
    if bad_idx.is_empty() {
        // Fallback: split by time if rank produced only one bucket.
        let split = (idxs.len() / 2).max(1);
        good_idx = idxs.iter().copied().take(split).collect();
        bad_idx = idxs.iter().copied().skip(split).collect();
    }

    let good_w = hypervolume_weights_for_good(evaluated, &good_idx);

    let mut models: Vec<TpeParamModel> = Vec::with_capacity(dim);
    for (param_idx, def) in space.params.iter().enumerate() {
        match &def.domain {
            ParamDomain::Categorical { choices } => {
                let k = choices.len().max(1);
                let mut gc = vec![1.0f64; k];
                let mut bc = vec![1.0f64; k];
                for (&i, &w) in good_idx.iter().zip(good_w.iter()) {
                    if let Some(ParamValue::Cat(ci)) = evaluated[i].values.get(param_idx) {
                        if *ci < k {
                            gc[*ci] += w;
                        }
                    }
                }
                for &i in &bad_idx {
                    if let Some(ParamValue::Cat(ci)) = evaluated[i].values.get(param_idx) {
                        if *ci < k {
                            bc[*ci] += 1.0;
                        }
                    }
                }
                let gs: f64 = gc.iter().sum();
                let bs: f64 = bc.iter().sum();
                let good_p: Vec<f64> = gc.iter().map(|c| c / gs).collect();
                let bad_p: Vec<f64> = bc.iter().map(|c| c / bs).collect();
                models.push(TpeParamModel::Categorical { good_p, bad_p });
            }
            ParamDomain::Bool => {
                let mut gc = [1.0f64, 1.0f64];
                let mut bc = [1.0f64, 1.0f64];
                for (&i, &w) in good_idx.iter().zip(good_w.iter()) {
                    if let Some(ParamValue::Bool(b)) = evaluated[i].values.get(param_idx) {
                        gc[if *b { 1 } else { 0 }] += w;
                    }
                }
                for &i in &bad_idx {
                    if let Some(ParamValue::Bool(b)) = evaluated[i].values.get(param_idx) {
                        bc[if *b { 1 } else { 0 }] += 1.0;
                    }
                }
                let gs = gc[0] + gc[1];
                let bs = bc[0] + bc[1];
                let good_p = [gc[0] / gs, gc[1] / gs];
                let bad_p = [bc[0] / bs, bc[1] / bs];
                models.push(TpeParamModel::Bool { good_p, bad_p });
            }
            ParamDomain::IntRange { low, high, step, log } => {
                let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
                let lo_t = if *log && lo > 0 { (lo as f64).ln() } else { lo as f64 };
                let hi_t = if *log && hi > 0 { (hi as f64).ln() } else { hi as f64 };
                let low_t = lo_t.min(hi_t);
                let high_t = lo_t.max(hi_t);
                let mut good_mus: Vec<(f64, f64)> = Vec::new();
                let mut bad_mus: Vec<f64> = Vec::new();
                for (&i, &w) in good_idx.iter().zip(good_w.iter()) {
                    if let Some(ParamValue::Int(x)) = evaluated[i].values.get(param_idx) {
                        let xv = (*x as f64).clamp(lo as f64, hi as f64);
                        let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                        good_mus.push((xt, w));
                    }
                }
                for &i in &bad_idx {
                    if let Some(ParamValue::Int(x)) = evaluated[i].values.get(param_idx) {
                        let xv = (*x as f64).clamp(lo as f64, hi as f64);
                        let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                        bad_mus.push(xt);
                    }
                }
                let good = ParzenEstimator::from_weighted_samples(good_mus, low_t, high_t);
                let bad = ParzenEstimator::from_samples(bad_mus, low_t, high_t);
                models.push(TpeParamModel::Int {
                    low: lo,
                    high: hi,
                    step: (*step).max(1),
                    log: *log,
                    good,
                    bad,
                });
            }
            ParamDomain::FloatRange { low, high, log } => {
                let (lo, hi) = if low <= high { (*low, *high) } else { (*high, *low) };
                let lo_t = if *log && lo > 0.0 { lo.ln() } else { lo };
                let hi_t = if *log && hi > 0.0 { hi.ln() } else { hi };
                let low_t = lo_t.min(hi_t);
                let high_t = lo_t.max(hi_t);
                let mut good_mus: Vec<(f64, f64)> = Vec::new();
                let mut bad_mus: Vec<f64> = Vec::new();
                for (&i, &w) in good_idx.iter().zip(good_w.iter()) {
                    if let Some(ParamValue::Float(x)) = evaluated[i].values.get(param_idx) {
                        let xv = (*x).clamp(lo, hi);
                        let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                        good_mus.push((xt, w));
                    }
                }
                for &i in &bad_idx {
                    if let Some(ParamValue::Float(x)) = evaluated[i].values.get(param_idx) {
                        let xv = (*x).clamp(lo, hi);
                        let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                        bad_mus.push(xt);
                    }
                }
                let good = ParzenEstimator::from_weighted_samples(good_mus, low_t, high_t);
                let bad = ParzenEstimator::from_samples(bad_mus, low_t, high_t);
                models.push(TpeParamModel::Float {
                    low: lo,
                    high: hi,
                    log: *log,
                    good,
                    bad,
                });
            }
        }
    }
    models
}

fn tpe_suggest_one<R, RepairFn>(
    space: &SearchSpace,
    models: &[TpeParamModel],
    rng: &mut R,
    repair: &RepairFn,
    cfg: &TpeMoConfig,
) -> Vec<ParamValue>
where
    R: Rng + ?Sized,
    RepairFn: Fn(&mut [ParamValue]),
{
    let dim = space.params.len();
    let mut best_values: Vec<ParamValue> = space.sample(rng);
    repair(&mut best_values);
    let mut best_score = f64::NEG_INFINITY;

    let candidates = cfg.n_ei_candidates.max(1).min(512);
    for _ in 0..candidates {
        let mut values: Vec<ParamValue> = Vec::with_capacity(dim);
        for m in models {
            values.push(m.sample_candidate(rng));
        }
        repair(&mut values);
        let mut score = 0.0f64;
        for (i, m) in models.iter().enumerate() {
            if let Some(v) = values.get(i) {
                score += m.log_ratio(v);
            }
        }
        if score.is_finite() && score > best_score {
            best_score = score;
            best_values = values;
        }
    }
    best_values
}

pub fn tpe_mo_search<T, F, Repair>(
    space: &SearchSpace,
    cfg: TpeMoConfig,
    repair: Repair,
    eval: F,
) -> Result<Vec<MoPoint<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(Vec::new());
    }
    if cfg.gamma <= 0.0 || cfg.gamma >= 1.0 || !cfg.gamma.is_finite() {
        return Err("tpe_mo_search: gamma must be in (0, 1)".to_string());
    }
    if cfg.batch_size == 0 {
        return Err("tpe_mo_search: batch_size must be > 0".to_string());
    }
    if cfg.max_model_points < 2 {
        return Err("tpe_mo_search: max_model_points must be >= 2".to_string());
    }

    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let mut evaluated: Vec<MoPoint<T>> = Vec::with_capacity(cfg.trials);
    let mut seen: HashSet<u64> = HashSet::with_capacity(cfg.trials.saturating_mul(2));

    let startup = cfg.startup_trials.min(cfg.trials);
    let mut remaining = cfg.trials;
    let mut next_idx = 0usize;
    while remaining > 0 {
        let batch = remaining.min(cfg.batch_size.max(1));
        let models: Option<Vec<TpeParamModel>> = if next_idx >= startup && evaluated.len() >= 2 {
            let m = build_tpe_models(space, &evaluated, &cfg);
            if m.len() == space.params.len() && !m.is_empty() {
                Some(m)
            } else {
                None
            }
        } else {
            None
        };
        let mut values: Vec<Vec<ParamValue>> = Vec::with_capacity(batch);
        for _ in 0..batch {
            let max_tries = 64usize;
            let mut tries = 0usize;
            loop {
                let mut v = if let Some(m) = models.as_ref() {
                    tpe_suggest_one(space, m, &mut rng, &repair, &cfg)
                } else {
                    space.sample(&mut rng)
                };
                repair(&mut v);
                let key = values_hash(&v);
                if seen.insert(key) || tries >= max_tries {
                    values.push(v);
                    next_idx = next_idx.saturating_add(1);
                    break;
                }
                tries = tries.saturating_add(1);
            }
        }

        let batch_eval: Vec<MoPoint<T>> = values
            .par_iter()
            .map(|v| {
                let out = eval(v);
                let violation = overall_violation(&out.constraints);
                MoPoint {
                    values: v.clone(),
                    objectives: out.objectives,
                    violation,
                    payload: out.payload,
                }
            })
            .collect();
        evaluated.extend(batch_eval);
        remaining = remaining.saturating_sub(batch);
    }

    Ok(pareto_front_constrained(&evaluated))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constrained_domination_prefers_feasible() {
        let a = MoPoint {
            values: vec![],
            objectives: [1.0, 1.0],
            violation: 0.0,
            payload: (),
        };
        let b = MoPoint {
            values: vec![],
            objectives: [100.0, 100.0],
            violation: 1.0,
            payload: (),
        };
        assert!(constrained_dominates(&a, &b));
        assert!(!constrained_dominates(&b, &a));
    }

    #[test]
    fn constrained_domination_prefers_lower_violation() {
        let a = MoPoint {
            values: vec![],
            objectives: [0.0, 0.0],
            violation: 0.5,
            payload: (),
        };
        let b = MoPoint {
            values: vec![],
            objectives: [100.0, 100.0],
            violation: 2.0,
            payload: (),
        };
        assert!(constrained_dominates(&a, &b));
        assert!(!constrained_dominates(&b, &a));
    }

    #[test]
    fn pareto_domination_works() {
        let a = MoPoint {
            values: vec![],
            objectives: [1.0, 2.0],
            violation: 0.0,
            payload: (),
        };
        let b = MoPoint {
            values: vec![],
            objectives: [1.0, 1.0],
            violation: 0.0,
            payload: (),
        };
        let c = MoPoint {
            values: vec![],
            objectives: [0.5, 3.0],
            violation: 0.0,
            payload: (),
        };
        assert!(constrained_dominates(&a, &b));
        assert!(!constrained_dominates(&b, &a));
        assert!(!constrained_dominates(&a, &c));
        assert!(!constrained_dominates(&c, &a));
    }

    #[test]
    fn nsga2_smoke_runs() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::FloatRange {
                        low: -5.0,
                        high: 5.0,
                        log: false,
                    },
                },
                ParamDef {
                    name: "y".to_string(),
                    domain: ParamDomain::FloatRange {
                        low: -5.0,
                        high: 5.0,
                        log: false,
                    },
                },
            ],
        };
        let cfg = Nsga2Config {
            seed: 123,
            population_size: 32,
            generations: 5,
            crossover_prob: 0.9,
            swapping_prob: 0.5,
            mutation_prob: None,
        };
        let repair = |_v: &mut [ParamValue]| {};
        let eval = |v: &[ParamValue]| {
            let x = match v[0] {
                ParamValue::Float(f) => f,
                _ => 0.0,
            };
            let y = match v[1] {
                ParamValue::Float(f) => f,
                _ => 0.0,
            };
            // Objectives: maximize -x^2 and maximize -y^2 (prefer near 0).
            EvalOutcome {
                objectives: [-x * x, -y * y],
                constraints: vec![],
                payload: (),
            }
        };

        let res = nsga2_optimize(&space, cfg, repair, eval).expect("nsga2 ok");
        assert_eq!(res.population.len(), 32);
        assert!(!res.pareto.is_empty());
        assert!(res.total_evaluations >= 32);
    }

    #[test]
    fn qmc_halton_smoke_runs() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::FloatRange {
                        low: 0.0,
                        high: 1.0,
                        log: false,
                    },
                },
                ParamDef {
                    name: "k".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    },
                },
            ],
        };
        let cfg = QmcMoConfig {
            seed: 7,
            trials: 64,
            sequence: QmcSequence::Halton,
        };
        let repair = |_v: &mut [ParamValue]| {};
        let eval = |v: &[ParamValue]| {
            let x = match v.get(0) {
                Some(ParamValue::Float(x)) => *x,
                _ => 0.0,
            };
            let obj = [x, 1.0 - x];
            EvalOutcome {
                objectives: obj,
                constraints: vec![],
                payload: (),
            }
        };
        let pareto = qmc_mo_search(&space, cfg, repair, eval).expect("qmc ok");
        assert!(!pareto.is_empty());
    }

    #[test]
    fn qmc_sobol_smoke_runs() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::FloatRange {
                        low: 0.0,
                        high: 1.0,
                        log: false,
                    },
                },
                ParamDef {
                    name: "k".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["a".to_string(), "b".to_string(), "c".to_string()],
                    },
                },
            ],
        };
        let cfg = QmcMoConfig {
            seed: 7,
            trials: 64,
            sequence: QmcSequence::Sobol,
        };
        let repair = |_v: &mut [ParamValue]| {};
        let eval = |v: &[ParamValue]| {
            let x = match v.get(0) {
                Some(ParamValue::Float(x)) => *x,
                _ => 0.0,
            };
            let obj = [x, 1.0 - x];
            EvalOutcome {
                objectives: obj,
                constraints: vec![],
                payload: (),
            }
        };
        let pareto = qmc_mo_search(&space, cfg, repair, eval).expect("qmc ok");
        assert!(!pareto.is_empty());
    }

    #[test]
    fn tpe_smoke_runs() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::FloatRange {
                        low: 0.0,
                        high: 1.0,
                        log: false,
                    },
                },
                ParamDef {
                    name: "y".to_string(),
                    domain: ParamDomain::FloatRange {
                        low: 0.0,
                        high: 1.0,
                        log: false,
                    },
                },
            ],
        };
        let cfg = TpeMoConfig {
            seed: 1,
            trials: 128,
            startup_trials: 10,
            n_ei_candidates: 24,
            gamma: 0.25,
            batch_size: 32,
            max_model_points: 256,
        };
        let repair = |_v: &mut [ParamValue]| {};
        let eval = |v: &[ParamValue]| {
            let x = match v.get(0) {
                Some(ParamValue::Float(x)) => *x,
                _ => 0.0,
            };
            let y = match v.get(1) {
                Some(ParamValue::Float(y)) => *y,
                _ => 0.0,
            };
            // toy: maximize closeness to (0.7, 0.2) on both objectives
            let obj1 = 1.0 - (x - 0.7).abs();
            let obj2 = 1.0 - (y - 0.2).abs();
            EvalOutcome {
                objectives: [obj1, obj2],
                constraints: vec![],
                payload: (),
            }
        };
        let pareto = tpe_mo_search(&space, cfg, repair, eval).expect("tpe ok");
        assert!(!pareto.is_empty());
    }

    #[test]
    fn grid_iter_enumerates_discrete_cartesian_product() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 1,
                        high: 3,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "k".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["a".to_string(), "b".to_string()],
                    },
                },
                ParamDef {
                    name: "flag".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };

        let pts: Vec<Vec<ParamValue>> = space.grid_iter(16, 128).unwrap().collect();
        assert_eq!(pts.len(), 3 * 2 * 2);

        // Odometer ordering: last axis fastest.
        assert_eq!(pts[0], vec![ParamValue::Int(1), ParamValue::Cat(0), ParamValue::Bool(false)]);
        assert_eq!(pts[1], vec![ParamValue::Int(1), ParamValue::Cat(0), ParamValue::Bool(true)]);
        assert_eq!(pts[2], vec![ParamValue::Int(1), ParamValue::Cat(1), ParamValue::Bool(false)]);
    }

    #[test]
    fn grid_iter_refuses_float_range_without_step() {
        let space = SearchSpace {
            params: vec![ParamDef {
                name: "x".to_string(),
                domain: ParamDomain::FloatRange {
                    low: 0.0,
                    high: 1.0,
                    log: false,
                },
            }],
        };
        let err = space.grid_iter(1024, 1024).unwrap_err();
        assert!(err.contains("FloatRange") && err.contains("use sampling"));
    }

    #[test]
    fn grid_iter_enforces_max_combos() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 0,
                        high: 9,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "y".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 0,
                        high: 9,
                        step: 1,
                        log: false,
                    },
                },
            ],
        };

        let err = space.grid_iter(1024, 50).unwrap_err();
        assert!(err.contains("exceeds max_combos"));
    }

    #[test]
    fn grid_get_matches_grid_iter_order() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 1,
                        high: 3,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "k".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["a".to_string(), "b".to_string()],
                    },
                },
                ParamDef {
                    name: "flag".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };

        let pts_iter: Vec<Vec<ParamValue>> = space.grid_iter(16, 128).unwrap().collect();
        let grid = space.grid(16, 128).unwrap();
        assert_eq!(grid.len(), pts_iter.len());

        assert_eq!(grid.get(0).unwrap(), pts_iter[0]);
        assert_eq!(grid.get(grid.len() - 1).unwrap(), pts_iter[pts_iter.len() - 1]);
        assert!(grid.get(grid.len()).is_none());

        let pts_grid: Vec<Vec<ParamValue>> = grid.iter().collect();
        assert_eq!(pts_grid, pts_iter);
    }

    #[test]
    fn grid_chunks_cover_all_values() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 1,
                        high: 3,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "k".to_string(),
                    domain: ParamDomain::Categorical {
                        choices: vec!["a".to_string(), "b".to_string()],
                    },
                },
                ParamDef {
                    name: "flag".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };

        let grid = space.grid(16, 128).unwrap();
        let all: Vec<Vec<ParamValue>> = grid.chunks(5).flatten().collect();
        assert_eq!(all.len(), grid.len());
        assert_eq!(all[0], grid.get(0).unwrap());
        assert_eq!(all[all.len() - 1], grid.get(grid.len() - 1).unwrap());
    }
}
