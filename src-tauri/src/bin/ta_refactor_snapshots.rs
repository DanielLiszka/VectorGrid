use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use ta_app_core::{
    read_candles_from_csv_compat, CompositeParams, CompositeValidationRequest, CscvPboConfig,
    DoubleMaValidationParams, DoubleMaValidationRequest, MetaClusterConfig, MetaGroupingMode,
    MetaHoldoutMode, MetaScoreWeights, MetaSelectionRequest, ValidationEngine,
};
use ta_strategies::composite_cpu::CompositeSignalMode;
use ta_strategies::double_ma::StrategyConfig;

fn data_csv(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
        .join(name)
}

fn canonicalize_json(v: &Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                if let Some(v2) = map.get(k) {
                    out.insert(k.clone(), canonicalize_json(v2));
                }
            }
            Value::Object(out)
        }
        Value::Array(xs) => Value::Array(xs.iter().map(canonicalize_json).collect()),
        _ => v.clone(),
    }
}

fn write_json(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let txt = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
    std::fs::write(path, txt).map_err(|e| e.to_string())
}

fn to_report_artifact(
    strategy_kind: &str,
    label: &str,
    csv_path: &Path,
    validation_req: Value,
    validation_res: Value,
) -> Value {
    json!({
        "kind": "vectorbt_run_artifact",
        "run": { "req": { "strategy_kind": strategy_kind }, "res": {} },
        "dataset_meta": {
            "label": label,
            "timeframe": "1m",
            "csv_path": csv_path.to_string_lossy().to_string()
        },
        "validation": {
            "req": validation_req,
            "res": validation_res
        }
    })
}

fn double_validation_req_json(req: &DoubleMaValidationRequest) -> Value {
    let cscv_pbo = req.cscv_pbo.as_ref().map_or(Value::Null, |cfg| {
        json!({
            "slices": cfg.slices,
            "max_combinations": cfg.max_combinations,
            "threshold": cfg.threshold
        })
    });
    json!({
        "data_id": req.data_id,
        "ma_source": req.ma_source,
        "fast_ma_params": req.fast_ma_params,
        "slow_ma_params": req.slow_ma_params,
        "candidates": req.candidates,
        "walk_forward": req.walk_forward,
        "perturb": req.perturb,
        "trade_bootstrap": req.trade_bootstrap,
        "cross_ticker": Value::Null,
        "cscv_pbo": cscv_pbo,
        "seed": req.seed,
        "max_candidates": req.max_candidates,
        "validation_engine": req.validation_engine,
        "iterative": req.iterative,
        "min_psr": req.min_psr,
        "min_dsr": req.min_dsr
    })
}

fn composite_validation_req_json(req: &CompositeValidationRequest) -> Value {
    let cscv_pbo = req.cscv_pbo.as_ref().map_or(Value::Null, |cfg| {
        json!({
            "slices": cfg.slices,
            "max_combinations": cfg.max_combinations,
            "threshold": cfg.threshold
        })
    });
    json!({
        "data_id": req.data_id,
        "ma_source": req.ma_source,
        "fast_ma_params": req.fast_ma_params,
        "slow_ma_params": req.slow_ma_params,
        "candidates": req.candidates,
        "walk_forward": req.walk_forward,
        "perturb": req.perturb,
        "trade_bootstrap": req.trade_bootstrap,
        "cross_ticker": Value::Null,
        "cscv_pbo": cscv_pbo,
        "seed": req.seed,
        "max_candidates": req.max_candidates,
        "validation_engine": req.validation_engine,
        "iterative": req.iterative,
        "min_psr": req.min_psr,
        "min_dsr": req.min_dsr
    })
}

