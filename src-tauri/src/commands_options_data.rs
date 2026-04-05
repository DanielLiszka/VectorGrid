use crate::settings;
use serde::Deserialize;
use ta_app_core::{
    dataset_manifest_path, load_imported_options_dataset_from_csv, read_options_dataset_manifest,
    scan_options_dataset_manifests, write_options_dataset_manifest,
    ImportedOptionsDatasetCsvBuildRequest, OptionsDatasetManifestV1, OptionsPriceSemantics,
};
use tauri::Manager;

fn default_provider_name() -> String {
    "manual_import".to_string()
}

fn default_provider_source_surface() -> String {
    "minute_aggregate".to_string()
}

fn default_quote_resolution() -> String {
    "1m".to_string()
}

fn default_price_semantics() -> OptionsPriceSemantics {
    OptionsPriceSemantics::MinuteAggregate
}

fn default_calendar_id() -> String {
    "market_hours_utc".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_contract_reference_source() -> String {
    "manual_import".to_string()
}

fn default_underlier_reference_source() -> String {
    "underlying_dataset".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct OptionsDatasetImportRequest {
    pub csv_path: String,
    #[serde(default)]
    pub options_dataset_id: Option<String>,
    #[serde(default)]
    pub root_symbol: Option<String>,
    #[serde(default = "default_provider_name")]
    pub provider_name: String,
    #[serde(default = "default_provider_source_surface")]
    pub provider_source_surface: String,
    #[serde(default)]
    pub provider_entitlement_profile: Option<String>,
    #[serde(default = "default_quote_resolution")]
    pub quote_resolution: String,
    #[serde(default = "default_price_semantics")]
    pub price_semantics: OptionsPriceSemantics,
    #[serde(default = "default_calendar_id")]
    pub calendar_id: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub normalization_version: Option<u32>,
    #[serde(default)]
    pub quote_semantics_version: Option<u32>,
    #[serde(default)]
    pub corporate_action_mapping_version: Option<u32>,
    #[serde(default = "default_contract_reference_source")]
    pub contract_reference_source: String,
    #[serde(default)]
    pub contract_reference_as_of_utc: Option<String>,
    #[serde(default = "default_underlier_reference_source")]
    pub underlier_reference_source: String,
    #[serde(default)]
    pub aggregate_price_field: Option<String>,
}

fn sanitize_id_component(raw: &str) -> String {
    let out: String = raw
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    out.trim_matches('_').to_string()
}

fn sanitize_dataset_id(raw: &str) -> Result<String, String> {
    let sanitized = sanitize_id_component(raw);
    if sanitized.is_empty() {
        return Err("options_dataset_id resolved to an empty value".to_string());
    }
    Ok(sanitized)
}

fn derive_dataset_id(req: &OptionsDatasetImportRequest) -> String {
    if let Some(explicit) = req
        .options_dataset_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return sanitize_id_component(explicit);
    }
    if let Some(root) = req
        .root_symbol
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!(
            "{}_{}",
            sanitize_id_component(root),
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );
    }
    let stem = std::path::Path::new(req.csv_path.trim())
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("options_dataset");
    format!(
        "{}_{}",
        sanitize_id_component(stem),
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    )
}

