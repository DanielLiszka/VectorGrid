use serde::Serialize;
use serde_json::Value;
use ta_strategies::double_ma::Metrics;

const OPTIMIZATION_METRIC_SURFACE_SCHEMA_VERSION: u64 = 3;

pub(crate) const CORE17_FIELDS: [&str; 17] = [
    "pnl",
    "trades",
    "max_dd",
    "mean",
    "std",
    "exposure",
    "net_exposure",
    "downside_deviation",
    "ulcer_index",
    "omega",
    "downside_frequency",
    "downside_severity",
    "long_mean_return",
    "short_mean_return",
    "kappa_tau",
    "underwater_fraction",
    "max_underwater_duration_bars",
];

pub(crate) const TRADE22_FIELDS: [&str; 22] = [
    "pnl",
    "trades",
    "max_dd",
    "mean",
    "std",
    "exposure",
    "net_exposure",
    "downside_deviation",
    "ulcer_index",
    "omega",
    "downside_frequency",
    "downside_severity",
    "long_mean_return",
    "short_mean_return",
    "kappa_tau",
    "underwater_fraction",
    "max_underwater_duration_bars",
    "closed_trades",
    "winning_trades",
    "losing_trades",
    "gross_profit",
    "gross_loss",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptimizationMetricSurfaceKind {
    Legacy,
    Core17,
    Trade22,
}

impl OptimizationMetricSurfaceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Core17 => "core17",
            Self::Trade22 => "trade22",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct ArtifactMetricSurfaceSummary {
    pub legacy: usize,
    pub core17: usize,
    pub trade22: usize,
    pub missing_metadata: usize,
}

impl ArtifactMetricSurfaceSummary {
    pub(crate) fn distinct_surface_count(&self) -> usize {
        (self.legacy > 0) as usize + (self.core17 > 0) as usize + (self.trade22 > 0) as usize
    }

    pub(crate) fn is_mixed(&self) -> bool {
        self.distinct_surface_count() > 1
    }

    pub(crate) fn total(&self) -> usize {
        self.legacy + self.core17 + self.trade22
    }

    pub(crate) fn describe_counts(&self) -> String {
        format!(
            "legacy={}, core17={}, trade22={}",
            self.legacy, self.core17, self.trade22
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OptimizationMetricSurfaceArtifact {
    pub schema_version: u64,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_tail_exact: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_metrics_row: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_metric_rows: Vec<Vec<f64>>,
}

fn normalize_text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn is_exact_gpu_kernel_route(route: &str) -> bool {
    let route = route.trim();
    !route.is_empty()
        && route.contains("GpuKernel")
        && !route.contains("GpuHybridKernel")
        && !route.contains("CpuOnly")
}

fn artifact_backend_route(artifact: &Value) -> String {
    normalize_text(artifact.pointer("/run/res/backend_route"))
}

fn artifact_backend_used(artifact: &Value) -> String {
    normalize_text(artifact.pointer("/run/res/backend_used"))
}

fn artifact_requested_backend(artifact: &Value) -> String {
    normalize_text(artifact.pointer("/run/req/backend/backend"))
}

fn artifact_strategy_kind(artifact: &Value) -> String {
    normalize_text(artifact.pointer("/run/req/strategy_kind"))
}

fn artifact_requested_metric_contract(artifact: &Value) -> String {
    normalize_text(artifact.pointer("/run/req/metric_contract")).to_ascii_lowercase()
}

fn artifact_requested_full17(artifact: &Value) -> bool {
    matches!(
        artifact_requested_metric_contract(artifact).as_str(),
        "full_17" | "full17" | "full-17"
    )
}

fn artifact_requested_trade22(artifact: &Value) -> bool {
    matches!(
        artifact_requested_metric_contract(artifact).as_str(),
        "trade_22" | "trade22" | "trade-22"
    ) || artifact
        .pointer("/run/req/include_trade_metrics")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn artifact_gpu_kernel_metrics_count(artifact: &Value) -> Option<usize> {
    artifact
        .pointer("/run/res/route_diagnostics")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().filter_map(Value::as_str).find_map(|entry| {
                entry
                    .strip_prefix("gpu_kernel.metrics_count=")
                    .and_then(|raw| raw.parse::<usize>().ok())
            })
        })
}

fn artifact_runtime_metric_surface_kind(artifact: &Value) -> Option<OptimizationMetricSurfaceKind> {
    match normalize_text(artifact.pointer("/run/res/metric_surface_kind"))
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "legacy" => Some(OptimizationMetricSurfaceKind::Legacy),
        "core17" => Some(OptimizationMetricSurfaceKind::Core17),
        "trade22" => Some(OptimizationMetricSurfaceKind::Trade22),
        _ => None,
    }
}

fn artifact_published_trade_tail_exact(artifact: &Value) -> bool {
    artifact
        .pointer("/optimization_metric_surface/trade_tail_exact")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn artifact_runtime_trade_tail_exact(artifact: &Value) -> bool {
    artifact
        .pointer("/run/res/metric_surface_trade_tail_exact")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn metrics_value_has_trade22_tail(metrics: &Value) -> bool {
    [
        "closed_trades",
        "winning_trades",
        "losing_trades",
        "gross_profit",
        "gross_loss",
    ]
    .iter()
    .all(|field| metrics.get(*field).and_then(Value::as_f64).is_some())
}

fn artifact_contains_trade22_tail_metrics(artifact: &Value) -> bool {
    if artifact
        .pointer("/run/res/best_metrics")
        .is_some_and(metrics_value_has_trade22_tail)
    {
        return true;
    }

    if artifact
        .pointer("/run/res/top")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_array)
                .filter_map(|row| row.get(1))
                .any(metrics_value_has_trade22_tail)
        })
    {
        return true;
    }

    artifact
        .pointer("/run/res/pareto")
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .filter_map(|row| row.get("metrics"))
                .any(metrics_value_has_trade22_tail)
        })
}

