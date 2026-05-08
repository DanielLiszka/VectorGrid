use super::validation_common::CrossTickerCandidate;
use super::{
    validation_common, CompositeValidationRequest, CompositeValidationResult,
    DoubleMaValidationRequest, DoubleMaValidationResult, OffsetProgressSink, TauriProgressSink,
};
use crate::state::AppState;
use crate::state::LoadedDataContext;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tauri::Manager;

const PRESERVED_FINALISTS_CROSS_TICKER_LIMIT: usize = 4;

trait CrossTickerValidationResultExt {
    type Candidate: validation_common::CrossTickerCandidate;

    fn candidates(&self) -> &Vec<Self::Candidate>;
    fn candidates_mut(&mut self) -> &mut Vec<Self::Candidate>;
    fn set_cross_ticker_summary(&mut self, summary: ta_app_core::CrossTickerValidationSummary);
}

fn slice_candles(
    candles: &vector_ta::utilities::data_loader::Candles,
    start: usize,
    end: usize,
) -> vector_ta::utilities::data_loader::Candles {
    let end = end.min(candles.close.len());
    let start = start.min(end);
    vector_ta::utilities::data_loader::Candles::new_with_fields(
        candles.timestamp[start..end].to_vec(),
        candles.open[start..end].to_vec(),
        candles.high[start..end].to_vec(),
        candles.low[start..end].to_vec(),
        candles.close[start..end].to_vec(),
        candles.volume[start..end].to_vec(),
        vector_ta::utilities::data_loader::CandleFieldFlags {
            open: candles.fields.open,
            high: candles.fields.high,
            low: candles.fields.low,
            close: candles.fields.close,
            volume: candles.fields.volume,
        },
    )
}

enum AlignedCandles<'a> {
    Borrowed(&'a vector_ta::utilities::data_loader::Candles),
    Owned(vector_ta::utilities::data_loader::Candles),
}

impl<'a> AlignedCandles<'a> {
    fn as_ref(&self) -> &vector_ta::utilities::data_loader::Candles {
        match self {
            Self::Borrowed(candles) => candles,
            Self::Owned(candles) => candles,
        }
    }
}

fn align_cross_ticker_candles_to_base_horizon<'a>(
    base: &'a vector_ta::utilities::data_loader::Candles,
    cross: &'a vector_ta::utilities::data_loader::Candles,
) -> AlignedCandles<'a> {
    let base_len = base.close.len();
    let cross_len = cross.close.len();
    if base_len == 0 || cross_len == 0 {
        return AlignedCandles::Borrowed(cross);
    }

    let base_start = *base.timestamp.first().unwrap_or(&0);
    let base_end = *base.timestamp.last().unwrap_or(&0);
    let start = cross.timestamp.partition_point(|&ts| ts < base_start);
    let end = cross.timestamp.partition_point(|&ts| ts <= base_end);
    if start < end {
        if start == 0 && end == cross_len {
            return AlignedCandles::Borrowed(cross);
        }
        return AlignedCandles::Owned(slice_candles(cross, start, end));
    }

    if cross_len > base_len {
        return AlignedCandles::Owned(slice_candles(
            cross,
            cross_len.saturating_sub(base_len),
            cross_len,
        ));
    }

    AlignedCandles::Borrowed(cross)
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

fn normalize_strategy_kind(raw: &str) -> String {
    match raw.trim() {
        "" => String::new(),
        "DoubleMa" => "DoubleMa".to_string(),
        "DynamicDoubleMa" => "DynamicDoubleMa".to_string(),
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

fn required_validation_context_text<'a>(
    raw: Option<&'a str>,
    field_name: &str,
) -> Result<&'a str, String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("validation_context.{field_name} is required"))
}

fn canonical_validation_context_path_key(raw: &str) -> String {
    crate::local_path_policy::canonical_existing_path(raw)
        .map(|validated| {
            validation_common::normalize_path_key(validated.canonical.to_string_lossy().as_ref())
        })
        .unwrap_or_else(|_| validation_common::normalize_path_key(raw))
}

fn validate_validation_context(
    ctx: Option<&ta_app_core::ValidationArtifactContext>,
    loaded: &LoadedDataContext,
    allowed_strategy_kinds: &[&str],
) -> Result<(), String> {
    let Some(ctx) = ctx else {
        return Err("validation_context is required".to_string());
    };

    let strategy_kind = normalize_strategy_kind(&ctx.strategy_kind);
    if !allowed_strategy_kinds
        .iter()
        .any(|kind| normalize_strategy_kind(kind) == strategy_kind)
    {
        return Err(format!(
            "validation strategy_kind '{}' is incompatible with this validation command",
            ctx.strategy_kind.trim()
        ));
    }

    let expected_csv_path = required_validation_context_text(ctx.csv_path.as_deref(), "csv_path")?;
    let expected = canonical_validation_context_path_key(expected_csv_path);
    let actual = canonical_validation_context_path_key(&loaded.csv_path);
    if actual.is_empty() {
        return Err("loaded data id is missing source CSV metadata".to_string());
    }
    if expected != actual {
        let label = ctx
            .label
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(expected_csv_path);
        return Err(format!(
            "validation requires dataset '{label}' ({expected_csv_path}), but loaded data id points to '{}'",
            loaded.csv_path
        ));
    }

    let expected_timeframe = normalize_timeframe_label(required_validation_context_text(
        ctx.timeframe.as_deref(),
        "timeframe",
    )?);
    if expected_timeframe.is_empty() {
        return Err("validation_context.timeframe is required".to_string());
    }
    let actual_timeframe_raw = loaded
        .timeframe
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "loaded data id is missing timeframe metadata".to_string())?;
    let actual_timeframe = normalize_timeframe_label(actual_timeframe_raw);
    if actual_timeframe.is_empty() {
        return Err("loaded data id is missing timeframe metadata".to_string());
    }
    if expected_timeframe != actual_timeframe {
        return Err(format!(
            "validation requires timeframe '{expected_timeframe}', but loaded data id has timeframe '{actual_timeframe}'"
        ));
    }

    Ok(())
}

fn validate_cross_ticker_config_paths_for_command<R: tauri::Runtime>(
    cross_cfg: &mut ta_app_core::CrossTickerConfig,
    state: &AppState,
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    validation_common::sanitize_cross_ticker_config(cross_cfg);
    if cross_cfg.max_datasets == 0 {
        cross_cfg.datasets.clear();
        return Ok(());
    }
    if cross_cfg.datasets.len() > cross_cfg.max_datasets {
        cross_cfg.datasets.truncate(cross_cfg.max_datasets);
    }

    for ds in &mut cross_cfg.datasets {
        let prior_path = ds.csv_path.clone();
        let canonical =
            super::commands_data::validate_csv_load_path_for_command(&prior_path, state, app)
                .map_err(|e| {
                    let label = validation_common::dataset_label(&prior_path, &ds.label);
                    format!("cross_ticker dataset '{label}' is not allowed: {e}")
                })?;
        ds.csv_path = canonical.to_string_lossy().to_string();
    }
    validation_common::sanitize_cross_ticker_config(cross_cfg);
    Ok(())
}

impl CrossTickerValidationResultExt for CompositeValidationResult {
    type Candidate = ta_app_core::composite_validate::CompositeValidationCandidateResult;

    fn candidates(&self) -> &Vec<Self::Candidate> {
        &self.candidates
    }

    fn candidates_mut(&mut self) -> &mut Vec<Self::Candidate> {
        &mut self.candidates
    }

    fn set_cross_ticker_summary(&mut self, summary: ta_app_core::CrossTickerValidationSummary) {
        self.cross_ticker = Some(summary);
    }
}

impl CrossTickerValidationResultExt for DoubleMaValidationResult {
    type Candidate = ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult;

    fn candidates(&self) -> &Vec<Self::Candidate> {
        &self.candidates
    }

    fn candidates_mut(&mut self) -> &mut Vec<Self::Candidate> {
        &mut self.candidates
    }

    fn set_cross_ticker_summary(&mut self, summary: ta_app_core::CrossTickerValidationSummary) {
        self.cross_ticker = Some(summary);
    }
}

