use rand::prelude::*;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sobol::{params::JoeKuoD6, Sobol};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

const INVALID_OBJECTIVE_VALUE: f64 = -f64::MAX;
const INVALID_CONSTRAINT_VIOLATION: f64 = f64::MAX;

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

pub fn param_values_hash(values: &[ParamValue]) -> u64 {
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
        let mut seen_names: HashSet<String> = HashSet::with_capacity(self.params.len());
        for p in &self.params {
            let name = p.name.trim();
            if name.is_empty() {
                return Err("param name cannot be empty".to_string());
            }
            let key = name.to_ascii_lowercase();
            if !seen_names.insert(key) {
                return Err(format!("duplicate param name '{name}'"));
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

    pub fn grid(
        &self,
        max_axis_values: usize,
        max_combos: usize,
    ) -> Result<SearchSpaceGrid, String> {
        self.validate()?;

        let mut axes: Vec<Vec<ParamValue>> = Vec::with_capacity(self.params.len());
        for p in &self.params {
            axes.push(domain_grid_values(&p.name, &p.domain, max_axis_values)?);
        }

        let len = grid_len_checked(&axes, max_combos)?;
        SearchSpaceGrid::new(axes, len)
    }
}

pub fn bounded_exact_grid(
    space: &SearchSpace,
    max_axis_values: usize,
    max_combos: usize,
) -> Result<Option<SearchSpaceGrid>, String> {
    let max_axis_values = max_axis_values.max(1);
    let max_combos = max_combos.max(1);

    space.validate()?;

    let mut total = 1usize;
    for p in &space.params {
        let axis_len = match &p.domain {
            ParamDomain::IntRange {
                low,
                high,
                step,
                log: _,
            } => {
                let step = *step;
                if step <= 0 {
                    return Err(format!("param '{}': step must be > 0", p.name));
                }
                let (lo, hi) = if low <= high {
                    (*low, *high)
                } else {
                    (*high, *low)
                };
                let span = hi
                    .checked_sub(lo)
                    .ok_or_else(|| format!("param '{}': range overflow", p.name))?;
                let steps = (span / step).saturating_add(1);
                let steps_usize = usize::try_from(steps)
                    .map_err(|_| format!("param '{}': integer grid too large", p.name))?;
                if steps_usize > max_axis_values {
                    return Ok(None);
                }
                steps_usize
            }
            ParamDomain::FloatRange { low, high, log: _ } => {
                if low == high {
                    1
                } else {
                    return Ok(None);
                }
            }
            ParamDomain::Categorical { choices } => {
                if choices.is_empty() {
                    return Err(format!("param '{}': categorical has no choices", p.name));
                }
                if choices.len() > max_axis_values {
                    return Ok(None);
                }
                choices.len()
            }
            ParamDomain::Bool => 2,
        };
        total = total
            .checked_mul(axis_len)
            .ok_or_else(|| "exact grid combination count overflow".to_string())?;
        if total > max_combos {
            return Ok(None);
        }
    }

    space.grid(max_axis_values, max_combos).map(Some)
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
            let (lo, hi) = if low <= high {
                (*low, *high)
            } else {
                (*high, *low)
            };
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

fn sanitize_objective(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        INVALID_OBJECTIVE_VALUE
    }
}

fn overall_violation(constraints: &[f64]) -> f64 {
    let mut total = 0.0;
    for &value in constraints {
        if !value.is_finite() {
            return INVALID_CONSTRAINT_VIOLATION;
        }
        total += value.max(0.0);
        if !total.is_finite() {
            return INVALID_CONSTRAINT_VIOLATION;
        }
    }
    total
}

fn mo_point_from_outcome<T>(values: Vec<ParamValue>, out: EvalOutcome<T>) -> MoPoint<T> {
    MoPoint {
        values,
        objectives: [
            sanitize_objective(out.objectives[0]),
            sanitize_objective(out.objectives[1]),
        ],
        violation: overall_violation(&out.constraints),
        payload: out.payload,
    }
}

#[derive(Clone, Debug)]
pub struct EvalOutcome<T> {
    pub objectives: [f64; 2],
    pub constraints: Vec<f64>,
    pub payload: T,
}

#[derive(Clone, Debug)]
pub struct MoPoint<T> {
    pub values: Vec<ParamValue>,
    pub objectives: [f64; 2],
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
    pub unique_evaluations: usize,
    pub duplicate_evaluations_avoided: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MoSearchStats {
    pub requested_evaluations: usize,
    pub actual_evaluations: usize,
    pub unique_evaluations: usize,
    pub duplicate_evaluations_avoided: usize,

    pub exact_grid_used: bool,
}

#[derive(Clone, Debug)]
pub struct MoSearchResult<T> {
    pub pareto: Vec<MoPoint<T>>,
    pub stats: MoSearchStats,
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
            .filter_map(|(i, p)| {
                if (p.violation - min_v).abs() <= eps {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
    }

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

type PointCache<T> = HashMap<u64, Vec<MoPoint<T>>>;

#[derive(Clone, Debug)]
struct BatchEvalResult<T> {
    points: Vec<MoPoint<T>>,
    newly_evaluated: Vec<MoPoint<T>>,
    actual_evaluations: usize,
    duplicate_evaluations_avoided: usize,
}

fn cached_point_clone<T: Clone>(
    cache: &PointCache<T>,
    values: &[ParamValue],
) -> Option<MoPoint<T>> {
    let key = param_values_hash(values);
    cache
        .get(&key)
        .and_then(|points| points.iter().find(|point| point.values == values).cloned())
}

fn insert_cached_point<T>(cache: &mut PointCache<T>, point: MoPoint<T>) {
    let key = param_values_hash(&point.values);
    cache.entry(key).or_default().push(point);
}

fn extend_frontier<T: Clone>(frontier: &mut Vec<MoPoint<T>>, new_points: Vec<MoPoint<T>>) {
    if new_points.is_empty() {
        return;
    }
    frontier.extend(new_points);
    *frontier = pareto_front_constrained(frontier);
}

fn evaluate_batch_with_cache<T, F>(
    values_batch: Vec<Vec<ParamValue>>,
    cache: &mut PointCache<T>,
    eval: &F,
) -> BatchEvalResult<T>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
{
    let mut points: Vec<Option<MoPoint<T>>> = vec![None; values_batch.len()];
    let mut pending_unique: Vec<Vec<ParamValue>> = Vec::new();
    let mut pending_origins: Vec<Vec<usize>> = Vec::new();
    let mut pending_by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut duplicate_evaluations_avoided = 0usize;

    for (idx, values) in values_batch.iter().enumerate() {
        if let Some(point) = cached_point_clone(cache, values) {
            points[idx] = Some(point);
            duplicate_evaluations_avoided = duplicate_evaluations_avoided.saturating_add(1);
            continue;
        }

        let key = param_values_hash(values);
        let mut matched_pending = None;
        if let Some(candidates) = pending_by_hash.get(&key) {
            for &pending_idx in candidates {
                if pending_unique
                    .get(pending_idx)
                    .is_some_and(|candidate| candidate == values)
                {
                    matched_pending = Some(pending_idx);
                    break;
                }
            }
        }

        if let Some(pending_idx) = matched_pending {
            pending_origins[pending_idx].push(idx);
            duplicate_evaluations_avoided = duplicate_evaluations_avoided.saturating_add(1);
            continue;
        }

        let pending_idx = pending_unique.len();
        pending_unique.push(values.clone());
        pending_origins.push(vec![idx]);
        pending_by_hash.entry(key).or_default().push(pending_idx);
    }

    let newly_evaluated: Vec<MoPoint<T>> = pending_unique
        .par_iter()
        .map(|values| mo_point_from_outcome(values.clone(), eval(values)))
        .collect();

    for (pending_idx, point) in newly_evaluated.iter().cloned().enumerate() {
        insert_cached_point(cache, point.clone());
        if let Some(origins) = pending_origins.get(pending_idx) {
            for &origin_idx in origins {
                if let Some(slot) = points.get_mut(origin_idx) {
                    *slot = Some(point.clone());
                }
            }
        }
    }

    let points: Vec<MoPoint<T>> = points.into_iter().flatten().collect();

    BatchEvalResult {
        points,
        actual_evaluations: newly_evaluated.len(),
        newly_evaluated,
        duplicate_evaluations_avoided,
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
    let mut duplicate_evaluations_avoided = 0usize;
    let mut cache: PointCache<T> = HashMap::new();
    let mut all_seen_unique: Vec<MoPoint<T>> = Vec::new();

    let mut population_values: Vec<Vec<ParamValue>> = (0..cfg.population_size)
        .map(|_| space.sample(&mut rng))
        .collect();
    for v in &mut population_values {
        repair(v);
    }
    let initial = evaluate_batch_with_cache(population_values, &mut cache, &eval);
    duplicate_evaluations_avoided =
        duplicate_evaluations_avoided.saturating_add(initial.duplicate_evaluations_avoided);
    total_evaluations = total_evaluations.saturating_add(initial.actual_evaluations);
    all_seen_unique.extend(initial.newly_evaluated.clone());
    let mut population: Vec<MoPoint<T>> = initial.points;

    for _gen in 0..cfg.generations {
        let mut offspring_values: Vec<Vec<ParamValue>> = Vec::with_capacity(cfg.population_size);
        while offspring_values.len() < cfg.population_size {
            let p0 = select_parent(&population, &mut rng);
            let p1 = if rng.gen_bool(cfg.crossover_prob.clamp(0.0, 1.0) as f64)
                && population.len() > 1
            {
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
        let offspring_eval = evaluate_batch_with_cache(offspring_values, &mut cache, &eval);
        duplicate_evaluations_avoided = duplicate_evaluations_avoided
            .saturating_add(offspring_eval.duplicate_evaluations_avoided);
        total_evaluations = total_evaluations.saturating_add(offspring_eval.actual_evaluations);
        all_seen_unique.extend(offspring_eval.newly_evaluated.clone());
        let offspring = offspring_eval.points;

        let mut combined = population;
        combined.extend(offspring);

        let elites = elite_indices(&combined, cfg.population_size);
        population = elites.into_iter().map(|i| combined[i].clone()).collect();
    }

    let pareto = pareto_front_constrained(&all_seen_unique);

    Ok(Nsga2Result {
        population,
        pareto,
        total_evaluations,
        unique_evaluations: all_seen_unique.len(),
        duplicate_evaluations_avoided,
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
    Ok(random_mo_search_with_stats(space, cfg, repair, eval)?.pareto)
}

pub fn random_mo_search_with_stats<T, F, Repair>(
    space: &SearchSpace,
    cfg: RandomMoConfig,
    repair: Repair,
    eval: F,
) -> Result<MoSearchResult<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(MoSearchResult {
            pareto: Vec::new(),
            stats: MoSearchStats {
                requested_evaluations: 0,
                ..MoSearchStats::default()
            },
        });
    }

    const CHUNK_SIZE: usize = 256;
    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let mut cache: PointCache<T> = HashMap::new();
    let mut frontier: Vec<MoPoint<T>> = Vec::new();
    let mut stats = MoSearchStats {
        requested_evaluations: cfg.trials,
        ..MoSearchStats::default()
    };
    let mut batch_values: Vec<Vec<ParamValue>> = Vec::with_capacity(CHUNK_SIZE);

    let mut flush_batch = |batch: &mut Vec<Vec<ParamValue>>| {
        if batch.is_empty() {
            return;
        }
        let eval_batch = std::mem::take(batch);
        let result = evaluate_batch_with_cache(eval_batch, &mut cache, &eval);
        stats.actual_evaluations = stats
            .actual_evaluations
            .saturating_add(result.actual_evaluations);
        stats.duplicate_evaluations_avoided = stats
            .duplicate_evaluations_avoided
            .saturating_add(result.duplicate_evaluations_avoided);
        extend_frontier(&mut frontier, result.newly_evaluated);
    };

    for _ in 0..cfg.trials {
        let mut values = space.sample(&mut rng);
        repair(&mut values);
        batch_values.push(values);
        if batch_values.len() >= CHUNK_SIZE {
            flush_batch(&mut batch_values);
        }
    }
    flush_batch(&mut batch_values);
    stats.unique_evaluations = stats.actual_evaluations;

    Ok(MoSearchResult {
        pareto: frontier,
        stats,
    })
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
    let uu = if u.is_finite() {
        u.clamp(0.0, 0.999_999_999_999)
    } else {
        0.5
    };
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
            let (lo, hi) = if low <= high {
                (*low, *high)
            } else {
                (*high, *low)
            };
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
    Ok(qmc_mo_search_with_stats(space, cfg, repair, eval)?.pareto)
}

pub fn qmc_mo_search_with_stats<T, F, Repair>(
    space: &SearchSpace,
    cfg: QmcMoConfig,
    repair: Repair,
    eval: F,
) -> Result<MoSearchResult<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(MoSearchResult {
            pareto: Vec::new(),
            stats: MoSearchStats::default(),
        });
    }
    const CHUNK_SIZE: usize = 256;
    let dim = space.params.len();
    let mut cache: PointCache<T> = HashMap::new();
    let mut frontier: Vec<MoPoint<T>> = Vec::new();
    let mut stats = MoSearchStats {
        requested_evaluations: cfg.trials,
        ..MoSearchStats::default()
    };
    let mut batch_values: Vec<Vec<ParamValue>> = Vec::with_capacity(CHUNK_SIZE);

    let mut flush_batch = |batch: &mut Vec<Vec<ParamValue>>| {
        if batch.is_empty() {
            return;
        }
        let eval_batch = std::mem::take(batch);
        let result = evaluate_batch_with_cache(eval_batch, &mut cache, &eval);
        stats.actual_evaluations = stats
            .actual_evaluations
            .saturating_add(result.actual_evaluations);
        stats.duplicate_evaluations_avoided = stats
            .duplicate_evaluations_avoided
            .saturating_add(result.duplicate_evaluations_avoided);
        extend_frontier(&mut frontier, result.newly_evaluated);
    };

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
                batch_values.push(v);
                if batch_values.len() >= CHUNK_SIZE {
                    flush_batch(&mut batch_values);
                }
            }
        }
        QmcSequence::Sobol => {
            let params = JoeKuoD6::minimal();
            if dim > params.max_dims {
                return Err(format!(
                    "sobol sequence supports at most {} dimensions, got {}",
                    params.max_dims, dim
                ));
            }
            let mut seq = Sobol::<f64>::new(dim, &params);

            let mut shift_rng = StdRng::seed_from_u64(cfg.seed);
            let shift: Vec<f64> = (0..dim).map(|_| shift_rng.gen_range(0.0..1.0)).collect();

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
                batch_values.push(v);
                if batch_values.len() >= CHUNK_SIZE {
                    flush_batch(&mut batch_values);
                }
            }
        }
    }
    flush_batch(&mut batch_values);
    stats.unique_evaluations = stats.actual_evaluations;

    Ok(MoSearchResult {
        pareto: frontier,
        stats,
    })
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
    mus: Vec<f64>,
    sigmas: Vec<f64>,
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
            let lp = lw - 0.5 * (2.0 * std::f64::consts::PI).ln() - sigma.ln() - 0.5 * (z * z);
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
                let (lo, hi) = if low <= high {
                    (*low, *high)
                } else {
                    (*high, *low)
                };
                if lo == hi {
                    return ParamValue::Int(lo);
                }
                let lo_t = if *log && lo > 0 {
                    (lo as f64).ln()
                } else {
                    lo as f64
                };
                let hi_t = if *log && hi > 0 {
                    (hi as f64).ln()
                } else {
                    hi as f64
                };
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
            TpeParamModel::Float {
                low,
                high,
                log,
                good,
                ..
            } => {
                let (lo, hi) = if low <= high {
                    (*low, *high)
                } else {
                    (*high, *low)
                };
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
            TpeParamModel::Bool { good_p, .. } => {
                ParamValue::Bool(rng.gen_bool(good_p[1].clamp(0.0, 1.0)))
            }
        }
    }

    fn log_ratio(&self, v: &ParamValue) -> f64 {
        match (self, v) {
            (
                TpeParamModel::Int {
                    low,
                    high,
                    log,
                    good,
                    bad,
                    ..
                },
                ParamValue::Int(x),
            ) => {
                let lo = (*low).min(*high) as f64;
                let hi = (*low).max(*high) as f64;
                let xv = (*x as f64).clamp(lo, hi);
                let xt = if *log && xv > 0.0 { xv.ln() } else { xv };
                good.log_pdf(xt) - bad.log_pdf(xt)
            }
            (
                TpeParamModel::Float {
                    low,
                    high,
                    log,
                    good,
                    bad,
                    ..
                },
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

fn hypervolume_weights_for_good<T: Clone>(
    evaluated: &[MoPoint<T>],
    good_idx: &[usize],
) -> Vec<f64> {
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
            let o = feas_points
                .get(i)
                .map(|p| p.objectives)
                .unwrap_or([0.0, 0.0]);
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
        let mut feas: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|&i| evaluated[i].violation <= 0.0)
            .collect();
        let mut infeas: Vec<usize> = idxs
            .iter()
            .copied()
            .filter(|&i| evaluated[i].violation > 0.0)
            .collect();
        let score = |p: &MoPoint<T>| -> f64 { p.objectives[0] + p.objectives[1] };
        feas.sort_by(|&a, &b| {
            score(&evaluated[b])
                .partial_cmp(&score(&evaluated[a]))
                .unwrap_or(Ordering::Equal)
        });
        infeas.sort_by(|&a, &b| {
            let va = evaluated[a].violation;
            let vb = evaluated[b].violation;
            let o = va.partial_cmp(&vb).unwrap_or(Ordering::Equal);
            if o != Ordering::Equal {
                return o;
            }
            score(&evaluated[b])
                .partial_cmp(&score(&evaluated[a]))
                .unwrap_or(Ordering::Equal)
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

    let subset: Vec<MoPoint<()>> = idxs
        .iter()
        .map(|&i| {
            let point = &evaluated[i];
            MoPoint {
                values: Vec::new(),
                objectives: point.objectives,
                violation: point.violation,
                payload: (),
            }
        })
        .collect();
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
                let lo_t = if *log && lo > 0 {
                    (lo as f64).ln()
                } else {
                    lo as f64
                };
                let hi_t = if *log && hi > 0 {
                    (hi as f64).ln()
                } else {
                    hi as f64
                };
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
                let (lo, hi) = if low <= high {
                    (*low, *high)
                } else {
                    (*high, *low)
                };
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
    Ok(tpe_mo_search_with_stats(space, cfg, repair, eval)?.pareto)
}

pub fn tpe_mo_search_with_stats<T, F, Repair>(
    space: &SearchSpace,
    cfg: TpeMoConfig,
    repair: Repair,
    eval: F,
) -> Result<MoSearchResult<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(MoSearchResult {
            pareto: Vec::new(),
            stats: MoSearchStats::default(),
        });
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
    let mut unique_evaluations = 0usize;
    let mut duplicate_evaluations_avoided = 0usize;

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
                let key = param_values_hash(&v);
                if seen.insert(key) || tries >= max_tries {
                    values.push(v);
                    next_idx = next_idx.saturating_add(1);
                    break;
                }
                duplicate_evaluations_avoided = duplicate_evaluations_avoided.saturating_add(1);
                tries = tries.saturating_add(1);
            }
        }

        let batch_eval: Vec<MoPoint<T>> = values
            .par_iter()
            .map(|v| mo_point_from_outcome(v.clone(), eval(v)))
            .collect();
        evaluated.extend(batch_eval);
        unique_evaluations = seen.len();
        remaining = remaining.saturating_sub(batch);
    }

    Ok(MoSearchResult {
        pareto: pareto_front_constrained(&evaluated),
        stats: MoSearchStats {
            requested_evaluations: cfg.trials,
            actual_evaluations: evaluated.len(),
            unique_evaluations,
            duplicate_evaluations_avoided,
            exact_grid_used: false,
        },
    })
}

pub fn exact_grid_mo_search_with_stats<T, F, Repair>(
    space: &SearchSpace,
    max_axis_values: usize,
    max_combos: usize,
    chunk_size: usize,
    repair: Repair,
    eval: F,
) -> Result<MoSearchResult<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    let grid = space.grid(max_axis_values, max_combos)?;
    exact_grid_mo_search_with_grid(grid, chunk_size, repair, eval)
}

pub fn bounded_exact_grid_mo_search_with_stats<T, F, Repair>(
    space: &SearchSpace,
    max_axis_values: usize,
    max_combos: usize,
    chunk_size: usize,
    repair: Repair,
    eval: F,
) -> Result<Option<MoSearchResult<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    let Some(grid) = bounded_exact_grid(space, max_axis_values, max_combos)? else {
        return Ok(None);
    };
    exact_grid_mo_search_with_grid(grid, chunk_size, repair, eval).map(Some)
}

pub fn candidate_set_mo_search_with_stats<T, F, Repair>(
    candidates: Vec<Vec<ParamValue>>,
    chunk_size: usize,
    repair: Repair,
    eval: F,
) -> Result<MoSearchResult<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    let requested = candidates.len();
    if requested == 0 {
        return Ok(MoSearchResult {
            pareto: Vec::new(),
            stats: MoSearchStats {
                exact_grid_used: true,
                ..MoSearchStats::default()
            },
        });
    }

    let chunk_size = chunk_size.max(1);
    let mut cache: PointCache<T> = HashMap::new();
    let mut frontier: Vec<MoPoint<T>> = Vec::new();
    let mut stats = MoSearchStats {
        requested_evaluations: requested,
        exact_grid_used: true,
        ..MoSearchStats::default()
    };

    for chunk in candidates.chunks(chunk_size) {
        let mut repaired: Vec<Vec<ParamValue>> = Vec::with_capacity(chunk.len());
        for values in chunk {
            let mut repaired_values = values.clone();
            repair(&mut repaired_values);
            repaired.push(repaired_values);
        }
        let result = evaluate_batch_with_cache(repaired, &mut cache, &eval);
        stats.actual_evaluations = stats
            .actual_evaluations
            .saturating_add(result.actual_evaluations);
        stats.duplicate_evaluations_avoided = stats
            .duplicate_evaluations_avoided
            .saturating_add(result.duplicate_evaluations_avoided);
        extend_frontier(&mut frontier, result.newly_evaluated);
    }

    stats.unique_evaluations = stats.actual_evaluations;
    Ok(MoSearchResult {
        pareto: frontier,
        stats,
    })
}

