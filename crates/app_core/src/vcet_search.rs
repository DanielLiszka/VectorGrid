//! Deterministic, bounded candidate generation for the effective VCET search space.
//!
//! The optimizer crate's QMC entry point couples sampling to objective evaluation,
//! caching, and Pareto reduction. A campaign manifest needs the sampled candidates
//! before any evaluation, so this module uses the same documented Halton construction
//! (successive prime bases with a deterministic seed-keyed Cranley-Patterson shift)
//! locally instead of depending on optimizer internals. The seed randomizes the
//! design itself; it is not merely a one-row skip into the same Halton sequence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const VCET_SEARCH_SCHEMA_VERSION: u32 = 1;
/// Hard product safety limit. Each campaign candidate expands into a full typed
/// strategy value and is retained in sealed summary evidence, so permitting a
/// million-entry request can consume multiple gigabytes before evaluation.
/// Larger research designs must be split into separately sealed replicates.
pub const MAX_VCET_SEARCH_BUDGET: usize = 8_192;
/// JSON numbers are the cross-surface campaign contract. Keep integer identity
/// material exactly representable when a manifest is parsed and re-emitted by
/// the Vue/JavaScript layer.
pub const MAX_SAFE_VCET_SEARCH_SEED: u64 = 9_007_199_254_740_991;
pub const MAX_VCET_SEARCH_CE_MULT: f64 = 1_000.0;

