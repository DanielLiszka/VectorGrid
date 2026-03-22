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

const DOUBLE_MA_CROSS_TICKER_FINALIST_LIMIT: usize = 4;

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

fn align_cross_ticker_candles_to_base_horizon(
    base: &vector_ta::utilities::data_loader::Candles,
    cross: &vector_ta::utilities::data_loader::Candles,
) -> vector_ta::utilities::data_loader::Candles {
    let base_len = base.close.len();
    let cross_len = cross.close.len();
    if base_len == 0 || cross_len == 0 {
        return slice_candles(cross, 0, cross_len);
    }

    let base_start = *base.timestamp.first().unwrap_or(&0);
    let base_end = *base.timestamp.last().unwrap_or(&0);
    let start = cross.timestamp.partition_point(|&ts| ts < base_start);
    let end = cross.timestamp.partition_point(|&ts| ts <= base_end);
    if start < end {
        return slice_candles(cross, start, end);
    }

    if cross_len > base_len {
        return slice_candles(cross, cross_len.saturating_sub(base_len), cross_len);
    }

    slice_candles(cross, 0, cross_len)
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

fn normalize_path_key(raw: &str) -> String {
    raw.trim().replace('/', "\\").to_ascii_lowercase()
}

fn normalize_strategy_kind(raw: &str) -> String {
    match raw.trim() {
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
        _ => "DoubleMa".to_string(),
    }
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

    if let Some(expected_csv_path) = ctx.csv_path.as_deref() {
        let expected = normalize_path_key(expected_csv_path);
        let actual = normalize_path_key(&loaded.csv_path);
        if !expected.is_empty() && !actual.is_empty() && expected != actual {
            let label = ctx
                .label
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(expected_csv_path.trim());
            return Err(format!(
                "validation requires dataset '{label}' ({expected_csv_path}), but loaded data id points to '{}'",
                loaded.csv_path
            ));
        }
    }

    if let Some(expected_timeframe) = ctx.timeframe.as_deref() {
        let expected = normalize_timeframe_label(expected_timeframe);
        let actual = loaded
            .timeframe
            .as_deref()
            .map(normalize_timeframe_label)
            .unwrap_or_default();
        if !expected.is_empty() && !actual.is_empty() && expected != actual {
            return Err(format!(
                "validation requires timeframe '{expected}', but loaded data id has timeframe '{actual}'"
            ));
        }
    }

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

    let mut accs: Vec<validation_common::CrossTickerAcc> =
        vec![validation_common::CrossTickerAcc::default(); total_candidates];
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
            bars: candles2.close.len(),
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
                &candles2,
                cancel,
                Some(&sink2 as &dyn ta_app_core::progress::ProgressSink),
            )?
        } else {
            run_validate(req2, &candles2, cancel, None)?
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
    req: CompositeValidationRequest,
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
        let seed = base_req.seed;
        let validation_engine = base_req.validation_engine;
        let iterative_cfg = base_req.iterative.clone();
        let cross_cfg = base_req.cross_ticker.take();
        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            base_req,
            &candles,
            cancel2.as_ref(),
            Some(&sink),
        )?;

        if let Some(cross_cfg) = cross_cfg {
            apply_cross_ticker_validation(
                &mut res,
                cross_cfg,
                &candles,
                backend,
                &ma_source,
                fast_ma_params,
                slow_ma_params,
                seed,
                &app_cache_dir,
                cancel2.as_ref(),
                Some(sink.app.clone()),
                validation_engine,
                iterative_cfg,
            )?;
        }
        res.runtime_ms = started.elapsed().as_millis() as u64;
        Ok(res)
    })
    .await
    .map_err(|e| e.to_string())?
}

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
) -> Result<(), String> {
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
                seed,
                validation_engine,
                iterative_cfg.clone(),
            )
        },
        ta_app_core::run_composite_validation_blocking_with_candles,
    )
}

fn build_composite_cross_ticker_request(
    dataset_index: usize,
    total_candidates: usize,
    candidates: Vec<ta_app_core::CompositeParams>,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    seed: u64,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
) -> ta_app_core::CompositeValidationRequest {
    ta_app_core::CompositeValidationRequest {
        data_id: format!("cross-{dataset_index}"),
        backend,
        ma_source: ma_source.to_string(),
        fast_ma_params,
        slow_ma_params,
        candidates,
        walk_forward: None,
        reopt_walk_forward: None,
        spa: None,
        perturb: None,
        trade_bootstrap: None,
        cross_ticker: None,
        cscv_pbo: None,
        seed,
        max_candidates: total_candidates,
        validation_engine,
        iterative: iterative_cfg,
        min_psr: None,
        min_dsr: None,
        min_cscv_oos_rank_frac: None,
        min_cscv_oos_metric: None,
        max_pbo: None,
        validation_context: None,
    }
}

