use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tauri::Manager;

use super::{reveal_path_blocking, sanitize_filename_component};

const DATASET_META_SCHEMA_VERSION: u64 = 2;
const CONFIG_FINGERPRINT_SCHEMA_VERSION: u64 = 1;
const RESAMPLE_RULE_SESSION_RELATIVE_OHLCV: &str = "session_relative_ohlcv";
const RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP: &str = "day_first_timestamp";

#[derive(Debug, Clone, Serialize)]
pub struct SaveRunArtifactResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveBatchSummaryResponse {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub kind: String,
    pub strategy_kind: String,
    pub config_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_fingerprint_version: Option<u32>,
    pub has_validation: bool,
    pub report_xlsx_path: String,
    pub csv_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub timeframe: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_timeframe: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub derived_timeframe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resample_applied: Option<bool>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resample_rule: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resample_anchor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mtime_utc_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_content_fingerprint: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaArtifactSummary {
    pub ts: String,
    pub path: String,
    pub bytes: u64,
    pub strategy_kind: String,
    pub timeframe: String,
    pub required_tag: String,
    pub groups: usize,
    pub artifacts_used: usize,
    pub holdout_datasets: usize,
    pub report_xlsx_path: String,
    pub tags: Vec<String>,
}

pub(super) fn runs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let dir = base.join("runs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(super) fn suggest_meta_artifact_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorbt_meta", ts, "json")
}

pub(super) fn suggest_batch_summary_path(
    ts: &str,
    app: &tauri::AppHandle,
) -> Result<PathBuf, String> {
    suggest_ts_file_path(runs_dir(app)?, "vectorbt_batch", ts, "json")
}

fn suggest_ts_file_path(
    dir: PathBuf,
    prefix: &str,
    ts: &str,
    ext: &str,
) -> Result<PathBuf, String> {
    let safe_ts = sanitize_filename_component(ts);
    let base_name = format!("{prefix}_{safe_ts}");
    let mut path = dir.join(format!("{base_name}.{ext}"));
    if !path.exists() {
        return Ok(path);
    }
    for i in 2..=999u32 {
        let p = dir.join(format!("{base_name}_{i}.{ext}"));
        if !p.exists() {
            path = p;
            break;
        }
    }
    Ok(path)
}

fn run_artifact_path(artifact: &Value, app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = runs_dir(app)?;

    let ts = artifact
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let safe_ts = sanitize_filename_component(ts);
    Ok(dir.join(format!("vectorbt_run_{safe_ts}.json")))
}

fn ensure_path_is_within_dir(path: &Path, dir: &Path) -> Result<(), String> {
    let dir = std::fs::canonicalize(dir).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if !path.starts_with(&dir) {
        return Err("path is outside app runs dir".to_string());
    }
    Ok(())
}

