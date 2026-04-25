use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorParamKind {
    Int,
    Float,
    Bool,
    EnumString,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum IndicatorParamValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    EnumString(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorValueType {
    F64,
    F32,
    I32,
    Bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndicatorInputKind {
    Slice,
    Candles,
    Ohlc,
    Ohlcv,
    HighLow,
    CloseVolume,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct IndicatorParamDescriptor {
    pub key: String,
    pub label: String,
    pub kind: IndicatorParamKind,
    pub required: bool,
    pub default: Option<IndicatorParamValue>,
    pub min: Option<f64>,
    pub max: Option<f64>,

    pub step: Option<f64>,
    pub enum_values: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IndicatorOutputDescriptor {
    pub id: String,
    pub label: String,
    pub value_type: IndicatorValueType,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct IndicatorParamKV {
    pub key: String,
    pub value: IndicatorParamValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct IndicatorDescriptor {
    pub id: String,
    pub label: String,
    pub category: String,
    pub dynamic_strategy_eligible: bool,
    pub input_kind: IndicatorInputKind,
    pub requires_candles: bool,
    pub supports_cpu_single: bool,
    pub supports_cpu_batch: bool,
    pub supports_cuda_single: bool,
    pub supports_cuda_sweep: bool,
    pub supports_cuda_vram: bool,

    pub supports_gpu_kernel: bool,

    pub prefers_gpu_by_speedup: bool,
    pub output_count: usize,
    pub outputs: Vec<IndicatorOutputDescriptor>,
    pub params: Vec<IndicatorParamDescriptor>,
    pub notes: Option<String>,
}

fn map_param_kind(kind: vector_ta::indicators::registry::IndicatorParamKind) -> IndicatorParamKind {
    match kind {
        vector_ta::indicators::registry::IndicatorParamKind::Int => IndicatorParamKind::Int,
        vector_ta::indicators::registry::IndicatorParamKind::Float => IndicatorParamKind::Float,
        vector_ta::indicators::registry::IndicatorParamKind::Bool => IndicatorParamKind::Bool,
        vector_ta::indicators::registry::IndicatorParamKind::EnumString => {
            IndicatorParamKind::EnumString
        }
    }
}

fn map_param_value(
    value: vector_ta::indicators::registry::ParamValueStatic,
) -> IndicatorParamValue {
    match value {
        vector_ta::indicators::registry::ParamValueStatic::Int(v) => IndicatorParamValue::Int(v),
        vector_ta::indicators::registry::ParamValueStatic::Float(v) => {
            IndicatorParamValue::Float(v)
        }
        vector_ta::indicators::registry::ParamValueStatic::Bool(v) => IndicatorParamValue::Bool(v),
        vector_ta::indicators::registry::ParamValueStatic::EnumString(v) => {
            IndicatorParamValue::EnumString(v.to_string())
        }
    }
}

fn map_output_value_type(
    value_type: vector_ta::indicators::registry::IndicatorValueType,
) -> IndicatorValueType {
    match value_type {
        vector_ta::indicators::registry::IndicatorValueType::F64 => IndicatorValueType::F64,
        vector_ta::indicators::registry::IndicatorValueType::F32 => IndicatorValueType::F32,
        vector_ta::indicators::registry::IndicatorValueType::I32 => IndicatorValueType::I32,
        vector_ta::indicators::registry::IndicatorValueType::Bool => IndicatorValueType::Bool,
    }
}

fn map_input_kind(kind: vector_ta::indicators::registry::IndicatorInputKind) -> IndicatorInputKind {
    match kind {
        vector_ta::indicators::registry::IndicatorInputKind::Slice => IndicatorInputKind::Slice,
        vector_ta::indicators::registry::IndicatorInputKind::Candles => IndicatorInputKind::Candles,
        vector_ta::indicators::registry::IndicatorInputKind::Ohlc => IndicatorInputKind::Ohlc,
        vector_ta::indicators::registry::IndicatorInputKind::Ohlcv => IndicatorInputKind::Ohlcv,
        vector_ta::indicators::registry::IndicatorInputKind::HighLow => IndicatorInputKind::HighLow,
        vector_ta::indicators::registry::IndicatorInputKind::CloseVolume => {
            IndicatorInputKind::CloseVolume
        }
    }
}

fn requires_candles(input_kind: IndicatorInputKind) -> bool {
    !matches!(input_kind, IndicatorInputKind::Slice)
}

fn normalize_dynamic_indicator_descriptor(
    info: &vector_ta::indicators::registry::IndicatorInfo,
) -> IndicatorDescriptor {
    let id = info.id.to_ascii_lowercase();
    let input_kind = map_input_kind(info.input_kind);
    let params = info
        .params
        .iter()
        .map(|p| IndicatorParamDescriptor {
            key: p.key.to_string(),
            label: p.label.to_string(),
            kind: map_param_kind(p.kind),
            required: p.required,
            default: p.default.map(map_param_value),
            min: p.min,
            max: p.max,
            step: p.step,
            enum_values: p.enum_values.iter().map(|v| v.to_string()).collect(),
            notes: p.notes.map(|v| v.to_string()),
        })
        .collect::<Vec<_>>();
    let outputs = info
        .outputs
        .iter()
        .map(|o| IndicatorOutputDescriptor {
            id: o.id.to_string(),
            label: o.label.to_string(),
            value_type: map_output_value_type(o.value_type),
        })
        .collect::<Vec<_>>();

    IndicatorDescriptor {
        id: id.clone(),
        label: info.label.to_string(),
        category: info.category.to_string(),
        dynamic_strategy_eligible: info.dynamic_strategy_eligible,
        input_kind,
        requires_candles: requires_candles(input_kind),
        supports_cpu_single: info.capabilities.supports_cpu_single,
        supports_cpu_batch: info.capabilities.supports_cpu_batch,
        supports_cuda_single: info.capabilities.supports_cuda_single,
        supports_cuda_sweep: info.capabilities.supports_cuda_batch,
        supports_cuda_vram: info.capabilities.supports_cuda_vram,
        supports_gpu_kernel: crate::ma_policy::supports_gpu_kernel_ma(&id),
        prefers_gpu_by_speedup: crate::ma_policy::prefers_gpu_by_speedup_table(&id),
        output_count: outputs.len(),
        outputs,
        params,
        notes: info.notes.map(|v| v.to_string()),
    }
}

fn normalize_param_key(key: &str) -> String {
    key.trim().to_ascii_lowercase()
}

fn check_numeric_range(
    indicator_id: &str,
    param: &IndicatorParamDescriptor,
    value: f64,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!(
            "indicator '{indicator_id}' param '{}' must be finite",
            param.key
        ));
    }
    if let Some(min) = param.min {
        if value < min {
            return Err(format!(
                "indicator '{indicator_id}' param '{}' is below minimum {min}",
                param.key
            ));
        }
    }
    if let Some(max) = param.max {
        if value > max {
            return Err(format!(
                "indicator '{indicator_id}' param '{}' is above maximum {max}",
                param.key
            ));
        }
    }
    Ok(())
}

fn normalize_param_value_for_schema(
    indicator_id: &str,
    param: &IndicatorParamDescriptor,
    value: &IndicatorParamValue,
) -> Result<IndicatorParamValue, String> {
    match (&param.kind, value) {
        (IndicatorParamKind::Int, IndicatorParamValue::Int(v)) => {
            check_numeric_range(indicator_id, param, *v as f64)?;
            Ok(IndicatorParamValue::Int(*v))
        }
        (IndicatorParamKind::Int, IndicatorParamValue::Float(v)) => {
            if !v.is_finite() {
                return Err(format!(
                    "indicator '{indicator_id}' param '{}' must be finite",
                    param.key
                ));
            }
            if v.fract() != 0.0 {
                return Err(format!(
                    "indicator '{indicator_id}' param '{}' expects integer value",
                    param.key
                ));
            }
            if *v < i64::MIN as f64 || *v > i64::MAX as f64 {
                return Err(format!(
                    "indicator '{indicator_id}' param '{}' integer value is out of range",
                    param.key
                ));
            }
            let iv = *v as i64;
            check_numeric_range(indicator_id, param, iv as f64)?;
            Ok(IndicatorParamValue::Int(iv))
        }
        (IndicatorParamKind::Float, IndicatorParamValue::Float(v)) => {
            check_numeric_range(indicator_id, param, *v)?;
            Ok(IndicatorParamValue::Float(*v))
        }
        (IndicatorParamKind::Float, IndicatorParamValue::Int(v)) => {
            let fv = *v as f64;
            check_numeric_range(indicator_id, param, fv)?;
            Ok(IndicatorParamValue::Float(fv))
        }
        (IndicatorParamKind::Bool, IndicatorParamValue::Bool(v)) => {
            Ok(IndicatorParamValue::Bool(*v))
        }
        (IndicatorParamKind::EnumString, IndicatorParamValue::EnumString(v)) => {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                return Err(format!(
                    "indicator '{indicator_id}' param '{}' enum value must not be empty",
                    param.key
                ));
            }
            if param.enum_values.is_empty() {
                return Ok(IndicatorParamValue::EnumString(trimmed.to_string()));
            }
            if let Some(canonical) = param
                .enum_values
                .iter()
                .find(|candidate| candidate.eq_ignore_ascii_case(trimmed))
            {
                return Ok(IndicatorParamValue::EnumString(canonical.clone()));
            }
            Err(format!(
                "indicator '{indicator_id}' param '{}' has invalid enum value '{}'",
                param.key, trimmed
            ))
        }
        (expected, actual) => Err(format!(
            "indicator '{indicator_id}' param '{}' expects {:?}, got {:?}",
            param.key, expected, actual
        )),
    }
}

pub fn list_dynamic_indicator_descriptors() -> Vec<IndicatorDescriptor> {
    vector_ta::indicators::registry::list_indicators()
        .iter()
        .filter(|info| info.dynamic_strategy_eligible)
        .map(normalize_dynamic_indicator_descriptor)
        .collect()
}

static DYNAMIC_INDICATOR_DESCRIPTORS_CACHE: OnceLock<Vec<IndicatorDescriptor>> = OnceLock::new();

pub fn list_dynamic_indicator_descriptors_cached() -> &'static [IndicatorDescriptor] {
    DYNAMIC_INDICATOR_DESCRIPTORS_CACHE
        .get_or_init(list_dynamic_indicator_descriptors)
        .as_slice()
}

pub fn find_dynamic_indicator_descriptor(
    indicator_id: &str,
) -> Option<&'static IndicatorDescriptor> {
    let id = indicator_id.trim();
    if id.is_empty() {
        return None;
    }
    list_dynamic_indicator_descriptors_cached()
        .iter()
        .find(|d| d.id.eq_ignore_ascii_case(id))
}

pub fn normalize_and_validate_indicator_params(
    indicator_id: &str,
    params: &[IndicatorParamKV],
) -> Result<Vec<IndicatorParamKV>, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("unsupported_indicator: '{indicator_id}' is not in dynamic indicator registry")
    })?;

    #[derive(Clone)]
    struct ProvidedParam {
        key: String,
        value: IndicatorParamValue,
    }

    let mut provided: BTreeMap<String, ProvidedParam> = BTreeMap::new();
    for kv in params {
        let raw_key = kv.key.trim();
        if raw_key.is_empty() {
            return Err(format!(
                "indicator '{}' contains an empty parameter key",
                descriptor.id
            ));
        }
        let normalized_key = normalize_param_key(raw_key);
        if provided.contains_key(&normalized_key) {
            return Err(format!(
                "indicator '{}' parameter '{}' is duplicated",
                descriptor.id, raw_key
            ));
        }
        provided.insert(
            normalized_key,
            ProvidedParam {
                key: raw_key.to_string(),
                value: kv.value.clone(),
            },
        );
    }

    let mut known_keys: BTreeSet<String> = BTreeSet::new();
    let mut normalized: Vec<IndicatorParamKV> = Vec::new();
    for schema in &descriptor.params {
        let schema_key = normalize_param_key(schema.key.as_str());
        known_keys.insert(schema_key.clone());
        if let Some(value) = provided.remove(&schema_key) {
            normalized.push(IndicatorParamKV {
                key: schema.key.clone(),
                value: normalize_param_value_for_schema(
                    descriptor.id.as_str(),
                    schema,
                    &value.value,
                )?,
            });
            continue;
        }

        if let Some(default) = schema.default.clone() {
            normalized.push(IndicatorParamKV {
                key: schema.key.clone(),
                value: normalize_param_value_for_schema(descriptor.id.as_str(), schema, &default)?,
            });
            continue;
        }

        if schema.required {
            return Err(format!(
                "indicator '{}' missing required parameter '{}'",
                descriptor.id, schema.key
            ));
        }
    }

    if let Some((_, unknown)) = provided.into_iter().next() {
        return Err(format!(
            "indicator '{}' has unknown parameter '{}'",
            descriptor.id, unknown.key
        ));
    }

    normalized.sort_by(|a, b| {
        let ia = descriptor
            .params
            .iter()
            .position(|p| p.key.eq_ignore_ascii_case(a.key.as_str()))
            .unwrap_or(usize::MAX);
        let ib = descriptor
            .params
            .iter()
            .position(|p| p.key.eq_ignore_ascii_case(b.key.as_str()))
            .unwrap_or(usize::MAX);
        ia.cmp(&ib)
    });

    if known_keys.len() != descriptor.params.len() {
        return Err(format!(
            "indicator '{}' has duplicate parameter keys in descriptor schema",
            descriptor.id
        ));
    }

    Ok(normalized)
}