fn exact_grid_mo_search_with_grid<T, F, Repair>(
    grid: SearchSpaceGrid,
    chunk_size: usize,
    repair: Repair,
    eval: F,
) -> Result<MoSearchResult<T>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    let requested = grid.len();
    if requested == 0 {
        return Ok(MoSearchResult {
            pareto: Vec::new(),
            stats: MoSearchStats {
                exact_grid_used: true,
                ..MoSearchStats::default()
            },
        });
    }

    let chunk_size = chunk_size.max(1);
    let mut cache: PointCache<T> = HashMap::new();
    let mut frontier: Vec<MoPoint<T>> = Vec::new();
    let mut stats = MoSearchStats {
        requested_evaluations: requested,
        exact_grid_used: true,
        ..MoSearchStats::default()
    };

    for chunk in grid.chunks(chunk_size) {
        let mut repaired: Vec<Vec<ParamValue>> = Vec::with_capacity(chunk.len());
        for mut values in chunk {
            repair(&mut values);
            repaired.push(values);
        }
        let result = evaluate_batch_with_cache(repaired, &mut cache, &eval);
        stats.actual_evaluations = stats
            .actual_evaluations
            .saturating_add(result.actual_evaluations);
        stats.duplicate_evaluations_avoided = stats
            .duplicate_evaluations_avoided
            .saturating_add(result.duplicate_evaluations_avoided);
        extend_frontier(&mut frontier, result.newly_evaluated);
    }

    stats.unique_evaluations = stats.actual_evaluations;
    Ok(MoSearchResult {
        pareto: frontier,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering as AtomicOrdering},
        Arc,
    };

    #[derive(Debug)]
    struct CountedPayload {
        clone_counter: Arc<AtomicUsize>,
    }

    impl Clone for CountedPayload {
        fn clone(&self) -> Self {
            self.clone_counter.fetch_add(1, AtomicOrdering::Relaxed);
            Self {
                clone_counter: Arc::clone(&self.clone_counter),
            }
        }
    }

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
    fn validate_rejects_duplicate_param_names_case_insensitively() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "Fast_Len".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 1,
                        high: 5,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "fast_len".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };

        let err = space.validate().unwrap_err();
        assert!(err.contains("duplicate param name"));
    }

    #[test]
    fn param_values_hash_distinguishes_value_kinds() {
        let a = vec![ParamValue::Int(1), ParamValue::Bool(true)];
        let b = vec![ParamValue::Int(1), ParamValue::Bool(true)];
        let c = vec![ParamValue::Float(1.0), ParamValue::Bool(true)];

        assert_eq!(param_values_hash(&a), param_values_hash(&b));
        assert_ne!(param_values_hash(&a), param_values_hash(&c));
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
    fn nsga2_dedups_repaired_values() {
        let space = SearchSpace {
            params: vec![ParamDef {
                name: "x".to_string(),
                domain: ParamDomain::IntRange {
                    low: 1,
                    high: 8,
                    step: 1,
                    log: false,
                },
            }],
        };
        let cfg = Nsga2Config {
            seed: 42,
            population_size: 12,
            generations: 4,
            crossover_prob: 0.9,
            swapping_prob: 0.5,
            mutation_prob: Some(1.0),
        };
        let repair = |values: &mut [ParamValue]| values[0] = ParamValue::Int(3);
        let eval = |values: &[ParamValue]| EvalOutcome {
            objectives: [
                match values[0] {
                    ParamValue::Int(v) => v as f64,
                    _ => 0.0,
                },
                1.0,
            ],
            constraints: vec![],
            payload: (),
        };

        let res = nsga2_optimize(&space, cfg, repair, eval).expect("nsga2 ok");
        assert_eq!(res.total_evaluations, 1);
        assert_eq!(res.unique_evaluations, 1);
        assert!(res.duplicate_evaluations_avoided > 0);
        assert_eq!(res.pareto.len(), 1);
        assert_eq!(res.pareto[0].values, vec![ParamValue::Int(3)]);
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
    fn random_search_dedups_repaired_values() {
        let space = SearchSpace {
            params: vec![ParamDef {
                name: "x".to_string(),
                domain: ParamDomain::IntRange {
                    low: 0,
                    high: 100,
                    step: 1,
                    log: false,
                },
            }],
        };
        let cfg = RandomMoConfig {
            seed: 9,
            trials: 32,
        };
        let repair = |values: &mut [ParamValue]| values[0] = ParamValue::Int(7);
        let eval = |values: &[ParamValue]| EvalOutcome {
            objectives: [
                match values[0] {
                    ParamValue::Int(v) => v as f64,
                    _ => 0.0,
                },
                0.0,
            ],
            constraints: vec![],
            payload: (),
        };

        let res = random_mo_search_with_stats(&space, cfg, repair, eval).expect("random ok");
        assert_eq!(res.stats.requested_evaluations, 32);
        assert_eq!(res.stats.actual_evaluations, 1);
        assert_eq!(res.stats.unique_evaluations, 1);
        assert_eq!(res.pareto.len(), 1);
        assert_eq!(res.pareto[0].values, vec![ParamValue::Int(7)]);
        assert_eq!(res.stats.duplicate_evaluations_avoided, 31);
    }

    #[test]
    fn qmc_search_dedups_repaired_values() {
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
        let cfg = QmcMoConfig {
            seed: 7,
            trials: 40,
            sequence: QmcSequence::Sobol,
        };
        let repair = |values: &mut [ParamValue]| values[0] = ParamValue::Float(0.25);
        let eval = |_values: &[ParamValue]| EvalOutcome {
            objectives: [1.0, 0.0],
            constraints: vec![],
            payload: (),
        };

        let res = qmc_mo_search_with_stats(&space, cfg, repair, eval).expect("qmc ok");
        assert_eq!(res.stats.requested_evaluations, 40);
        assert_eq!(res.stats.actual_evaluations, 1);
        assert_eq!(res.stats.unique_evaluations, 1);
        assert_eq!(res.stats.duplicate_evaluations_avoided, 39);
        assert_eq!(res.pareto.len(), 1);
        assert_eq!(res.pareto[0].values, vec![ParamValue::Float(0.25)]);
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
    fn tpe_search_reports_duplicate_avoidance() {
        let space = SearchSpace {
            params: vec![ParamDef {
                name: "x".to_string(),
                domain: ParamDomain::IntRange {
                    low: 0,
                    high: 7,
                    step: 1,
                    log: false,
                },
            }],
        };
        let cfg = TpeMoConfig {
            seed: 1,
            trials: 8,
            startup_trials: 2,
            n_ei_candidates: 8,
            gamma: 0.25,
            batch_size: 2,
            max_model_points: 8,
        };
        let repair = |values: &mut [ParamValue]| {
            if let Some(first) = values.first_mut() {
                *first = ParamValue::Int(0);
            }
        };
        let eval = |values: &[ParamValue]| {
            let x = match values.first() {
                Some(ParamValue::Int(v)) => *v as f64,
                _ => 0.0,
            };
            EvalOutcome {
                objectives: [x, -x],
                constraints: vec![],
                payload: (),
            }
        };

        let res = tpe_mo_search_with_stats(&space, cfg, repair, eval).expect("tpe ok");
        assert_eq!(res.stats.requested_evaluations, 8);
        assert_eq!(res.stats.unique_evaluations, 1);
        assert!(res.stats.actual_evaluations >= res.stats.unique_evaluations);
        assert!(res.stats.duplicate_evaluations_avoided > 0);
    }

    #[test]
    fn build_tpe_models_does_not_clone_payloads_for_ranking() {
        let clone_counter = Arc::new(AtomicUsize::new(0));
        let evaluated: Vec<MoPoint<CountedPayload>> = (0..8)
            .map(|i| MoPoint {
                values: vec![ParamValue::Int(i)],
                objectives: [i as f64, (7 - i) as f64],
                violation: 0.0,
                payload: CountedPayload {
                    clone_counter: Arc::clone(&clone_counter),
                },
            })
            .collect();
        let space = SearchSpace {
            params: vec![ParamDef {
                name: "x".to_string(),
                domain: ParamDomain::IntRange {
                    low: 0,
                    high: 7,
                    step: 1,
                    log: false,
                },
            }],
        };
        let cfg = TpeMoConfig {
            seed: 1,
            trials: 16,
            startup_trials: 4,
            n_ei_candidates: 8,
            gamma: 0.25,
            batch_size: 4,
            max_model_points: 8,
        };

        let models = build_tpe_models(&space, &evaluated, &cfg);
        assert_eq!(models.len(), 1);
        assert_eq!(clone_counter.load(AtomicOrdering::Relaxed), 0);
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

        assert_eq!(
            pts[0],
            vec![
                ParamValue::Int(1),
                ParamValue::Cat(0),
                ParamValue::Bool(false)
            ]
        );
        assert_eq!(
            pts[1],
            vec![
                ParamValue::Int(1),
                ParamValue::Cat(0),
                ParamValue::Bool(true)
            ]
        );
        assert_eq!(
            pts[2],
            vec![
                ParamValue::Int(1),
                ParamValue::Cat(1),
                ParamValue::Bool(false)
            ]
        );
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
        assert_eq!(
            grid.get(grid.len() - 1).unwrap(),
            pts_iter[pts_iter.len() - 1]
        );
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

    #[test]
    fn bounded_exact_grid_mo_search_uses_chunked_grid_evaluation() {
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
                    name: "flag".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };
        let call_counter = Arc::new(AtomicUsize::new(0));
        let repair = |values: &mut [ParamValue]| {
            if matches!(values.get(1), Some(ParamValue::Bool(true))) {
                values[1] = ParamValue::Bool(false);
            }
        };
        let eval_calls = Arc::clone(&call_counter);
        let eval = move |values: &[ParamValue]| {
            eval_calls.fetch_add(1, AtomicOrdering::Relaxed);
            let x = match values[0] {
                ParamValue::Int(v) => v as f64,
                _ => 0.0,
            };
            EvalOutcome {
                objectives: [x, 10.0 - x],
                constraints: vec![],
                payload: (),
            }
        };

        let res = bounded_exact_grid_mo_search_with_stats(&space, 8, 16, 2, repair, eval)
            .expect("bounded exact grid result")
            .expect("grid should fit");
        assert!(res.stats.exact_grid_used);
        assert_eq!(res.stats.requested_evaluations, 6);
        assert_eq!(res.stats.actual_evaluations, 3);
        assert_eq!(res.stats.unique_evaluations, 3);
        assert_eq!(res.stats.duplicate_evaluations_avoided, 3);
        assert_eq!(call_counter.load(AtomicOrdering::Relaxed), 3);
        assert!(!res.pareto.is_empty());
    }

    #[test]
    fn bounded_exact_grid_mo_search_returns_none_when_grid_too_large() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 0,
                        high: 99,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "y".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 0,
                        high: 99,
                        step: 1,
                        log: false,
                    },
                },
            ],
        };

        let res = bounded_exact_grid_mo_search_with_stats(
            &space,
            16,
            64,
            8,
            |_values| {},
            |_values| EvalOutcome {
                objectives: [0.0, 0.0],
                constraints: vec![],
                payload: (),
            },
        )
        .expect("bounded helper should not error");
        assert!(res.is_none());
    }

    #[test]
    fn exact_and_bounded_exact_grid_search_agree_on_pareto_and_stats() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 1,
                        high: 4,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "flag".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };
        let repair = |values: &mut [ParamValue]| {
            if matches!(values.get(1), Some(ParamValue::Bool(true))) {
                values[1] = ParamValue::Bool(false);
            }
        };
        let eval = |values: &[ParamValue]| {
            let x = match values.first() {
                Some(ParamValue::Int(v)) => *v as f64,
                _ => 0.0,
            };
            EvalOutcome {
                objectives: [x, 10.0 - x],
                constraints: vec![],
                payload: (),
            }
        };

        let exact =
            exact_grid_mo_search_with_stats(&space, 8, 16, 2, repair, eval).expect("exact grid ok");
        let bounded = bounded_exact_grid_mo_search_with_stats(&space, 8, 16, 2, repair, eval)
            .expect("bounded exact grid ok")
            .expect("bounded grid should fit");

        assert_eq!(
            exact.stats.requested_evaluations,
            bounded.stats.requested_evaluations
        );
        assert_eq!(
            exact.stats.actual_evaluations,
            bounded.stats.actual_evaluations
        );
        assert_eq!(
            exact.stats.unique_evaluations,
            bounded.stats.unique_evaluations
        );
        assert_eq!(
            exact.stats.duplicate_evaluations_avoided,
            bounded.stats.duplicate_evaluations_avoided
        );
        assert_eq!(exact.stats.exact_grid_used, bounded.stats.exact_grid_used);

        let exact_front: Vec<(Vec<ParamValue>, [f64; 2], f64)> = exact
            .pareto
            .iter()
            .map(|point| (point.values.clone(), point.objectives, point.violation))
            .collect();
        let bounded_front: Vec<(Vec<ParamValue>, [f64; 2], f64)> = bounded
            .pareto
            .iter()
            .map(|point| (point.values.clone(), point.objectives, point.violation))
            .collect();

        assert_eq!(exact_front, bounded_front);
    }

    #[test]
    fn bounded_exact_grid_bool_axis_ignores_max_axis_values_cap() {
        let space = SearchSpace {
            params: vec![
                ParamDef {
                    name: "x".to_string(),
                    domain: ParamDomain::IntRange {
                        low: 3,
                        high: 3,
                        step: 1,
                        log: false,
                    },
                },
                ParamDef {
                    name: "flag".to_string(),
                    domain: ParamDomain::Bool,
                },
            ],
        };

        let grid = bounded_exact_grid(&space, 1, 8)
            .expect("grid helper should not error")
            .expect("bool axis should still enumerate both states");
        assert_eq!(grid.len(), 2);
        assert_eq!(
            grid.iter().collect::<Vec<_>>(),
            vec![
                vec![ParamValue::Int(3), ParamValue::Bool(false)],
                vec![ParamValue::Int(3), ParamValue::Bool(true)],
            ]
        );
    }

    #[test]
    fn candidate_search_treats_non_finite_constraints_as_max_violation() {
        let candidates = vec![vec![ParamValue::Int(0)], vec![ParamValue::Int(1)]];
        let res = candidate_set_mo_search_with_stats(
            candidates,
            2,
            |_values| {},
            |values| match values[0] {
                ParamValue::Int(0) => EvalOutcome {
                    objectives: [10.0, 10.0],
                    constraints: vec![f64::NAN],
                    payload: (),
                },
                ParamValue::Int(1) => EvalOutcome {
                    objectives: [0.5, 0.25],
                    constraints: vec![0.0],
                    payload: (),
                },
                _ => unreachable!("test only uses integer candidates"),
            },
        )
        .expect("candidate search should succeed");

        assert_eq!(res.pareto.len(), 1);
        assert_eq!(res.pareto[0].values, vec![ParamValue::Int(1)]);
        assert_eq!(res.pareto[0].violation, 0.0);
    }

    #[test]
    fn candidate_search_keeps_frontier_non_empty_when_objectives_are_non_finite() {
        let candidates = vec![vec![ParamValue::Int(0)], vec![ParamValue::Int(1)]];
        let res = candidate_set_mo_search_with_stats(
            candidates,
            2,
            |_values| {},
            |values| match values[0] {
                ParamValue::Int(0) => EvalOutcome {
                    objectives: [1.0, f64::NAN],
                    constraints: vec![],
                    payload: (),
                },
                ParamValue::Int(1) => EvalOutcome {
                    objectives: [0.5, f64::NEG_INFINITY],
                    constraints: vec![],
                    payload: (),
                },
                _ => unreachable!("test only uses integer candidates"),
            },
        )
        .expect("candidate search should succeed");

        assert_eq!(res.pareto.len(), 1);
        assert_eq!(res.pareto[0].values, vec![ParamValue::Int(0)]);
        assert!(res.pareto[0].objectives[0].is_finite());
        assert!(res.pareto[0].objectives[1].is_finite());
    }

    #[test]
    fn qmc_sobol_rejects_spaces_above_minimal_dimension_limit() {
        let space = SearchSpace {
            params: (0..101)
                .map(|idx| ParamDef {
                    name: format!("x_{idx}"),
                    domain: ParamDomain::FloatRange {
                        low: 0.0,
                        high: 1.0,
                        log: false,
                    },
                })
                .collect(),
        };
        let cfg = QmcMoConfig {
            seed: 7,
            trials: 1,
            sequence: QmcSequence::Sobol,
        };

        let err = qmc_mo_search_with_stats(
            &space,
            cfg,
            |_values| {},
            |_values| EvalOutcome {
                objectives: [0.0, 0.0],
                constraints: vec![],
                payload: (),
            },
        )
        .expect_err("oversize Sobol spaces should error instead of panicking");

        assert!(
            err.contains("at most 100 dimensions"),
            "unexpected error: {err}"
        );
    }
}
