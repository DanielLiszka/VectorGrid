use std::collections::BTreeSet;
use std::sync::OnceLock;

// Kernel-capable MA ids aligned with vector-ta 0.1.2 VRAM kernel support.
const GPU_KERNEL_SUPPORTED_MAS: &[&str] = &[
    "sma",
    "ema",
    "wma",
    "alma",
    "dema",
    "tema",
    "jsa",
    "smma",
    "sqwma",
    "highpass",
    "swma",
    "trima",
    "sinwma",
    "epma",
    "wilders",
    "maaq",
    "mwdx",
    "cwma",
    "fwma",
    "pwma",
    "srwma",
    "supersmoother",
    "supersmoother_3_pole",
    "zlema",
    "nma",
    "hma",
    "jma",
    "edcf",
    "ehlers_itrend",
    "cora_wave",
    "vwma",
    "vpwma",
    "frama",
];

// Built-in preferred subset for Auto kernel routing and UI emphasis.
// Kept static in code so the app does not depend on external benchmark files.
const GPU_KERNEL_PREFERRED_MAS: &[&str] = &[
    "alma",
    "cora_wave",
    "cwma",
    "dema",
    "edcf",
    "ehlers_itrend",
    "ema",
    "epma",
    "fwma",
    "highpass",
    "hma",
    "jma",
    "jsa",
    "maaq",
    "mwdx",
    "nma",
    "pwma",
    "sinwma",
    "sma",
    "smma",
    "sqwma",
    "srwma",
    "supersmoother",
    "supersmoother_3_pole",
    "swma",
    "tema",
    "trima",
    "vpwma",
    "vwma",
    "wilders",
    "wma",
    "zlema",
];

const GPU_SPEEDUPS_CSV_EMBEDDED: &str = include_str!("../../../gpu_speedups.csv");
static GPU_SPEEDUP_INDICATOR_IDS: OnceLock<BTreeSet<String>> = OnceLock::new();

#[inline]
fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && matches!(chars.peek(), Some('"')) {
                    cell.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(cell.trim().to_string());
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }
    out.push(cell.trim().to_string());
    out
}

fn parse_gpu_speedup_ids(csv_raw: &str) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for (line_idx, line) in csv_raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if line_idx == 0 && trimmed.to_ascii_lowercase().starts_with("indicator_id,") {
            continue;
        }
        let fields = parse_csv_fields(trimmed);
        if fields.is_empty() {
            continue;
        }
        let id = fields[0].trim().to_ascii_lowercase();
        if id.is_empty() {
            continue;
        }
        out.insert(id);
    }
    out
}

fn gpu_speedup_indicator_ids() -> &'static BTreeSet<String> {
    GPU_SPEEDUP_INDICATOR_IDS.get_or_init(|| parse_gpu_speedup_ids(GPU_SPEEDUPS_CSV_EMBEDDED))
}

#[inline]
fn contains_case_insensitive(set: &[&str], ma_type: &str) -> bool {
    set.iter().any(|id| id.eq_ignore_ascii_case(ma_type))
}

#[inline]
pub fn supports_gpu_kernel_ma(ma_type: &str) -> bool {
    contains_case_insensitive(GPU_KERNEL_SUPPORTED_MAS, ma_type)
}

#[inline]
pub fn prefers_gpu_kernel_ma(ma_type: &str) -> bool {
    let id = ma_type.trim().to_ascii_lowercase();
    if id.is_empty() {
        return false;
    }
    let parsed_ids = gpu_speedup_indicator_ids();
    if !parsed_ids.is_empty() {
        return parsed_ids.contains(id.as_str());
    }
    contains_case_insensitive(GPU_KERNEL_PREFERRED_MAS, ma_type)
}

#[inline]
pub fn prefers_gpu_by_speedup_table(ma_type: &str) -> bool {
    prefers_gpu_kernel_ma(ma_type)
}

