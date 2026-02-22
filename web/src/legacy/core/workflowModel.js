export const WORKFLOW_TABS = ["data", "optimize", "validate", "analyze", "history", "settings"];
export const WORKFLOW_TAB_SET = new Set(WORKFLOW_TABS);
export const WORKFLOW_DEFAULTS = {
  data: { controls: "data", results: "overview" },
  optimize: { controls: "opt", results: "overview" },
  validate: { controls: "opt", results: "validate" },
  analyze: { controls: "opt", results: "drill" },
  history: { controls: "data", results: "history" },
  settings: { controls: "advanced", results: "meta" },
};
export const WORKFLOW_ALLOWED = {
  data: { controls: ["data"], results: ["overview"] },
  optimize: { controls: ["opt", "strategy", "advanced"], results: ["overview", "top", "pareto", "heatmap"] },
  validate: { controls: ["opt"], results: ["validate"] },
  analyze: { controls: ["opt"], results: ["overview", "top", "pareto", "drill", "heatmap"] },
  history: { controls: ["data"], results: ["history"] },
  settings: { controls: ["advanced", "distributed"], results: ["meta", "debug"] },
};
export const WORKFLOW_SUBTITLES = {
  data: "Load a dataset (CSV or Polygon) and select the optimization timeframe.",
  optimize: "Configure strategy + search space, then run an optimization.",
  validate: "Run robustness checks on the current run’s candidates.",
  analyze: "Inspect results via drilldown curves and heatmaps.",
  history: "Review, compare, and rerun saved optimizations.",
  settings: "Advanced tools (distributed runs, meta selection, debug).",
};

export const WORKSPACE_TABS = ["workflow", "library", "aggregate", "pro", "settings"];
export const WORKSPACE_TAB_SET = new Set(WORKSPACE_TABS);
export const WORKFLOW_STEPS = ["data", "optimize", "validate", "analyze"];
export const WORKFLOW_STEP_SET = new Set(WORKFLOW_STEPS);
export const AGGREGATE_TABS = ["meta", "batch", "saved", "compare"];
export const AGGREGATE_TAB_SET = new Set(AGGREGATE_TABS);
export const WORKFLOW_STEP_SUBTITLES = {
  data: "Data source: load CSV or Polygon candles, then confirm timeframe.",
  optimize: "Strategy setup: configure ranges, backend, and run optimization.",
  validate: "Validation: run robustness checks on the selected optimization.",
  analyze: "Summary: inspect top results, Pareto, drilldown, and heatmap.",
};
export const WORKSPACE_SUBTITLES = {
  workflow: "Guided pipeline for data -> optimize -> validate -> summary.",
  library: "Browse, compare, and rerun saved optimization artifacts.",
  aggregate: "Aggregate many runs into robust shortlist recommendations.",
  pro: "Distributed compute, advanced controls, and debug output.",
  settings: "Application preferences and storage configuration.",
};

export const CONTROLS_TAB_VALUES = ["data", "opt", "strategy", "advanced", "distributed"];
export const CONTROLS_TAB_SET = new Set(CONTROLS_TAB_VALUES);
export const RESULTS_TAB_VALUES = ["overview", "top", "pareto", "validate", "drill", "heatmap", "history", "meta", "debug"];
export const RESULTS_TAB_SET = new Set(RESULTS_TAB_VALUES);

export const TAB_LABELS = {
  controls: {
    data: "Data",
    opt: "Optimize",
    strategy: "Strategy",
    advanced: "Advanced",
    distributed: "Distributed",
  },
  results: {
    overview: "Overview",
    top: "Top",
    pareto: "Pareto",
    validate: "Validate",
    drill: "Drilldown",
    heatmap: "Heatmap",
    history: "History",
    meta: "Meta",
    debug: "Debug",
  },
};

export function normalizeWorkflowTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return WORKFLOW_TAB_SET.has(x) ? x : "";
}

export function normalizeWorkspaceTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return WORKSPACE_TAB_SET.has(x) ? x : "";
}

export function normalizeWorkflowStep(v) {
  const x = String(v || "").trim().toLowerCase();
  return WORKFLOW_STEP_SET.has(x) ? x : "";
}

export function normalizeAggregateTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return AGGREGATE_TAB_SET.has(x) ? x : "";
}

export function normalizeControlsTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return CONTROLS_TAB_SET.has(x) ? x : "";
}

export function normalizeResultsTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return RESULTS_TAB_SET.has(x) ? x : "";
}

export function workflowStepFromWorkflow(workflow) {
  const wf = normalizeWorkflowTab(workflow);
  return normalizeWorkflowStep(wf) || "data";
}
