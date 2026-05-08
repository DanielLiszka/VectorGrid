#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[path = "../bin_support/atomic_output.rs"]
mod atomic_output;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[path = "../bin_support/benchmark_lock.rs"]
mod benchmark_lock;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
mod imp {
    use serde::Serialize;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;
    use ta_app_core::double_ma::GpuExecutionPolicy;
    use ta_app_core::progress::{
        DoubleMaCampaignVramPrefetchFeedback, DoubleMaProgressPayload, ProgressSink,
    };
    use ta_app_core::{
        run_double_ma_optimization_blocking_with_candles, Backend, BackendUsed, DoubleMaRequest,
    };
    use ta_optimizer::{ObjectiveKind, OptimizationMode};
    use ta_strategies::double_ma::StrategyConfig;
    use vector_ta::utilities::data_loader::Candles;

    #[derive(Clone, Copy, Debug)]
    enum Mode {
        HostOnly,
        Staged,
    }

    #[derive(Clone, Debug)]
    struct Config {
        mode: Mode,
        len: usize,
        warmup_iterations: usize,
        measure_iterations: usize,
        device_id: u32,
        fast_ma_type: String,
        slow_ma_type: String,
        ma_source: String,
        fast_range: (u32, u32, u32),
        slow_range: (u32, u32, u32),
        top_k: usize,
        output_json: Option<String>,
    }

    #[derive(Clone, Copy, Debug, Serialize)]
    #[serde(rename_all = "snake_case")]
    enum StageOutcomeLabel {
        NotAttempted,
        Staged,
        AlreadyResident,
        Skipped,
        Error,
    }

    #[derive(Clone, Debug, Serialize)]
    struct IterationSummary {
        iteration: usize,
        first_wall_ms: f64,
        second_wall_ms: f64,
        pair_wall_ms: f64,
        first_minus_second_wall_ms: f64,
        first_backend_route: Option<String>,
        second_backend_route: Option<String>,
        first_backend_used: String,
        second_backend_used: String,
        stage_outcome: StageOutcomeLabel,
        stage_wall_ms: f64,
        stage_note: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct RunSummary {
        mode: String,
        device_id: u32,
        len: usize,
        fast_range: (u32, u32, u32),
        slow_range: (u32, u32, u32),
        fast_ma_type: String,
        slow_ma_type: String,
        ma_source: String,
        warmup_iterations: usize,
        measure_iterations: usize,
        stage_attempts: usize,
        stage_successes: usize,
        stage_already_resident: usize,
        stage_skips: usize,
        stage_errors: usize,
        stage_success_rate: f64,
        stage_error_rate: f64,
        mean_first_wall_ms: f64,
        mean_second_wall_ms: f64,
        mean_pair_wall_ms: f64,
        mean_first_minus_second_wall_ms: f64,
        mean_stage_wall_ms: Option<f64>,
        measured_iterations: Vec<IterationSummary>,
    }

    struct StageNextDatasetSink<'a> {
        device_id: u32,
        next_candles: &'a Candles,
        fast_ma_type: &'a str,
        slow_ma_type: &'a str,
        ma_source: &'a str,
        attempted: AtomicBool,
        outcome: Mutex<StageOutcomeLabel>,
        stage_wall_ms: Mutex<f64>,
        note: Mutex<Option<String>>,
    }

    impl<'a> StageNextDatasetSink<'a> {
        fn new(
            device_id: u32,
            next_candles: &'a Candles,
            fast_ma_type: &'a str,
            slow_ma_type: &'a str,
            ma_source: &'a str,
        ) -> Self {
            Self {
                device_id,
                next_candles,
                fast_ma_type,
                slow_ma_type,
                ma_source,
                attempted: AtomicBool::new(false),
                outcome: Mutex::new(StageOutcomeLabel::NotAttempted),
                stage_wall_ms: Mutex::new(0.0),
                note: Mutex::new(None),
            }
        }

