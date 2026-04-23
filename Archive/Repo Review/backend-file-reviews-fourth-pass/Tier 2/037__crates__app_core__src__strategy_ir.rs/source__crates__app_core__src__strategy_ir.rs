use crate::composite_cpu::{MaczGateParams, OttGateParams};
use crate::double_ma::HalfTrendRegimeFilter;
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, normalize_and_validate_indicator_output_ref,
    normalize_and_validate_indicator_params, IndicatorParamKV,
};
use crate::CompositeSignalMode;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
use ta_strategies::double_ma::StrategyConfig;

pub const STRATEGY_IR_SCHEMA_VERSION: u16 = 2;
pub const STRATEGY_IR_MIN_SUPPORTED_SCHEMA_VERSION: u16 = 1;

pub const IR_MAX_INDICATORS: usize = 64;
pub const IR_MAX_CONDITIONS: usize = 256;
pub const IR_MAX_GROUPS: usize = 64;
pub const IR_MAX_GROUP_CONDITIONS: usize = 64;

fn default_strategy_ir_schema_version() -> u16 {
    STRATEGY_IR_SCHEMA_VERSION
}

fn default_ma_source() -> String {
    "close".to_string()
}

fn default_halftrend_filter() -> HalfTrendRegimeFilter {
    HalfTrendRegimeFilter {
        amplitude: 2,
        channel_deviation: 2.0,
        atr_period: 100,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrategyIrSchemaCompatibility {
    Compatible,
    UnsupportedLegacy { found: u16, min_supported: u16 },
    UnsupportedFuture { found: u16, max_supported: u16 },
}

pub fn assess_strategy_ir_schema_compatibility(version: u16) -> StrategyIrSchemaCompatibility {
    if version < STRATEGY_IR_MIN_SUPPORTED_SCHEMA_VERSION {
        StrategyIrSchemaCompatibility::UnsupportedLegacy {
            found: version,
            min_supported: STRATEGY_IR_MIN_SUPPORTED_SCHEMA_VERSION,
        }
    } else if version > STRATEGY_IR_SCHEMA_VERSION {
        StrategyIrSchemaCompatibility::UnsupportedFuture {
            found: version,
            max_supported: STRATEGY_IR_SCHEMA_VERSION,
        }
    } else {
        StrategyIrSchemaCompatibility::Compatible
    }
}

fn ensure_schema_supported(version: u16) -> Result<(), String> {
    match assess_strategy_ir_schema_compatibility(version) {
        StrategyIrSchemaCompatibility::Compatible => Ok(()),
        StrategyIrSchemaCompatibility::UnsupportedLegacy {
            found,
            min_supported,
        } => Err(format!(
            "strategy_ir schema {found} is too old (minimum supported is {min_supported})"
        )),
        StrategyIrSchemaCompatibility::UnsupportedFuture {
            found,
            max_supported,
        } => Err(format!(
            "strategy_ir schema {found} is newer than this build supports (max {max_supported})"
        )),
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StrategyProfileKind {
    DoubleMa,
    CompositeCpu,
    CompositeTrend,
    RegimeHybrid,
    BreakoutMomentum,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct MovingAverageSpec {
    pub ma_type: String,
    #[serde(default)]
    pub params: HashMap<String, f64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorKind {
    FastMa,
    SlowMa,
    HalfTrendTrend,
    MaczHist,
    OttLine,
    TrendQualityRegime,
    BreakoutRegime,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValueRef {
    Indicator {
        id: String,
    },
    MaSource,
    Close,
    ConstantBits {
        #[serde(deserialize_with = "deserialize_constant_bits_u64")]
        bits: u64,
    },
}

impl ValueRef {
    pub fn constant(v: f64) -> Self {
        Self::ConstantBits { bits: v.to_bits() }
    }
}

fn deserialize_constant_bits_u64<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawConstantBits {
        Number(u64),
        String(String),
    }

    match RawConstantBits::deserialize(deserializer)? {
        RawConstantBits::Number(bits) => Ok(bits),
        RawConstantBits::String(bits) => {
            bits.trim().parse::<u64>().map_err(serde::de::Error::custom)
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOp {
    Gt,
    Ge,
    Lt,
    Le,
    CrossUp,
    CrossDown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct ConditionNode {
    pub id: String,
    pub op: ConditionOp,
    pub lhs: ValueRef,
    pub rhs: ValueRef,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConditionGroupMode {
    All,
    Any,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct ConditionGroup {
    pub id: String,
    pub mode: ConditionGroupMode,
    pub condition_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct SignalBindings {
    pub long_entry_group: Option<String>,
    pub long_exit_group: Option<String>,
    pub short_entry_group: Option<String>,
    pub short_exit_group: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct IndicatorNode {
    pub id: String,
    #[serde(default)]
    pub kind: Option<IndicatorKind>,
    #[serde(default)]
    pub indicator_id: Option<String>,
    #[serde(default)]
    pub params: Vec<IndicatorParamKV>,
    #[serde(default)]
    pub output_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SignalProgramGraph {
    #[serde(default)]
    pub indicators: Vec<IndicatorNode>,
    #[serde(default)]
    pub conditions: Vec<ConditionNode>,
    #[serde(default)]
    pub groups: Vec<ConditionGroup>,
    pub signals: SignalBindings,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExecutionPolicy {
    pub strategy: StrategyConfig,
    #[serde(default)]
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    #[serde(default)]
    pub halftrend: Option<HalfTrendRegimeFilter>,
    #[serde(default)]
    pub macz: Option<MaczGateParams>,
    #[serde(default)]
    pub ott: Option<OttGateParams>,
    #[serde(default)]
    pub signal_mode: Option<CompositeSignalMode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StrategyIr {
    #[serde(default = "default_strategy_ir_schema_version")]
    pub schema_version: u16,
    pub profile: StrategyProfileKind,
    #[serde(default = "default_ma_source")]
    pub ma_source: String,
    pub fast_ma: MovingAverageSpec,
    pub slow_ma: MovingAverageSpec,
    pub program: SignalProgramGraph,
    pub execution: ExecutionPolicy,
}

#[derive(Clone, Debug)]
pub struct StaticStrategyIrInput {
    pub ma_source: String,
    pub fast_ma_type: String,
    pub slow_ma_type: String,
    pub fast_ma_params: Option<HashMap<String, f64>>,
    pub slow_ma_params: Option<HashMap<String, f64>>,
    pub strategy: StrategyConfig,
    pub halftrend_filter: Option<HalfTrendRegimeFilter>,
    pub halftrend: Option<HalfTrendRegimeFilter>,
    pub macz: Option<MaczGateParams>,
    pub ott: Option<OttGateParams>,
    pub signal_mode: Option<CompositeSignalMode>,
}

impl StaticStrategyIrInput {
    pub fn from_double_ma(
        ma_source: String,
        fast_ma_type: String,
        slow_ma_type: String,
        fast_ma_params: Option<HashMap<String, f64>>,
        slow_ma_params: Option<HashMap<String, f64>>,
        strategy: StrategyConfig,
        halftrend_filter: Option<HalfTrendRegimeFilter>,
    ) -> Self {
        Self {
            ma_source,
            fast_ma_type,
            slow_ma_type,
            fast_ma_params,
            slow_ma_params,
            strategy,
            halftrend_filter,
            halftrend: None,
            macz: None,
            ott: None,
            signal_mode: None,
        }
    }
}

fn normalize_nonempty(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(trimmed.to_ascii_lowercase())
    }
}

fn validate_ma_spec(spec: &MovingAverageSpec, field: &str) -> Result<(), String> {
    let ma_type = spec.ma_type.trim();
    if ma_type.is_empty() {
        return Err(format!("{field}.ma_type must not be empty"));
    }
    for (k, v) in &spec.params {
        let key = k.trim();
        if key.is_empty() {
            return Err(format!("{field}.params contains an empty key"));
        }
        if !v.is_finite() {
            return Err(format!("{field}.params['{key}'] must be finite (got {v})"));
        }
    }
    Ok(())
}

fn validate_program(program: &SignalProgramGraph) -> Result<(), String> {
    if program.indicators.len() > IR_MAX_INDICATORS {
        return Err(format!(
            "strategy_ir has too many indicators ({} > {})",
            program.indicators.len(),
            IR_MAX_INDICATORS
        ));
    }
    if program.conditions.len() > IR_MAX_CONDITIONS {
        return Err(format!(
            "strategy_ir has too many conditions ({} > {})",
            program.conditions.len(),
            IR_MAX_CONDITIONS
        ));
    }
    if program.groups.len() > IR_MAX_GROUPS {
        return Err(format!(
            "strategy_ir has too many groups ({} > {})",
            program.groups.len(),
            IR_MAX_GROUPS
        ));
    }

    let mut indicator_ids: HashSet<&str> = HashSet::new();
    for indicator in &program.indicators {
        let id = indicator.id.trim();
        if id.is_empty() {
            return Err("strategy_ir indicator id must not be empty".to_string());
        }
        if !indicator_ids.insert(id) {
            return Err(format!("duplicate indicator id '{id}'"));
        }

        let kind = indicator.kind;
        let source_id = indicator
            .indicator_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_ascii_lowercase());

        if kind.is_some() && source_id.is_some() {
            return Err(format!(
                "indicator '{id}' cannot specify both 'kind' and 'indicator_id'"
            ));
        }
        if kind.is_none() && source_id.is_none() {
            return Err(format!(
                "indicator '{id}' must specify either 'kind' or 'indicator_id'"
            ));
        }

        if kind.is_some() {
            if !indicator.params.is_empty() {
                return Err(format!(
                    "indicator '{id}' cannot include params when using legacy 'kind'"
                ));
            }
            if indicator
                .output_id
                .as_deref()
                .map(str::trim)
                .is_some_and(|v| !v.is_empty())
            {
                return Err(format!(
                    "indicator '{id}' cannot include output_id when using legacy 'kind'"
                ));
            }
        }

        if let Some(source_id) = source_id {
            if find_dynamic_indicator_descriptor(source_id.as_str()).is_none() {
                return Err(format!(
                    "indicator '{id}' references unknown indicator_id '{}'",
                    source_id
                ));
            }
            normalize_and_validate_indicator_params(
                source_id.as_str(),
                indicator.params.as_slice(),
            )
            .map_err(|e| format!("indicator '{id}' has invalid params: {e}"))?;
            normalize_and_validate_indicator_output_ref(
                source_id.as_str(),
                indicator.output_id.as_deref(),
            )
            .map_err(|e| format!("indicator '{id}' has invalid output ref: {e}"))?;
        }
    }

    let mut condition_ids: HashSet<&str> = HashSet::new();
    for condition in &program.conditions {
        let id = condition.id.trim();
        if id.is_empty() {
            return Err("strategy_ir condition id must not be empty".to_string());
        }
        if !condition_ids.insert(id) {
            return Err(format!("duplicate condition id '{id}'"));
        }
        for value_ref in [&condition.lhs, &condition.rhs] {
            if let ValueRef::Indicator { id } = value_ref {
                let indicator_id = id.trim();
                if !indicator_ids.contains(indicator_id) {
                    return Err(format!(
                        "condition '{id}' references unknown indicator '{indicator_id}'"
                    ));
                }
            }
        }
    }

    let mut group_ids: HashSet<&str> = HashSet::new();
    for group in &program.groups {
        let id = group.id.trim();
        if id.is_empty() {
            return Err("strategy_ir group id must not be empty".to_string());
        }
        if !group_ids.insert(id) {
            return Err(format!("duplicate group id '{id}'"));
        }
        if group.condition_ids.is_empty() {
            return Err(format!("group '{id}' must include at least one condition"));
        }
        if group.condition_ids.len() > IR_MAX_GROUP_CONDITIONS {
            return Err(format!(
                "group '{id}' has too many conditions ({} > {})",
                group.condition_ids.len(),
                IR_MAX_GROUP_CONDITIONS
            ));
        }
        for cond_id in &group.condition_ids {
            let condition_id = cond_id.trim();
            if !condition_ids.contains(condition_id) {
                return Err(format!(
                    "group '{id}' references unknown condition '{condition_id}'"
                ));
            }
        }
    }

    let signal_groups = [
        program.signals.long_entry_group.as_ref(),
        program.signals.long_exit_group.as_ref(),
        program.signals.short_entry_group.as_ref(),
        program.signals.short_exit_group.as_ref(),
    ];
    for group_ref in signal_groups.into_iter().flatten() {
        let id = group_ref.trim();
        if !group_ids.contains(id) {
            return Err(format!("signal binding references unknown group '{id}'"));
        }
    }
    Ok(())
}

pub fn validate_strategy_ir(ir: &StrategyIr) -> Result<(), String> {
    ensure_schema_supported(ir.schema_version)?;
    validate_ma_spec(&ir.fast_ma, "fast_ma")?;
    validate_ma_spec(&ir.slow_ma, "slow_ma")?;
    let ma_source = ir.ma_source.trim();
    if ma_source.is_empty() {
        return Err("ma_source must not be empty".to_string());
    }
    validate_program(&ir.program)?;
    Ok(())
}

fn make_condition(id: &str, op: ConditionOp, lhs: ValueRef, rhs: ValueRef) -> ConditionNode {
    ConditionNode {
        id: id.to_string(),
        op,
        lhs,
        rhs,
    }
}

fn make_group(id: &str, mode: ConditionGroupMode, condition_ids: &[&str]) -> ConditionGroup {
    ConditionGroup {
        id: id.to_string(),
        mode,
        condition_ids: condition_ids.iter().map(|v| v.to_string()).collect(),
    }
}

fn make_static_indicator(id: &str, kind: IndicatorKind) -> IndicatorNode {
    IndicatorNode {
        id: id.to_string(),
        kind: Some(kind),
        indicator_id: None,
        params: Vec::new(),
        output_id: None,
    }
}

fn static_program_double_ma() -> SignalProgramGraph {
    SignalProgramGraph {
        indicators: vec![
            make_static_indicator("fast_ma", IndicatorKind::FastMa),
            make_static_indicator("slow_ma", IndicatorKind::SlowMa),
        ],
        conditions: vec![
            make_condition(
                "fast_cross_up_slow",
                ConditionOp::CrossUp,
                ValueRef::Indicator {
                    id: "fast_ma".to_string(),
                },
                ValueRef::Indicator {
                    id: "slow_ma".to_string(),
                },
            ),
            make_condition(
                "fast_cross_down_slow",
                ConditionOp::CrossDown,
                ValueRef::Indicator {
                    id: "fast_ma".to_string(),
                },
                ValueRef::Indicator {
                    id: "slow_ma".to_string(),
                },
            ),
        ],
        groups: vec![
            make_group(
                "long_entry",
                ConditionGroupMode::All,
                &["fast_cross_up_slow"],
            ),
            make_group(
                "long_exit",
                ConditionGroupMode::All,
                &["fast_cross_down_slow"],
            ),
            make_group(
                "short_entry",
                ConditionGroupMode::All,
                &["fast_cross_down_slow"],
            ),
            make_group(
                "short_exit",
                ConditionGroupMode::All,
                &["fast_cross_up_slow"],
            ),
        ],
        signals: SignalBindings {
            long_entry_group: Some("long_entry".to_string()),
            long_exit_group: Some("long_exit".to_string()),
            short_entry_group: Some("short_entry".to_string()),
            short_exit_group: Some("short_exit".to_string()),
        },
    }
}

fn static_program_composite_base(
    include_trend: bool,
    include_breakout: bool,
) -> SignalProgramGraph {
    let mut indicators = vec![
        make_static_indicator("fast_ma", IndicatorKind::FastMa),
        make_static_indicator("slow_ma", IndicatorKind::SlowMa),
        make_static_indicator("halftrend", IndicatorKind::HalfTrendTrend),
        make_static_indicator("macz_hist", IndicatorKind::MaczHist),
        make_static_indicator("ott_line", IndicatorKind::OttLine),
    ];
    if include_trend {
        indicators.push(make_static_indicator(
            "trend_quality",
            IndicatorKind::TrendQualityRegime,
        ));
    }
    if include_breakout {
        indicators.push(make_static_indicator(
            "breakout_regime",
            IndicatorKind::BreakoutRegime,
        ));
    }

    let mut conditions = vec![
        make_condition(
            "fast_gt_slow",
            ConditionOp::Gt,
            ValueRef::Indicator {
                id: "fast_ma".to_string(),
            },
            ValueRef::Indicator {
                id: "slow_ma".to_string(),
            },
        ),
        make_condition(
            "fast_lt_slow",
            ConditionOp::Lt,
            ValueRef::Indicator {
                id: "fast_ma".to_string(),
            },
            ValueRef::Indicator {
                id: "slow_ma".to_string(),
            },
        ),
        make_condition(
            "halftrend_long",
            ConditionOp::Lt,
            ValueRef::Indicator {
                id: "halftrend".to_string(),
            },
            ValueRef::constant(0.5),
        ),
        make_condition(
            "halftrend_short",
            ConditionOp::Ge,
            ValueRef::Indicator {
                id: "halftrend".to_string(),
            },
            ValueRef::constant(0.5),
        ),
        make_condition(
            "macz_pos",
            ConditionOp::Gt,
            ValueRef::Indicator {
                id: "macz_hist".to_string(),
            },
            ValueRef::constant(0.0),
        ),
        make_condition(
            "macz_neg",
            ConditionOp::Lt,
            ValueRef::Indicator {
                id: "macz_hist".to_string(),
            },
            ValueRef::constant(0.0),
        ),
        make_condition(
            "price_gt_ott",
            ConditionOp::Gt,
            ValueRef::MaSource,
            ValueRef::Indicator {
                id: "ott_line".to_string(),
            },
        ),
        make_condition(
            "price_lt_ott",
            ConditionOp::Lt,
            ValueRef::MaSource,
            ValueRef::Indicator {
                id: "ott_line".to_string(),
            },
        ),
    ];

    let mut long_conds: Vec<&str> =
        vec!["fast_gt_slow", "halftrend_long", "macz_pos", "price_gt_ott"];
    let mut short_conds: Vec<&str> = vec![
        "fast_lt_slow",
        "halftrend_short",
        "macz_neg",
        "price_lt_ott",
    ];

    if include_trend {
        conditions.push(make_condition(
            "trend_regime_long",
            ConditionOp::Gt,
            ValueRef::Indicator {
                id: "trend_quality".to_string(),
            },
            ValueRef::constant(0.0),
        ));
        conditions.push(make_condition(
            "trend_regime_short",
            ConditionOp::Lt,
            ValueRef::Indicator {
                id: "trend_quality".to_string(),
            },
            ValueRef::constant(0.0),
        ));
        long_conds.push("trend_regime_long");
        short_conds.push("trend_regime_short");
    }

    if include_breakout {
        conditions.push(make_condition(
            "breakout_regime_long",
            ConditionOp::Gt,
            ValueRef::Indicator {
                id: "breakout_regime".to_string(),
            },
            ValueRef::constant(0.0),
        ));
        conditions.push(make_condition(
            "breakout_regime_short",
            ConditionOp::Lt,
            ValueRef::Indicator {
                id: "breakout_regime".to_string(),
            },
            ValueRef::constant(0.0),
        ));
        long_conds.push("breakout_regime_long");
        short_conds.push("breakout_regime_short");
    }

    SignalProgramGraph {
        indicators,
        conditions,
        groups: vec![
            make_group("long_entry", ConditionGroupMode::All, &long_conds),
            make_group("long_exit", ConditionGroupMode::All, &["fast_lt_slow"]),
            make_group("short_entry", ConditionGroupMode::All, &short_conds),
            make_group("short_exit", ConditionGroupMode::All, &["fast_gt_slow"]),
        ],
        signals: SignalBindings {
            long_entry_group: Some("long_entry".to_string()),
            long_exit_group: Some("long_exit".to_string()),
            short_entry_group: Some("short_entry".to_string()),
            short_exit_group: Some("short_exit".to_string()),
        },
    }
}

pub fn build_static_strategy_ir(
    profile: StrategyProfileKind,
    input: StaticStrategyIrInput,
) -> Result<StrategyIr, String> {
    let ma_source = normalize_nonempty(&input.ma_source, "ma_source")?;
    let fast_ma_type = normalize_nonempty(&input.fast_ma_type, "fast_ma_type")?;
    let slow_ma_type = normalize_nonempty(&input.slow_ma_type, "slow_ma_type")?;

    let mut strategy = input.strategy;
    let program = match profile {
        StrategyProfileKind::DoubleMa => static_program_double_ma(),
        StrategyProfileKind::CompositeCpu => {
            strategy.trend_quality_enable = false;
            strategy.regime_hybrid_enable = false;
            strategy.breakout_momentum_enable = false;
            static_program_composite_base(false, false)
        }
        StrategyProfileKind::CompositeTrend => {
            strategy.trend_quality_enable = true;
            strategy.regime_hybrid_enable = false;
            strategy.breakout_momentum_enable = false;
            static_program_composite_base(true, false)
        }
        StrategyProfileKind::RegimeHybrid => {
            strategy.trend_quality_enable = true;
            strategy.regime_hybrid_enable = true;
            strategy.breakout_momentum_enable = false;
            static_program_composite_base(true, false)
        }
        StrategyProfileKind::BreakoutMomentum => {
            strategy.trend_quality_enable = false;
            strategy.regime_hybrid_enable = false;
            strategy.breakout_momentum_enable = true;
            static_program_composite_base(false, true)
        }
    };

    let ir = StrategyIr {
        schema_version: STRATEGY_IR_SCHEMA_VERSION,
        profile,
        ma_source,
        fast_ma: MovingAverageSpec {
            ma_type: fast_ma_type,
            params: input.fast_ma_params.unwrap_or_default(),
        },
        slow_ma: MovingAverageSpec {
            ma_type: slow_ma_type,
            params: input.slow_ma_params.unwrap_or_default(),
        },
        program,
        execution: ExecutionPolicy {
            strategy,
            halftrend_filter: input.halftrend_filter,
            halftrend: match profile {
                StrategyProfileKind::DoubleMa => None,
                _ => Some(input.halftrend.unwrap_or_else(default_halftrend_filter)),
            },
            macz: match profile {
                StrategyProfileKind::DoubleMa => None,
                _ => Some(input.macz.unwrap_or_default()),
            },
            ott: match profile {
                StrategyProfileKind::DoubleMa => None,
                _ => Some(input.ott.unwrap_or_default()),
            },
            signal_mode: match profile {
                StrategyProfileKind::DoubleMa => None,
                _ => Some(input.signal_mode.unwrap_or(CompositeSignalMode::HardGate)),
            },
        },
    };
    validate_strategy_ir(&ir)?;
    Ok(ir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator_catalog::{IndicatorParamKV, IndicatorParamValue};
    use serde_json::json;

    #[test]
    fn schema_compatibility_reports_supported_versions() {
        assert_eq!(
            assess_strategy_ir_schema_compatibility(STRATEGY_IR_SCHEMA_VERSION),
            StrategyIrSchemaCompatibility::Compatible
        );
        assert!(matches!(
            assess_strategy_ir_schema_compatibility(0),
            StrategyIrSchemaCompatibility::UnsupportedLegacy { .. }
        ));
        assert!(matches!(
            assess_strategy_ir_schema_compatibility(STRATEGY_IR_SCHEMA_VERSION + 1),
            StrategyIrSchemaCompatibility::UnsupportedFuture { .. }
        ));
    }

    #[test]
    fn deserialize_defaults_schema_version_when_missing() {
        let v = json!({
            "profile": "double_ma",
            "ma_source": "close",
            "fast_ma": { "ma_type": "alma", "params": {} },
            "slow_ma": { "ma_type": "sma", "params": {} },
            "program": {
                "indicators": [{ "id": "fast_ma", "kind": "fast_ma" }, { "id": "slow_ma", "kind": "slow_ma" }],
                "conditions": [{ "id": "x", "op": "cross_up", "lhs": { "kind": "indicator", "id": "fast_ma" }, "rhs": { "kind": "indicator", "id": "slow_ma" } }],
                "groups": [{ "id": "g", "mode": "all", "condition_ids": ["x"] }],
                "signals": { "long_entry_group": "g", "long_exit_group": null, "short_entry_group": null, "short_exit_group": null }
            },
            "execution": { "strategy": StrategyConfig::default() }
        });
        let ir: StrategyIr = serde_json::from_value(v).expect("strategy_ir should deserialize");
        assert_eq!(ir.schema_version, STRATEGY_IR_SCHEMA_VERSION);
    }

    #[test]
    fn deserialize_schema_v1_payload_remains_supported() {
        let v = json!({
            "schema_version": 1,
            "profile": "double_ma",
            "ma_source": "close",
            "fast_ma": { "ma_type": "alma", "params": {} },
            "slow_ma": { "ma_type": "sma", "params": {} },
            "program": {
                "indicators": [
                    { "id": "fast_ma", "kind": "fast_ma" },
                    { "id": "slow_ma", "kind": "slow_ma" }
                ],
                "conditions": [
                    {
                        "id": "x",
                        "op": "cross_up",
                        "lhs": { "kind": "indicator", "id": "fast_ma" },
                        "rhs": { "kind": "indicator", "id": "slow_ma" }
                    }
                ],
                "groups": [{ "id": "g", "mode": "all", "condition_ids": ["x"] }],
                "signals": {
                    "long_entry_group": "g",
                    "long_exit_group": null,
                    "short_entry_group": null,
                    "short_exit_group": null
                }
            },
            "execution": { "strategy": StrategyConfig::default() }
        });
        let ir: StrategyIr =
            serde_json::from_value(v).expect("schema v1 payload should deserialize");
        assert_eq!(ir.schema_version, 1);
        validate_strategy_ir(&ir).expect("schema v1 payload should validate");
    }

    #[test]
    fn deserialize_constant_bits_from_decimal_string() {
        let value = 42.25f64;
        let bits = value.to_bits().to_string();
        let v = json!({
            "schema_version": STRATEGY_IR_SCHEMA_VERSION,
            "profile": "double_ma",
            "ma_source": "close",
            "fast_ma": { "ma_type": "alma", "params": {} },
            "slow_ma": { "ma_type": "sma", "params": {} },
            "program": {
                "indicators": [
                    { "id": "fast_ma", "kind": "fast_ma" },
                    { "id": "slow_ma", "kind": "slow_ma" }
                ],
                "conditions": [
                    {
                        "id": "x",
                        "op": "gt",
                        "lhs": { "kind": "indicator", "id": "fast_ma" },
                        "rhs": { "kind": "constant_bits", "bits": bits }
                    }
                ],
                "groups": [{ "id": "g", "mode": "all", "condition_ids": ["x"] }],
                "signals": {
                    "long_entry_group": "g",
                    "long_exit_group": null,
                    "short_entry_group": null,
                    "short_exit_group": null
                }
            },
            "execution": { "strategy": StrategyConfig::default() }
        });
        let ir: StrategyIr = serde_json::from_value(v).expect("string bits should deserialize");
        assert_eq!(ir.program.conditions.len(), 1);
        assert_eq!(
            ir.program.conditions[0].rhs,
            ValueRef::ConstantBits {
                bits: value.to_bits()
            }
        );
    }

    #[test]
    fn static_builders_generate_valid_ir_for_all_profiles() {
        let base = StaticStrategyIrInput {
            ma_source: "close".to_string(),
            fast_ma_type: "alma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            halftrend: None,
            macz: None,
            ott: None,
            signal_mode: None,
        };
        let profiles = [
            StrategyProfileKind::DoubleMa,
            StrategyProfileKind::CompositeCpu,
            StrategyProfileKind::CompositeTrend,
            StrategyProfileKind::RegimeHybrid,
            StrategyProfileKind::BreakoutMomentum,
        ];
        for profile in profiles {
            let ir =
                build_static_strategy_ir(profile, base.clone()).expect("builder should succeed");
            validate_strategy_ir(&ir).expect("generated IR should validate");
            match profile {
                StrategyProfileKind::DoubleMa => {
                    assert!(ir.execution.macz.is_none());
                    assert!(ir.execution.ott.is_none());
                    assert!(ir.execution.signal_mode.is_none());
                }
                _ => {
                    assert!(ir.execution.macz.is_some());
                    assert!(ir.execution.ott.is_some());
                    assert!(ir.execution.signal_mode.is_some());
                }
            }
        }
    }

    #[test]
    fn validate_strategy_ir_rejects_unknown_group_reference() {
        let mut ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "alma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("builder should produce valid IR");
        ir.program.signals.long_entry_group = Some("missing_group".to_string());
        let err = validate_strategy_ir(&ir).unwrap_err();
        assert!(err.contains("unknown group"));
    }

    #[test]
    fn validate_strategy_ir_accepts_dynamic_indicator_node_with_valid_schema() {
        let ir = StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION,
            profile: StrategyProfileKind::DoubleMa,
            ma_source: "close".to_string(),
            fast_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: HashMap::new(),
            },
            slow_ma: MovingAverageSpec {
                ma_type: "alma".to_string(),
                params: HashMap::new(),
            },
            program: SignalProgramGraph {
                indicators: vec![IndicatorNode {
                    id: "dyn_sma".to_string(),
                    kind: None,
                    indicator_id: Some("sma".to_string()),
                    params: vec![IndicatorParamKV {
                        key: "period".to_string(),
                        value: IndicatorParamValue::Int(20),
                    }],
                    output_id: Some("value".to_string()),
                }],
                conditions: vec![ConditionNode {
                    id: "c".to_string(),
                    op: ConditionOp::Gt,
                    lhs: ValueRef::Indicator {
                        id: "dyn_sma".to_string(),
                    },
                    rhs: ValueRef::constant(0.0),
                }],
                groups: vec![ConditionGroup {
                    id: "g".to_string(),
                    mode: ConditionGroupMode::All,
                    condition_ids: vec!["c".to_string()],
                }],
                signals: SignalBindings {
                    long_entry_group: Some("g".to_string()),
                    long_exit_group: None,
                    short_entry_group: None,
                    short_exit_group: None,
                },
            },
            execution: ExecutionPolicy {
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                halftrend: None,
                macz: None,
                ott: None,
                signal_mode: None,
            },
        };
        validate_strategy_ir(&ir).expect("dynamic indicator IR should validate");
    }

    #[test]
    fn validate_strategy_ir_rejects_unknown_dynamic_indicator_id() {
        let mut ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "alma".to_string(),
                "sma".to_string(),
                None,
                None,
                StrategyConfig::default(),
                None,
            ),
        )
        .expect("builder should produce valid IR");
        ir.program.indicators[0].kind = None;
        ir.program.indicators[0].indicator_id =
            Some("__definitely_unknown_indicator__".to_string());
        let err = validate_strategy_ir(&ir).expect_err("unknown indicator id should fail");
        assert!(err.contains("unknown indicator_id"));
    }

    #[test]
    fn validate_strategy_ir_rejects_missing_output_ref_for_multi_output_indicator() {
        let ir = StrategyIr {
            schema_version: STRATEGY_IR_SCHEMA_VERSION,
            profile: StrategyProfileKind::DoubleMa,
            ma_source: "close".to_string(),
            fast_ma: MovingAverageSpec {
                ma_type: "sma".to_string(),
                params: HashMap::new(),
            },
            slow_ma: MovingAverageSpec {
                ma_type: "alma".to_string(),
                params: HashMap::new(),
            },
            program: SignalProgramGraph {
                indicators: vec![IndicatorNode {
                    id: "dyn_macd".to_string(),
                    kind: None,
                    indicator_id: Some("macd".to_string()),
                    params: Vec::new(),
                    output_id: None,
                }],
                conditions: vec![ConditionNode {
                    id: "c".to_string(),
                    op: ConditionOp::Gt,
                    lhs: ValueRef::Indicator {
                        id: "dyn_macd".to_string(),
                    },
                    rhs: ValueRef::constant(0.0),
                }],
                groups: vec![ConditionGroup {
                    id: "g".to_string(),
                    mode: ConditionGroupMode::All,
                    condition_ids: vec!["c".to_string()],
                }],
                signals: SignalBindings {
                    long_entry_group: Some("g".to_string()),
                    long_exit_group: None,
                    short_entry_group: None,
                    short_exit_group: None,
                },
            },
            execution: ExecutionPolicy {
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                halftrend: None,
                macz: None,
                ott: None,
                signal_mode: None,
            },
        };
        let err = validate_strategy_ir(&ir).expect_err("missing output_id should fail");
        assert!(err.contains("invalid output ref"));
    }
}
