use super::validation_common::CrossTickerCandidate;
use super::{
    validation_common, CompositeValidationRequest, CompositeValidationResult,
    DoubleMaValidationRequest, DoubleMaValidationResult, OffsetProgressSink, TauriProgressSink,
};
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use tauri::Manager;

trait CrossTickerValidationResultExt {
    type Candidate: validation_common::CrossTickerCandidate;

    fn candidates(&self) -> &Vec<Self::Candidate>;
    fn candidates_mut(&mut self) -> &mut Vec<Self::Candidate>;
    fn set_cross_ticker_summary(&mut self, summary: ta_app_core::CrossTickerValidationSummary);
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
        let candles2 = outcome.candles;
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
            validation_common::push_metrics(&mut accs[idx], c.in_sample_metrics());
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
    let candles = state.get_candles(&req.data_id)?;

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let sink = TauriProgressSink { app: app2 };
        let mut base_req = req;
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
        app_cache_dir,
        cancel,
        app_for_progress,
        "composite_validate_cross",
        |di, total_candidates, candidates| ta_app_core::CompositeValidationRequest {
            data_id: format!("cross-{di}"),
            ma_source: ma_source.to_string(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
            candidates,
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed,
            max_candidates: total_candidates,
            validation_engine,
            iterative: iterative_cfg.clone(),
            min_psr: None,
            min_dsr: None,
        },
        ta_app_core::run_composite_validation_blocking_with_candles,
    )
}

#[tauri::command]
pub async fn run_double_ma_validation(
    req: DoubleMaValidationRequest,
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<DoubleMaValidationResult, String> {
    let _run_guard = state.try_begin_run()?;
    let cancel = state.cancel_token();
    let candles = state.get_candles(&req.data_id)?;

    let base = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = crate::data_cache::app_data_cache_dir(&base);

    let cancel2 = cancel.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let started = std::time::Instant::now();
        let sink = TauriProgressSink { app: app2 };
        let mut base_req = req;
        let ma_source = base_req.ma_source.clone();
        let fast_ma_params = base_req.fast_ma_params.clone();
        let slow_ma_params = base_req.slow_ma_params.clone();
        let seed = base_req.seed;
        let validation_engine = base_req.validation_engine;
        let iterative_cfg = base_req.iterative.clone();
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

fn apply_cross_ticker_validation_double_ma(
    res: &mut DoubleMaValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
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
        app_cache_dir,
        cancel,
        app_for_progress,
        "double_ma_validate_cross",
        |di, total_candidates, candidates| ta_app_core::DoubleMaValidationRequest {
            data_id: format!("cross-{di}"),
            ma_source: ma_source.to_string(),
            fast_ma_params: fast_ma_params.clone(),
            slow_ma_params: slow_ma_params.clone(),
            candidates,
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed,
            max_candidates: total_candidates,
            validation_engine,
            iterative: iterative_cfg.clone(),
            min_psr: None,
            min_dsr: None,
        },
        ta_app_core::run_double_ma_validation_blocking_with_candles,
    )
}

/// Headless helper for applying cross-ticker validation to an already computed
/// `DoubleMaValidationResult` without requiring a running Tauri app.
pub fn apply_cross_ticker_validation_double_ma_headless(
    res: &mut DoubleMaValidationResult,
    cross_cfg: ta_app_core::CrossTickerConfig,
    ma_source: &str,
    fast_ma_params: Option<HashMap<String, f64>>,
    slow_ma_params: Option<HashMap<String, f64>>,
    seed: u64,
    app_cache_dir: &Path,
    cancel: &AtomicBool,
    validation_engine: ta_app_core::ValidationEngine,
    iterative_cfg: ta_app_core::IterativeSimConfig,
) -> Result<(), String> {
    apply_cross_ticker_validation_double_ma(
        res,
        cross_cfg,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        seed,
        app_cache_dir,
        cancel,
        None,
        validation_engine,
        iterative_cfg,
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

    #[test]
    fn cross_ticker_headless_double_ma_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::DoubleMaValidationRequest {
            data_id: "base".to_string(),
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
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
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
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation_double_ma_headless(
            &mut res,
            cross_cfg,
            "close",
            None,
            None,
            1,
            &make_cache_dir(),
            &cancel,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
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
    fn cross_ticker_composite_smoke_applies_stats() {
        let candles = load_candles(&data_csv("AAPL.csv"));
        let cancel = AtomicBool::new(false);
        let req = ta_app_core::CompositeValidationRequest {
            data_id: "base".to_string(),
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
                vemr: None,
                macz: ta_app_core::composite_cpu::MaczGateParams::default(),
                ott: ta_app_core::composite_cpu::OttGateParams::default(),
                signal_mode: ta_app_core::CompositeSignalMode::HardGate,
                strategy: ta_strategies::double_ma::StrategyConfig::default(),
            }],
            walk_forward: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 8,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
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
            gate: ta_app_core::CrossTickerGateConfig {
                enabled: false,
                ..ta_app_core::CrossTickerGateConfig::default()
            },
        };

        apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
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
}
