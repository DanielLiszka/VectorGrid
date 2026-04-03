use super::commands_meta::{
    run_meta_selection_blocking, run_meta_stage4_experimental_blocking, RunMetaSelectionResponse,
    RunMetaStage5Response,
};
use super::commands_optimize::{
    normalize_product_composite_cpu_optimization_mode,
    normalize_product_double_ma_optimization_mode, TauriProgressSink,
};
use super::run_artifacts;
use super::{
    BackendOptimizationResult, CompositeCpuRequest, CompositeMoRequest, CompositeMoResult,
    CompositeValidationRequest, DoubleMaRequest, DoubleMaValidationRequest, MetaSelectionRequest,
    MetaStage5Request,
};
use crate::data_cache::{self, LoadOutcome};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use ta_app_core::{
    progress::{DoubleMaCampaignVramPrefetchFeedback, DoubleMaProgressPayload, ProgressSink},
    Backend, BackendUsed, CrossTickerConfig, CscvPboConfig, PerturbConfig,
    ReoptWalkForwardConfigOptional, SpaConfigOptional, TradeBootstrapConfig,
    ValidationArtifactContext, WalkForwardConfigOptional,
};
use tauri::{Emitter, Manager};
use vector_ta::utilities::data_loader::Candles;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCampaignPreloadPolicy {
    Disabled,
    HostOnly,
    HostPlusDoublemaVram,
}