fn is_cpu_metric_route(artifact: &Value) -> bool {
    let backend_route = artifact_backend_route(artifact);
    if backend_route.contains("CpuOnly") {
        return true;
    }

    let backend_used = artifact_backend_used(artifact);
    if backend_used == "Cpu" {
        return true;
    }

    artifact_requested_backend(artifact) == "CpuOnly"
}

fn strategy_kind_supports_core17(artifact: &Value) -> bool {
    let strategy_kind = artifact_strategy_kind(artifact);
    match strategy_kind.as_str() {
        "DoubleMa" | "Pit" | "Set" | "Mts" | "Orc" | "Marrt" | "Vfct" | "CompositeCpu"
        | "RegimeHybrid" => true,
        "CompositeTrend" | "BreakoutMomentum" => {
            is_exact_gpu_kernel_route(&artifact_backend_route(artifact))
        }
        "Asr" | "Vcc" | "Vemr" | "Rocb" | "Vcet" | "Kqsr" | "Carm" | "Kvdf" | "Mkcs" | "Cdmc" => {
            is_cpu_metric_route(artifact)
        }
        _ => false,
    }
}

fn strategy_kind_supports_exact_gpu_metric_surface(artifact: &Value) -> bool {
    matches!(
        artifact_strategy_kind(artifact).as_str(),
        "DoubleMa"
            | "Pit"
            | "Set"
            | "Mts"
            | "Orc"
            | "Marrt"
            | "Vfct"
            | "CompositeTrend"
            | "BreakoutMomentum"
            | "RegimeHybrid"
            | "DynamicDoubleMa"
    )
}

fn strategy_kind_supports_requested_trade22_surface(artifact: &Value) -> bool {
    matches!(
        artifact_strategy_kind(artifact).as_str(),
        "DoubleMa" | "Pit" | "Set" | "Mts" | "Orc" | "Marrt" | "Vfct"
    )
}

fn artifact_has_exact_trade_tail_support(artifact: &Value) -> bool {
    if artifact_published_trade_tail_exact(artifact) || artifact_runtime_trade_tail_exact(artifact)
    {
        return true;
    }

    let backend_route = artifact_backend_route(artifact);
    if is_exact_gpu_kernel_route(&backend_route) {
        if artifact_strategy_kind(artifact) == "DynamicDoubleMa"
            && matches!(
                artifact_runtime_metric_surface_kind(artifact),
                Some(OptimizationMetricSurfaceKind::Trade22)
            )
        {
            return true;
        }

        return artifact_gpu_kernel_metrics_count(artifact)
            .map(|metrics_count| metrics_count >= TRADE22_FIELDS.len())
            .unwrap_or(false);
    }

    if strategy_kind_supports_requested_trade22_surface(artifact)
        && artifact_requested_trade22(artifact)
        && artifact_contains_trade22_tail_metrics(artifact)
    {
        return true;
    }

    matches!(
        artifact_runtime_metric_surface_kind(artifact),
        Some(OptimizationMetricSurfaceKind::Trade22)
    )
}

