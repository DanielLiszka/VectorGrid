use serde::Serialize;
use ta_app_core::{
    assess_strategy_ir_schema_compatibility, preview_dynamic_double_ma_strategy_ir,
    DynamicDoubleMaStrategyIrPreview, StrategyIr, StrategyIrSchemaCompatibility,
    StrategyPlanBackend,
};
use ta_optimizer::ObjectiveKind;

#[derive(Clone, Debug, Serialize)]
pub struct DynamicDoubleMaStrategyIrJsonPreviewResponse {
    pub schema_compatibility: String,
    pub ir: StrategyIr,
    pub pretty_json: String,
    pub preview: DynamicDoubleMaStrategyIrPreview,
}

#[derive(Clone, Debug, Serialize)]
pub struct DynamicStrategyConstantBitsResponse {
    pub bits: String,
}

fn schema_compatibility_label(value: StrategyIrSchemaCompatibility) -> &'static str {
    match value {
        StrategyIrSchemaCompatibility::Compatible => "Compatible",
        StrategyIrSchemaCompatibility::UnsupportedLegacy { .. } => "UnsupportedLegacy",
        StrategyIrSchemaCompatibility::UnsupportedFuture { .. } => "UnsupportedFuture",
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn preview_dynamic_double_ma_strategy_ir_json(
    ir_json: String,
    requested_backend: Option<StrategyPlanBackend>,
    objective: Option<ObjectiveKind>,
    top_k: Option<usize>,
    include_all: Option<bool>,
) -> Result<DynamicDoubleMaStrategyIrJsonPreviewResponse, String> {
    let ir: StrategyIr =
        serde_json::from_str(&ir_json).map_err(|e| format!("parse strategy_ir json: {e}"))?;
    let schema_compatibility = assess_strategy_ir_schema_compatibility(ir.schema_version);
    let preview = preview_dynamic_double_ma_strategy_ir(
        &ir,
        requested_backend.unwrap_or(StrategyPlanBackend::GpuUnified),
        objective.unwrap_or(ObjectiveKind::Sharpe),
        top_k.unwrap_or(50),
        include_all.unwrap_or(false),
    )?;
    let pretty_json = serde_json::to_string_pretty(&ir)
        .map_err(|e| format!("serialize strategy_ir json: {e}"))?;

    Ok(DynamicDoubleMaStrategyIrJsonPreviewResponse {
        schema_compatibility: schema_compatibility_label(schema_compatibility).to_string(),
        ir,
        pretty_json,
        preview,
    })
}

#[tauri::command]
pub fn format_dynamic_strategy_ir_json(ir: StrategyIr) -> Result<String, String> {
    serde_json::to_string_pretty(&ir).map_err(|e| format!("serialize strategy_ir json: {e}"))
}

#[tauri::command]
pub fn encode_dynamic_strategy_constant_bits(
    value: f64,
) -> Result<DynamicStrategyConstantBitsResponse, String> {
    if !value.is_finite() {
        return Err("dynamic strategy constant must be finite".to_string());
    }
    Ok(DynamicStrategyConstantBitsResponse {
        bits: value.to_bits().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ta_app_core::{
        build_static_strategy_ir, StaticStrategyIrInput, StrategyProfileKind, ValueRef,
        STRATEGY_IR_SCHEMA_VERSION,
    };
    use ta_strategies::double_ma::StrategyConfig;

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_json_parses_and_previews() {
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                ta_strategies::double_ma::StrategyConfig::default(),
                None,
            ),
        )
        .expect("ir should build");
        let ir_json = serde_json::to_string(&ir).expect("json");

        let preview = preview_dynamic_double_ma_strategy_ir_json(
            ir_json,
            Some(StrategyPlanBackend::GpuUnified),
            Some(ObjectiveKind::Sharpe),
            Some(16),
            Some(false),
        )
        .expect("preview should succeed");

        assert_eq!(preview.ir.profile, StrategyProfileKind::DoubleMa);
        assert_eq!(
            preview.preview.requested_backend,
            StrategyPlanBackend::GpuUnified
        );
        assert!(!preview.pretty_json.is_empty());
    }

    #[test]
    fn preview_dynamic_double_ma_strategy_ir_json_accepts_constant_bits_ir() {
        let value = -0.0f64;
        let bits = value.to_bits().to_string();
        let ir_json = serde_json::to_string(&json!({
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
        }))
        .expect("constant-bits json should serialize");

        let preview = preview_dynamic_double_ma_strategy_ir_json(
            ir_json,
            Some(StrategyPlanBackend::GpuUnified),
            Some(ObjectiveKind::Sharpe),
            Some(16),
            Some(false),
        )
        .expect("preview should accept decimal-string constant bits");

        assert_eq!(
            preview.ir.program.conditions[0].rhs,
            ValueRef::ConstantBits {
                bits: value.to_bits()
            }
        );
        assert!(preview.pretty_json.contains("\"constant_bits\""));
    }

    #[test]
    fn format_dynamic_strategy_ir_json_returns_pretty_json() {
        let ir = build_static_strategy_ir(
            StrategyProfileKind::DoubleMa,
            StaticStrategyIrInput::from_double_ma(
                "close".to_string(),
                "sma".to_string(),
                "sma".to_string(),
                None,
                None,
                ta_strategies::double_ma::StrategyConfig::default(),
                None,
            ),
        )
        .expect("ir should build");

        let pretty_json =
            format_dynamic_strategy_ir_json(ir).expect("formatting strategy_ir should succeed");

        assert!(pretty_json.contains('\n'));
        assert!(pretty_json.contains("\"profile\": \"double_ma\""));
    }

    #[test]
    fn encode_dynamic_strategy_constant_bits_returns_stringified_u64() {
        let encoded =
            encode_dynamic_strategy_constant_bits(-1.25).expect("encoding should succeed");
        assert_eq!(encoded.bits, (-1.25f64).to_bits().to_string());
    }

    #[test]
    fn encode_dynamic_strategy_constant_bits_preserves_negative_zero_bits() {
        let encoded = encode_dynamic_strategy_constant_bits(-0.0).expect("encoding should succeed");
        assert_eq!(encoded.bits, (-0.0f64).to_bits().to_string());
    }

    #[test]
    fn schema_compatibility_label_uses_stable_wire_literals() {
        assert_eq!(
            schema_compatibility_label(StrategyIrSchemaCompatibility::Compatible),
            "Compatible"
        );
        assert_eq!(
            schema_compatibility_label(StrategyIrSchemaCompatibility::UnsupportedLegacy {
                found: 0,
                min_supported: 1,
            }),
            "UnsupportedLegacy"
        );
        assert_eq!(
            schema_compatibility_label(StrategyIrSchemaCompatibility::UnsupportedFuture {
                found: 3,
                max_supported: 2,
            }),
            "UnsupportedFuture"
        );
    }
}
