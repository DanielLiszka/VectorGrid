use crate::composite_cpu::{MaczGateParams, OttGateParams};
use crate::double_ma::HalfTrendRegimeFilter;
use crate::indicator_catalog::{
    find_dynamic_indicator_descriptor, normalize_and_validate_indicator_output_ref,
    normalize_and_validate_indicator_params, IndicatorParamKV,
};
use crate::strategy_ir::{
    validate_strategy_ir, ConditionGroupMode, ConditionOp, IndicatorKind, IndicatorNode,
    StrategyIr, StrategyProfileKind, ValueRef, STRATEGY_IR_SCHEMA_VERSION,
};
use crate::CompositeSignalMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use ta_strategies::double_ma::StrategyConfig;

pub const STRATEGY_PLAN_SCHEMA_VERSION: u16 = 1;

fn default_strategy_plan_schema_version() -> u16 {
    STRATEGY_PLAN_SCHEMA_VERSION
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StrategyPlanBackend {
    Cpu,
    GpuUnified,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorRouteKind {
    Cpu,
    GpuKernel,
    GpuSingle,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct IndicatorComputePlan {
    // Slot id in the strategy graph (for condition references).
    pub indicator_id: String,
    #[serde(default)]
    pub indicator_kind: Option<IndicatorKind>,
    // Underlying VectorTA indicator id for descriptor-driven nodes.
    #[serde(default)]
    pub source_indicator_id: Option<String>,
    #[serde(default)]
    pub output_id: Option<String>,
    #[serde(default)]
    pub params: Vec<IndicatorParamKV>,
    pub route: IndicatorRouteKind,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompiledValueRef {
    IndicatorIndex { index: u32 },
    MaSource,
    Close,
    ConstantBits { bits: u64 },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct CompiledConditionOp {
    pub op: ConditionOp,
    pub lhs: CompiledValueRef,
    pub rhs: CompiledValueRef,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct CompiledConditionGroup {
    pub mode: ConditionGroupMode,
    pub condition_indexes: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct CompiledSignalBindings {
    pub long_entry_group: Option<u32>,
    pub long_exit_group: Option<u32>,
    pub short_entry_group: Option<u32>,
    pub short_exit_group: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompiledStrategyPlan {
    #[serde(default = "default_strategy_plan_schema_version")]
    pub plan_schema_version: u16,
    pub backend: StrategyPlanBackend,
    pub profile: StrategyProfileKind,
    pub ir_schema_version: u16,
    pub ir_fingerprint_hex: String,
    pub indicator_plan: Vec<IndicatorComputePlan>,
    pub conditions: Vec<CompiledConditionOp>,
    pub groups: Vec<CompiledConditionGroup>,
    pub signals: CompiledSignalBindings,
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

fn default_legacy_route(kind: IndicatorKind) -> IndicatorRouteKind {
    match kind {
        IndicatorKind::FastMa | IndicatorKind::SlowMa => IndicatorRouteKind::GpuKernel,
        IndicatorKind::TrendQualityRegime | IndicatorKind::BreakoutRegime => {
            IndicatorRouteKind::GpuKernel
        }
        IndicatorKind::HalfTrendTrend | IndicatorKind::MaczHist | IndicatorKind::OttLine => {
            IndicatorRouteKind::Cpu
        }
    }
}

fn normalized_source_indicator_id(indicator: &IndicatorNode) -> Option<String> {
    indicator
        .indicator_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase())
}

fn default_route_for_indicator(
    backend: StrategyPlanBackend,
    indicator: &IndicatorNode,
) -> Result<IndicatorRouteKind, String> {
    match backend {
        StrategyPlanBackend::Cpu => Ok(IndicatorRouteKind::Cpu),
        StrategyPlanBackend::GpuUnified => {
            if let Some(source_id) = normalized_source_indicator_id(indicator) {
                let descriptor =
                    find_dynamic_indicator_descriptor(source_id.as_str()).ok_or_else(|| {
                        format!(
                            "indicator '{}' references unknown indicator_id '{}'",
                            indicator.id, source_id
                        )
                    })?;
                if descriptor.prefers_gpu_by_speedup {
                    if descriptor.supports_gpu_kernel {
                        return Ok(IndicatorRouteKind::GpuKernel);
                    }
                    if descriptor.supports_cuda_single || descriptor.supports_cuda_sweep {
                        return Ok(IndicatorRouteKind::GpuSingle);
                    }
                }
                return Ok(IndicatorRouteKind::Cpu);
            }

            if let Some(kind) = indicator.kind {
                return Ok(default_legacy_route(kind));
            }

            Err(format!(
                "indicator '{}' missing both legacy kind and source indicator id",
                indicator.id
            ))
        }
    }
}

fn to_compiled_value(
    value: &ValueRef,
    indicator_index: &HashMap<&str, u32>,
) -> Result<CompiledValueRef, String> {
    match value {
        ValueRef::Indicator { id } => {
            let key = id.trim();
            let index = indicator_index
                .get(key)
                .copied()
                .ok_or_else(|| format!("unknown indicator id '{key}'"))?;
            Ok(CompiledValueRef::IndicatorIndex { index })
        }
        ValueRef::MaSource => Ok(CompiledValueRef::MaSource),
        ValueRef::Close => Ok(CompiledValueRef::Close),
        ValueRef::ConstantBits { bits } => Ok(CompiledValueRef::ConstantBits { bits: *bits }),
    }
}

fn fingerprint_ir(ir: &StrategyIr) -> Result<String, String> {
    let serialized = serde_json::to_vec(ir).map_err(|e| format!("serialize strategy_ir: {e}"))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    serialized.hash(&mut hasher);
    let h = hasher.finish();
    Ok(format!("{h:016x}"))
}

pub fn compile_strategy_plan(
    ir: &StrategyIr,
    backend: StrategyPlanBackend,
    route_overrides: Option<&HashMap<String, IndicatorRouteKind>>,
) -> Result<CompiledStrategyPlan, String> {
    validate_strategy_ir(ir)?;
    if ir.schema_version > STRATEGY_IR_SCHEMA_VERSION {
        return Err(format!(
            "strategy_ir schema {} is newer than this compiler supports ({STRATEGY_IR_SCHEMA_VERSION})",
            ir.schema_version
        ));
    }

    let mut indicator_index: HashMap<&str, u32> = HashMap::new();
    let mut indicator_plan: Vec<IndicatorComputePlan> =
        Vec::with_capacity(ir.program.indicators.len());
    for (i, indicator) in ir.program.indicators.iter().enumerate() {
        let idx =
            u32::try_from(i).map_err(|_| "too many indicators for u32 indexing".to_string())?;
        let id = indicator.id.trim();
        indicator_index.insert(id, idx);

        let (source_indicator_id, params, output_id) =
            if let Some(source_id) = normalized_source_indicator_id(indicator) {
                let normalized_params = normalize_and_validate_indicator_params(
                    source_id.as_str(),
                    indicator.params.as_slice(),
                )?;
                let normalized_output = normalize_and_validate_indicator_output_ref(
                    source_id.as_str(),
                    indicator.output_id.as_deref(),
                )?;
                (Some(source_id), normalized_params, Some(normalized_output))
            } else {
                (None, Vec::new(), None)
            };

        let route = route_overrides
            .and_then(|m| m.get(id))
            .copied()
            .unwrap_or(default_route_for_indicator(backend, indicator)?);
        indicator_plan.push(IndicatorComputePlan {
            indicator_id: id.to_string(),
            indicator_kind: indicator.kind,
            source_indicator_id,
            output_id,
            params,
            route,
        });
    }

    let mut condition_index: HashMap<&str, u32> = HashMap::new();
    let mut compiled_conditions: Vec<CompiledConditionOp> =
        Vec::with_capacity(ir.program.conditions.len());
    for (i, condition) in ir.program.conditions.iter().enumerate() {
        let idx =
            u32::try_from(i).map_err(|_| "too many conditions for u32 indexing".to_string())?;
        condition_index.insert(condition.id.trim(), idx);
        compiled_conditions.push(CompiledConditionOp {
            op: condition.op,
            lhs: to_compiled_value(&condition.lhs, &indicator_index)?,
            rhs: to_compiled_value(&condition.rhs, &indicator_index)?,
        });
    }

    let mut group_index: HashMap<&str, u32> = HashMap::new();
    let mut compiled_groups: Vec<CompiledConditionGroup> =
        Vec::with_capacity(ir.program.groups.len());
    for (i, group) in ir.program.groups.iter().enumerate() {
        let idx = u32::try_from(i).map_err(|_| "too many groups for u32 indexing".to_string())?;
        group_index.insert(group.id.trim(), idx);
        let mut condition_indexes: Vec<u32> = Vec::with_capacity(group.condition_ids.len());
        for condition_id in &group.condition_ids {
            let id = condition_id.trim();
            let ci = condition_index.get(id).copied().ok_or_else(|| {
                format!("group '{}' references unknown condition '{}'", group.id, id)
            })?;
            condition_indexes.push(ci);
        }
        compiled_groups.push(CompiledConditionGroup {
            mode: group.mode,
            condition_indexes,
        });
    }

    let map_group = |group_id: &Option<String>| -> Result<Option<u32>, String> {
        let Some(group_id) = group_id else {
            return Ok(None);
        };
        let id = group_id.trim();
        let index = group_index
            .get(id)
            .copied()
            .ok_or_else(|| format!("signal binding references unknown group '{id}'"))?;
        Ok(Some(index))
    };

    let signals = CompiledSignalBindings {
        long_entry_group: map_group(&ir.program.signals.long_entry_group)?,
        long_exit_group: map_group(&ir.program.signals.long_exit_group)?,
        short_entry_group: map_group(&ir.program.signals.short_entry_group)?,
        short_exit_group: map_group(&ir.program.signals.short_exit_group)?,
    };

    Ok(CompiledStrategyPlan {
        plan_schema_version: STRATEGY_PLAN_SCHEMA_VERSION,
        backend,
        profile: ir.profile,
        ir_schema_version: ir.schema_version,
        ir_fingerprint_hex: fingerprint_ir(ir)?,
        indicator_plan,
        conditions: compiled_conditions,
        groups: compiled_groups,
        signals,
        strategy: ir.execution.strategy.clone(),
        halftrend_filter: ir.execution.halftrend_filter.clone(),
        halftrend: ir.execution.halftrend.clone(),
        macz: ir.execution.macz.clone(),
        ott: ir.execution.ott.clone(),
        signal_mode: ir.execution.signal_mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator_catalog::{list_dynamic_indicator_descriptors_cached, IndicatorParamKV};
    use crate::strategy_ir::{
        build_static_strategy_ir, ConditionGroup, ConditionGroupMode, ConditionNode,
        ExecutionPolicy, IndicatorNode, MovingAverageSpec, SignalBindings, SignalProgramGraph,
        StaticStrategyIrInput, StrategyIr, ValueRef,
    };
    use serde_json::json;
    use ta_strategies::double_ma::StrategyConfig;

    fn make_ir(profile: StrategyProfileKind) -> StrategyIr {
        build_static_strategy_ir(
            profile,
            StaticStrategyIrInput {
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
            },
        )
        .expect("builder should produce valid IR")
    }

    fn make_dynamic_indicator_ir(
        source_indicator_id: &str,
        output_id: Option<&str>,
        params: Vec<IndicatorParamKV>,
    ) -> StrategyIr {
        StrategyIr {
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
                    id: "dyn".to_string(),
                    kind: None,
                    indicator_id: Some(source_indicator_id.to_string()),
                    params,
                    output_id: output_id.map(|v| v.to_string()),
                }],
                conditions: vec![ConditionNode {
                    id: "dyn_gt_zero".to_string(),
                    op: ConditionOp::Gt,
                    lhs: ValueRef::Indicator {
                        id: "dyn".to_string(),
                    },
                    rhs: ValueRef::constant(0.0),
                }],
                groups: vec![ConditionGroup {
                    id: "entry".to_string(),
                    mode: ConditionGroupMode::All,
                    condition_ids: vec!["dyn_gt_zero".to_string()],
                }],
                signals: SignalBindings {
                    long_entry_group: Some("entry".to_string()),
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
        }
    }

    #[test]
    fn compile_cpu_plan_assigns_cpu_routes() {
        let ir = make_ir(StrategyProfileKind::DoubleMa);
        let plan = compile_strategy_plan(&ir, StrategyPlanBackend::Cpu, None)
            .expect("plan should compile");
        assert_eq!(plan.backend, StrategyPlanBackend::Cpu);
        assert!(plan
            .indicator_plan
            .iter()
            .all(|p| p.route == IndicatorRouteKind::Cpu));
    }

    #[test]
    fn compile_plan_accepts_schema_v1_legacy_ir_payload() {
        let payload = json!({
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
            serde_json::from_value(payload).expect("schema v1 payload should deserialize");
        let plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)
            .expect("schema v1 payload should compile");
        assert_eq!(plan.ir_schema_version, 1);
        assert_eq!(plan.indicator_plan.len(), 2);
        assert_eq!(
            plan.indicator_plan[0].indicator_kind,
            Some(IndicatorKind::FastMa)
        );
        assert_eq!(
            plan.indicator_plan[1].indicator_kind,
            Some(IndicatorKind::SlowMa)
        );
    }

    #[test]
    fn compile_gpu_plan_uses_kernel_for_ma_indicators() {
        let ir = make_ir(StrategyProfileKind::CompositeTrend);
        let plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)
            .expect("plan should compile");
        let mut saw_fast = false;
        let mut saw_slow = false;
        for indicator in &plan.indicator_plan {
            if indicator.indicator_kind == Some(IndicatorKind::FastMa) {
                saw_fast = true;
                assert_eq!(indicator.route, IndicatorRouteKind::GpuKernel);
            }
            if indicator.indicator_kind == Some(IndicatorKind::SlowMa) {
                saw_slow = true;
                assert_eq!(indicator.route, IndicatorRouteKind::GpuKernel);
            }
        }
        assert!(saw_fast);
        assert!(saw_slow);
    }

    #[test]
    fn compile_plan_respects_route_overrides() {
        let ir = make_ir(StrategyProfileKind::CompositeCpu);
        let mut overrides: HashMap<String, IndicatorRouteKind> = HashMap::new();
        overrides.insert("fast_ma".to_string(), IndicatorRouteKind::Cpu);
        let plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, Some(&overrides))
            .expect("plan should compile");
        let fast = plan
            .indicator_plan
            .iter()
            .find(|p| p.indicator_id == "fast_ma")
            .expect("fast_ma must be present");
        assert_eq!(fast.route, IndicatorRouteKind::Cpu);
    }

    #[test]
    fn compile_plan_fails_for_unknown_signal_group_reference() {
        let mut ir = make_ir(StrategyProfileKind::DoubleMa);
        ir.program.signals.long_entry_group = Some("missing".to_string());
        let err = compile_strategy_plan(&ir, StrategyPlanBackend::Cpu, None).unwrap_err();
        assert!(err.contains("unknown group"));
    }

    #[test]
    fn compile_gpu_plan_routes_dynamic_indicator_to_gpu_when_policy_prefers_gpu() {
        let descriptor = list_dynamic_indicator_descriptors_cached()
            .iter()
            .find(|d| {
                d.prefers_gpu_by_speedup
                    && !d.outputs.is_empty()
                    && (d.supports_gpu_kernel || d.supports_cuda_single || d.supports_cuda_sweep)
                    && d.params.iter().all(|p| !p.required || p.default.is_some())
            })
            .expect("expected at least one GPU-preferred dynamic indicator");
        let params = descriptor
            .params
            .iter()
            .filter_map(|p| {
                p.default.clone().map(|value| IndicatorParamKV {
                    key: p.key.clone(),
                    value,
                })
            })
            .collect::<Vec<_>>();
        let ir = make_dynamic_indicator_ir(
            descriptor.id.as_str(),
            Some(descriptor.outputs[0].id.as_str()),
            params,
        );
        let plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)
            .expect("dynamic plan should compile");
        let slot = plan
            .indicator_plan
            .first()
            .expect("indicator slot expected");
        assert_eq!(
            slot.source_indicator_id.as_deref(),
            Some(descriptor.id.as_str())
        );
        assert!(matches!(
            slot.route,
            IndicatorRouteKind::GpuKernel | IndicatorRouteKind::GpuSingle
        ));
    }

    #[test]
    fn compile_gpu_plan_routes_dynamic_indicator_to_cpu_when_policy_prefers_cpu() {
        let descriptor = list_dynamic_indicator_descriptors_cached()
            .iter()
            .find(|d| {
                !d.prefers_gpu_by_speedup
                    && !d.outputs.is_empty()
                    && d.params.iter().all(|p| !p.required || p.default.is_some())
            })
            .expect("expected at least one CPU-preferred dynamic indicator");
        let params = descriptor
            .params
            .iter()
            .filter_map(|p| {
                p.default.clone().map(|value| IndicatorParamKV {
                    key: p.key.clone(),
                    value,
                })
            })
            .collect::<Vec<_>>();
        let ir = make_dynamic_indicator_ir(
            descriptor.id.as_str(),
            Some(descriptor.outputs[0].id.as_str()),
            params,
        );
        let plan = compile_strategy_plan(&ir, StrategyPlanBackend::GpuUnified, None)
            .expect("dynamic plan should compile");
        let slot = plan
            .indicator_plan
            .first()
            .expect("indicator slot expected");
        assert_eq!(slot.route, IndicatorRouteKind::Cpu);
    }
}