fn options_datasets_dir(
    app_local_data_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let market_root = settings::resolve_market_data_root_dir(app_local_data_dir)?;
    let dir = market_root.join("options_research").join("datasets");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn dataset_csv_copy_path(dataset_dir: &std::path::Path) -> std::path::PathBuf {
    dataset_dir.join("rows.csv")
}

fn copy_dataset_into_store(
    req: &OptionsDatasetImportRequest,
    app_local_data_dir: &std::path::Path,
) -> Result<OptionsDatasetManifestV1, String> {
    let datasets_dir = options_datasets_dir(app_local_data_dir)?;
    let dataset_id = derive_dataset_id(req);
    if dataset_id.is_empty() {
        return Err("options_dataset_id resolved to an empty value".to_string());
    }
    let dataset_dir = datasets_dir.join(&dataset_id);
    if dataset_dir.exists() {
        return Err(format!(
            "options dataset id '{}' already exists at {}",
            dataset_id,
            dataset_dir.display()
        ));
    }
    let source_csv = std::path::Path::new(req.csv_path.trim());
    if !source_csv.is_file() {
        return Err(format!(
            "options dataset CSV does not exist: {}",
            source_csv.display()
        ));
    }

    let import_result = (|| {
        std::fs::create_dir_all(&dataset_dir).map_err(|e| e.to_string())?;
        let copied_csv = dataset_csv_copy_path(&dataset_dir);
        std::fs::copy(source_csv, &copied_csv)
            .map_err(|e| format!("failed to copy options dataset CSV into store: {e}"))?;

        let aggregate_price_field = req.aggregate_price_field.clone().or_else(|| {
            if matches!(req.price_semantics, OptionsPriceSemantics::MinuteAggregate) {
                Some("close".to_string())
            } else {
                None
            }
        });
        if matches!(req.price_semantics, OptionsPriceSemantics::MinuteAggregate)
            && aggregate_price_field
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| !value.eq_ignore_ascii_case("close"))
                .unwrap_or(false)
        {
            return Err(
                "minute-aggregate imports currently require aggregate_price_field='close'"
                    .to_string(),
            );
        }

        let build_req = ImportedOptionsDatasetCsvBuildRequest {
            options_dataset_id: dataset_id.clone(),
            dataset_path: copied_csv.to_string_lossy().to_string(),
            root_symbol: req.root_symbol.clone(),
            provider_name: req.provider_name.clone(),
            provider_source_surface: req.provider_source_surface.clone(),
            provider_entitlement_profile: req.provider_entitlement_profile.clone(),
            quote_resolution: req.quote_resolution.clone(),
            price_semantics: req.price_semantics,
            calendar_id: req.calendar_id.clone(),
            timezone: req.timezone.clone(),
            normalization_version: req.normalization_version.unwrap_or(1),
            quote_semantics_version: req.quote_semantics_version.unwrap_or(1),
            corporate_action_mapping_version: req.corporate_action_mapping_version.unwrap_or(1),
            contract_reference_source: req.contract_reference_source.clone(),
            contract_reference_as_of_utc: req.contract_reference_as_of_utc.clone(),
            underlier_reference_source: req.underlier_reference_source.clone(),
            aggregate_price_field,
        };
        let dataset = load_imported_options_dataset_from_csv(&build_req)?;
        let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
        write_options_dataset_manifest(&manifest_path, &dataset.manifest)?;
        Ok(dataset.manifest)
    })();

    if import_result.is_err() {
        let _ = std::fs::remove_dir_all(&dataset_dir);
    }
    import_result
}

#[tauri::command]
pub async fn import_options_dataset(
    req: OptionsDatasetImportRequest,
    app: tauri::AppHandle,
) -> Result<OptionsDatasetManifestV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        copy_dataset_into_store(&req, &app_local_data_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_options_datasets(
    app: tauri::AppHandle,
) -> Result<Vec<OptionsDatasetManifestV1>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let datasets_dir = options_datasets_dir(&app_local_data_dir)?;
        scan_options_dataset_manifests(&datasets_dir)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_options_dataset_manifest(
    options_dataset_id: String,
    app: tauri::AppHandle,
) -> Result<OptionsDatasetManifestV1, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let app_local_data_dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let datasets_dir = options_datasets_dir(&app_local_data_dir)?;
        let dataset_id = sanitize_dataset_id(options_dataset_id.trim())?;
        let manifest_path = dataset_manifest_path(&datasets_dir, &dataset_id);
        if !manifest_path.is_file() {
            return Err(format!(
                "options dataset manifest not found for '{}' at {}",
                dataset_id,
                manifest_path.display()
            ));
        }
        read_options_dataset_manifest(&manifest_path)
    })
    .await
    .map_err(|e| e.to_string())?
}
