use clap::{Parser, ValueEnum};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;

use ta_app_core::{
    parse_timeframe_label, read_candles_from_csv_compat, resample_candles, Backend, DoubleMaRequest,
};
use ta_optimizer::{ObjectiveKind, OptimizationMode};
use ta_strategies::double_ma::StrategyConfig;

use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Copy, Debug)]
struct U32Range {
    start: u32,
    end: u32,
    step: u32,
}

impl FromStr for U32Range {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                let v = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start: v,
                    end: v,
                    step: 0,
                })
            }
            2 => {
                let start = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                let end = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self {
                    start,
                    end,
                    step: 1,
                })
            }
            3 => {
                let start = parts[0].parse::<u32>().map_err(|e| e.to_string())?;
                let end = parts[1].parse::<u32>().map_err(|e| e.to_string())?;
                let step = parts[2].parse::<u32>().map_err(|e| e.to_string())?;
                Ok(Self { start, end, step })
            }
            _ => Err("range must be N or start:end or start:end:step".to_string()),
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
enum BackendArg {
    Auto,
    Cpu,
    GpuSweep,
    GpuHybridKernel,
    GpuKernel,
}

#[derive(ValueEnum, Clone, Debug)]
enum ObjectiveArg {
    Pnl,
    Sharpe,
    MaxDrawdown,
}

#[derive(Parser, Debug)]
#[command(name = "ta_cli")]
struct Cli {
    #[arg(long)]
    csv: Option<PathBuf>,

    #[arg(long, default_value_t = 200_000)]
    synth_len: usize,

    #[arg(long, value_enum, default_value_t = BackendArg::Cpu)]
    backend: BackendArg,

    #[arg(long, default_value = "sma")]
    fast_ma: String,

    #[arg(long, default_value = "sma")]
    slow_ma: String,

    #[arg(long, default_value = "close")]
    ma_source: String,

    #[arg(long)]
    timeframe: Option<String>,

    #[arg(long, default_value = "5:50:1")]
    fast_period: U32Range,

    #[arg(long, default_value = "20:200:5")]
    slow_period: U32Range,

    #[arg(long, value_enum, default_value_t = ObjectiveArg::Sharpe)]
    objective: ObjectiveArg,

    #[arg(long, default_value_t = 50)]
    top_k: usize,

    #[arg(long, default_value_t = 64)]
    heatmap_bins: u16,

    #[arg(long, default_value_t = 0.85)]
    alma_offset: f64,

    #[arg(long, default_value_t = 6.0)]
    alma_sigma: f64,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    long_only: bool,

    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    allow_flip: bool,

    #[arg(long, default_value_t = false)]
    include_all: bool,

    #[arg(long)]
    export_all_csv_path: Option<PathBuf>,

    #[arg(long, default_value_t = 0)]
    halftrend_amplitude: u32,

    #[arg(long, default_value_t = 2.0)]
    halftrend_channel_deviation: f64,

    #[arg(long, default_value_t = 100)]
    halftrend_atr_period: u32,
}

fn make_synth_candles(len: usize) -> Candles {
    let mut ts = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol: Vec<f64> = Vec::with_capacity(len);

    let mut px = 100.0f64;
    for i in 0..len {
        ts.push(i as i64);
        let drift = 0.00005;
        let noise = (i as f64 * 0.017).sin() * 0.001;
        px *= 1.0 + drift + noise;

        open.push(px);
        high.push(px * 1.001);
        low.push(px * 0.999);
        close.push(px);
        vol.push(1_000.0);
    }

    Candles::new(ts, open, high, low, close, vol)
}