impl Default for ResearchCampaignPreloadPolicy {
    fn default() -> Self {
        Self::HostOnly
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCampaignExecutionMode {
    Full,
    OptimizeOnly,
    ValidateOnly,
}

impl Default for ResearchCampaignExecutionMode {
    fn default() -> Self {
        Self::Full
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ResearchCampaignDatasetInput {
    pub csv_path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub timeframe: Option<String>,
    #[serde(default)]
    pub source_size_bytes: Option<u64>,
    #[serde(default)]
    pub source_mtime_utc_ms: Option<i64>,
    #[serde(default)]
    pub source_content_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ResearchCampaignValidationConfig {
    #[serde(default = "default_validation_max_candidates")]
    pub max_candidates: usize,
    #[serde(default = "default_validation_seed")]
    pub seed: u64,
    #[serde(default)]
    pub walk_forward: Option<WalkForwardConfigOptional>,
    #[serde(default)]
    pub reopt_walk_forward: Option<ReoptWalkForwardConfigOptional>,
    #[serde(default)]
    pub spa: Option<SpaConfigOptional>,
    #[serde(default)]
    pub perturb: Option<PerturbConfig>,
    #[serde(default)]
    pub trade_bootstrap: Option<TradeBootstrapConfig>,
    #[serde(default)]
    pub cross_ticker: Option<CrossTickerConfig>,
    #[serde(default)]
    pub cscv_pbo: Option<CscvPboConfig>,
    #[serde(default)]
    pub min_psr: Option<f64>,
    #[serde(default)]
    pub min_dsr: Option<f64>,
    #[serde(default)]
    pub min_cscv_oos_rank_frac: Option<f64>,
    #[serde(default)]
    pub min_cscv_oos_metric: Option<f64>,
    #[serde(default)]
    pub max_pbo: Option<f64>,
}

fn default_validation_max_candidates() -> usize {
    32
}

fn default_validation_seed() -> u64 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResearchCampaignRequest {
    pub strategy_kind: String,
    pub datasets: Vec<ResearchCampaignDatasetInput>,
    #[serde(default)]
    pub timeframe_override: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub holdout_pct: u32,
    #[serde(default)]
    pub holdout_tag: Option<String>,
    #[serde(default)]
    pub skip_existing: bool,
    #[serde(default)]
    pub auto_meta: bool,
    #[serde(default)]
    pub auto_stage5: bool,
    #[serde(default)]
    pub execution_mode: ResearchCampaignExecutionMode,
    pub optimization_template: Value,
    #[serde(default)]
    pub validation_config: ResearchCampaignValidationConfig,
    #[serde(default)]
    pub meta_request: Option<MetaSelectionRequest>,
    #[serde(default)]
    pub stage5_request: Option<MetaStage5Request>,
    #[serde(default)]
    pub preload_policy: ResearchCampaignPreloadPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResearchCampaignCounts {
    pub total: usize,
    pub optimized: usize,
    pub validated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResearchCampaignItemSummary {
    pub csv_path: String,
    pub label: String,
    pub timeframe: String,
    pub is_holdout: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_xlsx_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResearchCampaignSummary {
    pub version: u32,
    pub kind: String,
    pub ts: String,
    pub state: String,
    pub strategy_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdout_tag: Option<String>,
    pub holdout_pct: u32,
    pub request: Value,
    pub counts: ResearchCampaignCounts,
    pub items: Vec<ResearchCampaignItemSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_json_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage5_json_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub runtime_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResearchCampaignResponse {
    pub ts: String,
    pub summary_json_path: String,
    pub summary_json_bytes: usize,
    pub summary: ResearchCampaignSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<RunMetaSelectionResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage5: Option<RunMetaStage5Response>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchCampaignProgressPayload {
    pub stage: String,
    pub dataset_index: usize,
    pub dataset_total: usize,
    pub label: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_json_path: Option<String>,
    pub optimized: usize,
    pub validated: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampaignOptimizationFamily {
    DoubleLike,
    CompositeCpu,
    CompositeMo,
}

enum CampaignOptimizationResult {
    Backend(BackendOptimizationResult),
    CompositeMo(CompositeMoResult),
    CompositeMoArtifact(Value),
}

struct PreparedCampaignDataset {
    input: ResearchCampaignDatasetInput,
    candles: vector_ta::utilities::data_loader::Candles,
    dataset_meta: Value,
    timeframe: String,
    label: String,
}

pub struct HeadlessPreparedResearchCampaignDatasetPack {
    datasets: VecDeque<PreparedCampaignDataset>,
}

struct DatasetPreloadState {
    receiver: mpsc::Receiver<Result<PreparedCampaignDataset, String>>,
    cached: Option<Result<PreparedCampaignDataset, String>>,
}

type SharedDatasetPreloadState = Arc<Mutex<DatasetPreloadState>>;

struct DatasetPreloadJob {
    state: SharedDatasetPreloadState,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl DatasetPreloadJob {
    fn spawn(
        input: ResearchCampaignDatasetInput,
        timeframe_override: Option<String>,
        app_cache_dir: PathBuf,
    ) -> Self {
        Self::spawn_with_task(move || {
            prepare_campaign_dataset(input, timeframe_override.as_deref(), &app_cache_dir)
        })
    }

    fn spawn_with_task<F>(task: F) -> Self
    where
        F: FnOnce() -> Result<PreparedCampaignDataset, String> + Send + 'static,
    {
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = std::thread::spawn(move || {
            let _ = tx.send(task());
        });
        Self {
            state: Arc::new(Mutex::new(DatasetPreloadState {
                receiver: rx,
                cached: None,
            })),
            handle: Some(handle),
        }
    }

    fn shared_state(&self) -> SharedDatasetPreloadState {
        Arc::clone(&self.state)
    }

    fn join(mut self) -> Result<PreparedCampaignDataset, String> {
        self.join_worker()?;
        self.take_result()
    }

    fn join_worker(&mut self) -> Result<(), String> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };
        handle
            .join()
            .map_err(|_| "campaign dataset preload worker panicked".to_string())
    }

    fn take_result(&self) -> Result<PreparedCampaignDataset, String> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(result) = state.cached.take() {
            return result;
        }
        match state.receiver.recv() {
            Ok(result) => result,
            Err(_) => Err("campaign dataset preload disconnected".to_string()),
        }
    }
}

impl Drop for DatasetPreloadJob {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn try_fill_preload_state(state: &mut DatasetPreloadState) {
    if state.cached.is_some() {
        return;
    }
    match state.receiver.try_recv() {
        Ok(result) => state.cached = Some(result),
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {
            state.cached = Some(Err("campaign dataset preload disconnected".to_string()))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CampaignDoubleMaVramPrefetchConfig {
    device_id: u32,
    fast_ma_type: String,
    slow_ma_type: String,
    ma_source: String,
}

struct CampaignDoubleMaVramPrefetch {
    preload_state: SharedDatasetPreloadState,
    config: CampaignDoubleMaVramPrefetchConfig,
    completed: AtomicBool,
    feedback: Mutex<Option<DoubleMaCampaignVramPrefetchFeedback>>,
}

impl CampaignDoubleMaVramPrefetch {
    fn with_request_if_ready(&self, f: &mut dyn FnMut(u32, &Candles, &str, &str, &str)) {
        if self.completed.load(Ordering::Relaxed) {
            return;
        }
        let mut preload = self.preload_state.lock().unwrap_or_else(|e| e.into_inner());
        try_fill_preload_state(&mut preload);
        let Some(result) = preload.cached.as_ref() else {
            return;
        };
        let Ok(prepared) = result else {
            self.completed.store(true, Ordering::Relaxed);
            return;
        };
        f(
            self.config.device_id,
            &prepared.candles,
            &self.config.fast_ma_type,
            &self.config.slow_ma_type,
            &self.config.ma_source,
        );
        self.completed.store(true, Ordering::Relaxed);
    }

    fn record_feedback(&self, feedback: DoubleMaCampaignVramPrefetchFeedback) {
        *self.feedback.lock().unwrap_or_else(|e| e.into_inner()) = Some(feedback);
    }

    fn take_feedback(&self) -> Option<DoubleMaCampaignVramPrefetchFeedback> {
        self.feedback
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
}

struct CampaignProgressSink {
    app: tauri::AppHandle,
    vram_prefetch: Option<CampaignDoubleMaVramPrefetch>,
}

impl CampaignProgressSink {
    fn take_prefetch_feedback(&self) -> Option<DoubleMaCampaignVramPrefetchFeedback> {
        self.vram_prefetch
            .as_ref()
            .and_then(CampaignDoubleMaVramPrefetch::take_feedback)
    }
}

impl ProgressSink for CampaignProgressSink {
    fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload) {
        let _ = self.app.emit("double_ma_progress", payload);
    }

    fn with_double_ma_campaign_vram_prefetch_request(
        &self,
        f: &mut dyn FnMut(u32, &Candles, &str, &str, &str),
    ) {
        if let Some(prefetch) = self.vram_prefetch.as_ref() {
            prefetch.with_request_if_ready(f);
        }
    }

    fn on_double_ma_campaign_vram_prefetch_feedback(
        &self,
        feedback: DoubleMaCampaignVramPrefetchFeedback,
    ) {
        if let Some(prefetch) = self.vram_prefetch.as_ref() {
            prefetch.record_feedback(feedback);
        }
    }
}

fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

fn normalize_strategy_kind(raw: &str) -> String {
    match raw.trim() {
        "" => "DoubleMa".to_string(),
        "DoubleMa" => "DoubleMa".to_string(),
        "DynamicDoubleMa" => "DynamicDoubleMa".to_string(),
        "MlSignal" => "MlSignal".to_string(),
        "Pit" => "Pit".to_string(),
        "Set" => "Set".to_string(),
        "Mts" => "Mts".to_string(),
        "Orc" => "Orc".to_string(),
        "Marrt" => "Marrt".to_string(),
        "Vfct" => "Vfct".to_string(),
        "Rocb" => "Rocb".to_string(),
        "Vcet" => "Vcet".to_string(),
        "Mkcs" => "Mkcs".to_string(),
        "Kqsr" => "Kqsr".to_string(),
        "Carm" => "Carm".to_string(),
        "Kvdf" => "Kvdf".to_string(),
        "Cdmc" => "Cdmc".to_string(),
        "Asr" => "Asr".to_string(),
        "Vcc" => "Vcc".to_string(),
        "Vemr" => "Vemr".to_string(),
        "CompositeCpu" => "CompositeCpu".to_string(),
        "CompositeMo" => "CompositeMo".to_string(),
        "CompositeTrend" => "CompositeTrend".to_string(),
        "RegimeHybrid" => "RegimeHybrid".to_string(),
        "BreakoutMomentum" => "BreakoutMomentum".to_string(),
        other => other.to_string(),
    }
}

fn resolve_campaign_strategy_kind(raw: &str) -> Result<String, String> {
    let normalized = normalize_strategy_kind(raw);
    match normalized.as_str() {
        "DoubleMa"
        | "Pit"
        | "Set"
        | "Mts"
        | "Orc"
        | "Marrt"
        | "Vfct"
        | "Rocb"
        | "Vcet"
        | "Mkcs"
        | "Kqsr"
        | "Carm"
        | "Kvdf"
        | "Cdmc"
        | "Asr"
        | "Vcc"
        | "Vemr"
        | "CompositeCpu"
        | "CompositeMo"
        | "CompositeTrend"
        | "RegimeHybrid"
        | "BreakoutMomentum" => Ok(normalized),
        "DynamicDoubleMa" => Err(
            "Research campaigns do not support DynamicDoubleMa yet because validation does not preserve its dynamic strategy IR."
                .to_string(),
        ),
        "MlSignal" => Err(
            "Research campaigns do not support MlSignal. Use the dedicated ML Signal workflow instead."
                .to_string(),
        ),
        other => Err(format!(
            "Unsupported research campaign strategy kind '{other}'."
        )),
    }
}

fn is_double_like_kind(kind: &str) -> bool {
    matches!(
        normalize_strategy_kind(kind).as_str(),
        "DoubleMa"
            | "Pit"
            | "Set"
            | "Mts"
            | "Orc"
            | "Marrt"
            | "Vfct"
            | "CompositeTrend"
            | "RegimeHybrid"
            | "BreakoutMomentum"
    )
}

fn optimization_family_for_kind(kind: &str) -> CampaignOptimizationFamily {
    match normalize_strategy_kind(kind).as_str() {
        "CompositeMo" => CampaignOptimizationFamily::CompositeMo,
        "CompositeCpu" | "Asr" | "Vcc" | "Vemr" | "Rocb" | "Vcet" | "Mkcs" | "Kqsr" | "Carm"
        | "Kvdf" | "Cdmc" => CampaignOptimizationFamily::CompositeCpu,
        _ => CampaignOptimizationFamily::DoubleLike,
    }
}

fn normalize_path_key(raw: &str) -> String {
    raw.trim().replace('/', "\\").to_ascii_lowercase()
}

fn preload_enabled(policy: ResearchCampaignPreloadPolicy) -> bool {
    !matches!(policy, ResearchCampaignPreloadPolicy::Disabled)
}

fn requested_gpu_device_id(backend: &Backend) -> Option<u32> {
    match backend {
        Backend::Auto { device_id }
        | Backend::GpuOnly { device_id }
        | Backend::GpuHybridKernel { device_id }
        | Backend::GpuKernel { device_id } => Some(*device_id),
        Backend::CpuOnly => None,
    }
}

fn supports_exact_gpu_kernel_route(result: &BackendOptimizationResult) -> bool {
    matches!(result.backend_used, BackendUsed::Gpu)
        && result
            .backend_route
            .as_deref()
            .map(|route| route.contains("GpuKernel"))
            .unwrap_or(false)
}

fn build_doublema_vram_prefetch_config(
    run_req: &DoubleMaRequest,
    learned_device_id: Option<u32>,
) -> Option<CampaignDoubleMaVramPrefetchConfig> {
    if run_req.dynamic_strategy_ir.is_some() {
        return None;
    }
    if run_req.fast_ma_types.len() != 1 || run_req.slow_ma_types.len() != 1 {
        return None;
    }
    let device_id = match &run_req.backend {
        Backend::GpuKernel { device_id } => Some(*device_id),
        Backend::Auto { .. } => learned_device_id,
        _ => None,
    }?;
    let fast_ma_type = normalize_text(&run_req.fast_ma_types[0]).to_ascii_lowercase();
    let slow_ma_type = normalize_text(&run_req.slow_ma_types[0]).to_ascii_lowercase();
    if fast_ma_type.is_empty() || slow_ma_type.is_empty() {
        return None;
    }
    let ma_source = {
        let source = normalize_text(&run_req.ma_source);
        if source.is_empty() {
            "close".to_string()
        } else {
            source
        }
    };
    Some(CampaignDoubleMaVramPrefetchConfig {
        device_id,
        fast_ma_type,
        slow_ma_type,
        ma_source,
    })
}

fn normalize_timeframe_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    ta_app_core::parse_timeframe_label(trimmed)
        .map(|tf| tf.label)
        .unwrap_or_else(|_| trimmed.to_ascii_lowercase())
}

fn push_unique_warning(warnings: &mut Vec<String>, warning: String) {
    if warning.trim().is_empty() {
        return;
    }
    if !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

fn file_label_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn dataset_label_from_input(input: &ResearchCampaignDatasetInput) -> String {
    input
        .label
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| file_label_from_path(&input.csv_path))
}

fn dataset_timeframe_hint(
    input: &ResearchCampaignDatasetInput,
    timeframe_override: Option<&str>,
) -> String {
    timeframe_override
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            input
                .timeframe
                .as_deref()
                .map(normalize_timeframe_label)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn failed_item(
    csv_path: String,
    label: String,
    timeframe: String,
    is_holdout: bool,
    artifact_path: Option<String>,
    stage: &str,
    error: impl Into<String>,
) -> ResearchCampaignItemSummary {
    ResearchCampaignItemSummary {
        csv_path,
        label,
        timeframe,
        is_holdout,
        status: "failed".to_string(),
        artifact_path,
        report_xlsx_path: None,
        error: Some(format!("{stage}: {}", error.into())),
    }
}

fn default_campaign_tag(ts: &str) -> String {
    let mut compact = String::with_capacity(ts.len());
    for ch in ts.chars() {
        if ch.is_ascii_alphanumeric() {
            compact.push(ch);
        }
    }
    if compact.is_empty() {
        "campaign".to_string()
    } else {
        format!("campaign-{compact}")
    }
}

fn dedup_tags(tags: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for tag in tags {
        let next = normalize_text(&tag);
        if next.is_empty() {
            continue;
        }
        let key = next.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(next);
        }
    }
    out
}

fn hash_fnv1a32(text: &str) -> u32 {
    let mut hash: u32 = 2166136261;
    for ch in text.chars() {
        hash ^= ch as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

fn split_holdout_paths(
    paths: &[String],
    holdout_pct: u32,
    seed: &str,
) -> (Vec<String>, HashSet<String>) {
    let pct = holdout_pct.min(50);
    if paths.is_empty() || pct == 0 {
        return (paths.to_vec(), HashSet::new());
    }
    let total = paths.len();
    let holdout_count = ((total as f64) * (pct as f64 / 100.0)).floor() as usize;
    if holdout_count == 0 {
        return (paths.to_vec(), HashSet::new());
    }
    let seed32 = hash_fnv1a32(seed);
    let mut scored: Vec<(String, u32)> = paths
        .iter()
        .cloned()
        .map(|path| {
            let score = hash_fnv1a32(&path) ^ seed32;
            (path, score)
        })
        .collect();
    scored.sort_by(|left, right| left.1.cmp(&right.1));
    let holdout: HashSet<String> = scored
        .iter()
        .take(holdout_count)
        .map(|(path, _)| path.clone())
        .collect();
    let train: Vec<String> = scored
        .into_iter()
        .filter_map(|(path, _)| (!holdout.contains(&path)).then_some(path))
        .collect();
    (train, holdout)
}

fn emit_campaign_progress(
    app: &tauri::AppHandle,
    stage: &str,
    dataset_index: usize,
    dataset_total: usize,
    label: &str,
    message: impl Into<String>,
    summary_json_path: Option<&str>,
    counts: &ResearchCampaignCounts,
) {
    let _ = app.emit(
        "research_campaign_progress",
        ResearchCampaignProgressPayload {
            stage: stage.to_string(),
            dataset_index,
            dataset_total,
            label: label.to_string(),
            message: message.into(),
            summary_json_path: summary_json_path.map(|value| value.to_string()),
            optimized: counts.optimized,
            validated: counts.validated,
            skipped: counts.skipped,
            failed: counts.failed,
        },
    );
}

fn prepare_campaign_dataset(
    input: ResearchCampaignDatasetInput,
    timeframe_override: Option<&str>,
    app_cache_dir: &Path,
) -> Result<PreparedCampaignDataset, String> {
    let csv_path = normalize_text(&input.csv_path);
    if csv_path.is_empty() {
        return Err("campaign dataset csv_path is empty".to_string());
    }

    let LoadOutcome {
        mut candles,
        source_identity,
        ..
    } = data_cache::load_or_build(Path::new(&csv_path), app_cache_dir)?;

    let base_timeframe = input
        .timeframe
        .as_deref()
        .map(normalize_timeframe_label)
        .unwrap_or_default();
    let mut effective_timeframe = base_timeframe.clone();
    if let Some(raw_tf) = timeframe_override {
        let target = normalize_timeframe_label(raw_tf);
        if !target.is_empty() {
            let spec = ta_app_core::parse_timeframe_label(raw_tf)?;
            if target != base_timeframe {
                candles = ta_app_core::resample_candles(&candles, spec.interval_ms)?;
            }
            effective_timeframe = spec.label;
        }
    }

    let label = input
        .label
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| file_label_from_path(&csv_path));

    let mut dataset_meta = json!({
        "csv_path": csv_path,
        "label": label,
        "symbol": label,
        "timeframe": effective_timeframe,
        "base_timeframe": if base_timeframe.is_empty() { effective_timeframe.clone() } else { base_timeframe.clone() },
        "derived_timeframe": effective_timeframe,
        "resample_applied": !base_timeframe.is_empty() && !effective_timeframe.is_empty() && base_timeframe != effective_timeframe,
        "source_size_bytes": source_identity.size,
        "source_mtime_utc_ms": source_identity.mtime_utc_ms,
    });
    if let Some(fp) = input
        .source_content_fingerprint
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
    {
        dataset_meta["source_content_fingerprint"] = Value::String(fp);
    }

    Ok(PreparedCampaignDataset {
        input,
        candles,
        dataset_meta,
        timeframe: effective_timeframe,
        label,
    })
}

fn make_campaign_data_id(index: usize, csv_path: &str, timeframe: &str) -> String {
    let label = file_label_from_path(csv_path)
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let tf = timeframe
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    format!("campaign_{}_{}_{}", index + 1, label, tf)
}

fn patch_optimization_template(
    template: &Value,
    strategy_kind: &str,
    data_id: &str,
) -> Result<Value, String> {
    let mut req = template.clone();
    let Some(map) = req.as_object_mut() else {
        return Err("campaign optimization_template must be an object".to_string());
    };
    map.insert("data_id".to_string(), Value::String(data_id.to_string()));
    map.insert(
        "strategy_kind".to_string(),
        Value::String(normalize_strategy_kind(strategy_kind)),
    );
    Ok(req)
}

fn run_optimization_blocking(
    family: CampaignOptimizationFamily,
    req_value: &Value,
    candles: &vector_ta::utilities::data_loader::Candles,
    cancel: &AtomicBool,
    sink: Option<&dyn ProgressSink>,
) -> Result<CampaignOptimizationResult, String> {
    match family {
        CampaignOptimizationFamily::DoubleLike => {
            let mut req: DoubleMaRequest =
                serde_json::from_value(req_value.clone()).map_err(|e| e.to_string())?;
            req.mode = normalize_product_double_ma_optimization_mode(&req);
            let res = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                req, candles, cancel, sink,
            )?;
            Ok(CampaignOptimizationResult::Backend(res))
        }
        CampaignOptimizationFamily::CompositeCpu => {
            let mut req: CompositeCpuRequest =
                serde_json::from_value(req_value.clone()).map_err(|e| e.to_string())?;
            req.mode = normalize_product_composite_cpu_optimization_mode(req.mode);
            let res = ta_app_core::run_composite_cpu_optimization_blocking_with_candles(
                req, candles, cancel, sink,
            )?;
            Ok(CampaignOptimizationResult::Backend(res))
        }
        CampaignOptimizationFamily::CompositeMo => {
            let req: CompositeMoRequest =
                serde_json::from_value(req_value.clone()).map_err(|e| e.to_string())?;
            let res = ta_app_core::run_composite_mo_optimization_blocking_with_candles(
                req, candles, cancel, sink,
            )?;
            Ok(CampaignOptimizationResult::CompositeMo(res))
        }
    }
}

fn normalize_backend_value(raw: Option<&Value>, strategy_kind: &str) -> Value {
    let device_id = raw
        .and_then(Value::as_object)
        .and_then(|row| row.get("device_id"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;
    let backend_name = raw
        .and_then(Value::as_object)
        .and_then(|row| row.get("backend"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();

    if !is_double_like_kind(strategy_kind) && backend_name != "CpuOnly" {
        return json!({ "backend": "Auto", "device_id": device_id });
    }

    match backend_name {
        "Auto" => json!({ "backend": "Auto", "device_id": device_id }),
        "GpuOnly" => json!({ "backend": "GpuOnly", "device_id": device_id }),
        "GpuHybridKernel" => json!({ "backend": "GpuHybridKernel", "device_id": device_id }),
        "GpuKernel" => json!({ "backend": "GpuKernel", "device_id": device_id }),
        _ => json!({ "backend": "CpuOnly" }),
    }
}

fn candidate_values_from_optimization(
    strategy_kind: &str,
    family: CampaignOptimizationFamily,
    run_req: &Value,
    run_res: &CampaignOptimizationResult,
    max_candidates: usize,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    match (family, run_res) {
        (CampaignOptimizationFamily::DoubleLike, CampaignOptimizationResult::Backend(res))
        | (CampaignOptimizationFamily::CompositeCpu, CampaignOptimizationResult::Backend(res)) => {
            let top_values: Vec<Value> = if !res.top.is_empty() {
                res.top
                    .iter()
                    .map(|(params, _)| serde_json::to_value(params).map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                vec![serde_json::to_value(&res.best_params).map_err(|e| e.to_string())?]
            };
            for params_value in top_values {
                let Some(p) = params_value.as_object() else {
                    continue;
                };
                let fast_ma_type = p
                    .get("fast_ma_type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let slow_ma_type = p
                    .get("slow_ma_type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let fast_len = p.get("fast_len").and_then(Value::as_u64).unwrap_or(0);
                let slow_len = p.get("slow_len").and_then(Value::as_u64).unwrap_or(0);
                if fast_ma_type.is_empty()
                    || slow_ma_type.is_empty()
                    || fast_len == 0
                    || slow_len == 0
                {
                    continue;
                }

                let candidate = if is_double_like_kind(strategy_kind) {
                    json!({
                        "fast_ma_type": fast_ma_type,
                        "slow_ma_type": slow_ma_type,
                        "fast_len": fast_len,
                        "slow_len": slow_len,
                        "fast_ma_params": p.get("fast_ma_params").cloned().unwrap_or(Value::Null),
                        "slow_ma_params": p.get("slow_ma_params").cloned().unwrap_or(Value::Null),
                        "strategy": run_req.get("strategy").cloned().or_else(|| run_req.get("base_strategy").cloned()).unwrap_or_else(|| json!({})),
                        "halftrend_filter": run_req.get("halftrend_filter").cloned().or_else(|| run_req.get("halftrendFilter").cloned()).unwrap_or(Value::Null),
                    })
                } else {
                    build_composite_candidate_value(
                        strategy_kind,
                        run_req,
                        p,
                        fast_ma_type,
                        slow_ma_type,
                        fast_len,
                        slow_len,
                    )
                };

                let key = serde_json::to_string(&candidate).map_err(|e| e.to_string())?;
                if seen.insert(key) {
                    out.push(candidate);
                    if out.len() >= max_candidates {
                        break;
                    }
                }
            }
        }
        (CampaignOptimizationFamily::CompositeMo, CampaignOptimizationResult::CompositeMo(res)) => {
            for point in &res.pareto {
                let params = serde_json::to_value(&point.params).map_err(|e| e.to_string())?;
                let key = serde_json::to_string(&params).map_err(|e| e.to_string())?;
                if seen.insert(key) {
                    out.push(params);
                    if out.len() >= max_candidates {
                        break;
                    }
                }
            }
        }
        (
            CampaignOptimizationFamily::CompositeMo,
            CampaignOptimizationResult::CompositeMoArtifact(run_res),
        ) => {
            let pareto = run_res
                .get("pareto")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    "campaign validation could not read pareto candidates from the saved CompositeMo optimization artifact"
                        .to_string()
                })?;
            for point in pareto {
                let Some(params) = point.get("params").cloned() else {
                    continue;
                };
                let key = serde_json::to_string(&params).map_err(|e| e.to_string())?;
                if seen.insert(key) {
                    out.push(params);
                    if out.len() >= max_candidates {
                        break;
                    }
                }
            }
        }
        _ => return Err("campaign optimization result family mismatch".to_string()),
    }

    if out.is_empty() {
        return Err(
            "campaign validation could not derive any candidates from optimization output"
                .to_string(),
        );
    }
    Ok(out)
}

fn build_composite_candidate_value(
    strategy_kind: &str,
    run_req: &Value,
    params: &serde_json::Map<String, Value>,
    fast_ma_type: String,
    slow_ma_type: String,
    fast_len: u64,
    slow_len: u64,
) -> Value {
    let base = json!({
        "fast_ma_type": fast_ma_type,
        "slow_ma_type": slow_ma_type,
        "fast_len": fast_len,
        "slow_len": slow_len,
        "fast_ma_params": params.get("fast_ma_params").cloned().unwrap_or(Value::Null),
        "slow_ma_params": params.get("slow_ma_params").cloned().unwrap_or(Value::Null),
        "strategy": run_req.get("strategy").cloned().or_else(|| run_req.get("base_strategy").cloned()).unwrap_or_else(|| json!({})),
    });
    match normalize_strategy_kind(strategy_kind).as_str() {
        "Asr" => merge_json(
            base,
            json!({ "asr": run_req.get("asr").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Vcc" => merge_json(
            base,
            json!({ "vcc": run_req.get("vcc").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Vemr" => merge_json(
            base,
            json!({ "vemr": run_req.get("vemr").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Rocb" => merge_json(
            base,
            json!({ "rocb": run_req.get("rocb").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Vcet" => merge_json(
            base,
            json!({ "vcet": run_req.get("vcet").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Mkcs" => merge_json(
            base,
            json!({ "mkcs": run_req.get("mkcs").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Kqsr" => merge_json(
            base,
            json!({ "kqsr": run_req.get("kqsr").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Carm" => merge_json(
            base,
            json!({ "carm": run_req.get("carm").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Kvdf" => merge_json(
            base,
            json!({ "kvdf": run_req.get("kvdf").cloned().unwrap_or_else(|| json!({})) }),
        ),
        "Cdmc" => merge_json(
            base,
            json!({ "cdmc": run_req.get("cdmc").cloned().unwrap_or_else(|| json!({})) }),
        ),
        _ => merge_json(
            base,
            json!({
                "halftrend": run_req.get("halftrend").cloned().unwrap_or(Value::Null),
                "macz": run_req.get("macz").cloned().unwrap_or_else(|| json!({})),
                "ott": run_req.get("ott").cloned().unwrap_or_else(|| json!({})),
                "signal_mode": run_req.get("signal_mode").cloned().or_else(|| run_req.get("signalMode").cloned()).unwrap_or_else(|| Value::String("hard_gate".to_string())),
            }),
        ),
    }
}

fn merge_json(mut left: Value, right: Value) -> Value {
    if let (Some(left_map), Some(right_map)) = (left.as_object_mut(), right.as_object()) {
        for (key, value) in right_map {
            left_map.insert(key.clone(), value.clone());
        }
    }
    left
}

fn build_validation_request_value(
    strategy_kind: &str,
    data_id: &str,
    run_req: &Value,
    run_res: &CampaignOptimizationResult,
    dataset_meta: &Value,
    validation_cfg: &ResearchCampaignValidationConfig,
    family: CampaignOptimizationFamily,
) -> Result<Value, String> {
    let candidates = candidate_values_from_optimization(
        strategy_kind,
        family,
        run_req,
        run_res,
        validation_cfg.max_candidates.max(1),
    )?;
    let dataset = dataset_meta.as_object();
    let csv_path = dataset
        .and_then(|row| row.get("csv_path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let timeframe = dataset
        .and_then(|row| row.get("timeframe"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let label = dataset
        .and_then(|row| row.get("symbol").or_else(|| row.get("label")))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    Ok(json!({
        "data_id": data_id,
        "backend": normalize_backend_value(run_req.get("backend"), strategy_kind),
        "ma_source": run_req.get("ma_source").cloned().unwrap_or_else(|| Value::String("close".to_string())),
        "fast_ma_params": run_req.get("fast_ma_params").cloned().unwrap_or(Value::Null),
        "slow_ma_params": run_req.get("slow_ma_params").cloned().unwrap_or(Value::Null),
        "candidates": candidates,
        "walk_forward": validation_cfg.walk_forward,
        "reopt_walk_forward": validation_cfg.reopt_walk_forward,
        "spa": validation_cfg.spa,
        "perturb": validation_cfg.perturb,
        "trade_bootstrap": validation_cfg.trade_bootstrap,
        "cross_ticker": validation_cfg.cross_ticker,
        "cscv_pbo": cscv_pbo_to_value(validation_cfg.cscv_pbo.as_ref()),
        "seed": validation_cfg.seed,
        "max_candidates": validation_cfg.max_candidates.max(1),
        "min_psr": validation_cfg.min_psr,
        "min_dsr": validation_cfg.min_dsr,
        "min_cscv_oos_rank_frac": validation_cfg.min_cscv_oos_rank_frac,
        "min_cscv_oos_metric": validation_cfg.min_cscv_oos_metric,
        "max_pbo": validation_cfg.max_pbo,
        "validation_context": ValidationArtifactContext {
            strategy_kind: normalize_strategy_kind(strategy_kind),
            csv_path,
            timeframe,
            label,
        }
    }))
}

fn cscv_pbo_to_value(value: Option<&CscvPboConfig>) -> Value {
    match value {
        Some(cfg) => json!({
            "slices": cfg.slices,
            "max_combinations": cfg.max_combinations,
            "threshold": cfg.threshold,
        }),
        None => Value::Null,
    }
}

fn build_request_summary_value(
    req: &ResearchCampaignRequest,
    resolved_tags: &[String],
    execution_mode: ResearchCampaignExecutionMode,
) -> Value {
    json!({
        "strategy_kind": normalize_strategy_kind(&req.strategy_kind),
        "execution_mode": execution_mode,
        "datasets": req.datasets.iter().map(|dataset| {
            json!({
                "csv_path": dataset.csv_path,
                "label": dataset.label,
                "timeframe": dataset.timeframe,
            })
        }).collect::<Vec<_>>(),
        "timeframe_override": req.timeframe_override,
        "tags": resolved_tags,
        "holdout_pct": req.holdout_pct,
        "holdout_tag": req.holdout_tag,
        "skip_existing": req.skip_existing,
        "auto_meta": req.auto_meta,
        "auto_stage5": req.auto_stage5,
        "optimization_template": req.optimization_template,
        "validation_config": {
            "max_candidates": req.validation_config.max_candidates,
            "seed": req.validation_config.seed,
            "walk_forward": req.validation_config.walk_forward,
            "reopt_walk_forward": req.validation_config.reopt_walk_forward,
            "spa": req.validation_config.spa,
            "perturb": req.validation_config.perturb,
            "trade_bootstrap": req.validation_config.trade_bootstrap,
            "cross_ticker": req.validation_config.cross_ticker,
            "cscv_pbo": cscv_pbo_to_value(req.validation_config.cscv_pbo.as_ref()),
            "min_psr": req.validation_config.min_psr,
            "min_dsr": req.validation_config.min_dsr,
            "min_cscv_oos_rank_frac": req.validation_config.min_cscv_oos_rank_frac,
            "min_cscv_oos_metric": req.validation_config.min_cscv_oos_metric,
            "max_pbo": req.validation_config.max_pbo,
        },
        "meta_request": req.meta_request,
        "stage5_request": req.stage5_request,
        "preload_policy": req.preload_policy,
    })
}

fn build_campaign_summary(
    ts: &str,
    state: &str,
    strategy_kind: &str,
    timeframe: Option<String>,
    req: &ResearchCampaignRequest,
    execution_mode: ResearchCampaignExecutionMode,
    tags: &[String],
    holdout_tag: Option<String>,
    counts: &ResearchCampaignCounts,
    items: &[ResearchCampaignItemSummary],
    meta_response: Option<&RunMetaSelectionResponse>,
    stage5_response: Option<&RunMetaStage5Response>,
    warnings: &[String],
    runtime_ms: u64,
) -> ResearchCampaignSummary {
    ResearchCampaignSummary {
        version: 1,
        kind: "vectorgrid_campaign_summary".to_string(),
        ts: ts.to_string(),
        state: state.to_string(),
        strategy_kind: strategy_kind.to_string(),
        timeframe,
        tags: tags.to_vec(),
        holdout_tag,
        holdout_pct: req.holdout_pct.min(50),
        request: build_request_summary_value(req, tags, execution_mode),
        counts: counts.clone(),
        items: items.to_vec(),
        meta_json_path: meta_response.map(|value| value.meta_json_path.clone()),
        stage5_json_path: stage5_response.map(|value| value.stage5_json_path.clone()),
        warnings: warnings.to_vec(),
        runtime_ms,
    }
}

fn save_campaign_summary_blocking(
    summary: &ResearchCampaignSummary,
    app: &tauri::AppHandle,
    existing_path: Option<&str>,
) -> Result<(String, usize), String> {
    let path = existing_path
        .map(PathBuf::from)
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or(run_artifacts::suggest_campaign_summary_path(
            &summary.ts,
            app,
        )?);
    let bytes = serde_json::to_vec_pretty(summary).map_err(|e| e.to_string())?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok((path.to_string_lossy().to_string(), bytes.len()))
}

fn artifact_stub(run_req: &Value, validation_req: Option<&Value>) -> Value {
    json!({
        "run": { "req": run_req },
        "validation": validation_req.map(|req| json!({ "req": req })),
    })
}

fn existing_validation_matches(
    existing: &[run_artifacts::RunArtifactSummary],
    strategy_kind: &str,
    config_fingerprint: &str,
    primary_tag: &str,
    prepared: &PreparedCampaignDataset,
) -> bool {
    existing_optimization_match(existing, strategy_kind, config_fingerprint, primary_tag, prepared)
        .map(|row| row.has_validation)
        .unwrap_or(false)
}

fn existing_optimization_match<'a>(
    existing: &'a [run_artifacts::RunArtifactSummary],
    strategy_kind: &str,
    config_fingerprint: &str,
    primary_tag: &str,
    prepared: &PreparedCampaignDataset,
) -> Option<&'a run_artifacts::RunArtifactSummary> {
    let dataset_tf = normalize_timeframe_label(&prepared.timeframe);
    let dataset_path = normalize_path_key(&prepared.input.csv_path);
    let strict_fp = prepared
        .input
        .source_content_fingerprint
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty());
    for row in existing {
        if normalize_strategy_kind(&row.strategy_kind) != normalize_strategy_kind(strategy_kind) {
            continue;
        }
        if row.config_fingerprint.trim() != config_fingerprint.trim() {
            continue;
        }
        if !row
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(primary_tag))
        {
            continue;
        }
        if normalize_path_key(&row.csv_path) != dataset_path {
            continue;
        }
        if !dataset_tf.is_empty() && normalize_timeframe_label(&row.timeframe) != dataset_tf {
            continue;
        }
        if let Some(expected_fp) = strict_fp.as_deref() {
            if row.source_content_fingerprint.as_deref() == Some(expected_fp) {
                return Some(row);
            }
            continue;
        }
        if let (Some(size), Some(mtime)) = (row.source_size_bytes, row.source_mtime_utc_ms) {
            if prepared.input.source_size_bytes == Some(size)
                && prepared.input.source_mtime_utc_ms == Some(mtime)
            {
                return Some(row);
            }
        } else {
            return Some(row);
        }
    }
    None
}

fn can_reuse_optimization_artifact_for_validation(family: CampaignOptimizationFamily) -> bool {
    matches!(
        family,
        CampaignOptimizationFamily::DoubleLike
            | CampaignOptimizationFamily::CompositeCpu
            | CampaignOptimizationFamily::CompositeMo
    )
}

fn optimization_result_from_artifact_value(
    family: CampaignOptimizationFamily,
    artifact: &Value,
) -> Result<CampaignOptimizationResult, String> {
    let run_res = artifact
        .pointer("/run/res")
        .cloned()
        .ok_or_else(|| "existing optimization artifact was missing run.res".to_string())?;
    match family {
        CampaignOptimizationFamily::DoubleLike | CampaignOptimizationFamily::CompositeCpu => {
            let res: BackendOptimizationResult =
                serde_json::from_value(run_res).map_err(|e| e.to_string())?;
            Ok(CampaignOptimizationResult::Backend(res))
        }
        CampaignOptimizationFamily::CompositeMo => {
            let pareto = run_res
                .get("pareto")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    "existing CompositeMo optimization artifact was missing pareto candidates"
                        .to_string()
                })?;
            if pareto.is_empty() {
                return Err(
                    "existing CompositeMo optimization artifact had an empty pareto frontier"
                        .to_string(),
                );
            }
            Ok(CampaignOptimizationResult::CompositeMoArtifact(run_res))
        }
    }
}

fn optimization_request_from_artifact_value(artifact: &Value) -> Result<Value, String> {
    artifact
        .pointer("/run/req")
        .cloned()
        .ok_or_else(|| "existing optimization artifact was missing run.req".to_string())
}

fn build_run_artifact_value(
    ts: &str,
    config_fingerprint: &str,
    csv_path: &str,
    dataset_meta: &Value,
    tags: &[String],
    run_req: &Value,
    run_res: &Value,
    validation_req: Option<&Value>,
    validation_res: Option<&Value>,
) -> Value {
    json!({
        "version": 1,
        "kind": "vectorgrid_run_artifact",
        "ts": ts,
        "config_fingerprint": config_fingerprint,
        "run": { "req": run_req, "res": run_res },
        "validation": validation_req.zip(validation_res).map(|(req, res)| json!({ "req": req, "res": res })),
        "report_xlsx_path": "",
        "data": { "csv_path": csv_path },
        "dataset_meta": dataset_meta,
        "tags": tags,
    })
}

fn run_research_campaign_blocking_with_mode(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
    execution_mode: ResearchCampaignExecutionMode,
    mut preloaded_pack: Option<HeadlessPreparedResearchCampaignDatasetPack>,
) -> Result<RunResearchCampaignResponse, String> {
    let started = std::time::Instant::now();
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = data_cache::app_data_cache_dir(&base);
    let strategy_kind = resolve_campaign_strategy_kind(&req.strategy_kind)?;
    if req.datasets.is_empty() {
        return Err("campaign requires at least one dataset".to_string());
    }

    let family = optimization_family_for_kind(&strategy_kind);
    let ts = chrono::Utc::now().to_rfc3339();
    let mut tags = dedup_tags(req.tags.clone());
    if tags.is_empty() {
        tags.push(default_campaign_tag(&ts));
    }
    let primary_tag = tags[0].clone();
    let holdout_tag = req
        .holdout_tag
        .as_deref()
        .map(normalize_text)
        .filter(|value| !value.is_empty())
        .or_else(|| (req.holdout_pct > 0).then_some("holdout".to_string()));

    let dataset_paths: Vec<String> = req
        .datasets
        .iter()
        .map(|entry| normalize_text(&entry.csv_path))
        .filter(|entry| !entry.is_empty())
        .collect();
    if dataset_paths.is_empty() {
        return Err("campaign datasets resolved to zero csv_path entries".to_string());
    }

    let (_train_paths, holdout_paths) =
        split_holdout_paths(&dataset_paths, req.holdout_pct, &primary_tag);
    let mut counts = ResearchCampaignCounts {
        total: dataset_paths.len(),
        optimized: 0,
        validated: 0,
        skipped: 0,
        failed: 0,
        cancelled: false,
    };
    let mut warnings = Vec::new();
    if req.preload_policy == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
        && strategy_kind == "DoubleMa"
    {
        warnings.push(
            "host_plus_doublema_vram is experimental, limited to the plain DoubleMa exact GpuKernel route, and falls back to host preload when GPU staging is unavailable."
                .to_string(),
        );
    }

    let existing = if req.skip_existing {
        run_artifacts::list_run_artifacts_blocking(app.clone()).unwrap_or_default()
    } else {
        Vec::new()
    };

    let timeframe_override = req
        .timeframe_override
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty());
    let dataset_total = req.datasets.len();
    let using_preloaded_pack = preloaded_pack.is_some();
    if let Some(pack) = preloaded_pack.as_ref() {
        if pack.datasets.len() != req.datasets.len() {
            return Err(format!(
                "preloaded campaign dataset pack length {} did not match request dataset length {}",
                pack.datasets.len(),
                req.datasets.len()
            ));
        }
    }
    if using_preloaded_pack {
        warnings.push(
            "headless benchmark supplied a preloaded dataset pack; request.preload_policy remains unchanged because full_pack_host is benchmark-only."
                .to_string(),
        );
    }
    let mut preload = if !using_preloaded_pack && preload_enabled(req.preload_policy) {
        Some(DatasetPreloadJob::spawn(
            req.datasets[0].clone(),
            timeframe_override.clone(),
            app_cache_dir.clone(),
        ))
    } else {
        None
    };
    let sink = TauriProgressSink { app: app.clone() };
    let mut learned_doublema_vram_device: Option<u32> = None;
    let mut doublema_vram_prefetch_disabled = false;
    let mut items: Vec<ResearchCampaignItemSummary> = req
        .datasets
        .iter()
        .map(|dataset| {
            let csv_path = normalize_text(&dataset.csv_path);
            ResearchCampaignItemSummary {
                is_holdout: holdout_paths.contains(&csv_path),
                csv_path,
                label: dataset_label_from_input(dataset),
                timeframe: dataset_timeframe_hint(dataset, timeframe_override.as_deref()),
                status: "pending".to_string(),
                artifact_path: None,
                report_xlsx_path: None,
                error: None,
            }
        })
        .collect();
    let mut summary = build_campaign_summary(
        &ts,
        "running",
        &strategy_kind,
        timeframe_override.clone(),
        &req,
        execution_mode,
        &tags,
        holdout_tag.clone(),
        &counts,
        &items,
        None,
        None,
        &warnings,
        started.elapsed().as_millis() as u64,
    );
    let (summary_json_path, _) = save_campaign_summary_blocking(&summary, &app, None)?;
    emit_campaign_progress(
        &app,
        "checkpoint",
        0,
        dataset_total,
        &strategy_kind,
        "campaign initialized",
        Some(&summary_json_path),
        &counts,
    );

    for (index, dataset) in req.datasets.iter().cloned().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            counts.cancelled = true;
            break;
        }

        let dataset_csv_path = normalize_text(&dataset.csv_path);
        let is_holdout = holdout_paths.contains(&dataset_csv_path);
        let fallback_label = dataset_label_from_input(&dataset);
        let fallback_timeframe = dataset_timeframe_hint(&dataset, timeframe_override.as_deref());
        let prepared_result = match preloaded_pack.as_mut() {
            Some(pack) => pack
                .datasets
                .pop_front()
                .ok_or_else(|| "preloaded campaign dataset pack was missing an entry".to_string()),
            None => match preload.take() {
                Some(job) => job.join(),
                None => prepare_campaign_dataset(
                    dataset.clone(),
                    timeframe_override.as_deref(),
                    &app_cache_dir,
                ),
            },
        };
        if !using_preloaded_pack
            && preload_enabled(req.preload_policy)
            && index + 1 < req.datasets.len()
        {
            preload = Some(DatasetPreloadJob::spawn(
                req.datasets[index + 1].clone(),
                timeframe_override.clone(),
                app_cache_dir.clone(),
            ));
        }
        let prepared = match prepared_result {
            Ok(prepared) => prepared,
            Err(error) => {
                counts.failed = counts.failed.saturating_add(1);
                items[index] = failed_item(
                    dataset_csv_path.clone(),
                    fallback_label.clone(),
                    fallback_timeframe.clone(),
                    is_holdout,
                    None,
                    "prepare",
                    error,
                );
                summary = build_campaign_summary(
                    &ts,
                    "running",
                    &strategy_kind,
                    timeframe_override.clone(),
                    &req,
                    execution_mode,
                    &tags,
                    holdout_tag.clone(),
                    &counts,
                    &items,
                    None,
                    None,
                    &warnings,
                    started.elapsed().as_millis() as u64,
                );
                let _ = save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                continue;
            }
        };
        if using_preloaded_pack {
            let expected_csv_path = normalize_text(&dataset.csv_path);
            let prepared_csv_path = normalize_text(&prepared.input.csv_path);
            if prepared_csv_path != expected_csv_path {
                return Err(format!(
                    "preloaded campaign dataset pack order mismatch at dataset {}: expected '{}' but got '{}'",
                    index + 1,
                    expected_csv_path,
                    prepared_csv_path
                ));
            }
            let expected_timeframe =
                dataset_timeframe_hint(&dataset, timeframe_override.as_deref());
            if prepared.timeframe != expected_timeframe {
                return Err(format!(
                    "preloaded campaign dataset pack timeframe mismatch for '{}': expected '{}' but got '{}'",
                    expected_csv_path,
                    expected_timeframe,
                    prepared.timeframe
                ));
            }
        }

        let label = prepared.label.clone();
        emit_campaign_progress(
            &app,
            "prepare",
            index + 1,
            dataset_total,
            &label,
            "dataset ready",
            Some(&summary_json_path),
            &counts,
        );

        let run_ts = chrono::Utc::now().to_rfc3339();
        let data_id = make_campaign_data_id(index, &prepared.input.csv_path, &prepared.timeframe);
        let mut run_req_value =
            patch_optimization_template(&req.optimization_template, &strategy_kind, &data_id)?;
        let parsed_doublema_req = if strategy_kind == "DoubleMa" {
            serde_json::from_value::<DoubleMaRequest>(run_req_value.clone()).ok()
        } else {
            None
        };
        let run_only_fingerprint = run_artifacts::compute_config_fingerprint_from_artifact(
            &artifact_stub(&run_req_value, None),
        );
        let per_run_tags = {
            let mut next = tags.clone();
            next.push(label.clone());
            if is_holdout {
                if let Some(tag) = holdout_tag.as_ref() {
                    next.push(tag.clone());
                }
            }
            dedup_tags(next)
        };

        let mut existing_optimize_path: Option<String> = None;
        let optimize_result = if execution_mode == ResearchCampaignExecutionMode::ValidateOnly {
            let Some(existing_run) = existing_optimization_match(
                &existing,
                &strategy_kind,
                &run_only_fingerprint,
                &primary_tag,
                &prepared,
            ) else {
                counts.skipped = counts.skipped.saturating_add(1);
                items[index] = ResearchCampaignItemSummary {
                    csv_path: prepared.input.csv_path.clone(),
                    label,
                    timeframe: prepared.timeframe.clone(),
                    is_holdout,
                    status: "skipped".to_string(),
                    artifact_path: None,
                    report_xlsx_path: None,
                    error: Some(
                        "no matching optimization artifact was found for validate_only mode"
                            .to_string(),
                    ),
                };
                summary = build_campaign_summary(
                    &ts,
                    "running",
                    &strategy_kind,
                    timeframe_override.clone(),
                    &req,
                    execution_mode,
                    &tags,
                    holdout_tag.clone(),
                    &counts,
                    &items,
                    None,
                    None,
                    &warnings,
                    started.elapsed().as_millis() as u64,
                );
                let _ =
                    save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                continue;
            };
            existing_optimize_path = Some(existing_run.path.clone());
            emit_campaign_progress(
                &app,
                "optimize",
                index + 1,
                dataset_total,
                &label,
                "loading existing optimization artifact for validation",
                Some(&summary_json_path),
                &counts,
            );
            let artifact = match run_artifacts::load_run_artifact_blocking(
                existing_run.path.clone(),
                app.clone(),
            ) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        existing_optimize_path.clone(),
                        "load optimization artifact",
                        error,
                    );
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ = save_campaign_summary_blocking(
                        &summary,
                        &app,
                        Some(&summary_json_path),
                    )?;
                    continue;
                }
            };
            run_req_value = match optimization_request_from_artifact_value(&artifact) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        existing_optimize_path.clone(),
                        "decode optimization request",
                        error,
                    );
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ = save_campaign_summary_blocking(
                        &summary,
                        &app,
                        Some(&summary_json_path),
                    )?;
                    continue;
                }
            };
            match optimization_result_from_artifact_value(family, &artifact) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        existing_optimize_path.clone(),
                        "decode optimization artifact",
                        error,
                    );
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ = save_campaign_summary_blocking(
                        &summary,
                        &app,
                        Some(&summary_json_path),
                    )?;
                    continue;
                }
            }
        } else if req.skip_existing {
            if let Some(existing_run) = existing_optimization_match(
                &existing,
                &strategy_kind,
                &run_only_fingerprint,
                &primary_tag,
                &prepared,
            ) {
                if execution_mode == ResearchCampaignExecutionMode::OptimizeOnly {
                    existing_optimize_path = Some(existing_run.path.clone());
                    counts.skipped = counts.skipped.saturating_add(1);
                    items[index] = ResearchCampaignItemSummary {
                        csv_path: prepared.input.csv_path.clone(),
                        label,
                        timeframe: prepared.timeframe.clone(),
                        is_holdout,
                        status: "skipped".to_string(),
                        artifact_path: existing_optimize_path.clone(),
                        report_xlsx_path: None,
                        error: None,
                    };
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ =
                        save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                    continue;
                }

                if !can_reuse_optimization_artifact_for_validation(family) {
                    push_unique_warning(
                        &mut warnings,
                        "skip_existing found a matching CompositeMo optimization artifact, but full campaign reuse still reruns optimization because saved CompositeMo results are not deserializable for downstream validation yet."
                            .to_string(),
                    );
                    emit_campaign_progress(
                        &app,
                        "optimize",
                        index + 1,
                        dataset_total,
                        &label,
                        "existing optimization artifact found, rerunning optimization",
                        Some(&summary_json_path),
                        &counts,
                    );
                    let optimize_progress_sink = if req.preload_policy
                        == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
                        && strategy_kind == "DoubleMa"
                        && !doublema_vram_prefetch_disabled
                        && !using_preloaded_pack
                    {
                        match (
                            parsed_doublema_req.as_ref().and_then(|run_req| {
                                build_doublema_vram_prefetch_config(
                                    run_req,
                                    learned_doublema_vram_device,
                                )
                            }),
                            preload.as_ref().map(DatasetPreloadJob::shared_state),
                        ) {
                            (Some(config), Some(preload_state)) => Some(CampaignProgressSink {
                                app: app.clone(),
                                vram_prefetch: Some(CampaignDoubleMaVramPrefetch {
                                    preload_state,
                                    config,
                                    completed: AtomicBool::new(false),
                                    feedback: Mutex::new(None),
                                }),
                            }),
                            _ => Some(CampaignProgressSink {
                                app: app.clone(),
                                vram_prefetch: None,
                            }),
                        }
                    } else {
                        Some(CampaignProgressSink {
                            app: app.clone(),
                            vram_prefetch: None,
                        })
                    };
                    let optimize_result = match run_optimization_blocking(
                        family,
                        &run_req_value,
                        &prepared.candles,
                        cancel.as_ref(),
                        optimize_progress_sink
                            .as_ref()
                            .map(|value| value as &dyn ProgressSink),
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                None,
                                "optimization",
                                error,
                            );
                            summary = build_campaign_summary(
                                &ts,
                                "running",
                                &strategy_kind,
                                timeframe_override.clone(),
                                &req,
                                execution_mode,
                                &tags,
                                holdout_tag.clone(),
                                &counts,
                                &items,
                                None,
                                None,
                                &warnings,
                                started.elapsed().as_millis() as u64,
                            );
                            let _ = save_campaign_summary_blocking(
                                &summary,
                                &app,
                                Some(&summary_json_path),
                            )?;
                            continue;
                        }
                    };
                    if let Some(feedback) = optimize_progress_sink
                        .as_ref()
                        .and_then(CampaignProgressSink::take_prefetch_feedback)
                    {
                        match feedback {
                            DoubleMaCampaignVramPrefetchFeedback::Staged
                            | DoubleMaCampaignVramPrefetchFeedback::AlreadyResident => {}
                            DoubleMaCampaignVramPrefetchFeedback::Warning(note) => {
                                push_unique_warning(&mut warnings, note);
                            }
                            DoubleMaCampaignVramPrefetchFeedback::Fatal(note) => {
                                push_unique_warning(&mut warnings, note);
                                doublema_vram_prefetch_disabled = true;
                            }
                        }
                    }
                    counts.optimized = counts.optimized.saturating_add(1);
                    optimize_result
                } else {
                    existing_optimize_path = Some(existing_run.path.clone());
                    emit_campaign_progress(
                        &app,
                        "optimize",
                        index + 1,
                        dataset_total,
                        &label,
                        "reusing optimization artifact",
                        Some(&summary_json_path),
                        &counts,
                    );
                    let artifact = match run_artifacts::load_run_artifact_blocking(
                        existing_run.path.clone(),
                        app.clone(),
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                existing_optimize_path.clone(),
                                "load optimization artifact",
                                error,
                            );
                            summary = build_campaign_summary(
                                &ts,
                                "running",
                                &strategy_kind,
                                timeframe_override.clone(),
                                &req,
                                execution_mode,
                                &tags,
                                holdout_tag.clone(),
                                &counts,
                                &items,
                                None,
                                None,
                                &warnings,
                                started.elapsed().as_millis() as u64,
                            );
                            let _ = save_campaign_summary_blocking(
                                &summary,
                                &app,
                                Some(&summary_json_path),
                            )?;
                            continue;
                        }
                    };
                    run_req_value = match optimization_request_from_artifact_value(&artifact) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                existing_optimize_path.clone(),
                                "decode optimization request",
                                error,
                            );
                            summary = build_campaign_summary(
                                &ts,
                                "running",
                                &strategy_kind,
                                timeframe_override.clone(),
                                &req,
                                execution_mode,
                                &tags,
                                holdout_tag.clone(),
                                &counts,
                                &items,
                                None,
                                None,
                                &warnings,
                                started.elapsed().as_millis() as u64,
                            );
                            let _ = save_campaign_summary_blocking(
                                &summary,
                                &app,
                                Some(&summary_json_path),
                            )?;
                            continue;
                        }
                    };
                    match optimization_result_from_artifact_value(family, &artifact) {
                        Ok(value) => value,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                label,
                                prepared.timeframe.clone(),
                                is_holdout,
                                existing_optimize_path.clone(),
                                "decode optimization artifact",
                                error,
                            );
                            summary = build_campaign_summary(
                                &ts,
                                "running",
                                &strategy_kind,
                                timeframe_override.clone(),
                                &req,
                                execution_mode,
                                &tags,
                                holdout_tag.clone(),
                                &counts,
                                &items,
                                None,
                                None,
                                &warnings,
                                started.elapsed().as_millis() as u64,
                            );
                            let _ = save_campaign_summary_blocking(
                                &summary,
                                &app,
                                Some(&summary_json_path),
                            )?;
                            continue;
                        }
                    }
                }
            } else {
                emit_campaign_progress(
                    &app,
                    "optimize",
                    index + 1,
                    dataset_total,
                    &label,
                    "running optimization",
                    Some(&summary_json_path),
                    &counts,
                );
                let optimize_progress_sink = if req.preload_policy
                    == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
                    && strategy_kind == "DoubleMa"
                    && !doublema_vram_prefetch_disabled
                    && !using_preloaded_pack
                {
                    match (
                        parsed_doublema_req.as_ref().and_then(|run_req| {
                            build_doublema_vram_prefetch_config(
                                run_req,
                                learned_doublema_vram_device,
                            )
                        }),
                        preload.as_ref().map(DatasetPreloadJob::shared_state),
                    ) {
                        (Some(config), Some(preload_state)) => Some(CampaignProgressSink {
                            app: app.clone(),
                            vram_prefetch: Some(CampaignDoubleMaVramPrefetch {
                                preload_state,
                                config,
                                completed: AtomicBool::new(false),
                                feedback: Mutex::new(None),
                            }),
                        }),
                        _ => Some(CampaignProgressSink {
                            app: app.clone(),
                            vram_prefetch: None,
                        }),
                    }
                } else {
                    Some(CampaignProgressSink {
                        app: app.clone(),
                        vram_prefetch: None,
                    })
                };
                let optimize_result = match run_optimization_blocking(
                    family,
                    &run_req_value,
                    &prepared.candles,
                    cancel.as_ref(),
                    optimize_progress_sink
                        .as_ref()
                        .map(|value| value as &dyn ProgressSink),
                ) {
                    Ok(value) => value,
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            label,
                            prepared.timeframe.clone(),
                            is_holdout,
                            None,
                            "optimization",
                            error,
                        );
                        summary = build_campaign_summary(
                            &ts,
                            "running",
                            &strategy_kind,
                            timeframe_override.clone(),
                            &req,
                            execution_mode,
                            &tags,
                            holdout_tag.clone(),
                            &counts,
                            &items,
                            None,
                            None,
                            &warnings,
                            started.elapsed().as_millis() as u64,
                        );
                        let _ = save_campaign_summary_blocking(
                            &summary,
                            &app,
                            Some(&summary_json_path),
                        )?;
                        continue;
                    }
                };
                if let Some(feedback) = optimize_progress_sink
                    .as_ref()
                    .and_then(CampaignProgressSink::take_prefetch_feedback)
                {
                    match feedback {
                        DoubleMaCampaignVramPrefetchFeedback::Staged
                        | DoubleMaCampaignVramPrefetchFeedback::AlreadyResident => {}
                        DoubleMaCampaignVramPrefetchFeedback::Warning(note) => {
                            push_unique_warning(&mut warnings, note);
                        }
                        DoubleMaCampaignVramPrefetchFeedback::Fatal(note) => {
                            push_unique_warning(&mut warnings, note);
                            doublema_vram_prefetch_disabled = true;
                        }
                    }
                }
                counts.optimized = counts.optimized.saturating_add(1);
                optimize_result
            }
        } else {
            emit_campaign_progress(
                &app,
                "optimize",
                index + 1,
                dataset_total,
                &label,
                "running optimization",
                Some(&summary_json_path),
                &counts,
            );
            let optimize_progress_sink = if req.preload_policy
                == ResearchCampaignPreloadPolicy::HostPlusDoublemaVram
                && strategy_kind == "DoubleMa"
                && !doublema_vram_prefetch_disabled
                && !using_preloaded_pack
            {
                match (
                    parsed_doublema_req.as_ref().and_then(|run_req| {
                        build_doublema_vram_prefetch_config(run_req, learned_doublema_vram_device)
                    }),
                    preload.as_ref().map(DatasetPreloadJob::shared_state),
                ) {
                    (Some(config), Some(preload_state)) => Some(CampaignProgressSink {
                        app: app.clone(),
                        vram_prefetch: Some(CampaignDoubleMaVramPrefetch {
                            preload_state,
                            config,
                            completed: AtomicBool::new(false),
                            feedback: Mutex::new(None),
                        }),
                    }),
                    _ => Some(CampaignProgressSink {
                        app: app.clone(),
                        vram_prefetch: None,
                    }),
                }
            } else {
                Some(CampaignProgressSink {
                    app: app.clone(),
                    vram_prefetch: None,
                })
            };
            let optimize_result = match run_optimization_blocking(
                family,
                &run_req_value,
                &prepared.candles,
                cancel.as_ref(),
                optimize_progress_sink
                    .as_ref()
                    .map(|value| value as &dyn ProgressSink),
            ) {
                Ok(value) => value,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        label,
                        prepared.timeframe.clone(),
                        is_holdout,
                        None,
                        "optimization",
                        error,
                    );
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ = save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                    continue;
                }
            };
            if let Some(feedback) = optimize_progress_sink
                .as_ref()
                .and_then(CampaignProgressSink::take_prefetch_feedback)
            {
                match feedback {
                    DoubleMaCampaignVramPrefetchFeedback::Staged
                    | DoubleMaCampaignVramPrefetchFeedback::AlreadyResident => {}
                    DoubleMaCampaignVramPrefetchFeedback::Warning(note) => {
                        push_unique_warning(&mut warnings, note);
                    }
                    DoubleMaCampaignVramPrefetchFeedback::Fatal(note) => {
                        push_unique_warning(&mut warnings, note);
                        doublema_vram_prefetch_disabled = true;
                    }
                }
            }
            counts.optimized = counts.optimized.saturating_add(1);
            optimize_result
        };
        if strategy_kind == "DoubleMa" {
            learned_doublema_vram_device = match (&optimize_result, parsed_doublema_req.as_ref()) {
                (CampaignOptimizationResult::Backend(res), Some(run_req))
                    if supports_exact_gpu_kernel_route(res) =>
                {
                    requested_gpu_device_id(&run_req.backend)
                }
                _ => None,
            };
        }
        let run_res_value = match &optimize_result {
            CampaignOptimizationResult::Backend(res) => {
                serde_json::to_value(res).map_err(|e| e.to_string())?
            }
            CampaignOptimizationResult::CompositeMo(res) => {
                serde_json::to_value(res).map_err(|e| e.to_string())?
            }
            CampaignOptimizationResult::CompositeMoArtifact(res) => res.clone(),
        };

        let mut artifact_path = if let Some(path) = existing_optimize_path {
            path
        } else {
            let optimize_artifact = build_run_artifact_value(
                &run_ts,
                &run_only_fingerprint,
                &prepared.input.csv_path,
                &prepared.dataset_meta,
                &per_run_tags,
                &run_req_value,
                &run_res_value,
                None,
                None,
            );
            match run_artifacts::save_run_artifact_blocking(optimize_artifact, app.clone()) {
                Ok(saved) => saved.path,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        prepared.label.clone(),
                        prepared.timeframe.clone(),
                        is_holdout,
                        None,
                        "optimization artifact",
                        error,
                    );
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ =
                        save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                    continue;
                }
            }
        };

        if execution_mode == ResearchCampaignExecutionMode::OptimizeOnly {
            items[index] = ResearchCampaignItemSummary {
                csv_path: prepared.input.csv_path.clone(),
                label,
                timeframe: prepared.timeframe.clone(),
                is_holdout,
                status: "optimized".to_string(),
                artifact_path: Some(artifact_path),
                report_xlsx_path: None,
                error: None,
            };
            summary = build_campaign_summary(
                &ts,
                "running",
                &strategy_kind,
                timeframe_override.clone(),
                &req,
                execution_mode,
                &tags,
                holdout_tag.clone(),
                &counts,
                &items,
                None,
                None,
                &warnings,
                started.elapsed().as_millis() as u64,
            );
            let _ = save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
            continue;
        }

        let validation_req_value = match build_validation_request_value(
            &strategy_kind,
            &data_id,
            &run_req_value,
            &optimize_result,
            &prepared.dataset_meta,
            &req.validation_config,
            family,
        ) {
            Ok(value) => value,
            Err(error) => {
                counts.failed = counts.failed.saturating_add(1);
                items[index] = failed_item(
                    prepared.input.csv_path.clone(),
                    prepared.label.clone(),
                    prepared.timeframe.clone(),
                    is_holdout,
                    Some(artifact_path.clone()),
                    "validation request",
                    error,
                );
                summary = build_campaign_summary(
                    &ts,
                    "running",
                    &strategy_kind,
                    timeframe_override.clone(),
                    &req,
                    execution_mode,
                    &tags,
                    holdout_tag.clone(),
                    &counts,
                    &items,
                    None,
                    None,
                    &warnings,
                    started.elapsed().as_millis() as u64,
                );
                let _ = save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                continue;
            }
        };
        let validation_fingerprint = run_artifacts::compute_config_fingerprint_from_artifact(
            &artifact_stub(&run_req_value, Some(&validation_req_value)),
        );
        if req.skip_existing
            && existing_validation_matches(
                &existing,
                &strategy_kind,
                &validation_fingerprint,
                &primary_tag,
                &prepared,
            )
        {
            counts.skipped = counts.skipped.saturating_add(1);
            items[index] = ResearchCampaignItemSummary {
                csv_path: prepared.input.csv_path.clone(),
                label,
                timeframe: prepared.timeframe.clone(),
                is_holdout,
                status: "skipped".to_string(),
                artifact_path: Some(artifact_path),
                report_xlsx_path: None,
                error: None,
            };
            summary = build_campaign_summary(
                &ts,
                "running",
                &strategy_kind,
                timeframe_override.clone(),
                &req,
                execution_mode,
                &tags,
                holdout_tag.clone(),
                &counts,
                &items,
                None,
                None,
                &warnings,
                started.elapsed().as_millis() as u64,
            );
            let _ = save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
            continue;
        }

        emit_campaign_progress(
            &app,
            "validate",
            index + 1,
            dataset_total,
            &prepared.label,
            "running validation",
            Some(&summary_json_path),
            &counts,
        );
        let validation_res_value = match family {
            CampaignOptimizationFamily::DoubleLike => {
                let validation_req: DoubleMaValidationRequest =
                    match serde_json::from_value(validation_req_value.clone())
                        .map_err(|e| e.to_string())
                    {
                        Ok(req) => req,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                prepared.label.clone(),
                                prepared.timeframe.clone(),
                                is_holdout,
                                Some(artifact_path.clone()),
                                "validation request decode",
                                error,
                            );
                            summary = build_campaign_summary(
                                &ts,
                                "running",
                                &strategy_kind,
                                timeframe_override.clone(),
                                &req,
                                execution_mode,
                                &tags,
                                holdout_tag.clone(),
                                &counts,
                                &items,
                                None,
                                None,
                                &warnings,
                                started.elapsed().as_millis() as u64,
                            );
                            let _ = save_campaign_summary_blocking(
                                &summary,
                                &app,
                                Some(&summary_json_path),
                            )?;
                            continue;
                        }
                    };
                match ta_app_core::run_double_ma_validation_blocking_with_candles(
                    validation_req,
                    &prepared.candles,
                    cancel.as_ref(),
                    Some(&sink),
                ) {
                    Ok(validation_res) => {
                        match serde_json::to_value(validation_res).map_err(|e| e.to_string()) {
                            Ok(value) => value,
                            Err(error) => {
                                counts.failed = counts.failed.saturating_add(1);
                                items[index] = failed_item(
                                    prepared.input.csv_path.clone(),
                                    prepared.label.clone(),
                                    prepared.timeframe.clone(),
                                    is_holdout,
                                    Some(artifact_path.clone()),
                                    "validation encode",
                                    error,
                                );
                                summary = build_campaign_summary(
                                    &ts,
                                    "running",
                                    &strategy_kind,
                                    timeframe_override.clone(),
                                    &req,
                                    execution_mode,
                                    &tags,
                                    holdout_tag.clone(),
                                    &counts,
                                    &items,
                                    None,
                                    None,
                                    &warnings,
                                    started.elapsed().as_millis() as u64,
                                );
                                let _ = save_campaign_summary_blocking(
                                    &summary,
                                    &app,
                                    Some(&summary_json_path),
                                )?;
                                continue;
                            }
                        }
                    }
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            prepared.label.clone(),
                            prepared.timeframe.clone(),
                            is_holdout,
                            Some(artifact_path.clone()),
                            "validation",
                            error,
                        );
                        summary = build_campaign_summary(
                            &ts,
                            "running",
                            &strategy_kind,
                            timeframe_override.clone(),
                            &req,
                            execution_mode,
                            &tags,
                            holdout_tag.clone(),
                            &counts,
                            &items,
                            None,
                            None,
                            &warnings,
                            started.elapsed().as_millis() as u64,
                        );
                        let _ = save_campaign_summary_blocking(
                            &summary,
                            &app,
                            Some(&summary_json_path),
                        )?;
                        continue;
                    }
                }
            }
            CampaignOptimizationFamily::CompositeCpu | CampaignOptimizationFamily::CompositeMo => {
                let validation_req: CompositeValidationRequest =
                    match serde_json::from_value(validation_req_value.clone())
                        .map_err(|e| e.to_string())
                    {
                        Ok(req) => req,
                        Err(error) => {
                            counts.failed = counts.failed.saturating_add(1);
                            items[index] = failed_item(
                                prepared.input.csv_path.clone(),
                                prepared.label.clone(),
                                prepared.timeframe.clone(),
                                is_holdout,
                                Some(artifact_path.clone()),
                                "validation request decode",
                                error,
                            );
                            summary = build_campaign_summary(
                                &ts,
                                "running",
                                &strategy_kind,
                                timeframe_override.clone(),
                                &req,
                                execution_mode,
                                &tags,
                                holdout_tag.clone(),
                                &counts,
                                &items,
                                None,
                                None,
                                &warnings,
                                started.elapsed().as_millis() as u64,
                            );
                            let _ = save_campaign_summary_blocking(
                                &summary,
                                &app,
                                Some(&summary_json_path),
                            )?;
                            continue;
                        }
                    };
                match ta_app_core::run_composite_validation_blocking_with_candles(
                    validation_req,
                    &prepared.candles,
                    cancel.as_ref(),
                    Some(&sink),
                ) {
                    Ok(validation_res) => {
                        match serde_json::to_value(validation_res).map_err(|e| e.to_string()) {
                            Ok(value) => value,
                            Err(error) => {
                                counts.failed = counts.failed.saturating_add(1);
                                items[index] = failed_item(
                                    prepared.input.csv_path.clone(),
                                    prepared.label.clone(),
                                    prepared.timeframe.clone(),
                                    is_holdout,
                                    Some(artifact_path.clone()),
                                    "validation encode",
                                    error,
                                );
                                summary = build_campaign_summary(
                                    &ts,
                                    "running",
                                    &strategy_kind,
                                    timeframe_override.clone(),
                                    &req,
                                    execution_mode,
                                    &tags,
                                    holdout_tag.clone(),
                                    &counts,
                                    &items,
                                    None,
                                    None,
                                    &warnings,
                                    started.elapsed().as_millis() as u64,
                                );
                                let _ = save_campaign_summary_blocking(
                                    &summary,
                                    &app,
                                    Some(&summary_json_path),
                                )?;
                                continue;
                            }
                        }
                    }
                    Err(error) => {
                        counts.failed = counts.failed.saturating_add(1);
                        items[index] = failed_item(
                            prepared.input.csv_path.clone(),
                            prepared.label.clone(),
                            prepared.timeframe.clone(),
                            is_holdout,
                            Some(artifact_path.clone()),
                            "validation",
                            error,
                        );
                        summary = build_campaign_summary(
                            &ts,
                            "running",
                            &strategy_kind,
                            timeframe_override.clone(),
                            &req,
                            execution_mode,
                            &tags,
                            holdout_tag.clone(),
                            &counts,
                            &items,
                            None,
                            None,
                            &warnings,
                            started.elapsed().as_millis() as u64,
                        );
                        let _ = save_campaign_summary_blocking(
                            &summary,
                            &app,
                            Some(&summary_json_path),
                        )?;
                        continue;
                    }
                }
            }
        };
        counts.validated = counts.validated.saturating_add(1);
        let validation_artifact = build_run_artifact_value(
            &run_ts,
            &validation_fingerprint,
            &prepared.input.csv_path,
            &prepared.dataset_meta,
            &per_run_tags,
            &run_req_value,
            &run_res_value,
            Some(&validation_req_value),
            Some(&validation_res_value),
        );
        artifact_path =
            match run_artifacts::save_run_artifact_blocking(validation_artifact, app.clone()) {
                Ok(saved) => saved.path,
                Err(error) => {
                    counts.failed = counts.failed.saturating_add(1);
                    items[index] = failed_item(
                        prepared.input.csv_path.clone(),
                        prepared.label.clone(),
                        prepared.timeframe.clone(),
                        is_holdout,
                        Some(artifact_path.clone()),
                        "validation artifact",
                        error,
                    );
                    summary = build_campaign_summary(
                        &ts,
                        "running",
                        &strategy_kind,
                        timeframe_override.clone(),
                        &req,
                        execution_mode,
                        &tags,
                        holdout_tag.clone(),
                        &counts,
                        &items,
                        None,
                        None,
                        &warnings,
                        started.elapsed().as_millis() as u64,
                    );
                    let _ =
                        save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
                    continue;
                }
            };
        items[index] = ResearchCampaignItemSummary {
            csv_path: prepared.input.csv_path.clone(),
            label,
            timeframe: prepared.timeframe.clone(),
            is_holdout,
            status: "ok".to_string(),
            artifact_path: Some(artifact_path),
            report_xlsx_path: None,
            error: None,
        };
        summary = build_campaign_summary(
            &ts,
            "running",
            &strategy_kind,
            timeframe_override.clone(),
            &req,
            execution_mode,
            &tags,
            holdout_tag.clone(),
            &counts,
            &items,
            None,
            None,
            &warnings,
            started.elapsed().as_millis() as u64,
        );
        let _ = save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
    }

