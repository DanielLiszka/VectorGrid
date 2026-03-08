use crate::strategy_ir::{ConditionGroupMode, ConditionOp, IndicatorKind, StrategyProfileKind};
use crate::strategy_plan::{
    CompiledStrategyPlan, CompiledValueRef, IndicatorRouteKind, StrategyPlanBackend,
};
use serde::{Deserialize, Serialize};

pub const STRATEGY_GPU_PROGRAM_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GpuValueSourceKind {
    Indicator,
    MaSource,
    Close,
    Constant,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuValueOperand {
    pub source: GpuValueSourceKind,
    pub index: u32,
    pub constant_bits: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuConditionPredicate {
    pub op: ConditionOp,
    pub lhs: GpuValueOperand,
    pub rhs: GpuValueOperand,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuConditionGroupSlice {
    pub mode: ConditionGroupMode,
    pub condition_offset: u32,
    pub condition_count: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuSignalBindings {
    pub long_entry_group: Option<u32>,
    pub long_exit_group: Option<u32>,
    pub short_entry_group: Option<u32>,
    pub short_exit_group: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuIndicatorSlot {
    pub indicator_index: u32,
    pub indicator_id: String,
    #[serde(default)]
    pub indicator_kind: Option<IndicatorKind>,
    #[serde(default)]
    pub source_indicator_id: Option<String>,
    #[serde(default)]
    pub output_id: Option<String>,
    pub route: IndicatorRouteKind,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompiledGpuPredicateProgram {
    pub schema_version: u16,
    pub profile: StrategyProfileKind,
    pub plan_fingerprint_hex: String,
    pub indicator_slots: Vec<GpuIndicatorSlot>,
    pub conditions: Vec<GpuConditionPredicate>,
    pub groups: Vec<GpuConditionGroupSlice>,
    pub group_condition_indexes: Vec<u32>,
    pub signals: GpuSignalBindings,
}

fn compile_operand(
    value: &CompiledValueRef,
    indicator_count: usize,
) -> Result<GpuValueOperand, String> {
    match value {
        CompiledValueRef::IndicatorIndex { index } => {
            if (*index as usize) >= indicator_count {
                return Err(format!(
                    "indicator index {} out of range (len={indicator_count})",
                    index
                ));
            }
            Ok(GpuValueOperand {
                source: GpuValueSourceKind::Indicator,
                index: *index,
                constant_bits: 0,
            })
        }
        CompiledValueRef::MaSource => Ok(GpuValueOperand {
            source: GpuValueSourceKind::MaSource,
            index: 0,
            constant_bits: 0,
        }),
        CompiledValueRef::Close => Ok(GpuValueOperand {
            source: GpuValueSourceKind::Close,
            index: 0,
            constant_bits: 0,
        }),
        CompiledValueRef::ConstantBits { bits } => Ok(GpuValueOperand {
            source: GpuValueSourceKind::Constant,
            index: 0,
            constant_bits: *bits,
        }),
    }
}

fn validate_group_binding(
    group_index: Option<u32>,
    group_count: usize,
    label: &str,
) -> Result<Option<u32>, String> {
    match group_index {
        Some(idx) if (idx as usize) >= group_count => Err(format!(
            "signal binding '{label}' references out-of-range group index {idx} (len={group_count})"
        )),
        _ => Ok(group_index),
    }
}

pub fn compile_gpu_predicate_program(
    plan: &CompiledStrategyPlan,
) -> Result<CompiledGpuPredicateProgram, String> {
    if plan.backend != StrategyPlanBackend::GpuUnified {
        return Err("gpu predicate program requires StrategyPlanBackend::GpuUnified".to_string());
    }

    let indicator_count = plan.indicator_plan.len();
    let mut indicator_slots: Vec<GpuIndicatorSlot> = Vec::with_capacity(indicator_count);
    for (i, indicator) in plan.indicator_plan.iter().enumerate() {
        let indicator_index =
            u32::try_from(i).map_err(|_| "too many indicators for u32 indexing".to_string())?;
        indicator_slots.push(GpuIndicatorSlot {
            indicator_index,
            indicator_id: indicator.indicator_id.clone(),
            indicator_kind: indicator.indicator_kind,
            source_indicator_id: indicator.source_indicator_id.clone(),
            output_id: indicator.output_id.clone(),
            route: indicator.route,
        });
    }

    let mut conditions: Vec<GpuConditionPredicate> = Vec::with_capacity(plan.conditions.len());
    for c in &plan.conditions {
        conditions.push(GpuConditionPredicate {
            op: c.op,
            lhs: compile_operand(&c.lhs, indicator_count)?,
            rhs: compile_operand(&c.rhs, indicator_count)?,
        });
    }

    let condition_count = conditions.len();
    let mut groups: Vec<GpuConditionGroupSlice> = Vec::with_capacity(plan.groups.len());
    let mut group_condition_indexes: Vec<u32> = Vec::new();
    for group in &plan.groups {
        let offset = group_condition_indexes.len();
        for idx in &group.condition_indexes {
            if (*idx as usize) >= condition_count {
                return Err(format!(
                    "group condition index {} out of range (len={condition_count})",
                    idx
                ));
            }
            group_condition_indexes.push(*idx);
        }
        let count = group_condition_indexes.len().saturating_sub(offset);
        groups.push(GpuConditionGroupSlice {
            mode: group.mode,
            condition_offset: u32::try_from(offset)
                .map_err(|_| "group condition offset overflow".to_string())?,
            condition_count: u32::try_from(count)
                .map_err(|_| "group condition count overflow".to_string())?,
        });
    }

    let group_count = groups.len();
    let signals = GpuSignalBindings {
        long_entry_group: validate_group_binding(
            plan.signals.long_entry_group,
            group_count,
            "long_entry_group",
        )?,
        long_exit_group: validate_group_binding(
            plan.signals.long_exit_group,
            group_count,
            "long_exit_group",
        )?,
        short_entry_group: validate_group_binding(
            plan.signals.short_entry_group,
            group_count,
            "short_entry_group",
        )?,
        short_exit_group: validate_group_binding(
            plan.signals.short_exit_group,
            group_count,
            "short_exit_group",
        )?,
    };

    Ok(CompiledGpuPredicateProgram {
        schema_version: STRATEGY_GPU_PROGRAM_SCHEMA_VERSION,
        profile: plan.profile,
        plan_fingerprint_hex: plan.ir_fingerprint_hex.clone(),
        indicator_slots,
        conditions,
        groups,
        group_condition_indexes,
        signals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_ir::{build_static_strategy_ir, StaticStrategyIrInput};
    use crate::strategy_plan::{compile_strategy_plan, CompiledValueRef};
    use ta_strategies::double_ma::StrategyConfig;

    fn make_plan(
        profile: StrategyProfileKind,
        backend: StrategyPlanBackend,
    ) -> CompiledStrategyPlan {
        let ir = build_static_strategy_ir(
            profile,
            StaticStrategyIrInput {
                ma_source: "close".to_string(),
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
                halftrend: None,
                macz: None,
                ott: None,
                signal_mode: None,
            },
        )
        .expect("static ir should build");
        compile_strategy_plan(&ir, backend, None).expect("plan should compile")
    }

    #[test]
    fn compile_gpu_program_smoke_from_double_ma_plan() {
        let plan = make_plan(
            StrategyProfileKind::DoubleMa,
            StrategyPlanBackend::GpuUnified,
        );
        let program = compile_gpu_predicate_program(&plan).expect("gpu program should compile");
        assert_eq!(program.schema_version, STRATEGY_GPU_PROGRAM_SCHEMA_VERSION);
        assert_eq!(program.profile, StrategyProfileKind::DoubleMa);
        assert_eq!(program.indicator_slots.len(), plan.indicator_plan.len());
        assert_eq!(program.conditions.len(), plan.conditions.len());
        assert_eq!(program.groups.len(), plan.groups.len());
        let expected_indexes: usize = plan.groups.iter().map(|g| g.condition_indexes.len()).sum();
        assert_eq!(program.group_condition_indexes.len(), expected_indexes);
    }

    #[test]
    fn compile_gpu_program_rejects_cpu_plan_backend() {
        let plan = make_plan(StrategyProfileKind::DoubleMa, StrategyPlanBackend::Cpu);
        let err = compile_gpu_predicate_program(&plan).expect_err("cpu backend should be rejected");
        assert!(err.contains("GpuUnified"));
    }

    #[test]
    fn compile_gpu_program_rejects_out_of_range_condition_index() {
        let mut plan = make_plan(
            StrategyProfileKind::CompositeTrend,
            StrategyPlanBackend::GpuUnified,
        );
        let first = plan.groups.first_mut().expect("group expected");
        first.condition_indexes.push(u32::MAX);
        let err =
            compile_gpu_predicate_program(&plan).expect_err("invalid condition index should fail");
        assert!(err.contains("condition index"));
    }

    #[test]
    fn compile_gpu_program_rejects_out_of_range_indicator_index() {
        let mut plan = make_plan(
            StrategyProfileKind::DoubleMa,
            StrategyPlanBackend::GpuUnified,
        );
        let first = plan.conditions.first_mut().expect("condition expected");
        first.lhs = CompiledValueRef::IndicatorIndex { index: u32::MAX };
        let err =
            compile_gpu_predicate_program(&plan).expect_err("invalid indicator index should fail");
        assert!(err.contains("indicator index"));
    }
}