        fn take_summary(&self) -> (StageOutcomeLabel, f64, Option<String>) {
            (
                *self.outcome.lock().unwrap_or_else(|e| e.into_inner()),
                *self.stage_wall_ms.lock().unwrap_or_else(|e| e.into_inner()),
                self.note.lock().unwrap_or_else(|e| e.into_inner()).clone(),
            )
        }
    }

    impl ProgressSink for StageNextDatasetSink<'_> {
        fn emit_double_ma_progress(&self, _payload: DoubleMaProgressPayload) {}

        fn with_double_ma_campaign_vram_prefetch_request(
            &self,
            f: &mut dyn FnMut(u32, &Candles, &str, &str, &str),
        ) {
            if self.attempted.swap(true, Ordering::Relaxed) {
                return;
            }
            f(
                self.device_id,
                self.next_candles,
                self.fast_ma_type,
                self.slow_ma_type,
                self.ma_source,
            );
        }

        fn on_double_ma_campaign_vram_prefetch_feedback(
            &self,
            feedback: DoubleMaCampaignVramPrefetchFeedback,
        ) {
            let (outcome, stage_wall_ms, note) = match feedback {
                DoubleMaCampaignVramPrefetchFeedback::Staged { stage_wall_ms } => {
                    (StageOutcomeLabel::Staged, stage_wall_ms, None)
                }
                DoubleMaCampaignVramPrefetchFeedback::AlreadyResident { stage_wall_ms } => {
                    (StageOutcomeLabel::AlreadyResident, stage_wall_ms, None)
                }
                DoubleMaCampaignVramPrefetchFeedback::Warning {
                    note,
                    stage_wall_ms,
                } => (StageOutcomeLabel::Skipped, stage_wall_ms, Some(note)),
                DoubleMaCampaignVramPrefetchFeedback::Fatal {
                    note,
                    stage_wall_ms,
                } => (StageOutcomeLabel::Error, stage_wall_ms, Some(note)),
            };
            *self.outcome.lock().unwrap_or_else(|e| e.into_inner()) = outcome;
            *self.stage_wall_ms.lock().unwrap_or_else(|e| e.into_inner()) = stage_wall_ms;
            *self.note.lock().unwrap_or_else(|e| e.into_inner()) = note;
        }
    }

    fn parse_arg_value(flag: &str) -> Option<String> {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == flag {
                return args.next();
            }
        }
        None
    }

    fn parse_usize_flag(flag: &str, default: usize) -> Result<usize, String> {
        let raw = parse_arg_value(flag).unwrap_or_else(|| default.to_string());
        raw.trim()
            .parse::<usize>()
            .map_err(|e| format!("invalid {flag} '{}': {e}", raw.trim()))
    }

    fn parse_u32_flag(flag: &str, default: u32) -> Result<u32, String> {
        let raw = parse_arg_value(flag).unwrap_or_else(|| default.to_string());
        raw.trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid {flag} '{}': {e}", raw.trim()))
    }

    fn parse_mode() -> Result<Mode, String> {
        let raw = parse_arg_value("--mode").unwrap_or_else(|| "host_only".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "host_only" => Ok(Mode::HostOnly),
            "staged" => Ok(Mode::Staged),
            other => Err(format!(
                "unsupported --mode '{other}' (expected: host_only|staged)"
            )),
        }
    }

    fn parse_range_flag(flag: &str, default: (u32, u32, u32)) -> Result<(u32, u32, u32), String> {
        let raw = parse_arg_value(flag)
            .unwrap_or_else(|| format!("{}:{}:{}", default.0, default.1, default.2));
        let parts: Vec<&str> = raw.split(':').collect();
        if parts.len() != 3 {
            return Err(format!(
                "invalid {flag} '{}': expected start:end:step",
                raw.trim()
            ));
        }
        let start = parts[0]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid {flag} start '{}': {e}", parts[0].trim()))?;
        let end = parts[1]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid {flag} end '{}': {e}", parts[1].trim()))?;
        let step = parts[2]
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid {flag} step '{}': {e}", parts[2].trim()))?;
        if start == 0 || end == 0 || step == 0 || start > end {
            return Err(format!(
                "invalid {flag} '{}': expected positive start:end:step with start <= end",
                raw.trim()
            ));
        }
        Ok((start, end, step))
    }

    fn parse_config() -> Result<Config, String> {
        Ok(Config {
            mode: parse_mode()?,
            len: parse_usize_flag("--len", 250_000)?,
            warmup_iterations: parse_usize_flag("--warmup-iterations", 1)?,
            measure_iterations: parse_usize_flag("--measure-iterations", 3)?,
            device_id: parse_u32_flag("--device-id", 0)?,
            fast_ma_type: parse_arg_value("--fast-ma").unwrap_or_else(|| "sma".to_string()),
            slow_ma_type: parse_arg_value("--slow-ma").unwrap_or_else(|| "ema".to_string()),
            ma_source: parse_arg_value("--ma-source").unwrap_or_else(|| "close".to_string()),
            fast_range: parse_range_flag("--fast-range", (5, 80, 5))?,
            slow_range: parse_range_flag("--slow-range", (20, 160, 10))?,
            top_k: parse_usize_flag("--top-k", 16)?,
            output_json: parse_arg_value("--output-json"),
        })
    }

    fn make_trending_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);

        for i in 0..len {
            ts.push(i as i64);
            let base = 100.0 + i as f64 * 0.03;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.4);
            vol.push(1_000.0 + (i % 7) as f64 * 15.0);
        }

        Candles::new(ts, open, high, low, close, vol)
    }

    fn make_breakout_candles(len: usize) -> Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol = Vec::with_capacity(len);

        let up_start = len / 3;
        let down_start = (len * 2) / 3;
        for i in 0..len {
            ts.push(i as i64);
            let base = if i < up_start {
                100.0 + i as f64 * 0.01
            } else if i < down_start {
                101.0 + (i - up_start) as f64 * 0.12
            } else {
                101.0 + (down_start - up_start) as f64 * 0.12 - (i - down_start) as f64 * 0.14
            };
            let noise = ((i as f64) / 9.0).sin() * 0.12;
            let o = base + noise;
            let c = if i < down_start {
                base + 0.25 + noise
            } else {
                base - 0.25 + noise
            };
            open.push(o);
            close.push(c);
            high.push(o.max(c) + 0.3);
            low.push(o.min(c) - 0.3);
            vol.push(if i < up_start { 900.0 } else { 2_200.0 } + (i % 11) as f64 * 30.0);
        }

        Candles::new(ts, open, high, low, close, vol)
    }

    fn build_request(
        device_id: u32,
        data_id: String,
        fast_range: (u32, u32, u32),
        slow_range: (u32, u32, u32),
        fast_ma_type: &str,
        slow_ma_type: &str,
        ma_source: &str,
        top_k: usize,
    ) -> DoubleMaRequest {
        DoubleMaRequest {
            backend: Backend::GpuKernel { device_id },
            data_id,
            fast_range,
            slow_range,
            fast_ma_types: vec![fast_ma_type.to_string()],
            slow_ma_types: vec![slow_ma_type.to_string()],
            ma_source: ma_source.to_string(),
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
            top_k: Some(top_k),
            include_all: Some(false),
            heatmap_bins: Some(64),
            constraints: None,
            vdps: None,
            dynamic_strategy_ir: None,
            gpu_execution: GpuExecutionPolicy::BestEffort,
            persist_followup_cache: false,
            include_trade_metrics: false,
            metric_contract: None,
        }
    }

    fn backend_used_label(value: BackendUsed) -> String {
        match value {
            BackendUsed::Cpu => "cpu".to_string(),
            BackendUsed::Gpu => "gpu".to_string(),
        }
    }

    fn mean(values: &[f64]) -> f64 {
        if values.is_empty() {
            0.0
        } else {
            values.iter().copied().sum::<f64>() / values.len() as f64
        }
    }

    fn mean_nonzero(values: &[f64]) -> Option<f64> {
        let filtered: Vec<f64> = values
            .iter()
            .copied()
            .filter(|value| *value > 0.0)
            .collect();
        (!filtered.is_empty()).then(|| mean(&filtered))
    }

    fn rate(numerator: usize, denominator: usize) -> f64 {
        if denominator == 0 {
            0.0
        } else {
            numerator as f64 / denominator as f64
        }
    }

    fn run_profile(cfg: &Config) -> Result<RunSummary, String> {
        #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
        {
            let _ = cfg;
            return Err(
                "profile_doublema_campaign_prefetch requires `cuda,cuda-backtest-kernel` features."
                    .to_string(),
            );
        }

        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        {
            if !vector_ta::cuda::cuda_available() {
                return Err("CUDA is not available on this machine.".to_string());
            }

            let first_candles = make_trending_candles(cfg.len);
            let second_candles = make_breakout_candles(cfg.len);
            let cancel = AtomicBool::new(false);
            let total_iterations = cfg.warmup_iterations + cfg.measure_iterations;
            let mut measured_iterations = Vec::with_capacity(cfg.measure_iterations);

            for iteration in 0..total_iterations {
                let first_req = build_request(
                    cfg.device_id,
                    format!("campaign-prefetch-first-{iteration}"),
                    cfg.fast_range,
                    cfg.slow_range,
                    &cfg.fast_ma_type,
                    &cfg.slow_ma_type,
                    &cfg.ma_source,
                    cfg.top_k,
                );
                let second_req = build_request(
                    cfg.device_id,
                    format!("campaign-prefetch-second-{iteration}"),
                    cfg.fast_range,
                    cfg.slow_range,
                    &cfg.fast_ma_type,
                    &cfg.slow_ma_type,
                    &cfg.ma_source,
                    cfg.top_k,
                );

                let stage_sink = matches!(cfg.mode, Mode::Staged).then(|| {
                    StageNextDatasetSink::new(
                        cfg.device_id,
                        &second_candles,
                        &cfg.fast_ma_type,
                        &cfg.slow_ma_type,
                        &cfg.ma_source,
                    )
                });

                let pair_started = Instant::now();
                let first_started = Instant::now();
                let first_res = run_double_ma_optimization_blocking_with_candles(
                    first_req,
                    &first_candles,
                    &cancel,
                    stage_sink.as_ref().map(|value| value as &dyn ProgressSink),
                )?;
                let first_wall_ms = first_started.elapsed().as_secs_f64() * 1_000.0;

                let second_started = Instant::now();
                let second_res = run_double_ma_optimization_blocking_with_candles(
                    second_req,
                    &second_candles,
                    &cancel,
                    None,
                )?;
                let second_wall_ms = second_started.elapsed().as_secs_f64() * 1_000.0;
                let pair_wall_ms = pair_started.elapsed().as_secs_f64() * 1_000.0;

                if !matches!(first_res.backend_used, BackendUsed::Gpu)
                    || !first_res
                        .backend_route
                        .as_deref()
                        .map(|route| route.contains("GpuKernel"))
                        .unwrap_or(false)
                {
                    return Err(format!(
                    "first run did not stay on the exact GpuKernel route: backend_used={:?}, route={:?}",
                    first_res.backend_used, first_res.backend_route
                ));
                }
                if !matches!(second_res.backend_used, BackendUsed::Gpu)
                    || !second_res
                        .backend_route
                        .as_deref()
                        .map(|route| route.contains("GpuKernel"))
                        .unwrap_or(false)
                {
                    return Err(format!(
                    "second run did not stay on the exact GpuKernel route: backend_used={:?}, route={:?}",
                    second_res.backend_used, second_res.backend_route
                ));
                }

                if iteration >= cfg.warmup_iterations {
                    let (stage_outcome, stage_wall_ms, stage_note) = stage_sink
                        .as_ref()
                        .map(StageNextDatasetSink::take_summary)
                        .unwrap_or((StageOutcomeLabel::NotAttempted, 0.0, None));
                    measured_iterations.push(IterationSummary {
                        iteration,
                        first_wall_ms,
                        second_wall_ms,
                        pair_wall_ms,
                        first_minus_second_wall_ms: first_wall_ms - second_wall_ms,
                        first_backend_route: first_res.backend_route.clone(),
                        second_backend_route: second_res.backend_route.clone(),
                        first_backend_used: backend_used_label(first_res.backend_used),
                        second_backend_used: backend_used_label(second_res.backend_used),
                        stage_outcome,
                        stage_wall_ms,
                        stage_note,
                    });
                }
            }

            let stage_attempts = measured_iterations
                .iter()
                .filter(|entry| !matches!(entry.stage_outcome, StageOutcomeLabel::NotAttempted))
                .count();
            let stage_successes = measured_iterations
                .iter()
                .filter(|entry| matches!(entry.stage_outcome, StageOutcomeLabel::Staged))
                .count();
            let stage_already_resident = measured_iterations
                .iter()
                .filter(|entry| matches!(entry.stage_outcome, StageOutcomeLabel::AlreadyResident))
                .count();
            let stage_skips = measured_iterations
                .iter()
                .filter(|entry| matches!(entry.stage_outcome, StageOutcomeLabel::Skipped))
                .count();
            let stage_errors = measured_iterations
                .iter()
                .filter(|entry| matches!(entry.stage_outcome, StageOutcomeLabel::Error))
                .count();

            Ok(RunSummary {
                mode: match cfg.mode {
                    Mode::HostOnly => "host_only",
                    Mode::Staged => "staged",
                }
                .to_string(),
                device_id: cfg.device_id,
                len: cfg.len,
                fast_range: cfg.fast_range,
                slow_range: cfg.slow_range,
                fast_ma_type: cfg.fast_ma_type.clone(),
                slow_ma_type: cfg.slow_ma_type.clone(),
                ma_source: cfg.ma_source.clone(),
                warmup_iterations: cfg.warmup_iterations,
                measure_iterations: cfg.measure_iterations,
                stage_attempts,
                stage_successes,
                stage_already_resident,
                stage_skips,
                stage_errors,
                stage_success_rate: rate(stage_successes + stage_already_resident, stage_attempts),
                stage_error_rate: rate(stage_errors, stage_attempts),
                mean_first_wall_ms: mean(
                    &measured_iterations
                        .iter()
                        .map(|entry| entry.first_wall_ms)
                        .collect::<Vec<_>>(),
                ),
                mean_second_wall_ms: mean(
                    &measured_iterations
                        .iter()
                        .map(|entry| entry.second_wall_ms)
                        .collect::<Vec<_>>(),
                ),
                mean_pair_wall_ms: mean(
                    &measured_iterations
                        .iter()
                        .map(|entry| entry.pair_wall_ms)
                        .collect::<Vec<_>>(),
                ),
                mean_first_minus_second_wall_ms: mean(
                    &measured_iterations
                        .iter()
                        .map(|entry| entry.first_minus_second_wall_ms)
                        .collect::<Vec<_>>(),
                ),
                mean_stage_wall_ms: mean_nonzero(
                    &measured_iterations
                        .iter()
                        .map(|entry| entry.stage_wall_ms)
                        .collect::<Vec<_>>(),
                ),
                measured_iterations,
            })
        }
    }

    pub(crate) fn main() -> Result<(), String> {
        let cfg = parse_config()?;
        let _benchmark_guard =
            crate::benchmark_lock::acquire("double-MA campaign prefetch profile")?;
        let summary = run_profile(&cfg)?;
        let rendered = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
        if let Some(output_json) = cfg.output_json.as_deref() {
            crate::atomic_output::write_str(std::path::Path::new(output_json), &rendered)?;
            println!("wrote {}", output_json);
        } else {
            println!("{rendered}");
        }
        Ok(())
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn main() -> Result<(), String> {
    imp::main()
}

#[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
fn main() -> Result<(), String> {
    Err(
        "profile_doublema_campaign_prefetch requires `cuda,cuda-backtest-kernel` features."
            .to_string(),
    )
}