    let mut meta_response = None;
    let mut stage5_response = None;
    if execution_mode == ResearchCampaignExecutionMode::OptimizeOnly {
        if req.auto_meta || req.auto_stage5 {
            warnings.push(
                "execution_mode=optimize_only ignores auto_meta and auto_stage5 follow-up stages."
                    .to_string(),
            );
        }
    } else if !counts.cancelled && req.auto_meta {
        if let Some(meta_req) = req.meta_request.clone() {
            emit_campaign_progress(
                &app,
                "meta",
                counts.total,
                counts.total,
                &strategy_kind,
                "running meta selection",
                Some(&summary_json_path),
                &counts,
            );
            match run_meta_selection_blocking(meta_req, app.clone(), cancel.clone()) {
                Ok(response) => {
                    if req.auto_stage5 {
                        if let Some(mut stage5_req) = req.stage5_request.clone() {
                            if stage5_req.source_meta_path.is_none() {
                                stage5_req.source_meta_path = Some(response.meta_json_path.clone());
                            }
                            emit_campaign_progress(
                                &app,
                                "stage5",
                                counts.total,
                                counts.total,
                                &strategy_kind,
                                "running stage5",
                                Some(&summary_json_path),
                                &counts,
                            );
                            match run_meta_stage4_experimental_blocking(stage5_req, app.clone()) {
                                Ok(stage5) => {
                                    stage5_response = Some(stage5);
                                }
                                Err(error) => {
                                    warnings.push(format!("stage5: {error}"));
                                }
                            }
                        }
                    }
                    meta_response = Some(response);
                }
                Err(error) => {
                    warnings.push(format!("meta: {error}"));
                }
            }
        } else {
            warnings.push("auto_meta was requested without a meta_request payload".to_string());
        }
    }