fn validate_cli_configuration(cli: &Cli, backend: &Backend) -> Result<(), String> {
    if cli.export_all_csv_path.is_some() && cli.include_all {
        return Err(
            "export_all_csv_path requires include_all=false (stream export avoids huge RAM usage)"
                .to_string(),
        );
    }
    if cli.export_all_csv_path.is_some()
        && !matches!(
            backend,
            Backend::GpuKernel { .. } | Backend::GpuHybridKernel { .. } | Backend::Auto { .. }
        )
    {
        return Err(
            "export_all_csv_path requires backend=gpu-kernel/gpu-hybrid-kernel (or auto that resolves to kernel)"
                .to_string(),
        );
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();

    let mut candles = if let Some(path) = cli.csv.as_ref() {
        read_candles_from_csv_compat(
            path.to_str()
                .ok_or_else(|| "invalid csv path".to_string())?,
        )?
    } else {
        make_synth_candles(cli.synth_len)
    };

    if let Some(raw_tf) = cli.timeframe.as_deref() {
        let tf = parse_timeframe_label(raw_tf)?;
        candles = resample_candles(&candles, tf.interval_ms)?;
    }

    let backend = match cli.backend {
        BackendArg::Auto => Backend::Auto { device_id: 0 },
        BackendArg::Cpu => Backend::CpuOnly,
        BackendArg::GpuSweep => Backend::GpuOnly { device_id: 0 },
        BackendArg::GpuHybridKernel => Backend::GpuHybridKernel { device_id: 0 },
        BackendArg::GpuKernel => Backend::GpuKernel { device_id: 0 },
    };

    let objective = match cli.objective {
        ObjectiveArg::Pnl => ObjectiveKind::Pnl,
        ObjectiveArg::Sharpe => ObjectiveKind::Sharpe,
        ObjectiveArg::MaxDrawdown => ObjectiveKind::MaxDrawdown,
    };
    validate_cli_configuration(&cli, &backend)?;

    let fast_ma_params = if cli.fast_ma.trim().eq_ignore_ascii_case("alma") {
        let mut p: HashMap<String, f64> = HashMap::new();
        p.insert("offset".to_string(), cli.alma_offset);
        p.insert("sigma".to_string(), cli.alma_sigma);
        Some(p)
    } else {
        None
    };

    let slow_ma_params = if cli.slow_ma.trim().eq_ignore_ascii_case("alma") {
        let mut p: HashMap<String, f64> = HashMap::new();
        p.insert("offset".to_string(), cli.alma_offset);
        p.insert("sigma".to_string(), cli.alma_sigma);
        Some(p)
    } else {
        None
    };

    let halftrend_filter = if cli.halftrend_amplitude == 0 {
        None
    } else {
        if cli.halftrend_amplitude > u16::MAX as u32 {
            return Err("halftrend_amplitude exceeds u16::MAX".to_string());
        }
        if cli.halftrend_atr_period == 0 || cli.halftrend_atr_period > u16::MAX as u32 {
            return Err("halftrend_atr_period must be in [1, u16::MAX]".to_string());
        }
        Some(ta_app_core::double_ma::HalfTrendRegimeFilter {
            amplitude: cli.halftrend_amplitude as u16,
            channel_deviation: cli.halftrend_channel_deviation,
            atr_period: cli.halftrend_atr_period as u16,
        })
    };

    let mut strategy = StrategyConfig::default();
    strategy.long_only = cli.long_only;
    strategy.allow_flip = cli.allow_flip;

    let req = DoubleMaRequest {
        backend,
        data_id: "cli".to_string(),
        fast_range: (
            cli.fast_period.start,
            cli.fast_period.end,
            cli.fast_period.step,
        ),
        slow_range: (
            cli.slow_period.start,
            cli.slow_period.end,
            cli.slow_period.step,
        ),
        fast_ma_types: vec![cli.fast_ma],
        slow_ma_types: vec![cli.slow_ma],
        ma_source: cli.ma_source,
        export_all_csv_path: cli
            .export_all_csv_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        fast_ma_params,
        slow_ma_params,
        fast_ma_param_sweeps: None,
        slow_ma_param_sweeps: None,
        strategy,
        halftrend_filter,
        objective,
        score_mode: None,
        extra_shortlist_objectives: None,
        shortlist_confidence_mode: None,
        shortlist_union_mode: None,
        shortlist_slice_mode: None,
        shortlist_slice_benchmark: None,
        shortlist_neighborhood_radius: None,
        shortlist_neighborhood_step: None,
        mode: OptimizationMode::Grid,
        top_k: Some(cli.top_k),
        include_all: Some(cli.include_all),
        heatmap_bins: Some(cli.heatmap_bins),
        constraints: None,
        vdps: None,
        dynamic_strategy_ir: None,
        gpu_execution: Default::default(),
        persist_followup_cache: false,
        include_trade_metrics: false,
        metric_contract: None,

        hurdle_tau: None,
    };

    let cancel = AtomicBool::new(false);
    let res = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req, &candles, &cancel, None,
    )?;

    println!(
        "{}",
        serde_json::to_string_pretty(&res).map_err(|e| e.to_string())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn u32_range_parses_single_end_and_step_forms() {
        let single = U32Range::from_str("7").expect("single value");
        assert_eq!(single.start, 7);
        assert_eq!(single.end, 7);
        assert_eq!(single.step, 0);

        let stepped = U32Range::from_str("5:25:5").expect("stepped range");
        assert_eq!(stepped.start, 5);
        assert_eq!(stepped.end, 25);
        assert_eq!(stepped.step, 5);
    }

    #[test]
    fn validate_cli_configuration_rejects_export_all_conflicts() {
        let include_all_cli = Cli::try_parse_from([
            "ta_cli",
            "--csv",
            "C:/data/AAPL.csv",
            "--include-all",
            "--export-all-csv-path",
            "C:/tmp/out.csv",
        ])
        .expect("cli should parse");
        let err =
            validate_cli_configuration(&include_all_cli, &Backend::GpuKernel { device_id: 0 })
                .expect_err("include_all conflict should fail");
        assert!(err.contains("include_all=false"));

        let cpu_cli = Cli::try_parse_from([
            "ta_cli",
            "--csv",
            "C:/data/AAPL.csv",
            "--export-all-csv-path",
            "C:/tmp/out.csv",
            "--backend",
            "cpu",
        ])
        .expect("cli should parse");
        let err = validate_cli_configuration(&cpu_cli, &Backend::CpuOnly)
            .expect_err("cpu backend should fail");
        assert!(err.contains("backend=gpu-kernel"));
    }
}