fn main() -> Result<(), String> {
    let out_dir = PathBuf::from("_write_test")
        .join("refactor_snapshots")
        .join("validation_meta");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let cancel = AtomicBool::new(false);

    let aapl_path = data_csv("AAPL.csv");
    let nvda_path = data_csv("NVDA.csv");

    let candles_aapl =
        read_candles_from_csv_compat(aapl_path.to_str().ok_or("invalid AAPL csv path")?)?;
    let candles_nvda =
        read_candles_from_csv_compat(nvda_path.to_str().ok_or("invalid NVDA csv path")?)?;

    // Double-MA validation snapshot
    let double_req = DoubleMaValidationRequest {
        data_id: "AAPL".to_string(),
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: vec![
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "sma".to_string(),
                fast_len: 10,
                slow_len: 30,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
            DoubleMaValidationParams {
                fast_ma_type: "sma".to_string(),
                slow_ma_type: "ema".to_string(),
                fast_len: 15,
                slow_len: 60,
                fast_ma_params: None,
                slow_ma_params: None,
                strategy: StrategyConfig::default(),
                halftrend_filter: None,
            },
        ],
        walk_forward: Some(ta_app_core::WalkForwardConfigOptional {
            window_bars: 3_000,
            step_bars: 3_000,
        }),
        perturb: Some(ta_app_core::PerturbConfig {
            iterations: 16,
            pct_min: 0.01,
            pct_max: 0.03,
            flip_bool_prob: 0.02,
        }),
        trade_bootstrap: Some(ta_app_core::TradeBootstrapConfig { simulations: 128 }),
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: 8,
            max_combinations: 512,
            threshold: 0.0,
        }),
        seed: 1,
        max_candidates: 32,
        validation_engine: ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        validation_context: None,
    };

    let double_res = ta_app_core::run_double_ma_validation_blocking_with_candles(
        double_req.clone(),
        &candles_aapl,
        &cancel,
        None,
    )?;

    let double_artifact = to_report_artifact(
        "DoubleMa",
        "AAPL",
        &aapl_path,
        double_validation_req_json(&double_req),
        serde_json::to_value(&double_res).map_err(|e| e.to_string())?,
    );
    write_json(
        &out_dir.join("double_ma_validation_artifact.json"),
        &double_artifact,
    )?;
    write_json(
        &out_dir.join("double_ma_validation_artifact.canonical.json"),
        &canonicalize_json(&double_artifact),
    )?;

    // Composite validation snapshots on two datasets (used for meta-select artifact set)
    let composite_candidate = CompositeParams {
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
        macz: ta_app_core::composite_cpu::MaczGateParams::default(),
        ott: ta_app_core::composite_cpu::OttGateParams::default(),
        signal_mode: CompositeSignalMode::HardGate,
        strategy: StrategyConfig::default(),
    };

    let composite_req_aapl = CompositeValidationRequest {
        data_id: "AAPL".to_string(),
        ma_source: "close".to_string(),
        fast_ma_params: None,
        slow_ma_params: None,
        candidates: vec![composite_candidate.clone()],
        walk_forward: Some(ta_app_core::WalkForwardConfigOptional {
            window_bars: 3_000,
            step_bars: 3_000,
        }),
        perturb: Some(ta_app_core::PerturbConfig {
            iterations: 16,
            pct_min: 0.01,
            pct_max: 0.03,
            flip_bool_prob: 0.02,
        }),
        trade_bootstrap: Some(ta_app_core::TradeBootstrapConfig { simulations: 128 }),
        cross_ticker: None,
        cscv_pbo: Some(CscvPboConfig {
            slices: 8,
            max_combinations: 512,
            threshold: 0.0,
        }),
        seed: 1,
        max_candidates: 32,
        validation_engine: ValidationEngine::Vectorized,
        iterative: ta_app_core::IterativeSimConfig::default(),
        min_psr: None,
        min_dsr: None,
        validation_context: None,
    };
    let composite_req_nvda = CompositeValidationRequest {
        data_id: "NVDA".to_string(),
        ..composite_req_aapl.clone()
    };

    let composite_res_aapl = ta_app_core::run_composite_validation_blocking_with_candles(
        composite_req_aapl.clone(),
        &candles_aapl,
        &cancel,
        None,
    )?;
    let composite_res_nvda = ta_app_core::run_composite_validation_blocking_with_candles(
        composite_req_nvda.clone(),
        &candles_nvda,
        &cancel,
        None,
    )?;

    let composite_artifact = to_report_artifact(
        "CompositeMo",
        "AAPL",
        &aapl_path,
        composite_validation_req_json(&composite_req_aapl),
        serde_json::to_value(&composite_res_aapl).map_err(|e| e.to_string())?,
    );
    write_json(
        &out_dir.join("composite_validation_artifact.json"),
        &composite_artifact,
    )?;
    write_json(
        &out_dir.join("composite_validation_artifact.canonical.json"),
        &canonicalize_json(&composite_artifact),
    )?;

    // Meta-select snapshot built from two composite artifacts
    let art_aapl = to_report_artifact(
        "CompositeMo",
        "AAPL",
        &aapl_path,
        composite_validation_req_json(&composite_req_aapl),
        serde_json::to_value(&composite_res_aapl).map_err(|e| e.to_string())?,
    );
    let art_nvda = to_report_artifact(
        "CompositeMo",
        "NVDA",
        &nvda_path,
        composite_validation_req_json(&composite_req_nvda),
        serde_json::to_value(&composite_res_nvda).map_err(|e| e.to_string())?,
    );

    let meta_req = MetaSelectionRequest {
        strategy_kind: "CompositeMo".to_string(),
        timeframe: Some("1m".to_string()),
        required_tag: None,
        holdout_tag: None,
        eval_holdout: false,
        save_holdout_rows: false,
        holdout_eval_top_n: None,
        holdout_eval_explore_n: 0,
        holdout_eval_rank_by: None,
        holdout_mode: MetaHoldoutMode::Light,
        holdout_validation: None,
        top_k: 16,
        min_samples: 1,
        strict_shortlist: false,
        allow_shortlist_fallback: true,
        weights: MetaScoreWeights::default(),
        grouping_mode: MetaGroupingMode::Clustered,
        cluster_config: MetaClusterConfig::default(),
        cluster_representatives: 2,
    };

    let meta_res = ta_app_core::meta_select_from_artifacts(&meta_req, &[art_aapl, art_nvda], None);
    let meta_artifact = json!({
        "kind": "vectorbt_meta_artifact",
        "req": meta_req,
        "res": meta_res
    });
    write_json(
        &out_dir.join("meta_select_output_artifact.json"),
        &meta_artifact,
    )?;
    write_json(
        &out_dir.join("meta_select_output_artifact.canonical.json"),
        &canonicalize_json(&meta_artifact),
    )?;

    println!("Wrote snapshot artifacts to {}", out_dir.to_string_lossy());
    Ok(())
}