    let final_state = if counts.cancelled {
        "cancelled"
    } else if warnings
        .iter()
        .any(|entry| entry.starts_with("meta:") || entry.starts_with("stage5:"))
    {
        "completed_with_warnings"
    } else if counts.failed > 0 {
        "completed_with_failures"
    } else {
        "completed"
    };
    summary = build_campaign_summary(
        &ts,
        final_state,
        &strategy_kind,
        timeframe_override,
        &req,
        execution_mode,
        &tags,
        holdout_tag,
        &counts,
        &items,
        meta_response.as_ref(),
        stage5_response.as_ref(),
        &warnings,
        started.elapsed().as_millis() as u64,
    );
    let (_, summary_json_bytes) =
        save_campaign_summary_blocking(&summary, &app, Some(&summary_json_path))?;
    Ok(RunResearchCampaignResponse {
        ts,
        summary_json_path,
        summary_json_bytes,
        summary,
        meta: meta_response,
        stage5: stage5_response,
    })
}

fn run_research_campaign_blocking(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    let execution_mode = req.execution_mode;
    run_research_campaign_blocking_with_mode(
        req,
        app,
        cancel,
        execution_mode,
        None,
    )
}

pub fn run_research_campaign_blocking_headless(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    run_research_campaign_blocking(req, app, cancel)
}