const FLOAT_SCALE: i64 = 1_000_000;
const HALTON_BASES: [u32; 8] = [2, 3, 5, 7, 11, 13, 17, 19];
const HALTON_ATTEMPTS_PER_TARGET: usize = 64;
const MIN_HALTON_ATTEMPTS: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VcetSearchSequence {
    /// Historical unshifted identifier retained for diagnostic decoding only.
    /// Exact campaign replay regenerates the current design and therefore
    /// classifies such a manifest as restart-required instead of silently
    /// treating it as equivalent.
    Halton,
    HaltonCranleyPatterson,
    /// Complete canonical enumeration used only when the requested budget can
    /// cover the entire valid effective search space.
    ExhaustiveLexicographic,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VcetU16Range {
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VcetQuantizedFloatRange {
    pub min: f64,
    pub max: f64,
    pub quantum: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VcetBoolChoices {
    pub values: Vec<bool>,
}

impl VcetBoolChoices {
    pub fn both() -> Self {
        Self {
            values: vec![false, true],
        }
    }

    pub fn fixed(value: bool) -> Self {
        Self {
            values: vec![value],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VcetSearchConfig {
    /// Maximum number of unique effective candidates emitted by the manifest.
    /// This field intentionally has no serde default: persisted requests must
    /// state their research budget explicitly.
    pub budget: usize,
    pub seed: u64,
    pub vidya_short_period: VcetU16Range,
    pub vidya_long_period: VcetU16Range,
    pub vidya_alpha: VcetQuantizedFloatRange,
    pub ce_period: VcetU16Range,
    pub ce_mult: VcetQuantizedFloatRange,
    pub ce_use_close: VcetBoolChoices,
    pub require_vidya_slope: VcetBoolChoices,
    pub max_trade_bars: VcetU16Range,
}

impl Default for VcetSearchConfig {
    fn default() -> Self {
        Self {
            budget: 2_048,
            seed: 0,
            vidya_short_period: VcetU16Range {
                min: 4,
                max: 48,
                step: 2,
            },
            vidya_long_period: VcetU16Range {
                min: 12,
                max: 160,
                step: 4,
            },
            vidya_alpha: VcetQuantizedFloatRange {
                min: 0.05,
                max: 0.8,
                quantum: 0.05,
            },
            ce_period: VcetU16Range {
                min: 10,
                max: 80,
                step: 2,
            },
            ce_mult: VcetQuantizedFloatRange {
                min: 1.5,
                max: 5.0,
                quantum: 0.25,
            },
            ce_use_close: VcetBoolChoices::both(),
            require_vidya_slope: VcetBoolChoices::both(),
            max_trade_bars: VcetU16Range {
                min: 30,
                max: 480,
                step: 30,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VcetEffectiveCandidate {
    pub vidya_short_period: u16,
    pub vidya_long_period: u16,
    pub vidya_alpha: f64,
    pub ce_period: u16,
    pub ce_mult: f64,
    pub ce_use_close: bool,
    pub require_vidya_slope: bool,
    pub max_trade_bars: u16,
}

/// Integer identity material used instead of formatted floating-point values.
/// Alpha and CE multiplier units are fixed millionths in schema version 1.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct VcetCandidateIdentityInputs {
    pub vidya_short_period: u16,
    pub vidya_long_period: u16,
    pub vidya_alpha_micros: i64,
    pub ce_period: u16,
    pub ce_mult_micros: i64,
    pub ce_use_close: bool,
    pub require_vidya_slope: bool,
    pub max_trade_bars: u16,
}

impl VcetCandidateIdentityInputs {
    pub fn canonical_key(&self) -> String {
        format!(
            "vcet_effective_v1|vs={}|vl={}|alpha_micros={}|ce_period={}|ce_mult_micros={}|ce_use_close={}|require_vidya_slope={}|max_trade_bars={}",
            self.vidya_short_period,
            self.vidya_long_period,
            self.vidya_alpha_micros,
            self.ce_period,
            self.ce_mult_micros,
            u8::from(self.ce_use_close),
            u8::from(self.require_vidya_slope),
            self.max_trade_bars,
        )
    }

    pub fn candidate_id(&self) -> String {
        format!("vcet_v1_{}", sha256_hex(self.canonical_key().as_bytes()))
    }

    fn to_effective_candidate(&self) -> VcetEffectiveCandidate {
        VcetEffectiveCandidate {
            vidya_short_period: self.vidya_short_period,
            vidya_long_period: self.vidya_long_period,
            vidya_alpha: units_to_float(self.vidya_alpha_micros),
            ce_period: self.ce_period,
            ce_mult: units_to_float(self.ce_mult_micros),
            ce_use_close: self.ce_use_close,
            require_vidya_slope: self.require_vidya_slope,
            max_trade_bars: self.max_trade_bars,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VcetSearchCandidate {
    pub ordinal: usize,
    pub candidate_id: String,
    /// Strategy-run identity assigned by the consuming evaluation surface.
    /// Generation leaves this unset because the campaign owns that artifact
    /// identity contract; a sealed campaign manifest fills it deterministically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_candidate_id: Option<String>,
    pub identity_inputs: VcetCandidateIdentityInputs,
    pub effective: VcetEffectiveCandidate,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct VcetSearchGenerationStats {
    pub halton_attempts: usize,
    pub invalid_period_pairs_discarded: usize,
    pub duplicate_candidates_discarded: usize,
    /// Legacy field retained so schema-v1 manifests remain decodable. New
    /// generation never mixes a biased lexicographic fallback into a QMC run.
    pub lexicographic_fallback_candidates: usize,
    pub exhaustive_candidates: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VcetSearchManifest {
    pub schema_version: u32,
    pub sequence: VcetSearchSequence,
    pub search_id: String,
    pub seed: u64,
    pub requested_budget: usize,
    pub effective_budget: usize,
    /// Decimal string avoids JSON's lossy integer range for very large spaces.
    pub valid_search_space_size: String,
    pub float_identity_scale: i64,
    pub generation_stats: VcetSearchGenerationStats,
    pub candidates: Vec<VcetSearchCandidate>,
}

#[derive(Clone, Copy, Debug)]
struct U16Axis {
    min: u16,
    step: u16,
    count: u64,
}

impl U16Axis {
    fn value(self, index: u64) -> u16 {
        debug_assert!(index < self.count);
        let value = u64::from(self.min) + index * u64::from(self.step);
        value as u16
    }
}

#[derive(Clone, Copy, Debug)]
struct I64Axis {
    min: i64,
    quantum: i64,
    count: u64,
}

#[derive(Clone, Copy, Debug)]
struct ValidPeriodPairBucket {
    short: u16,
    first_long_index: u64,
    long_count: u64,
    cumulative_end: u64,
}

impl I64Axis {
    fn value(self, index: u64) -> i64 {
        debug_assert!(index < self.count);
        let value = i128::from(self.min) + i128::from(index) * i128::from(self.quantum);
        value as i64
    }
}

#[derive(Clone, Debug)]
struct CanonicalSearchAxes {
    short: U16Axis,
    long: U16Axis,
    alpha: I64Axis,
    ce_period: U16Axis,
    ce_mult: I64Axis,
    ce_use_close: Vec<bool>,
    require_slope: Vec<bool>,
    max_trade_bars: U16Axis,
    valid_period_pair_buckets: Vec<ValidPeriodPairBucket>,
    valid_period_pairs: u64,
}

pub fn validate_vcet_search_config(cfg: &VcetSearchConfig) -> Result<(), String> {
    canonical_axes(cfg).map(|_| ())
}

pub fn generate_vcet_search_manifest(cfg: &VcetSearchConfig) -> Result<VcetSearchManifest, String> {
    let axes = canonical_axes(cfg)?;
    let valid_space_size = valid_search_space_size(&axes)?;
    if valid_space_size == 0 {
        return Err(
            "VCET search has no valid candidates with vidya_long_period > vidya_short_period"
                .to_string(),
        );
    }

    let effective_budget = if valid_space_size < cfg.budget as u128 {
        valid_space_size as usize
    } else {
        cfg.budget
    };
    let exhaustive = valid_space_size <= cfg.budget as u128;
    let sequence = if exhaustive {
        VcetSearchSequence::ExhaustiveLexicographic
    } else {
        VcetSearchSequence::HaltonCranleyPatterson
    };
    let mut identities = BTreeSet::new();
    let mut stats = VcetSearchGenerationStats::default();

    if exhaustive {
        fill_exhaustively(&axes, effective_budget, &mut identities, &mut stats);
    } else {
        let max_halton_attempts = effective_budget
            .saturating_mul(HALTON_ATTEMPTS_PER_TARGET)
            .max(MIN_HALTON_ATTEMPTS);
        for sample_offset in 0..max_halton_attempts {
            if identities.len() >= effective_budget {
                break;
            }
            stats.halton_attempts = stats.halton_attempts.saturating_add(1);
            let sample_index = sample_offset as u64 + 1;
            let identity = sample_halton_candidate(&axes, sample_index, cfg.seed);
            if !identities.insert(identity) {
                stats.duplicate_candidates_discarded =
                    stats.duplicate_candidates_discarded.saturating_add(1);
            }
        }
    }
    if identities.len() != effective_budget {
        return Err(format!(
            "VCET randomized Halton design produced {} unique candidates after {} attempts, expected {}; reduce the budget or split the design into independently seeded replicates",
            identities.len(),
            stats.halton_attempts,
            effective_budget
        ));
    }

    let candidates = identities
        .into_iter()
        .enumerate()
        .map(|(ordinal, identity_inputs)| VcetSearchCandidate {
            ordinal,
            candidate_id: identity_inputs.candidate_id(),
            evaluation_candidate_id: None,
            effective: identity_inputs.to_effective_candidate(),
            identity_inputs,
        })
        .collect();

    Ok(VcetSearchManifest {
        schema_version: VCET_SEARCH_SCHEMA_VERSION,
        sequence,
        search_id: search_id(cfg, &axes, sequence),
        seed: cfg.seed,
        requested_budget: cfg.budget,
        effective_budget,
        valid_search_space_size: valid_space_size.to_string(),
        float_identity_scale: FLOAT_SCALE,
        generation_stats: stats,
        candidates,
    })
}

fn canonical_axes(cfg: &VcetSearchConfig) -> Result<CanonicalSearchAxes, String> {
    if cfg.budget == 0 {
        return Err("VCET search budget must be > 0".to_string());
    }
    if cfg.budget > MAX_VCET_SEARCH_BUDGET {
        return Err(format!(
            "VCET search budget must be <= {MAX_VCET_SEARCH_BUDGET}"
        ));
    }
    if cfg.seed > MAX_SAFE_VCET_SEARCH_SEED {
        return Err(format!(
            "VCET search seed must be <= {MAX_SAFE_VCET_SEARCH_SEED}"
        ));
    }

    let short = canonical_u16_axis("vidya_short_period", &cfg.vidya_short_period, 2)?;
    let long = canonical_u16_axis("vidya_long_period", &cfg.vidya_long_period, 2)?;
    let alpha = canonical_float_axis("vidya_alpha", &cfg.vidya_alpha)?;
    let ce_period = canonical_u16_axis("ce_period", &cfg.ce_period, 1)?;
    let ce_mult = canonical_float_axis("ce_mult", &cfg.ce_mult)?;
    let max_trade_bars = canonical_u16_axis("max_trade_bars", &cfg.max_trade_bars, 1)?;
    let (valid_period_pair_buckets, valid_period_pairs) =
        build_valid_period_pair_buckets(short, long)?;

    let zero = 0;
    let one = FLOAT_SCALE;
    if alpha.min < zero || alpha.value(alpha.count - 1) > one {
        return Err("VCET vidya_alpha range must stay within [0, 1]".to_string());
    }
    if ce_mult.min <= 0
        || ce_mult.value(ce_mult.count - 1)
            > float_to_units("ce_mult", "product_max", MAX_VCET_SEARCH_CE_MULT)?
    {
        return Err(format!(
            "VCET ce_mult range must stay within (0, {MAX_VCET_SEARCH_CE_MULT}]"
        ));
    }

    Ok(CanonicalSearchAxes {
        short,
        long,
        alpha,
        ce_period,
        ce_mult,
        ce_use_close: canonical_bool_choices("ce_use_close", &cfg.ce_use_close)?,
        require_slope: canonical_bool_choices("require_vidya_slope", &cfg.require_vidya_slope)?,
        max_trade_bars,
        valid_period_pair_buckets,
        valid_period_pairs,
    })
}

fn canonical_u16_axis(label: &str, range: &VcetU16Range, minimum: u16) -> Result<U16Axis, String> {
    if range.step == 0 {
        return Err(format!("VCET {label}.step must be > 0"));
    }
    if range.min < minimum {
        return Err(format!("VCET {label}.min must be >= {minimum}"));
    }
    if range.max < range.min {
        return Err(format!("VCET {label}.max must be >= min"));
    }
    let span = u32::from(range.max) - u32::from(range.min);
    if span % u32::from(range.step) != 0 {
        return Err(format!("VCET {label} endpoints must align to its step"));
    }
    Ok(U16Axis {
        min: range.min,
        step: range.step,
        count: u64::from(span / u32::from(range.step) + 1),
    })
}

fn canonical_float_axis(label: &str, range: &VcetQuantizedFloatRange) -> Result<I64Axis, String> {
    let min = float_to_units(label, "min", range.min)?;
    let max = float_to_units(label, "max", range.max)?;
    let quantum = float_to_units(label, "quantum", range.quantum)?;
    if quantum <= 0 {
        return Err(format!("VCET {label}.quantum must be > 0"));
    }
    if max < min {
        return Err(format!("VCET {label}.max must be >= min"));
    }
    let span = i128::from(max) - i128::from(min);
    if span % i128::from(quantum) != 0 {
        return Err(format!("VCET {label} endpoints must align to its quantum"));
    }
    let count = span / i128::from(quantum) + 1;
    let count = u64::try_from(count)
        .map_err(|_| format!("VCET {label} range has too many quantized values"))?;
    Ok(I64Axis {
        min,
        quantum,
        count,
    })
}

fn canonical_bool_choices(label: &str, choices: &VcetBoolChoices) -> Result<Vec<bool>, String> {
    let mut values = choices.values.clone();
    values.sort_unstable();
    values.dedup();
    if values.is_empty() {
        return Err(format!("VCET {label}.values must not be empty"));
    }
    Ok(values)
}

fn float_to_units(label: &str, field: &str, value: f64) -> Result<i64, String> {
    if !value.is_finite() {
        return Err(format!("VCET {label}.{field} must be finite"));
    }
    let scaled = value * FLOAT_SCALE as f64;
    if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
        return Err(format!("VCET {label}.{field} is out of range"));
    }
    let rounded = scaled.round();
    let tolerance = 1.0e-7_f64.max(scaled.abs() * f64::EPSILON * 8.0);
    if (scaled - rounded).abs() > tolerance {
        return Err(format!(
            "VCET {label}.{field} must align to 0.000001 canonical precision"
        ));
    }
    Ok(if rounded == 0.0 { 0 } else { rounded as i64 })
}

fn units_to_float(units: i64) -> f64 {
    units as f64 / FLOAT_SCALE as f64
}

fn build_valid_period_pair_buckets(
    short_axis: U16Axis,
    long_axis: U16Axis,
) -> Result<(Vec<ValidPeriodPairBucket>, u64), String> {
    let mut buckets = Vec::with_capacity(short_axis.count as usize);
    let mut cumulative_end = 0_u64;
    for short_index in 0..short_axis.count {
        let short = short_axis.value(short_index);
        let first_long_index = first_long_index_greater_than(long_axis, short);
        let long_count = long_axis.count.saturating_sub(first_long_index);
        if long_count == 0 {
            continue;
        }
        cumulative_end = cumulative_end
            .checked_add(long_count)
            .ok_or_else(|| "VCET valid period-pair count overflow".to_string())?;
        buckets.push(ValidPeriodPairBucket {
            short,
            first_long_index,
            long_count,
            cumulative_end,
        });
    }
    Ok((buckets, cumulative_end))
}

fn valid_search_space_size(axes: &CanonicalSearchAxes) -> Result<u128, String> {
    let valid_period_pairs = u128::from(axes.valid_period_pairs);

    [
        u128::from(axes.alpha.count),
        u128::from(axes.ce_period.count),
        u128::from(axes.ce_mult.count),
        axes.ce_use_close.len() as u128,
        axes.require_slope.len() as u128,
        u128::from(axes.max_trade_bars.count),
    ]
    .into_iter()
    .try_fold(valid_period_pairs, |total, count| {
        total
            .checked_mul(count)
            .ok_or_else(|| "VCET search-space cardinality overflow".to_string())
    })
}

fn first_long_index_greater_than(axis: U16Axis, short: u16) -> u64 {
    if axis.min > short {
        return 0;
    }
    let delta = u64::from(short - axis.min);
    (delta / u64::from(axis.step) + 1).min(axis.count)
}

fn sample_halton_candidate(
    axes: &CanonicalSearchAxes,
    sample_index: u64,
    seed: u64,
) -> VcetCandidateIdentityInputs {
    let unit = |dimension: usize| {
        shifted_radical_inverse(
            sample_index,
            HALTON_BASES[dimension],
            halton_rotation(seed, dimension),
        )
    };
    let index = |dimension: usize, count: u64| discrete_index(unit(dimension), count);
    let (vidya_short_period, vidya_long_period) = sample_valid_period_pair(axes, unit(0), unit(1));
    VcetCandidateIdentityInputs {
        vidya_short_period,
        vidya_long_period,
        vidya_alpha_micros: axes.alpha.value(index(2, axes.alpha.count)),
        ce_period: axes.ce_period.value(index(3, axes.ce_period.count)),
        ce_mult_micros: axes.ce_mult.value(index(4, axes.ce_mult.count)),
        ce_use_close: axes.ce_use_close[index(5, axes.ce_use_close.len() as u64) as usize],
        require_vidya_slope: axes.require_slope[index(6, axes.require_slope.len() as u64) as usize],
        max_trade_bars: axes
            .max_trade_bars
            .value(index(7, axes.max_trade_bars.count)),
    }
}

/// Samples only the valid triangular period-pair domain. Selecting a short
/// bucket in proportion to its number of admissible long periods, then a long
/// value within that bucket, gives every valid pair equal measure without the
/// rejection/fallback bias that occurs when the rectangular axes are sampled
/// independently.
fn sample_valid_period_pair(
    axes: &CanonicalSearchAxes,
    short_unit: f64,
    long_unit: f64,
) -> (u16, u16) {
    debug_assert!(axes.valid_period_pairs > 0);
    let pair_rank = discrete_index(short_unit, axes.valid_period_pairs);
    let bucket_index = axes
        .valid_period_pair_buckets
        .partition_point(|bucket| bucket.cumulative_end <= pair_rank);
    let bucket = axes.valid_period_pair_buckets[bucket_index];
    let long_offset = discrete_index(long_unit, bucket.long_count);
    (
        bucket.short,
        axes.long.value(bucket.first_long_index + long_offset),
    )
}

fn discrete_index(unit: f64, count: u64) -> u64 {
    debug_assert!(count > 0);
    let index = (unit * count as f64).floor() as u64;
    index.min(count - 1)
}

fn radical_inverse(mut index: u64, base: u32) -> f64 {
    debug_assert!(base >= 2);
    let base_u64 = u64::from(base);
    let inverse_base = 1.0 / f64::from(base);
    let mut fraction = inverse_base;
    let mut result = 0.0;
    while index > 0 {
        result += (index % base_u64) as f64 * fraction;
        index /= base_u64;
        fraction *= inverse_base;
    }
    result
}

fn shifted_radical_inverse(index: u64, base: u32, rotation: f64) -> f64 {
    let shifted = radical_inverse(index, base) + rotation;
    shifted - shifted.floor()
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn halton_rotation(seed: u64, dimension: usize) -> f64 {
    let mixed = splitmix64(seed ^ (dimension as u64 + 1).wrapping_mul(0xd1b5_4a32_d192_ed03));
    // Use the high 53 bits so conversion is exact and the result stays in [0, 1).
    ((mixed >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64))
}

fn fill_exhaustively(
    axes: &CanonicalSearchAxes,
    target: usize,
    identities: &mut BTreeSet<VcetCandidateIdentityInputs>,
    stats: &mut VcetSearchGenerationStats,
) {
    'outer: for short_index in 0..axes.short.count {
        let vidya_short_period = axes.short.value(short_index);
        let first_long = first_long_index_greater_than(axes.long, vidya_short_period);
        for long_index in first_long..axes.long.count {
            let vidya_long_period = axes.long.value(long_index);
            for alpha_index in 0..axes.alpha.count {
                for ce_period_index in 0..axes.ce_period.count {
                    for ce_mult_index in 0..axes.ce_mult.count {
                        for &ce_use_close in &axes.ce_use_close {
                            for &require_vidya_slope in &axes.require_slope {
                                for max_trade_index in 0..axes.max_trade_bars.count {
                                    if identities.len() >= target {
                                        break 'outer;
                                    }
                                    let identity = VcetCandidateIdentityInputs {
                                        vidya_short_period,
                                        vidya_long_period,
                                        vidya_alpha_micros: axes.alpha.value(alpha_index),
                                        ce_period: axes.ce_period.value(ce_period_index),
                                        ce_mult_micros: axes.ce_mult.value(ce_mult_index),
                                        ce_use_close,
                                        require_vidya_slope,
                                        max_trade_bars: axes.max_trade_bars.value(max_trade_index),
                                    };
                                    if identities.insert(identity) {
                                        stats.exhaustive_candidates =
                                            stats.exhaustive_candidates.saturating_add(1);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn search_id(
    cfg: &VcetSearchConfig,
    axes: &CanonicalSearchAxes,
    sequence: VcetSearchSequence,
) -> String {
    let bools = |values: &[bool]| {
        values
            .iter()
            .map(|value| u8::from(*value).to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let sequence_key = match sequence {
        VcetSearchSequence::Halton => "halton_legacy",
        VcetSearchSequence::HaltonCranleyPatterson => "halton_cranley_patterson",
        VcetSearchSequence::ExhaustiveLexicographic => "exhaustive_lexicographic",
    };
    let key = format!(
        "vcet_search_v1|sequence={sequence_key}|period_pair_mapping=weighted_conditional_v1|randomization=cp_splitmix64_v1|attempts_per_target={HALTON_ATTEMPTS_PER_TARGET}|dedup=fail_closed_v1|seed={}|budget={}|short={}:{}:{}|long={}:{}:{}|alpha={}:{}:{}|ce_period={}:{}:{}|ce_mult={}:{}:{}|ce_use_close={}|require_slope={}|max_trade_bars={}:{}:{}",
        cfg.seed,
        cfg.budget,
        axes.short.min,
        axes.short.step,
        axes.short.count,
        axes.long.min,
        axes.long.step,
        axes.long.count,
        axes.alpha.min,
        axes.alpha.quantum,
        axes.alpha.count,
        axes.ce_period.min,
        axes.ce_period.step,
        axes.ce_period.count,
        axes.ce_mult.min,
        axes.ce_mult.quantum,
        axes.ce_mult.count,
        bools(&axes.ce_use_close),
        bools(&axes.require_slope),
        axes.max_trade_bars.min,
        axes.max_trade_bars.step,
        axes.max_trade_bars.count,
    );
    format!("vcet_search_v1_{}", sha256_hex(key.as_bytes()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compact_config(budget: usize) -> VcetSearchConfig {
        VcetSearchConfig {
            budget,
            seed: 41,
            vidya_short_period: VcetU16Range {
                min: 2,
                max: 6,
                step: 2,
            },
            vidya_long_period: VcetU16Range {
                min: 4,
                max: 10,
                step: 2,
            },
            vidya_alpha: VcetQuantizedFloatRange {
                min: 0.0,
                max: 0.4,
                quantum: 0.2,
            },
            ce_period: VcetU16Range {
                min: 2,
                max: 4,
                step: 2,
            },
            ce_mult: VcetQuantizedFloatRange {
                min: 1.5,
                max: 2.0,
                quantum: 0.5,
            },
            ce_use_close: VcetBoolChoices::both(),
            require_vidya_slope: VcetBoolChoices::both(),
            max_trade_bars: VcetU16Range {
                min: 10,
                max: 20,
                step: 10,
            },
        }
    }

    #[test]
    fn generation_is_deterministic_budgeted_unique_and_canonically_ordered() {
        let cfg = compact_config(73);
        let first = generate_vcet_search_manifest(&cfg).unwrap();
        let second = generate_vcet_search_manifest(&cfg).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.requested_budget, 73);
        assert_eq!(first.effective_budget, 73);
        assert_eq!(first.candidates.len(), 73);
        assert_eq!(first.sequence, VcetSearchSequence::HaltonCranleyPatterson);
        assert_eq!(first.generation_stats.invalid_period_pairs_discarded, 0);
        assert_eq!(first.generation_stats.lexicographic_fallback_candidates, 0);
        assert_eq!(first.generation_stats.exhaustive_candidates, 0);

        let ids = first
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), first.candidates.len());
        assert!(first.candidates.windows(2).all(|window| {
            window[0].identity_inputs < window[1].identity_inputs
                && window[0].ordinal + 1 == window[1].ordinal
        }));
        assert!(first.candidates.iter().all(|candidate| {
            candidate.effective.vidya_short_period >= 2
                && candidate.effective.vidya_long_period > candidate.effective.vidya_short_period
                && (0.0..=1.0).contains(&candidate.effective.vidya_alpha)
                && candidate.effective.ce_period > 0
                && candidate.effective.ce_mult > 0.0
                && candidate.effective.max_trade_bars > 0
        }));
    }

    #[test]
    fn seed_changes_sample_membership_but_not_identity_or_ordering_rules() {
        let first = generate_vcet_search_manifest(&compact_config(31)).unwrap();
        let mut changed = compact_config(31);
        changed.seed += 1_000;
        let second = generate_vcet_search_manifest(&changed).unwrap();

        assert_ne!(first.search_id, second.search_id);
        let first_ids = first
            .candidates
            .iter()
            .map(|candidate| &candidate.candidate_id)
            .collect::<BTreeSet<_>>();
        let second_ids = second
            .candidates
            .iter()
            .map(|candidate| &candidate.candidate_id)
            .collect::<BTreeSet<_>>();
        assert_ne!(first_ids, second_ids);
        assert!(second
            .candidates
            .windows(2)
            .all(|window| window[0].identity_inputs < window[1].identity_inputs));
    }

    #[test]
    fn adjacent_seeds_are_independent_randomized_designs_not_one_row_offsets() {
        let mut first_cfg = VcetSearchConfig::default();
        first_cfg.budget = 256;
        first_cfg.seed = 0;
        let mut second_cfg = first_cfg.clone();
        second_cfg.seed = 1;
        let first = generate_vcet_search_manifest(&first_cfg).unwrap();
        let second = generate_vcet_search_manifest(&second_cfg).unwrap();
        let first_ids = first
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect::<BTreeSet<_>>();
        let second_ids = second
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_id.as_str())
            .collect::<BTreeSet<_>>();
        let overlap = first_ids.intersection(&second_ids).count();

        assert!(
            overlap < first.effective_budget / 4,
            "adjacent seed designs overlap too heavily: {overlap}/{}",
            first.effective_budget
        );
        assert_ne!(first.candidates, second.candidates);
    }

    #[test]
    fn seeded_randomized_halton_has_a_stable_golden_prefix() {
        let manifest = generate_vcet_search_manifest(&compact_config(4)).unwrap();
        let identities: Vec<String> = manifest
            .candidates
            .iter()
            .map(|candidate| candidate.identity_inputs.canonical_key())
            .collect();
        assert_eq!(
            identities,
            vec![
                "vcet_effective_v1|vs=2|vl=4|alpha_micros=0|ce_period=2|ce_mult_micros=1500000|ce_use_close=0|require_vidya_slope=0|max_trade_bars=20",
                "vcet_effective_v1|vs=2|vl=8|alpha_micros=400000|ce_period=4|ce_mult_micros=1500000|ce_use_close=0|require_vidya_slope=0|max_trade_bars=20",
                "vcet_effective_v1|vs=4|vl=8|alpha_micros=200000|ce_period=2|ce_mult_micros=1500000|ce_use_close=0|require_vidya_slope=0|max_trade_bars=20",
                "vcet_effective_v1|vs=6|vl=10|alpha_micros=200000|ce_period=2|ce_mult_micros=1500000|ce_use_close=0|require_vidya_slope=1|max_trade_bars=20",
            ]
        );
    }

    #[test]
    fn budget_above_space_cardinality_emits_every_candidate_once() {
        let mut cfg = compact_config(999);
        cfg.vidya_short_period.max = 2;
        cfg.vidya_long_period.min = 4;
        cfg.vidya_long_period.max = 4;
        cfg.vidya_alpha.max = 0.0;
        cfg.ce_period.max = 2;
        cfg.ce_mult.max = 1.5;
        cfg.ce_use_close = VcetBoolChoices::fixed(true);
        cfg.require_vidya_slope = VcetBoolChoices::fixed(false);
        cfg.max_trade_bars.max = 10;

        let manifest = generate_vcet_search_manifest(&cfg).unwrap();
        assert_eq!(manifest.valid_search_space_size, "1");
        assert_eq!(manifest.requested_budget, 999);
        assert_eq!(manifest.effective_budget, 1);
        assert_eq!(manifest.candidates.len(), 1);
        assert_eq!(
            manifest.sequence,
            VcetSearchSequence::ExhaustiveLexicographic
        );
        assert_eq!(manifest.generation_stats.halton_attempts, 0);
        assert_eq!(manifest.generation_stats.exhaustive_candidates, 1);
        assert_eq!(
            manifest.generation_stats.lexicographic_fallback_candidates,
            0
        );
    }

    #[test]
    fn skewed_valid_period_domain_is_sampled_without_rejection_or_fallback() {
        let mut cfg = compact_config(128);
        cfg.vidya_short_period = VcetU16Range {
            min: 2,
            max: 62,
            step: 2,
        };
        cfg.vidya_long_period = VcetU16Range {
            min: 50,
            max: 64,
            step: 2,
        };

        let manifest = generate_vcet_search_manifest(&cfg).unwrap();
        assert_eq!(
            manifest.sequence,
            VcetSearchSequence::HaltonCranleyPatterson
        );
        assert_eq!(manifest.candidates.len(), 128);
        assert_eq!(manifest.generation_stats.invalid_period_pairs_discarded, 0);
        assert_eq!(
            manifest.generation_stats.lexicographic_fallback_candidates,
            0
        );
        assert_eq!(manifest.generation_stats.exhaustive_candidates, 0);
        assert!(manifest
            .candidates
            .iter()
            .all(|candidate| candidate.identity_inputs.vidya_long_period
                > candidate.identity_inputs.vidya_short_period));
        let distinct_shorts = manifest
            .candidates
            .iter()
            .map(|candidate| candidate.identity_inputs.vidya_short_period)
            .collect::<BTreeSet<_>>();
        assert!(
            distinct_shorts.len() >= 12,
            "weighted triangular sampler covered only {} short-period buckets",
            distinct_shorts.len()
        );
    }

    #[test]
    fn bool_choice_input_order_and_duplicates_do_not_change_identity() {
        let first = generate_vcet_search_manifest(&compact_config(25)).unwrap();
        let mut changed = compact_config(25);
        changed.ce_use_close.values = vec![true, false, true, false];
        changed.require_vidya_slope.values = vec![true, false, false];
        let second = generate_vcet_search_manifest(&changed).unwrap();

        assert_eq!(first.search_id, second.search_id);
        assert_eq!(first.candidates, second.candidates);
    }

    #[test]
    fn validation_rejects_unbounded_or_noncanonical_contracts() {
        let mut cfg = compact_config(10);
        cfg.budget = 0;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            "VCET search budget must be > 0"
        );

        cfg = compact_config(10);
        cfg.budget = MAX_VCET_SEARCH_BUDGET + 1;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            format!("VCET search budget must be <= {MAX_VCET_SEARCH_BUDGET}")
        );

        cfg = compact_config(10);
        cfg.vidya_short_period.min = 1;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            "VCET vidya_short_period.min must be >= 2"
        );

        cfg = compact_config(10);
        cfg.vidya_alpha.max = 1.2;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            "VCET vidya_alpha range must stay within [0, 1]"
        );

        cfg = compact_config(10);
        cfg.ce_mult.min = 0.0;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            "VCET ce_mult range must stay within (0, 1000]"
        );

        cfg = compact_config(10);
        cfg.seed = MAX_SAFE_VCET_SEARCH_SEED;
        validate_vcet_search_config(&cfg).expect("maximum JSON-safe seed is valid");
        cfg.seed += 1;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            format!("VCET search seed must be <= {MAX_SAFE_VCET_SEARCH_SEED}")
        );

        cfg = compact_config(10);
        cfg.ce_mult.max = MAX_VCET_SEARCH_CE_MULT;
        cfg.ce_mult.quantum = 0.5;
        validate_vcet_search_config(&cfg).expect("product CE multiplier maximum is valid");
        cfg.ce_mult.max += 0.5;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            "VCET ce_mult range must stay within (0, 1000]"
        );

        cfg = compact_config(10);
        cfg.vidya_alpha.quantum = 0.0000004;
        assert_eq!(
            validate_vcet_search_config(&cfg).unwrap_err(),
            "VCET vidya_alpha.quantum must align to 0.000001 canonical precision"
        );
    }

    #[test]
    fn serde_round_trip_preserves_manifest_identity_material() {
        let manifest = generate_vcet_search_manifest(&compact_config(17)).unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        let decoded: VcetSearchManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, manifest);
        assert!(decoded.candidates.iter().all(|candidate| {
            candidate.candidate_id == candidate.identity_inputs.candidate_id()
        }));

        let mut legacy_value = serde_json::to_value(&manifest).unwrap();
        legacy_value["generation_stats"]
            .as_object_mut()
            .unwrap()
            .remove("exhaustive_candidates");
        let legacy_decoded: VcetSearchManifest = serde_json::from_value(legacy_value).unwrap();
        assert_eq!(legacy_decoded.generation_stats.exhaustive_candidates, 0);
    }
}
