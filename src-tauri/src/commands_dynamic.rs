use serde::Serialize;
use ta_app_core::{
    assess_strategy_ir_schema_compatibility, preview_dynamic_double_ma_strategy_ir,
    DynamicDoubleMaStrategyIrPreview, StrategyIr, StrategyPlanBackend,
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

#[tauri::command]
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
    let pretty_json =
        serde_json::to_string_pretty(&ir).map_err(|e| format!("serialize strategy_ir json: {e}"))?;

    Ok(DynamicDoubleMaStrategyIrJsonPreviewResponse {
        schema_compatibility: format!("{schema_compatibility:?}"),
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
pub fn encode_dynamic_strategy_constant_bits(value: f64) -> Result<DynamicStrategyConstantBitsResponse, String> {
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
    use ta_app_core::{build_static_strategy_ir, StaticStrategyIrInput, StrategyProfileKind};

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
        assert_eq!(preview.preview.requested_backend, StrategyPlanBackend::GpuUnified);
        assert!(!preview.pretty_json.is_empty());
    }

    #[test]
    fn encode_dynamic_strategy_constant_bits_returns_stringified_u64() {
        let encoded =
            encode_dynamic_strategy_constant_bits(-1.25).expect("encoding should succeed");
        assert_eq!(encoded.bits, (-1.25f64).to_bits().to_string());
    }
}
