use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CdmcStrategyParams {
    #[serde(default = "default_cdmc_dpo_period")]
    pub dpo_period: u16,
    #[serde(default = "default_cdmc_correlation_cycle_period")]
    pub correlation_cycle_period: u16,
    #[serde(default = "default_cdmc_mama_fast_limit")]
    pub mama_fast_limit: f64,
    #[serde(default = "default_cdmc_mama_slow_limit")]
    pub mama_slow_limit: f64,
    #[serde(default = "default_cdmc_decycler_period")]
    pub decycler_period: u16,
    #[serde(default = "default_cdmc_phase_threshold")]
    pub phase_threshold: f64,
}

fn default_cdmc_dpo_period() -> u16 {
    20
}

fn default_cdmc_correlation_cycle_period() -> u16 {
    20
}

fn default_cdmc_mama_fast_limit() -> f64 {
    0.5
}

fn default_cdmc_mama_slow_limit() -> f64 {
    0.05
}

fn default_cdmc_decycler_period() -> u16 {
    20
}

fn default_cdmc_phase_threshold() -> f64 {
    0.0
}

impl Default for CdmcStrategyParams {
    fn default() -> Self {
        Self {
            dpo_period: default_cdmc_dpo_period(),
            correlation_cycle_period: default_cdmc_correlation_cycle_period(),
            mama_fast_limit: default_cdmc_mama_fast_limit(),
            mama_slow_limit: default_cdmc_mama_slow_limit(),
            decycler_period: default_cdmc_decycler_period(),
            phase_threshold: default_cdmc_phase_threshold(),
        }
    }
}

pub fn validate_cdmc_params(cfg: &CdmcStrategyParams) -> Result<(), String> {
    if cfg.dpo_period == 0 {
        return Err("CDMC dpo_period must be >= 1.".to_string());
    }
    if cfg.correlation_cycle_period == 0 {
        return Err("CDMC correlation_cycle_period must be >= 1.".to_string());
    }
    if !cfg.mama_fast_limit.is_finite() || cfg.mama_fast_limit <= 0.0 || cfg.mama_fast_limit > 1.0 {
        return Err("CDMC mama_fast_limit must be in (0, 1].".to_string());
    }
    if !cfg.mama_slow_limit.is_finite() || cfg.mama_slow_limit <= 0.0 || cfg.mama_slow_limit > 1.0 {
        return Err("CDMC mama_slow_limit must be in (0, 1].".to_string());
    }
    if cfg.mama_slow_limit > cfg.mama_fast_limit {
        return Err("CDMC mama_slow_limit must be <= mama_fast_limit.".to_string());
    }
    if cfg.decycler_period == 0 {
        return Err("CDMC decycler_period must be >= 1.".to_string());
    }
    if !cfg.phase_threshold.is_finite() {
        return Err("CDMC phase_threshold must be finite.".to_string());
    }
    Ok(())
}