pub fn normalize_and_validate_indicator_output_ref(
    indicator_id: &str,
    output_id: Option<&str>,
) -> Result<String, String> {
    let descriptor = find_dynamic_indicator_descriptor(indicator_id).ok_or_else(|| {
        format!("unsupported_indicator: '{indicator_id}' is not in dynamic indicator registry")
    })?;

    if descriptor.outputs.is_empty() {
        return Err(format!(
            "indicator '{}' has no registered outputs",
            descriptor.id
        ));
    }

    if descriptor.outputs.len() == 1 {
        let only = descriptor.outputs[0].id.as_str();
        if let Some(requested) = output_id {
            let req = requested.trim();
            if !req.is_empty() && !req.eq_ignore_ascii_case(only) {
                return Err(format!(
                    "invalid_output_ref: indicator '{}' only supports output '{}'",
                    descriptor.id, only
                ));
            }
        }
        return Ok(only.to_string());
    }

    let requested = output_id
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            format!(
                "invalid_output_ref: indicator '{}' requires output_id for multi-output selection",
                descriptor.id
            )
        })?;

    if let Some(out) = descriptor
        .outputs
        .iter()
        .find(|o| o.id.eq_ignore_ascii_case(requested.as_str()))
    {
        return Ok(out.id.clone());
    }

    Err(format!(
        "invalid_output_ref: indicator '{}' does not expose output '{}'",
        descriptor.id, requested
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn dynamic_indicator_descriptors_match_registry_dynamic_count() {
        let descriptors = list_dynamic_indicator_descriptors();
        let registry_count = vector_ta::indicators::registry::list_indicators()
            .iter()
            .filter(|info| info.dynamic_strategy_eligible)
            .count();
        assert_eq!(descriptors.len(), registry_count);
    }

    #[test]
    fn dynamic_indicator_descriptors_have_nonempty_id_and_label() {
        let descriptors = list_dynamic_indicator_descriptors();
        assert!(!descriptors.is_empty());
        for d in descriptors {
            assert!(!d.id.trim().is_empty());
            assert!(!d.label.trim().is_empty());
        }
    }

    #[test]
    fn descriptors_expose_output_slots_with_matching_count() {
        let descriptors = list_dynamic_indicator_descriptors();
        for d in descriptors {
            let maybe_info = vector_ta::indicators::registry::list_indicators()
                .iter()
                .find(|row| row.id.eq_ignore_ascii_case(d.id.as_str()));
            let info = maybe_info.expect("descriptor id should exist in registry");
            assert_eq!(d.output_count, info.outputs.len());
            assert_eq!(d.outputs.len(), info.outputs.len());
        }
    }

    #[test]
    fn dynamic_indicator_descriptors_have_unique_normalized_ids() {
        let descriptors = list_dynamic_indicator_descriptors();
        let mut seen = BTreeSet::new();
        for d in descriptors {
            assert_eq!(d.id, d.id.trim().to_ascii_lowercase());
            assert!(
                seen.insert(d.id.clone()),
                "duplicate normalized descriptor id '{}'",
                d.id
            );
        }
    }

    #[test]
    fn descriptor_outputs_have_unique_nonempty_ids_within_each_descriptor() {
        let descriptors = list_dynamic_indicator_descriptors();
        for d in descriptors {
            let mut seen = BTreeSet::new();
            for output in &d.outputs {
                let normalized = output.id.trim().to_ascii_lowercase();
                assert!(
                    !normalized.is_empty(),
                    "descriptor '{}' has empty output id",
                    d.id
                );
                assert!(
                    seen.insert(normalized.clone()),
                    "descriptor '{}' has duplicate normalized output id '{}'",
                    d.id,
                    normalized
                );
            }
        }
    }

    #[test]
    fn descriptor_defaults_validate_against_their_own_schema() {
        let descriptors = list_dynamic_indicator_descriptors();
        for d in descriptors {
            for param in &d.params {
                if let Some(default) = &param.default {
                    let normalized =
                        normalize_param_value_for_schema(d.id.as_str(), param, default)
                            .unwrap_or_else(|err| {
                                panic!(
                                    "descriptor '{}' default for param '{}' should validate: {err}",
                                    d.id, param.key
                                )
                            });
                    assert_eq!(&normalized, default);
                }
            }
        }
    }

    #[test]
    fn numeric_step_is_metadata_only_and_not_enforced() {
        let param = IndicatorParamDescriptor {
            key: "period".to_string(),
            label: "Period".to_string(),
            kind: IndicatorParamKind::Float,
            required: true,
            default: None,
            min: Some(0.0),
            max: Some(10.0),
            step: Some(0.5),
            enum_values: Vec::new(),
            notes: None,
        };
        let normalized = normalize_param_value_for_schema(
            "test_indicator",
            &param,
            &IndicatorParamValue::Float(1.3),
        )
        .expect("off-step numeric values should still normalize today");
        assert_eq!(normalized, IndicatorParamValue::Float(1.3));
    }

    #[test]
    fn cached_lookup_finds_known_indicator() {
        let found = find_dynamic_indicator_descriptor("sma");
        assert!(found.is_some());
        let descriptor = found.expect("sma descriptor should exist");
        assert_eq!(descriptor.id, "sma");
    }

    #[test]
    fn normalize_params_accepts_case_insensitive_keys_and_int_to_float_widening() {
        let input = vec![IndicatorParamKV {
            key: "PERIOD".to_string(),
            value: IndicatorParamValue::Int(22),
        }];
        let normalized = normalize_and_validate_indicator_params("sma", input.as_slice())
            .expect("sma params should normalize");
        let period = normalized
            .iter()
            .find(|p| p.key.eq_ignore_ascii_case("period"))
            .expect("normalized params should include period");
        assert_eq!(period.value, IndicatorParamValue::Int(22));
    }

    #[test]
    fn normalize_params_rejects_unknown_parameter_keys() {
        let input = vec![IndicatorParamKV {
            key: "definitely_unknown".to_string(),
            value: IndicatorParamValue::Int(1),
        }];
        let err = normalize_and_validate_indicator_params("sma", input.as_slice())
            .expect_err("unknown key should fail");
        assert!(err.contains("unknown parameter"));
    }

    #[test]
    fn normalize_params_accepts_integer_as_float_for_int_schema() {
        let input = vec![IndicatorParamKV {
            key: "period".to_string(),
            value: IndicatorParamValue::Float(14.0),
        }];
        let normalized = normalize_and_validate_indicator_params("sma", input.as_slice())
            .expect("integral float should coerce to int");
        let period = normalized
            .iter()
            .find(|p| p.key.eq_ignore_ascii_case("period"))
            .expect("normalized params should include period");
        assert_eq!(period.value, IndicatorParamValue::Int(14));
    }

    #[test]
    fn output_ref_validation_accepts_single_output_without_explicit_ref() {
        let out = normalize_and_validate_indicator_output_ref("sma", None)
            .expect("single-output indicators should allow empty output_id");
        assert_eq!(out, "value");
    }

    #[test]
    fn output_ref_validation_requires_explicit_ref_for_multi_output() {
        let err = normalize_and_validate_indicator_output_ref("macd", None)
            .expect_err("multi-output indicator should require output_id");
        assert!(err.contains("requires output_id"));
    }

    #[test]
    fn output_ref_validation_is_case_insensitive() {
        let out = normalize_and_validate_indicator_output_ref("macd", Some("HiSt"))
            .expect("case-insensitive output selection should succeed");
        assert_eq!(out, "hist");
    }
}