fn apply_cross_ticker_validation_generic<R, Req, FMakeReq, FRun>(
    res: &mut R,
    mut cross_cfg: ta_app_core::CrossTickerConfig,
    base_candles: &vector_ta::utilities::data_loader::Candles,
    app_cache_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    app_for_progress: Option<tauri::AppHandle>,
    phase: &'static str,
    make_req: FMakeReq,
    run_validate: FRun,
) -> Result<(), String>
where
    R: CrossTickerValidationResultExt,
    <R::Candidate as validation_common::CrossTickerCandidate>::Params: Serialize + Clone,
    FMakeReq: Fn(
        usize,
        usize,
        Vec<<R::Candidate as validation_common::CrossTickerCandidate>::Params>,
    ) -> Req,
    FRun: Fn(
        Req,
        &vector_ta::utilities::data_loader::Candles,
        &std::sync::atomic::AtomicBool,
        Option<&dyn ta_app_core::progress::ProgressSink>,
    ) -> Result<R, String>,
{
    validation_common::sanitize_cross_ticker_config(&mut cross_cfg);
    if cross_cfg.max_datasets == 0 {
        return Ok(());
    }
    if cross_cfg.datasets.len() > cross_cfg.max_datasets {
        cross_cfg.datasets.truncate(cross_cfg.max_datasets);
    }
    if cross_cfg.datasets.is_empty() || res.candidates().is_empty() {
        return Ok(());
    }

    let mut key_to_idx: HashMap<String, usize> =
        HashMap::with_capacity(res.candidates().len().saturating_mul(2));
    for (i, c) in res.candidates().iter().enumerate() {
        let key = validation_common::candidate_key_canonical(c.params())?;
        key_to_idx.insert(key, i);
    }

    let total_candidates = res.candidates().len();
    let cross_total = total_candidates.saturating_mul(cross_cfg.datasets.len());
    let candidate_params_template: Vec<_> = res
        .candidates()
        .iter()
        .map(|c| c.params().clone())
        .collect();

    let mut accs: Vec<validation_common::CrossTickerAcc> = (0..total_candidates)
        .map(|_| validation_common::CrossTickerAcc::with_capacity(cross_cfg.datasets.len()))
        .collect();
    let mut datasets_meta: Vec<ta_app_core::CrossTickerDatasetMeta> =
        Vec::with_capacity(cross_cfg.datasets.len());

    for (di, ds) in cross_cfg.datasets.iter().enumerate() {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("cancelled".to_string());
        }

        let csv_path = ds.csv_path.trim();
        if csv_path.is_empty() {
            continue;
        }
        let outcome = crate::data_cache::load_or_build(Path::new(csv_path), app_cache_dir)?;
        let candles2 = align_cross_ticker_candles_to_base_horizon(base_candles, &outcome.candles);
        datasets_meta.push(ta_app_core::CrossTickerDatasetMeta {
            label: validation_common::dataset_label(csv_path, &ds.label),
            csv_path: csv_path.to_string(),
            bars: candles2.as_ref().close.len(),
        });

        let req2 = make_req(di, total_candidates, candidate_params_template.clone());
        let offset = di.saturating_mul(total_candidates);

        let res2 = if let Some(app) = app_for_progress.as_ref() {
            let sink2 = OffsetProgressSink {
                app: app.clone(),
                base_offset: offset,
                total: cross_total,
                phase,
            };
            run_validate(
                req2,
                candles2.as_ref(),
                cancel,
                Some(&sink2 as &dyn ta_app_core::progress::ProgressSink),
            )?
        } else {
            run_validate(req2, candles2.as_ref(), cancel, None)?
        };

        for c in res2.candidates() {
            let key = validation_common::candidate_key_canonical(c.params())?;
            let Some(&idx) = key_to_idx.get(&key) else {
                continue;
            };

            if c.validation_passed() {
                validation_common::push_metrics(&mut accs[idx], c.cross_ticker_metrics());
            }
        }
    }

    validation_common::finalize_cross_ticker_candidates(res.candidates_mut(), accs);
    let gate = &cross_cfg.gate;
    res.candidates_mut().retain(|cand| {
        let Some(stats) = cand.cross_ticker() else {
            return false;
        };
        validation_common::cross_ticker_passes_gate(stats, cand.cross_ticker_pareto(), gate)
    });
    res.candidates_mut()
        .sort_by(|a, b| b.robust_score().total_cmp(&a.robust_score()));

    res.set_cross_ticker_summary(ta_app_core::CrossTickerValidationSummary {
        mode: cross_cfg.mode.clone(),
        datasets: datasets_meta,
    });
    Ok(())
}

#[tauri::command]
pub async fn run_composite_validation(
    mut req: CompositeValidationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<CompositeValidationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let loaded = state.get_loaded_data_context(&req.data_id)?;
    validate_validation_context(
        req.validation_context.as_ref(),
        &loaded,
        &[
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
            "CompositeCpu",
            "CompositeMo",
        ],
    )?;
    let candles = state.get_candles(&req.data_id)?;

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = crate::data_cache::app_data_cache_dir(&base);
    if let Some(cross_cfg) = req.cross_ticker.as_mut() {
        validate_cross_ticker_config_paths_for_command(cross_cfg, &state, &app)?;
    }

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let sink = TauriProgressSink { app: app2 };
        let mut base_req = req;
        let backend = base_req.backend.clone();
        let ma_source = base_req.ma_source.clone();
        let fast_ma_params = base_req.fast_ma_params.clone();
        let slow_ma_params = base_req.slow_ma_params.clone();
        let walk_forward = base_req.walk_forward.clone();
        let reopt_walk_forward = base_req.reopt_walk_forward.clone();
        let spa = base_req.spa.clone();
        let perturb = base_req.perturb.clone();
        let trade_bootstrap = base_req.trade_bootstrap.clone();
        let monte_carlo = base_req.monte_carlo.clone();
        let seed = base_req.seed;
        let cscv_pbo = base_req.cscv_pbo.clone();
        let validation_engine = base_req.validation_engine;
        let iterative_cfg = base_req.iterative.clone();
        let min_psr = base_req.min_psr;
        let min_dsr = base_req.min_dsr;
        let min_cscv_oos_rank_frac = base_req.min_cscv_oos_rank_frac;
        let min_cscv_oos_metric = base_req.min_cscv_oos_metric;
        let max_pbo = base_req.max_pbo;
        let cross_cfg = base_req.cross_ticker.take();
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            base_req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )?;

        if let Some(cross_cfg) = cross_cfg {
            apply_cross_ticker_validation_with_stack(
                &mut res,
                cross_cfg,
                &candles,
                backend,
                &ma_source,
                fast_ma_params,
                slow_ma_params,
                walk_forward,
                reopt_walk_forward,
                spa,
                perturb,
                trade_bootstrap,
                monte_carlo,
                seed,
                &app_cache_dir,
                cancel2.as_ref(),
                Some(sink.app.clone()),
                validation_engine,
                iterative_cfg,
                cscv_pbo,
                min_psr,
                min_dsr,
                min_cscv_oos_rank_frac,
                min_cscv_oos_metric,
                max_pbo,
            )?;
        }
        res.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[allow(dead_code)]