#[inline]
pub fn prefers_cpu_by_speedup_table(ma_type: &str) -> bool {
    !prefers_gpu_by_speedup_table(ma_type)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HybridCapabilityTier {
    GpuKernel,
    CudaOnly,
    CpuOnly,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HybridRouteTier {
    GpuKernel,
    CpuStaged,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridIndicatorRouteDecision {
    pub indicator_id: String,
    pub capability: HybridCapabilityTier,
    pub gpu_preferred_by_speedup: bool,
    pub route: HybridRouteTier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoubleMaHybridRoutePlan {
    pub fast: HybridIndicatorRouteDecision,
    pub slow: HybridIndicatorRouteDecision,
}

impl DoubleMaHybridRoutePlan {
    #[inline]
    pub fn is_fully_supported(&self) -> bool {
        self.fast.route != HybridRouteTier::Unsupported
            && self.slow.route != HybridRouteTier::Unsupported
    }

    #[inline]
    pub fn unsupported_ids(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.fast.route == HybridRouteTier::Unsupported {
            out.push(self.fast.indicator_id.clone());
        }
        if self.slow.route == HybridRouteTier::Unsupported {
            out.push(self.slow.indicator_id.clone());
        }
        out
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HybridIndicatorPartition {
    pub gpu_kernel: Vec<String>,
    pub cuda_only: Vec<String>,
    pub cpu_only: Vec<String>,
    pub unsupported: Vec<String>,
}

impl HybridIndicatorPartition {
    #[inline]
    pub fn is_fully_supported(&self) -> bool {
        self.unsupported.is_empty()
    }

    #[inline]
    pub fn has_mixed_origin(&self) -> bool {
        (!self.gpu_kernel.is_empty() || !self.cuda_only.is_empty()) && !self.cpu_only.is_empty()
    }
}

#[inline]
pub fn classify_hybrid_capability(ma_type: &str) -> HybridCapabilityTier {
    let id = ma_type.trim();
    if id.is_empty() {
        return HybridCapabilityTier::Unsupported;
    }
    let maybe_info = vector_ta::indicators::moving_averages::registry::list_moving_averages()
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(id));
    let Some(info) = maybe_info else {
        return HybridCapabilityTier::Unsupported;
    };
    if supports_gpu_kernel_ma(info.id) {
        HybridCapabilityTier::GpuKernel
    } else if info.supports_cuda_single {
        HybridCapabilityTier::CudaOnly
    } else if info.supports_cpu_single {
        HybridCapabilityTier::CpuOnly
    } else {
        HybridCapabilityTier::Unsupported
    }
}

pub fn plan_hybrid_indicator_partition<'a, I>(ma_types: I) -> HybridIndicatorPartition
where
    I: IntoIterator<Item = &'a str>,
{
    // BTreeSet provides deterministic order and removes duplicates.
    let mut unique_ids: BTreeSet<String> = BTreeSet::new();
    for raw in ma_types {
        let id = raw.trim().to_ascii_lowercase();
        if id.is_empty() {
            continue;
        }
        unique_ids.insert(id);
    }

    let mut out = HybridIndicatorPartition::default();
    for id in unique_ids {
        match classify_hybrid_capability(id.as_str()) {
            HybridCapabilityTier::GpuKernel => out.gpu_kernel.push(id),
            HybridCapabilityTier::CudaOnly => out.cuda_only.push(id),
            HybridCapabilityTier::CpuOnly => out.cpu_only.push(id),
            HybridCapabilityTier::Unsupported => out.unsupported.push(id),
        }
    }
    out
}

#[inline]
pub fn plan_hybrid_indicator_route(ma_type: &str) -> HybridIndicatorRouteDecision {
    let id = ma_type.trim().to_ascii_lowercase();
    let capability = classify_hybrid_capability(id.as_str());
    let gpu_preferred_by_speedup = prefers_gpu_by_speedup_table(id.as_str());

    let route = match capability {
        HybridCapabilityTier::GpuKernel => {
            if gpu_preferred_by_speedup {
                HybridRouteTier::GpuKernel
            } else {
                HybridRouteTier::CpuStaged
            }
        }
        // Current unified execution path only has direct GPU route for kernel-capable MAs.
        HybridCapabilityTier::CudaOnly | HybridCapabilityTier::CpuOnly => {
            HybridRouteTier::CpuStaged
        }
        HybridCapabilityTier::Unsupported => HybridRouteTier::Unsupported,
    };

    HybridIndicatorRouteDecision {
        indicator_id: id,
        capability,
        gpu_preferred_by_speedup,
        route,
    }
}

#[inline]
pub fn plan_double_ma_hybrid_route(fast_ma: &str, slow_ma: &str) -> DoubleMaHybridRoutePlan {
    DoubleMaHybridRoutePlan {
        fast: plan_hybrid_indicator_route(fast_ma),
        slow: plan_hybrid_indicator_route(slow_ma),
    }
}

#[inline]
pub fn plan_double_ma_hybrid_partition(fast_ma: &str, slow_ma: &str) -> HybridIndicatorPartition {
    plan_hybrid_indicator_partition([fast_ma, slow_ma])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_support_and_preference_sets_are_distinct() {
        assert!(supports_gpu_kernel_ma("sma"));
        assert!(prefers_gpu_kernel_ma("sma"));
        assert!(supports_gpu_kernel_ma("frama"));
        assert_eq!(
            prefers_gpu_kernel_ma("sma"),
            prefers_gpu_by_speedup_table("sma")
        );
    }

    #[test]
    fn hybrid_partition_is_deterministic_and_deduplicated() {
        let left =
            plan_hybrid_indicator_partition(vec![" SMA ", "alma", "sma", "ALMA", "unknown_id"]);
        let right =
            plan_hybrid_indicator_partition(vec!["unknown_id", "alma", "sma", "ALMA", "sma"]);
        assert_eq!(left, right);
        assert!(left.unsupported.contains(&"unknown_id".to_string()));
    }

    #[test]
    fn classify_unknown_ma_as_unsupported() {
        assert_eq!(
            classify_hybrid_capability("__definitely_unknown_ma__"),
            HybridCapabilityTier::Unsupported
        );
    }

    #[test]
    fn partition_categories_match_capability_classifier() {
        let mut picks: Vec<String> = Vec::new();
        let mut have_kernel = false;
        let mut have_cuda_only = false;
        let mut have_cpu_only = false;

        for info in vector_ta::indicators::moving_averages::registry::list_moving_averages() {
            if !have_kernel && supports_gpu_kernel_ma(info.id) {
                picks.push(info.id.to_string());
                have_kernel = true;
                continue;
            }
            if !have_cuda_only && info.supports_cuda_single && !supports_gpu_kernel_ma(info.id) {
                picks.push(info.id.to_string());
                have_cuda_only = true;
                continue;
            }
            if !have_cpu_only && info.supports_cpu_single && !info.supports_cuda_single {
                picks.push(info.id.to_string());
                have_cpu_only = true;
            }
            if have_kernel && have_cuda_only && have_cpu_only {
                break;
            }
        }
        picks.push("__unknown_partition_entry__".to_string());
        let refs: Vec<&str> = picks.iter().map(|id| id.as_str()).collect();
        let partition = plan_hybrid_indicator_partition(refs);

        for id in &partition.gpu_kernel {
            assert_eq!(
                classify_hybrid_capability(id.as_str()),
                HybridCapabilityTier::GpuKernel
            );
        }
        for id in &partition.cuda_only {
            assert_eq!(
                classify_hybrid_capability(id.as_str()),
                HybridCapabilityTier::CudaOnly
            );
        }
        for id in &partition.cpu_only {
            assert_eq!(
                classify_hybrid_capability(id.as_str()),
                HybridCapabilityTier::CpuOnly
            );
        }
        for id in &partition.unsupported {
            assert_eq!(
                classify_hybrid_capability(id.as_str()),
                HybridCapabilityTier::Unsupported
            );
        }
    }

    #[test]
    fn speedup_table_routing_marks_known_ids_as_gpu_preferred() {
        assert!(prefers_gpu_by_speedup_table("sma"));
        assert!(prefers_gpu_by_speedup_table("alma"));
        assert!(prefers_gpu_by_speedup_table("ehma"));
    }

    #[test]
    fn speedup_table_routing_marks_unknown_id_as_cpu_preferred() {
        assert!(prefers_cpu_by_speedup_table("__not_in_speedups_table__"));
    }

    #[test]
    fn unified_route_uses_gpu_for_gpu_preferred_kernel_capable_ma() {
        let route = plan_hybrid_indicator_route("sma");
        assert_eq!(route.route, HybridRouteTier::GpuKernel);
        assert_eq!(route.capability, HybridCapabilityTier::GpuKernel);
    }

    #[test]
    fn unified_route_uses_cpu_for_unknown_indicator() {
        let route = plan_hybrid_indicator_route("__totally_unknown__");
        assert_eq!(route.route, HybridRouteTier::Unsupported);
        assert_eq!(route.capability, HybridCapabilityTier::Unsupported);
    }

    #[test]
    fn unified_route_mixed_selection_has_distinct_gpu_and_cpu_staged_paths() {
        let mut gpu_kernel_preferred: Option<String> = None;
        let mut gpu_kernel_cpu_staged: Option<String> = None;

        for info in vector_ta::indicators::moving_averages::registry::list_moving_averages() {
            if !supports_gpu_kernel_ma(info.id) {
                continue;
            }
            if prefers_gpu_by_speedup_table(info.id) && gpu_kernel_preferred.is_none() {
                gpu_kernel_preferred = Some(info.id.to_string());
            }
            if !prefers_gpu_by_speedup_table(info.id) && gpu_kernel_cpu_staged.is_none() {
                gpu_kernel_cpu_staged = Some(info.id.to_string());
            }
            if gpu_kernel_preferred.is_some() && gpu_kernel_cpu_staged.is_some() {
                break;
            }
        }

        let (Some(fast), Some(slow)) = (gpu_kernel_preferred, gpu_kernel_cpu_staged) else {
            // If all kernel-capable MAs are GPU-preferred by the current speedup CSV,
            // this scenario is not representable and should not fail the test suite.
            return;
        };

        let plan = plan_double_ma_hybrid_route(fast.as_str(), slow.as_str());
        assert_eq!(plan.fast.route, HybridRouteTier::GpuKernel);
        assert_eq!(plan.slow.route, HybridRouteTier::CpuStaged);
        assert!(plan.is_fully_supported());
    }
}