pub fn run_research_campaign_optimize_only_blocking_headless(
    req: ResearchCampaignRequest,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    run_research_campaign_blocking_with_mode(
        req,
        app,
        cancel,
        ResearchCampaignExecutionMode::OptimizeOnly,
        None,
    )
}

pub fn preload_research_campaign_dataset_pack_headless(
    req: &ResearchCampaignRequest,
    app: &tauri::AppHandle,
) -> Result<HeadlessPreparedResearchCampaignDatasetPack, String> {
    if req.datasets.is_empty() {
        return Err("campaign requires at least one dataset".to_string());
    }
    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = data_cache::app_data_cache_dir(&base);
    let timeframe_override = req
        .timeframe_override
        .as_deref()
        .map(normalize_timeframe_label)
        .filter(|value| !value.is_empty());
    let mut prepared = VecDeque::with_capacity(req.datasets.len());
    for dataset in req.datasets.iter().cloned() {
        prepared.push_back(prepare_campaign_dataset(
            dataset,
            timeframe_override.as_deref(),
            &app_cache_dir,
        )?);
    }
    Ok(HeadlessPreparedResearchCampaignDatasetPack { datasets: prepared })
}

pub fn run_research_campaign_optimize_only_preloaded_pack_blocking_headless(
    req: ResearchCampaignRequest,
    preloaded_pack: HeadlessPreparedResearchCampaignDatasetPack,
    app: tauri::AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<RunResearchCampaignResponse, String> {
    run_research_campaign_blocking_with_mode(
        req,
        app,
        cancel,
        ResearchCampaignExecutionMode::OptimizeOnly,
        Some(preloaded_pack),
    )
}

#[tauri::command]
pub async fn run_research_campaign(
    req: ResearchCampaignRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunResearchCampaignResponse, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    tauri::async_runtime::spawn_blocking(move || run_research_campaign_blocking(req, app, cancel))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{
        build_composite_candidate_value, build_doublema_vram_prefetch_config,
        candidate_values_from_optimization,
        can_reuse_optimization_artifact_for_validation, is_double_like_kind,
        normalize_backend_value, optimization_family_for_kind,
        optimization_result_from_artifact_value, push_unique_warning, requested_gpu_device_id,
        resolve_campaign_strategy_kind, Backend, CampaignDoubleMaVramPrefetchConfig,
        CampaignOptimizationFamily, CampaignOptimizationResult, DatasetPreloadJob, DoubleMaRequest,
    };
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc};
    use ta_app_core::double_ma::GpuExecutionPolicy;
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;

    const SUPPORTED_CAMPAIGN_STRATEGIES: &[&str] = &[
        "DoubleMa",
        "Pit",
        "Set",
        "Mts",
        "Orc",
        "Marrt",
        "Vfct",
        "Rocb",
        "Vcet",
        "Mkcs",
        "Kqsr",
        "Carm",
        "Kvdf",
        "Cdmc",
        "Asr",
        "Vcc",
        "Vemr",
        "CompositeCpu",
        "CompositeMo",
        "CompositeTrend",
        "RegimeHybrid",
        "BreakoutMomentum",
    ];

    #[test]
    fn dynamic_double_ma_is_rejected_for_campaigns() {
        let err = resolve_campaign_strategy_kind("DynamicDoubleMa").unwrap_err();
        assert!(
            err.contains("do not support DynamicDoubleMa"),
            "unexpected error: {err}"
        );
        assert!(!is_double_like_kind("DynamicDoubleMa"));
    }

    #[test]
    fn ml_signal_is_rejected_for_campaigns() {
        let err = resolve_campaign_strategy_kind("MlSignal").unwrap_err();
        assert!(
            err.contains("do not support MlSignal"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn unknown_strategy_is_rejected_for_campaigns() {
        let err = resolve_campaign_strategy_kind("UnknownFutureStrategy").unwrap_err();
        assert!(err.contains("Unsupported research campaign strategy kind"));
    }

    #[test]
    fn supported_campaign_strategy_matrix_is_accepted() {
        for kind in SUPPORTED_CAMPAIGN_STRATEGIES {
            assert_eq!(
                resolve_campaign_strategy_kind(kind).as_deref(),
                Ok(*kind),
                "expected {kind} to stay supported by campaign orchestration"
            );
        }
    }

    #[test]
    fn campaign_strategy_matrix_maps_to_expected_optimization_families() {
        for kind in [
            "DoubleMa",
            "Pit",
            "Set",
            "Mts",
            "Orc",
            "Marrt",
            "Vfct",
            "CompositeTrend",
            "RegimeHybrid",
            "BreakoutMomentum",
        ] {
            assert_eq!(
                optimization_family_for_kind(kind),
                CampaignOptimizationFamily::DoubleLike,
                "expected {kind} to stay on the double-like campaign optimization path"
            );
        }

        for kind in [
            "CompositeCpu",
            "Asr",
            "Vcc",
            "Vemr",
            "Rocb",
            "Vcet",
            "Mkcs",
            "Kqsr",
            "Carm",
            "Kvdf",
            "Cdmc",
        ] {
            assert_eq!(
                optimization_family_for_kind(kind),
                CampaignOptimizationFamily::CompositeCpu,
                "expected {kind} to stay on the composite CPU campaign optimization path"
            );
        }

        assert_eq!(
            optimization_family_for_kind("CompositeMo"),
            CampaignOptimizationFamily::CompositeMo
        );
    }

    #[test]
    fn optimization_artifact_reuse_policy_supports_all_campaign_families() {
        assert!(can_reuse_optimization_artifact_for_validation(
            CampaignOptimizationFamily::DoubleLike
        ));
        assert!(can_reuse_optimization_artifact_for_validation(
            CampaignOptimizationFamily::CompositeCpu
        ));
        assert!(can_reuse_optimization_artifact_for_validation(
            CampaignOptimizationFamily::CompositeMo
        ));
    }

    #[test]
    fn composite_mo_artifact_reuse_reads_pareto_candidates_from_raw_json() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo",
                    "strategy": { "fee_bps": 1.5 }
                },
                "res": {
                    "data_id": "campaign_1",
                    "pareto": [
                        {
                            "params": {
                                "fast_ma_type": "alma",
                                "slow_ma_type": "alma",
                                "fast_len": 12,
                                "slow_len": 48,
                                "strategy": { "fee_bps": 1.5 }
                            }
                        }
                    ],
                    "total_evaluations": 1,
                    "runtime_ms": 10,
                    "telemetry": {
                        "prescan": null,
                        "search": {
                            "sampler": "ExactGrid",
                            "requested_evaluations": 1,
                            "actual_evaluations": 1,
                            "unique_evaluations": 1,
                            "duplicate_evaluations_avoided": 0,
                            "feasible_points": 1,
                            "rejected_decode_or_order": 0,
                            "rejected_prescan_pair": 0,
                            "rejected_other_constraints": 0,
                            "exact_grid_used": true,
                            "exact_grid_total_combinations": 1
                        }
                    }
                }
            }
        });
        let run_req = artifact.pointer("/run/req").cloned().expect("run.req");
        let optimization = optimization_result_from_artifact_value(
            CampaignOptimizationFamily::CompositeMo,
            &artifact,
        )
        .expect("decode CompositeMo artifact");
        assert!(matches!(
            optimization,
            CampaignOptimizationResult::CompositeMoArtifact(_)
        ));

        let candidates = candidate_values_from_optimization(
            "CompositeMo",
            CampaignOptimizationFamily::CompositeMo,
            &run_req,
            &optimization,
            8,
        )
        .expect("candidates from raw artifact");
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].get("fast_ma_type").and_then(Value::as_str),
            Some("alma")
        );
        assert_eq!(
            candidates[0].get("slow_len").and_then(Value::as_u64),
            Some(48)
        );
    }

    #[test]
    fn composite_campaign_validation_defaults_missing_backend_to_auto() {
        assert_eq!(
            normalize_backend_value(None, "CompositeMo"),
            json!({ "backend": "Auto", "device_id": 0 })
        );
    }

    #[test]
    fn double_like_campaign_validation_defaults_missing_backend_to_cpu_only() {
        assert_eq!(
            normalize_backend_value(None, "DoubleMa"),
            json!({ "backend": "CpuOnly" })
        );
    }

    #[test]
    fn campaign_validation_backend_normalization_preserves_or_collapses_by_family() {
        let gpu_kernel = json!({ "backend": "GpuKernel", "device_id": 7 });
        let gpu_hybrid = json!({ "backend": "GpuHybridKernel", "device_id": 5 });
        let cpu_only = json!({ "backend": "CpuOnly" });

        for kind in [
            "DoubleMa",
            "Pit",
            "Set",
            "Mts",
            "Orc",
            "Marrt",
            "Vfct",
            "CompositeTrend",
            "RegimeHybrid",
            "BreakoutMomentum",
        ] {
            assert_eq!(
                normalize_backend_value(Some(&gpu_kernel), kind),
                gpu_kernel,
                "expected {kind} to preserve kernel-capable validation backend requests"
            );
            assert_eq!(
                normalize_backend_value(Some(&gpu_hybrid), kind),
                gpu_hybrid,
                "expected {kind} to preserve hybrid validation backend requests"
            );
        }

        for kind in [
            "CompositeCpu",
            "Asr",
            "Vcc",
            "Vemr",
            "Rocb",
            "Vcet",
            "Mkcs",
            "Kqsr",
            "Carm",
            "Kvdf",
            "Cdmc",
            "CompositeMo",
        ] {
            assert_eq!(
                normalize_backend_value(Some(&gpu_kernel), kind),
                json!({ "backend": "Auto", "device_id": 7 }),
                "expected {kind} to collapse GPU kernel requests to Auto for composite-family validation"
            );
            assert_eq!(
                normalize_backend_value(Some(&gpu_hybrid), kind),
                json!({ "backend": "Auto", "device_id": 5 }),
                "expected {kind} to collapse GPU hybrid requests to Auto for composite-family validation"
            );
            assert_eq!(
                normalize_backend_value(Some(&cpu_only), kind),
                cpu_only,
                "expected {kind} to preserve CpuOnly requests"
            );
        }
    }

    fn make_plain_doublema_request(backend: Backend) -> DoubleMaRequest {
        DoubleMaRequest {
            backend,
            data_id: "campaign-test".to_string(),
            fast_range: (5, 5, 1),
            slow_range: (20, 20, 1),
            fast_ma_types: vec!["sma".to_string()],
            slow_ma_types: vec!["ema".to_string()],
            ma_source: "hl2".to_string(),
            export_all_csv_path: None,
            fast_ma_params: None,
            slow_ma_params: None,
            fast_ma_param_sweeps: None,
            slow_ma_param_sweeps: None,
            strategy: StrategyConfig::default(),
            halftrend_filter: None,
            objective: ObjectiveKind::Sharpe,
            score_mode: None,
            hurdle_tau: None,
            extra_shortlist_objectives: None,
            shortlist_confidence_mode: None,
            shortlist_union_mode: None,
            shortlist_slice_mode: None,
            shortlist_slice_benchmark: None,
            shortlist_neighborhood_radius: None,
            shortlist_neighborhood_step: None,
            mode: OptimizationMode::Grid,
            top_k: Some(8),
            include_all: Some(false),
            heatmap_bins: None,
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
        }
    }

    #[test]
    fn doublema_vram_prefetch_config_accepts_exact_kernel_and_learned_auto() {
        let exact = make_plain_doublema_request(Backend::GpuKernel { device_id: 7 });
        assert_eq!(
            build_doublema_vram_prefetch_config(&exact, None).map(
                |CampaignDoubleMaVramPrefetchConfig {
                     device_id,
                     fast_ma_type,
                     slow_ma_type,
                     ma_source,
                 }| (device_id, fast_ma_type, slow_ma_type, ma_source)
            ),
            Some((7, "sma".to_string(), "ema".to_string(), "hl2".to_string()))
        );

        let auto = make_plain_doublema_request(Backend::Auto { device_id: 99 });
        assert_eq!(build_doublema_vram_prefetch_config(&auto, None), None);
        assert_eq!(
            build_doublema_vram_prefetch_config(&auto, Some(3))
                .map(|CampaignDoubleMaVramPrefetchConfig { device_id, .. }| device_id),
            Some(3)
        );
    }

    #[test]
    fn doublema_vram_prefetch_config_rejects_non_plain_shapes() {
        let mut multi_ma = make_plain_doublema_request(Backend::GpuKernel { device_id: 0 });
        multi_ma.fast_ma_types.push("alma".to_string());
        assert_eq!(build_doublema_vram_prefetch_config(&multi_ma, None), None);

        let cpu = make_plain_doublema_request(Backend::CpuOnly);
        assert_eq!(build_doublema_vram_prefetch_config(&cpu, Some(0)), None);
    }

    #[test]
    fn requested_gpu_device_id_tracks_gpu_capable_backends_only() {
        assert_eq!(requested_gpu_device_id(&Backend::CpuOnly), None);
        assert_eq!(
            requested_gpu_device_id(&Backend::Auto { device_id: 2 }),
            Some(2)
        );
        assert_eq!(
            requested_gpu_device_id(&Backend::GpuKernel { device_id: 4 }),
            Some(4)
        );
    }

    #[test]
    fn push_unique_warning_deduplicates_identical_entries() {
        let mut warnings = vec!["existing".to_string()];
        push_unique_warning(&mut warnings, "existing".to_string());
        push_unique_warning(&mut warnings, String::new());
        push_unique_warning(&mut warnings, "new".to_string());
        assert_eq!(warnings, vec!["existing".to_string(), "new".to_string()]);
    }

    #[test]
    fn build_composite_candidate_value_preserves_strategy_specific_payloads() {
        let params = serde_json::Map::from_iter([
            ("fast_ma_params".to_string(), json!({ "alpha": 2.0 })),
            ("slow_ma_params".to_string(), json!({ "offset": 0.85 })),
        ]);
        let base_strategy = json!({ "fee_bps": 1.5 });
        let run_req = json!({
            "strategy": base_strategy,
            "halftrend": { "amplitude": 2 },
            "macz": { "fast_length": 12 },
            "ott": { "period": 9 },
            "signal_mode": "soft_vote",
            "asr": { "atr_period": 21 },
            "vcc": { "trend_period": 6 },
            "vemr": { "mfi_period": 14 },
            "rocb": { "roc_period": 9 },
            "vcet": { "vol_ema_period": 18 },
            "mkcs": { "breakout_lookback": 30 },
            "kqsr": { "prb_period": 24 },
            "carm": { "otto_period": 16 },
            "kvdf": { "fvg_lookback": 11 },
            "cdmc": { "decycler_period": 34 }
        });

        for (kind, expected_key, expected_value) in [
            ("Asr", "asr", json!({ "atr_period": 21 })),
            ("Vcc", "vcc", json!({ "trend_period": 6 })),
            ("Vemr", "vemr", json!({ "mfi_period": 14 })),
            ("Rocb", "rocb", json!({ "roc_period": 9 })),
            ("Vcet", "vcet", json!({ "vol_ema_period": 18 })),
            ("Mkcs", "mkcs", json!({ "breakout_lookback": 30 })),
            ("Kqsr", "kqsr", json!({ "prb_period": 24 })),
            ("Carm", "carm", json!({ "otto_period": 16 })),
            ("Kvdf", "kvdf", json!({ "fvg_lookback": 11 })),
            ("Cdmc", "cdmc", json!({ "decycler_period": 34 })),
        ] {
            let candidate = build_composite_candidate_value(
                kind,
                &run_req,
                &params,
                "ema".to_string(),
                "sma".to_string(),
                12,
                48,
            );
            assert_eq!(candidate.get(expected_key), Some(&expected_value));
            assert_eq!(candidate.get("strategy"), Some(&base_strategy));
            assert_eq!(
                candidate.get("fast_ma_params"),
                Some(&json!({ "alpha": 2.0 }))
            );
            assert_eq!(
                candidate.get("slow_ma_params"),
                Some(&json!({ "offset": 0.85 }))
            );
        }

        let composite_cpu_candidate = build_composite_candidate_value(
            "CompositeCpu",
            &run_req,
            &params,
            "ema".to_string(),
            "sma".to_string(),
            12,
            48,
        );
        assert_eq!(
            composite_cpu_candidate.get("halftrend"),
            Some(&json!({ "amplitude": 2 }))
        );
        assert_eq!(
            composite_cpu_candidate.get("macz"),
            Some(&json!({ "fast_length": 12 }))
        );
        assert_eq!(
            composite_cpu_candidate.get("ott"),
            Some(&json!({ "period": 9 }))
        );
        assert_eq!(
            composite_cpu_candidate.get("signal_mode"),
            Some(&Value::String("soft_vote".to_string()))
        );
    }

    #[test]
    fn dataset_preload_job_drop_waits_for_worker_completion() {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (drop_done_tx, drop_done_rx) = mpsc::channel();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_worker = Arc::clone(&finished);
        let job = DatasetPreloadJob::spawn_with_task(move || {
            let _ = ready_tx.send(());
            let _ = release_rx.recv();
            finished_worker.store(true, Ordering::Relaxed);
            Err("cancelled for test".to_string())
        });

        ready_rx.recv().expect("worker should start");

        let drop_thread = std::thread::spawn(move || {
            drop(job);
            let _ = drop_done_tx.send(());
        });

        assert!(
            drop_done_rx.try_recv().is_err(),
            "dropping the preload job should wait for the worker to finish"
        );
        assert!(
            !finished.load(Ordering::Relaxed),
            "worker should still be blocked before release"
        );

        release_tx.send(()).expect("should unblock worker");
        drop_thread.join().expect("drop thread should join cleanly");
        drop_done_rx
            .recv()
            .expect("drop completion signal should arrive");
        assert!(
            finished.load(Ordering::Relaxed),
            "worker should complete before the preload job is dropped"
        );
    }
}