pub(crate) fn apply_cross_ticker_validation(
    res: &mut CompositeValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
    base_candles: &vector_ta::utilities::data_loader::Candles,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    app_for_progress: Option<tauri::AppHandle>,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
    cscv_pbo: Option<ta_app_core::CscvPboConfig>,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> Result<(), String> {
    apply_cross_ticker_validation_with_stack(
        res,
        cross_cfg,
        base_candles,
        backend,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        None,
        None,
        None,
        None,
        None,
        None,
        seed,
        app_cache_dir,
        cancel,
        app_for_progress,
        validation_engine,
        iterative_cfg,
        cscv_pbo,
        min_psr,
        min_dsr,
        min_cscv_oos_rank_frac,
        min_cscv_oos_metric,
        max_pbo,
    )
}

fn apply_cross_ticker_validation_with_stack(
    res: &mut CompositeValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
    base_candles: &vector_ta::utilities::data_loader::Candles,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    reopt_walk_forward: Option<ta_app_core::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
    monte_carlo: Option<ta_app_core::MonteCarloValidationConfig>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    app_for_progress: Option<tauri::AppHandle>,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
    cscv_pbo: Option<ta_app_core::CscvPboConfig>,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> Result<(), String> {
    let preserved_cross_mode = matches!(
        cross_cfg.mode,
        ta_app_core::CrossTickerMode::PreservedFinalists
    );
    let full_stack_cross_mode = matches!(cross_cfg.mode, ta_app_core::CrossTickerMode::FullStack);
    let finalists_cross_mode = preserved_cross_mode || full_stack_cross_mode;

    let original_candidates_tested = res.run_verdict.candidates_tested.max(res.candidates.len());
    if finalists_cross_mode && res.candidates.len() > PRESERVED_FINALISTS_CROSS_TICKER_LIMIT {
        res.candidates = select_cross_ticker_preserved_finalists(&res.candidates);
    }

    apply_cross_ticker_validation_generic(
        res,
        cross_cfg,
        base_candles,
        app_cache_dir,
        cancel,
        app_for_progress,
        "composite_validate_cross",
        |di, total_candidates, candidates| {
            build_composite_cross_ticker_request(
                di,
                total_candidates,
                candidates,
                backend.clone(),
                ma_source,
                fast_ma_params.clone(),
                slow_ma_params.clone(),
                if full_stack_cross_mode {
                    walk_forward.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    reopt_walk_forward.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    spa.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    perturb.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    trade_bootstrap.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    monte_carlo.clone()
                } else {
                    None
                },
                seed,
                validation_engine,
                iterative_cfg.clone(),
                if finalists_cross_mode {
                    cscv_pbo.clone()
                } else {
                    None
                },
                if finalists_cross_mode { min_psr } else { None },
                if finalists_cross_mode { min_dsr } else { None },
                if finalists_cross_mode {
                    min_cscv_oos_rank_frac
                } else {
                    None
                },
                if finalists_cross_mode {
                    min_cscv_oos_metric
                } else {
                    None
                },
                if finalists_cross_mode { max_pbo } else { None },
            )
        },
        |req2, candles2, cancel2, progress2| {
            if preserved_cross_mode {
                ta_app_core::run_composite_validation_blocking_with_candles_with_aux_mask(
                    req2,
                    candles2,
                    cancel2,
                    progress2,
                    ta_app_core::ValidationAuxWorkMask::preserved_finalists(),
                )
            } else {
                ta_app_core::run_composite_validation_blocking_with_candles(
                    req2, candles2, cancel2, progress2,
                )
            }
        },
    )?;

    let hard_gates = ta_app_core::validation_pipeline::sanitize_hard_gates(
        min_psr,
        min_dsr,
        min_cscv_oos_rank_frac,
        min_cscv_oos_metric,
        max_pbo,
    );
    res.run_verdict = ta_app_core::validation_pipeline::build_run_verdict(
        &res.candidates,
        original_candidates_tested,
        &hard_gates,
        res.cscv_pbo.as_ref(),
        res.spa.as_ref(),
    );
    let cscv_requested = finalists_cross_mode && cscv_pbo.is_some();
    let spa_requested = full_stack_cross_mode && spa.is_some();
    res.validation_provenance = Some(
        ta_app_core::validation_pipeline::build_validation_semantics_provenance(
            &res.candidates,
            cscv_requested,
            res.cscv_pbo.as_ref(),
            res.cross_ticker.as_ref(),
            Some("volatility_terciles_plus_trend_range"),
        ),
    );
    res.evidence_policy_summary = Some(
        ta_app_core::validation_pipeline::build_general_validation_evidence_policy_summary(
            &hard_gates,
            cscv_requested,
            res.cscv_pbo.as_ref(),
        ),
    );
    let route_diagnostics = res.route_diagnostics.clone().unwrap_or_default();
    let realized_backend = match &res.backend_used {
        ta_app_core::BackendUsed::Cpu => "Cpu",
        ta_app_core::BackendUsed::Gpu => "Gpu",
    };
    res.validation_evidence_pack = Some(
        ta_app_core::validation_pipeline::build_validation_evidence_pack(
            ta_app_core::validation_pipeline::ValidationEvidencePackBuildInput {
                workflow: "composite_validation",
                data_id: &res.data_id,
                validation_context: None,
                requested_backend: &res.requested_backend,
                realized_backend,
                backend_route: res.backend_route.as_deref(),
                residency_tier: &res.validation_residency_tier,
                fallback_reason: res.backend_fallback_reason.as_deref(),
                route_diagnostics: &route_diagnostics,
                candidate_source: "cross_ticker_finalists",
                requested_candidate_count: original_candidates_tested,
                evaluated_candidate_count: res.run_verdict.candidates_tested,
                retained_candidate_count: res.candidates.len(),
                optimization_trials: None,
                admission_policy: None,
                hard_gates: &hard_gates,
                run_verdict: &res.run_verdict,
                evidence_policy_summary: res.evidence_policy_summary.as_ref(),
                validation_provenance: res.validation_provenance.as_ref(),
                cscv_requested,
                cscv_pbo: res.cscv_pbo.as_ref(),
                spa_requested,
                spa: res.spa.as_ref(),
            },
        ),
    );
    Ok(())
}

fn build_composite_cross_ticker_request(
    dataset_index: usize,
    total_candidates: usize,
    candidates: Vec<ta_app_core::CompositeParams>,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    reopt_walk_forward: Option<ta_app_core::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
    monte_carlo: Option<ta_app_core::MonteCarloValidationConfig>,
    seed: u64,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
    cscv_pbo: Option<ta_app_core::CscvPboConfig>,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> ta_app_core::CompositeValidationRequest {
    ta_app_core::CompositeValidationRequest {
        data_id: format!("cross-{dataset_index}"),
        backend,
        ma_source: ma_source.to_string(),
        fast_ma_params,
        slow_ma_params,
        candidates,
        walk_forward,
        reopt_walk_forward,
        spa,
        perturb,
        trade_bootstrap,
        monte_carlo,
        cross_ticker: None,
        cscv_pbo,
        seed,
        max_candidates: total_candidates,
        validation_engine,
        iterative: iterative_cfg,
        min_psr,
        min_dsr,
        min_cscv_oos_rank_frac,
        min_cscv_oos_metric,
        max_pbo,
        validation_context: None,
    }
}

#[tauri::command]
pub async fn run_double_ma_validation(
    mut req: DoubleMaValidationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<DoubleMaValidationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let loaded = state.get_loaded_data_context(&req.data_id)?;
    validate_validation_context(
        req.validation_context.as_ref(),
        &loaded,
        &[
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
        ],
    )?;
    let candles = state.get_candles(&req.data_id)?;

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = crate::data_cache::app_data_cache_dir(&base);
    if let Some(cross_cfg) = req.cross_ticker.as_mut() {
        validate_cross_ticker_config_paths_for_command(cross_cfg, &state, &app)?;
    }

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let sink = TauriProgressSink { app: app2 };
        let mut base_req = req;
        let backend = base_req.backend.clone();
        let ma_source = base_req.ma_source.clone();
        let fast_ma_params = base_req.fast_ma_params.clone();
        let slow_ma_params = base_req.slow_ma_params.clone();
        let walk_forward = base_req.walk_forward.clone();
        let reopt_walk_forward = base_req.reopt_walk_forward.clone();
        let spa = base_req.spa.clone();
        let perturb = base_req.perturb.clone();
        let trade_bootstrap = base_req.trade_bootstrap.clone();
        let monte_carlo = base_req.monte_carlo.clone();
        let cscv_pbo = base_req.cscv_pbo.clone();
        let seed = base_req.seed;
        let validation_engine = base_req.validation_engine;
        let iterative_cfg = base_req.iterative.clone();
        let min_psr = base_req.min_psr;
        let min_dsr = base_req.min_dsr;
        let min_cscv_oos_rank_frac = base_req.min_cscv_oos_rank_frac;
        let min_cscv_oos_metric = base_req.min_cscv_oos_metric;
        let max_pbo = base_req.max_pbo;
        let cross_cfg = base_req.cross_ticker.take();
        let mut res = ta_app_core::run_double_ma_validation_blocking_with_candles(
            base_req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )?;

        if let Some(cross_cfg) = cross_cfg {
            apply_cross_ticker_validation_double_ma(
                &mut res,
                cross_cfg,
                &candles,
                backend,
                &ma_source,
                fast_ma_params,
                slow_ma_params,
                walk_forward,
                reopt_walk_forward,
                spa,
                perturb,
                trade_bootstrap,
                monte_carlo,
                cscv_pbo,
                seed,
                &app_cache_dir,
                cancel2.as_ref(),
                Some(sink.app.clone()),
                validation_engine,
                iterative_cfg,
                min_psr,
                min_dsr,
                min_cscv_oos_rank_frac,
                min_cscv_oos_metric,
                max_pbo,
            )?;
        }
        res.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn apply_cross_ticker_validation_double_ma(
    res: &mut DoubleMaValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
    base_candles: &vector_ta::utilities::data_loader::Candles,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    reopt_walk_forward: Option<ta_app_core::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
    monte_carlo: Option<ta_app_core::MonteCarloValidationConfig>,
    cscv_pbo: Option<ta_app_core::CscvPboConfig>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &std::sync::atomic::AtomicBool,
    app_for_progress: Option<tauri::AppHandle>,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> Result<(), String> {
    let preserved_cross_mode = matches!(
        cross_cfg.mode,
        ta_app_core::CrossTickerMode::PreservedFinalists
    );
    let full_stack_cross_mode = matches!(cross_cfg.mode, ta_app_core::CrossTickerMode::FullStack);
    let finalists_cross_mode = preserved_cross_mode || full_stack_cross_mode;

    let original_candidates_tested = res.run_verdict.candidates_tested.max(res.candidates.len());
    if finalists_cross_mode && res.candidates.len() > PRESERVED_FINALISTS_CROSS_TICKER_LIMIT {
        res.candidates = select_cross_ticker_preserved_finalists(&res.candidates);
    }

    apply_cross_ticker_validation_generic(
        res,
        cross_cfg,
        base_candles,
        app_cache_dir,
        cancel,
        app_for_progress,
        "double_ma_validate_cross",
        |di, total_candidates, candidates| {
            build_double_ma_cross_ticker_request(
                di,
                total_candidates,
                candidates,
                backend.clone(),
                ma_source,
                fast_ma_params.clone(),
                slow_ma_params.clone(),
                if full_stack_cross_mode {
                    walk_forward.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    reopt_walk_forward.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    spa.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    perturb.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    trade_bootstrap.clone()
                } else {
                    None
                },
                if full_stack_cross_mode {
                    monte_carlo.clone()
                } else {
                    None
                },
                if finalists_cross_mode {
                    cscv_pbo.clone()
                } else {
                    None
                },
                seed,
                validation_engine,
                iterative_cfg.clone(),
                if finalists_cross_mode { min_psr } else { None },
                if finalists_cross_mode { min_dsr } else { None },
                if finalists_cross_mode {
                    min_cscv_oos_rank_frac
                } else {
                    None
                },
                if finalists_cross_mode {
                    min_cscv_oos_metric
                } else {
                    None
                },
                if finalists_cross_mode { max_pbo } else { None },
            )
        },
        |req2, candles2, cancel2, progress2| {
            if preserved_cross_mode {
                ta_app_core::run_double_ma_validation_blocking_with_candles_with_aux_mask(
                    req2,
                    candles2,
                    cancel2,
                    progress2,
                    ta_app_core::ValidationAuxWorkMask::preserved_finalists(),
                )
            } else {
                ta_app_core::run_double_ma_validation_blocking_with_candles(
                    req2, candles2, cancel2, progress2,
                )
            }
        },
    )?;

    let hard_gates = ta_app_core::validation_pipeline::sanitize_hard_gates(
        min_psr,
        min_dsr,
        min_cscv_oos_rank_frac,
        min_cscv_oos_metric,
        max_pbo,
    );
    res.run_verdict = ta_app_core::validation_pipeline::build_run_verdict(
        &res.candidates,
        original_candidates_tested,
        &hard_gates,
        res.cscv_pbo.as_ref(),
        res.spa.as_ref(),
    );
    let cscv_requested = finalists_cross_mode && cscv_pbo.is_some();
    let spa_requested = full_stack_cross_mode && spa.is_some();
    res.validation_provenance = Some(
        ta_app_core::validation_pipeline::build_validation_semantics_provenance(
            &res.candidates,
            cscv_requested,
            res.cscv_pbo.as_ref(),
            res.cross_ticker.as_ref(),
            Some("volatility_terciles_plus_trend_range"),
        ),
    );
    res.evidence_policy_summary = Some(
        ta_app_core::validation_pipeline::build_general_validation_evidence_policy_summary(
            &hard_gates,
            cscv_requested,
            res.cscv_pbo.as_ref(),
        ),
    );
    let route_diagnostics: Vec<String> = Vec::new();
    let realized_backend = match &res.backend_used {
        ta_app_core::BackendUsed::Cpu => "Cpu",
        ta_app_core::BackendUsed::Gpu => "Gpu",
    };
    res.validation_evidence_pack = Some(
        ta_app_core::validation_pipeline::build_validation_evidence_pack(
            ta_app_core::validation_pipeline::ValidationEvidencePackBuildInput {
                workflow: "double_ma_validation",
                data_id: &res.data_id,
                validation_context: None,
                requested_backend: &res.requested_backend,
                realized_backend,
                backend_route: res.backend_route.as_deref(),
                residency_tier: &res.validation_residency_tier,
                fallback_reason: res.backend_fallback_reason.as_deref(),
                route_diagnostics: &route_diagnostics,
                candidate_source: "cross_ticker_finalists",
                requested_candidate_count: original_candidates_tested,
                evaluated_candidate_count: res.run_verdict.candidates_tested,
                retained_candidate_count: res.candidates.len(),
                optimization_trials: None,
                admission_policy: None,
                hard_gates: &hard_gates,
                run_verdict: &res.run_verdict,
                evidence_policy_summary: res.evidence_policy_summary.as_ref(),
                validation_provenance: res.validation_provenance.as_ref(),
                cscv_requested,
                cscv_pbo: res.cscv_pbo.as_ref(),
                spa_requested,
                spa: res.spa.as_ref(),
            },
        ),
    );
    Ok(())
}

fn select_cross_ticker_preserved_finalists<
    C: ta_app_core::validation_pipeline::ValidationPostCandidate + Clone,
>(
    candidates: &[C],
) -> Vec<C> {
    let mut approved_indices = Vec::with_capacity(PRESERVED_FINALISTS_CROSS_TICKER_LIMIT);
    let mut hard_gate_survivor_indices = Vec::with_capacity(PRESERVED_FINALISTS_CROSS_TICKER_LIMIT);
    for (index, cand) in candidates.iter().enumerate() {
        let hard_gate_passed = cand.validation_verdict().passed;
        if hard_gate_passed {
            if approved_indices.len() < PRESERVED_FINALISTS_CROSS_TICKER_LIMIT
                && cand
                    .min_track_record_length_bars()
                    .map(|mintrl| cand.track_record_bars().max(2) >= mintrl)
                    .unwrap_or(false)
            {
                approved_indices.push(index);
            }
            if hard_gate_survivor_indices.len() < PRESERVED_FINALISTS_CROSS_TICKER_LIMIT {
                hard_gate_survivor_indices.push(index);
            }
        }
        if approved_indices.len() >= PRESERVED_FINALISTS_CROSS_TICKER_LIMIT
            && hard_gate_survivor_indices.len() >= PRESERVED_FINALISTS_CROSS_TICKER_LIMIT
        {
            break;
        }
    }
    if !approved_indices.is_empty() {
        return approved_indices
            .into_iter()
            .filter_map(|index| candidates.get(index).cloned())
            .collect();
    }

    if !hard_gate_survivor_indices.is_empty() {
        return hard_gate_survivor_indices
            .into_iter()
            .filter_map(|index| candidates.get(index).cloned())
            .collect();
    }

    let fallback_len = candidates.len().min(PRESERVED_FINALISTS_CROSS_TICKER_LIMIT);
    let mut fallback = Vec::with_capacity(fallback_len);
    fallback.extend_from_slice(&candidates[..fallback_len]);
    fallback
}

fn build_double_ma_cross_ticker_request(
    dataset_index: usize,
    total_candidates: usize,
    candidates: Vec<ta_app_core::DoubleMaValidationParams>,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    reopt_walk_forward: Option<ta_app_core::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
    monte_carlo: Option<ta_app_core::MonteCarloValidationConfig>,
    cscv_pbo: Option<ta_app_core::CscvPboConfig>,
    seed: u64,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> ta_app_core::DoubleMaValidationRequest {
    ta_app_core::DoubleMaValidationRequest {
        data_id: format!("cross-{dataset_index}"),
        backend,
        ma_source: ma_source.to_string(),
        fast_ma_params,
        slow_ma_params,
        candidates,
        walk_forward,
        reopt_walk_forward,
        spa,
        perturb,
        trade_bootstrap,
        monte_carlo,
        cross_ticker: None,
        cscv_pbo,
        seed,
        max_candidates: total_candidates,
        validation_engine,
        iterative: iterative_cfg,
        min_psr,
        min_dsr,
        min_cscv_oos_rank_frac,
        min_cscv_oos_metric,
        max_pbo,
        validation_context: None,
    }
}

pub fn apply_cross_ticker_validation_double_ma_headless(
    res: &mut DoubleMaValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
    base_candles: &vector_ta::utilities::data_loader::Candles,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    reopt_walk_forward: Option<ta_app_core::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
    monte_carlo: Option<ta_app_core::MonteCarloValidationConfig>,
    cscv_pbo: Option<ta_app_core::CscvPboConfig>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &AtomicBool,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
    min_psr: Option<f64>,
    min_dsr: Option<f64>,
    min_cscv_oos_rank_frac: Option<f64>,
    min_cscv_oos_metric: Option<f64>,
    max_pbo: Option<f64>,
) -> Result<(), String> {
    apply_cross_ticker_validation_double_ma(
        res,
        cross_cfg,
        base_candles,
        backend,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        walk_forward,
        reopt_walk_forward,
        spa,
        perturb,
        trade_bootstrap,
        monte_carlo,
        cscv_pbo,
        seed,
        app_cache_dir,
        cancel,
        None,
        validation_engine,
        iterative_cfg,
        min_psr,
        min_dsr,
        min_cscv_oos_rank_frac,
        min_cscv_oos_metric,
        max_pbo,
    )
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceBar {
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub from_index: u32,
    pub to_index: u32,
}

fn build_price_bars(
    candles: &vector_ta::utilities::data_loader::Candles,
    max_points: usize,
) -> Result<Vec<PriceBar>, String> {
    let n = candles.timestamp.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    if candles.open.len() != n
        || candles.high.len() != n
        || candles.low.len() != n
        || candles.close.len() != n
        || candles.volume.len() != n
    {
        return Err("candles vectors have inconsistent lengths".to_string());
    }

    let cap = max_points.clamp(300, 20_000);
    let step = if n <= cap {
        1usize
    } else {
        (n + cap - 1) / cap
    };
    let mut out: Vec<PriceBar> = Vec::with_capacity((n + step - 1) / step);
    let mut prev_time: Option<i64> = None;

    let mut from = 0usize;
    while from < n {
        let to_exclusive = n.min(from.saturating_add(step));
        let to = to_exclusive.saturating_sub(1);

        let mut open = f64::NAN;
        let mut close = f64::NAN;
        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        let mut volume = 0.0f64;
        for i in from..to_exclusive {
            let o = candles.open[i];
            let c = candles.close[i];
            if !open.is_finite() {
                if o.is_finite() {
                    open = o;
                } else if c.is_finite() {
                    open = c;
                }
            }
            if c.is_finite() {
                close = c;
            } else if o.is_finite() {
                close = o;
            }

            for v in [candles.high[i], o, c, candles.low[i]] {
                if v.is_finite() {
                    if v > high {
                        high = v;
                    }
                    if v < low {
                        low = v;
                    }
                }
            }
            let vv = candles.volume[i];
            if vv.is_finite() {
                volume += vv;
            }
        }

        if open.is_finite() && close.is_finite() && high.is_finite() && low.is_finite() {
            let time = candles.timestamp[from];
            if prev_time.is_none_or(|p| time > p) {
                out.push(PriceBar {
                    time,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    from_index: u32::try_from(from)
                        .map_err(|_| "dataset too large for chart index".to_string())?,
                    to_index: u32::try_from(to)
                        .map_err(|_| "dataset too large for chart index".to_string())?,
                });
                prev_time = Some(time);
            }
        }

        from = to_exclusive;
    }

    Ok(out)
}

#[tauri::command]
pub async fn get_price_bars(
    data_id: String,
    max_points: Option<u32>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PriceBar>, String> {
    let candles = state.get_candles(&data_id)?;
    let cap = max_points.unwrap_or(6000) as usize;
    tauri::async_runtime::spawn_blocking(move || build_price_bars(&candles, cap))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;
    use ta_app_core::CrossTickerStats;
    use ta_strategies::double_ma::Metrics;
    use tauri::Manager;

    fn data_csv(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("data")
            .join(name)
    }

    fn make_cache_dir() -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let p = std::env::temp_dir().join(format!("ta_desktop_demo_cross_cache_{ts}"));
        let _ = std::fs::create_dir_all(&p);
        p
    }

    fn make_external_dir(prefix: &str) -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let current = std::env::current_dir().expect("current dir");
        let parent = current.parent().unwrap_or(current.as_path());
        let p = parent.join(format!("{prefix}_{}_{}", std::process::id(), ts));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("create external dir");
        p
    }

    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("build mock app")
    }

    fn write_minimal_csv(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create csv parent");
        }
        std::fs::write(
            path,
            "timestamp,open,high,low,close,volume\n1,1,1,1,1,1\n2,2,2,2,2,2\n",
        )
        .expect("write csv");
    }

    fn cross_config_for(path: &Path) -> ta_app_core::CrossTickerConfig {
        ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: path.to_string_lossy().to_string(),
                label: "Cross".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig::default(),
        }
    }

    fn load_candles(path: &Path) -> vector_ta::utilities::data_loader::Candles {
        ta_app_core::read_candles_from_csv_compat(path.to_str().expect("path utf8")).expect("csv")
    }

    fn candles_with_ts(start: i64, len: usize) -> vector_ta::utilities::data_loader::Candles {
        let timestamp: Vec<i64> = (0..len).map(|idx| start + idx as i64).collect();
        let values: Vec<f64> = (0..len).map(|idx| idx as f64).collect();
        vector_ta::utilities::data_loader::Candles::new_with_fields(
            timestamp,
            values.clone(),
            values.clone(),
            values.clone(),
            values.clone(),
            values,
            vector_ta::utilities::data_loader::CandleFieldFlags {
                open: true,
                high: true,
                low: true,
                close: true,
                volume: true,
            },
        )
    }

    fn candles_from_rows(
        rows: &[(i64, f64, f64, f64, f64, f64)],
    ) -> vector_ta::utilities::data_loader::Candles {
        vector_ta::utilities::data_loader::Candles::new_with_fields(
            rows.iter().map(|row| row.0).collect(),
            rows.iter().map(|row| row.1).collect(),
            rows.iter().map(|row| row.2).collect(),
            rows.iter().map(|row| row.3).collect(),
            rows.iter().map(|row| row.4).collect(),
            rows.iter().map(|row| row.5).collect(),
            vector_ta::utilities::data_loader::CandleFieldFlags {
                open: true,
                high: true,
                low: true,
                close: true,
                volume: true,
            },
        )
    }

    fn loaded_ctx(path: &str, timeframe: Option<&str>) -> LoadedDataContext {
        LoadedDataContext {
            csv_path: path.to_string(),
            label: "loaded".to_string(),
            timeframe: timeframe.map(str::to_string),
            source_identity: None,
        }
    }

    #[derive(Clone, Serialize)]
    struct MockCrossParams {
        id: u32,
    }

    #[derive(Clone)]
    struct MockCrossCandidate {
        params: MockCrossParams,
        metrics: Metrics,
        validation_passed: bool,
        cross_ticker: Option<CrossTickerStats>,
        cross_ticker_pareto: Option<bool>,
        robust_score: f64,
    }

    struct MockCrossResult {
        candidates: Vec<MockCrossCandidate>,
        summary: Option<ta_app_core::CrossTickerValidationSummary>,
    }

    impl validation_common::CrossTickerCandidate for MockCrossCandidate {
        type Params = MockCrossParams;

        fn params(&self) -> &Self::Params {
            &self.params
        }

        fn cross_ticker_metrics(&self) -> &Metrics {
            &self.metrics
        }

        fn validation_passed(&self) -> bool {
            self.validation_passed
        }

        fn cross_ticker(&self) -> Option<&CrossTickerStats> {
            self.cross_ticker.as_ref()
        }

        fn cross_ticker_pareto(&self) -> Option<bool> {
            self.cross_ticker_pareto
        }

        fn set_cross_ticker(&mut self, stats: CrossTickerStats) {
            self.cross_ticker = Some(stats);
        }

        fn set_cross_ticker_pareto(&mut self, is_pareto: bool) {
            self.cross_ticker_pareto = Some(is_pareto);
        }

        fn robust_score(&self) -> f64 {
            self.robust_score
        }

        fn set_robust_score(&mut self, score: f64) {
            self.robust_score = score;
        }
    }

    impl CrossTickerValidationResultExt for MockCrossResult {
        type Candidate = MockCrossCandidate;

        fn candidates(&self) -> &Vec<Self::Candidate> {
            &self.candidates
        }

        fn candidates_mut(&mut self) -> &mut Vec<Self::Candidate> {
            &mut self.candidates
        }

        fn set_cross_ticker_summary(&mut self, summary: ta_app_core::CrossTickerValidationSummary) {
            self.summary = Some(summary);
        }
    }

    #[test]
    fn validation_context_rejects_dataset_mismatch() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "DoubleMa".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        let err = validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\MSFT.csv", Some("15m")),
            &["DoubleMa"],
        )
        .expect_err("dataset mismatch should fail");
        assert!(
            err.contains("requires dataset 'AAPL'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validation_context_rejects_timeframe_mismatch() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "CompositeCpu".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("1h".to_string()),
            label: Some("AAPL".to_string()),
        };
        let err = validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["CompositeCpu", "CompositeTrend"],
        )
        .expect_err("timeframe mismatch should fail");
        assert!(
            err.contains("requires timeframe '1h'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validation_context_rejects_missing_dataset_path() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "DoubleMa".to_string(),
            csv_path: None,
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        let err = validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa"],
        )
        .expect_err("missing context csv_path should fail");
        assert!(err.contains("csv_path"), "unexpected error: {err}");
    }

    #[test]
    fn validation_context_rejects_missing_timeframe() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "DoubleMa".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("   ".to_string()),
            label: Some("AAPL".to_string()),
        };
        let err = validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa"],
        )
        .expect_err("missing context timeframe should fail");
        assert!(err.contains("timeframe"), "unexpected error: {err}");
    }

    #[test]
    fn cross_ticker_command_path_validation_rejects_unissued_external_csv() {
        let app = build_mock_app();
        let state = AppState::default();
        let external_dir = make_external_dir("ta_desktop_demo_cross_external");
        let csv = external_dir.join("outside.csv");
        write_minimal_csv(&csv);
        let mut cfg = cross_config_for(&csv);

        let err = validate_cross_ticker_config_paths_for_command(&mut cfg, &state, app.handle())
            .expect_err("external cross CSV should require a picker token");
        assert!(
            err.contains("native open dialog") || err.contains("outside allowed data roots"),
            "unexpected error: {err}"
        );

        let _ = std::fs::remove_dir_all(external_dir);
    }

    #[test]
    fn cross_ticker_command_path_validation_accepts_issued_csv_once() {
        let app = build_mock_app();
        let state = AppState::default();
        let external_dir = make_external_dir("ta_desktop_demo_cross_external_issued");
        let csv = external_dir.join("issued.csv");
        write_minimal_csv(&csv);
        let canonical = csv.canonicalize().expect("canonical csv");
        state
            .record_issued_csv_load_path(canonical.clone())
            .expect("record picker token");

        let mut cfg = cross_config_for(&csv);
        validate_cross_ticker_config_paths_for_command(&mut cfg, &state, app.handle())
            .expect("issued cross CSV should pass");
        assert_eq!(PathBuf::from(&cfg.datasets[0].csv_path), canonical);

        let mut cfg = cross_config_for(&csv);
        validate_cross_ticker_config_paths_for_command(&mut cfg, &state, app.handle())
            .expect_err("picker token should be single-use");

        let _ = std::fs::remove_dir_all(external_dir);
    }

    #[test]
    fn cross_ticker_command_path_validation_accepts_app_local_csv() {
        let app = build_mock_app();
        let state = AppState::default();
        let app_root = app.path().app_local_data_dir().expect("app local data dir");
        let csv = app_root.join("cross").join("allowed.csv");
        write_minimal_csv(&csv);
        let canonical = csv.canonicalize().expect("canonical csv");

        let mut cfg = cross_config_for(&csv);
        validate_cross_ticker_config_paths_for_command(&mut cfg, &state, app.handle())
            .expect("app-local cross CSV should pass");
        assert_eq!(PathBuf::from(&cfg.datasets[0].csv_path), canonical);

        let _ = std::fs::remove_dir_all(app_root);
    }

    #[test]
    fn cross_ticker_alignment_uses_base_timestamp_overlap() {
        let base = candles_with_ts(100, 10);
        let cross = candles_with_ts(90, 30);
        let aligned = align_cross_ticker_candles_to_base_horizon(&base, &cross);
        assert_eq!(aligned.as_ref().timestamp.first().copied(), Some(100));
        assert_eq!(aligned.as_ref().timestamp.last().copied(), Some(109));
        assert_eq!(aligned.as_ref().close.len(), 10);
    }

    #[test]
    fn cross_ticker_alignment_borrows_when_cross_already_matches_base_horizon() {
        let base = candles_with_ts(100, 10);
        let cross = candles_with_ts(100, 10);
        let aligned = align_cross_ticker_candles_to_base_horizon(&base, &cross);

        assert!(matches!(aligned, AlignedCandles::Borrowed(_)));
    }

    #[test]
    fn cross_ticker_alignment_without_overlap_uses_cross_tail_when_cross_is_longer() {
        let base = candles_with_ts(100, 10);
        let cross = candles_with_ts(0, 30);
        let aligned = align_cross_ticker_candles_to_base_horizon(&base, &cross);

        assert_eq!(aligned.as_ref().timestamp.first().copied(), Some(20));
        assert_eq!(aligned.as_ref().timestamp.last().copied(), Some(29));
        assert_eq!(aligned.as_ref().close.len(), 10);
    }

    #[test]
    fn cross_ticker_generic_accumulates_only_successful_reruns() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let mut res = MockCrossResult {
            candidates: vec![MockCrossCandidate {
                params: MockCrossParams { id: 1 },
                metrics: Metrics {
                    pnl: 1.0,
                    sharpe: 1.0,
                    max_dd: 0.1,
                    trades: 10,
                    ..Metrics::default()
                },
                validation_passed: true,
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: 1.0,
            }],
            summary: None,
        };
        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig::default(),
        };

        apply_cross_ticker_validation_generic(
            &mut res,
            cross_cfg,
            &candles,
            &make_cache_dir(),
            &cancel,
            None,
            "mock_cross",
            |_dataset_index, _total_candidates, candidates| candidates,
            |req2: Vec<MockCrossParams>, _candles2, _cancel2, _progress2| {
                Ok(MockCrossResult {
                    candidates: req2
                        .into_iter()
                        .map(|params| MockCrossCandidate {
                            params,
                            metrics: Metrics {
                                pnl: 100.0,
                                sharpe: 10.0,
                                max_dd: 0.01,
                                trades: 99,
                                ..Metrics::default()
                            },
                            validation_passed: false,
                            cross_ticker: None,
                            cross_ticker_pareto: None,
                            robust_score: 5.0,
                        })
                        .collect(),
                    summary: None,
                })
            },
        )
        .expect("cross-ticker generic mock");

        assert!(res.candidates.is_empty());
        assert_eq!(res.summary.as_ref().map(|s| s.datasets.len()), Some(1));
    }

    #[test]
    fn build_price_bars_clamps_cap_and_preserves_aggregated_ranges() {
        let rows: Vec<(i64, f64, f64, f64, f64, f64)> = (0..301)
            .map(|idx| {
                let base = idx as f64;
                (idx as i64, base, base + 10.0, base - 10.0, base + 0.5, 1.0)
            })
            .collect();
        let candles = candles_from_rows(&rows);

        let bars = build_price_bars(&candles, 1).expect("price bars");

        assert_eq!(bars.len(), 151, "min cap should clamp to 300 points");
        assert_eq!(bars[0].time, 0);
        assert_eq!(bars[0].from_index, 0);
        assert_eq!(bars[0].to_index, 1);
        assert!((bars[0].open - 0.0).abs() < 1e-12);
        assert!((bars[0].close - 1.5).abs() < 1e-12);
        assert!((bars[0].high - 11.0).abs() < 1e-12);
        assert!((bars[0].low + 10.0).abs() < 1e-12);
        assert!((bars[0].volume - 2.0).abs() < 1e-12);
        assert_eq!(bars.last().map(|bar| bar.to_index), Some(300));
        assert!(bars.windows(2).all(|pair| pair[1].time > pair[0].time));
        assert!(bars
            .windows(2)
            .all(|pair| pair[1].from_index == pair[0].to_index.saturating_add(1)));
    }

    #[test]
    fn build_price_bars_skips_non_increasing_timestamps() {
        let candles = candles_from_rows(&[
            (10, 1.0, 2.0, 0.5, 1.5, 5.0),
            (10, 2.0, 3.0, 1.5, 2.5, 6.0),
            (11, 3.0, 4.0, 2.5, 3.5, 7.0),
        ]);

        let bars = build_price_bars(&candles, 6000).expect("price bars");

        assert_eq!(bars.len(), 2, "duplicate timestamps should be skipped");
        assert_eq!(bars[0].time, 10);
        assert_eq!(bars[0].from_index, 0);
        assert_eq!(bars[0].to_index, 0);
        assert_eq!(bars[1].time, 11);
        assert_eq!(bars[1].from_index, 2);
        assert_eq!(bars[1].to_index, 2);
    }

    #[test]
    fn build_price_bars_preserves_nan_open_close_fallback_order_inside_bucket() {
        let mut rows = vec![
            (10, f64::NAN, 10.0, 1.0, 5.0, 2.0),
            (11, 7.0, 12.0, 2.0, f64::NAN, 3.0),
        ];
        rows.extend((2..301).map(|idx| {
            let value = idx as f64;
            (10 + idx as i64, value, value, value, value, 0.0)
        }));
        let candles = candles_from_rows(&rows);

        let bars = build_price_bars(&candles, 1).expect("price bars");

        assert_eq!(bars[0].time, 10);
        assert_eq!(bars[0].from_index, 0);
        assert_eq!(bars[0].to_index, 1);
        assert!((bars[0].open - 5.0).abs() < 1e-12);
        assert!((bars[0].close - 7.0).abs() < 1e-12);
        assert!((bars[0].high - 12.0).abs() < 1e-12);
        assert!((bars[0].low - 1.0).abs() < 1e-12);
        assert!((bars[0].volume - 5.0).abs() < 1e-12);
    }

    #[test]
    fn validation_context_rejects_strategy_command_mismatch() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "CompositeMo".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        let err = validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa"],
        )
        .expect_err("strategy mismatch should fail");
        assert!(err.contains("incompatible"), "unexpected error: {err}");
    }

    #[test]
    fn validation_context_accepts_matching_context() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "CompositeTrend".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["CompositeCpu", "CompositeTrend", "RegimeHybrid"],
        )
        .expect("matching validation context should pass");
    }

    #[test]
    fn validation_context_accepts_pit_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Pit".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa", "Pit"],
        )
        .expect("matching Pit validation context should pass");
    }

    #[test]
    fn validation_context_accepts_marrt_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Marrt".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa", "Marrt"],
        )
        .expect("matching Marrt validation context should pass");
    }

    #[test]
    fn validation_context_accepts_set_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Set".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa", "Set"],
        )
        .expect("matching Set validation context should pass");
    }

    #[test]
    fn validation_context_accepts_mts_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Mts".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa", "Mts"],
        )
        .expect("matching Mts validation context should pass");
    }

    #[test]
    fn validation_context_accepts_orc_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Orc".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa", "Orc"],
        )
        .expect("matching Orc validation context should pass");
    }

    #[test]
    fn validation_context_accepts_vfct_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Vfct".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa", "Vfct"],
        )
        .expect("matching Vfct validation context should pass");
    }

    #[test]
    fn validation_context_accepts_rocb_for_composite_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Rocb".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["CompositeCpu", "Rocb"],
        )
        .expect("matching Rocb validation context should pass");
    }

    #[test]
    fn validation_context_accepts_vcc_for_composite_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Vcc".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["CompositeCpu", "Vcc"],
        )
        .expect("matching Vcc validation context should pass");
    }

    #[test]
    fn validation_context_accepts_mkcs_for_composite_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "Mkcs".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["CompositeCpu", "Mkcs"],
        )
        .expect("matching Mkcs validation context should pass");
    }

    #[test]
    fn validation_context_rejects_unknown_strategy_kind_instead_of_falling_back_to_double_ma() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "DoubleMaTypo".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        let err = validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa"],
        )
        .expect_err("unknown strategy kind should fail");
        assert!(err.contains("DoubleMaTypo"), "unexpected error: {err}");
    }

    #[test]
    fn validation_context_rejects_dynamic_double_ma_for_double_ma_validation() {
        let ctx = ta_app_core::ValidationArtifactContext {
            strategy_kind: "DynamicDoubleMa".to_string(),
            csv_path: Some("C:\\data\\AAPL.csv".to_string()),
            timeframe: Some("15m".to_string()),
            label: Some("AAPL".to_string()),
        };
        validate_validation_context(
            Some(&ctx),
            &loaded_ctx("C:\\data\\AAPL.csv", Some("15m")),
            &["DoubleMa"],
        )
        .expect_err("dynamic validation context should not masquerade as DoubleMa");
    }

    #[test]
    fn cross_ticker_headless_double_ma_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::DoubleMaValidationRequest {
            data_id: "base".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_double_ma_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base double-ma validation");
        assert!(!res.candidates.is_empty());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                    label: "NVDA".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("SPY.csv").to_string_lossy().to_string(),
                    label: "SPY".to_string(),
                },
            ],
            max_datasets: 2,
            mode: ta_app_core::CrossTickerMode::PreservedFinalists,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation_double_ma_headless(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker double-ma");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[test]
    fn cross_ticker_headless_double_ma_recomputes_run_verdict_after_filtering() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::DoubleMaValidationRequest {
            data_id: "base".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_double_ma_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base double-ma validation");
        let base_candidates_tested = res.run_verdict.candidates_tested.max(res.candidates.len());
        assert_eq!(base_candidates_tested, 1);

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::PreservedFinalists,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: true,
                min_datasets: 2,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation_double_ma_headless(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker double-ma");

        assert!(res.candidates.is_empty());
        assert_eq!(res.run_verdict.candidates_tested, base_candidates_tested);
        assert_eq!(res.run_verdict.approved_candidate_count, 0);
        assert!(!res.run_verdict.approved);
    }

    #[test]
    fn cross_ticker_headless_double_ma_repo_local_multi_dataset_summary_respects_cap() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::DoubleMaValidationRequest {
            data_id: "base-multi-cross".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_double_ma_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base double-ma validation");

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                    label: "NVDA".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("SPY.csv").to_string_lossy().to_string(),
                    label: "SPY".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("TSLA.csv").to_string_lossy().to_string(),
                    label: "TSLA".to_string(),
                },
            ],
            max_datasets: 2,
            mode: ta_app_core::CrossTickerMode::PreservedFinalists,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation_double_ma_headless(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker double-ma");

        let summary = res.cross_ticker.as_ref().expect("cross summary");
        assert_eq!(
            summary.mode,
            ta_app_core::CrossTickerMode::PreservedFinalists
        );
        assert_eq!(summary.datasets.len(), 2);
        assert_eq!(summary.datasets[0].label, "NVDA");
        assert_eq!(summary.datasets[1].label, "SPY");
        assert!(res.candidates.iter().all(|cand| cand
            .cross_ticker
            .as_ref()
            .map(|stats| stats.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_headless_double_ma_gpu_preserved_finalists_preserves_gpu_route() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let walk_forward = Some(ta_app_core::WalkForwardConfigOptional {
            window_bars: 20000,
            step_bars: 20000,
        });
        let cscv_pbo = Some(ta_app_core::CscvPboConfig {
            slices: 8,
            max_combinations: 512,
            threshold: 0.0,
        });
        let req = ta_app_core::DoubleMaValidationRequest {
            data_id: "base-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::DoubleMaValidationParams {
                fast_ma_type: "alma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_len: 9,
                slow_len: 30,
                fast_ma_params: Some(std::collections::HashMap::from([
                    ("offset".to_string(), 0.85),
                    ("sigma".to_string(), 6.0),
                ])),
                slow_ma_params: Some(std::collections::HashMap::from([
                    ("offset".to_string(), 0.85),
                    ("sigma".to_string(), 6.0),
                ])),
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
                halftrend_filter: None,
            }],
            walk_forward: walk_forward.clone(),
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: cscv_pbo.clone(),
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_double_ma_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base double-ma gpu validation");
        assert!(!res.candidates.is_empty());
        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::PreservedFinalists,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation_double_ma_headless(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            walk_forward,
            None,
            None,
            None,
            None,
            None,
            cscv_pbo,
            1,
            &make_cache_dir(),
            &cancel,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker double-ma gpu");

        assert_eq!(res.backend_route.as_deref(), Some("GpuKernel"));
        assert!(matches!(res.backend_used, ta_app_core::BackendUsed::Gpu));
        let summary = res.cross_ticker.as_ref().expect("cross summary");
        assert_eq!(
            summary.mode,
            ta_app_core::CrossTickerMode::PreservedFinalists
        );
        assert_eq!(summary.datasets.len(), 1);
        assert!(summary.datasets[0].bars <= candles.close.len());
    }

    #[test]
    fn cross_ticker_composite_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "base".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base composite validation");
        assert!(!res.candidates.is_empty());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                    label: "NVDA".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("SPY.csv").to_string_lossy().to_string(),
                    label: "SPY".to_string(),
                },
            ],
            max_datasets: 2,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker composite");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[test]
    fn cross_ticker_composite_preserved_finalists_recomputes_run_verdict_after_filtering() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "comp-preserved-finalists".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(ta_app_core::AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: Some(ta_app_core::CscvPboConfig {
                slices: 8,
                max_combinations: 256,
                threshold: 0.0,
            }),
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: Some(0.1),
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base composite validation");
        let base_candidates_tested = res.run_verdict.candidates_tested.max(res.candidates.len());
        let base_pbo = res.run_verdict.pbo;
        assert_eq!(base_candidates_tested, 1);

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::PreservedFinalists,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: true,
                min_datasets: 2,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            Some(ta_app_core::CscvPboConfig {
                slices: 8,
                max_combinations: 256,
                threshold: 0.0,
            }),
            Some(0.1),
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker composite preserved finalists");

        assert!(res.candidates.is_empty());
        assert_eq!(res.run_verdict.candidates_tested, base_candidates_tested);
        assert_eq!(res.run_verdict.approved_candidate_count, 0);
        assert!(!res.run_verdict.approved);
        assert_eq!(res.run_verdict.pbo, base_pbo);
        let summary = res.cross_ticker.as_ref().expect("cross summary");
        assert_eq!(
            summary.mode,
            ta_app_core::CrossTickerMode::PreservedFinalists
        );
    }

    #[test]
    fn cross_ticker_composite_repo_local_multi_dataset_summary_respects_cap() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "comp-base-multi-cross".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(ta_app_core::AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base composite validation");

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                    label: "NVDA".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("SPY.csv").to_string_lossy().to_string(),
                    label: "SPY".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: data_csv("QQQ.csv").to_string_lossy().to_string(),
                    label: "QQQ".to_string(),
                },
            ],
            max_datasets: 2,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker composite");

        let summary = res.cross_ticker.as_ref().expect("cross summary");
        assert_eq!(summary.mode, ta_app_core::CrossTickerMode::Lightweight);
        assert_eq!(summary.datasets.len(), 2);
        assert_eq!(summary.datasets[0].label, "NVDA");
        assert_eq!(summary.datasets[1].label, "SPY");
        assert!(res.candidates.iter().all(|cand| cand
            .cross_ticker
            .as_ref()
            .map(|stats| stats.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[test]
    fn cross_ticker_composite_mo_cpu_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "base-mo".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::EntryCoreExitOverlay,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base composite mo validation");
        assert!(!res.candidates.is_empty());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker composite mo");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[test]
    fn composite_cross_ticker_request_preserves_backend() {
        let req = build_composite_cross_ticker_request(
            1,
            7,
            vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(ta_app_core::AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            11,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert_eq!(req.data_id, "cross-1");
        assert_eq!(req.max_candidates, 7);
        assert!(matches!(
            req.backend,
            ta_app_core::Backend::GpuKernel { device_id: 0 }
        ));
        assert_eq!(req.candidates.len(), 1);
    }

    #[test]
    fn composite_cross_ticker_request_strips_full_validation_stack() {
        let req = build_composite_cross_ticker_request(
            2,
            9,
            vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(ta_app_core::AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            13,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        assert_eq!(req.data_id, "cross-2");
        assert!(req.walk_forward.is_none());
        assert!(req.reopt_walk_forward.is_none());
        assert!(req.spa.is_none());
        assert!(req.perturb.is_none());
        assert!(req.trade_bootstrap.is_none());
        assert!(req.cscv_pbo.is_none());
        assert!(req.cross_ticker.is_none());
    }

    #[test]
    fn composite_cross_ticker_request_preserves_verdict_relevant_stack() {
        let req = build_composite_cross_ticker_request(
            3,
            4,
            vec![ta_app_core::CompositeParams {
                fast_ma_type: "alma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_len: 9,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(ta_app_core::AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            19,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            Some(ta_app_core::CscvPboConfig {
                slices: 8,
                max_combinations: 256,
                threshold: 0.0,
            }),
            Some(0.6),
            Some(0.4),
            Some(0.5),
            Some(0.1),
            Some(0.7),
        );

        assert_eq!(req.data_id, "cross-3");
        assert_eq!(req.max_candidates, 4);
        assert!(req.walk_forward.is_none());
        assert!(req.reopt_walk_forward.is_none());
        assert!(req.spa.is_none());
        assert!(req.perturb.is_none());
        assert!(req.trade_bootstrap.is_none());
        assert!(req.cscv_pbo.is_some());
        assert_eq!(req.min_psr, Some(0.6));
        assert_eq!(req.min_dsr, Some(0.4));
        assert_eq!(req.min_cscv_oos_rank_frac, Some(0.5));
        assert_eq!(req.min_cscv_oos_metric, Some(0.1));
        assert_eq!(req.max_pbo, Some(0.7));
        assert!(req.cross_ticker.is_none());
    }

    #[test]
    fn double_ma_cross_ticker_request_preserves_requested_full_stack_modules() {
        let req = build_double_ma_cross_ticker_request(
            2,
            5,
            vec![ta_app_core::DoubleMaValidationParams {
                fast_ma_type: "alma".to_string(),
                slow_ma_type: "alma".to_string(),
                fast_len: 9,
                slow_len: 30,
                fast_ma_params: Some(std::collections::HashMap::from([
                    ("offset".to_string(), 0.85),
                    ("sigma".to_string(), 6.0),
                ])),
                slow_ma_params: Some(std::collections::HashMap::from([
                    ("offset".to_string(), 0.85),
                    ("sigma".to_string(), 6.0),
                ])),
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
                halftrend_filter: None,
            }],
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            Some(ta_app_core::WalkForwardConfigOptional {
                window_bars: 1000,
                step_bars: 250,
            }),
            Some(ta_app_core::ReoptWalkForwardConfigOptional {
                window_bars: 800,
                step_bars: 200,
                top_n_candidates: 3,
            }),
            Some(ta_app_core::SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 32,
                block_length: 2,
                benchmark: ta_app_core::SpaBenchmarkKind::BuyAndHold,
            }),
            Some(ta_app_core::PerturbConfig {
                iterations: 8,
                pct_min: -0.05,
                pct_max: 0.05,
                flip_bool_prob: 0.02,
            }),
            Some(ta_app_core::TradeBootstrapConfig {
                simulations: 64,
                stationary_mean_block_len: None,
            }),
            None,
            Some(ta_app_core::CscvPboConfig {
                slices: 8,
                max_combinations: 256,
                threshold: 0.0,
            }),
            17,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            Some(0.6),
            Some(0.4),
            Some(0.5),
            Some(0.1),
            Some(0.7),
        );

        assert_eq!(req.data_id, "cross-2");
        assert_eq!(req.max_candidates, 5);
        assert!(matches!(
            req.backend,
            ta_app_core::Backend::GpuKernel { device_id: 0 }
        ));
        assert!(req.walk_forward.is_some());
        assert!(req.reopt_walk_forward.is_some());
        assert!(req.spa.is_some());
        assert!(req.perturb.is_some());
        assert!(req.trade_bootstrap.is_some());
        assert!(req.cscv_pbo.is_some());
        assert_eq!(req.min_psr, Some(0.6));
        assert_eq!(req.min_dsr, Some(0.4));
        assert_eq!(req.min_cscv_oos_rank_frac, Some(0.5));
        assert_eq!(req.min_cscv_oos_metric, Some(0.1));
        assert_eq!(req.max_pbo, Some(0.7));
        assert!(req.cross_ticker.is_none());
    }

    #[test]
    fn double_ma_cross_ticker_selects_bounded_finalists() {
        let candidates: Vec<ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult> =
            (0..6)
                .map(
                    |idx| ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult {
                        params: ta_app_core::DoubleMaValidationParams {
                            fast_ma_type: "sma".to_string(),
                            slow_ma_type: "sma".to_string(),
                            fast_len: 10 + idx as u16,
                            slow_len: 30 + idx as u16,
                            fast_ma_params: None,
                            slow_ma_params: None,
                            strategy: ta_strategies::double_ma::StrategyConfig::default(),
                            halftrend_filter: None,
                        },
                        in_sample: ta_strategies::double_ma::Metrics {
                            pnl: 0.2 - idx as f64 * 0.01,
                            sharpe: 1.0 - idx as f64 * 0.01,
                            max_dd: 0.2,
                            trades: 10,
                            exposure: 0.5,
                            ..Default::default()
                        },
                        iterative_in_sample: None,
                        iterative_delta: None,
                        psr: Some(0.8),
                        dsr: Some(0.7),
                        track_record_bars: 300,
                        min_track_record_length_bars: Some(250),
                        walk_forward: None,
                        perturb: None,
                        trade_bootstrap: None,
                        monte_carlo: None,
                        regime_slice_robustness: None,
                        friction_stress: None,
                        entry_margin_fragility: None,
                        underwater_duration: None,
                        checkpoint_robustness: None,
                        benchmark_slice_summary: None,
                        cross_ticker: None,
                        cross_ticker_pareto: None,
                        cscv_pbo: None,
                        validation_verdict:
                            ta_app_core::validation_pipeline::ValidationCandidateVerdict {
                                passed: true,
                                warning_flags: Vec::new(),
                                fail_reasons: Vec::new(),
                            },
                        robust_score: 10.0 - idx as f64,
                        robust_pareto: idx < 4,
                    },
                )
                .collect();

        let finalists = select_cross_ticker_preserved_finalists(&candidates);
        assert_eq!(finalists.len(), PRESERVED_FINALISTS_CROSS_TICKER_LIMIT);
        assert_eq!(finalists[0].params.fast_len, 10);
        assert_eq!(finalists[3].params.fast_len, 13);
    }

    #[test]
    fn composite_cross_ticker_selects_bounded_finalists() {
        let candidates: Vec<ta_app_core::composite_validate::CompositeValidationCandidateResult> =
            (0..6)
                .map(
                    |idx| ta_app_core::composite_validate::CompositeValidationCandidateResult {
                        params: ta_app_core::CompositeParams {
                            fast_ma_type: "sma".to_string(),
                            slow_ma_type: "sma".to_string(),
                            fast_len: 10 + idx as u16,
                            slow_len: 30 + idx as u16,
                            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                                amplitude: 2,
                                channel_deviation: 2.0,
                                atr_period: 100,
                            },
                            asr: Some(ta_app_core::AsrStrategyParams::default()),
                            vcc: None,
                            vemr: None,
                            rocb: None,
                            mkcs: None,
                            kqsr: None,
                            carm: None,
                            kvdf: None,
                            cdmc: None,
                            vcet: None,
                            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                            ott: ta_app_core::composite_cpu::OttGateParams::default(),
                            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                            strategy: ta_strategies::double_ma::StrategyConfig::default(),
                        },
                        in_sample: ta_strategies::double_ma::Metrics {
                            pnl: 0.2 - idx as f64 * 0.01,
                            sharpe: 1.0 - idx as f64 * 0.01,
                            max_dd: 0.2,
                            trades: 10,
                            exposure: 0.5,
                            ..Default::default()
                        },
                        iterative_in_sample: None,
                        iterative_delta: None,
                        psr: Some(0.8),
                        dsr: Some(0.7),
                        track_record_bars: 300,
                        min_track_record_length_bars: Some(250),
                        walk_forward: None,
                        perturb: None,
                        trade_bootstrap: None,
                        monte_carlo: None,
                        regime_slice_robustness: None,
                        friction_stress: None,
                        cross_ticker: None,
                        cross_ticker_pareto: None,
                        cscv_pbo: None,
                        validation_verdict:
                            ta_app_core::validation_pipeline::ValidationCandidateVerdict {
                                passed: true,
                                warning_flags: Vec::new(),
                                fail_reasons: Vec::new(),
                            },
                        robust_score: 10.0 - idx as f64,
                        robust_pareto: idx < 4,
                    },
                )
                .collect();

        let finalists = select_cross_ticker_preserved_finalists(&candidates);
        assert_eq!(finalists.len(), PRESERVED_FINALISTS_CROSS_TICKER_LIMIT);
        assert_eq!(finalists[0].params.fast_len, 10);
        assert_eq!(finalists[3].params.fast_len, 13);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_composite_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "base-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: Some(ta_app_core::AsrStrategyParams::default()),
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base composite gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker composite gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_mkcs_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "mkcs-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: Some(ta_app_core::mkcs::MkcsStrategyParams::default()),
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base mkcs gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker mkcs gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_vcc_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "vcc-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: Some(ta_app_core::VccStrategyParams::default()),
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base vcc gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker vcc gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_rocb_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "rocb-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: Some(ta_app_core::RocbStrategyParams::default()),
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base rocb gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker rocb gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_vemr_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "vemr-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 8,
                slow_len: 24,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: Some(ta_app_core::VemrStrategyParams::default()),
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base vemr gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker vemr gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_kqsr_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "kqsr-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 10,
                slow_len: 34,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: Some(ta_app_core::kqsr::KqsrStrategyParams::default()),
                carm: None,
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base kqsr gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker kqsr gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_carm_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "carm-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 10,
                slow_len: 34,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: Some(ta_app_core::carm::CarmStrategyParams::default()),
                kvdf: None,
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base carm gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker carm gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_kvdf_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "kvdf-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 10,
                slow_len: 34,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: Some(ta_app_core::kvdf::KvdfStrategyParams::default()),
                cdmc: None,
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base kvdf gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker kvdf gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn cross_ticker_cdmc_gpu_kernel_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "cdmc-gpu".to_string(),
            backend: ta_app_core::Backend::GpuKernel { device_id: 0 },
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![ta_app_core::CompositeParams {
                fast_ma_type: "ema".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 10,
                slow_len: 34,
                halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                    amplitude: 2,
                    channel_deviation: 2.0,
                    atr_period: 100,
                },
                asr: None,
                vcc: None,
                vemr: None,
                rocb: None,
                mkcs: None,
                kqsr: None,
                carm: None,
                kvdf: None,
                cdmc: Some(ta_app_core::cdmc::CdmcStrategyParams::default()),
                vcet: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            monte_carlo: None,
            cross_ticker: None,
            cscv_pbo: None,
            spa: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles, &cancel, None,
        )
        .expect("base cdmc gpu validation");
        assert!(!res.candidates.is_empty());
        assert!(res.backend_route.is_some());

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: data_csv("NVDA.csv").to_string_lossy().to_string(),
                label: "NVDA".to_string(),
            }],
            max_datasets: 1,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            &candles,
            ta_app_core::Backend::GpuKernel { device_id: 0 },
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("cross-ticker cdmc gpu");

        assert!(res.cross_ticker.is_some());
        assert!(res.candidates.iter().all(|c| c
            .cross_ticker
            .as_ref()
            .map(|s| s.datasets)
            .unwrap_or(0)
            >= 1));
    }
}