fn detect_exact_gpu_kernel_surface_kind(artifact: &Value) -> Option<OptimizationMetricSurfaceKind> {
    if !strategy_kind_supports_exact_gpu_metric_surface(artifact) {
        return None;
    }
    if !is_exact_gpu_kernel_route(&artifact_backend_route(artifact)) {
        return None;
    }

    if let Some(metrics_count) = artifact_gpu_kernel_metrics_count(artifact) {
        return Some(if metrics_count >= TRADE22_FIELDS.len() {
            OptimizationMetricSurfaceKind::Trade22
        } else if metrics_count >= CORE17_FIELDS.len() {
            OptimizationMetricSurfaceKind::Core17
        } else {
            OptimizationMetricSurfaceKind::Legacy
        });
    }

    if artifact_has_exact_trade_tail_support(artifact) {
        return Some(OptimizationMetricSurfaceKind::Trade22);
    }
    if artifact_requested_full17(artifact) {
        return Some(OptimizationMetricSurfaceKind::Core17);
    }

    Some(OptimizationMetricSurfaceKind::Legacy)
}

fn heuristic_surface_kind(artifact: &Value) -> OptimizationMetricSurfaceKind {
    if let Some(exact_gpu_kind) = detect_exact_gpu_kernel_surface_kind(artifact) {
        return exact_gpu_kind;
    }

    if !strategy_kind_supports_core17(artifact) {
        return OptimizationMetricSurfaceKind::Legacy;
    }

    if artifact_has_exact_trade_tail_support(artifact) {
        return OptimizationMetricSurfaceKind::Trade22;
    }

    OptimizationMetricSurfaceKind::Core17
}

fn current_surface_kind_from_metadata(artifact: &Value) -> Option<OptimizationMetricSurfaceKind> {
    let kind = artifact
        .pointer("/optimization_metric_surface/surface")
        .and_then(Value::as_str)
        .and_then(parse_surface_label)?;

    match kind {
        OptimizationMetricSurfaceKind::Trade22
            if !artifact_published_trade_tail_exact(artifact) =>
        {
            None
        }
        _ => Some(kind),
    }
}

fn rebuild_surface_kind(artifact: &Value) -> OptimizationMetricSurfaceKind {
    if let Some(runtime_kind) = artifact_runtime_metric_surface_kind(artifact) {
        if !matches!(runtime_kind, OptimizationMetricSurfaceKind::Trade22)
            || artifact_has_exact_trade_tail_support(artifact)
        {
            return runtime_kind;
        }
    }

    if has_current_metric_surface_metadata(artifact) {
        if let Some(current_kind) = current_surface_kind_from_metadata(artifact) {
            return current_kind;
        }
    }

    heuristic_surface_kind(artifact)
}

fn metrics_mean(metrics: &Metrics) -> f64 {
    metrics.sharpe * metrics.volatility
}

fn surface_fields(kind: OptimizationMetricSurfaceKind) -> &'static [&'static str] {
    match kind {
        OptimizationMetricSurfaceKind::Legacy => &[],
        OptimizationMetricSurfaceKind::Core17 => &CORE17_FIELDS,
        OptimizationMetricSurfaceKind::Trade22 => &TRADE22_FIELDS,
    }
}

