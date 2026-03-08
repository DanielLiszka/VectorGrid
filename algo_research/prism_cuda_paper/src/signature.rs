use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RegimeGate {
    None,
    SetGate,
    VtrsGate,
}

impl RegimeGate {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SetGate => "set_gate",
            Self::VtrsGate => "vtrs_gate",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EntryModule {
    Vcc,
    Vemr,
    Abmt,
    Tdb,
}

impl EntryModule {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vcc => "vcc",
            Self::Vemr => "vemr",
            Self::Abmt => "abmt",
            Self::Tdb => "tdb",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RiskOverlay {
    Atr,
    DevStop,
    Chandelier,
}

impl RiskOverlay {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Atr => "atr",
            Self::DevStop => "devstop",
            Self::Chandelier => "chandelier",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum MaType {
    None,
    Alma,
    Ema,
    Sma,
    Wma,
    Vidya,
}

impl MaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Alma => "alma",
            Self::Ema => "ema",
            Self::Sma => "sma",
            Self::Wma => "wma",
            Self::Vidya => "vidya",
        }
    }
}

#[derive(Clone, Debug)]
pub struct LatentParamSpec {
    pub name: &'static str,
    pub low: f32,
    pub high: f32,
    pub kind: &'static str,
    pub step: Option<f32>,
    pub group: &'static str,
    pub quantile_normalized: bool,
}

impl LatentParamSpec {
    pub fn decode(&self, u: f32) -> f32 {
        let uu = u.clamp(0.0, 1.0);
        let mut v = self.low + uu * (self.high - self.low);
        if let Some(step) = self.step {
            let n = ((v - self.low) / step).round();
            v = self.low + n * step;
        }
        if self.kind == "int" {
            v = v.round();
        }
        v.clamp(self.low, self.high)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct StrategySignature {
    pub family: &'static str,
    pub regime_gate: RegimeGate,
    pub entry_module: EntryModule,
    pub risk_overlay: RiskOverlay,
    pub ma_type: MaType,
    pub coverage_mode: bool,
}

impl StrategySignature {
    pub fn key(&self) -> String {
        format!(
            "{}::{}::{}::{}::{}::{}",
            self.family,
            self.regime_gate.as_str(),
            self.entry_module.as_str(),
            self.risk_overlay.as_str(),
            self.ma_type.as_str(),
            if self.coverage_mode { "cov1" } else { "cov0" }
        )
    }

    pub fn graph_shape(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.regime_gate.as_str(),
            self.entry_module.as_str(),
            self.risk_overlay.as_str(),
            self.ma_type.as_str()
        )
    }
}

#[derive(Clone, Debug)]
pub struct StrategyProfile {
    pub signature: StrategySignature,
    pub numeric_specs: Vec<LatentParamSpec>,
    pub indicator_count: usize,
    pub kernel_families: Vec<&'static str>,
    pub search_cap: usize,
}

impl StrategyProfile {
    pub fn dim(&self) -> usize {
        self.numeric_specs.len()
    }

    pub fn latent_groups(&self) -> HashMap<String, Vec<usize>> {
        let mut out: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, spec) in self.numeric_specs.iter().enumerate() {
            out.entry(spec.group.to_string()).or_default().push(i);
        }
        out
    }