#[tauri::command]
pub async fn run_double_ma_validation(
    req: DoubleMaValidationRequest,
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
    _walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    _reopt_walk_forward: Option<ta_app_core::composite_validate::ReoptWalkForwardConfigOptional>,
    _spa: Option<ta_app_core::composite_validate::SpaConfigOptional>,
    _perturb: Option<ta_app_core::PerturbConfig>,
    _trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
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
    if preserved_cross_mode && res.candidates.len() > DOUBLE_MA_CROSS_TICKER_FINALIST_LIMIT {
        res.candidates = select_double_ma_cross_ticker_finalists(&res.candidates);
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
            // Cross-ticker aggregation only consumes candidate-level verdicts and
            // cross metrics. Preserve the verdict-relevant stack here rather than
            // rerunning non-gating evidence modules whose outputs are not used by
            // the cross filter or workbook summaries.
            build_double_ma_cross_ticker_request(
                di,
                total_candidates,
                candidates,
                backend.clone(),
                ma_source,
                fast_ma_params.clone(),
                slow_ma_params.clone(),
                None,
                None,
                None,
                None,
                None,
                if preserved_cross_mode {
                    cscv_pbo.clone()
                } else {
                    None
                },
                seed,
                validation_engine,
                iterative_cfg.clone(),
                if preserved_cross_mode { min_psr } else { None },
                if preserved_cross_mode { min_dsr } else { None },
                if preserved_cross_mode {
                    min_cscv_oos_rank_frac
                } else {
                    None
                },
                if preserved_cross_mode {
                    min_cscv_oos_metric
                } else {
                    None
                },
                if preserved_cross_mode { max_pbo } else { None },
            )
        },
        ta_app_core::run_double_ma_validation_blocking_with_candles,
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
        res.candidates.len(),
        &hard_gates,
        res.cscv_pbo.as_ref(),
        res.spa.as_ref(),
    );
    Ok(())
}

fn double_ma_candidate_is_approved(
    cand: &ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult,
) -> bool {
    cand.validation_verdict.passed
        && cand
            .min_track_record_length_bars
            .map(|mintrl| cand.track_record_bars.max(2) >= mintrl)
            .unwrap_or(false)
}

fn select_double_ma_cross_ticker_finalists(
    candidates: &[ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult],
) -> Vec<ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult> {
    let mut approved: Vec<_> = candidates
        .iter()
        .filter(|cand| double_ma_candidate_is_approved(cand))
        .cloned()
        .collect();
    if !approved.is_empty() {
        approved.truncate(DOUBLE_MA_CROSS_TICKER_FINALIST_LIMIT);
        return approved;
    }

    let mut hard_gate_survivors: Vec<_> = candidates
        .iter()
        .filter(|cand| cand.validation_verdict.passed)
        .cloned()
        .collect();
    if !hard_gate_survivors.is_empty() {
        hard_gate_survivors.truncate(DOUBLE_MA_CROSS_TICKER_FINALIST_LIMIT);
        return hard_gate_survivors;
    }

    let mut fallback = candidates.to_vec();
    fallback.truncate(DOUBLE_MA_CROSS_TICKER_FINALIST_LIMIT);
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
    reopt_walk_forward: Option<ta_app_core::composite_validate::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::composite_validate::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
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

/// Headless helper for applying cross-ticker validation to an already computed
/// `DoubleMaValidationResult` without requiring a running Tauri app.
pub fn apply_cross_ticker_validation_double_ma_headless(
    res: &mut DoubleMaValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
    base_candles: &vector_ta::utilities::data_loader::Candles,
    backend: ta_app_core::Backend,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    walk_forward: Option<ta_app_core::WalkForwardConfigOptional>,
    reopt_walk_forward: Option<ta_app_core::composite_validate::ReoptWalkForwardConfigOptional>,
    spa: Option<ta_app_core::composite_validate::SpaConfigOptional>,
    perturb: Option<ta_app_core::PerturbConfig>,
    trade_bootstrap: Option<ta_app_core::TradeBootstrapConfig>,
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
        for i in from..to_exclusive {
            let o = candles.open[i];
            if o.is_finite() {
                open = o;
                break;
            }
            let c = candles.close[i];
            if c.is_finite() {
                open = c;
                break;
            }
        }

        let mut close = f64::NAN;
        for i in (from..to_exclusive).rev() {
            let c = candles.close[i];
            if c.is_finite() {
                close = c;
                break;
            }
            let o = candles.open[i];
            if o.is_finite() {
                close = o;
                break;
            }
        }

        let mut high = f64::NEG_INFINITY;
        let mut low = f64::INFINITY;
        let mut volume = 0.0f64;
        for i in from..to_exclusive {
            for v in [
                candles.high[i],
                candles.open[i],
                candles.close[i],
                candles.low[i],
            ] {
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
            if prev_time.map_or(true, |p| time > p) {
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
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;

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

    fn loaded_ctx(path: &str, timeframe: Option<&str>) -> LoadedDataContext {
        LoadedDataContext {
            csv_path: path.to_string(),
            label: "loaded".to_string(),
            timeframe: timeframe.map(str::to_string),
            source_identity: None,
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
    fn cross_ticker_alignment_uses_base_timestamp_overlap() {
        let base = candles_with_ts(100, 10);
        let cross = candles_with_ts(90, 30);
        let aligned = align_cross_ticker_candles_to_base_horizon(&base, &cross);
        assert_eq!(aligned.timestamp.first().copied(), Some(100));
        assert_eq!(aligned.timestamp.last().copied(), Some(109));
        assert_eq!(aligned.close.len(), 10);
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
        assert_eq!(res.run_verdict.approved_candidate_count, 1);

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
            11,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
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
            13,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
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
    fn double_ma_cross_ticker_request_preserves_verdict_relevant_stack() {
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
            Some(
                ta_app_core::composite_validate::ReoptWalkForwardConfigOptional {
                    window_bars: 800,
                    step_bars: 200,
                    top_n_candidates: 3,
                },
            ),
            Some(ta_app_core::composite_validate::SpaConfigOptional {
                slices: 6,
                bootstrap_samples: 32,
                block_length: 2,
                benchmark: ta_app_core::composite_validate::SpaBenchmarkKind::BuyAndHold,
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

        let finalists = select_double_ma_cross_ticker_finalists(&candidates);
        assert_eq!(finalists.len(), DOUBLE_MA_CROSS_TICKER_FINALIST_LIMIT);
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            reopt_walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
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