fn project_metrics_row(metrics: &Metrics, kind: OptimizationMetricSurfaceKind) -> Option<Vec<f64>> {
    match kind {
        OptimizationMetricSurfaceKind::Legacy => None,
        OptimizationMetricSurfaceKind::Core17 => Some(vec![
            metrics.pnl,
            metrics.trades as f64,
            metrics.max_dd,
            metrics_mean(metrics),
            metrics.volatility,
            metrics.exposure,
            metrics.net_exposure,
            metrics.downside_deviation,
            metrics.ulcer_index,
            metrics.omega,
            metrics.downside_frequency,
            metrics.downside_severity,
            metrics.long_mean_return,
            metrics.short_mean_return,
            metrics.kappa_tau,
            metrics.underwater_fraction,
            metrics.max_underwater_duration_bars as f64,
        ]),
        OptimizationMetricSurfaceKind::Trade22 => Some(vec![
            metrics.pnl,
            metrics.trades as f64,
            metrics.max_dd,
            metrics_mean(metrics),
            metrics.volatility,
            metrics.exposure,
            metrics.net_exposure,
            metrics.downside_deviation,
            metrics.ulcer_index,
            metrics.omega,
            metrics.downside_frequency,
            metrics.downside_severity,
            metrics.long_mean_return,
            metrics.short_mean_return,
            metrics.kappa_tau,
            metrics.underwater_fraction,
            metrics.max_underwater_duration_bars as f64,
            metrics.closed_trades as f64,
            metrics.winning_trades as f64,
            metrics.losing_trades as f64,
            metrics.gross_profit,
            metrics.gross_loss,
        ]),
    }
}

fn detect_surface_kind(artifact: &Value) -> OptimizationMetricSurfaceKind {
    if let Some(runtime_kind) = artifact_runtime_metric_surface_kind(artifact) {
        return runtime_kind;
    }

    heuristic_surface_kind(artifact)
}

fn parse_surface_label(label: &str) -> Option<OptimizationMetricSurfaceKind> {
    match label.trim().to_ascii_lowercase().as_str() {
        "legacy" => Some(OptimizationMetricSurfaceKind::Legacy),
        "core17" | "core_17" | "core-17" => Some(OptimizationMetricSurfaceKind::Core17),
        "trade22" | "trade_22" | "trade-22" => Some(OptimizationMetricSurfaceKind::Trade22),
        _ => None,
    }
}

fn has_current_metric_surface_metadata(artifact: &Value) -> bool {
    artifact
        .pointer("/optimization_metric_surface/schema_version")
        .and_then(Value::as_u64)
        == Some(OPTIMIZATION_METRIC_SURFACE_SCHEMA_VERSION)
}

pub(crate) fn artifact_metric_surface_kind(artifact: &Value) -> OptimizationMetricSurfaceKind {
    if has_current_metric_surface_metadata(artifact) {
        current_surface_kind_from_metadata(artifact)
            .unwrap_or_else(|| detect_surface_kind(artifact))
    } else {
        detect_surface_kind(artifact)
    }
}

pub(crate) fn summarize_artifact_metric_surfaces(
    artifacts: &[Value],
) -> ArtifactMetricSurfaceSummary {
    let mut summary = ArtifactMetricSurfaceSummary::default();
    for artifact in artifacts {
        if !has_current_metric_surface_metadata(artifact) {
            summary.missing_metadata = summary.missing_metadata.saturating_add(1);
        }
        match artifact_metric_surface_kind(artifact) {
            OptimizationMetricSurfaceKind::Legacy => {
                summary.legacy = summary.legacy.saturating_add(1);
            }
            OptimizationMetricSurfaceKind::Core17 => {
                summary.core17 = summary.core17.saturating_add(1);
            }
            OptimizationMetricSurfaceKind::Trade22 => {
                summary.trade22 = summary.trade22.saturating_add(1);
            }
        }
    }
    summary
}

fn parse_metrics(value: Option<&Value>) -> Option<Metrics> {
    value
        .cloned()
        .and_then(|row| serde_json::from_value::<Metrics>(row).ok())
}