    pub fn decode(&self, latent: &[f32]) -> HashMap<String, f32> {
        let mut out = HashMap::new();
        for (u, spec) in latent.iter().zip(self.numeric_specs.iter()) {
            out.insert(spec.name.to_string(), spec.decode(*u));
        }
        // Derived valid latent coordinates.
        if let (Some(fast), Some(gap)) = (out.get("fast_len").copied(), out.get("slow_gap").copied()) {
            out.insert("slow_len".to_string(), fast + gap.max(1.0));
        }
        if let (Some(entry), Some(exit_gap)) = (out.get("entry_q").copied(), out.get("exit_gap_q").copied()) {
            out.insert("exit_q".to_string(), (entry + exit_gap).min(0.99));
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct StrategyRegistry {
    pub profiles: HashMap<String, StrategyProfile>,
}

impl StrategyRegistry {
    pub fn keys(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    pub fn get(&self, key: &str) -> &StrategyProfile {
        self.profiles.get(key).expect("missing strategy profile")
    }
}

fn specs_for_regime(g: RegimeGate) -> (usize, Vec<&'static str>, Vec<LatentParamSpec>) {
    match g {
        RegimeGate::None => (0, vec![], vec![]),
        RegimeGate::SetGate => (
            3,
            vec!["linreg", "osc", "trend"],
            vec![
                LatentParamSpec { name: "linreg_len", low: 10.0, high: 80.0, kind: "int", step: Some(1.0), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "angle_min", low: 5.0, high: 45.0, kind: "float", step: Some(1.0), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "er_len", low: 8.0, high: 64.0, kind: "int", step: Some(1.0), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "er_min", low: 0.15, high: 0.60, kind: "float", step: Some(0.01), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "chop_len", low: 8.0, high: 64.0, kind: "int", step: Some(1.0), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "chop_max", low: 40.0, high: 70.0, kind: "float", step: Some(1.0), group: "regime", quantile_normalized: false },
            ],
        ),
        RegimeGate::VtrsGate => (
            3,
            vec!["stats", "baseline"],
            vec![
                LatentParamSpec { name: "vol_len", low: 20.0, high: 256.0, kind: "int", step: Some(1.0), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "vol_q", low: 0.60, high: 0.95, kind: "float", step: Some(0.05), group: "regime", quantile_normalized: true },
                LatentParamSpec { name: "kurt_len", low: 50.0, high: 400.0, kind: "int", step: Some(1.0), group: "regime", quantile_normalized: false },
                LatentParamSpec { name: "kurt_q", low: 0.60, high: 0.95, kind: "float", step: Some(0.05), group: "regime", quantile_normalized: true },
                LatentParamSpec { name: "buff_len", low: 8.0, high: 128.0, kind: "int", step: Some(1.0), group: "regime", quantile_normalized: false },
            ],
        ),
    }
}

fn specs_for_entry(e: EntryModule) -> (usize, Vec<&'static str>, usize, Vec<LatentParamSpec>) {
    match e {
        EntryModule::Vcc => (
            3,
            vec!["vol", "momentum", "trend"],
            80_000,
            vec![
                LatentParamSpec { name: "bb_len", low: 10.0, high: 40.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "kc_len", low: 10.0, high: 40.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "mom_len", low: 5.0, high: 50.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "fast_len", low: 4.0, high: 64.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "slow_gap", low: 1.0, high: 64.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "entry_q", low: 0.70, high: 0.98, kind: "float", step: Some(0.01), group: "entry", quantile_normalized: true },
                LatentParamSpec { name: "exit_gap_q", low: 0.00, high: 0.15, kind: "float", step: Some(0.01), group: "entry", quantile_normalized: true },
            ],
        ),
        EntryModule::Vemr => (
            3,
            vec!["stats", "vol"],
            80_000,
            vec![
                LatentParamSpec { name: "z_len", low: 10.0, high: 100.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "z_entry", low: 1.0, high: 3.5, kind: "float", step: Some(0.05), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "z_exit", low: 0.0, high: 1.0, kind: "float", step: Some(0.05), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "bb_len", low: 10.0, high: 60.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "entry_q", low: 0.70, high: 0.98, kind: "float", step: Some(0.01), group: "entry", quantile_normalized: true },
                LatentParamSpec { name: "exit_gap_q", low: 0.00, high: 0.20, kind: "float", step: Some(0.01), group: "entry", quantile_normalized: true },
            ],
        ),
        EntryModule::Abmt => (
            2,
            vec!["ma", "momentum"],
            70_000,
            vec![
                LatentParamSpec { name: "baseline_len", low: 10.0, high: 200.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "macz_fast", low: 5.0, high: 20.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "macz_slow", low: 15.0, high: 60.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "macz_signal", low: 3.0, high: 20.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
            ],
        ),
        EntryModule::Tdb => (
            4,
            vec!["channel", "trend", "risk"],
            80_000,
            vec![
                LatentParamSpec { name: "donchian_len", low: 10.0, high: 200.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "di_len", low: 7.0, high: 50.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "adxr_len", low: 7.0, high: 50.0, kind: "int", step: Some(1.0), group: "entry", quantile_normalized: false },
                LatentParamSpec { name: "adxr_min", low: 10.0, high: 40.0, kind: "float", step: Some(1.0), group: "entry", quantile_normalized: false },
            ],
        ),
    }
}

fn specs_for_risk(r: RiskOverlay) -> (usize, Vec<&'static str>, Vec<LatentParamSpec>) {
    match r {
        RiskOverlay::Atr => (
            1,
            vec!["risk"],
            vec![
                LatentParamSpec { name: "atr_len", low: 7.0, high: 50.0, kind: "int", step: Some(1.0), group: "risk", quantile_normalized: false },
                LatentParamSpec { name: "atr_mult", low: 1.0, high: 5.0, kind: "float", step: Some(0.25), group: "risk", quantile_normalized: false },
            ],
        ),
        RiskOverlay::DevStop => (
            1,
            vec!["risk"],
            vec![
                LatentParamSpec { name: "dev_len", low: 5.0, high: 100.0, kind: "int", step: Some(1.0), group: "risk", quantile_normalized: false },
                LatentParamSpec { name: "dev_mult", low: 1.0, high: 6.0, kind: "float", step: Some(0.25), group: "risk", quantile_normalized: false },
            ],
        ),
        RiskOverlay::Chandelier => (
            1,
            vec!["risk"],
            vec![
                LatentParamSpec { name: "chand_period", low: 10.0, high: 50.0, kind: "int", step: Some(1.0), group: "risk", quantile_normalized: false },
                LatentParamSpec { name: "chand_mult", low: 1.5, high: 5.0, kind: "float", step: Some(0.25), group: "risk", quantile_normalized: false },
            ],
        ),
    }
}

pub fn build_modular_triad_registry() -> StrategyRegistry {
    let regimes = [RegimeGate::None, RegimeGate::SetGate, RegimeGate::VtrsGate];
    let entries = [EntryModule::Vcc, EntryModule::Vemr, EntryModule::Abmt, EntryModule::Tdb];
    let risks = [RiskOverlay::Atr, RiskOverlay::DevStop, RiskOverlay::Chandelier];
    let ma_types = [MaType::None, MaType::Alma, MaType::Ema, MaType::Sma, MaType::Wma, MaType::Vidya];

    let mut profiles = HashMap::new();
    for regime in regimes {
        for entry in entries {
            for risk in risks {
                let mats: Vec<MaType> = if matches!(entry, EntryModule::Abmt) {
                    ma_types.to_vec()
                } else {
                    vec![MaType::None]
                };
                for ma_type in mats {
                    let sig = StrategySignature {
                        family: "modular_triad",
                        regime_gate: regime,
                        entry_module: entry,
                        risk_overlay: risk,
                        ma_type,
                        coverage_mode: false,
                    };
                    let (ri, rk, rs) = specs_for_regime(regime);
                    let (ei, ek, cap, es) = specs_for_entry(entry);
                    let (ki, kk, ks) = specs_for_risk(risk);
                    let mut specs = Vec::new();
                    specs.extend(rs);
                    specs.extend(es);
                    specs.extend(ks);
                    if matches!(entry, EntryModule::Abmt) && !matches!(ma_type, MaType::None) {
                        specs.push(LatentParamSpec {
                            name: "ma_type_bias",
                            low: 0.0,
                            high: 1.0,
                            kind: "float",
                            step: Some(0.05),
                            group: "entry",
                            quantile_normalized: false,
                        });
                    }
                    let mut kernels = Vec::new();
                    kernels.extend(rk);
                    kernels.extend(ek);
                    kernels.extend(kk);
                    kernels.sort();
                    kernels.dedup();
                    let indicator_count = ri + ei + ki;
                    profiles.insert(
                        sig.key(),
                        StrategyProfile {
                            signature: sig,
                            numeric_specs: specs,
                            indicator_count,
                            kernel_families: kernels,
                            search_cap: cap,
                        },
                    );
                }
            }
        }
    }
    StrategyRegistry { profiles }
}
