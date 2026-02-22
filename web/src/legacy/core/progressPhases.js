const PHASE_LABELS = {
  "cpu": "CPU",
  "gpu-sweep": "GPU sweep",
  "gpu-kernel": "GPU kernel",
  "parity_cpu": "Parity (CPU)",
  "parity_gpu_sweep": "Parity (GPU sweep)",
  "parity_gpu_kernel": "Parity (GPU kernel)",
  "composite_cpu": "Composite (CPU)",
  "distributed_assets": "Distributed (assets)",
  "distributed_tiles": "Distributed (tiles)",
  "double_ma_validate": "Validation",
  "double_ma_validate_cross": "Validation (cross-ticker)",
  "double_ma_validate_cscv_prep": "Validation (CSCV prep)",
  "double_ma_validate_cscv": "Validation (CSCV/PBO)",
  "composite_validate": "Validation",
  "composite_validate_cross": "Validation (cross-ticker)",
  "composite_validate_cscv_prep": "Validation (CSCV prep)",
  "composite_validate_cscv": "Validation (CSCV/PBO)",
  "composite_mo_prep": "Composite MO (prep)",
  "composite_mo_nsga2": "Composite MO (NSGA-II)",
  "composite_mo_random": "Composite MO (Random)",
  "composite_mo_tpe": "Composite MO (TPE)",
  "composite_mo_qmc": "Composite MO (QMC)",
  "meta_read": "Meta (read artifacts)",
  "meta_scan": "Meta (scan candidates)",
  "meta_aggregate": "Meta (aggregate)",
  "meta_holdout": "Meta (holdout)",
  "meta_export": "Meta (export XLSX)",
  "meta_save": "Meta (save JSON)",
  "meta_stage4": "Meta Stage 4 (ML finalize)",
  "export_xlsx": "Export XLSX report",
  "reveal_report": "Reveal report",
};

const PHASE_HINTS = {
  "cpu": "Evaluating MA parameter pairs on CPU.",
  "gpu-sweep": "Evaluating MA pairs with GPU indicator sweeps.",
  "gpu-kernel": "Evaluating MA pairs in VRAM kernel backtester.",
  "composite_cpu": "Evaluating composite strategy parameter grid.",
  "composite_mo_prep": "Preparing search space and optional prescan ranges.",
  "composite_mo_nsga2": "Evolving multi-objective population (NSGA-II).",
  "composite_mo_random": "Random-searching multi-objective candidates.",
  "composite_mo_tpe": "Model-guided search (TPE) is sampling trials.",
  "composite_mo_qmc": "Low-discrepancy (QMC) sampling is in progress.",
  "double_ma_validate": "Validating shortlisted candidates on current dataset.",
  "composite_validate": "Validating shortlisted candidates on current dataset.",
  "double_ma_validate_cross": "Running cross-ticker validation on selected datasets.",
  "composite_validate_cross": "Running cross-ticker validation on selected datasets.",
  "double_ma_validate_cscv_prep": "Preparing slice returns for CSCV/PBO.",
  "composite_validate_cscv_prep": "Preparing slice returns for CSCV/PBO.",
  "double_ma_validate_cscv": "Computing CSCV/PBO combinations.",
  "composite_validate_cscv": "Computing CSCV/PBO combinations.",
  "distributed_assets": "Dispatching and collecting asset-level distributed runs.",
  "distributed_tiles": "Dispatching and collecting tiled distributed runs.",
  "meta_read": "Reading saved run artifacts from disk.",
  "meta_scan": "Scanning artifacts and clustering parameter groups.",
  "meta_aggregate": "Aggregating robust statistics across artifacts.",
  "meta_holdout": "Scoring groups on holdout datasets.",
  "meta_export": "Generating meta XLSX report.",
  "meta_save": "Saving meta JSON artifact.",
  "meta_stage4": "Applying ML confidence finalizer to the latest meta groups.",
  "export_xlsx": "Building and saving XLSX report.",
  "reveal_report": "Opening report location in file explorer.",
};

const PHASE_GROUP_LABELS = {
  optimize: "Optimize",
  validate: "Validate",
  meta: "Meta",
  export: "Export",
  reveal: "Reveal",
};

export function phaseGroup(phase) {
  const ph = String(phase || "").trim();
  if (!ph) return "";
  if (ph.startsWith("meta_")) return "meta";
  if (ph === "export_xlsx" || ph.startsWith("export_")) return "export";
  if (ph === "reveal_report" || ph.startsWith("reveal_")) return "reveal";
  if (ph.startsWith("double_ma_validate")) return "validate";
  if (ph.startsWith("composite_validate")) return "validate";
  if (ph.startsWith("distributed_")) return "optimize";
  if (ph.startsWith("parity_")) return "optimize";
  if (ph.startsWith("composite_")) return "optimize";
  if (ph === "cpu" || ph.startsWith("gpu")) return "optimize";
  return "";
}

export function phaseDisplay(phase) {
  const ph = String(phase || "").trim();
  const detail = PHASE_LABELS[ph] || ph || "Working";
  const group = phaseGroup(ph);
  const groupLabel = PHASE_GROUP_LABELS[group] || "";
  if (!groupLabel) return { group, groupLabel: "", detailLabel: detail, label: detail };
  const label = detail && detail !== groupLabel ? `${groupLabel} · ${detail}` : groupLabel;
  return { group, groupLabel, detailLabel: detail, label };
}

export function phaseLabel(phase) {
  return phaseDisplay(phase).label;
}

export function phaseHint(phase) {
  const ph = String(phase || "").trim();
  const hint = String(PHASE_HINTS[ph] || "").trim();
  if (hint) return hint;
  const group = phaseGroup(ph);
  if (group === "optimize") return "Evaluating parameter combinations.";
  if (group === "validate") return "Computing robustness checks for selected candidates.";
  if (group === "meta") return "Aggregating and scoring run artifacts.";
  if (group === "export") return "Preparing export output.";
  if (group === "reveal") return "Opening generated output.";
  return "";
}

export function phaseProgressUnit(phase) {
  const ph = String(phase || "").trim();
  if (!ph) return "items";
  if (ph === "cpu" || ph === "gpu-sweep" || ph === "gpu-kernel" || ph === "composite_cpu") return "pairs";
  if (ph.startsWith("composite_mo_")) return "evals";
  if (ph.endsWith("_cross")) return "datasets";
  if (ph.endsWith("_cscv")) return "combos";
  if (ph.startsWith("meta_")) return ph === "meta_holdout" ? "groups" : "artifacts";
  if (ph.startsWith("distributed_")) return "tasks";
  if (ph.startsWith("parity_")) return "runs";
  return "items";
}