fn non_empty_string(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_dataset_meta(ds: &mut serde_json::Map<String, Value>, csv_path: &str) {
    if !csv_path.trim().is_empty() {
        ds.entry("csv_path".to_string())
            .or_insert_with(|| Value::String(csv_path.to_string()));
    }

    ds.entry("schema_version".to_string())
        .or_insert_with(|| Value::Number(DATASET_META_SCHEMA_VERSION.into()));

    let timeframe = non_empty_string(ds.get("timeframe"));
    let mut base_timeframe = non_empty_string(ds.get("base_timeframe"));
    let mut derived_timeframe =
        non_empty_string(ds.get("derived_timeframe")).or_else(|| timeframe.clone());

    if timeframe.is_none() {
        if let Some(tf) = &derived_timeframe {
            ds.insert("timeframe".to_string(), Value::String(tf.clone()));
        }
    }
    if derived_timeframe.is_none() {
        if let Some(tf) = &timeframe {
            derived_timeframe = Some(tf.clone());
            ds.insert("derived_timeframe".to_string(), Value::String(tf.clone()));
        }
    } else if ds.get("derived_timeframe").is_none() {
        if let Some(tf) = &derived_timeframe {
            ds.insert("derived_timeframe".to_string(), Value::String(tf.clone()));
        }
    }

    if base_timeframe.is_none() {
        if let Some(tf) = &derived_timeframe {
            base_timeframe = Some(tf.clone());
            ds.insert("base_timeframe".to_string(), Value::String(tf.clone()));
        }
    } else if ds.get("base_timeframe").is_none() {
        if let Some(tf) = &base_timeframe {
            ds.insert("base_timeframe".to_string(), Value::String(tf.clone()));
        }
    }

    let resample_applied = match (&base_timeframe, &derived_timeframe) {
        (Some(base), Some(derived)) => base != derived,
        _ => false,
    };
    if ds
        .get("resample_applied")
        .and_then(|v| v.as_bool())
        .is_none()
    {
        ds.insert(
            "resample_applied".to_string(),
            Value::Bool(resample_applied),
        );
    }

    if derived_timeframe.is_some() {
        if non_empty_string(ds.get("resample_rule")).is_none() {
            ds.insert(
                "resample_rule".to_string(),
                Value::String(RESAMPLE_RULE_SESSION_RELATIVE_OHLCV.to_string()),
            );
        }
        if non_empty_string(ds.get("resample_anchor")).is_none() {
            ds.insert(
                "resample_anchor".to_string(),
                Value::String(RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP.to_string()),
            );
        }
    }
}

pub(super) fn save_run_artifact_blocking(
    mut artifact: Value,
    app: tauri::AppHandle,
) -> Result<SaveRunArtifactResponse, String> {
    let path = run_artifact_path(&artifact, &app)?;

    // Best-effort enrichment for reproducibility and batch "skip completed" correctness.
    if let Value::Object(root) = &mut artifact {
        // Inject minimal environment metadata if missing.
        {
            let env_v = root
                .entry("env".to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if !env_v.is_object() {
                *env_v = Value::Object(serde_json::Map::new());
            }
            if let Some(env) = env_v.as_object_mut() {
                let pkg = app.package_info();
                env.entry("app_name".to_string())
                    .or_insert_with(|| Value::String(pkg.name.clone()));
                env.entry("app_version".to_string())
                    .or_insert_with(|| Value::String(pkg.version.to_string()));
                env.entry("os".to_string())
                    .or_insert_with(|| Value::String(std::env::consts::OS.to_string()));
                env.entry("arch".to_string())
                    .or_insert_with(|| Value::String(std::env::consts::ARCH.to_string()));
            }
        }

        if root
            .get("config_fingerprint_version")
            .and_then(|v| v.as_u64())
            .is_none()
        {
            let cfg = root
                .get("config_fingerprint")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let cfg_ver = if cfg.starts_with("cfg1-") {
                1u64
            } else {
                CONFIG_FINGERPRINT_SCHEMA_VERSION
            };
            root.insert(
                "config_fingerprint_version".to_string(),
                Value::Number(cfg_ver.into()),
            );
        }

        // Ensure dataset identity is present (size + mtime) so artifacts can be matched even if files change.
        let csv_path = root
            .get("data")
            .and_then(|v| v.get("csv_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if !csv_path.is_empty() {
            let ds_v = root
                .entry("dataset_meta".to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if !ds_v.is_object() {
                *ds_v = Value::Object(serde_json::Map::new());
            }
            if let Some(ds) = ds_v.as_object_mut() {
                normalize_dataset_meta(ds, &csv_path);

                let has_size = ds
                    .get("source_size_bytes")
                    .and_then(|v| v.as_u64())
                    .is_some();
                let has_mtime = ds
                    .get("source_mtime_utc_ms")
                    .and_then(|v| v.as_i64())
                    .is_some();
                let has_fp = ds
                    .get("source_content_fingerprint")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
                if !has_size || !has_mtime {
                    if let Ok(id) = crate::data_cache::identity_for_source(Path::new(&csv_path)) {
                        if !has_size {
                            ds.insert(
                                "source_size_bytes".to_string(),
                                Value::Number(serde_json::Number::from(id.size)),
                            );
                        }
                        if !has_mtime {
                            ds.insert(
                                "source_mtime_utc_ms".to_string(),
                                Value::Number(serde_json::Number::from(id.mtime_utc_ms)),
                            );
                        }
                    }
                }
                if !has_fp {
                    if let Ok(fp) =
                        crate::data_cache::content_fingerprint_for_source(Path::new(&csv_path))
                    {
                        ds.insert("source_content_fingerprint".to_string(), Value::String(fp));
                    }
                }
            }
        }
    }

    let bytes = serde_json::to_vec_pretty(&artifact).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;

    // Retention policy: keep newest N untagged runs, but never delete tagged runs.
    prune_old_run_artifacts_blocking(&app, 200);
    Ok(SaveRunArtifactResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

pub(super) fn save_batch_summary_blocking(
    summary: Value,
    app: tauri::AppHandle,
) -> Result<SaveBatchSummaryResponse, String> {
    let ts = summary
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let path = suggest_batch_summary_path(ts, &app)?;
    let bytes = serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(SaveBatchSummaryResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

fn prune_old_artifacts_by_prefix(app: &tauri::AppHandle, keep_newest: usize, prefix: &str) {
    let Ok(dir) = runs_dir(app) else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with(prefix) {
            continue;
        }
        paths.push(path);
    }

    // Newest first (ISO timestamps sort lexicographically).
    paths.sort_by(|a, b| {
        let aa = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let bb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        bb.cmp(aa)
    });

    if paths.len() <= keep_newest {
        return;
    }

    for path in paths.into_iter().skip(keep_newest) {
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let tags = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map_or(&[][..], |xs| xs.as_slice());
        let tagged = tags
            .iter()
            .any(|t| t.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false));
        if tagged {
            continue;
        }
        let _ = std::fs::remove_file(&path);
    }
}

fn prune_old_run_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_run_");
}

pub(super) fn prune_old_meta_artifacts_blocking(app: &tauri::AppHandle, keep_newest: usize) {
    prune_old_artifacts_by_prefix(app, keep_newest, "vectorbt_meta_");
}

pub(super) fn open_runs_dir_blocking(app: tauri::AppHandle) -> Result<String, String> {
    let dir = runs_dir(&app)?;
    reveal_path_blocking(dir.to_string_lossy().to_string())?;
    Ok(dir.to_string_lossy().to_string())
}

fn canonical_json(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
        Value::Array(xs) => {
            let mut out = String::from("[");
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(x));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();

            let mut out = String::from("{");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                let v = map.get(*k).unwrap_or(&Value::Null);
                out.push_str(&canonical_json(v));
            }
            out.push('}');
            out
        }
    }
}

fn fnv1a64_hex_utf16(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    let prime: u64 = 0x100000001b3;
    for u in s.encode_utf16() {
        h ^= u as u64;
        h = h.wrapping_mul(prime);
    }
    format!("{h:016x}")
}

fn compute_config_fingerprint_from_artifact(artifact: &Value) -> String {
    let mut run_req = artifact.pointer("/run/req").cloned().unwrap_or(Value::Null);
    if let Value::Object(map) = &mut run_req {
        map.remove("data_id");
        map.remove("dataId");
    }

    let mut val_req = artifact
        .pointer("/validation/req")
        .cloned()
        .unwrap_or(Value::Null);
    if let Value::Object(map) = &mut val_req {
        map.remove("data_id");
        map.remove("dataId");
        map.remove("candidates");

        if let Some(cross) = map.get_mut("cross_ticker").and_then(|v| v.as_object_mut()) {
            if let Some(ds) = cross.get_mut("datasets").and_then(|v| v.as_array_mut()) {
                let mut out: Vec<Value> = Vec::with_capacity(ds.len());
                for d in ds.iter() {
                    let p = d
                        .get("csv_path")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .trim();
                    if p.is_empty() {
                        continue;
                    }
                    out.push(serde_json::json!({ "csv_path": p }));
                }
                out.sort_by(|a, b| {
                    let aa = a.get("csv_path").and_then(|x| x.as_str()).unwrap_or("");
                    let bb = b.get("csv_path").and_then(|x| x.as_str()).unwrap_or("");
                    aa.cmp(bb)
                });
                *ds = out;
            }
        }
    }

    let payload = serde_json::json!({
        "version": 1,
        "run_req": run_req,
        "validation_req": val_req,
    });
    let canon = canonical_json(&payload);
    let hex = fnv1a64_hex_utf16(&canon);
    format!("cfg1-fnv1a64-{hex}")
}

pub(super) fn list_run_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<RunArtifactSummary>, String> {
    let dir = runs_dir(&app)?;
    let mut out: Vec<RunArtifactSummary> = Vec::new();

    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with("vectorbt_meta_") {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let bytes = meta.len();

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let ts = v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let kind = v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if kind != "vectorbt_run_artifact" {
            continue;
        }
        let strategy_kind = v
            .pointer("/run/req/strategy_kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let config_fingerprint = v
            .get("config_fingerprint")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| compute_config_fingerprint_from_artifact(&v));
        let config_fingerprint_version = v
            .get("config_fingerprint_version")
            .and_then(|x| x.as_u64())
            .map(|x| x as u32)
            .or_else(|| {
                if config_fingerprint.starts_with("cfg1-") {
                    Some(1)
                } else {
                    None
                }
            });
        let has_validation = v.pointer("/validation/res").is_some();
        let report_xlsx_path = v
            .get("report_xlsx_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let csv_path = v
            .pointer("/data/csv_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let timeframe = v
            .pointer("/dataset_meta/timeframe")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let base_timeframe = v
            .pointer("/dataset_meta/base_timeframe")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let derived_timeframe = v
            .pointer("/dataset_meta/derived_timeframe")
            .and_then(|x| x.as_str())
            .or_else(|| {
                v.pointer("/dataset_meta/timeframe")
                    .and_then(|x| x.as_str())
            })
            .unwrap_or("")
            .to_string();
        let resample_applied = v
            .pointer("/dataset_meta/resample_applied")
            .and_then(|x| x.as_bool());
        let resample_rule = v
            .pointer("/dataset_meta/resample_rule")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let resample_anchor = v
            .pointer("/dataset_meta/resample_anchor")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let source_size_bytes = v
            .pointer("/dataset_meta/source_size_bytes")
            .and_then(|x| x.as_u64());
        let source_mtime_utc_ms = v
            .pointer("/dataset_meta/source_mtime_utc_ms")
            .and_then(|x| x.as_i64());
        let source_content_fingerprint = v
            .pointer("/dataset_meta/source_content_fingerprint")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let tags: Vec<String> = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map(|xs| {
                xs.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        out.push(RunArtifactSummary {
            ts,
            path: path.to_string_lossy().to_string(),
            bytes,
            kind,
            strategy_kind,
            config_fingerprint,
            config_fingerprint_version,
            has_validation,
            report_xlsx_path,
            csv_path,
            timeframe,
            base_timeframe,
            derived_timeframe,
            resample_applied,
            resample_rule,
            resample_anchor,
            source_size_bytes,
            source_mtime_utc_ms,
            source_content_fingerprint,
            tags,
        });
    }

    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

pub(super) fn list_meta_artifacts_blocking(
    app: tauri::AppHandle,
) -> Result<Vec<MetaArtifactSummary>, String> {
    let dir = runs_dir(&app)?;
    let mut out: Vec<MetaArtifactSummary> = Vec::new();

    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("vectorbt_meta_") {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let bytes = meta.len();

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let kind = v
            .get("kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if kind != "vectorbt_meta_selection" {
            continue;
        }

        let ts = v
            .get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let strategy_kind = v
            .pointer("/req/strategy_kind")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let timeframe = v
            .pointer("/req/timeframe")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let required_tag = v
            .pointer("/req/required_tag")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let groups = v
            .pointer("/res/groups")
            .and_then(|x| x.as_array())
            .map(|xs| xs.len())
            .unwrap_or(0);
        let artifacts_used = v
            .pointer("/res/artifacts_used")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        let holdout_datasets = v
            .pointer("/res/holdout/datasets")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        let report_xlsx_path = v
            .get("report_xlsx_path")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let tags: Vec<String> = v
            .get("tags")
            .and_then(|x| x.as_array())
            .map(|xs| {
                xs.iter()
                    .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        out.push(MetaArtifactSummary {
            ts,
            path: path.to_string_lossy().to_string(),
            bytes,
            strategy_kind,
            timeframe,
            required_tag,
            groups,
            artifacts_used,
            holdout_datasets,
            report_xlsx_path,
            tags,
        });
    }

    out.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok(out)
}

pub(super) fn load_run_artifact_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<Value, String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

pub(super) fn delete_run_artifact_blocking(
    path: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

pub(super) fn update_artifact_tags_blocking(
    path: String,
    tags: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let dir = runs_dir(&app)?;
    let path = PathBuf::from(path);
    ensure_path_is_within_dir(&path, &dir)?;

    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;

    let kind = v.get("kind").and_then(|x| x.as_str()).unwrap_or("");
    if kind != "vectorbt_run_artifact" && kind != "vectorbt_meta_selection" {
        return Err(format!("unsupported artifact kind: {kind}"));
    }

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for t in tags {
        let tt = t.trim();
        if tt.is_empty() {
            continue;
        }
        let key = tt.to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        out.push(tt.to_string());
    }

    let tags_v = serde_json::to_value(out).map_err(|e| e.to_string())?;
    if let Value::Object(obj) = &mut v {
        obj.insert("tags".to_string(), tags_v);
    }

    let bytes = serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{canonical_json, compute_config_fingerprint_from_artifact, normalize_dataset_meta};
    use serde_json::json;

    #[test]
    fn canonical_json_is_order_invariant_for_objects() {
        let a = json!({
            "b": 2,
            "a": 1,
            "nested": { "z": 9, "m": 3 }
        });
        let b = json!({
            "nested": { "m": 3, "z": 9 },
            "a": 1,
            "b": 2
        });
        assert_eq!(canonical_json(&a), canonical_json(&b));
    }

    #[test]
    fn config_fingerprint_ignores_data_ids_candidates_and_cross_dataset_order() {
        let base = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "data_id": "D1",
                    "dataId": "legacy_d1"
                }
            },
            "validation": {
                "req": {
                    "data_id": "V1",
                    "dataId": "legacy_v1",
                    "candidates": [1, 2, 3],
                    "cross_ticker": {
                        "datasets": [
                            { "csv_path": "b.csv", "label": "B" },
                            { "csv_path": "a.csv", "label": "A" }
                        ]
                    }
                }
            }
        });

        let variant = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "data_id": "D2",
                    "dataId": "legacy_d2"
                }
            },
            "validation": {
                "req": {
                    "data_id": "V2",
                    "dataId": "legacy_v2",
                    "candidates": [99, 100],
                    "cross_ticker": {
                        "datasets": [
                            { "csv_path": "a.csv", "label": "A2" },
                            { "csv_path": "b.csv", "label": "B2" }
                        ]
                    }
                }
            }
        });

        let fa = compute_config_fingerprint_from_artifact(&base);
        let fb = compute_config_fingerprint_from_artifact(&variant);
        assert_eq!(fa, fb);
    }

    #[test]
    fn dataset_meta_normalization_backfills_resample_identity() {
        let mut ds = serde_json::Map::new();
        ds.insert("timeframe".to_string(), json!("1h"));
        normalize_dataset_meta(&mut ds, "x.csv");

        assert_eq!(ds.get("csv_path").and_then(|v| v.as_str()), Some("x.csv"));
        assert_eq!(ds.get("schema_version").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(
            ds.get("base_timeframe").and_then(|v| v.as_str()),
            Some("1h")
        );
        assert_eq!(
            ds.get("derived_timeframe").and_then(|v| v.as_str()),
            Some("1h")
        );
        assert_eq!(
            ds.get("resample_applied").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            ds.get("resample_rule").and_then(|v| v.as_str()),
            Some("session_relative_ohlcv")
        );
        assert_eq!(
            ds.get("resample_anchor").and_then(|v| v.as_str()),
            Some("day_first_timestamp")
        );
    }

    #[test]
    fn dataset_meta_normalization_marks_resampled_when_base_and_derived_differ() {
        let mut ds = serde_json::Map::new();
        ds.insert("base_timeframe".to_string(), json!("1m"));
        ds.insert("derived_timeframe".to_string(), json!("1h"));
        normalize_dataset_meta(&mut ds, "");

        assert_eq!(ds.get("timeframe").and_then(|v| v.as_str()), Some("1h"));
        assert_eq!(
            ds.get("resample_applied").and_then(|v| v.as_bool()),
            Some(true)
        );
    }
}