pub(crate) fn build_optimization_metric_surface_artifact(
    artifact: &Value,
) -> OptimizationMetricSurfaceArtifact {
    let surface_kind = rebuild_surface_kind(artifact);
    let fields = surface_fields(surface_kind)
        .iter()
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    let trade_tail_exact = matches!(surface_kind, OptimizationMetricSurfaceKind::Trade22)
        .then(|| artifact_has_exact_trade_tail_support(artifact))
        .filter(|exact| *exact);
    let best_metrics_row = parse_metrics(artifact.pointer("/run/res/best_metrics"))
        .and_then(|metrics| project_metrics_row(&metrics, surface_kind));
    let mut top_metric_rows = artifact
        .pointer("/run/res/top")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_array)
                .filter_map(|row| parse_metrics(row.get(1)))
                .filter_map(|metrics| project_metrics_row(&metrics, surface_kind))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if top_metric_rows.is_empty() {
        top_metric_rows = artifact
            .pointer("/run/res/pareto")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(Value::as_object)
                    .filter_map(|row| parse_metrics(row.get("metrics")))
                    .filter_map(|metrics| project_metrics_row(&metrics, surface_kind))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    OptimizationMetricSurfaceArtifact {
        schema_version: OPTIMIZATION_METRIC_SURFACE_SCHEMA_VERSION,
        surface: surface_kind.as_str().to_string(),
        fields,
        trade_tail_exact,
        best_metrics_row,
        top_metric_rows,
    }
}

pub(crate) fn build_optimization_metric_surface_artifact_value(artifact: &Value) -> Option<Value> {
    artifact.pointer("/run/res")?;

    serde_json::to_value(build_optimization_metric_surface_artifact(artifact)).ok()
}

#[cfg(test)]
mod tests {
    use super::{
        build_optimization_metric_surface_artifact,
        build_optimization_metric_surface_artifact_value, parse_surface_label, project_metrics_row,
        summarize_artifact_metric_surfaces, Metrics, OptimizationMetricSurfaceKind, CORE17_FIELDS,
        TRADE22_FIELDS,
    };
    use serde_json::json;

    fn sample_metrics() -> serde_json::Value {
        json!({
            "pnl": 1.25,
            "sharpe": 0.5,
            "sortino": 0.7,
            "downside_deviation": 0.2,
            "downside_frequency": 0.1,
            "downside_severity": 0.05,
            "calmar": 0.9,
            "omega": 1.1,
            "kappa_tau": 0.4,
            "volatility": 0.3,
            "ulcer_index": 0.12,
            "martin_ratio": 0.8,
            "max_dd": 0.25,
            "trades": 12,
            "exposure": 0.6,
            "net_exposure": 0.2,
            "long_mean_return": 0.03,
            "short_mean_return": -0.02,
            "closed_trades": 9,
            "winning_trades": 5,
            "losing_trades": 4,
            "gross_profit": 2.4,
            "gross_loss": 1.2,
            "underwater_fraction": 0.18,
            "max_underwater_duration_bars": 14
        })
    }

    #[test]
    fn project_metrics_row_preserves_core17_field_order() {
        let metrics: Metrics = serde_json::from_value(sample_metrics()).expect("metrics");
        let row = project_metrics_row(&metrics, OptimizationMetricSurfaceKind::Core17)
            .expect("core17 row");
        assert_eq!(
            row,
            vec![
                1.25,
                12.0,
                0.25,
                0.5 * 0.3,
                0.3,
                0.6,
                0.2,
                0.2,
                0.12,
                1.1,
                0.1,
                0.05,
                0.03,
                -0.02,
                0.4,
                0.18,
                14.0,
            ]
        );
    }

    #[test]
    fn project_metrics_row_preserves_trade22_field_order() {
        let metrics: Metrics = serde_json::from_value(sample_metrics()).expect("metrics");
        let row = project_metrics_row(&metrics, OptimizationMetricSurfaceKind::Trade22)
            .expect("trade22 row");
        assert_eq!(
            row,
            vec![
                1.25,
                12.0,
                0.25,
                0.5 * 0.3,
                0.3,
                0.6,
                0.2,
                0.2,
                0.12,
                1.1,
                0.1,
                0.05,
                0.03,
                -0.02,
                0.4,
                0.18,
                14.0,
                9.0,
                5.0,
                4.0,
                2.4,
                1.2,
            ]
        );
    }

