use crate::strategy_ir::{ConditionGroupMode, ConditionOp, IndicatorKind, StrategyProfileKind};
use crate::strategy_plan::{
    CompiledStrategyPlan, CompiledValueRef, IndicatorRouteKind, StrategyPlanBackend,
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const STRATEGY_GPU_PROGRAM_SCHEMA_VERSION: u16 = 1;
pub const STRATEGY_GPU_EXECUTABLE_ABI_VERSION: u32 = 1;

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
    #[serde(default)]
    pub compiled_plan_fingerprint_hex: String,
    #[serde(default)]
    pub executable_fingerprint_hex: String,
    pub indicator_slots: Vec<GpuIndicatorSlot>,
    pub conditions: Vec<GpuConditionPredicate>,
    pub groups: Vec<GpuConditionGroupSlice>,
    pub group_condition_indexes: Vec<u32>,
    pub signals: GpuSignalBindings,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuValueOperandAbi {
    pub source: u32,
    pub index: u32,
    pub constant_bits: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuConditionAbi {
    pub op: u32,
    pub lhs: GpuValueOperandAbi,
    pub rhs: GpuValueOperandAbi,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuConditionGroupAbi {
    pub mode: u32,
    pub condition_offset: u32,
    pub condition_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuSignalBindingsAbi {
    pub long_entry_group: i32,
    pub long_exit_group: i32,
    pub short_entry_group: i32,
    pub short_exit_group: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct GpuIndicatorSlotAbi {
    pub indicator_index: u32,
    pub route: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CompiledGpuPredicateProgramAbi {
    pub abi_version: u32,
    pub plan_fingerprint_hex: String,
    #[serde(default)]
    pub compiled_plan_fingerprint_hex: String,
    #[serde(default)]
    pub executable_fingerprint_hex: String,
    pub indicator_slots: Vec<GpuIndicatorSlotAbi>,
    pub conditions: Vec<GpuConditionAbi>,
    pub groups: Vec<GpuConditionGroupAbi>,
    pub group_condition_indexes: Vec<u32>,
    pub signals: GpuSignalBindingsAbi,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
unsafe impl cust::memory::DeviceCopy for GpuValueOperandAbi {}
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
unsafe impl cust::memory::DeviceCopy for GpuConditionAbi {}
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
unsafe impl cust::memory::DeviceCopy for GpuConditionGroupAbi {}
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
unsafe impl cust::memory::DeviceCopy for GpuSignalBindingsAbi {}
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
unsafe impl cust::memory::DeviceCopy for GpuIndicatorSlotAbi {}

fn encode_value_source_kind(kind: GpuValueSourceKind) -> u32 {
    match kind {
        GpuValueSourceKind::Indicator => 0,
        GpuValueSourceKind::MaSource => 1,
        GpuValueSourceKind::Close => 2,
        GpuValueSourceKind::Constant => 3,
    }
}

fn encode_condition_op(op: ConditionOp) -> u32 {
    match op {
        ConditionOp::Gt => 0,
        ConditionOp::Ge => 1,
        ConditionOp::Lt => 2,
        ConditionOp::Le => 3,
        ConditionOp::CrossUp => 4,
        ConditionOp::CrossDown => 5,
    }
}

fn encode_group_mode(mode: ConditionGroupMode) -> u32 {
    match mode {
        ConditionGroupMode::All => 0,
        ConditionGroupMode::Any => 1,
    }
}

fn encode_indicator_route(route: IndicatorRouteKind) -> u32 {
    match route {
        IndicatorRouteKind::Cpu => 0,
        IndicatorRouteKind::GpuKernel => 1,
        IndicatorRouteKind::GpuSingle => 2,
    }
}

fn encode_optional_group_binding(value: Option<u32>) -> i32 {
    value.map(|v| v as i32).unwrap_or(-1)
}

fn fingerprint_serialized<T: Serialize>(value: &T, label: &str) -> Result<String, String> {
    let serialized = serde_json::to_vec(value).map_err(|e| format!("serialize {label}: {e}"))?;
    let mut hasher = DefaultHasher::new();
    serialized.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

pub(crate) fn fingerprint_compiled_strategy_plan_for_gpu_program(
    plan: &CompiledStrategyPlan,
) -> Result<String, String> {
    fingerprint_serialized(plan, "compiled strategy plan")
}

fn fingerprint_gpu_predicate_program_executable(
    program: &CompiledGpuPredicateProgram,
) -> Result<String, String> {
    #[derive(Serialize)]
    struct ExecutableFingerprintInput<'a> {
        abi_version: u32,
        schema_version: u16,
        profile: StrategyProfileKind,
        compiled_plan_fingerprint_hex: &'a str,
        indicator_slots: &'a [GpuIndicatorSlot],
        conditions: &'a [GpuConditionPredicate],
        groups: &'a [GpuConditionGroupSlice],
        group_condition_indexes: &'a [u32],
        signals: &'a GpuSignalBindings,
    }

    fingerprint_serialized(
        &ExecutableFingerprintInput {
            abi_version: STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
            schema_version: program.schema_version,
            profile: program.profile,
            compiled_plan_fingerprint_hex: program.compiled_plan_fingerprint_hex.as_str(),
            indicator_slots: program.indicator_slots.as_slice(),
            conditions: program.conditions.as_slice(),
            groups: program.groups.as_slice(),
            group_condition_indexes: program.group_condition_indexes.as_slice(),
            signals: &program.signals,
        },
        "compiled gpu predicate executable",
    )
}

pub fn compile_gpu_predicate_program_abi(
    program: &CompiledGpuPredicateProgram,
) -> CompiledGpuPredicateProgramAbi {
    let indicator_slots = program
        .indicator_slots
        .iter()
        .map(|slot| GpuIndicatorSlotAbi {
            indicator_index: slot.indicator_index,
            route: encode_indicator_route(slot.route),
        })
        .collect();
    let conditions = program
        .conditions
        .iter()
        .map(|condition| GpuConditionAbi {
            op: encode_condition_op(condition.op),
            lhs: GpuValueOperandAbi {
                source: encode_value_source_kind(condition.lhs.source),
                index: condition.lhs.index,
                constant_bits: condition.lhs.constant_bits,
            },
            rhs: GpuValueOperandAbi {
                source: encode_value_source_kind(condition.rhs.source),
                index: condition.rhs.index,
                constant_bits: condition.rhs.constant_bits,
            },
        })
        .collect();
    let groups = program
        .groups
        .iter()
        .map(|group| GpuConditionGroupAbi {
            mode: encode_group_mode(group.mode),
            condition_offset: group.condition_offset,
            condition_count: group.condition_count,
        })
        .collect();
    let signals = GpuSignalBindingsAbi {
        long_entry_group: encode_optional_group_binding(program.signals.long_entry_group),
        long_exit_group: encode_optional_group_binding(program.signals.long_exit_group),
        short_entry_group: encode_optional_group_binding(program.signals.short_entry_group),
        short_exit_group: encode_optional_group_binding(program.signals.short_exit_group),
    };

    CompiledGpuPredicateProgramAbi {
        abi_version: STRATEGY_GPU_EXECUTABLE_ABI_VERSION,
        plan_fingerprint_hex: program.plan_fingerprint_hex.clone(),
        compiled_plan_fingerprint_hex: program.compiled_plan_fingerprint_hex.clone(),
        executable_fingerprint_hex: program.executable_fingerprint_hex.clone(),
        indicator_slots,
        conditions,
        groups,
        group_condition_indexes: program.group_condition_indexes.clone(),
        signals,
    }
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
    let compiled_plan_fingerprint_hex = fingerprint_compiled_strategy_plan_for_gpu_program(plan)?;
    let mut program = CompiledGpuPredicateProgram {
        schema_version: STRATEGY_GPU_PROGRAM_SCHEMA_VERSION,
        profile: plan.profile,
        plan_fingerprint_hex: plan.ir_fingerprint_hex.clone(),
        compiled_plan_fingerprint_hex,
        executable_fingerprint_hex: String::new(),
        indicator_slots,
        conditions,
        groups,
        group_condition_indexes,
        signals,
    };
    program.executable_fingerprint_hex = fingerprint_gpu_predicate_program_executable(&program)?;
    Ok(program)
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
        assert!(!program.compiled_plan_fingerprint_hex.is_empty());
        assert!(!program.executable_fingerprint_hex.is_empty());
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

    #[test]
    fn compile_gpu_program_abi_preserves_program_shape() {
        let plan = make_plan(
            StrategyProfileKind::DoubleMa,
            StrategyPlanBackend::GpuUnified,
        );
        let program = compile_gpu_predicate_program(&plan).expect("gpu program should compile");
        let abi = compile_gpu_predicate_program_abi(&program);
        assert_eq!(abi.abi_version, STRATEGY_GPU_EXECUTABLE_ABI_VERSION);
        assert_eq!(abi.plan_fingerprint_hex, program.plan_fingerprint_hex);
        assert_eq!(
            abi.compiled_plan_fingerprint_hex,
            program.compiled_plan_fingerprint_hex
        );
        assert_eq!(
            abi.executable_fingerprint_hex,
            program.executable_fingerprint_hex
        );
        assert_eq!(abi.indicator_slots.len(), program.indicator_slots.len());
        assert_eq!(abi.conditions.len(), program.conditions.len());
        assert_eq!(abi.groups.len(), program.groups.len());
        assert_eq!(
            abi.group_condition_indexes.len(),
            program.group_condition_indexes.len()
        );
    }

    #[test]
    fn compile_gpu_program_executable_fingerprint_tracks_compiled_route_shape() {
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
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
        let base_plan =
            compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None).expect("base plan");
        let first_indicator = base_plan
            .indicator_plan
            .first()
            .expect("indicator expected")
            .indicator_id
            .clone();
        let mut overrides = std::collections::HashMap::new();
        overrides.insert(first_indicator, IndicatorRouteKind::Cpu);
        let overridden_plan = compile_strategy_plan(
            &ir,
            StrategyPlanBackend::GpuUnified,
            Some(&overrides),
        )
        .expect("overridden plan");
        assert_eq!(base_plan.ir_fingerprint_hex, overridden_plan.ir_fingerprint_hex);

        let base_program =
            compile_gpu_predicate_program(&base_plan).expect("base gpu program should compile");
        let overridden_program = compile_gpu_predicate_program(&overridden_plan)
            .expect("overridden gpu program should compile");

        assert_ne!(
            base_program.compiled_plan_fingerprint_hex,
            overridden_program.compiled_plan_fingerprint_hex
        );
        assert_ne!(
            base_program.executable_fingerprint_hex,
            overridden_program.executable_fingerprint_hex
        );
    }
}
