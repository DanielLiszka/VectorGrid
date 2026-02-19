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
    contains_case_insensitive(GPU_KERNEL_PREFERRED_MAS, ma_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_support_and_preference_sets_are_distinct() {
        assert!(supports_gpu_kernel_ma("sma"));
        assert!(prefers_gpu_kernel_ma("sma"));
        assert!(supports_gpu_kernel_ma("frama"));
        assert!(!prefers_gpu_kernel_ma("frama"));
    }
}