    #[test]
    fn metric_surface_marks_composite_trend_as_core17_when_exact_route_diagnostics_report_17_rows()
    {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "Auto->GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_keeps_composite_trend_exact_gpu_kernel_legacy_without_surface_evidence() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "Auto->GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.fields.is_empty());
        assert!(surface.best_metrics_row.is_none());
        assert!(surface.top_metric_rows.is_empty());
    }

    #[test]
    fn metric_surface_keeps_composite_trend_on_legacy_surface_when_hybrid_route_is_used() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_route": "GpuHybridKernel",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.fields.is_empty());
        assert!(surface.best_metrics_row.is_none());
        assert!(surface.top_metric_rows.is_empty());
    }

    #[test]
    fn metric_surface_marks_breakout_momentum_as_core17_when_exact_route_diagnostics_report_17_rows(
    ) {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "BreakoutMomentum",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_keeps_double_ma_exact_gpu_kernel_legacy_without_surface_evidence() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.fields.is_empty());
        assert!(surface.best_metrics_row.is_none());
        assert!(surface.top_metric_rows.is_empty());
    }

    #[test]
    fn metric_surface_marks_composite_cpu_as_core17() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_kqsr_as_core17_on_cpu_route() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Kqsr",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Cpu",
                    "backend_route": "GpuKernel->CpuOnly",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_kqsr_as_core17_when_cpu_backend_was_requested_and_runtime_metadata_is_missing(
    ) {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Kqsr",
                    "backend": { "backend": "CpuOnly" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_keeps_kqsr_on_legacy_surface_when_gpu_route_is_used() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Kqsr",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.fields.is_empty());
        assert!(surface.best_metrics_row.is_none());
        assert!(surface.top_metric_rows.is_empty());
    }

    #[test]
    fn metric_surface_marks_asr_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Asr",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_vcc_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Vcc",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_vemr_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Vemr",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_rocb_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Rocb",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_vcet_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Vcet",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_kqsr_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Kqsr",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_carm_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Carm",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_mkcs_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Mkcs",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_kvdf_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Kvdf",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_cdmc_gpu_artifacts_as_core17_when_runtime_hint_is_present() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Cdmc",
                    "backend": { "backend": "GpuKernel" },
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "core17",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_marks_trade22_when_trade_metrics_requested() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "metric_contract": "trade_22",
                    "include_trade_metrics": true
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Trade22.as_str()
        );
        assert_eq!(surface.fields.len(), TRADE22_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(22));
    }

    #[test]
    fn metric_surface_marks_trade22_when_exact_route_diagnostics_report_22_rows() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=22"],
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Trade22.as_str()
        );
        assert_eq!(surface.fields.len(), TRADE22_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(22));
    }

    #[test]
    fn metric_surface_marks_dynamic_double_ma_as_core17_when_explicit_full17_is_requested() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DynamicDoubleMa",
                    "metric_contract": "full_17",
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel(dynamic_strategy_v1)",
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_honors_runtime_trade22_hint_for_dynamic_double_ma() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DynamicDoubleMa",
                    "metric_contract": "trade_22",
                    "include_trade_metrics": true
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel(dynamic_strategy_v1)",
                    "metric_surface_kind": "trade22",
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Trade22.as_str()
        );
        assert_eq!(surface.fields.len(), TRADE22_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(22));
    }

    #[test]
    fn metric_surface_core17_projection_preserves_canonical_field_order() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.best_metrics_row,
            Some(vec![
                1.25, 12.0, 0.25, 0.15, 0.3, 0.6, 0.2, 0.2, 0.12, 1.1, 0.1, 0.05, 0.03, -0.02, 0.4,
                0.18, 14.0,
            ])
        );
    }

    #[test]
    fn metric_surface_trade22_projection_preserves_canonical_field_order() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "metric_contract": "trade_22",
                    "include_trade_metrics": true
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.best_metrics_row,
            Some(vec![
                1.25, 12.0, 0.25, 0.15, 0.3, 0.6, 0.2, 0.2, 0.12, 1.1, 0.1, 0.05, 0.03, -0.02, 0.4,
                0.18, 14.0, 9.0, 5.0, 4.0, 2.4, 1.2,
            ])
        );
    }

    #[test]
    fn metric_surface_keeps_composite_cpu_on_core17_when_trade_metrics_are_requested() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeCpu",
                    "metric_contract": "trade_22",
                    "include_trade_metrics": true
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_keeps_kqsr_on_core17_when_trade_metrics_are_requested_on_cpu_route() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "Kqsr",
                    "backend": { "backend": "CpuOnly" },
                    "metric_contract": "trade_22",
                    "include_trade_metrics": true
                },
                "res": {
                    "backend_used": "Cpu",
                    "backend_route": "CpuOnly",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(surface.fields.len(), CORE17_FIELDS.len());
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(17));
    }

    #[test]
    fn metric_surface_keeps_composite_mo_on_legacy_surface() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo"
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.fields.is_empty());
        assert!(surface.best_metrics_row.is_none());
        assert!(surface.top_metric_rows.is_empty());
    }

    #[test]
    fn metric_surface_projects_trade22_rows_from_composite_mo_pareto_points() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "CompositeMo"
                },
                "res": {
                    "metric_surface_kind": "trade22",
                    "pareto": [
                        {
                            "params": { "fast_len": 5, "slow_len": 10 },
                            "metrics": sample_metrics(),
                            "maximize_value": 1.0,
                            "minimize_value": 0.25,
                            "violation": 0.0
                        }
                    ]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Trade22.as_str()
        );
        assert_eq!(surface.fields.len(), TRADE22_FIELDS.len());
        assert!(surface.best_metrics_row.is_none());
        assert_eq!(surface.top_metric_rows.len(), 1);
        assert_eq!(surface.top_metric_rows.first().map(Vec::len), Some(22));
    }

    #[test]
    fn summarize_artifact_metric_surfaces_counts_effective_surfaces_and_missing_metadata() {
        let legacy = json!({
            "run": {
                "req": { "strategy_kind": "CompositeMo" },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });
        let core17 = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "core17"
            },
            "run": {
                "req": { "strategy_kind": "CompositeTrend" },
                "res": {
                    "backend_route": "GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });
        let trade22 = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "trade22",
                "trade_tail_exact": true
            },
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let summary = summarize_artifact_metric_surfaces(&[legacy, core17, trade22]);
        assert_eq!(summary.legacy, 1);
        assert_eq!(summary.core17, 1);
        assert_eq!(summary.trade22, 1);
        assert_eq!(summary.missing_metadata, 1);
        assert!(summary.is_mixed());
        assert_eq!(summary.total(), 3);
    }

    #[test]
    fn artifact_metric_surface_kind_rederives_when_schema_is_stale() {
        let artifact = json!({
            "optimization_metric_surface": {
                "schema_version": 1,
                "surface": "legacy"
            },
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let summary = summarize_artifact_metric_surfaces(&[artifact]);
        assert_eq!(summary.core17, 1);
        assert_eq!(summary.missing_metadata, 1);
    }

    #[test]
    fn artifact_metric_surface_kind_keeps_stale_exact_gpu_kernel_artifact_legacy_without_evidence()
    {
        let artifact = json!({
            "optimization_metric_surface": {
                "schema_version": 1,
                "surface": "core17"
            },
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let summary = summarize_artifact_metric_surfaces(&[artifact]);
        assert_eq!(summary.legacy, 1);
        assert_eq!(summary.missing_metadata, 1);
    }

    #[test]
    fn build_metric_surface_artifact_prefers_current_surface_metadata_when_runtime_hint_is_absent()
    {
        let artifact = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "core17"
            },
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuHybridKernel",
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let rebuilt = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            rebuilt.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(rebuilt.fields.len(), CORE17_FIELDS.len());
    }

    #[test]
    fn build_metric_surface_artifact_handles_regime_hybrid_exact_gpu_metric_contract() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "RegimeHybrid",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let rebuilt = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            rebuilt.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
    }

    #[test]
    fn build_metric_surface_artifact_treats_regime_hybrid_hybrid_route_as_core17() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "RegimeHybrid",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuHybridKernel",
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let rebuilt = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            rebuilt.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
        assert_eq!(rebuilt.fields.len(), CORE17_FIELDS.len());
    }

    #[test]
    fn build_metric_surface_artifact_falls_back_when_current_surface_label_is_invalid() {
        let artifact = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "bogus"
            },
            "run": {
                "req": {
                    "strategy_kind": "CompositeTrend",
                    "metric_contract": null,
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "route_diagnostics": ["gpu_kernel.metrics_count=17"],
                    "best_metrics": sample_metrics(),
                    "top": []
                }
            }
        });

        let rebuilt = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            rebuilt.surface,
            OptimizationMetricSurfaceKind::Core17.as_str()
        );
    }

    #[test]
    fn parse_surface_label_accepts_current_alias_spellings() {
        assert_eq!(
            parse_surface_label("core_17"),
            Some(OptimizationMetricSurfaceKind::Core17)
        );
        assert_eq!(
            parse_surface_label("core-17"),
            Some(OptimizationMetricSurfaceKind::Core17)
        );
        assert_eq!(
            parse_surface_label("trade_22"),
            Some(OptimizationMetricSurfaceKind::Trade22)
        );
        assert_eq!(
            parse_surface_label("trade-22"),
            Some(OptimizationMetricSurfaceKind::Trade22)
        );
    }

    #[test]
    fn metric_surface_refuses_trade22_without_exact_trade_tail_evidence_on_exact_gpu_routes() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.trade_tail_exact.is_none());
        assert!(surface.best_metrics_row.is_none());
    }

    #[test]
    fn metric_surface_refuses_runtime_trade22_hint_without_exact_trade_tail_evidence_on_exact_gpu_routes(
    ) {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "metric_surface_kind": "trade22",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Legacy.as_str()
        );
        assert!(surface.trade_tail_exact.is_none());
        assert!(surface.best_metrics_row.is_none());
    }

    #[test]
    fn metric_surface_prefers_current_trade22_metadata_only_when_trade_tail_exact_is_published() {
        let artifact = json!({
            "optimization_metric_surface": {
                "schema_version": 3,
                "surface": "trade22",
                "trade_tail_exact": true
            },
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "backend_used": "Gpu",
                    "backend_route": "GpuKernel",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let rebuilt = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            rebuilt.surface,
            OptimizationMetricSurfaceKind::Trade22.as_str()
        );
        assert_eq!(rebuilt.trade_tail_exact, Some(true));
        assert_eq!(rebuilt.fields.len(), TRADE22_FIELDS.len());
    }

    #[test]
    fn metric_surface_publishes_trade_tail_exact_for_non_exact_trade22_surfaces() {
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "include_trade_metrics": true
                },
                "res": {
                    "backend_used": "Cpu",
                    "backend_route": "CpuOnly",
                    "best_metrics": sample_metrics(),
                    "top": [[{"fast_len": 5}, sample_metrics()]]
                }
            }
        });

        let surface = build_optimization_metric_surface_artifact(&artifact);
        assert_eq!(
            surface.surface,
            OptimizationMetricSurfaceKind::Trade22.as_str()
        );
        assert_eq!(surface.trade_tail_exact, Some(true));
        assert_eq!(surface.best_metrics_row.as_ref().map(Vec::len), Some(22));
    }

    #[test]
    fn metric_surface_row_serialization_turns_non_finite_cells_into_null() {
        let mut metrics: Metrics = serde_json::from_value(sample_metrics()).expect("metrics");
        metrics.pnl = f64::NAN;

        let artifact_value = serde_json::to_value(super::OptimizationMetricSurfaceArtifact {
            schema_version: 3,
            surface: OptimizationMetricSurfaceKind::Core17.as_str().to_string(),
            fields: CORE17_FIELDS
                .iter()
                .map(|field| (*field).to_string())
                .collect(),
            trade_tail_exact: None,
            best_metrics_row: project_metrics_row(&metrics, OptimizationMetricSurfaceKind::Core17),
            top_metric_rows: Vec::new(),
        })
        .expect("surface artifact should serialize");

        assert_eq!(
            artifact_value.pointer("/best_metrics_row/0"),
            Some(&serde_json::Value::Null)
        );
    }

    #[test]
    fn build_metric_surface_artifact_value_keeps_non_finite_rows_serializable() {
        let mut metrics: Metrics = serde_json::from_value(sample_metrics()).expect("metrics");
        metrics.pnl = f64::INFINITY;
        let artifact = json!({
            "run": {
                "req": {
                    "strategy_kind": "DoubleMa",
                    "metric_contract": "full17",
                    "include_trade_metrics": false
                },
                "res": {
                    "backend_used": "Cpu",
                    "backend_route": "CpuOnly",
                    "best_metrics": serde_json::to_value(metrics).expect("metrics json"),
                    "top": []
                }
            }
        });

        let built = build_optimization_metric_surface_artifact_value(&artifact)
            .expect("surface artifact should build");
        assert!(built.pointer("/best_metrics_row").is_none());
    }
}
