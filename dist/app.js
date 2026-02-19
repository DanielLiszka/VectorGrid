import { CandlestickSeries, HistogramSeries, LineSeries, createChart, createSeriesMarkers } from "./vendor/lightweight-charts.standalone.production.mjs";

const $ = (id) => document.getElementById(id);

function tauriInvoke() {
  const t = window.__TAURI__ || {};
  return t?.core?.invoke || t?.invoke || null;
}

function tauriListen() {
  const t = window.__TAURI__ || {};
  return t?.event?.listen || null;
}

function setHintVisible(id, visible) {
  const el = $(id);
  if (!el) return;
  el.style.display = visible ? "block" : "none";
}

let UI_REDUCED_MOTION_MQL = null;
let UI_REDUCED_MOTION_LISTENER_ATTACHED = false;

function prefersReducedMotion() {
  if (typeof window.matchMedia !== "function") return false;
  if (!UI_REDUCED_MOTION_MQL) UI_REDUCED_MOTION_MQL = window.matchMedia("(prefers-reduced-motion: reduce)");
  return !!UI_REDUCED_MOTION_MQL.matches;
}

function supportsViewTransitions() {
  return typeof document.startViewTransition === "function";
}

function syncUiTransitionClass() {
  const root = document.documentElement;
  if (!root) return;
  root.classList.toggle("ui-transitions", !prefersReducedMotion());
}

function initUiMotionPrefs() {
  syncUiTransitionClass();
  if (UI_REDUCED_MOTION_LISTENER_ATTACHED) return;
  if (typeof window.matchMedia !== "function") return;
  if (!UI_REDUCED_MOTION_MQL) UI_REDUCED_MOTION_MQL = window.matchMedia("(prefers-reduced-motion: reduce)");
  const onChange = () => syncUiTransitionClass();
  try {
    if (UI_REDUCED_MOTION_MQL.addEventListener) UI_REDUCED_MOTION_MQL.addEventListener("change", onChange);
    else UI_REDUCED_MOTION_MQL.addListener(onChange);
    UI_REDUCED_MOTION_LISTENER_ATTACHED = true;
  } catch {}
}

function runUiTransition(updateFn, opts) {
  if (typeof updateFn !== "function") return;
  const animate = opts?.animate === true;
  if (!animate || document.hidden || prefersReducedMotion()) {
    updateFn();
    return;
  }
  if (supportsViewTransitions()) {
    try {
      document.startViewTransition(() => {
        updateFn();
      });
      return;
    } catch {}
  }
  updateFn();
}

const CONTROLS_TAB_KEY = "vectorbt_controls_tab_v1";
const RESULTS_TAB_KEY = "vectorbt_results_tab_v1";
let CONTROLS_TABS = null;
let RESULTS_TABS = null;

const WORKFLOW_TAB_KEY = "vta_workflow_tab_v1";
const WORKFLOW_STATE_KEY = "vta_workflow_state_v1";
const WORKSPACE_TAB_KEY = "vta_workspace_tab_v1";
const WORKFLOW_STEP_KEY = "vta_workflow_step_v1";
const WORKFLOW_ADVANCED_KEY = "vta_workflow_advanced_v1";
const AGGREGATE_TAB_KEY = "vta_aggregate_tab_v1";
let CURRENT_WORKFLOW = "";
let WORKFLOW_STATE = {};
let WORKFLOW_STATE_SAVE_SEQ = 0;
let WORKFLOW_SYNC_SUPPRESS = false;
let WORKFLOW_UI = null;
let CURRENT_WORKSPACE = "";
let CURRENT_WORKFLOW_STEP = "data";
let CURRENT_AGGREGATE_TAB = "meta";
let WORKSPACE_NAV_UI = null;
let WORKFLOW_STEP_NAV_UI = null;
let WORKSPACE_OVERRIDE = "";
let WORKSPACE_SYNC_SUPPRESS = false;

const CONTROLS_WIDTH_KEY = "vta_controls_width_v1";
const CONTROLS_COLLAPSED_KEY = "vta_controls_collapsed_v1";
const CONTROLS_W_MIN = 320;
const CONTROLS_W_MAX = 820;

const WINDOW_EFFECT_KEY = "vta_window_effect_v1";

const THEME_PREF_KEY = "vta_theme_pref_v1";
let THEME_MQL = null;
let THEME_MQL_HANDLER = null;

function isValidThemePref(v) {
  return v === "light" || v === "dark" || v === "system";
}

function resolvedThemeFromPref(pref) {
  const p = String(pref || "").trim();
  if (p === "dark") return "dark";
  if (p === "light") return "light";
  if (p !== "system") return "light";
  if (!THEME_MQL && window.matchMedia) THEME_MQL = window.matchMedia("(prefers-color-scheme: dark)");
  return THEME_MQL && THEME_MQL.matches ? "dark" : "light";
}

function detachThemeListener() {
  if (!THEME_MQL || !THEME_MQL_HANDLER) return;
  try {
    if (THEME_MQL.removeEventListener) THEME_MQL.removeEventListener("change", THEME_MQL_HANDLER);
    else THEME_MQL.removeListener(THEME_MQL_HANDLER);
  } catch {}
  THEME_MQL_HANDLER = null;
}

function applyThemePref(pref, opts) {
  const p0 = String(pref || "").trim();
  const p = isValidThemePref(p0) ? p0 : "light";
  const persist = opts?.persist !== false;

  const root = document.documentElement;
  root.dataset.themePref = p;
  root.dataset.theme = resolvedThemeFromPref(p);

  if (persist) {
    try { localStorage.setItem(THEME_PREF_KEY, p); } catch {}
  }

  const sel = $("themeSelect");
  if (sel && String(sel.value || "") !== p) sel.value = p;

  detachThemeListener();
  if (p === "system") {
    if (!THEME_MQL && window.matchMedia) THEME_MQL = window.matchMedia("(prefers-color-scheme: dark)");
    if (THEME_MQL) {
      THEME_MQL_HANDLER = () => {
        const root2 = document.documentElement;
        if (String(root2.dataset.themePref || "").trim() !== "system") return;
        root2.dataset.theme = resolvedThemeFromPref("system");
        refreshDrillTvChartTheme();
        refreshExecTvChartTheme();
      };
      try {
        if (THEME_MQL.addEventListener) THEME_MQL.addEventListener("change", THEME_MQL_HANDLER);
        else THEME_MQL.addListener(THEME_MQL_HANDLER);
      } catch {}
    }
  }

  refreshDrillTvChartTheme();
  refreshExecTvChartTheme();
  scheduleApplyWindowEffect();
  syncSettingsWorkspaceUi();
}

let WINDOW_EFFECT_APPLY_SEQ = 0;
let WINDOW_EFFECT_APPLY_INFLIGHT = 0;

function isValidWindowEffect(v) {
  const x = String(v || "").trim().toLowerCase();
  return x === "none" || x === "mica" || x === "acrylic" || x === "vibrancy";
}

function platformKind() {
  const ua = String(navigator?.userAgent || "").toLowerCase();
  if (ua.includes("windows")) return "windows";
  if (ua.includes("mac")) return "macos";
  if (ua.includes("linux")) return "linux";
  return "unknown";
}

function supportedWindowEffects() {
  const p = platformKind();
  if (p === "windows") return new Set(["none", "mica", "acrylic"]);
  if (p === "macos") return new Set(["none", "vibrancy"]);
  return new Set(["none"]);
}

function currentWindowEffect() {
  const eff = String(document.documentElement.dataset.windowEffect || "").trim().toLowerCase();
  return isValidWindowEffect(eff) ? eff : "none";
}

async function applyWindowEffectNative(effect) {
  const invoke = tauriInvoke();
  if (!invoke) return;

  const eff = isValidWindowEffect(effect) ? String(effect).toLowerCase() : "none";
  const dark = String(document.documentElement.dataset.theme || "light").trim() === "dark";

  try {
    WINDOW_EFFECT_APPLY_INFLIGHT += 1;
    await invoke("set_window_effect", { effect: eff === "none" ? null : eff, dark });
  } catch (e) {
    setStatus(`Window effect failed: ${String(e)}`, "danger");
  } finally {
    WINDOW_EFFECT_APPLY_INFLIGHT = Math.max(0, WINDOW_EFFECT_APPLY_INFLIGHT - 1);
  }
}

function scheduleApplyWindowEffect() {
  const eff = currentWindowEffect();
  if (!eff || eff === "none") return;
  WINDOW_EFFECT_APPLY_SEQ = (WINDOW_EFFECT_APPLY_SEQ + 1) >>> 0;
  const seq = WINDOW_EFFECT_APPLY_SEQ;
  setTimeout(() => {
    if (seq !== WINDOW_EFFECT_APPLY_SEQ) return;
    if (WINDOW_EFFECT_APPLY_INFLIGHT > 0) return;
    applyWindowEffectNative(eff);
  }, 0);
}

function applyWindowEffectPref(pref, opts) {
  const p0 = String(pref || "").trim().toLowerCase();
  const p = isValidWindowEffect(p0) ? p0 : "none";
  const supported = supportedWindowEffects();
  const chosen = supported.has(p) ? p : "none";
  const persist = opts?.persist !== false;

  document.documentElement.dataset.windowEffect = chosen;
  const sel = $("effectSelect");
  if (sel && String(sel.value || "") !== chosen) sel.value = chosen;

  if (persist) {
    try {
      if (chosen && chosen !== "none") localStorage.setItem(WINDOW_EFFECT_KEY, chosen);
      else localStorage.removeItem(WINDOW_EFFECT_KEY);
    } catch {}
  }

  applyWindowEffectNative(chosen);
  syncSettingsWorkspaceUi();
}

function initWindowEffectUi() {
  const sel = $("effectSelect");
  if (!sel) return;

  const supported = supportedWindowEffects();
  for (const opt of Array.from(sel.options || [])) {
    const v = String(opt.value || "").trim().toLowerCase();
    opt.disabled = !supported.has(v);
  }

  let pref = String(document.documentElement.dataset.windowEffect || "").trim().toLowerCase();
  if (!isValidWindowEffect(pref)) {
    try {
      const raw = localStorage.getItem(WINDOW_EFFECT_KEY);
      pref = String(raw || "").trim().toLowerCase();
    } catch {}
  }
  if (!isValidWindowEffect(pref)) pref = "none";
  if (!supported.has(pref)) pref = "none";

  sel.value = pref;
  sel.addEventListener("change", () => {
    applyWindowEffectPref(String(sel.value || "").trim().toLowerCase(), { persist: true });
  });

  applyWindowEffectPref(pref, { persist: false });
}

function initThemeUi() {
  const sel = $("themeSelect");
  if (!sel) return;

  let pref = String(document.documentElement.dataset.themePref || "").trim();
  if (!isValidThemePref(pref)) {
    try {
      const raw = localStorage.getItem(THEME_PREF_KEY);
      pref = String(raw || "").trim();
    } catch {}
  }
  if (!isValidThemePref(pref)) pref = "light";

  sel.value = pref;
  sel.addEventListener("change", () => {
    applyThemePref(String(sel.value || "").trim(), { persist: true });
  });
  applyThemePref(pref, { persist: false });
}

function syncSettingsWorkspaceUi() {
  const themeMain = $("themeSelect");
  const themeSettings = $("settingsThemeSelect");
  if (themeMain && themeSettings && String(themeSettings.value || "") !== String(themeMain.value || "")) {
    themeSettings.value = String(themeMain.value || "");
  }

  const effectMain = $("effectSelect");
  const effectSettings = $("settingsEffectSelect");
  if (effectMain && effectSettings) {
    const byValue = {};
    for (const opt of Array.from(effectMain.options || [])) byValue[String(opt.value || "").trim().toLowerCase()] = opt;
    for (const opt of Array.from(effectSettings.options || [])) {
      const key = String(opt.value || "").trim().toLowerCase();
      const src = byValue[key];
      opt.disabled = !!src?.disabled;
    }
    if (String(effectSettings.value || "") !== String(effectMain.value || "")) {
      effectSettings.value = String(effectMain.value || "");
    }
  }

  const marketDataMain = $("marketDataDir");
  const marketDataSettings = $("settingsMarketDataDir");
  if (
    marketDataMain &&
    marketDataSettings &&
    document.activeElement !== marketDataSettings &&
    String(marketDataSettings.value || "") !== String(marketDataMain.value || "")
  ) {
    marketDataSettings.value = String(marketDataMain.value || "");
  }

  const moveMain = $("marketDataMoveExisting");
  const moveSettings = $("settingsMoveExisting");
  if (moveMain && moveSettings) moveSettings.checked = !!moveMain.checked;

  const hintMain = $("marketDataDirHint");
  const hintSettings = $("settingsMarketDataDirHint");
  if (hintSettings) hintSettings.textContent = String(hintMain?.textContent || "—").trim() || "—";

  const wfAdv = $("settingsWorkflowAdvanced");
  if (wfAdv) wfAdv.checked = String(document.documentElement.dataset.workflowAdvanced || "").trim() === "1";

  const collapsed = $("settingsControlsCollapsed");
  if (collapsed) collapsed.checked = isControlsCollapsed();

  const running = !!RUNNING;
  for (const id of [
    "settingsThemeSelect",
    "settingsEffectSelect",
    "settingsWorkflowAdvanced",
    "settingsControlsCollapsed",
    "settingsMarketDataDir",
    "settingsMoveExisting",
    "btnSettingsBrowseMarketDataDir",
    "btnSettingsSetMarketDataDir",
    "btnSettingsResetMarketDataDir",
    "btnSettingsOpenMarketDataDir",
    "btnSettingsOpenWorkflowData",
    "btnSettingsOpenRunsDir",
    "btnSettingsOpenReportsDir",
    "btnSettingsOpenPolygonModal",
    "btnSettingsGotoLibrary",
    "btnSettingsGotoAggregate",
  ]) {
    const el = $(id);
    if (!el) continue;
    if (id === "btnSettingsGotoLibrary" || id === "btnSettingsGotoAggregate") {
      el.disabled = false;
    } else {
      el.disabled = running;
    }
  }
}

function initSettingsWorkspaceUi() {
  if (!$("settingsWorkspaceCard")) return;

  $("settingsThemeSelect")?.addEventListener("change", () => {
    const v = String($("settingsThemeSelect")?.value || "").trim();
    applyThemePref(v, { persist: true });
  });

  $("settingsEffectSelect")?.addEventListener("change", () => {
    const v = String($("settingsEffectSelect")?.value || "").trim().toLowerCase();
    applyWindowEffectPref(v, { persist: true });
  });

  $("settingsWorkflowAdvanced")?.addEventListener("change", () => {
    setWorkflowAdvancedEnabled(!!$("settingsWorkflowAdvanced")?.checked, { persist: true });
  });

  $("settingsControlsCollapsed")?.addEventListener("change", () => {
    setControlsCollapsed(!!$("settingsControlsCollapsed")?.checked, { persist: true });
  });

  $("settingsMarketDataDir")?.addEventListener("input", () => {
    const target = $("marketDataDir");
    if (!target) return;
    target.value = String($("settingsMarketDataDir")?.value || "");
  });

  $("settingsMoveExisting")?.addEventListener("change", () => {
    const target = $("marketDataMoveExisting");
    if (!target) return;
    target.checked = !!$("settingsMoveExisting")?.checked;
  });

  $("btnSettingsBrowseMarketDataDir")?.addEventListener("click", () => {
    const src = $("btnBrowseMarketDataDir");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsSetMarketDataDir")?.addEventListener("click", () => {
    const target = $("marketDataDir");
    if (target) target.value = String($("settingsMarketDataDir")?.value || "").trim();
    const move = $("marketDataMoveExisting");
    if (move) move.checked = !!$("settingsMoveExisting")?.checked;
    const src = $("btnSetMarketDataDir");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsResetMarketDataDir")?.addEventListener("click", () => {
    const src = $("btnResetMarketDataDir");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsOpenMarketDataDir")?.addEventListener("click", () => {
    const src = $("btnOpenMarketDataDir");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsOpenWorkflowData")?.addEventListener("click", () => {
    setWorkspace("workflow", { save: true, focus: false, step: "data" });
    const details = $("dataStorageOptions");
    if (details && typeof details.open === "boolean") details.open = true;
  });

  $("btnSettingsOpenRunsDir")?.addEventListener("click", () => {
    const src = $("btnOpenRunsDir");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsOpenReportsDir")?.addEventListener("click", () => {
    const src = $("btnOpenReportsDir");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsOpenPolygonModal")?.addEventListener("click", () => {
    const src = $("btnPolygonOpen");
    if (src && !src.disabled) src.click();
  });

  $("btnSettingsGotoLibrary")?.addEventListener("click", () => {
    setWorkspace("library", { save: true, focus: true });
  });

  $("btnSettingsGotoAggregate")?.addEventListener("click", () => {
    setWorkspace("aggregate", { save: true, focus: true });
    setAggregateTab("meta", { persist: true });
  });

  $("themeSelect")?.addEventListener("change", syncSettingsWorkspaceUi);
  $("effectSelect")?.addEventListener("change", syncSettingsWorkspaceUi);
  $("marketDataDir")?.addEventListener("input", syncSettingsWorkspaceUi);
  $("marketDataDir")?.addEventListener("change", syncSettingsWorkspaceUi);
  $("marketDataMoveExisting")?.addEventListener("change", syncSettingsWorkspaceUi);

  syncSettingsWorkspaceUi();
}

function initTabSystem({ barId, panelsId, storageKey, defaultTab }) {
  const bar = $(barId);
  const panelsRoot = $(panelsId);
  if (!bar || !panelsRoot) return null;

  const buttons = Array.from(bar.querySelectorAll("button[data-tab]"));
  const panels = Array.from(panelsRoot.querySelectorAll("[data-tab-panel]"));

  const tabNames = [];
  const btnByTab = {};
  for (const btn of buttons) {
    const t = String(btn.dataset.tab || "").trim();
    if (!t) continue;
    tabNames.push(t);
    btnByTab[t] = btn;
  }

  const panelByTab = {};
  for (const p of panels) {
    const t = String(p.dataset.tabPanel || "").trim();
    if (!t) continue;
    panelByTab[t] = p;
  }

  bar.setAttribute("role", "tablist");

  for (const t of tabNames) {
    const btn = btnByTab[t];
    const panel = panelByTab[t];
    if (!btn || !panel) continue;

    if (!btn.id) btn.id = `${barId}__tab__${t}`;
    if (!panel.id) panel.id = `${panelsId}__panel__${t}`;

    btn.setAttribute("role", "tab");
    btn.setAttribute("aria-controls", panel.id);
    panel.setAttribute("role", "tabpanel");
    panel.setAttribute("aria-labelledby", btn.id);
  }

  const setActive = (tab, opts) => {
    const t = String(tab || "").trim();
    const save = opts?.save !== false;
    const focus = !!opts?.focus;
    const animate = opts?.animate === true;
    if (!t || !btnByTab[t] || !panelByTab[t]) return;

    runUiTransition(() => {
      for (const name of tabNames) {
        const b = btnByTab[name];
        if (!b) continue;
        const isActive = name === t;
        b.classList.toggle("active", isActive);
        b.setAttribute("aria-selected", isActive ? "true" : "false");
        b.tabIndex = isActive ? 0 : -1;
      }
      for (const [name, p] of Object.entries(panelByTab)) {
        const isActive = name === t;
        p.classList.toggle("active", isActive);
        p.setAttribute("aria-hidden", isActive ? "false" : "true");
      }
    }, { animate });

    if (save) {
      try { localStorage.setItem(storageKey, t); } catch {}
    }
    if (focus) {
      try { btnByTab[t].focus(); } catch {}
    }
  };

  const getActive = () => {
    const cur = bar.querySelector("button[data-tab].active");
    const t = String(cur?.dataset?.tab || "").trim();
    return t || "";
  };

  for (const t of tabNames) {
    const btn = btnByTab[t];
    if (!btn) continue;
    btn.addEventListener("click", () => setActive(t, { save: true, focus: false, animate: true }));
    btn.addEventListener("keydown", (e) => {
      const key = e.key;
      if (key !== "ArrowLeft" && key !== "ArrowRight" && key !== "Home" && key !== "End") return;
      e.preventDefault();

      const idx = tabNames.indexOf(t);
      if (idx < 0) return;
      let next = idx;
      if (key === "ArrowLeft") next = (idx - 1 + tabNames.length) % tabNames.length;
      else if (key === "ArrowRight") next = (idx + 1) % tabNames.length;
      else if (key === "Home") next = 0;
      else if (key === "End") next = tabNames.length - 1;

      const nt = tabNames[next];
      setActive(nt, { save: true, focus: true, animate: true });
    });
  }

  let initial = defaultTab;
  try {
    const saved = String(localStorage.getItem(storageKey) || "").trim();
    if (saved && btnByTab[saved] && panelByTab[saved]) initial = saved;
  } catch {}
  if (!initial || !btnByTab[initial] || !panelByTab[initial]) initial = tabNames[0] || "";
  setActive(initial, { save: false, focus: false });

  return { set: setActive, get: getActive, tabs: tabNames };
}

const WORKFLOW_TABS = ["data", "optimize", "validate", "analyze", "history", "settings"];
const WORKFLOW_TAB_SET = new Set(WORKFLOW_TABS);
const WORKFLOW_DEFAULTS = {
  data: { controls: "data", results: "overview" },
  optimize: { controls: "opt", results: "overview" },
  validate: { controls: "opt", results: "validate" },
  analyze: { controls: "opt", results: "drill" },
  history: { controls: "data", results: "history" },
  settings: { controls: "advanced", results: "meta" },
};
const WORKFLOW_ALLOWED = {
  data: { controls: ["data"], results: ["overview"] },
  optimize: { controls: ["opt", "strategy", "advanced"], results: ["overview", "top", "pareto", "heatmap"] },
  validate: { controls: ["opt"], results: ["validate"] },
  analyze: { controls: ["opt"], results: ["overview", "top", "pareto", "drill", "heatmap"] },
  history: { controls: ["data"], results: ["history"] },
  settings: { controls: ["advanced", "distributed"], results: ["meta", "debug"] },
};
const WORKFLOW_SUBTITLES = {
  data: "Load a dataset (CSV or Polygon) and select the optimization timeframe.",
  optimize: "Configure strategy + search space, then run an optimization.",
  validate: "Run robustness checks on the current run’s candidates.",
  analyze: "Inspect results via drilldown curves and heatmaps.",
  history: "Review, compare, and rerun saved optimizations.",
  settings: "Advanced tools (distributed runs, meta selection, debug).",
};

const WORKSPACE_TABS = ["workflow", "library", "aggregate", "pro", "settings"];
const WORKSPACE_TAB_SET = new Set(WORKSPACE_TABS);
const WORKFLOW_STEPS = ["data", "optimize", "validate", "analyze"];
const WORKFLOW_STEP_SET = new Set(WORKFLOW_STEPS);
const AGGREGATE_TABS = ["meta", "batch", "saved", "compare"];
const AGGREGATE_TAB_SET = new Set(AGGREGATE_TABS);
const WORKFLOW_STEP_SUBTITLES = {
  data: "Data source: load CSV or Polygon candles, then confirm timeframe.",
  optimize: "Strategy setup: configure ranges, backend, and run optimization.",
  validate: "Validation: run robustness checks on the selected optimization.",
  analyze: "Summary: inspect top results, Pareto, drilldown, and heatmap.",
};
const WORKSPACE_SUBTITLES = {
  workflow: "Guided pipeline for data -> optimize -> validate -> summary.",
  library: "Browse, compare, and rerun saved optimization artifacts.",
  aggregate: "Aggregate many runs into robust shortlist recommendations.",
  pro: "Distributed compute, advanced controls, and debug output.",
  settings: "Application preferences and storage configuration.",
};

const TAB_LABELS = {
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

function normalizeWorkflowTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return WORKFLOW_TAB_SET.has(x) ? x : "";
}

function normalizeWorkspaceTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return WORKSPACE_TAB_SET.has(x) ? x : "";
}

function normalizeWorkflowStep(v) {
  const x = String(v || "").trim().toLowerCase();
  return WORKFLOW_STEP_SET.has(x) ? x : "";
}

function normalizeAggregateTab(v) {
  const x = String(v || "").trim().toLowerCase();
  return AGGREGATE_TAB_SET.has(x) ? x : "";
}

function workflowStepFromWorkflow(workflow) {
  const wf = normalizeWorkflowTab(workflow);
  return normalizeWorkflowStep(wf) || "data";
}

function workspaceFromWorkflowState(workflow) {
  const wf = normalizeWorkflowTab(workflow) || "data";
  if (wf === "history") return "library";
  if (wf === "settings") {
    const c = String(CONTROLS_TABS?.get?.() || "").trim();
    const r = String(RESULTS_TABS?.get?.() || "").trim();
    if (c === "distributed" || r === "debug") return "pro";
    return "aggregate";
  }
  if (WORKSPACE_OVERRIDE === "settings" && wf === "data") return "settings";
  return "workflow";
}

function loadWorkflowState() {
  try {
    const raw = localStorage.getItem(WORKFLOW_STATE_KEY);
    if (!raw) return {};
    const obj = JSON.parse(raw);
    if (!obj || typeof obj !== "object") return {};

    const out = {};
    for (const [k, v] of Object.entries(obj)) {
      const wf = normalizeWorkflowTab(k);
      if (!wf || !v || typeof v !== "object") continue;
      const controls = String(v.controls || "").trim();
      const results = String(v.results || "").trim();
      out[wf] = { controls, results };
    }
    return out;
  } catch {
    return {};
  }
}

function scheduleSaveWorkflowState() {
  WORKFLOW_STATE_SAVE_SEQ = (WORKFLOW_STATE_SAVE_SEQ + 1) >>> 0;
  const seq = WORKFLOW_STATE_SAVE_SEQ;
  setTimeout(() => {
    if (seq !== WORKFLOW_STATE_SAVE_SEQ) return;
    try {
      localStorage.setItem(WORKFLOW_STATE_KEY, JSON.stringify(WORKFLOW_STATE));
    } catch {}
  }, 80);
}

function recordWorkflowState(workflow) {
  const wf = normalizeWorkflowTab(workflow);
  if (!wf) return;
  WORKFLOW_STATE[wf] = {
    controls: CONTROLS_TABS?.get?.() || "",
    results: RESULTS_TABS?.get?.() || "",
  };
  scheduleSaveWorkflowState();
}

function inferWorkflowFromTabs(controlsTab, resultsTab) {
  const c = String(controlsTab || "").trim();
  const r = String(resultsTab || "").trim();

  if (r === "validate") return "validate";
  if (r === "history") return "history";
  if (r === "meta" || r === "debug") return "settings";
  if (r === "drill") return "analyze";
  if (c === "data" && (!r || r === "overview")) return "data";
  return "optimize";
}

function updateWorkflowBarUi(workflow) {
  const wf = normalizeWorkflowTab(workflow);
  if (!wf || !WORKFLOW_UI) return;

  const { tabs, btnBy } = WORKFLOW_UI;
  for (const t of tabs) {
    const btn = btnBy[t];
    if (!btn) continue;
    const isActive = t === wf;
    btn.classList.toggle("active", isActive);
    btn.setAttribute("aria-selected", isActive ? "true" : "false");
    btn.tabIndex = isActive ? 0 : -1;
  }
}

function updateWorkspaceBarUi(workspace) {
  const ws = normalizeWorkspaceTab(workspace);
  if (!ws || !WORKSPACE_NAV_UI) return;
  const { tabs, btnBy } = WORKSPACE_NAV_UI;
  for (const t of tabs) {
    const btn = btnBy[t];
    if (!btn) continue;
    const isActive = t === ws;
    btn.classList.toggle("active", isActive);
    btn.setAttribute("aria-selected", isActive ? "true" : "false");
    btn.tabIndex = isActive ? 0 : -1;
  }
}

function updateWorkflowStepBarUi(step) {
  const st = normalizeWorkflowStep(step) || "data";
  if (!WORKFLOW_STEP_NAV_UI) return;
  const { tabs, btnBy } = WORKFLOW_STEP_NAV_UI;
  for (const t of tabs) {
    const btn = btnBy[t];
    if (!btn) continue;
    const isActive = t === st;
    btn.classList.toggle("active", isActive);
    btn.setAttribute("aria-selected", isActive ? "true" : "false");
    btn.tabIndex = isActive ? 0 : -1;
  }
}

function updateWorkspaceContextStrip() {
  const datasetEl = $("workspaceCtxDataset");
  if (!datasetEl) return;

  const timeframeEl = $("workspaceCtxTimeframe");
  const strategyEl = $("workspaceCtxStrategy");
  const backendEl = $("workspaceCtxBackend");
  const statusEl = $("workspaceCtxStatus");

  const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};
  const meta = CURRENT_DATA_META && typeof CURRENT_DATA_META === "object" ? CURRENT_DATA_META : null;
  const csvPath = String($("csvPath")?.value || "").trim();

  const dataset =
    String(ctx.label || "").trim() ||
    String(ctx.symbol || "").trim() ||
    (csvPath ? (fileBasename(csvPath) || csvPath) : "");
  datasetEl.textContent = dataset || "—";

  const tf = String(ctx.timeframe || meta?.timeframe || "").trim();
  if (timeframeEl) timeframeEl.textContent = tf || "—";

  const kind = CURRENT_RUN?.req ? kindFromReq(CURRENT_RUN.req) : strategyKind();
  const kindLabel = strategyKindLabel(kind);
  if (strategyEl) strategyEl.textContent = kindLabel || "—";

  const backendFromRun = String(CURRENT_RUN?.res?.backend_used || "").trim();
  const backendFromReq = String(CURRENT_RUN?.req?.backend?.backend || "").trim();
  const backendFromUi = String($("backend")?.value || "").trim();
  const backend = isDoubleLikeKind(kind)
    ? (backendFromRun || backendFromReq || backendFromUi || "CpuOnly")
    : "CpuOnly";
  if (backendEl) backendEl.textContent = backend || "—";

  if (statusEl) {
    const statusText = String($("status")?.textContent || "").trim();
    statusEl.textContent = statusText || "Ready.";
  }
}

function updateWorkflowAdvancedToggleUi() {
  const btn = $("btnToggleWorkflowAdvanced");
  if (!btn) return;
  const enabled = String(document.documentElement.dataset.workflowAdvanced || "").trim() === "1";
  btn.setAttribute("aria-pressed", enabled ? "true" : "false");
  btn.textContent = enabled ? "Hide advanced controls" : "Show advanced controls";
}

function setWorkflowAdvancedEnabled(enabled, opts) {
  const on = !!enabled;
  const persist = opts?.persist !== false;
  document.documentElement.dataset.workflowAdvanced = on ? "1" : "";
  if (persist) {
    try {
      if (on) localStorage.setItem(WORKFLOW_ADVANCED_KEY, "1");
      else localStorage.removeItem(WORKFLOW_ADVANCED_KEY);
    } catch {}
  }
  updateWorkflowAdvancedToggleUi();

  if (!on && normalizeWorkspaceTab(CURRENT_WORKSPACE) === "workflow") {
    const cur = String(CONTROLS_TABS?.get?.() || "").trim();
    if (cur === "strategy" || cur === "advanced") CONTROLS_TABS?.set("opt", { save: true, focus: false });
  }
  syncSettingsWorkspaceUi();
}

function initWorkflowAdvancedToggle() {
  let on = false;
  try { on = String(localStorage.getItem(WORKFLOW_ADVANCED_KEY) || "").trim() === "1"; } catch {}
  setWorkflowAdvancedEnabled(on, { persist: false });
  $("btnToggleWorkflowAdvanced")?.addEventListener("click", () => {
    const cur = String(document.documentElement.dataset.workflowAdvanced || "").trim() === "1";
    setWorkflowAdvancedEnabled(!cur, { persist: true });
  });
}

function setAggregateTab(tab, opts) {
  const t = normalizeAggregateTab(tab) || "meta";
  const persist = opts?.persist !== false;
  const bar = $("aggregateTabBar");
  const buttons = bar ? Array.from(bar.querySelectorAll("button[data-aggregate-tab]")) : [];

  CURRENT_AGGREGATE_TAB = t;
  document.documentElement.dataset.aggregateTab = t;

  for (const btn of buttons) {
    const bt = normalizeAggregateTab(btn.dataset.aggregateTab);
    const active = bt === t;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-selected", active ? "true" : "false");
    btn.tabIndex = active ? 0 : -1;
  }

  const batchWrap = $("batchWrap");
  const metaSection = $("aggregateMetaSection");
  const savedWrap = $("metaSavedWrap");
  const compareWrap = $("metaCompareWrap");

  if (batchWrap) {
    batchWrap.style.display = t === "batch" ? "" : "none";
    if (t === "batch") batchWrap.open = true;
  }

  if (metaSection) {
    if (t === "batch") {
      metaSection.style.display = "none";
    } else {
      metaSection.style.display = "";
      const children = Array.from(metaSection.children || []);
      for (const child of children) {
        if (child === savedWrap || child === compareWrap) continue;
        child.style.display = t === "meta" ? "" : "none";
      }
    }
  }

  if (savedWrap) {
    savedWrap.style.display = t === "saved" ? "" : "none";
    if (t === "saved") savedWrap.open = true;
  }
  if (compareWrap) {
    compareWrap.style.display = t === "compare" ? "" : "none";
    if (t === "compare") compareWrap.open = true;
  }

  if (persist) {
    try { localStorage.setItem(AGGREGATE_TAB_KEY, t); } catch {}
  }
}

function initAggregateTabUi() {
  const bar = $("aggregateTabBar");
  if (!bar) return;
  const buttons = Array.from(bar.querySelectorAll("button[data-aggregate-tab]"));
  const tabs = [];
  const btnBy = {};
  for (const btn of buttons) {
    const t = normalizeAggregateTab(btn.dataset.aggregateTab);
    if (!t) continue;
    tabs.push(t);
    btnBy[t] = btn;
  }
  if (!tabs.length) return;

  bar.setAttribute("role", "tablist");
  for (const t of tabs) {
    const btn = btnBy[t];
    if (!btn) continue;
    btn.setAttribute("role", "tab");
    btn.addEventListener("click", () => setAggregateTab(t, { persist: true }));
    btn.addEventListener("keydown", (e) => {
      const key = e.key;
      if (key !== "ArrowLeft" && key !== "ArrowRight" && key !== "Home" && key !== "End") return;
      e.preventDefault();
      const idx = tabs.indexOf(t);
      if (idx < 0) return;
      let next = idx;
      if (key === "ArrowLeft") next = (idx - 1 + tabs.length) % tabs.length;
      else if (key === "ArrowRight") next = (idx + 1) % tabs.length;
      else if (key === "Home") next = 0;
      else if (key === "End") next = tabs.length - 1;
      const nt = tabs[next];
      setAggregateTab(nt, { persist: true });
      try { btnBy[nt]?.focus(); } catch {}
    });
  }

  let saved = "";
  try { saved = normalizeAggregateTab(localStorage.getItem(AGGREGATE_TAB_KEY)); } catch {}
  setAggregateTab(saved || "meta", { persist: false });
}

function syncWorkspaceFromWorkflow(opts) {
  if (WORKSPACE_SYNC_SUPPRESS) return;

  const save = opts?.save !== false;
  const wf = normalizeWorkflowTab(CURRENT_WORKFLOW) || "data";
  const step = workflowStepFromWorkflow(wf);
  const ws = workspaceFromWorkflowState(wf);

  CURRENT_WORKFLOW_STEP = step;
  CURRENT_WORKSPACE = ws;
  document.documentElement.dataset.workspace = ws;

  updateWorkspaceBarUi(ws);
  updateWorkflowStepBarUi(step);

  if (save) {
    try { localStorage.setItem(WORKSPACE_TAB_KEY, ws); } catch {}
    if (ws === "workflow") {
      try { localStorage.setItem(WORKFLOW_STEP_KEY, step); } catch {}
    }
  }

  if (!(ws === "settings" && wf === "data")) WORKSPACE_OVERRIDE = "";
  updateWorkflowSubtitle(wf);
  updateWorkspaceContextStrip();
  if (ws === "aggregate") setAggregateTab(CURRENT_AGGREGATE_TAB, { persist: false });
  updateWorkflowAdvancedToggleUi();
  syncSettingsWorkspaceUi();
}

function setWorkspace(workspace, opts) {
  const ws = normalizeWorkspaceTab(workspace) || "workflow";
  const save = opts?.save !== false;
  const focus = !!opts?.focus;
  let step = normalizeWorkflowStep(opts?.step);

  if (!step) {
    if (ws === "workflow") {
      step = normalizeWorkflowStep(CURRENT_WORKFLOW_STEP) || workflowStepFromWorkflow(CURRENT_WORKFLOW);
      if (!step) {
        try { step = normalizeWorkflowStep(localStorage.getItem(WORKFLOW_STEP_KEY)); } catch {}
      }
    } else {
      step = workflowStepFromWorkflow(CURRENT_WORKFLOW);
    }
  }
  if (!step) step = "data";

  WORKSPACE_SYNC_SUPPRESS = true;
  try {
    if (ws === "workflow") {
      WORKSPACE_OVERRIDE = "";
      applyWorkflow(step, { save, focus: false, setTabs: true });
    } else if (ws === "library") {
      WORKSPACE_OVERRIDE = "";
      applyWorkflow("history", { save, focus: false, setTabs: true });
    } else if (ws === "aggregate") {
      WORKSPACE_OVERRIDE = "";
      applyWorkflow("settings", { save, focus: false, setTabs: true });
      CONTROLS_TABS?.set("advanced", { save, focus: false });
      RESULTS_TABS?.set("meta", { save, focus: false });
    } else if (ws === "pro") {
      WORKSPACE_OVERRIDE = "";
      applyWorkflow("settings", { save, focus: false, setTabs: true });
      CONTROLS_TABS?.set("distributed", { save, focus: false });
      RESULTS_TABS?.set("debug", { save, focus: false });
    } else if (ws === "settings") {
      WORKSPACE_OVERRIDE = "settings";
      applyWorkflow("data", { save, focus: false, setTabs: true });
      CONTROLS_TABS?.set("data", { save, focus: false });
      RESULTS_TABS?.set("overview", { save, focus: false });
      const details = $("dataStorageOptions");
      if (details && typeof details.open === "boolean") details.open = true;
    }
  } finally {
    WORKSPACE_SYNC_SUPPRESS = false;
  }

  CURRENT_WORKFLOW_STEP = step;
  CURRENT_WORKSPACE = ws;
  document.documentElement.dataset.workspace = ws;
  updateWorkspaceBarUi(ws);
  updateWorkflowStepBarUi(step);

  if (save) {
    try { localStorage.setItem(WORKSPACE_TAB_KEY, ws); } catch {}
    if (ws === "workflow") {
      try { localStorage.setItem(WORKFLOW_STEP_KEY, step); } catch {}
    }
  }

  updateWorkflowSubtitle(CURRENT_WORKFLOW || "data");
  updateWorkspaceContextStrip();
  if (ws === "aggregate") setAggregateTab(CURRENT_AGGREGATE_TAB, { persist: false });
  updateWorkflowAdvancedToggleUi();
  syncSettingsWorkspaceUi();

  if (focus && WORKSPACE_NAV_UI?.btnBy?.[ws]) {
    try { WORKSPACE_NAV_UI.btnBy[ws].focus(); } catch {}
  }
}

function updateWorkflowSubtitle(workflow) {
  const el = $("workflowSubtitle");
  if (!el) return;

  const wf = normalizeWorkflowTab(workflow || CURRENT_WORKFLOW) || "data";
  const ws = normalizeWorkspaceTab(CURRENT_WORKSPACE) || workspaceFromWorkflowState(wf);
  const step = workflowStepFromWorkflow(wf);

  let base = "";
  if (ws === "workflow") base = String(WORKFLOW_STEP_SUBTITLES[step] || WORKFLOW_SUBTITLES[wf] || "").trim();
  else base = String(WORKSPACE_SUBTITLES[ws] || "").trim();

  const c = String(CONTROLS_TABS?.get?.() || "").trim();
  const r = String(RESULTS_TABS?.get?.() || "").trim();

  let view = "";
  if (ws === "workflow" && wf === "optimize") {
    const cLabel = TAB_LABELS.controls?.[c] || "";
    if (cLabel && c !== "opt") view = cLabel;
    const rLabel = TAB_LABELS.results?.[r] || "";
    if (rLabel && r !== "overview") view = view ? `${view} • ${rLabel}` : rLabel;
  } else if (ws === "workflow" && wf === "analyze") {
    const rLabel = TAB_LABELS.results?.[r] || "";
    if (rLabel) view = rLabel;
  } else if (ws === "aggregate" || ws === "pro") {
    const cLabel = TAB_LABELS.controls?.[c] || "";
    const rLabel = TAB_LABELS.results?.[r] || "";
    if (cLabel && ws === "pro") view = cLabel;
    if (rLabel && (ws === "pro" || r !== "meta")) view = view ? `${view} • ${rLabel}` : rLabel;
  } else if (ws === "settings") {
    view = "Appearance + storage";
  }

  el.textContent = base ? (view ? `${base} (${view})` : base) : (view || "—");
}

function applyWorkflow(workflow, opts) {
  const wf = normalizeWorkflowTab(workflow) || "data";
  const save = opts?.save !== false;
  const focus = !!opts?.focus;
  const setTabs = opts?.setTabs !== false;

  if (wf !== CURRENT_WORKFLOW && CURRENT_WORKFLOW) recordWorkflowState(CURRENT_WORKFLOW);

  CURRENT_WORKFLOW = wf;
  document.documentElement.dataset.workflow = wf;
  updateWorkflowBarUi(wf);

  if (save) {
    try {
      localStorage.setItem(WORKFLOW_TAB_KEY, wf);
    } catch {}
  }

  if (setTabs) {
    const st = WORKFLOW_STATE?.[wf] || {};
    const d = WORKFLOW_DEFAULTS[wf] || {};
    const allowed = WORKFLOW_ALLOWED[wf] || {};
    const allowedControls = allowed.controls || [];
    const allowedResults = allowed.results || [];

    const desiredControls = String(st.controls || d.controls || "").trim();
    const desiredResults = String(st.results || d.results || "").trim();

    const canSetControls = !!CONTROLS_TABS && typeof CONTROLS_TABS.set === "function";
    const canSetResults = !!RESULTS_TABS && typeof RESULTS_TABS.set === "function";

    WORKFLOW_SYNC_SUPPRESS = true;
    try {
      if (canSetControls) {
        const fallback = allowedControls.includes(d.controls) ? d.controls : allowedControls[0];
        const t0 = allowedControls.includes(desiredControls) ? desiredControls : fallback;
        const t = (CONTROLS_TABS.tabs || []).includes(t0) ? t0 : fallback;
        if (t) CONTROLS_TABS.set(t, { save: true, focus: false });
      }
      if (canSetResults) {
        const fallback = allowedResults.includes(d.results) ? d.results : allowedResults[0];
        const t0 = allowedResults.includes(desiredResults) ? desiredResults : fallback;
        const t = (RESULTS_TABS.tabs || []).includes(t0) ? t0 : fallback;
        if (t) RESULTS_TABS.set(t, { save: true, focus: false });
      }
    } finally {
      WORKFLOW_SYNC_SUPPRESS = false;
    }

    recordWorkflowState(wf);
  }

  if (!WORKSPACE_SYNC_SUPPRESS) syncWorkspaceFromWorkflow({ save });
  else {
    CURRENT_WORKFLOW_STEP = workflowStepFromWorkflow(wf);
    updateWorkflowStepBarUi(CURRENT_WORKFLOW_STEP);
    updateWorkflowSubtitle(wf);
    updateWorkspaceContextStrip();
  }

  if (focus && WORKFLOW_UI?.btnBy?.[wf]) {
    try {
      WORKFLOW_UI.btnBy[wf].focus();
    } catch {}
  }
}

function syncWorkflowToActiveTabs() {
  if (WORKFLOW_SYNC_SUPPRESS) return;
  const c = CONTROLS_TABS?.get?.() || "";
  const r = RESULTS_TABS?.get?.() || "";
  let wf = CURRENT_WORKFLOW || "";
  if (r === "validate") wf = "validate";
  else if (r === "history") wf = "history";
  else if (r === "meta" || r === "debug") wf = "settings";
  else if (r === "drill") wf = "analyze";
  else if (String(c).trim() === "data") wf = "data";
  else if (CURRENT_WORKFLOW === "data") wf = "optimize";
  else if (!wf) wf = inferWorkflowFromTabs(c, r);

  if (wf && wf !== CURRENT_WORKFLOW) applyWorkflow(wf, { save: true, focus: false, setTabs: false });
  syncWorkspaceFromWorkflow({ save: true });
  recordWorkflowState(CURRENT_WORKFLOW);
}

function wrapTabSet(tabSystem) {
  if (!tabSystem || typeof tabSystem.set !== "function") return;
  if (tabSystem.__vtaWorkflowWrapped) return;
  const orig = tabSystem.set;
  tabSystem.set = (tab, opts) => {
    orig(tab, opts);
    syncWorkflowToActiveTabs();
  };
  tabSystem.__vtaWorkflowWrapped = true;
}

function initWorkflowNav() {
  WORKFLOW_STATE = loadWorkflowState();

  const bar = $("workflowTabBar");
  if (!bar) return;
  const buttons = Array.from(bar.querySelectorAll("button[data-workflow]"));

  const tabs = [];
  const btnBy = {};
  for (const btn of buttons) {
    const wf = normalizeWorkflowTab(btn.dataset.workflow);
    if (!wf) continue;
    tabs.push(wf);
    btnBy[wf] = btn;
  }

  if (!tabs.length) return;
  WORKFLOW_UI = { bar, tabs, btnBy };

  bar.setAttribute("role", "tablist");
  for (const wf of tabs) {
    const btn = btnBy[wf];
    if (!btn) continue;
    btn.setAttribute("role", "tab");
    btn.addEventListener("click", () => applyWorkflow(wf, { save: true, focus: false, setTabs: true }));
    btn.addEventListener("keydown", (e) => {
      const key = e.key;
      if (key !== "ArrowLeft" && key !== "ArrowRight" && key !== "Home" && key !== "End") return;
      e.preventDefault();
      const idx = tabs.indexOf(wf);
      if (idx < 0) return;

      let next = idx;
      if (key === "ArrowLeft") next = (idx - 1 + tabs.length) % tabs.length;
      else if (key === "ArrowRight") next = (idx + 1) % tabs.length;
      else if (key === "Home") next = 0;
      else if (key === "End") next = tabs.length - 1;

      const nwf = tabs[next];
      applyWorkflow(nwf, { save: true, focus: true, setTabs: true });
    });
  }

  wrapTabSet(CONTROLS_TABS);
  wrapTabSet(RESULTS_TABS);

  let saved = "";
  try {
    saved = normalizeWorkflowTab(localStorage.getItem(WORKFLOW_TAB_KEY));
  } catch {}
  const preset = normalizeWorkflowTab(document.documentElement.dataset.workflow);
  const inferred = normalizeWorkflowTab(inferWorkflowFromTabs(CONTROLS_TABS?.get?.() || "", RESULTS_TABS?.get?.() || ""));

  if (saved) applyWorkflow(saved, { save: false, focus: false, setTabs: true });
  else if (preset) applyWorkflow(preset, { save: false, focus: false, setTabs: false });
  else if (inferred) applyWorkflow(inferred, { save: false, focus: false, setTabs: false });
  else applyWorkflow("data", { save: false, focus: false, setTabs: false });
}

function initWorkspaceNav() {
  const workspaceBar = $("workspaceBar");
  const workspaceButtons = workspaceBar ? Array.from(workspaceBar.querySelectorAll("button[data-workspace]")) : [];
  const wsTabs = [];
  const wsBtnBy = {};
  for (const btn of workspaceButtons) {
    const ws = normalizeWorkspaceTab(btn.dataset.workspace);
    if (!ws) continue;
    wsTabs.push(ws);
    wsBtnBy[ws] = btn;
  }
  if (workspaceBar && wsTabs.length) {
    WORKSPACE_NAV_UI = { bar: workspaceBar, tabs: wsTabs, btnBy: wsBtnBy };
    workspaceBar.setAttribute("role", "tablist");
    for (const ws of wsTabs) {
      const btn = wsBtnBy[ws];
      if (!btn) continue;
      btn.setAttribute("role", "tab");
      btn.addEventListener("click", () => setWorkspace(ws, { save: true, focus: false }));
      btn.addEventListener("keydown", (e) => {
        const key = e.key;
        if (key !== "ArrowLeft" && key !== "ArrowRight" && key !== "Home" && key !== "End") return;
        e.preventDefault();
        const idx = wsTabs.indexOf(ws);
        if (idx < 0) return;
        let next = idx;
        if (key === "ArrowLeft") next = (idx - 1 + wsTabs.length) % wsTabs.length;
        else if (key === "ArrowRight") next = (idx + 1) % wsTabs.length;
        else if (key === "Home") next = 0;
        else if (key === "End") next = wsTabs.length - 1;
        const nws = wsTabs[next];
        setWorkspace(nws, { save: true, focus: true });
      });
    }
  }

  const stepBar = $("workflowStepBar");
  const stepButtons = stepBar ? Array.from(stepBar.querySelectorAll("button[data-workflow-step]")) : [];
  const stepTabs = [];
  const stepBtnBy = {};
  for (const btn of stepButtons) {
    const st = normalizeWorkflowStep(btn.dataset.workflowStep);
    if (!st) continue;
    stepTabs.push(st);
    stepBtnBy[st] = btn;
  }
  if (stepBar && stepTabs.length) {
    WORKFLOW_STEP_NAV_UI = { bar: stepBar, tabs: stepTabs, btnBy: stepBtnBy };
    stepBar.setAttribute("role", "tablist");
    for (const st of stepTabs) {
      const btn = stepBtnBy[st];
      if (!btn) continue;
      btn.setAttribute("role", "tab");
      btn.addEventListener("click", () => setWorkspace("workflow", { save: true, focus: false, step: st }));
      btn.addEventListener("keydown", (e) => {
        const key = e.key;
        if (key !== "ArrowLeft" && key !== "ArrowRight" && key !== "Home" && key !== "End") return;
        e.preventDefault();
        const idx = stepTabs.indexOf(st);
        if (idx < 0) return;
        let next = idx;
        if (key === "ArrowLeft") next = (idx - 1 + stepTabs.length) % stepTabs.length;
        else if (key === "ArrowRight") next = (idx + 1) % stepTabs.length;
        else if (key === "Home") next = 0;
        else if (key === "End") next = stepTabs.length - 1;
        const nst = stepTabs[next];
        setWorkspace("workflow", { save: true, focus: true, step: nst });
      });
    }
  }

  let savedWs = "";
  let savedStep = "";
  try { savedWs = normalizeWorkspaceTab(localStorage.getItem(WORKSPACE_TAB_KEY)); } catch {}
  try { savedStep = normalizeWorkflowStep(localStorage.getItem(WORKFLOW_STEP_KEY)); } catch {}

  const wf = normalizeWorkflowTab(CURRENT_WORKFLOW) || "data";
  const inferredWs = workspaceFromWorkflowState(wf);
  const inferredStep = workflowStepFromWorkflow(wf);

  const initialWs = savedWs || inferredWs || "workflow";
  const initialStep = savedStep || inferredStep || "data";
  setWorkspace(initialWs, { save: false, focus: false, step: initialStep });
}

function readControlsWidthPx() {
  const raw = String(getComputedStyle(document.documentElement).getPropertyValue("--controls-w") || "").trim();
  const m = raw.match(/([0-9.]+)/);
  const x = m ? Number(m[1]) : NaN;
  return Number.isFinite(x) ? x : 420;
}

function isControlsCollapsed() {
  return String(document.documentElement.dataset.controlsCollapsed || "").trim() === "1";
}

function setControlsCollapsed(collapsed, opts) {
  const root = document.documentElement;
  const on = collapsed ? "1" : "";
  root.dataset.controlsCollapsed = on;
  const persist = opts?.persist !== false;
  if (persist) {
    try {
      if (on) localStorage.setItem(CONTROLS_COLLAPSED_KEY, "1");
      else localStorage.removeItem(CONTROLS_COLLAPSED_KEY);
    } catch {}
  }
  updateControlsToggleUi();
  syncSettingsWorkspaceUi();
}

function applyControlsWidth(px, opts) {
  const raw = Number(px);
  if (!Number.isFinite(raw)) return;
  const w = Math.max(CONTROLS_W_MIN, Math.min(CONTROLS_W_MAX, Math.round(raw)));
  document.documentElement.style.setProperty("--controls-w", `${w}px`);

  const persist = opts?.persist !== false;
  if (persist) {
    try { localStorage.setItem(CONTROLS_WIDTH_KEY, String(w)); } catch {}
  }
  updateControlsToggleUi();
}

function updateControlsToggleUi() {
  const btn = $("btnToggleControls");
  if (btn) btn.setAttribute("aria-pressed", isControlsCollapsed() ? "true" : "false");

  const splitter = $("mainSplitter");
  if (splitter) {
    const w = readControlsWidthPx();
    splitter.setAttribute("aria-valuemin", String(CONTROLS_W_MIN));
    splitter.setAttribute("aria-valuemax", String(CONTROLS_W_MAX));
    splitter.setAttribute("aria-valuenow", String(Math.round(w)));
  }
}

function toggleControlsCollapsed() {
  const collapsed = isControlsCollapsed();
  setControlsCollapsed(!collapsed, { persist: true });
  if (!collapsed) return;
  // Ensure a sane width when expanding from collapsed.
  const w = readControlsWidthPx();
  if (!Number.isFinite(w) || w < CONTROLS_W_MIN) applyControlsWidth(420, { persist: true });
}

function initControlsSplitter() {
  const splitter = $("mainSplitter");
  const toggleBtn = $("btnToggleControls");
  if (!splitter && !toggleBtn) return;

  updateControlsToggleUi();
  toggleBtn?.addEventListener("click", () => toggleControlsCollapsed());

  if (!splitter) return;

  splitter.addEventListener("dblclick", () => toggleControlsCollapsed());

  splitter.addEventListener("keydown", (e) => {
    const key = e.key;
    if (key === "Enter" || key === " ") {
      e.preventDefault();
      toggleControlsCollapsed();
      return;
    }
    if (isControlsCollapsed()) return;
    const step = e.shiftKey ? 48 : 20;
    if (key !== "ArrowLeft" && key !== "ArrowRight" && key !== "Home" && key !== "End") return;
    e.preventDefault();
    if (key === "Home") applyControlsWidth(CONTROLS_W_MIN, { persist: true });
    else if (key === "End") applyControlsWidth(CONTROLS_W_MAX, { persist: true });
    else if (key === "ArrowLeft") applyControlsWidth(readControlsWidthPx() - step, { persist: true });
    else if (key === "ArrowRight") applyControlsWidth(readControlsWidthPx() + step, { persist: true });
  });

  let dragging = false;
  let startX = 0;
  let startW = 0;

  const onMove = (ev) => {
    if (!dragging) return;
    const dx = Number(ev.clientX) - startX;
    if (!Number.isFinite(dx)) return;
    applyControlsWidth(startW + dx, { persist: false });
  };
  const endDrag = (ev) => {
    if (!dragging) return;
    dragging = false;
    try { splitter.releasePointerCapture(ev.pointerId); } catch {}
    applyControlsWidth(readControlsWidthPx(), { persist: true });
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };

  splitter.addEventListener("pointerdown", (ev) => {
    if (ev.button !== 0) return;
    if (isControlsCollapsed()) setControlsCollapsed(false, { persist: true });
    dragging = true;
    startX = Number(ev.clientX);
    startW = readControlsWidthPx();
    try { splitter.setPointerCapture(ev.pointerId); } catch {}
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  });
  splitter.addEventListener("pointermove", onMove);
  splitter.addEventListener("pointerup", endDrag);
  splitter.addEventListener("pointercancel", endDrag);
}

let MA_LIST = [];
let MA_BY_ID = {};
let RUNNING = false;
let RUNNING_COUNT = 0;
let OPT_PROGRESS_TIMER_ID = 0;
let OPT_PROGRESS_STARTED_AT_MS = 0;
let OPT_PROGRESS_ESTIMATE_LABEL = "";
let OPT_PROGRESS_LAST_EVENT_AT_MS = 0;
let MA_PARAMS_STATE = { fast: {}, slow: {} };
let MA_PARAM_SWEEPS_STATE = { fast: {}, slow: {} };
let NEXT_RUN_TAGS = null;
let BATCH_RUNNING = false;
let BATCH_STOP_REQUESTED = false;
let BATCH_DATASETS_CACHE = [];
let LAST_BATCH_SUMMARY = null;
let LAST_GPU_STATUS = null;
let LAST_GPU_STATUS_AT_MS = 0;
let LAST_CUDA_DEVICES = null;
let LAST_CUDA_DEVICES_AT_MS = 0;
let CUDA_DEVICES_INFLIGHT = null;
let EXEC_BUSY = false;
let LAST_RUN_READINESS = null;

const SETTINGS_KEY = "vectorbt_settings_v1";
const HISTORY_KEY = "vectorbt_history_v1";
const RECENT_CSV_KEY = "vectorbt_recent_csv_v1";
const RECENT_CSV_MAX = 12;
const LAST_BATCH_SUMMARY_KEY = "vectorbt_last_batch_summary_path_v1";
const EXEC_KEYS_STORAGE_KEY = "vectorbt_exec_keys_v1";

let CURRENT_RUN = null; // { ts, req, res }
let CURRENT_VALIDATION = null; // { ts, req, res }
let SAVED_RUNS = []; // disk-backed run artifacts
let SAVED_META = []; // disk-backed meta artifacts
let SAVED_STAGE4 = []; // disk-backed stage-4 artifacts
let LAST_META_RUN = null; // last meta selection response (paths + result)
let LAST_META_STAGE4_RUN = null; // last stage-4 response (artifact path + selected groups)
let SAVED_RUN_DETAIL_CACHE = {};
let SAVED_RUN_DETAIL_SEQ = 0;
let CURRENT_SAVED_RUN_DETAIL_PATH = "";
let TOP_ROWS = [];
let TOP_ROWS_FILTERED = [];
let TOP_SORT = { key: "rank", dir: "asc" };
let HEATMAP_META_BASE = "";
let HEATMAP_STATE = null; // { binsFast, binsSlow, fastBins, slowBins, topPoints, topByCell, keyToCell, hover, selected }
let HEATMAP_HOVER = null; // { fBin, sBin, idx, v }
let HEATMAP_SELECTED = null; // { fBin, sBin, idx, key }
let DRILL_SEQ = 0;
let DRILL_SELECTED_KEY = "";
let DRILL_PRICE_BARS_CACHE = new Map(); // data_id -> [{time,open,high,low,close,volume,from_index,to_index}]
let DRILL_CURVES_CACHE = new Map(); // key -> curves
let DRILL_CURVES_INFLIGHT = new Map(); // key -> Promise<curves>
let DRILL_PREFETCH_SEQ = 0;
let DRILL_INDICATOR_BARS_CACHE = new Map(); // request_key -> [{time,fast,slow,from_index,to_index}]
let DRILL_TV_CHART = null;
let DRILL_TV_SERIES = null;
let DRILL_TV_FAST_SERIES = null;
let DRILL_TV_SLOW_SERIES = null;
let DRILL_TV_HALF_SERIES = null;
let DRILL_TV_MACZ_SERIES = null;
let DRILL_TV_MARKERS = null;
let DRILL_TV_RESIZE_OBSERVER = null;
let DRILL_TV_LEGEND_EL = null;
let DRILL_TV_LEGEND_LAST_TEXT = "";
let DRILL_TV_LEGEND_PINNED_TEXT = "";
let DRILL_TV_LEGEND_SUBSCRIBED = false;
let DRILL_TV_CLICK_SUBSCRIBED = false;
let DRILL_TV_RANGE_SUBSCRIBED = false;
let DRILL_TV_MARKER_RAF = 0;
let DRILL_TV_MARKER_LAST_KEY = "";
let DRILL_TV_ALL_MARKERS = [];
let DRILL_TV_TRADE_EVENTS_BY_TIME = null; // time(sec) -> {entries:[], exits:[]}
let DRILL_SELECTED_TRADE_IDX = -1;
let DRILL_TRADES_RANGE_START = 0;
let DRILL_LAST_CURVES = null;
let DRILL_LAST_BARS = null;
let DRILL_LAST_BARS_DATA_ID = "";
let STABILITY_SEQ = 0;
let STABILITY_CACHE = new Map();
let PARETO_POINTS_RENDERED = [];
let CURRENT_DATA_META = null;
let CURRENT_DATASET_CTX = null;
let CURRENT_BASE_DATA_ID = "";
let CURRENT_BASE_DATA_META = null;
let CURRENT_BASE_DATASET_CTX = null;
var COMPARE_SEQ = 0;
var COMPARE_BASELINE_CACHE = {};
var META_COMPARE_SEQ = 0;
var META_BASELINE_CACHE = {};
let EXEC_TV_CHART = null;
let EXEC_TV_SERIES = null;
let EXEC_TV_MARKERS = null;
let EXEC_TV_RESIZE_OBSERVER = null;
let EXEC_LAST_SNAPSHOT = null;
let EXEC_TV_BASE_BARS = [];
let EXEC_TV_BASE_TIME_SET = new Set();
let EXEC_TV_LIVE_BARS = new Map(); // time(sec) -> candle
let EXEC_TV_MARKER_COUNT = 0;
let EXEC_TV_MERGED_COUNT = 0;
let EXEC_LIVE_STREAM_ACTIVE = false;
let EXEC_LIVE_STREAM_CFG = null;
let EXEC_LIVE_LISTENERS_READY = false;
let EXEC_RUNNER_ACTIVE = false;
let EXEC_RUNNER_STATUS = null;
let ACTIVE_PROGRESS_CONTEXT = null;

const DRILL_CURVES_CACHE_MAX = 96;
const DRILL_PREFETCH_TOP_N = 6;
const DRILL_TRADES_TABLE_MAX_ROWS = 200;

function clearDrillCaches(opts) {
  const clearBars = opts?.clearBars !== false;
  DRILL_SEQ++;
  DRILL_PREFETCH_SEQ++;
  DRILL_SELECTED_KEY = "";
  DRILL_SELECTED_TRADE_IDX = -1;
  DRILL_TV_LEGEND_PINNED_TEXT = "";
  DRILL_TRADES_RANGE_START = 0;
  DRILL_LAST_CURVES = null;
  DRILL_CURVES_CACHE.clear();
  DRILL_CURVES_INFLIGHT.clear();
  DRILL_INDICATOR_BARS_CACHE.clear();
  if (clearBars) {
    DRILL_PRICE_BARS_CACHE.clear();
    DRILL_LAST_BARS = null;
    DRILL_LAST_BARS_DATA_ID = "";
  }
  DRILL_TV_ALL_MARKERS = [];
  DRILL_TV_MARKER_LAST_KEY = "";
  if (DRILL_TV_MARKER_RAF) {
    if (typeof cancelAnimationFrame === "function") cancelAnimationFrame(DRILL_TV_MARKER_RAF);
    else clearTimeout(DRILL_TV_MARKER_RAF);
    DRILL_TV_MARKER_RAF = 0;
  }
  DRILL_TV_TRADE_EVENTS_BY_TIME = null;
}

function fileBasename(path) {
  const s = String(path || "");
  if (!s) return "";
  const parts = s.split(/[\\/]/g).filter((p) => !!p);
  return parts.length ? parts[parts.length - 1] : s;
}

const DATASET_META_SCHEMA_VERSION = 2;
const RESAMPLE_RULE_SESSION_RELATIVE_OHLCV = "session_relative_ohlcv";
const RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP = "day_first_timestamp";

function buildDatasetMetaForArtifact(csvPath) {
  const path = String(csvPath || "").trim();
  if (!path) return null;

  const meta = CURRENT_DATA_META && typeof CURRENT_DATA_META === "object" ? CURRENT_DATA_META : null;
  const baseMeta = CURRENT_BASE_DATA_META && typeof CURRENT_BASE_DATA_META === "object" ? CURRENT_BASE_DATA_META : null;
  const baseCtx = CURRENT_BASE_DATASET_CTX && typeof CURRENT_BASE_DATASET_CTX === "object" ? CURRENT_BASE_DATASET_CTX : {};
  const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};

  const label = String(ctx.label || "").trim() || fileBasename(path) || path;
  const tfCurrent = String(ctx.timeframe || meta?.timeframe || "").trim() || null;
  const base_timeframe = String(
    ctx.base_timeframe || baseCtx.base_timeframe || baseCtx.timeframe || baseMeta?.timeframe || tfCurrent || ""
  ).trim() || null;
  const derived_timeframe = String(ctx.derived_timeframe || tfCurrent || "").trim() || null;
  const timeframe = derived_timeframe || tfCurrent || null;
  const resample_applied = !!(base_timeframe && derived_timeframe && base_timeframe !== derived_timeframe);
  const resample_rule = String(ctx.resample_rule || "").trim() || (derived_timeframe ? RESAMPLE_RULE_SESSION_RELATIVE_OHLCV : null);
  const resample_anchor = String(ctx.resample_anchor || "").trim() || (derived_timeframe ? RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP : null);
  const source = String(ctx.source || "").trim() || null;
  const market = String(ctx.market || "").trim() || null;
  const symbol = String(ctx.symbol || "").trim() || null;
  const adjusted = ctx.adjusted === true ? true : (ctx.adjusted === false ? false : null);

  const len = Number(meta?.len || 0) || null;
  const min_ts = Number(meta?.min_ts || 0) || null;
  const max_ts = Number(meta?.max_ts || 0) || null;

  const sizeRaw = meta?.source_size_bytes ?? baseMeta?.source_size_bytes;
  const mtimeRaw = meta?.source_mtime_utc_ms ?? baseMeta?.source_mtime_utc_ms;
  const fpRaw = meta?.source_content_fingerprint ?? baseMeta?.source_content_fingerprint;
  const source_size_bytes = Number.isFinite(Number(sizeRaw)) ? Number(sizeRaw) : null;
  const source_mtime_utc_ms = Number.isFinite(Number(mtimeRaw)) ? Number(mtimeRaw) : null;
  const source_content_fingerprint = String(fpRaw || "").trim() || null;

  return {
    schema_version: DATASET_META_SCHEMA_VERSION,
    label,
    csv_path: path,
    timeframe,
    base_timeframe,
    derived_timeframe,
    resample_applied,
    resample_rule,
    resample_anchor,
    source,
    market,
    symbol,
    adjusted,
    len,
    min_ts,
    max_ts,
    source_size_bytes,
    source_mtime_utc_ms,
    source_content_fingerprint,
  };
}

function setStatus(msg, kind) {
  const el = $("status");
  el.textContent = msg;
  el.className = "statusChip mono " + (kind || "");
  maybeToastFromStatus(msg, kind);
  updateWorkspaceContextStrip();
}

function datasetSummaryLine() {
  const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};
  const meta = CURRENT_DATA_META && typeof CURRENT_DATA_META === "object" ? CURRENT_DATA_META : null;

  const label = String(ctx.label || "").trim();
  const tf = String(ctx.timeframe || meta?.timeframe || "").trim();
  const rows = Number(meta?.len || 0);
  const parts = [];
  if (label) parts.push(label);
  if (tf) parts.push(`tf=${tf}`);
  if (Number.isFinite(rows) && rows > 0) parts.push(`rows=${rows}`);
  return parts.join(" · ");
}

function ctxChip(label, value, opts) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "chip";
  const onClick = typeof opts?.onClick === "function" ? opts.onClick : null;
  if (!onClick) btn.classList.add("passive");
  else btn.addEventListener("click", onClick);

  const title = String(opts?.title || "").trim();
  if (title) btn.title = title;

  const k = document.createElement("span");
  k.className = "chipK";
  k.textContent = String(label || "").trim() || "—";

  const v = document.createElement("span");
  v.className = "chipV";
  v.textContent = String(value ?? "").trim() || "—";

  btn.appendChild(k);
  btn.appendChild(v);
  return btn;
}

function renderContextChips() {
  const host = $("contextChips");
  if (!host) return;
  host.innerHTML = "";

  const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};
  const meta = CURRENT_DATA_META && typeof CURRENT_DATA_META === "object" ? CURRENT_DATA_META : null;

  const label = String(ctx.label || "").trim();
  const tf = String(ctx.timeframe || meta?.timeframe || "").trim();
  const rows = Number(meta?.len || 0);
  const source = String(ctx.source || "").trim();
  const market = String(ctx.market || "").trim();
  const symbol = String(ctx.symbol || "").trim();
  const csvPath = String($("csvPath")?.value || "").trim();

  const range = meta ? `${fmtYmdUtc(meta.min_ts)} → ${fmtYmdUtc(meta.max_ts)}` : "—";
  const dataValueParts = [];
  if (label) dataValueParts.push(label);
  else if (symbol) dataValueParts.push(symbol);
  else if (csvPath) dataValueParts.push(fileBasename(csvPath) || csvPath);
  if (tf) dataValueParts.push(tf);
  if (Number.isFinite(rows) && rows > 0) dataValueParts.push(`${rows} rows`);
  const dataValue = dataValueParts.length ? dataValueParts.join(" · ") : "—";

  const dataTitleParts = [];
  if (source) dataTitleParts.push(`source=${source}`);
  if (market) dataTitleParts.push(`market=${market}`);
  if (symbol) dataTitleParts.push(`symbol=${symbol}`);
  if (csvPath) dataTitleParts.push(`file=${fileBasename(csvPath)}`);
  if (range && range !== "—") dataTitleParts.push(`range=${range}`);
  const dataTitle = dataTitleParts.join(" • ");

  host.appendChild(ctxChip("Data", dataValue, {
    title: dataTitle,
    onClick: () => {
      CONTROLS_TABS?.set("data", { save: true, focus: true });
      try { $("csvPath")?.focus(); } catch {}
    },
  }));

  const kind = CURRENT_RUN?.req ? kindFromReq(CURRENT_RUN.req) : strategyKind();
  const kindLabel = strategyKindLabel(kind);
  host.appendChild(ctxChip("Strategy", kindLabel, {
    title: isDoubleLikeKind(kind) ? "Fast double moving-average crossover family." : "Composite strategy optimization.",
    onClick: () => CONTROLS_TABS?.set("strategy", { save: true, focus: true }),
  }));

  const backendFromRun = String(CURRENT_RUN?.res?.backend_used || "").trim();
  const backendFromReq = String(CURRENT_RUN?.req?.backend?.backend || "").trim();
  const backendFromUi = String($("backend")?.value || "").trim();
  const backend = isDoubleLikeKind(kind)
    ? (backendFromRun || backendFromReq || backendFromUi || "CpuOnly")
    : "CpuOnly";
  const deviceFromReq = Number(CURRENT_RUN?.req?.backend?.device_id ?? NaN);
  const deviceFromUi = Number($("deviceId")?.value ?? NaN);
  const dev = Number.isFinite(deviceFromReq) ? deviceFromReq : (Number.isFinite(deviceFromUi) ? deviceFromUi : null);
  const backendValue = dev != null && backend.toLowerCase().includes("gpu") ? `${backend} · dev ${dev}` : backend;
  host.appendChild(ctxChip("Backend", backendValue, {
    title: isDoubleLikeKind(kind) ? "Execution backend used for optimization." : "Composite strategies currently run on CPU.",
    onClick: () => CONTROLS_TABS?.set("opt", { save: true, focus: true }),
  }));

  const objective = (() => {
    if (kind === "CompositeMo") {
      const max = String(CURRENT_RUN?.req?.objectives?.maximize || $("objective")?.value || "Sharpe").trim() || "Sharpe";
      const min = String(CURRENT_RUN?.req?.objectives?.minimize || $("moObjectiveMin")?.value || "MaxDrawdown").trim() || "MaxDrawdown";
      return `max ${max} / min ${min}`;
    }
    const obj = String(CURRENT_RUN?.req?.objective || $("objective")?.value || "Sharpe").trim() || "Sharpe";
    return obj;
  })();
  host.appendChild(ctxChip("Objective", objective, {
    title: "Optimization objective(s).",
    onClick: () => CONTROLS_TABS?.set("opt", { save: true, focus: true }),
  }));

  const mode = (() => {
    const m = String(CURRENT_RUN?.res?.mode_used || CURRENT_RUN?.req?.mode || $("mode")?.value || "Grid").trim() || "Grid";
    return m;
  })();
  host.appendChild(ctxChip("Mode", mode, {
    title: "Search mode (Grid / Coarse→Fine / GPU VDPS).",
    onClick: () => CONTROLS_TABS?.set("opt", { save: true, focus: true }),
  }));

  updateWorkspaceContextStrip();
}

function initOverviewEmptyUi() {
  const primary = $("btnOverviewEmptyPrimary");
  const secondary = $("btnOverviewEmptySecondary");
  const onClick = (btn) => {
    const action = String(btn?.dataset?.action || "").trim();
    if (!action) return;
    try {
      if (action === "browse_csv") {
        setWorkspace("workflow", { save: true, focus: false, step: "data" });
        $("btnBrowseCsv")?.click();
        return;
      }
      if (action === "polygon") {
        setWorkspace("workflow", { save: true, focus: false, step: "data" });
        $("btnPolygonOpen")?.click();
        return;
      }
      if (action === "go_opt") {
        setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
        CONTROLS_TABS?.set("opt", { save: true, focus: true });
        return;
      }
      if (action === "run_opt") {
        setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
        CONTROLS_TABS?.set("opt", { save: true, focus: false });
        $("btnRun")?.click();
        return;
      }
      if (action === "cancel_run") {
        $("btnCancel")?.click();
        return;
      }
    } catch {}
  };
  primary?.addEventListener("click", () => onClick(primary));
  secondary?.addEventListener("click", () => onClick(secondary));
}

function updateOverviewEmptyUi() {
  const empty = $("overviewEmpty");
  const content = $("overviewContent");
  const title = $("overviewEmptyTitle");
  const body = $("overviewEmptyBody");
  const hint = $("overviewEmptyHint");
  const primary = $("btnOverviewEmptyPrimary");
  const secondary = $("btnOverviewEmptySecondary");
  if (!empty || !content || !title || !body || !hint || !primary || !secondary) return;

  const dataIdText = String($("dataId")?.textContent || "").trim();
  const hasDataId = !!dataIdText && dataIdText !== "—" && dataIdText !== "-";
  const hasMetaRows = Number(CURRENT_DATA_META?.len || 0) > 0;
  const hasData = hasDataId || hasMetaRows;
  const hasRun = !!(CURRENT_RUN?.req && CURRENT_RUN?.res);

  if (hasRun) {
    empty.style.display = "none";
    content.style.display = "block";
    return;
  }

  if (!hasData) {
    empty.style.display = "block";
    content.style.display = "none";
    title.textContent = "Load data to begin";
    body.textContent = "Load a CSV dataset or download candles from Polygon, then run an optimization to view results here.";
    hint.textContent = "Tip: start with a small dataset to validate your strategy settings.";
    primary.textContent = "Browse CSV…";
    primary.dataset.action = "browse_csv";
    secondary.style.display = "";
    secondary.textContent = "Polygon…";
    secondary.dataset.action = "polygon";
    return;
  }

  if (RUNNING) {
    empty.style.display = "block";
    content.style.display = "none";
    title.textContent = "Working…";
    const ds = datasetSummaryLine();
    const progressText = String($("progressMeta")?.textContent || "").trim();
    const statusText = String($("status")?.textContent || "").trim();
    const line = progressText || (statusText && statusText !== "Ready." ? statusText : "Running…");
    body.textContent = ds ? `${line} · ${ds}` : line;
    hint.textContent = "Tip: You can cancel safely; some backends may take a moment to stop.";
    primary.textContent = "Cancel";
    primary.dataset.action = "cancel_run";
    secondary.style.display = "";
    secondary.textContent = "Go to Optimize";
    secondary.dataset.action = "go_opt";
    return;
  }

  if (!hasRun) {
    empty.style.display = "block";
    content.style.display = "none";
    title.textContent = "Ready to optimize";
    const ds = datasetSummaryLine();
    body.textContent = ds
      ? `Loaded dataset: ${ds}. Configure your strategy settings and run an optimization to populate results.`
      : "Configure your strategy settings and run an optimization to populate results.";
    hint.textContent = "Tip: GPU (kernel) is fastest for large grids and stays VRAM-resident.";
    primary.textContent = "Run optimization";
    primary.dataset.action = "run_opt";
    secondary.style.display = "";
    secondary.textContent = "Go to Optimize";
    secondary.dataset.action = "go_opt";
    return;
  }
}

function resetBestCardUi() {
  const kicker = $("bestKicker");
  const title = $("bestTitle");
  const meta = $("bestMeta");
  const grid = $("bestGrid");
  if (kicker) kicker.textContent = "—";
  if (title) title.textContent = "—";
  if (meta) meta.textContent = "—";
  if (grid) grid.innerHTML = "";
}

function renderBestCard(model) {
  const kicker = $("bestKicker");
  const title = $("bestTitle");
  const meta = $("bestMeta");
  const grid = $("bestGrid");
  if (!kicker || !title || !meta || !grid) return;

  const m = model && typeof model === "object" ? model : {};
  kicker.textContent = String(m.kicker || "Best").trim() || "Best";
  title.textContent = String(m.title || "—").trim() || "—";
  meta.textContent = String(m.meta || "—").trim() || "—";

  grid.innerHTML = "";
  const stats = Array.isArray(m.stats) ? m.stats : [];
  for (const s0 of stats) {
    const s = s0 && typeof s0 === "object" ? s0 : {};
    const label = String(s.label || "").trim();
    const value = String(s.value ?? "").trim();
    if (!label) continue;
    const tone = String(s.tone || "").trim();

    const box = document.createElement("div");
    box.className = `bestStat ${tone}`.trim();

    const k = document.createElement("div");
    k.className = "bestStatK";
    k.textContent = label;

    const v = document.createElement("div");
    v.className = "bestStatV";
    v.textContent = value || "—";
    if (value && value.length > 18) v.title = value;

    box.appendChild(k);
    box.appendChild(v);
    grid.appendChild(box);
  }
}

async function copyTextToClipboard(text) {
  const s = String(text || "").trim();
  if (!s) return false;
  try {
    if (navigator?.clipboard?.writeText) {
      await navigator.clipboard.writeText(s);
      return true;
    }
  } catch {}

  try {
    const ta = document.createElement("textarea");
    ta.value = s;
    ta.setAttribute("readonly", "");
    ta.style.position = "fixed";
    ta.style.left = "-9999px";
    ta.style.top = "0";
    document.body.appendChild(ta);
    ta.focus();
    ta.select();
    const ok = document.execCommand("copy");
    ta.remove();
    return !!ok;
  } catch {
    return false;
  }
}

function initBestCardUi() {
  const btn = $("btnCopyBest");
  if (btn) {
    btn.addEventListener("click", async () => {
      const txt = String($("bestSummary")?.textContent || "").trim();
      if (!txt || txt === "-" || txt === "—") {
        showToast("Nothing to copy yet.", "danger", { ms: 2600 });
        return;
      }
      const ok = await copyTextToClipboard(txt);
      showToast(ok ? "Copied best summary." : "Copy failed.", ok ? "ok" : "danger", { ms: ok ? 2200 : 4200 });
    });
  }
  resetBestCardUi();
}

let LAST_TOAST_AT_MS = 0;
let LAST_TOAST_MSG = "";

function ensureToastHost() {
  const existing = $("toastHost");
  if (existing) return existing;
  const host = document.createElement("div");
  host.id = "toastHost";
  host.className = "toastHost";
  host.setAttribute("aria-live", "polite");
  host.setAttribute("aria-relevant", "additions");
  host.setAttribute("aria-atomic", "false");
  document.body.appendChild(host);
  return host;
}

function removeToast(el) {
  if (!el) return;
  try { el.classList.remove("in"); } catch {}
  setTimeout(() => {
    try { el.remove(); } catch {}
  }, 220);
}

function showToast(msg, kind, opts) {
  let text = String(msg || "").trim();
  if (!text) return;
  const maxCharsRaw = Number(opts?.maxChars ?? 240);
  const maxChars = Number.isFinite(maxCharsRaw) ? Math.max(80, Math.min(800, Math.floor(maxCharsRaw))) : 240;
  if (text.length > maxChars) {
    text = text.slice(0, Math.max(0, maxChars - 1)).trimEnd() + "…";
  }
  const k = String(kind || "").trim();

  const host = ensureToastHost();

  const toast = document.createElement("div");
  toast.className = `toast ${k}`.trim();

  const dot = document.createElement("div");
  dot.className = "toastDot";

  const body = document.createElement("div");
  body.className = "toastBody";
  body.textContent = text;

  const close = document.createElement("button");
  close.className = "toastClose";
  close.type = "button";
  close.title = "Dismiss";
  close.setAttribute("aria-label", "Dismiss");
  close.textContent = "×";
  close.addEventListener("click", () => removeToast(toast));

  toast.appendChild(dot);
  toast.appendChild(body);
  toast.appendChild(close);
  host.prepend(toast);

  // Cap visible toasts.
  while (host.children.length > 4) {
    const last = host.lastElementChild;
    if (!last) break;
    last.remove();
  }

  requestAnimationFrame(() => {
    try { toast.classList.add("in"); } catch {}
  });

  const ms = Number(opts?.ms || 0);
  const ttl = Number.isFinite(ms) && ms > 0 ? ms : (k === "danger" ? 6500 : 3800);
  setTimeout(() => removeToast(toast), ttl);
}

function maybeToastFromStatus(msg, kind) {
  const k = String(kind || "").trim();
  if (k !== "ok" && k !== "danger") return;

  const text = String(msg || "").trim();
  if (!text) return;
  // Avoid spamming toasts during in-flight status updates.
  if (text.includes("...")) return;

  const now = Date.now();
  if (!Number.isFinite(now)) return;
  if ((now - LAST_TOAST_AT_MS) < 1200) return;
  if (text === LAST_TOAST_MSG) return;

  LAST_TOAST_AT_MS = now;
  LAST_TOAST_MSG = text;
  showToast(text, k);
}

let CMD_LAST_FOCUS = null;
let CMD_SELECTED_IDX = 0;

function isCmdPaletteOpen() {
  const modal = $("cmdPaletteModal");
  return !!modal && modal.classList.contains("open");
}

function setCmdPaletteOpen(open) {
  const modal = $("cmdPaletteModal");
  if (!modal) return;
  if (open) {
    modal.classList.add("open");
    modal.setAttribute("aria-hidden", "false");
  } else {
    modal.classList.remove("open");
    modal.setAttribute("aria-hidden", "true");
  }
}

function hasLoadedData() {
  const dataIdText = String($("dataId")?.textContent || "").trim();
  const hasDataId = !!dataIdText && dataIdText !== "—" && dataIdText !== "-";
  const hasMetaRows = Number(CURRENT_DATA_META?.len || 0) > 0;
  return hasDataId || hasMetaRows;
}

function updateQuickBarUi() {
  const browse = $("btnQaBrowseCsv");
  const run = $("btnQaRun");
  const cancel = $("btnQaCancel");
  const validate = $("btnQaValidate");
  const exportCsv = $("btnQaExportCsv");
  const palette = $("btnQaPalette");
  const hint = $("quickBarHint");

  const hasData = hasLoadedData();
  const hasRun = !!(CURRENT_RUN?.req && CURRENT_RUN?.res);
  const kind = strategyKind();
  const isComp = kind === "CompositeCpu" || kind === "CompositeMo";
  const supportsValidate = isComp || isDoubleLikeKind(kind);
  const hasReadiness = !!(LAST_RUN_READINESS && typeof LAST_RUN_READINESS === "object");
  const blockers = hasReadiness && Array.isArray(LAST_RUN_READINESS.blockers) ? LAST_RUN_READINESS.blockers : [];
  const canRun = hasReadiness ? !!LAST_RUN_READINESS.can_run : (!RUNNING && hasData);

  if (browse) browse.disabled = RUNNING;
  if (run) run.disabled = !canRun;
  if (cancel) cancel.disabled = !RUNNING;
  if (validate) {
    validate.disabled = RUNNING || !supportsValidate || !hasRun;
    validate.style.display = supportsValidate ? "" : "none";
  }
  if (exportCsv) exportCsv.disabled = RUNNING || !hasRun;
  if (palette) palette.disabled = false;

  if (hint) {
    const parts = [];
    if (!hasData) parts.push("Load data first");
    if (!RUNNING && blockers.length) parts.push(`blocked: ${blockers[0]}`);
    if (RUNNING) parts.push("running");
    hint.textContent = parts.length ? `Tip: ${parts.join(" • ")} • Ctrl+K = commands` : "Tip: Use the command palette for everything (Ctrl+K).";
  }
}

function initQuickBarUi() {
  $("btnQaBrowseCsv")?.addEventListener("click", () => {
    if (RUNNING) return;
    setWorkspace("workflow", { save: true, focus: false, step: "data" });
    $("btnBrowseCsv")?.click();
  });
  $("btnQaRun")?.addEventListener("click", () => {
    if (RUNNING) return;
    setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
    CONTROLS_TABS?.set("opt", { save: true, focus: false });
    $("btnRun")?.click();
  });
  $("btnQaCancel")?.addEventListener("click", () => {
    if (!RUNNING) return;
    $("btnCancel")?.click();
  });
  $("btnQaValidate")?.addEventListener("click", () => {
    if (RUNNING) return;
    setWorkspace("workflow", { save: true, focus: false, step: "validate" });
    $("btnValidate")?.click();
  });
  $("btnQaExportCsv")?.addEventListener("click", () => {
    if (RUNNING) return;
    $("btnExportCsv")?.click();
  });
  $("btnQaPalette")?.addEventListener("click", () => openCmdPalette());
  updateQuickBarUi();
}

function buildCommandActions() {
  const invoke = tauriInvoke();
  const okInvoke = !!invoke;

  const hasData = hasLoadedData();
  const hasRun = !!(CURRENT_RUN?.req && CURRENT_RUN?.res);
  const kind = strategyKind();
  const isComp = kind === "CompositeCpu" || kind === "CompositeMo";
  const supportsValidate = isComp || isDoubleLikeKind(kind);
  const eff = currentWindowEffect();
  const supportedEff = supportedWindowEffects();

  const keyCaps = (s) => String(s || "").split("+").map((x) => x.trim()).filter((x) => !!x);

  const actions = [
    {
      id: "effect_none",
      name: `${eff === "none" ? "✓ " : ""}Window effects: None`,
      desc: "Disable native window effects (use CSS only).",
      keys: [],
      enabled: () => true,
      run: () => applyWindowEffectPref("none", { persist: true }),
      keywords: ["effects", "window", "vibrancy", "mica"],
    },
    {
      id: "effect_mica",
      name: `${eff === "mica" ? "✓ " : ""}Window effects: Mica`,
      desc: "Enable Mica (Windows 11).",
      keys: [],
      enabled: () => supportedEff.has("mica"),
      run: () => applyWindowEffectPref("mica", { persist: true }),
      keywords: ["effects", "mica", "windows"],
    },
    {
      id: "effect_acrylic",
      name: `${eff === "acrylic" ? "✓ " : ""}Window effects: Acrylic`,
      desc: "Enable Acrylic blur (Windows).",
      keys: [],
      enabled: () => supportedEff.has("acrylic"),
      run: () => applyWindowEffectPref("acrylic", { persist: true }),
      keywords: ["effects", "acrylic", "windows"],
    },
    {
      id: "effect_vibrancy",
      name: `${eff === "vibrancy" ? "✓ " : ""}Window effects: Vibrancy`,
      desc: "Enable vibrancy (macOS).",
      keys: [],
      enabled: () => supportedEff.has("vibrancy"),
      run: () => applyWindowEffectPref("vibrancy", { persist: true }),
      keywords: ["effects", "vibrancy", "mac"],
    },
    {
      id: "toggle_controls",
      name: isControlsCollapsed() ? "Show controls panel" : "Hide controls panel",
      desc: "Collapse/expand the left Controls column.",
      keys: keyCaps("Ctrl+\\"),
      enabled: () => true,
      run: () => toggleControlsCollapsed(),
      keywords: ["controls", "sidebar", "resize"],
    },
    {
      id: "browse_csv",
      name: "Browse CSV…",
      desc: "Pick a local CSV dataset file.",
      keys: keyCaps("Ctrl+O"),
      enabled: () => !RUNNING,
      run: () => {
        setWorkspace("workflow", { save: true, focus: false, step: "data" });
        $("btnBrowseCsv")?.click();
      },
      keywords: ["data", "load", "csv"],
    },
    {
      id: "open_polygon",
      name: "Polygon…",
      desc: "Download/cached datasets from Polygon and load into the app.",
      keys: keyCaps("Ctrl+Shift+O"),
      enabled: () => !RUNNING,
      run: () => {
        setWorkspace("workflow", { save: true, focus: false, step: "data" });
        $("btnPolygonOpen")?.click();
      },
      keywords: ["polygon", "download", "cache"],
    },
    {
      id: "run",
      name: "Run optimization",
      desc: "Run the current optimization workflow.",
      keys: keyCaps("Ctrl+Enter"),
      enabled: () => !RUNNING && hasData,
      run: () => {
        setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
        CONTROLS_TABS?.set("opt", { save: true, focus: false });
        $("btnRun")?.click();
      },
      keywords: ["optimize", "run", "backtest"],
    },
    {
      id: "cancel",
      name: "Cancel",
      desc: "Request cancellation of the current run.",
      keys: keyCaps("Esc"),
      enabled: () => RUNNING,
      run: () => $("btnCancel")?.click(),
      keywords: ["stop"],
    },
    {
      id: "validate",
      name: "Run validation",
      desc: "Run robustness checks for current Top/Pareto candidates.",
      keys: keyCaps("Ctrl+Shift+V"),
      enabled: () => !RUNNING && supportsValidate && hasRun,
      run: () => {
        setWorkspace("workflow", { save: true, focus: false, step: "validate" });
        $("btnValidate")?.click();
      },
      keywords: ["validate", "robust", "wf", "mc", "cscv"],
    },
    {
      id: "export_csv",
      name: "Export Top CSV",
      desc: "Export current Top rows to CSV.",
      keys: keyCaps("Ctrl+Shift+C"),
      enabled: () => !RUNNING && hasRun,
      run: () => $("btnExportCsv")?.click(),
      keywords: ["export", "csv"],
    },
    {
      id: "export_json",
      name: "Export JSON",
      desc: "Export current run result JSON.",
      keys: keyCaps("Ctrl+Shift+J"),
      enabled: () => !RUNNING && hasRun,
      run: () => $("btnExportJson")?.click(),
      keywords: ["export", "json"],
    },
    {
      id: "open_latest_report",
      name: "Open latest report",
      desc: "Open the most recent XLSX report on disk.",
      keys: keyCaps("Ctrl+Shift+R"),
      enabled: () => !RUNNING && okInvoke && !!String(CURRENT_RUN?.report_xlsx_path || "").trim(),
      run: () => $("btnOpenLatestReport")?.click(),
      keywords: ["report", "xlsx"],
    },
    {
      id: "open_latest_meta_report",
      name: "Open latest meta report",
      desc: "Open the most recent meta XLSX report on disk.",
      keys: keyCaps("Ctrl+Shift+M"),
      enabled: () => !RUNNING && okInvoke && !!String(LAST_META_RUN?.report_xlsx_path || "").trim(),
      run: () => $("btnOpenLatestMetaReport")?.click(),
      keywords: ["meta", "report", "xlsx"],
    },
    {
      id: "go_workflow_data",
      name: "Go to: Workflow / Data Source",
      desc: "Workflow step 1: load candles and set timeframe.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("workflow", { save: true, focus: true, step: "data" }),
      keywords: ["nav", "workflow", "data"],
    },
    {
      id: "go_workflow_optimize",
      name: "Go to: Workflow / Strategy Setup",
      desc: "Workflow step 2: configure and run optimization.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("workflow", { save: true, focus: true, step: "optimize" }),
      keywords: ["nav", "workflow", "optimize", "run"],
    },
    {
      id: "go_workflow_validate",
      name: "Go to: Workflow / Validation",
      desc: "Workflow step 3: run robustness checks.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("workflow", { save: true, focus: true, step: "validate" }),
      keywords: ["nav", "workflow", "validate", "robust"],
    },
    {
      id: "go_workflow_analyze",
      name: "Go to: Workflow / Summary",
      desc: "Workflow step 4: overview, drilldown, and heatmap.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("workflow", { save: true, focus: true, step: "analyze" }),
      keywords: ["nav", "workflow", "analyze", "drilldown", "heatmap"],
    },
    {
      id: "go_workflow_history",
      name: "Go to: Run Library",
      desc: "Browse, compare, and rerun saved optimizations.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("library", { save: true, focus: true }),
      keywords: ["nav", "library", "history", "runs"],
    },
    {
      id: "go_workflow_settings",
      name: "Go to: Aggregate Analysis",
      desc: "Meta selection and run aggregation workflows.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("aggregate", { save: true, focus: true }),
      keywords: ["nav", "aggregate", "meta", "batch"],
    },
    {
      id: "go_workspace_pro",
      name: "Go to: Pro Tools",
      desc: "Distributed compute, advanced controls, and debug.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("pro", { save: true, focus: true }),
      keywords: ["nav", "pro", "distributed", "debug"],
    },
    {
      id: "go_workspace_settings",
      name: "Go to: Settings",
      desc: "Theme, effects, and storage options.",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("settings", { save: true, focus: true }),
      keywords: ["nav", "settings", "theme", "storage"],
    },
    {
      id: "go_aggregate_meta",
      name: "Aggregate: Meta Selection",
      desc: "Open Aggregate Analysis and show Meta Selection.",
      keys: [],
      enabled: () => true,
      run: () => {
        setWorkspace("aggregate", { save: true, focus: true });
        setAggregateTab("meta", { persist: true });
      },
      keywords: ["aggregate", "meta", "analysis"],
    },
    {
      id: "go_aggregate_batch",
      name: "Aggregate: Batch Generation",
      desc: "Open Aggregate Analysis and show Batch Generation.",
      keys: [],
      enabled: () => true,
      run: () => {
        setWorkspace("aggregate", { save: true, focus: true });
        setAggregateTab("batch", { persist: true });
      },
      keywords: ["aggregate", "batch", "datasets"],
    },
    {
      id: "go_aggregate_saved",
      name: "Aggregate: Saved Meta",
      desc: "Open Aggregate Analysis and show Saved Meta runs.",
      keys: [],
      enabled: () => true,
      run: () => {
        setWorkspace("aggregate", { save: true, focus: true });
        setAggregateTab("saved", { persist: true });
      },
      keywords: ["aggregate", "saved", "meta", "runs"],
    },
    {
      id: "go_aggregate_compare",
      name: "Aggregate: Compare Meta",
      desc: "Open Aggregate Analysis and show Compare Meta.",
      keys: [],
      enabled: () => true,
      run: () => {
        setWorkspace("aggregate", { save: true, focus: true });
        setAggregateTab("compare", { persist: true });
      },
      keywords: ["aggregate", "compare", "meta"],
    },
    {
      id: "go_data",
      name: "Go to: Data tab",
      desc: "Switch to the Data tab (controls).",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("workflow", { save: true, focus: true, step: "data" }),
      keywords: ["nav", "tab", "controls"],
    },
    {
      id: "go_opt",
      name: "Go to: Optimize tab",
      desc: "Switch to the Optimize tab (controls).",
      keys: [],
      enabled: () => true,
      run: () => setWorkspace("workflow", { save: true, focus: true, step: "optimize" }),
      keywords: ["nav", "tab", "controls"],
    },
    {
      id: "go_overview",
      name: "Go to: Overview tab",
      desc: "Switch to the Overview tab (results).",
      keys: [],
      enabled: () => true,
      run: () => {
        setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
        RESULTS_TABS?.set("overview", { save: true, focus: true });
      },
      keywords: ["nav", "tab", "results"],
    },
    {
      id: "go_top",
      name: "Go to: Top tab",
      desc: "Switch to the Top tab (results).",
      keys: [],
      enabled: () => true,
      run: () => {
        setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
        RESULTS_TABS?.set("top", { save: true, focus: true });
      },
      keywords: ["nav", "tab", "results"],
    },
  ];

  return actions;
}

function cmdScore(action, tokens) {
  const hay = (String(action.name || "") + " " + String(action.desc || "") + " " + String((action.keywords || []).join(" ") || "")).toLowerCase();
  let score = 0;
  for (const t of tokens) {
    const idx = hay.indexOf(t);
    if (idx < 0) return null;
    score += idx;
  }
  return score;
}

function renderCmdPaletteList(query) {
  const list = $("cmdPaletteList");
  if (!list) return;
  list.innerHTML = "";

  const q = String(query || "").trim().toLowerCase();
  const tokens = q ? q.split(/\s+/g).filter((t) => !!t) : [];
  const actions = buildCommandActions();

  const scored = [];
  for (const a of actions) {
    const s = tokens.length ? cmdScore(a, tokens) : 0;
    if (s == null) continue;
    scored.push({ a, s });
  }
  scored.sort((x, y) => x.s - y.s);
  const filtered = scored.map((x) => x.a);

  if (CMD_SELECTED_IDX >= filtered.length) CMD_SELECTED_IDX = Math.max(0, filtered.length - 1);
  if (CMD_SELECTED_IDX < 0) CMD_SELECTED_IDX = 0;

  const onPick = (idx) => {
    const a = filtered[idx];
    if (!a) return;
    if (!a.enabled()) return;
    try { a.run(); } catch {}
    closeCmdPalette({ restoreFocus: false });
  };

  for (let i = 0; i < filtered.length; i++) {
    const a = filtered[i];
    const enabled = !!a.enabled();
    const row = document.createElement("div");
    row.className = `cmdItem${i === CMD_SELECTED_IDX ? " selected" : ""}${enabled ? "" : " disabled"}`;
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", i === CMD_SELECTED_IDX ? "true" : "false");
    row.addEventListener("click", () => onPick(i));

    const left = document.createElement("div");
    left.className = "cmdLeft";
    const name = document.createElement("div");
    name.className = "cmdName";
    name.textContent = String(a.name || "").trim() || "(unnamed)";
    const desc = document.createElement("div");
    desc.className = "cmdDesc";
    desc.textContent = String(a.desc || "").trim() || "";
    left.appendChild(name);
    if (desc.textContent) left.appendChild(desc);

    const right = document.createElement("div");
    right.className = "cmdRight";
    const keys = Array.isArray(a.keys) ? a.keys : [];
    for (const k of keys) {
      const el = document.createElement("kbd");
      el.textContent = k;
      right.appendChild(el);
    }

    row.appendChild(left);
    row.appendChild(right);
    list.appendChild(row);
  }

  if (!filtered.length) {
    const empty = document.createElement("div");
    empty.className = "muted";
    empty.style.fontSize = "0.85rem";
    empty.textContent = "No matching commands.";
    list.appendChild(empty);
  }
}

function openCmdPalette() {
  if (isCmdPaletteOpen()) return;
  CMD_LAST_FOCUS = document.activeElement;
  CMD_SELECTED_IDX = 0;
  setCmdPaletteOpen(true);
  const input = $("cmdPaletteInput");
  if (input) {
    input.value = "";
    try { input.focus(); } catch {}
  }
  renderCmdPaletteList("");
}

function closeCmdPalette(opts) {
  if (!isCmdPaletteOpen()) return;
  setCmdPaletteOpen(false);
  const restore = opts?.restoreFocus !== false;
  if (restore && CMD_LAST_FOCUS && typeof CMD_LAST_FOCUS.focus === "function") {
    try { CMD_LAST_FOCUS.focus(); } catch {}
  }
  CMD_LAST_FOCUS = null;
}

function initCmdPaletteUi() {
  const modal = $("cmdPaletteModal");
  const input = $("cmdPaletteInput");
  const close = $("btnCmdPaletteClose");
  const backdrop = $("cmdPaletteBackdrop");
  if (!modal || !input || !close || !backdrop) return;

  close.addEventListener("click", () => closeCmdPalette());
  backdrop.addEventListener("click", () => closeCmdPalette());
  input.addEventListener("input", () => renderCmdPaletteList(input.value));

  input.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      closeCmdPalette();
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const list = $("cmdPaletteList");
      const items = list ? Array.from(list.querySelectorAll(".cmdItem")) : [];
      if (!items.length) return;
      CMD_SELECTED_IDX = e.key === "ArrowDown"
        ? Math.min(items.length - 1, CMD_SELECTED_IDX + 1)
        : Math.max(0, CMD_SELECTED_IDX - 1);
      renderCmdPaletteList(input.value);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const q = String(input.value || "");
      const actions = buildCommandActions();
      const tokens = String(q || "").trim().toLowerCase().split(/\s+/g).filter((t) => !!t);
      const filtered = tokens.length
        ? actions.map((a) => ({ a, s: cmdScore(a, tokens) })).filter((x) => x.s != null).sort((x, y) => x.s - y.s).map((x) => x.a)
        : actions;
      const a = filtered[CMD_SELECTED_IDX] || null;
      if (!a) return;
      if (!a.enabled()) return;
      try { a.run(); } catch {}
      closeCmdPalette({ restoreFocus: false });
      return;
    }
  });
}

function isTypingTarget(el) {
  const t = el && el.tagName ? String(el.tagName).toLowerCase() : "";
  if (t === "input" || t === "textarea" || t === "select") return true;
  return !!el && !!el.isContentEditable;
}

function isMultilineEditorTarget(el) {
  const t = el && el.tagName ? String(el.tagName).toLowerCase() : "";
  if (t === "textarea") return true;
  return !!el && !!el.isContentEditable;
}

function initGlobalShortcuts() {
  document.addEventListener("keydown", (e) => {
    if (e.defaultPrevented) return;
    const key = String(e.key || "");
    const lower = key.toLowerCase();
    const mod = e.ctrlKey || e.metaKey;

    if (key === "Escape") {
      if (isCmdPaletteOpen()) {
        e.preventDefault();
        closeCmdPalette();
        return;
      }
      if (RUNNING) {
        // Let modals close first; otherwise treat Esc as cancel.
        e.preventDefault();
        $("btnCancel")?.click();
        return;
      }
      // If a trade is selected in the drilldown chart, Esc clears the selection.
      const polygonOpen = !!$("polygonModal")?.classList?.contains("open");
      if (!polygonOpen && DRILL_SELECTED_TRADE_IDX >= 0) {
        const wrap = $("drillWrap");
        const visible = !!wrap && wrap.style.display !== "none";
        if (visible) {
          clearTradeSelection();
          applyTradeRowSelection();
        }
      }
      return;
    }

    if (isCmdPaletteOpen()) return;

    if (mod && lower === "k") {
      e.preventDefault();
      openCmdPalette();
      return;
    }

    if (mod && lower === "o" && !e.shiftKey) {
      e.preventDefault();
      if (RUNNING) return;
      setWorkspace("workflow", { save: true, focus: false, step: "data" });
      $("btnBrowseCsv")?.click();
      return;
    }

    if (mod && lower === "o" && e.shiftKey) {
      e.preventDefault();
      if (RUNNING) return;
      setWorkspace("workflow", { save: true, focus: false, step: "data" });
      $("btnPolygonOpen")?.click();
      return;
    }

    if (mod && key === "\\") {
      e.preventDefault();
      toggleControlsCollapsed();
      return;
    }

    if (mod && key === "Enter") {
      if (isMultilineEditorTarget(document.activeElement) && !e.shiftKey) return;
      e.preventDefault();
      if (RUNNING) return;
      if (!hasLoadedData()) return;
      setWorkspace("workflow", { save: true, focus: false, step: "optimize" });
      CONTROLS_TABS?.set("opt", { save: true, focus: false });
      $("btnRun")?.click();
      return;
    }

    if (mod && e.shiftKey && lower === "v") {
      e.preventDefault();
      if (RUNNING) return;
      setWorkspace("workflow", { save: true, focus: false, step: "validate" });
      $("btnValidate")?.click();
      return;
    }

    if (mod && e.shiftKey && lower === "c") {
      e.preventDefault();
      if (RUNNING) return;
      $("btnExportCsv")?.click();
      return;
    }

    if (mod && e.shiftKey && lower === "j") {
      e.preventDefault();
      if (RUNNING) return;
      $("btnExportJson")?.click();
      return;
    }

    if (mod && e.shiftKey && lower === "r") {
      e.preventDefault();
      if (RUNNING) return;
      $("btnOpenLatestReport")?.click();
      return;
    }

    // Drilldown trade navigation: J/K or ArrowDown/ArrowUp.
    // Avoid hijacking keystrokes while typing in inputs.
    if (!mod && !isTypingTarget(e.target)) {
      const curves = DRILL_LAST_CURVES && typeof DRILL_LAST_CURVES === "object" ? DRILL_LAST_CURVES : null;
      const trades = Array.isArray(curves?.trades) ? curves.trades : [];
      if (trades.length) {
        const wrap = $("drillWrap");
        const visible = !!wrap && wrap.style.display !== "none";
        if (visible) {
          const isPrev = lower === "k" || key === "ArrowUp";
          const isNext = lower === "j" || key === "ArrowDown";
          if (isPrev || isNext) {
            e.preventDefault();
            const total = trades.length;
            let idx = DRILL_SELECTED_TRADE_IDX;
            if (!Number.isFinite(Number(idx)) || idx < 0) idx = total - 1;
            const nextIdx = isNext ? Math.min(total - 1, idx + 1) : Math.max(0, idx - 1);
            selectDrillTradeByIndex(nextIdx, { scroll: true });
            return;
          }
        }
      }
    }
  }, true);
}

function pretty(obj) {
  try {
    return JSON.stringify(obj, null, 2);
  } catch {
    return String(obj);
  }
}

function fmtFixed(n, digits) {
  const x = Number(n);
  if (!Number.isFinite(x)) return "0";
  const d = Math.max(0, Math.min(12, Math.floor(Number(digits ?? 3))));
  return x.toFixed(d);
}

function formatCompositeParamsCompact(params) {
  const p = params && typeof params === "object" ? params : {};
  const ht = p.halftrend && typeof p.halftrend === "object" ? p.halftrend : {};
  const macz = p.macz && typeof p.macz === "object" ? p.macz : {};
  const ott = p.ott && typeof p.ott === "object" ? p.ott : {};
  const signalMode = String(p.signal_mode || p.signalMode || "").trim().toLowerCase();
  const parts = [];
  if (signalMode) {
    const modeLabel = signalMode === "trend_pullback_reversion"
      ? "TrendMR"
      : (signalMode === "entry_core_exit_overlay" ? "EntryCoreExitOverlay" : "HardGate");
    parts.push(`mode=${modeLabel}`);
  }
  if (ht && (ht.amplitude != null || ht.atr_period != null || ht.channel_deviation != null)) {
    parts.push(`HT(amp=${Number(ht.amplitude || 0)},atr=${Number(ht.atr_period || 0)},dev=${fmtFixed(ht.channel_deviation || 0, 2)})`);
  }
  if (macz && (macz.fast_length != null || macz.slow_length != null || macz.signal_length != null)) {
    parts.push(`MACZ(${Number(macz.fast_length || 0)}/${Number(macz.slow_length || 0)}/${Number(macz.signal_length || 0)})`);
  }
  if (ott && (ott.period != null || ott.percent != null || ott.ma_type != null)) {
    parts.push(`OTT(p=${Number(ott.period || 0)},pct=${fmtFixed(ott.percent || 0, 2)}${ott.ma_type ? `,${String(ott.ma_type || "").toUpperCase()}` : ""})`);
  }
  return parts.join(" ");
}

function readCompositeSignalMode() {
  const raw = String($("compositeSignalMode")?.value || "hard_gate").trim().toLowerCase();
  if (raw === "entry_core_exit_overlay") return raw;
  if (raw === "trend_pullback_reversion") return raw;
  return "hard_gate";
}

function compositeSignalModeSpec(modeRaw) {
  const mode = String(modeRaw || "hard_gate").trim().toLowerCase();
  if (mode === "entry_core_exit_overlay") {
    return {
      title: "Entry core + exit overlay",
      details:
        "Entry uses MA cross + HalfTrend direction. Exit uses overlay signals from MACZ/OTT or opposing regime shift.",
    };
  }
  if (mode === "trend_pullback_reversion") {
    return {
      title: "Trend pullback reversion",
      details:
        "Trend is defined by MA + HalfTrend. Entries buy/sell pullbacks in-trend; exits trigger on reversion completion or trend break (MACZ/OTT assist).",
    };
  }
  return {
    title: "Hard gate",
    details:
      "Most selective mode: both entry and exit require agreement from MA, HalfTrend, MACZ, and OTT filters.",
  };
}

function updateAggregateSummaryCards() {
  const set = (id, text) => {
    const el = $(id);
    if (!el) return;
    el.textContent = String(text || "—");
  };

  set("aggregateSummarySavedRuns", String(SAVED_RUNS.length));
  set("aggregateSummarySavedMeta", String(SAVED_META.length));

  const result = LAST_META_RUN?.result && typeof LAST_META_RUN.result === "object" ? LAST_META_RUN.result : null;
  if (!result) {
    set("aggregateSummaryLatestGroups", "—");
    set("aggregateSummaryLatestArtifacts", "—");
    set("aggregateSummaryLatestHoldout", "—");
    set("aggregateSummaryLatestRuntime", "—");
    return;
  }

  const groups = Array.isArray(result.groups) ? result.groups.length : 0;
  const used = Number(result.artifacts_used || 0);
  const considered = Number(result.artifacts_considered || 0);
  const holdoutDatasets = Number(result?.holdout?.datasets || 0);
  const holdoutSkipped = Number(result?.holdout?.skipped || 0);
  const runtimeMs = Number(result.runtime_ms || 0);

  set("aggregateSummaryLatestGroups", String(groups));
  set("aggregateSummaryLatestArtifacts", `${used}/${considered}`);
  set("aggregateSummaryLatestHoldout", holdoutDatasets ? `${holdoutDatasets}${holdoutSkipped ? ` (skip ${holdoutSkipped})` : ""}` : "0");
  set("aggregateSummaryLatestRuntime", runtimeMs >= 1000 ? `${fmtFixed(runtimeMs / 1000, 2)}s` : `${runtimeMs}ms`);
}

function renderMetaSelection(resp) {
  const outEl = $("metaOutput");
  const metaEl = $("metaMeta");
  const body = $("metaTableBody");

  const result = resp?.result || null;
  const groups = Array.isArray(result?.groups) ? result.groups : [];
  const holdoutDatasets = Number(result?.holdout?.datasets || 0);
  const holdoutSkipped = Number(result?.holdout?.skipped || 0);

  if (outEl) outEl.textContent = result ? pretty(result) : "-";

  if (metaEl) {
    const warn = Array.isArray(result?.warnings) && result.warnings.length ? ` · warn=${result.warnings.length}` : "";
    const holdout = holdoutDatasets ? ` · holdout=${holdoutDatasets}${holdoutSkipped ? ` (skipped ${holdoutSkipped})` : ""}` : "";
    const rankMode = groups.some((g) => Number.isFinite(Number(g?.final_score)))
      ? " · rank=holdout-adjusted"
      : "";
    metaEl.textContent = result
      ? `groups=${groups.length} · artifacts=${Number(result.artifacts_used || 0)}/${Number(result.artifacts_considered || 0)} · candidates=${Number(result.candidates_considered || 0)} · runtime=${Number(result.runtime_ms || 0)}ms${holdout}${rankMode}${warn}`
      : "—";
  }

  if (body) {
    body.innerHTML = "";
    for (let i = 0; i < groups.length; i++) {
      const g = groups[i] || {};
      const p = g.params && typeof g.params === "object" ? g.params : {};
      const fast = `${String(p.fast_ma_type || "").toUpperCase()} ${Number(p.fast_len || 0)}`.trim();
      const slow = `${String(p.slow_ma_type || "").toUpperCase()} ${Number(p.slow_len || 0)}`.trim();
      const paramsCompact = formatCompositeParamsCompact(p);
      const h = g.holdout && typeof g.holdout === "object" ? g.holdout : null;
      const ml = typeof g.ml_confidence === "number" && Number.isFinite(g.ml_confidence) ? g.ml_confidence : null;
      const finalScore = Number(g.final_score);
      const rankScore = Number.isFinite(finalScore) ? finalScore : Number(g.meta_score || 0);

      const tr = document.createElement("tr");
      const cells = [
        String(i + 1),
        String(Number(g.samples || 0)),
        fmtFixed(Number(g.pass_rate || 0) * 100, 1) + "%",
        fmtFixed(rankScore, 6),
        ml != null ? (fmtFixed(ml * 100, 1) + "%") : "-",
        fmtFixed(g.robust_score_p10 || 0, 6),
        fmtFixed(g.robust_score_median || 0, 6),
        fmtFixed(g.max_dd_p90 || 0, 6),
        fmtFixed(g.psr_median || 0, 4),
        fmtFixed(g.dsr_median || 0, 4),
        h ? (fmtFixed(Number(h.pass_rate || 0) * 100, 1) + "%") : "-",
        h ? fmtFixed(h.score_p10 || 0, 6) : "-",
        h ? fmtFixed(h.score_median || 0, 6) : "-",
        h ? fmtFixed(h.max_dd_p90 || 0, 6) : "-",
        fast,
        slow,
        paramsCompact || JSON.stringify(p || {}),
      ];
      for (let c = 0; c < cells.length; c++) {
        const td = document.createElement("td");
        td.textContent = cells[c];
        if (c === 14 || c === 15 || c === 16) td.className = "left";
        tr.appendChild(td);
      }
      body.appendChild(tr);
    }
  }

  const reportBtn = $("btnOpenMetaReport");
  const revealBtn = $("btnRevealMetaFiles");
  const reportPath = String(resp?.report_xlsx_path || "").trim();
  const metaPath = String(resp?.meta_json_path || "").trim();
  const prevMetaPath = String(LAST_META_RUN?.meta_json_path || "").trim();
  if (reportBtn) reportBtn.disabled = RUNNING || !reportPath;
  if (revealBtn) revealBtn.disabled = RUNNING || (!reportPath && !metaPath);
  LAST_META_RUN = resp || null;
  updateValidationExportButtons();
  if (!resp || metaPath !== prevMetaPath) {
    LAST_META_STAGE4_RUN = null;
    setMetaStage4Status("—");
    renderMetaStage4Output(null);
  }
  updateMetaMlButtons();
  updateMetaStage4Buttons();
  updateAggregateSummaryCards();
}

let LAST_META_ML_STATUS = null;

function metaMlEnabled() {
  return !!$("metaUseMlConfidence")?.checked;
}

function metaMlAutopilotEnabled() {
  return !!$("metaMlAutopilot")?.checked;
}

function metaMlScopeFromUi() {
  const result = LAST_META_RUN?.result || null;
  const strategy_kind = String(result?.strategy_kind || $("metaStrategyKind")?.value || "CompositeMo").trim() || "CompositeMo";
  let timeframe = String(result?.timeframe || "").trim();
  if (!timeframe) timeframe = String($("metaTimeframe")?.value || "").trim();
  return { strategy_kind, timeframe };
}

function setMetaMlStatus(text) {
  const el = $("metaMlStatus");
  if (!el) return;
  el.textContent = String(text || "—");
}

function updateMetaMlButtons() {
  const scope = metaMlScopeFromUi();
  const hasTf = !!String(scope.timeframe || "").trim();
  const hasMeta = !!LAST_META_RUN?.result;
  const hasModel = !!LAST_META_ML_STATUS?.exists && !LAST_META_ML_STATUS?.error;

  const btnApply = $("btnMetaMlApply");
  const btnTrain = $("btnMetaMlTrain");
  const btnRefresh = $("btnMetaMlRefresh");
  const btnDelete = $("btnMetaMlDelete");

  if (btnRefresh) btnRefresh.disabled = RUNNING || !hasTf;
  if (btnTrain) btnTrain.disabled = RUNNING || !hasTf;
  if (btnApply) btnApply.disabled = RUNNING || !hasTf || !hasMeta || !hasModel;
  if (btnDelete) btnDelete.disabled = RUNNING || !hasTf || !hasModel;
  updateMetaStage4Buttons();
}

function setMetaStage4Status(text) {
  const el = $("metaStage4Status");
  if (!el) return;
  el.textContent = String(text || "—");
}

function renderMetaStage4Output(resp) {
  const out = $("metaStage4Output");
  if (!out) return;
  const result = resp?.result && typeof resp.result === "object" ? resp.result : null;
  const groups = Array.isArray(result?.selected_groups) ? result.selected_groups : [];
  if (!result || !groups.length) {
    out.textContent = "-";
    return;
  }
  const lines = [];
  const dc = result?.decision_counts && typeof result.decision_counts === "object" ? result.decision_counts : {};
  lines.push(
    `scope=${String(result.strategy_kind || "")}/${String(result.timeframe || "")} ` +
    `selected=${Number(result.groups_selected || 0)} scored=${Number(result.groups_scored || 0)}/${Number(result.groups_considered || 0)} ` +
    `min_ml=${fmtFixed(Number(result.min_ml_confidence || 0), 3)}`
  );
  lines.push(
    `weights(meta/ml/unc)=${fmtFixed(Number(result.meta_weight || 0), 3)}/${fmtFixed(Number(result.ml_weight || 0), 3)}/${fmtFixed(Number(result.uncertainty_weight || 0), 3)} ` +
    `buckets(accept/review/reject)=${Number(dc.accept || 0)}/${Number(dc.review || 0)}/${Number(dc.reject || 0)}`
  );
  lines.push(
    `policy accept>=${fmtFixed(Number(result.accept_confidence || 0), 3)} review>=${fmtFixed(Number(result.review_confidence || 0), 3)} ` +
    `max_accept_unc=${fmtFixed(Number(result.max_accept_uncertainty || 0), 3)}`
  );
  for (let i = 0; i < groups.length; i++) {
    const g = groups[i] || {};
    const p = g.params && typeof g.params === "object" ? g.params : {};
    const conf = Number(g.ml_confidence);
    const confText = Number.isFinite(conf) ? `${fmtFixed(conf * 100, 1)}%` : "-";
    const unc = Number(g.ml_uncertainty);
    const uncText = Number.isFinite(unc) ? fmtFixed(unc, 3) : "-";
    const rank = Number(g.stage4_rank || (i + 1));
    const bucket = String(g.decision_bucket || "reject").trim().toLowerCase() || "reject";
    const fast = `${String(p.fast_ma_type || "").toUpperCase()} ${Number(p.fast_len || 0)}`.trim();
    const slow = `${String(p.slow_ma_type || "").toUpperCase()} ${Number(p.slow_len || 0)}`.trim();
    const finalRank = Number(g.final_rank_score);
    const rankScore = Number.isFinite(finalRank)
      ? finalRank
      : (Number.isFinite(Number(g.final_score)) ? Number(g.final_score) : Number(g.meta_score || 0));
    lines.push(`#${rank} ${bucket} ml=${confText} unc=${uncText} final=${fmtFixed(rankScore, 6)} ${fast} vs ${slow}`.trim());
  }
  const warns = Array.isArray(result.warnings) ? result.warnings : [];
  if (warns.length) lines.push(`warnings=${warns.length}: ${warns.join(" | ")}`);
  out.textContent = lines.join("\n");
}

function updateMetaStage4Buttons() {
  const hasTf = !!String(metaMlScopeFromUi().timeframe || "").trim();
  const hasMeta = !!LAST_META_RUN?.result;
  const btnRun = $("btnRunMetaStage4");
  const btnReveal = $("btnRevealMetaStage4");
  if (btnRun) btnRun.disabled = RUNNING || !hasTf || !hasMeta;
  if (btnReveal) btnReveal.disabled = RUNNING || !String(LAST_META_STAGE4_RUN?.stage4_json_path || "").trim();
}

async function runMetaStage4Experimental() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  if (!LAST_META_RUN?.result?.groups?.length) throw new Error("Run or load a meta analysis first.");
  const scope = metaMlScopeFromUi();
  if (!String(scope.timeframe || "").trim()) throw new Error("Timeframe required for Stage 4.");

  const top_k = Math.max(1, Math.floor(Number($("metaStage4TopK")?.value || 10)));
  const min_ml_confidence = Number($("metaStage4MinConf")?.value || 0.55);
  const meta_weight = Math.max(0, Number($("metaStage4MetaWeight")?.value || 0.60));
  const ml_weight = Math.max(0, Number($("metaStage4MlWeight")?.value || 0.35));
  const uncertainty_weight = Math.max(0, Number($("metaStage4UncertaintyWeight")?.value || 0.05));
  const accept_confidence = Number($("metaStage4AcceptConf")?.value || 0.70);
  const review_confidence = Number($("metaStage4ReviewConf")?.value || 0.55);
  const max_accept_uncertainty = Number($("metaStage4MaxAcceptUncertainty")?.value || 0.45);
  const auto_train_model = !!$("metaStage4AutoTrain")?.checked;
  const auto_retrain_stale = auto_train_model;
  const require_healthy_model = !!$("metaStage4RequireHealthy")?.checked;
  const fallback_to_meta_if_empty = !!$("metaStage4FallbackToMeta")?.checked;
  const source_meta_path = String(LAST_META_RUN?.meta_json_path || "").trim() || null;
  const req = {
    strategy_kind: String(scope.strategy_kind || "").trim() || "CompositeMo",
    timeframe: String(scope.timeframe || "").trim() || null,
    source_meta_path,
    top_k,
    min_ml_confidence,
    meta_weight,
    ml_weight,
    uncertainty_weight,
    accept_confidence,
    review_confidence,
    max_accept_uncertainty,
    auto_train_model,
    auto_retrain_stale,
    require_healthy_model,
    fallback_to_meta_if_empty,
  };

  const wasRunning = RUNNING;
  setRunning(true);
  setProgressUi(0, 1, "meta_stage4", "Preparing Stage 4 inputs...");
  setStatus("Running experimental Stage 4...", "muted");
  try {
    const resp = await invoke("run_meta_stage4_experimental", { req });
    LAST_META_STAGE4_RUN = resp || null;
    const res = resp?.result || {};
    const selected = Number(res?.groups_selected || 0);
    const scored = Number(res?.groups_scored || 0);
    const total = Number(res?.groups_considered || 0);
    const dc = res?.decision_counts && typeof res.decision_counts === "object" ? res.decision_counts : {};
    const path = String(resp?.stage4_json_path || "").trim();
    setMetaStage4Status(
      `Stage 4 complete: selected=${selected} scored=${scored}/${total} ` +
      `accept/review/reject=${Number(dc.accept || 0)}/${Number(dc.review || 0)}/${Number(dc.reject || 0)} ` +
      `· file=${path ? fileBasename(path) : "—"}`
    );
    renderMetaStage4Output(resp);
    updateMetaStage4Buttons();
    await refreshStage4Saved(path);
    setStatus(path ? `Stage 4 complete. Saved: ${path}` : "Stage 4 complete.", "ok");
  } finally {
    if (!wasRunning) clearProgressUi();
    setRunning(false);
  }
}

async function refreshMetaMlStatus() {
  const invoke = tauriInvoke();
  if (!invoke) return;
  const scope = metaMlScopeFromUi();
  if (!String(scope.timeframe || "").trim()) {
    LAST_META_ML_STATUS = null;
    setMetaMlStatus("Timeframe required (set Meta timeframe filter or run meta once).");
    updateMetaMlButtons();
    return;
  }

  try {
    const st = await invoke("ml_confidence_model_status", { scope });
    LAST_META_ML_STATUS = st;

    const availMeta = Number(st?.available_meta_artifacts || 0);
    const availRows = Number(st?.available_labeled_rows || 0);
    const newestAvail = st?.newest_meta_ts_available ? String(st.newest_meta_ts_available) : "";
    const stale = !!st?.stale;
    const retrain = !!st?.retrain_recommended;
    const retrainReason = st?.retrain_reason ? String(st.retrain_reason) : "";

    if (!st?.exists) {
      setMetaMlStatus(
        `No ML model saved for ${scope.strategy_kind} / ${scope.timeframe}. · available: meta=${availMeta}, rows=${availRows}${newestAvail ? ` · newest=${newestAvail}` : ""}`
      );
    } else if (st?.error) {
      setMetaMlStatus(
        `ML model exists but failed to load: ${String(st.error || "")} · available: meta=${availMeta}, rows=${availRows}${newestAvail ? ` · newest=${newestAvail}` : ""}`
      );
    } else {
      const label = st?.model?.label || {};
      const dd = label?.max_holdout_max_dd_p90 != null ? ` dd_p90<=${fmtFixed(label.max_holdout_max_dd_p90, 3)}` : "";
      const mm = st?.model?.meta || null;
      const healthy = mm?.healthy === false ? "UNHEALTHY" : (mm?.healthy === true ? "healthy" : "unknown");
      const target = mm?.target != null ? String(mm.target) : "";
      const trainedRows = mm?.trained_rows != null ? Number(mm.trained_rows || 0) : null;
      const trainedAt = mm?.trained_at_ms != null ? new Date(Number(mm.trained_at_ms || 0)).toLocaleString() : "";
      const auc = mm?.cv_auc != null ? Number(mm.cv_auc) : null;
      const brier = mm?.cv_brier_calibrated != null ? Number(mm.cv_brier_calibrated) : (mm?.cv_brier != null ? Number(mm.cv_brier) : null);
      const baseBrier = mm?.baseline_brier != null ? Number(mm.baseline_brier) : null;
      setMetaMlStatus(
        `Model: ${fileBasename(st.model_path)} · bytes=${Number(st.bytes || 0)} · ${healthy}` +
          (target ? ` · target=${target}` : "") +
          (trainedRows != null ? ` · trained_rows=${trainedRows}` : "") +
          (trainedAt ? ` · trained=${trainedAt}` : "") +
          (auc != null ? ` · cv_auc=${fmtFixed(auc, 3)}` : "") +
          (brier != null ? ` · cv_brier=${fmtFixed(brier, 4)}` : "") +
          (baseBrier != null ? ` · baseline=${fmtFixed(baseBrier, 4)}` : "") +
          ` · label: pass>=${fmtFixed(label.min_holdout_pass_rate || 0, 2)}, p10>=${fmtFixed(label.min_holdout_score_p10 || 0, 3)}, datasets>=${Number(label.min_holdout_datasets || 0)}${dd}` +
          ` · available: meta=${availMeta}, rows=${availRows}` +
          (newestAvail ? ` · newest=${newestAvail}` : "") +
          (stale ? " · stale" : " · fresh") +
          (retrain ? " · retrain recommended" : "") +
          (retrainReason ? ` · ${retrainReason}` : "")
      );
    }
  } catch (e) {
    LAST_META_ML_STATUS = null;
    setMetaMlStatus(`ML status failed: ${String(e)}`);
  } finally {
    updateMetaMlButtons();
  }
}

async function trainMetaMlModel(opts) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const scope = metaMlScopeFromUi();
  if (!String(scope.timeframe || "").trim()) throw new Error("Timeframe required for ML model training.");

  if (!opts?.silent) setStatus("Training ML confidence model...", "muted");
  const resp = await invoke("train_ml_confidence_model", { scope, cfg: null });
  await refreshMetaMlStatus();

  const m = resp.metrics || {};
  const auc = (m.cv_auc != null ? m.cv_auc : m.train_auc);
  const brier = (m.cv_brier_calibrated != null ? m.cv_brier_calibrated
    : (m.cv_brier != null ? m.cv_brier
      : (m.train_brier_calibrated != null ? m.train_brier_calibrated : m.train_brier)));
  const baseBrier = (m.baseline_brier != null ? m.baseline_brier : null);
  const health = (m.healthy === false ? "UNHEALTHY" : "healthy");
  const warnN = Array.isArray(m.warnings) ? m.warnings.length : 0;

  setMetaMlStatus(
    `Trained: rows=${Number(m.rows || 0)} · pos=${Number(m.positives || 0)} · neg=${Number(m.negatives || 0)} · ${health} · auc=${fmtFixed(Number(auc || 0), 3)} · brier=${fmtFixed(Number(brier || 0), 4)}` +
      (baseBrier != null ? ` · baseline=${fmtFixed(Number(baseBrier || 0), 4)}` : "") +
      `${warnN ? ` · warnings=${warnN}` : ""}`
  );

  if (!opts?.silent) {
    if (warnN) {
      // Keep the status bar clean; show details in the Meta panel.
      setStatus("ML model trained (with warnings).", "ok");
    } else {
      setStatus("ML model trained.", "ok");
    }
  }
  updateMetaMlButtons();
  return resp;
}

function sortMetaGroupsByMl(groups) {
  const gs = Array.isArray(groups) ? groups : [];
  gs.sort((a, b) => {
    const pa = typeof a?.ml_confidence === "number" ? a.ml_confidence : -1;
    const pb = typeof b?.ml_confidence === "number" ? b.ml_confidence : -1;
    return pb - pa;
  });
  return gs;
}

async function applyMetaMlConfidenceToLastMeta(opts) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const resp = LAST_META_RUN;
  const result = resp?.result || null;
  const groups = Array.isArray(result?.groups) ? result.groups : [];
  if (!result || !groups.length) throw new Error("Run or load a meta analysis first.");

  const scope = metaMlScopeFromUi();
  if (!String(scope.timeframe || "").trim()) throw new Error("Timeframe required for ML apply.");

  const out = await invoke("apply_ml_confidence_model", { scope, groups });
  const cs = Array.isArray(out?.confidences) ? out.confidences : [];

  for (let i = 0; i < groups.length; i++) {
    const c = cs[i];
    groups[i].ml_confidence = typeof c === "number" && Number.isFinite(c) ? c : null;
  }

  const healthy = LAST_META_ML_STATUS?.model?.meta?.healthy;
  if (metaMlEnabled() && healthy !== false) sortMetaGroupsByMl(groups);
  renderMetaSelection(resp);

  const ws = Array.isArray(out?.warnings) ? out.warnings : [];
  if (!opts?.silent) {
    if (ws.length) setStatus(`Applied ML confidence (warnings: ${ws.length}).`, "muted");
    else setStatus("Applied ML confidence.", "ok");
  }
  updateMetaMlButtons();
  return out;
}

async function maybeAutoTrainAndApplyMetaMl(opts) {
  if (!metaMlEnabled()) return;
  if (!metaMlAutopilotEnabled()) return;

  await refreshMetaMlStatus();
  const st0 = LAST_META_ML_STATUS;
  const hasModel = !!st0?.exists && !st0?.error;
  const needsRetrain = !!st0?.retrain_recommended;
  if (!hasModel || needsRetrain) {
    try {
      await trainMetaMlModel({ silent: true });
    } catch (e) {
      setMetaMlStatus(`Auto-train skipped: ${String(e)}`);
      updateMetaMlButtons();
      return;
    }
  }

  // Apply if we have a meta result loaded.
  if (LAST_META_RUN?.result?.groups?.length) {
    try {
      await applyMetaMlConfidenceToLastMeta({ silent: true });
    } catch {}
  }

  if (!opts?.silent) setStatus("ML confidence autopilot applied.", "muted");
}

async function runMetaSelection(opts) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  if (RUNNING && !opts?.allowWhileRunning) return;

  const strategy_kind = String($("metaStrategyKind")?.value || "CompositeMo").trim() || "CompositeMo";
  const supportsHoldout = strategy_kind === "CompositeCpu" || strategy_kind === "CompositeMo" || strategy_kind === "DoubleMa" || strategy_kind === "CompositeTrend" || strategy_kind === "RegimeHybrid" || strategy_kind === "BreakoutMomentum";
  const timeframeRaw = String($("metaTimeframe")?.value || "").trim();
  const requiredTagRaw = String($("metaRequiredTag")?.value || "").trim();
  const min_samples = Math.max(1, Math.floor(Number($("metaMinSamples")?.value || 30)));
  const top_k = Math.max(1, Math.floor(Number($("metaTopK")?.value || 25)));
  const holdoutTagRaw = String($("metaHoldoutTag")?.value || "").trim();
  const eval_holdout = supportsHoldout && !!$("metaEvalHoldout")?.checked;
  const holdoutModeRaw = String($("metaHoldoutMode")?.value || "light").trim() || "light";
  const holdout_mode = eval_holdout ? holdoutModeRaw : "light";
  const holdoutEvalTopN = Math.max(0, Math.floor(Number($("metaHoldoutEvalTopN")?.value || 0)));
  const holdoutExploreN = Math.max(0, Math.floor(Number($("metaHoldoutExploreN")?.value || 0)));
  const holdoutRankBy = String($("metaHoldoutEvalRankBy")?.value || "auto").trim() || "auto";
  const repsRaw = Math.floor(Number($("metaHoldoutReps")?.value || 2));
  const cluster_representatives = Math.min(3, Math.max(1, Number.isFinite(repsRaw) ? repsRaw : 2));
  const grouping_mode = String($("metaGroupingMode")?.value || "clustered").trim() || "clustered";
  const int_bucket = Math.max(1, Math.floor(Number($("metaClusterIntBucket")?.value || 5)));
  const floatBucketRaw = Number($("metaClusterFloatBucket")?.value || 0.25);
  const float_bucket = Number.isFinite(floatBucketRaw) ? Math.max(0, floatBucketRaw) : 0.25;

  const holdout_validation = (eval_holdout && holdout_mode === "heavy")
    ? (() => {
      // Heavy holdout: run iterative re-score plus walk-forward; perturb/MC follow Validation toggles.
      const seed = Math.max(0, Math.floor(num($("valSeed")?.value, 1)));
      const walk_forward = {
        window_bars: Math.max(64, Math.floor(num($("valWfWindow")?.value, 20000))),
        step_bars: Math.max(1, Math.floor(num($("valWfStep")?.value, 20000))),
      };
      const perturb = $("valPerturbEnable")?.checked
        ? {
          iterations: Math.max(0, Math.floor(num($("valPerturbIters")?.value, 8))),
          pct_min: Math.max(0, num($("valPerturbMin")?.value, 0.015)),
          pct_max: Math.max(0, num($("valPerturbMax")?.value, 0.06)),
          flip_bool_prob: 0.02,
        }
        : null;
      const trade_bootstrap = $("valMcEnable")?.checked
        ? { simulations: Math.max(0, Math.floor(num($("valMcSims")?.value, 500))) }
        : null;
      const validation_engine = "iterative";
      const iterative = {
        stop_loss_pct: null,
        take_profit_pct: null,
        intrabar_rule: "pessimistic",
        slippage_vol_mult: 0.15,
      };
      return { seed, walk_forward, perturb, trade_bootstrap, validation_engine, iterative };
    })()
    : null;

  const req = {
    strategy_kind,
    timeframe: timeframeRaw ? timeframeRaw : null,
    required_tag: requiredTagRaw ? requiredTagRaw : null,
    holdout_tag: eval_holdout && holdoutTagRaw ? holdoutTagRaw : null,
    eval_holdout,
    holdout_mode,
    holdout_validation,
    save_holdout_rows: eval_holdout && !!$("metaSaveHoldoutRows")?.checked,
    holdout_eval_top_n: eval_holdout && holdoutEvalTopN > 0 ? holdoutEvalTopN : null,
    holdout_eval_explore_n: eval_holdout ? holdoutExploreN : 0,
    holdout_eval_rank_by: eval_holdout ? holdoutRankBy : null,
    grouping_mode,
    cluster_config: { int_bucket, float_bucket },
    cluster_representatives,
    min_samples,
    top_k,
    strict_shortlist: true,
    allow_shortlist_fallback: false,
  };

  const wasRunning = RUNNING;
  setRunning(true);
  clearProgressUi();
  setActiveProgressContext(buildMetaProgressContext(req));
  setProgressUi(0, 1, "meta_read", "");
  setStatus("Running meta analysis...", "muted");
  try {
    const resp = await invoke("run_meta_selection", { req });
    LAST_META_RUN = resp;
    renderMetaSelection(resp);
    updateValidationExportButtons();
    await refreshMetaSaved(String(resp?.meta_json_path || "").trim());
    const path = String(resp?.report_xlsx_path || "").trim();
    setStatus(path ? `Meta complete. Saved report: ${path}` : "Meta complete.", "ok");
    await maybeAutoTrainAndApplyMetaMl({ silent: true });
    if (opts?.switchTab !== false) {
      RESULTS_TABS?.set("meta", { save: true, focus: false });
    }
  } finally {
    setActiveProgressContext(null);
    if (!wasRunning) clearProgressUi();
    setRunning(false);
  }
}

function isCancelledError(err) {
  const s = String(err || "").toLowerCase();
  return s.includes("cancelled") || s.includes("canceled");
}

function updateValidationExportButtons() {
  const hasRun = !!CURRENT_RUN?.res;
  const hasVal = !!CURRENT_VALIDATION?.res;
  const hasMetaXlsx = !!String(LAST_META_RUN?.report_xlsx_path || "").trim();
  const btnCsv = $("btnExportValidationCsv");
  const btnJson = $("btnExportValidationJson");
  const btnReport = $("btnExportReportJson");
  const btnXlsx = $("btnExportReportXlsx");
  if (btnCsv) btnCsv.disabled = RUNNING || !hasVal;
  if (btnJson) btnJson.disabled = RUNNING || !hasVal;
  if (btnReport) btnReport.disabled = RUNNING || !hasRun;
  if (btnXlsx) btnXlsx.disabled = RUNNING || !hasMetaXlsx;
}

function maParamSchema(maId) {
  const m = MA_BY_ID[maId];
  const params = m?.params;
  return Array.isArray(params) ? params : [];
}

function ensureMaParamState(side, maId) {
  if (!MA_PARAMS_STATE || typeof MA_PARAMS_STATE !== "object") MA_PARAMS_STATE = { fast: {}, slow: {} };
  if (!MA_PARAMS_STATE[side] || typeof MA_PARAMS_STATE[side] !== "object") MA_PARAMS_STATE[side] = {};
  if (!MA_PARAMS_STATE[side][maId] || typeof MA_PARAMS_STATE[side][maId] !== "object") MA_PARAMS_STATE[side][maId] = {};
  return MA_PARAMS_STATE[side][maId];
}

function ensureMaParamSweepState(side, maId) {
  if (!MA_PARAM_SWEEPS_STATE || typeof MA_PARAM_SWEEPS_STATE !== "object") MA_PARAM_SWEEPS_STATE = { fast: {}, slow: {} };
  if (!MA_PARAM_SWEEPS_STATE[side] || typeof MA_PARAM_SWEEPS_STATE[side] !== "object") MA_PARAM_SWEEPS_STATE[side] = {};
  if (!MA_PARAM_SWEEPS_STATE[side][maId] || typeof MA_PARAM_SWEEPS_STATE[side][maId] !== "object") MA_PARAM_SWEEPS_STATE[side][maId] = {};
  return MA_PARAM_SWEEPS_STATE[side][maId];
}

function resolvedMaParams(side, maId) {
  const schema = maParamSchema(maId);
  if (!schema.length) return null;
  const state = ensureMaParamState(side, maId);
  const out = {};
  for (const p of schema) {
    const k = String(p.key || "");
    if (!k) continue;
    const v = Number(state[k]);
    out[k] = Number.isFinite(v) ? v : Number(p.default || 0);
  }
  return out;
}

function resolvedMaParamSweeps(side, maId) {
  const schema = maParamSchema(maId);
  if (!schema.length) return null;
  const state = ensureMaParamSweepState(side, maId);
  const out = {};
  for (const p of schema) {
    const k = String(p.key || "");
    if (!k) continue;
    const cfg = state[k];
    if (!cfg || typeof cfg !== "object" || cfg.enabled !== true) continue;
    const start = Number(cfg.start);
    const end = Number(cfg.end);
    const step = Number(cfg.step);
    if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step)) continue;
    out[k] = [start, end, step];
  }
  return Object.keys(out).length ? out : null;
}

function renderMaParamForm(side, maId) {
  const wrap = $(side === "fast" ? "fastMaParamsWrap" : "slowMaParamsWrap");
  const body = $(side === "fast" ? "fastMaParamsBody" : "slowMaParamsBody");
  if (!wrap || !body) return;

  const schema = maParamSchema(maId);
  body.innerHTML = "";
  if (!schema.length) {
    wrap.style.display = "none";
    return;
  }
  wrap.style.display = "block";

  const state = ensureMaParamState(side, maId);
  const sweepState = ensureMaParamSweepState(side, maId);
  for (const p of schema) {
    const key = String(p.key || "");
    const labelText = String(p.label || key || "param");
    if (!key) continue;

    const id = `${side}_${maId}_param_${key}`;

    const block = document.createElement("div");
    block.style.marginTop = "0.6rem";

    const label = document.createElement("label");
    label.htmlFor = id;
    label.textContent = labelText;

    const input = document.createElement("input");
    input.id = id;
    input.type = "number";
    input.dataset.maParam = "1";
    input.dataset.maSide = side;
    input.dataset.maId = maId;
    input.dataset.maKey = key;
    if (p.min != null) input.min = String(p.min);
    if (p.max != null) input.max = String(p.max);
    if (p.step != null) input.step = String(p.step);
    else input.step = p.kind === "int" ? "1" : "any";

    const v = Number(state[key]);
    input.value = String(Number.isFinite(v) ? v : Number(p.default || 0));

    const onChange = () => {
      const x = Number(input.value);
      if (Number.isFinite(x)) state[key] = x;
      saveSettingsSoon();
    };
    input.addEventListener("change", onChange);
    input.addEventListener("input", onChange);

    block.appendChild(label);
    block.appendChild(input);

    if (!sweepState[key] || typeof sweepState[key] !== "object") sweepState[key] = {};
    const sweepCfg = sweepState[key];
    const curV = Number(input.value);
    const defStep = p.step != null ? Number(p.step) : (p.kind === "int" ? 1 : 0.1);
    if (!Number.isFinite(Number(sweepCfg.start))) sweepCfg.start = Number.isFinite(curV) ? curV : Number(p.default || 0);
    if (!Number.isFinite(Number(sweepCfg.end))) sweepCfg.end = Number.isFinite(curV) ? curV : Number(p.default || 0);
    if (!Number.isFinite(Number(sweepCfg.step))) sweepCfg.step = Number.isFinite(defStep) ? defStep : 0.1;

    const sweepWrap = document.createElement("div");
    sweepWrap.className = "checks";
    sweepWrap.style.marginTop = "0.35rem";

    const sweepLabel = document.createElement("label");
    const sweepToggle = document.createElement("input");
    sweepToggle.type = "checkbox";
    sweepToggle.checked = sweepCfg.enabled === true;
    sweepLabel.appendChild(sweepToggle);
    sweepLabel.appendChild(document.createTextNode(" Optimize (sweep)"));
    sweepWrap.appendChild(sweepLabel);

    const rangeRow = document.createElement("div");
    rangeRow.className = "row3";
    rangeRow.style.marginTop = "0.35rem";
    rangeRow.style.display = sweepToggle.checked ? "grid" : "none";

    const mkField = (suffix, title, value, min, max, step) => {
      const cell = document.createElement("div");
      const lab = document.createElement("label");
      const id2 = `${id}_${suffix}`;
      lab.htmlFor = id2;
      lab.textContent = title;
      lab.style.fontSize = "0.78rem";
      lab.style.opacity = "0.85";

      const inp = document.createElement("input");
      inp.id = id2;
      inp.type = "number";
      if (min != null) inp.min = String(min);
      if (max != null) inp.max = String(max);
      if (step != null) inp.step = String(step);
      inp.value = String(value);

      cell.appendChild(lab);
      cell.appendChild(inp);
      return { cell, inp };
    };

    const rangeMin = p.min != null ? Number(p.min) : null;
    const rangeMax = p.max != null ? Number(p.max) : null;
    const stepAttr = p.step != null ? Number(p.step) : (p.kind === "int" ? 1 : "any");

    const fStart = mkField("sweep_start", "Start", sweepCfg.start, rangeMin, rangeMax, stepAttr);
    const fEnd = mkField("sweep_end", "End", sweepCfg.end, rangeMin, rangeMax, stepAttr);
    const fStep = mkField("sweep_step", "Step", sweepCfg.step, 0, null, stepAttr);
    rangeRow.appendChild(fStart.cell);
    rangeRow.appendChild(fEnd.cell);
    rangeRow.appendChild(fStep.cell);

    const syncSweepCfg = () => {
      sweepCfg.start = Number(fStart.inp.value);
      sweepCfg.end = Number(fEnd.inp.value);
      sweepCfg.step = Number(fStep.inp.value);
      saveSettingsSoon();
    };
    fStart.inp.addEventListener("change", syncSweepCfg);
    fStart.inp.addEventListener("input", syncSweepCfg);
    fEnd.inp.addEventListener("change", syncSweepCfg);
    fEnd.inp.addEventListener("input", syncSweepCfg);
    fStep.inp.addEventListener("change", syncSweepCfg);
    fStep.inp.addEventListener("input", syncSweepCfg);

    sweepToggle.addEventListener("change", () => {
      sweepCfg.enabled = !!sweepToggle.checked;
      if (sweepCfg.enabled) {
        const nowV = Number(input.value);
        if (!Number.isFinite(Number(sweepCfg.start)) && Number.isFinite(nowV)) sweepCfg.start = nowV;
        if (!Number.isFinite(Number(sweepCfg.end)) && Number.isFinite(nowV)) sweepCfg.end = nowV;
        if (!Number.isFinite(Number(sweepCfg.step))) sweepCfg.step = Number.isFinite(defStep) ? defStep : 0.1;
      }
      rangeRow.style.display = sweepCfg.enabled ? "grid" : "none";
      saveSettingsSoon();
    });

    block.appendChild(sweepWrap);
    block.appendChild(rangeRow);

    const notes = String(p.notes || "");
    if (notes) {
      const note = document.createElement("div");
      note.className = "muted";
      note.style.marginTop = "0.25rem";
      note.style.fontSize = "0.8rem";
      note.textContent = notes;
      block.appendChild(note);
    }

    body.appendChild(block);
  }
}

function refreshMaParamsUi() {
  const fast = $("fastMaType")?.value || "";
  const slow = $("slowMaType")?.value || "";
  renderMaParamForm("fast", fast);
  renderMaParamForm("slow", slow);
}

function num(v, fallback) {
  const x = Number(v);
  return Number.isFinite(x) ? x : fallback;
}

const DEFAULT_STARTING_EQUITY = 10_000;

function normalizeStartingEquity(raw, fallback) {
  const x = Number(raw);
  const fb = Number.isFinite(Number(fallback)) && Number(fallback) > 0 ? Number(fallback) : DEFAULT_STARTING_EQUITY;
  if (!Number.isFinite(x) || x <= 0) return fb;
  return x;
}

function readStartingEquityFromUi() {
  return normalizeStartingEquity($("startingEquity")?.value, DEFAULT_STARTING_EQUITY);
}

function startingEquityFromReq(req) {
  const r = req && typeof req === "object" ? req : {};
  const direct = Number(r.starting_equity ?? r.startingEquity);
  if (Number.isFinite(direct) && direct > 0) return direct;

  const st = (r.strategy && typeof r.strategy === "object")
    ? r.strategy
    : ((r.base_strategy && typeof r.base_strategy === "object") ? r.base_strategy : null);
  const fromStrategy = Number(st?.starting_equity ?? st?.startingEquity);
  if (Number.isFinite(fromStrategy) && fromStrategy > 0) return fromStrategy;
  return DEFAULT_STARTING_EQUITY;
}

function finalEquityFromPnl(pnl, startingEquity) {
  const r = Number(pnl);
  const s = normalizeStartingEquity(startingEquity, DEFAULT_STARTING_EQUITY);
  if (!Number.isFinite(r)) return NaN;
  return s * (1 + r);
}

function fmtMoney(x, digits) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const d = Number.isFinite(digits) ? Math.max(0, Math.min(8, Math.floor(digits))) : 2;
  return n.toLocaleString(undefined, { minimumFractionDigits: d, maximumFractionDigits: d });
}

function parseRiskPctInput(id) {
  const el = $(id);
  if (!el) return null;
  const raw = num(el.value, NaN);
  if (!Number.isFinite(raw) || raw <= 0) return null;
  const frac = raw > 1 ? raw / 100.0 : raw;
  if (!Number.isFinite(frac) || frac <= 0 || frac >= 1) return null;
  return frac;
}

function fmt(x, digits) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const d = Number.isFinite(digits) ? digits : 4;
  return n.toFixed(d);
}

function expandRangeU16Like(start, end, step) {
  const s0 = Math.floor(Number(start));
  const e0 = Math.floor(Number(end));
  const st0 = Math.floor(Number(step));
  if (!Number.isFinite(s0) || !Number.isFinite(e0) || !Number.isFinite(st0)) return [];

  if (st0 === 0 || s0 === e0) return [s0];
  const st = Math.max(1, st0);
  const lo = Math.min(s0, e0);
  const hi = Math.max(s0, e0);
  const out = [];
  let v = lo;
  for (;;) {
    out.push(v);
    if (v === hi) break;
    const next = v + st;
    if (next <= v || next > hi) break;
    v = next;
  }
  return out;
}

function countPairsFastLtSlow(fastPeriods, slowPeriods) {
  const f = Array.isArray(fastPeriods) ? fastPeriods : [];
  const s = Array.isArray(slowPeriods) ? slowPeriods : [];
  if (!f.length || !s.length) return 0;

  const slowSorted = [...s].sort((a, b) => a - b);
  let total = 0;
  for (const ff of f) {
    let lo = 0;
    let hi = slowSorted.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (slowSorted[mid] <= ff) lo = mid + 1;
      else hi = mid;
    }
    total += slowSorted.length - lo;
  }
  return total;
}

function maRequiresVolumeField(maId) {
  const id = String(maId || "").trim().toLowerCase();
  return id === "vwma" || id === "vpwma";
}

function maRequiresHighLowField(maId) {
  const id = String(maId || "").trim().toLowerCase();
  return id === "frama";
}

function requiredFieldsForMaSource(source) {
  const s = String(source || "").trim().toLowerCase();
  if (s === "close") return ["close"];
  if (s === "open") return ["open"];
  if (s === "high") return ["high"];
  if (s === "low") return ["low"];
  if (s === "hl2") return ["high", "low"];
  if (s === "hlc3") return ["high", "low", "close"];
  if (s === "ohlc4") return ["open", "high", "low", "close"];
  if (s === "hlcc" || s === "hlcc4") return ["high", "low", "close"];
  return [];
}

function validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource) {
  if (!meta || typeof meta !== "object") return;

  const missing = new Set();
  const require = (field) => {
    if (field === "open" && !meta.has_open) missing.add("open");
    if (field === "high" && !meta.has_high) missing.add("high");
    if (field === "low" && !meta.has_low) missing.add("low");
    if (field === "close" && !meta.has_close) missing.add("close");
    if (field === "volume" && !meta.has_volume) missing.add("volume");
  };

  if (maRequiresVolumeField(fastMaType) || maRequiresVolumeField(slowMaType)) require("volume");
  if (maRequiresHighLowField(fastMaType) || maRequiresHighLowField(slowMaType)) {
    require("high");
    require("low");
  }
  for (const f of requiredFieldsForMaSource(maSource)) require(f);

  if (missing.size) {
    const parts = [];
    if (missing.has("volume")) parts.push("volume (needed by VWMA/VPWMA)");
    if (missing.has("high") || missing.has("low")) parts.push("high/low (needed by FRAMA and some sources)");
    if (missing.has("open")) parts.push("open (needed by some sources)");
    if (missing.has("close")) parts.push("close");
    throw new Error(`Loaded CSV is missing required candle fields: ${parts.join(", ")}.`);
  }
}

function fieldPresent(meta, field) {
  if (!meta || typeof meta !== "object") return false;
  if (field === "open") return !!meta.has_open;
  if (field === "high") return !!meta.has_high;
  if (field === "low") return !!meta.has_low;
  if (field === "close") return !!meta.has_close;
  if (field === "volume") return !!meta.has_volume;
  return false;
}

function renderDataMeta(meta) {
  const lenEl = $("dataLen");
  if (lenEl) lenEl.textContent = meta && Number.isFinite(Number(meta.len)) ? String(meta.len) : "—";

  const labelEl = $("datasetLabel");
  const rangeEl = $("datasetRange");
  const ctxEl = $("datasetContext");
  if (labelEl || rangeEl || ctxEl) {
    const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};
    const csvPath = String($("csvPath")?.value || "").trim();
    const label = String(ctx.label || "").trim() || fileBasename(csvPath) || "—";
    const timeframe = String(ctx.timeframe || meta?.timeframe || "").trim();
    const minTs = meta ? fmtYmdUtc(meta.min_ts) : "—";
    const maxTs = meta ? fmtYmdUtc(meta.max_ts) : "—";

    if (labelEl) labelEl.textContent = label || "—";
    if (rangeEl) {
      const parts = [];
      if (timeframe) parts.push(timeframe);
      if (minTs !== "—" || maxTs !== "—") parts.push(`${minTs} → ${maxTs}`);
      rangeEl.textContent = parts.length ? parts.join(" • ") : "—";
    }
    if (ctxEl) {
      const parts = [];
      const source = String(ctx.source || "").trim();
      if (source) parts.push(`source=${source}`);

      const market = String(ctx.market || "").trim();
      const symbol = String(ctx.symbol || "").trim();
      const adjusted = ctx.adjusted === true ? true : (ctx.adjusted === false ? false : null);
      if (market) {
        const m = market === "stocks" && adjusted != null ? `${market} (${adjusted ? "adj" : "raw"})` : market;
        parts.push(`market=${m}`);
      }
      if (symbol) parts.push(`symbol=${symbol}`);
      if (csvPath) parts.push(`file=${fileBasename(csvPath)}`);
      ctxEl.textContent = parts.length ? parts.join(" • ") : "—";
    }
  }

  const fieldsEl = $("dataFields");
  const hintEl = $("dataFieldHint");
  renderContextChips();
  updateRunPlanUi();
  if (!fieldsEl) {
    updateDataTimeframeUi();
    return;
  }

  fieldsEl.innerHTML = "";
  if (hintEl) hintEl.textContent = "";

  if (!meta || typeof meta !== "object") {
    fieldsEl.textContent = "—";
    updateDataTimeframeUi();
    return;
  }

  const fields = [
    ["open", !!meta.has_open],
    ["high", !!meta.has_high],
    ["low", !!meta.has_low],
    ["close", !!meta.has_close],
    ["volume", !!meta.has_volume],
  ];
  for (const [name, ok] of fields) {
    const badge = document.createElement("span");
    badge.className = `badge ${ok ? "on" : "off"}`;
    badge.textContent = name;
    fieldsEl.appendChild(badge);
  }

  const msgs = [];
  if (!meta.has_close) msgs.push("Missing close (required).");
  if (!meta.has_volume) msgs.push("No volume → VWMA/VPWMA disabled.");
  if (!meta.has_high || !meta.has_low) msgs.push("No high/low → FRAMA + some sources disabled.");
  if (!meta.has_open) msgs.push("No open → open/ohlc4 disabled.");
  if (hintEl) hintEl.textContent = msgs.join(" · ");

  updateDataTimeframeUi();
}

function updateDataTimeframeUi() {
  const sel = $("dataTimeframe");
  const btn = $("btnApplyTimeframe");
  const hint = $("dataTimeframeHint");
  if (!sel || !hint) return;

  const baseId = String(CURRENT_BASE_DATA_ID || "").trim();
  const hasBase = !!baseId && baseId !== "—" && baseId !== "-";

  if (!hasBase) {
    sel.value = "";
    sel.disabled = true;
    if (btn) btn.disabled = true;
    hint.textContent = "Load a dataset to enable timeframe resampling.";
    return;
  }

  sel.disabled = RUNNING;
  if (btn) btn.disabled = RUNNING;

  const baseMeta = CURRENT_BASE_DATA_META && typeof CURRENT_BASE_DATA_META === "object" ? CURRENT_BASE_DATA_META : null;
  const baseCtx = CURRENT_BASE_DATASET_CTX && typeof CURRENT_BASE_DATASET_CTX === "object" ? CURRENT_BASE_DATASET_CTX : {};
  const curMeta = CURRENT_DATA_META && typeof CURRENT_DATA_META === "object" ? CURRENT_DATA_META : null;
  const curCtx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};

  const baseTf = String(baseCtx.timeframe || baseMeta?.timeframe || "").trim();
  const curTf = String(curCtx.timeframe || curMeta?.timeframe || "").trim();

  const hasCurOpt = curTf && Array.from(sel.options || []).some((o) => String(o.value || "") === curTf);
  if (hasCurOpt && baseTf && curTf && curTf !== baseTf) sel.value = curTf;
  else sel.value = "";

  const baseLabel = baseTf || "unknown";
  const effLabel = curTf || baseTf || "unknown";
  const derived = baseTf && curTf && curTf !== baseTf;
  hint.textContent = derived
    ? `Using ${effLabel} derived from base ${baseLabel} (no extra downloads). Periods are in candles.`
    : `Base timeframe: ${baseLabel}. Choose a higher timeframe to derive candles (no extra downloads). Periods are in candles.`;
}

async function applySelectedDataTimeframe(opts) {
  if (RUNNING && !opts?.allowWhileRunning) return;
  const persist = opts?.persist !== false;

  const baseId = String(CURRENT_BASE_DATA_ID || "").trim();
  if (!baseId || baseId === "—" || baseId === "-") {
    throw new Error("Load a dataset first (missing base data id).");
  }

  const sel = $("dataTimeframe");
  if (!sel) return;
  const target = String(sel.value || "").trim();

  const baseMeta = CURRENT_BASE_DATA_META && typeof CURRENT_BASE_DATA_META === "object" ? CURRENT_BASE_DATA_META : null;
  const baseCtx = CURRENT_BASE_DATASET_CTX && typeof CURRENT_BASE_DATASET_CTX === "object" ? CURRENT_BASE_DATASET_CTX : {};
  const baseTf = String(baseCtx.timeframe || baseMeta?.timeframe || "").trim();

  // Revert to the raw loaded dataset (no backend call needed).
  if (!target || (baseTf && target === baseTf)) {
    CURRENT_DATA_META = baseMeta;
    CURRENT_DATASET_CTX = {
      ...baseCtx,
      timeframe: baseTf || null,
      base_timeframe: baseTf || null,
      derived_timeframe: baseTf || null,
      resample_applied: false,
      resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
      resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
    };
    clearDrillCaches({ clearBars: true });
    $("dataId").textContent = baseId;
    renderDataMeta(baseMeta);
    applyMaSourceConstraints();
    applyBackendMaConstraints();
    updateOverviewEmptyUi();
    if (persist) saveSettingsSoon();
    setStatus(baseTf ? `Using base timeframe (${baseTf}).` : "Using base timeframe.", "muted");
    return;
  }

  const invoke = tauriInvoke();
  if (!invoke) {
    throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  }

  setRunning(true);
  try {
    setStatus(`Resampling to ${target}...`, "muted");
    const resp = await invoke("resample_price_data", { baseDataId: baseId, timeframe: target });
    const id = String(resp?.id || "").trim();
    const meta = resp?.meta && typeof resp.meta === "object" ? resp.meta : null;
    const cacheHit = !!resp?.cache_hit;
    const effectiveTf = String(meta?.timeframe || target).trim() || target;

    CURRENT_DATA_META = meta;
    CURRENT_DATASET_CTX = {
      ...baseCtx,
      timeframe: effectiveTf,
      base_timeframe: baseTf || null,
      derived_timeframe: effectiveTf || null,
      resample_applied: !!(baseTf && effectiveTf && baseTf !== effectiveTf),
      resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
      resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
    };
    clearDrillCaches({ clearBars: true });
    $("dataId").textContent = id || "—";
    renderDataMeta(meta);
    applyMaSourceConstraints();
    applyBackendMaConstraints();
    updateOverviewEmptyUi();

    setStatus(cacheHit ? `Using ${target} (cached).` : `Using ${target}.`, "ok");
    $("output").textContent = pretty(resp);
    if (persist) saveSettingsSoon();
  } finally {
    setRunning(false);
  }
}

function applyMaSourceConstraints() {
  const sel = $("maSource");
  if (!sel) return;

  const meta = CURRENT_DATA_META;
  for (const opt of sel.options) {
    const required = requiredFieldsForMaSource(opt.value);
    let ok = true;
    if (meta) {
      for (const f of required) {
        if (!fieldPresent(meta, f)) {
          ok = false;
          break;
        }
      }
    }
    opt.disabled = !ok;
  }

  if (sel.value && sel.selectedOptions?.[0]?.disabled) {
    let picked = "";
    for (const opt of sel.options) {
      if (!opt.disabled) {
        picked = opt.value;
        break;
      }
    }
    if (picked) sel.value = picked;
  }
}

function downloadText(filename, text, mime) {
  const blob = new Blob([text], { type: mime || "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2500);
}

function downloadBase64(filename, b64, mime) {
  const raw = atob(String(b64 || ""));
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
  const blob = new Blob([bytes], { type: mime || "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 2500);
}

async function requestCancelCurrentRun() {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    await invoke("cancel_current_run");
  } catch {
    // Back-compat: older backend command name.
    await invoke("cancel_double_ma_optimization");
  }
}

function setRunning(running) {
  if (running) RUNNING_COUNT += 1;
  else RUNNING_COUNT = Math.max(0, RUNNING_COUNT - 1);
  RUNNING = RUNNING_COUNT > 0;
  const btnRun = $("btnRun");
  const btnCancel = $("btnCancel");
  const btnDistCancel = $("btnDistCancel");
  if (btnRun) btnRun.disabled = RUNNING;
  if (btnCancel) btnCancel.disabled = !RUNNING;
  if (btnDistCancel) btnDistCancel.disabled = !RUNNING;

  const ids = [
    "csvPath",
    "btnBrowseCsv",
    "btnPolygonOpen",
    "dataTimeframe",
    "btnApplyTimeframe",
    "marketDataDir",
    "btnBrowseMarketDataDir",
    "btnSetMarketDataDir",
    "btnResetMarketDataDir",
    "btnOpenMarketDataDir",
    "marketDataMoveExisting",
    "btnLoad",
    "btnClear",
    "polygonApiKey",
    "btnPolygonSaveKey",
    "btnPolygonClearKey",
    "polygonMarket",
    "polygonStocksAdjusted",
    "polygonSymbol",
    "polygonTimeframe",
    "polygonFrom",
    "polygonTo",
    "btnPolygonDownloadLoad",
    "strategyKind",
    "presetProfile",
    "btnApplyPreset",
    "backend",
    "deviceId",
    "btnPresetKernelSmaAlma",
    "btnRefreshGpuStatus",
    "btnGpuParityCheck",
    "fastStart",
    "fastEnd",
    "fastStep",
    "slowStart",
    "slowEnd",
    "slowStep",
    "fastMaType",
    "slowMaType",
    "maSource",
    "objective",
    "mode",
    "vdpsEta",
    "vdpsRungs",
    "vdpsMinInitialPairs",
    "vdpsMaxSurvivorSeeds",
    "vdpsMaxPeriodCandidates",
    "vdpsRandomInjectionRatio",
    "vdpsEliteMutationRatio",
    "vdpsDifferentialRatio",
    "vdpsDeterministic",
    "vdpsSeed",
    "vdpsQualityCheckMaxPairs",
    "vdpsQualityCheckTopK",
    "topK",
    "includeAll",
    "exportAllCsvPath",
    "btnBrowseExportAll",
    "heatmapBins",
    "commission",
    "epsRel",
    "stopLossPct",
    "takeProfitPct",
    "trailingStopPct",
    "cooldownBars",
    "signalPersistenceBars",
    "dmConstraintMinTrades",
    "dmConstraintMaxDd",
    "exitMode",
    "kaufmanStopPeriod",
    "kaufmanStopMult",
    "kaufmanStopMaType",
    "trendLrPeriod",
    "trendLrMinDeg",
    "trendBbwPeriod",
    "trendBbwMult",
    "trendBbwMin",
    "trendRocPeriod",
    "trendUseRoc",
    "trendUseAdx",
    "trendAdxPeriod",
    "trendAdxMin",
    "regimeAdxPeriod",
    "regimeAdxEnter",
    "regimeAdxExit",
    "regimeRevEntryRel",
    "regimeRevExitRel",
    "breakoutBbPeriod",
    "breakoutBbMult",
    "breakoutConfirmBars",
    "breakoutMacdFast",
    "breakoutMacdSlow",
    "breakoutMacdSignal",
    "breakoutCmoPeriod",
    "breakoutCmoMinAbs",
    "breakoutAdoscShort",
    "breakoutAdoscLong",
    "breakoutUseAdx",
    "breakoutAdxPeriod",
    "breakoutAdxMin",
    "execMode",
    "execSymbol",
    "execQty",
    "execParamSource",
    "execManualFastLen",
    "execManualSlowLen",
    "execRiskStopLossPct",
    "execRiskTakeProfitPct",
    "execApiKey",
    "execApiSecret",
    "execDryRun",
    "execRememberKeys",
    "execLiveUseDelayed",
    "btnExecTestConnection",
    "btnExecClearKeys",
    "btnExecRunOnce",
    "btnExecRefresh",
    "btnExecLiveStart",
    "btnExecLiveStop",
    "btnExecRunnerStart",
    "btnExecRunnerStop",
    "longOnly",
    "allowFlip",
    "tradeOnNext",
    "compositeSignalMode",
    "halfTrendAmplitude",
    "halfTrendAtrPeriod",
    "halfTrendChannelDev",
    "maczFastLength",
    "maczSlowLength",
    "maczSignalLength",
    "maczLengthZ",
    "maczLengthStdev",
    "maczGamma",
    "maczA",
    "maczB",
    "maczUseVolume",
    "maczUseLag",
    "ottPeriod",
    "ottPercent",
    "ottMaType",
    "moObjectiveMin",
    "moSampler",
    "moNsga2Seed",
    "moNsga2Population",
    "moNsga2Generations",
    "moNsga2CrossoverProb",
    "moNsga2SwappingProb",
    "moNsga2MutationProb",
    "moRandomSeed",
    "moRandomTrials",
    "moTpeSeed",
    "moTpeTrials",
    "moTpeStartupTrials",
    "moTpeCandidates",
    "moTpeGamma",
    "moTpeBatchSize",
    "moTpeMaxModelPoints",
    "moQmcSeed",
    "moQmcTrials",
    "moQmcSequence",
    "moConstraintMinTrades",
    "moConstraintMaxDd",
    "moConstraintMaxAbsExposure",
    "moPrescanEnable",
    "moPrescanRequireGpu",
    "moPrescanTopK",
    "moPrescanWindow",
    "moPrescanPredictiveTopK",
    "moPrescanPredictiveStride",
    "moPrescanObjectives",
    "moPrescanMode",
    "moPrescanMaPairs",
    "moHtAmpLow",
    "moHtAmpHigh",
    "moHtAmpStep",
    "moHtDevLow",
    "moHtDevHigh",
    "moHtDevStep",
    "moHtAtrLow",
    "moHtAtrHigh",
    "moHtAtrStep",
    "moMaczFastLow",
    "moMaczFastHigh",
    "moMaczFastStep",
    "moMaczSlowLow",
    "moMaczSlowHigh",
    "moMaczSlowStep",
    "moMaczSignalLow",
    "moMaczSignalHigh",
    "moMaczSignalStep",
    "moMaczLengthZLow",
    "moMaczLengthZHigh",
    "moMaczLengthZStep",
    "moMaczLengthStdevLow",
    "moMaczLengthStdevHigh",
    "moMaczLengthStdevStep",
    "moMaczGammaLow",
    "moMaczGammaHigh",
    "moMaczGammaStep",
    "moMaczALow",
    "moMaczAHigh",
    "moMaczAStep",
    "moMaczBLow",
    "moMaczBHigh",
    "moMaczBStep",
    "moMaczUseLagMode",
    "moMaczUseVolumeMode",
    "moOttPeriodLow",
    "moOttPeriodHigh",
    "moOttPeriodStep",
    "moOttPercentLow",
    "moOttPercentHigh",
    "moOttPercentStep",
    "moOttMaTypes",
    "moSearchSpace",
    "btnMoResetSearchSpace",
    "valCandidates",
    "valSeed",
    "valAutoRun",
    "valAutoExportXlsx",
    "valAutoRevealReport",
    "valAutoExportTopN",
    "valWfEnable",
    "valWfWindow",
    "valWfStep",
    "valPerturbEnable",
    "valPerturbIters",
    "valPerturbMin",
    "valPerturbMax",
    "valMcEnable",
    "valMcSims",
    "valCscvEnable",
    "valCscvSlices",
    "valCrossEnable",
    "valCrossMax",
    "valCrossSelected",
    "valCrossSearch",
    "btnCrossRefresh",
    "btnCrossSelectAll",
    "btnCrossSelectNone",
    "btnCrossSelectMatch",
    "btnValidate",
    "btnOpenReportsDir",
    "btnOpenLatestReport",
    "btnOpenLatestMetaReport",
    "savedRunsSearch",
    "savedRunsSelect",
    "savedRunsTags",
    "btnSaveSavedRunTags",
    "btnRefreshSavedRuns",
    "btnLoadSavedRun",
    "btnApplySavedRun",
    "btnRerunSavedRun",
    "btnOpenSavedRunReport",
    "btnRevealSavedRun",
    "btnDeleteSavedRun",
    "btnOpenRunsDir",
    "metaStrategyKind",
    "metaTimeframe",
    "metaRequiredTag",
    "metaMinSamples",
    "metaTopK",
    "metaHoldoutTag",
    "metaEvalHoldout",
    "metaHoldoutMode",
    "metaHoldoutReps",
    "metaGroupingMode",
    "metaClusterIntBucket",
    "metaClusterFloatBucket",
    "btnRunMeta",
    "btnOpenMetaReport",
    "btnRevealMetaFiles",
    "metaStage4TopK",
    "metaStage4MinConf",
    "metaStage4MetaWeight",
    "metaStage4MlWeight",
    "metaStage4UncertaintyWeight",
    "metaStage4AcceptConf",
    "metaStage4ReviewConf",
    "metaStage4MaxAcceptUncertainty",
    "metaStage4AutoTrain",
    "metaStage4RequireHealthy",
    "metaStage4FallbackToMeta",
    "btnRunMetaStage4",
    "btnRevealMetaStage4",
    "metaSavedSearch",
    "metaSavedSelect",
    "metaSavedTags",
    "btnSaveMetaSavedTags",
    "btnRefreshMetaSaved",
    "btnLoadMetaSaved",
    "btnOpenMetaSavedReport",
    "btnRevealMetaSaved",
    "btnDeleteMetaSaved",
    "stage4SavedSearch",
    "stage4SavedSelect",
    "stage4SavedTags",
    "btnSaveStage4SavedTags",
    "btnRefreshStage4Saved",
    "btnLoadStage4Saved",
    "btnRevealStage4Saved",
    "btnDeleteStage4Saved",
    "metaCompareSelect",
    "batchDatasetSearch",
    "batchTags",
    "batchHoldoutPct",
    "batchHoldoutTag",
    "batchSkipDone",
    "btnRunBatch",
    "btnBatchRefresh",
    "btnBatchSelectAll",
    "btnBatchSelectNone",
    "btnBatchRetryFailed",
    "btnBatchRevealSummary",
    "distWorkerBindAddr",
    "distWorkerToken",
    "btnDistGenToken",
    "btnDistStartWorker",
    "btnDistStopWorker",
    "btnDistWorkerStatus",
    "distLeaderToken",
    "distLeaderWorkers",
    "distLeaderMode",
    "distAssetsCsvs",
    "distPathFrom",
    "distPathTo",
    "btnDistFillFromPolygon",
    "btnDistRunAssets",
    "distTileCsvPath",
    "btnDistUseCurrentCsv",
    "distTilesPerWorker",
    "btnDistRunTiles",
  ];
  for (const id of ids) {
    if (id === "btnRun" || id === "btnCancel") continue;
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING;
  }
  for (const el of document.querySelectorAll("input[data-ma-param]")) {
    el.disabled = RUNNING;
  }
  for (const el of document.querySelectorAll("#sampleCsvs button")) {
    el.disabled = RUNNING;
  }
  for (const el of document.querySelectorAll("#polygonDatasets button")) {
    el.disabled = RUNNING;
  }
  for (const el of document.querySelectorAll("#valCrossDatasets input[type=checkbox]")) {
    el.disabled = RUNNING;
  }
  for (const el of document.querySelectorAll("#batchDatasets input[type=checkbox]")) {
    el.disabled = RUNNING;
  }
  if (running) {
    const wrap = $("progressWrap");
    const fill = $("progressFill");
    if (wrap) wrap.style.display = "block";
    if (fill) fill.style.width = "0%";
    const meta = $("progressMeta");
    if (meta) meta.textContent = "";
  } else if (!RUNNING) {
    clearProgressUi();
  }
  updateStrategyUi();
  updateOverviewEmptyUi();
  updateTopTableNoRunUi();
  updateValidationExportButtons();
  syncSavedRunSelectionUi();
  syncMetaSavedSelectionUi();
  syncStage4SavedSelectionUi();
  updateMetaMlButtons();
  updateMetaStage4Buttons();
  syncBatchSummaryButtons();
  syncSettingsWorkspaceUi();
  setExecBusy(EXEC_BUSY);
}

function maSupportsKernel(id) {
  const info = MA_BY_ID[id];
  return info ? !!info.supports_cuda_kernel : false;
}

function maSupportsSweep(id) {
  const info = MA_BY_ID[id];
  return info ? !!info.supports_cuda_ma_sweep : false;
}

function maSupportsCpu(id) {
  const info = MA_BY_ID[id];
  return info ? !!info.supports_cpu : false;
}

function maSupportsCuda(id) {
  const info = MA_BY_ID[id];
  return info ? !!info.supports_cuda : false;
}

function maSelectableForBackend(id, backend) {
  const info = MA_BY_ID[id];
  if (!info) return false;
  if (!info.period_based || !info.single_output) return false;

  const b = String(backend || "CpuOnly");
  if (b === "CpuOnly") return maSupportsCpu(id);
  if (b === "GpuOnly") return maSupportsCuda(id);
  if (b === "GpuKernel") return maSupportsKernel(id);
  if (b === "Auto") return maSupportsCpu(id) || maSupportsCuda(id) || maSupportsKernel(id);
  return maSupportsCpu(id);
}

function applyBackendMaConstraints() {
  const backend = $("backend")?.value || "CpuOnly";
  const meta = CURRENT_DATA_META;
  const fastSel = $("fastMaType");
  const slowSel = $("slowMaType");
  if (!fastSel || !slowSel) return;

  for (const sel of [fastSel, slowSel]) {
    for (const opt of sel.options) {
      const id = opt.value;
      const info = MA_BY_ID[id];
      const baseOk = info && !!info.period_based && !!info.single_output;
      let dataOk = true;
      if (meta) {
        if (maRequiresVolumeField(id) && !meta.has_volume) dataOk = false;
        if (maRequiresHighLowField(id) && (!meta.has_high || !meta.has_low)) dataOk = false;
      }
      opt.disabled = !(baseOk && maSelectableForBackend(id, backend) && dataOk);
    }

    if (sel.value && sel.selectedOptions?.[0]?.disabled) {
      let picked = "";
      for (const opt of sel.options) {
        if (!opt.disabled) {
          picked = opt.value;
          break;
        }
      }
      if (picked) sel.value = picked;
    }
  }
  refreshMaParamsUi();
}

async function initMaSelects() {
  const invoke = tauriInvoke();
  if (!invoke) return;

  try {
    MA_LIST = await invoke("list_moving_averages");
    MA_LIST = Array.isArray(MA_LIST) ? MA_LIST : [];
    MA_BY_ID = {};
    for (const m of MA_LIST) MA_BY_ID[m.id] = m;

    const fastSel = $("fastMaType");
    const slowSel = $("slowMaType");
    if (!fastSel || !slowSel) return;

    const prevFast = (fastSel.value || "").trim();
    const prevSlow = (slowSel.value || "").trim();

    const maRank = (m) => {
      if (m && m.prefers_cuda_kernel) return 0;
      if (m && m.supports_cuda_kernel) return 1;
      return 2;
    };
    const mas = [...MA_LIST].sort((a, b) => {
      const ra = maRank(a);
      const rb = maRank(b);
      if (ra !== rb) return ra - rb;
      return (a.label || a.id).localeCompare(b.label || b.id);
    });
    for (const sel of [fastSel, slowSel]) {
      sel.innerHTML = "";
      for (const m of mas) {
        const opt = document.createElement("option");
        opt.value = m.id;
        const isOk = !!m.period_based && !!m.single_output;
        opt.disabled = !isOk;
        if (isOk) {
          const gpuTag = m.prefers_cuda_kernel
            ? " [GPU preferred]"
            : (m.supports_cuda_kernel ? " [GPU kernel]" : "");
          opt.textContent = `${m.label} (${m.id})${gpuTag}`;
        } else {
          opt.textContent = `${m.label} (${m.id}) [not supported]`;
        }
        sel.appendChild(opt);
      }
    }

    const isOk = (id) => {
      const m = MA_BY_ID[id];
      return m && !!m.period_based && !!m.single_output;
    };
    const defaultId = mas.find((m) => m.id === "sma") ? "sma" : (mas[0]?.id || "");
    fastSel.value = isOk(prevFast) ? prevFast : defaultId;
    slowSel.value = isOk(prevSlow) ? prevSlow : defaultId;
    applyBackendMaConstraints();
    refreshMaParamsUi();
    refreshGpuStatusUi();

    fastSel.addEventListener("change", () => {
      refreshMaParamsUi();
      refreshGpuStatusUi();
      saveSettingsSoon();
    });
    slowSel.addEventListener("change", () => {
      refreshMaParamsUi();
      refreshGpuStatusUi();
      saveSettingsSoon();
    });
  } catch (e) {
    setStatus(`Failed to load MA list: ${String(e)}`, "danger");
  }
}

function clearProgressUi() {
  PROGRESS_ETA_BY_PHASE = {};
  PROGRESS_EVENT_SEQ = 0;
  const wrap = $("progressWrap");
  const fill = $("progressFill");
  const meta = $("progressMeta");
  const detail = $("progressDetail");
  if (fill) fill.style.width = "0%";
  if (fill) fill.classList.remove("indeterminate");
  if (meta) meta.textContent = "";
  if (detail) detail.textContent = "";
  if (wrap) wrap.style.display = "none";
  ACTIVE_PROGRESS_CONTEXT = null;
}

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
  "export_xlsx": "Open Stage 3 XLSX",
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
  "export_xlsx": "Opening Stage 3 meta XLSX report.",
  "reveal_report": "Opening report location in file explorer.",
};

const PHASE_GROUP_LABELS = {
  optimize: "Optimize",
  validate: "Validate",
  meta: "Meta",
  export: "Export",
  reveal: "Reveal",
};

var PROGRESS_ETA_BY_PHASE = {};
var PROGRESS_EVENT_SEQ = 0;
const OPT_PROGRESS_EVENT_STALE_MS = 2600;

function extractSearchSpaceIntRange(searchSpace, name) {
  const space = searchSpace && typeof searchSpace === "object" ? searchSpace : null;
  const params = Array.isArray(space?.params) ? space.params : [];
  const target = String(name || "").trim().toLowerCase();
  if (!target) return null;
  for (const p0 of params) {
    const p = p0 && typeof p0 === "object" ? p0 : null;
    if (!p) continue;
    const n = String(p.name || "").trim().toLowerCase();
    if (n !== target) continue;
    const d = p.domain && typeof p.domain === "object" ? p.domain : null;
    const r = d?.IntRange && typeof d.IntRange === "object" ? d.IntRange : null;
    if (!r) return null;
    const low = Number(r.low);
    const high = Number(r.high);
    const step = Number(r.step ?? 1);
    if (!Number.isFinite(low) || !Number.isFinite(high) || !Number.isFinite(step)) return null;
    return { low, high, step };
  }
  return null;
}

function countParamSweepVariants(sweeps) {
  const sw = sweeps && typeof sweeps === "object" ? sweeps : null;
  if (!sw) return 1;
  let total = 1;
  for (const v of Object.values(sw)) {
    const arr = Array.isArray(v) ? v : null;
    if (!arr || arr.length < 3) continue;
    total *= countF64RangeValues(arr[0], arr[1], arr[2]);
    if (total > 1_000_000) return 1_000_000;
  }
  return Math.max(1, Math.floor(total));
}

function estimateOptimizationWorkLabel(req) {
  const kindRaw = String(req?.strategy_kind || req?.strategyKind || "DoubleMa").trim();
  const kind = normalizeStrategyKindRaw(kindRaw);

  if (kind === "CompositeMo") {
    const sampler = req?.sampler && typeof req.sampler === "object" ? req.sampler : null;
    const cfg = sampler?.config && typeof sampler.config === "object" ? sampler.config : null;
    const samplerKind = String(sampler?.sampler || "Nsga2").trim() || "Nsga2";
    let evals = 0;
    if (samplerKind === "Nsga2") {
      const pop = Math.max(0, Math.floor(Number(cfg?.population_size ?? cfg?.population ?? 0)));
      const gens = Math.max(0, Math.floor(Number(cfg?.generations ?? 0)));
      evals = pop > 0 ? (pop * (gens + 1)) : 0;
    } else {
      evals = Math.max(0, Math.floor(Number(cfg?.trials ?? 0)));
    }

    const parts = [];
    if (evals > 0) parts.push(`${samplerKind} ~${formatCompactNumber(evals)} evals`);
    else parts.push(`${samplerKind} sampler`);

    const gp = req?.gpu_prescan && typeof req.gpu_prescan === "object" ? req.gpu_prescan : null;
    if (gp) {
      const fastR = extractSearchSpaceIntRange(req?.search_space, "fast_len");
      const slowR = extractSearchSpaceIntRange(req?.search_space, "slow_len");
      const fast = fastR ? expandRangeU16Like(fastR.low, fastR.high, fastR.step) : [];
      const slow = slowR ? expandRangeU16Like(slowR.low, slowR.high, slowR.step) : [];
      const pairs = countPairsFastLtSlow(fast, slow);
      const maPairsN = Array.isArray(gp.ma_pairs) && gp.ma_pairs.length > 0 ? gp.ma_pairs.length : 1;
      const prescanPairs = pairs > 0 ? (pairs * maPairsN) : 0;
      if (prescanPairs > 0) parts.push(`pre-scan ~${formatCompactNumber(prescanPairs)} pairs`);
      else parts.push("pre-scan enabled");
      const predictiveTopK = Math.max(0, Math.floor(Number(gp.predictive_top_k ?? 0)));
      if (predictiveTopK > 0) {
        const predictiveStride = Math.max(1, Math.floor(Number(gp.predictive_stride ?? 8)));
        parts.push(`predictive rescue K=${predictiveTopK} stride=${predictiveStride}`);
      }
    }

    return parts.join(" · ");
  }

  const fastR = Array.isArray(req?.fast_range) ? req.fast_range : [];
  const slowR = Array.isArray(req?.slow_range) ? req.slow_range : [];
  if (fastR.length < 3 || slowR.length < 3) return "";

  const fast = expandRangeU16Like(fastR[0], fastR[1], fastR[2]);
  const slow = expandRangeU16Like(slowR[0], slowR[1], slowR[2]);
  const pairs = countPairsFastLtSlow(fast, slow);
  if (pairs <= 0) return "";

  const fastTypeN = Array.isArray(req?.fast_ma_types) ? Math.max(1, req.fast_ma_types.length) : 1;
  const slowTypeN = Array.isArray(req?.slow_ma_types) ? Math.max(1, req.slow_ma_types.length) : 1;
  const fastSweepN = countParamSweepVariants(req?.fast_ma_param_sweeps);
  const slowSweepN = countParamSweepVariants(req?.slow_ma_param_sweeps);
  const variants = Math.max(1, fastTypeN * slowTypeN * fastSweepN * slowSweepN);
  const combos = Math.max(0, Math.floor(pairs * variants));
  if (combos > pairs) return `~${formatCompactNumber(combos)} combos (${formatCompactNumber(pairs)} MA pairs)`;
  return `~${formatCompactNumber(pairs)} MA pairs`;
}

function compactProgressText(text, maxChars) {
  const s = String(text || "").trim();
  const max = Math.max(64, Math.floor(Number(maxChars || 360)));
  if (s.length <= max) return s;
  return `${s.slice(0, Math.max(1, max - 1)).trimEnd()}…`;
}

function backendProgressLabel(backendRaw) {
  const b = String(backendRaw || "CpuOnly").trim() || "CpuOnly";
  if (b === "GpuKernel") return "GPU kernel";
  if (b === "GpuOnly") return "GPU sweep";
  if (b === "Auto") return "Auto backend";
  return "CPU";
}

function samplerProgressLabel(samplerRaw) {
  const s = String(samplerRaw || "Nsga2").trim() || "Nsga2";
  if (s === "Nsga2") return "NSGA-II";
  if (s === "Tpe") return "TPE";
  if (s === "Qmc") return "QMC";
  if (s === "Random") return "Random";
  return s;
}

function formatRiskProgressSummary(strategy) {
  const st = strategy && typeof strategy === "object" ? strategy : null;
  if (!st) return "";
  const sl = Number(st.stop_loss_pct);
  const tp = Number(st.take_profit_pct);
  const trail = Number(st.trailing_stop_pct);
  const cooldown = Math.max(0, Math.floor(num(st.cooldown_bars, 0)));
  const persist = Math.max(1, Math.floor(num(st.signal_persistence_bars, 1)));
  const parts = [];
  if (Number.isFinite(sl) && sl > 0) parts.push(`SL ${fmtPct(sl, 2)}`);
  if (Number.isFinite(tp) && tp > 0) parts.push(`TP ${fmtPct(tp, 2)}`);
  if (Number.isFinite(trail) && trail > 0) parts.push(`Trail ${fmtPct(trail, 2)}`);
  if (cooldown > 0) parts.push(`CD ${cooldown}b`);
  if (persist > 1) parts.push(`Persist ${persist}b`);
  if (!parts.length) return "";
  return `risk ${parts.join(" / ")}`;
}

function formatDoubleConstraintsProgressSummary(constraints) {
  const c = constraints && typeof constraints === "object" ? constraints : null;
  if (!c) return "";
  const parts = [];
  const minTrades = Number(c.min_trades);
  if (Number.isFinite(minTrades) && minTrades > 0) parts.push(`trades>=${Math.floor(minTrades)}`);
  const maxDd = Number(c.max_drawdown);
  if (Number.isFinite(maxDd) && maxDd >= 0) parts.push(`dd<=${fmtPct(maxDd, 2)}`);
  if (!parts.length) return "";
  return `filters ${parts.join(", ")}`;
}

function formatMaPairProgressSummary(fastTypes, slowTypes) {
  const fast = (Array.isArray(fastTypes) ? fastTypes : [])
    .map((x) => String(x || "").trim().toUpperCase())
    .filter((x) => !!x);
  const slow = (Array.isArray(slowTypes) ? slowTypes : [])
    .map((x) => String(x || "").trim().toUpperCase())
    .filter((x) => !!x);
  if (fast.length === 1 && slow.length === 1) return `${fast[0]} vs ${slow[0]}`;
  if (fast.length || slow.length) return `MA sets f${Math.max(1, fast.length)}×s${Math.max(1, slow.length)}`;
  return "";
}

function buildOptimizationProgressContext(req) {
  const kind = kindFromReq(req);
  const parts = [strategyKindLabel(kind)];
  const startEq = startingEquityFromReq(req);
  const risk = formatRiskProgressSummary(kind === "CompositeMo" ? (req?.base_strategy || req?.strategy) : req?.strategy);
  const work = estimateOptimizationWorkLabel(req);
  if (Number.isFinite(startEq) && startEq > 0) parts.push(`eq ${fmtMoney(startEq, 0)}`);

  if (isDoubleLikeKind(kind)) {
    parts.push(backendProgressLabel(req?.backend?.backend));
    const maPair = formatMaPairProgressSummary(req?.fast_ma_types, req?.slow_ma_types);
    if (maPair) parts.push(maPair);
    const objective = String(req?.objective || "").trim();
    if (objective) parts.push(`obj ${objective}`);
    const mode = String(req?.mode || "").trim();
    if (mode) parts.push(mode === "CoarseToFine" ? "Coarse→Fine" : mode);
    const constraintText = formatDoubleConstraintsProgressSummary(req?.constraints);
    if (constraintText) parts.push(constraintText);
  } else if (kind === "CompositeCpu") {
    parts.push("CPU");
    const maPair = formatMaPairProgressSummary(req?.fast_ma_types, req?.slow_ma_types);
    if (maPair) parts.push(maPair);
    const objective = String(req?.objective || "").trim();
    if (objective) parts.push(`obj ${objective}`);
  } else {
    parts.push(samplerProgressLabel(req?.sampler?.sampler));
    const fastMa = String(req?.fast_ma_type || "").trim().toUpperCase();
    const slowMa = String(req?.slow_ma_type || "").trim().toUpperCase();
    if (fastMa && slowMa) parts.push(`${fastMa} vs ${slowMa}`);
    const maxObj = String(req?.objectives?.maximize || req?.objective || "").trim();
    const minObj = String(req?.objectives?.minimize || "").trim();
    if (maxObj && minObj) parts.push(`obj ${maxObj}/${minObj}`);
    else if (maxObj) parts.push(`obj ${maxObj}`);
    if (req?.gpu_prescan && typeof req.gpu_prescan === "object") parts.push("GPU prescan");
  }

  if (risk) parts.push(risk);
  if (work) parts.push(work);
  return { group: "optimize", summary: parts.join(" · ") };
}

function buildValidationProgressContext(req, runKind) {
  const runReq = CURRENT_RUN?.req && typeof CURRENT_RUN.req === "object" ? CURRENT_RUN.req : null;
  const kind = String(runKind || (runReq ? kindFromReq(runReq) : "DoubleMa")).trim() || "DoubleMa";
  const parts = [strategyKindLabel(kind)];

  const candidatesN = Array.isArray(req?.candidates)
    ? req.candidates.length
    : Math.max(0, Math.floor(Number(req?.max_candidates || 0)));
  if (candidatesN > 0) parts.push(`${formatCompactNumber(candidatesN)} candidates`);

  const objective = kind === "CompositeMo"
    ? String(runReq?.objectives?.maximize || runReq?.objective || "").trim()
    : String(runReq?.objective || "").trim();
  if (objective) parts.push(`obj ${objective}`);

  const crossN = Array.isArray(req?.cross_ticker?.datasets) ? req.cross_ticker.datasets.length : 0;
  if (crossN > 0) parts.push(`${crossN} cross datasets`);

  const wf = req?.walk_forward && typeof req.walk_forward === "object" ? req.walk_forward : null;
  if (wf) {
    const wb = Math.max(0, Math.floor(Number(wf.window_bars || 0)));
    const sb = Math.max(0, Math.floor(Number(wf.step_bars || 0)));
    if (wb > 0 && sb > 0) parts.push(`WF ${formatCompactNumber(wb)}/${formatCompactNumber(sb)}`);
  }

  const cscv = req?.cscv_pbo && typeof req.cscv_pbo === "object" ? req.cscv_pbo : null;
  if (cscv) {
    const slices = Math.max(0, Math.floor(Number(cscv.slices || 0)));
    if (slices > 0) parts.push(`CSCV ${slices} slices`);
  }

  const perturb = req?.perturb && typeof req.perturb === "object" ? req.perturb : null;
  if (perturb) {
    const iters = Math.max(0, Math.floor(Number(perturb.iterations || 0)));
    if (iters > 0) parts.push(`perturb ${iters}`);
  }

  const mc = req?.trade_bootstrap && typeof req.trade_bootstrap === "object" ? req.trade_bootstrap : null;
  if (mc) {
    const sims = Math.max(0, Math.floor(Number(mc.simulations || 0)));
    if (sims > 0) parts.push(`bootstrap ${formatCompactNumber(sims)}`);
  }

  const minPsr = Number(req?.min_psr);
  const minDsr = Number(req?.min_dsr);
  const gates = [];
  if (Number.isFinite(minPsr)) gates.push(`PSR≥${fmtFixed(minPsr, 2)}`);
  if (Number.isFinite(minDsr)) gates.push(`DSR≥${fmtFixed(minDsr, 2)}`);
  if (gates.length) parts.push(gates.join(" "));

  const risk = formatRiskProgressSummary(
    kind === "CompositeMo"
      ? (runReq?.base_strategy || runReq?.strategy)
      : runReq?.strategy
  );
  if (risk) parts.push(risk);

  return { group: "validate", summary: parts.join(" · ") };
}

function buildMetaProgressContext(req) {
  const kind = String(req?.strategy_kind || "CompositeMo").trim() || "CompositeMo";
  const parts = [strategyKindLabel(kind)];

  const tf = String(req?.timeframe || "").trim();
  if (tf) parts.push(`tf ${tf}`);

  const minSamples = Math.max(0, Math.floor(Number(req?.min_samples || 0)));
  if (minSamples > 0) parts.push(`min ${formatCompactNumber(minSamples)}`);

  const topK = Math.max(0, Math.floor(Number(req?.top_k || 0)));
  if (topK > 0) parts.push(`top ${formatCompactNumber(topK)}`);

  const grouping = String(req?.grouping_mode || "").trim();
  if (grouping) parts.push(grouping === "clustered" ? "clustered groups" : `${grouping} groups`);

  const requiredTag = String(req?.required_tag || "").trim();
  if (requiredTag) parts.push(`tag ${requiredTag}`);

  if (req?.eval_holdout) {
    const holdoutMode = String(req?.holdout_mode || "light").trim() || "light";
    parts.push(`holdout ${holdoutMode}`);
    const holdoutTag = String(req?.holdout_tag || "").trim();
    if (holdoutTag) parts.push(`holdout-tag ${holdoutTag}`);
  }

  return { group: "meta", summary: parts.join(" · ") };
}

function setActiveProgressContext(ctx) {
  ACTIVE_PROGRESS_CONTEXT = ctx && typeof ctx === "object" ? ctx : null;
}

function phaseHint(phase) {
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

function phaseProgressUnit(phase) {
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

function activeProgressContextSummary(phase) {
  const ctx = ACTIVE_PROGRESS_CONTEXT && typeof ACTIVE_PROGRESS_CONTEXT === "object" ? ACTIVE_PROGRESS_CONTEXT : null;
  if (!ctx) return "";
  const phaseGrp = phaseGroup(phase);
  const ctxGrp = String(ctx.group || "").trim();
  if (ctxGrp && phaseGrp && ctxGrp !== phaseGrp) return "";
  return String(ctx.summary || "").trim();
}

function renderProgressDetail(opts) {
  const detail = $("progressDetail");
  if (!detail) return;
  const phase = String(opts?.phase || "").trim();
  const processed = Number(opts?.processed || 0);
  const total = Number(opts?.total || 0);

  const hint = phaseHint(phase);
  let remainText = "";
  if (Number.isFinite(processed) && Number.isFinite(total) && total > 0) {
    const remaining = Math.max(0, Math.floor(total - processed));
    if (remaining > 0) {
      const unit = phaseProgressUnit(phase);
      remainText = `remaining ${formatCompactNumber(remaining)} ${unit}`;
    } else if (processed >= total) {
      remainText = "finalizing";
    }
  }

  const left = [hint, remainText].filter((x) => !!String(x || "").trim()).join(" · ");
  const ctx = activeProgressContextSummary(phase);
  const text = ctx ? (left ? `${left} | ${ctx}` : ctx) : left;
  detail.textContent = compactProgressText(text, 380);
}

function optimizationProgressPhase(req) {
  const kindRaw = String(req?.strategy_kind || req?.strategyKind || "DoubleMa").trim();
  if (kindRaw === "CompositeCpu") return "composite_cpu";
  if (kindRaw === "CompositeMo") {
    const sampler = String(req?.sampler?.sampler || "Nsga2").trim();
    if (sampler === "Random") return "composite_mo_random";
    if (sampler === "Tpe") return "composite_mo_tpe";
    if (sampler === "Qmc") return "composite_mo_qmc";
    return "composite_mo_nsga2";
  }

  const backend = String(req?.backend?.backend || "CpuOnly").trim();
  if (backend === "GpuKernel") return "gpu-kernel";
  if (backend === "GpuOnly") return "gpu-sweep";
  return "cpu";
}

function setProgressIndeterminateUi(phase, suffixText) {
  const wrap = $("progressWrap");
  const fill = $("progressFill");
  const meta = $("progressMeta");
  if (!wrap || !fill) return;

  const ph = String(phase || "").trim();
  const label = phaseLabel(ph);
  const suffix = String(suffixText || "").trim();

  wrap.style.display = "block";
  fill.classList.add("indeterminate");

  if (meta) {
    meta.textContent = suffix ? `${label} · ${suffix}` : label;
  }
  renderProgressDetail({ phase: ph, processed: 0, total: 0, suffixText });
  if (RUNNING) updateOverviewEmptyUi();
}

function stopOptimizationProgressHeartbeat() {
  if (OPT_PROGRESS_TIMER_ID) {
    clearInterval(OPT_PROGRESS_TIMER_ID);
    OPT_PROGRESS_TIMER_ID = 0;
  }
  OPT_PROGRESS_STARTED_AT_MS = 0;
  OPT_PROGRESS_ESTIMATE_LABEL = "";
  OPT_PROGRESS_LAST_EVENT_AT_MS = 0;
}

function startOptimizationProgressHeartbeat(req) {
  stopOptimizationProgressHeartbeat();
  OPT_PROGRESS_STARTED_AT_MS = performance.now();
  OPT_PROGRESS_ESTIMATE_LABEL = estimateOptimizationWorkLabel(req);
  OPT_PROGRESS_LAST_EVENT_AT_MS = 0;
  const phase = optimizationProgressPhase(req);

  const tick = () => {
    if (!RUNNING) return;
    const now = performance.now();
    const isRecentEvent = OPT_PROGRESS_LAST_EVENT_AT_MS > 0 && (now - OPT_PROGRESS_LAST_EVENT_AT_MS) <= OPT_PROGRESS_EVENT_STALE_MS;
    if (isRecentEvent) return;

    const elapsed = formatDurationSeconds((now - OPT_PROGRESS_STARTED_AT_MS) / 1000);
    const suffixParts = [];
    if (OPT_PROGRESS_ESTIMATE_LABEL) suffixParts.push(OPT_PROGRESS_ESTIMATE_LABEL);
    if (elapsed) suffixParts.push(`elapsed ${elapsed}`);
    const suffix = suffixParts.join(" · ");
    setProgressIndeterminateUi(phase, suffix);

    const statusSuffix = suffix ? ` · ${suffix}` : "";
    setStatus(`Optimization running${statusSuffix}`, "muted");
  };

  tick();
  OPT_PROGRESS_TIMER_ID = window.setInterval(tick, 1000);
}

function phaseGroup(phase) {
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

function phaseDisplay(phase) {
  const ph = String(phase || "").trim();
  const detail = PHASE_LABELS[ph] || ph || "Working";
  const group = phaseGroup(ph);
  const groupLabel = PHASE_GROUP_LABELS[group] || "";
  if (!groupLabel) return { group, groupLabel: "", detailLabel: detail, label: detail };
  const label = detail && detail !== groupLabel ? `${groupLabel} · ${detail}` : groupLabel;
  return { group, groupLabel, detailLabel: detail, label };
}

function phaseLabel(phase) {
  return phaseDisplay(phase).label;
}

function formatDurationSeconds(sec) {
  const s0 = Math.max(0, Math.floor(Number(sec || 0)));
  if (!Number.isFinite(s0) || s0 <= 0) return "";
  const h = Math.floor(s0 / 3600);
  const m = Math.floor((s0 % 3600) / 60);
  const s = s0 % 60;
  if (h > 0) return `${h}h${String(m).padStart(2, "0")}m`;
  if (m > 0) return `${m}m${String(s).padStart(2, "0")}s`;
  return `${s}s`;
}

function formatCompactNumber(n) {
  const x0 = Number(n);
  if (!Number.isFinite(x0)) return "";
  const abs = Math.abs(x0);
  let x = x0;
  let suffix = "";
  if (abs >= 1e9) {
    x = x0 / 1e9;
    suffix = "G";
  } else if (abs >= 1e6) {
    x = x0 / 1e6;
    suffix = "M";
  } else if (abs >= 1e3) {
    x = x0 / 1e3;
    suffix = "k";
  }
  const ax = Math.abs(x);
  const digits = ax >= 100 ? 0 : (ax >= 10 ? 1 : 2);
  const s = x
    .toFixed(digits)
    .replace(/\.0+$/, "")
    .replace(/(\.\d*[1-9])0+$/, "$1");
  return `${s}${suffix}`;
}

function formatPerSecond(rate, unit) {
  const r = Number(rate);
  if (!Number.isFinite(r) || r <= 0) return "";
  const c = formatCompactNumber(r);
  if (!c) return "";
  return unit ? `${c} ${unit}/s` : `${c}/s`;
}

function updateProgressStats(phase, processed, total) {
  const ph = String(phase || "").trim();
  const proc = Number(processed || 0);
  const tot = Number(total || 0);
  if (!ph || !Number.isFinite(proc) || !Number.isFinite(tot) || tot <= 0 || proc <= 0) {
    return { etaText: "", rateText: "" };
  }

  const now = performance.now();
  let st = PROGRESS_ETA_BY_PHASE[ph];
  if (!st || st.total !== tot || proc < st.lastProcessed) {
    st = { lastTsMs: now, lastProcessed: proc, emaRate: 0, total: tot };
    PROGRESS_ETA_BY_PHASE[ph] = st;
    return { etaText: "", rateText: "" };
  }

  const dt = (now - st.lastTsMs) / 1000;
  const dp = proc - st.lastProcessed;
  if (dt > 0 && dp > 0) {
    const rate = dp / dt;
    if (Number.isFinite(rate) && rate > 0) {
      const alpha = 0.2;
      st.emaRate = st.emaRate > 0 ? (st.emaRate * (1 - alpha) + rate * alpha) : rate;
    }
  }

  st.lastTsMs = now;
  st.lastProcessed = proc;

  const rateText = formatPerSecond(st.emaRate, "");
  if (!Number.isFinite(st.emaRate) || st.emaRate <= 0 || proc >= tot) {
    return { etaText: "", rateText };
  }

  const remaining = tot - proc;
  const etaSec = remaining / st.emaRate;
  if (!Number.isFinite(etaSec) || etaSec <= 0) return { etaText: "", rateText };
  const pretty = formatDurationSeconds(etaSec);
  return { etaText: pretty ? `ETA ${pretty}` : "", rateText };
}

function setProgressUi(processed, total, phase, suffixText) {
  const wrap = $("progressWrap");
  const fill = $("progressFill");
  const meta = $("progressMeta");
  if (!wrap || !fill) return;

  const proc = Number(processed || 0);
  const tot = Number(total || 0);
  const ph = String(phase || "").trim();
  const label = phaseLabel(ph);

  wrap.style.display = "block";
  fill.classList.remove("indeterminate");

  let frac = tot > 0 ? proc / tot : 0;
  if (!Number.isFinite(frac)) frac = 0;
  if (frac < 0) frac = 0;
  if (frac > 1) frac = 1;
  fill.style.width = `${(frac * 100).toFixed(1)}%`;

  if (meta) {
    const pct = tot > 0 ? ` (${(frac * 100).toFixed(1)}%)` : "";
    const suffix = String(suffixText || "").trim();
    const suffix2 = suffix ? ` · ${suffix}` : "";
    meta.textContent = `${label} ${proc}/${tot}${pct}${suffix2}`.trim();
  }
  renderProgressDetail({ phase: ph, processed: proc, total: tot, suffixText });
  if (RUNNING) updateOverviewEmptyUi();
}

async function initProgressListener() {
  const listen = tauriListen();
  if (!listen) return;
  try {
    await listen("double_ma_progress", (event) => {
      if (!RUNNING) return;
      PROGRESS_EVENT_SEQ = (PROGRESS_EVENT_SEQ + 1) >>> 0;
      OPT_PROGRESS_LAST_EVENT_AT_MS = performance.now();
      const p = event?.payload || {};
      const processed = Number(p.processed_pairs || 0);
      const total = Number(p.total_pairs || 0);
      const phase = String(p.phase || "");
      const pct = total > 0 ? ((processed / total) * 100).toFixed(1) : "";
      const info = phaseDisplay(phase);
      const stats = updateProgressStats(phase, processed, total);
      const suffix = [stats.rateText, stats.etaText].filter((s) => !!String(s || "").trim()).join(" · ");
      const suffix2 = suffix ? ` · ${suffix}` : "";
      setStatus(`${info.label} ${processed}/${total}${pct ? ` (${pct}%)` : ""}${suffix2}`, "muted");
      setProgressUi(processed, total, phase, suffix);
    });
  } catch {
    // no-op (events are optional for this demo)
  }
}

async function initSampleCsvs() {
  const wrap = $("sampleCsvWrap");
  const body = $("sampleCsvs");
  if (!wrap || !body) return;

  const invoke = tauriInvoke();
  if (!invoke) {
    wrap.style.display = "none";
    return;
  }

  try {
    const list = await invoke("list_sample_csvs");
    const samples = Array.isArray(list) ? list : [];
    body.innerHTML = "";

    for (const s of samples) {
      const path = String(s?.path || "").trim();
      const label = String(s?.label || s?.id || "").trim();
      if (!path || !label) continue;

      const btn = document.createElement("button");
      btn.textContent = label;
      btn.title = path;
      btn.disabled = RUNNING;
      btn.addEventListener("click", async () => {
        if (RUNNING) return;
        if ($("csvPath")) $("csvPath").value = path;
        saveSettingsSoon();
        try {
          await loadCsv();
        } catch (e) {
          setStatus(String(e), "danger");
        }
      });
      body.appendChild(btn);
    }

    wrap.style.display = body.children.length ? "block" : "none";
  } catch {
    wrap.style.display = "none";
  }
}

let MARKET_DATA_DIR_INFO = null;
let MARKET_DATA_DIR_UI_INIT = false;

async function refreshMarketDataDirUi(opts) {
  const invoke = tauriInvoke();
  if (!invoke) return;

  const input = $("marketDataDir");
  const hint = $("marketDataDirHint");

  try {
    const info = await invoke("get_market_data_dir_info");
    const defaultRoot = String(info?.default_root_dir || "").trim();
    const configuredRoot = String(info?.configured_root_dir || "").trim();
    const effectiveRoot = String(info?.effective_root_dir || "").trim();
    const polygonCacheDir = String(info?.polygon_cache_dir || "").trim();

    MARKET_DATA_DIR_INFO = { defaultRoot, configuredRoot, effectiveRoot, polygonCacheDir };

    if (input && !opts?.preserveInput) {
      input.value = configuredRoot || "";
    }

    if (hint) {
      if (configuredRoot) {
        hint.textContent = `Using custom root: ${configuredRoot} · Polygon cache: ${polygonCacheDir || "—"}`;
      } else {
        hint.textContent = `Using default root: ${defaultRoot || "—"} · Polygon cache: ${polygonCacheDir || "—"}`;
      }
    }
  } catch (e) {
    if (hint) hint.textContent = `Market data folder: ${String(e)}`;
  } finally {
    syncSettingsWorkspaceUi();
  }
}

async function initMarketDataDirUi() {
  if (MARKET_DATA_DIR_UI_INIT) return;
  MARKET_DATA_DIR_UI_INIT = true;

  $("btnBrowseMarketDataDir")?.addEventListener("click", async () => {
    if (RUNNING) return;
    const invoke = tauriInvoke();
    if (!invoke) return;

    try {
      const picked = await invoke("pick_folder");
      const path = String(picked || "").trim();
      if (!path) return;
      if ($("marketDataDir")) $("marketDataDir").value = path;
      syncSettingsWorkspaceUi();
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });

  $("btnSetMarketDataDir")?.addEventListener("click", async () => {
    if (RUNNING) return;
    const invoke = tauriInvoke();
    if (!invoke) return;

    try {
      const dir = String($("marketDataDir")?.value || "").trim();
      const moveExisting = !!$("marketDataMoveExisting")?.checked;
      if (!dir) {
        setStatus("Enter a market data folder path first.", "danger");
        return;
      }

      setStatus("Updating market data folder...", "muted");
      await invoke("set_market_data_dir", { dir, moveExisting });
      await refreshMarketDataDirUi({ preserveInput: false });
      await initPolygonUi();
      setStatus("Updated market data folder.", "ok");
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });

  $("btnResetMarketDataDir")?.addEventListener("click", async () => {
    if (RUNNING) return;
    const invoke = tauriInvoke();
    if (!invoke) return;

    try {
      setStatus("Resetting market data folder...", "muted");
      await invoke("clear_market_data_dir");
      await refreshMarketDataDirUi({ preserveInput: false });
      await initPolygonUi();
      setStatus("Reset market data folder to default.", "ok");
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });

  $("btnOpenMarketDataDir")?.addEventListener("click", async () => {
    if (RUNNING) return;
    const invoke = tauriInvoke();
    if (!invoke) return;

    try {
      if (!MARKET_DATA_DIR_INFO) await refreshMarketDataDirUi({ preserveInput: true });
      const path = String(MARKET_DATA_DIR_INFO?.polygonCacheDir || MARKET_DATA_DIR_INFO?.effectiveRoot || "").trim();
      if (!path) {
        setStatus("No market data folder path available.", "muted");
        return;
      }

      try {
        await invoke("open_path", { path });
        setStatus(`Opened: ${path}`, "ok");
      } catch {
        await invoke("reveal_path", { path });
        setStatus(`Revealed: ${path}`, "ok");
      }
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });

  await refreshMarketDataDirUi({ preserveInput: false });
}

function ymdLocal(d) {
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

function ensurePolygonDateDefaults() {
  const toEl = $("polygonTo");
  const fromEl = $("polygonFrom");
  if (!toEl || !fromEl) return;

  const today = new Date();
  if (!String(toEl.value || "").trim()) {
    toEl.value = ymdLocal(today);
  }
  if (!String(fromEl.value || "").trim()) {
    const d = new Date(today);
    d.setDate(d.getDate() - 365);
    fromEl.value = ymdLocal(d);
  }
}

function updatePolygonAdjustedUi() {
  const market = String($("polygonMarket")?.value || "stocks").trim();
  const wrap = $("polygonStocksAdjustedWrap");
  const cb = $("polygonStocksAdjusted");
  const isStocks = market === "stocks";
  if (wrap) wrap.style.display = isStocks ? "block" : "none";
  if (cb) cb.disabled = RUNNING || !isStocks;
}

function openPolygonModal() {
  const modal = $("polygonModal");
  if (!modal) return;
  modal.classList.add("open");
  document.body.style.overflow = "hidden";
  const el = $("polygonSymbol") || $("polygonApiKey");
  if (el && typeof el.focus === "function") el.focus();
}

function closePolygonModal() {
  const modal = $("polygonModal");
  if (!modal) return;
  modal.classList.remove("open");
  document.body.style.overflow = "";
}

function fmtYmdUtc(ms) {
  const n = Number(ms);
  if (!Number.isFinite(n)) return "—";
  const d = new Date(n);
  const t = d.getTime();
  if (!Number.isFinite(t)) return "—";
  return d.toISOString().slice(0, 10);
}

async function initPolygonUi() {
  ensurePolygonDateDefaults();
  const invoke = tauriInvoke();
  if (!invoke) return;

  const keyStatus = $("polygonKeyStatus");
  const cacheHint = $("polygonCacheHint");
  const dl = $("polygonCachedSymbols");
  const openBtn = $("btnPolygonOpen");
  const list = $("polygonDatasets");

  try {
    const info = await invoke("polygon_get_cache_info");
    const hasKey = !!info?.has_api_key;
    const dir = String(info?.cache_dir || "").trim();
    const datasets = Array.isArray(info?.datasets) ? info.datasets : [];

    if (keyStatus) keyStatus.textContent = hasKey ? "Saved key: yes" : "Saved key: no";
    if (cacheHint) cacheHint.textContent = dir ? `Cache folder: ${dir} (${datasets.length} dataset${datasets.length === 1 ? "" : "s"})` : "Cache folder: —";
    if (openBtn) {
      openBtn.classList.toggle("primary", hasKey);
      openBtn.title = dir ? `Polygon cache folder: ${dir}` : "Polygon.io";
    }

    if (dl) {
      dl.innerHTML = "";
      const uniq = new Set();
      for (const d of datasets) {
        const sym = String(d?.display_symbol || d?.polygon_ticker || "").trim();
        if (!sym || uniq.has(sym)) continue;
        uniq.add(sym);
        const opt = document.createElement("option");
        opt.value = sym;
        dl.appendChild(opt);
      }
    }

    if (list) {
      list.innerHTML = "";
      const items = [...datasets].sort((a, b) => {
        const am = String(a?.market || "");
        const bm = String(b?.market || "");
        const as = String(a?.display_symbol || a?.polygon_ticker || "");
        const bs = String(b?.display_symbol || b?.polygon_ticker || "");
        return (am + as).localeCompare(bm + bs);
      });

      if (!items.length) {
        const empty = document.createElement("div");
        empty.className = "muted";
        empty.style.fontSize = "0.8rem";
        empty.textContent = "No cached datasets yet.";
        list.appendChild(empty);
      }

      for (const ds of items) {
        const path = String(ds?.csv_path || "").trim();
        const title = String(ds?.display_symbol || ds?.polygon_ticker || "").trim();
        const market = String(ds?.market || "").trim();
        const tfLabel = String(ds?.timeframe || "").trim() || "1m";
        const minTs = fmtYmdUtc(ds?.min_ts);
        const maxTs = fmtYmdUtc(ds?.max_ts);
        const updated = fmtYmdUtc(ds?.last_updated_utc_ms);
        const rowsN = Number(ds?.row_count || 0);
        const rowsHint = rowsN > 0 ? rowsN.toLocaleString() : "—";
        const adj = market === "stocks" && !!ds?.adjusted;
        const marketLabel = market ? (market === "stocks" ? (adj ? `${market} (adj)` : `${market} (raw)`) : market) : "market";
        const integ = ds?.integrity && typeof ds.integrity === "object" ? ds.integrity : null;
        const checkedMs = integ ? Number(integ.checked_utc_ms || 0) : 0;
        const checked = checkedMs > 0 ? fmtYmdUtc(checkedMs) : "";
        const dupN = integ ? Number(integ.duplicates || 0) : 0;
        const ooN = integ ? Number(integ.out_of_order || 0) : 0;
        const parseN = integ ? Number(integ.parse_errors || 0) : 0;
        const gapsN = integ ? Number(integ.same_day_gaps_gt_2min || 0) : 0;
        const hasIntegrityIssues = (dupN + ooN + parseN + gapsN) > 0;
        const integrityHint = checked ? (hasIntegrityIssues ? " • issues" : " • ok") : "";
        const gapsHint = checked && gapsN > 0 ? ` • gaps=${gapsN}` : "";
        const stale = (() => {
          const mx = Number(ds?.max_ts || 0);
          if (!Number.isFinite(mx) || mx <= 0) return false;
          const ageMs = Date.now() - mx;
          return Number.isFinite(ageMs) && ageMs > (3 * 24 * 60 * 60 * 1000);
        })();

        const item = document.createElement("div");
        item.className = "listItem";

        const row = document.createElement("div");
        row.style.display = "flex";
        row.style.alignItems = "center";
        row.style.justifyContent = "space-between";
        row.style.gap = "0.75rem";

        const left = document.createElement("div");
        const t1 = document.createElement("div");
        t1.className = "mono";
        t1.textContent = title || "(unknown)";
        const t2 = document.createElement("div");
        t2.className = "muted";
        t2.style.fontSize = "0.8rem";
        const staleHint = stale ? " • stale" : "";
        const checkedHint = checked ? ` • checked=${checked}` : "";
        t2.textContent = `${marketLabel} • tf=${tfLabel} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}${checkedHint}${integrityHint}${gapsHint}${staleHint}`;
        left.appendChild(t1);
        left.appendChild(t2);

        const actions = document.createElement("div");
        actions.className = "actions";
        actions.style.marginTop = "0";

        const btnLoad = document.createElement("button");
        btnLoad.textContent = "Load";
        btnLoad.disabled = RUNNING || !path;
        btnLoad.addEventListener("click", async () => {
          if (RUNNING) return;
          if (!path) return;
          if ($("csvPath")) $("csvPath").value = path;
          saveSettingsSoon();
          try {
            await loadCsv();
            closePolygonModal();
          } catch (e) {
            setStatus(String(e), "danger");
          }
        });
        actions.appendChild(btnLoad);

        const btnUpdate = document.createElement("button");
        btnUpdate.textContent = "Update";
        btnUpdate.disabled = RUNNING;
        btnUpdate.addEventListener("click", async () => {
          if (RUNNING) return;
          const sym = String(ds?.display_symbol || ds?.polygon_ticker || "").trim();
          if (!sym) return;
          if ($("polygonMarket")) $("polygonMarket").value = market || "stocks";
          if ($("polygonStocksAdjusted") && (market || "stocks") === "stocks") $("polygonStocksAdjusted").checked = !!ds?.adjusted;
          updatePolygonAdjustedUi();
          if ($("polygonSymbol")) $("polygonSymbol").value = sym;
          if ($("polygonTimeframe")) $("polygonTimeframe").value = tfLabel;
          if ($("polygonFrom") && maxTs !== "—") $("polygonFrom").value = maxTs;
          if ($("polygonTo")) $("polygonTo").value = ymdLocal(new Date());
          try {
            setRunning(true);
            setStatus(tfLabel === "1m" ? `Updating ${sym} (1m)...` : `Updating ${sym} (1m; deriving ${tfLabel})...`, "muted");
            const adjFlag = (market || "stocks") === "stocks" ? !!ds?.adjusted : false;
            const resp = await invoke("polygon_download_1m_and_load", { req: { market: market || "stocks", symbol: sym, from: maxTs !== "—" ? maxTs : ymdLocal(new Date()), to: ymdLocal(new Date()), adjusted: adjFlag, timeframe: tfLabel } });

            const path2 = String(resp?.csv_path || "").trim();
            if (path2 && $("csvPath")) $("csvPath").value = path2;

            const id = String(resp?.id || "").trim();
            const meta2 = resp?.meta && typeof resp.meta === "object" ? resp.meta : null;
            const baseId = String(resp?.base_load?.id || "").trim() || id;
            const baseMeta = resp?.base_load?.meta && typeof resp.base_load.meta === "object" ? resp.base_load.meta : meta2;
            const baseTf = String(baseMeta?.timeframe || "1m").trim() || "1m";
            const curTf = String(tfLabel || baseTf).trim() || baseTf;
            CURRENT_BASE_DATA_ID = baseId;
            CURRENT_BASE_DATA_META = baseMeta;
            CURRENT_BASE_DATASET_CTX = {
              source: "polygon",
              label: sym,
              symbol: sym,
              market: market || "stocks",
              adjusted: adjFlag,
              timeframe: baseTf,
              base_timeframe: baseTf,
              derived_timeframe: baseTf,
              resample_applied: false,
              resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
              resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
            };
            CURRENT_DATA_META = meta2;
            CURRENT_DATASET_CTX = {
              ...CURRENT_BASE_DATASET_CTX,
              timeframe: curTf,
              base_timeframe: baseTf,
              derived_timeframe: curTf,
              resample_applied: !!(baseTf && curTf && baseTf !== curTf),
              resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
              resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
            };
            clearDrillCaches({ clearBars: true });
            $("dataId").textContent = id || "—";
            renderDataMeta(meta2);
            applyMaSourceConstraints();
            applyBackendMaConstraints();
            updateOverviewEmptyUi();

            const dlBars = Number(resp?.downloaded_bars || 0);
            const usedCacheOnly = !!resp?.used_cache_only;
            const msg = usedCacheOnly ? "Dataset up to date (cache)." : `Downloaded ${dlBars} new bar${dlBars === 1 ? "" : "s"} and loaded.`;
            setStatus(msg, "ok");
            $("output").textContent = pretty(resp);

            if (path2) addRecentCsvPath(path2);
            saveSettingsSoon();
            await initPolygonUi();
            await initCrossTickerList();
          } catch (e) {
            setStatus(String(e), "danger");
          } finally {
            setRunning(false);
          }
        });
        actions.appendChild(btnUpdate);

        const btnReveal = document.createElement("button");
        btnReveal.textContent = "Reveal";
        btnReveal.disabled = RUNNING || !path;
        btnReveal.addEventListener("click", async () => {
          if (RUNNING) return;
          if (!path) return;
          try {
            await invoke("reveal_path", { path });
            setStatus("Revealed dataset file.", "ok");
          } catch (e) {
            setStatus(String(e), "danger");
          }
        });
        actions.appendChild(btnReveal);

        const btnValidate = document.createElement("button");
        btnValidate.textContent = "Validate";
        btnValidate.disabled = RUNNING || !path;
        btnValidate.addEventListener("click", async () => {
          if (RUNNING) return;
          if (!path) return;
          try {
            setRunning(true);
            setStatus(`Validating ${title || "dataset"}...`, "muted");
            const out = await invoke("polygon_validate_dataset", { csvPath: path });
            const r = out?.integrity && typeof out.integrity === "object" ? out.integrity : null;
            const dup = Number(r?.duplicates || 0);
            const oo = Number(r?.out_of_order || 0);
            const perr = Number(r?.parse_errors || 0);
            const gaps = Number(r?.same_day_gaps_gt_2min || 0);
            const issues = (dup + oo + perr + gaps) > 0;
            const msg = `Validated: dup=${dup} oo=${oo} parse=${perr} gaps=${gaps}`;
            setStatus(msg, issues ? "danger" : "ok");
            await initPolygonUi();
            await initCrossTickerList();
          } catch (e) {
            setStatus(String(e), "danger");
          } finally {
            setRunning(false);
          }
        });
        actions.appendChild(btnValidate);

        const btnRepair = document.createElement("button");
        btnRepair.textContent = "Repair";
        btnRepair.disabled = RUNNING || !path || !checkedMs || !hasIntegrityIssues;
        btnRepair.addEventListener("click", async () => {
          if (RUNNING) return;
          if (!path) return;
          if (!confirm("Repair this dataset CSV? (rewrites file sorted + deduped; drops malformed rows; may re-fetch missing minutes if gaps are detected and API key is set)")) return;
          try {
            setRunning(true);
            setStatus(`Repairing ${title || "dataset"}...`, "muted");
            const out = await invoke("polygon_repair_dataset", { csvPath: path });
            const r = out?.integrity && typeof out.integrity === "object" ? out.integrity : null;
            const perr = Number(r?.parse_errors || 0);
            const gaps = Number(r?.same_day_gaps_gt_2min || 0);
            setStatus(`Repaired: parse_dropped=${perr} gaps=${gaps}`, "ok");
            await initPolygonUi();
            await initCrossTickerList();
          } catch (e) {
            setStatus(String(e), "danger");
          } finally {
            setRunning(false);
          }
        });
        actions.appendChild(btnRepair);

        row.appendChild(left);
        row.appendChild(actions);
        item.appendChild(row);
        list.appendChild(item);
      }
    }
  } catch (e) {
    if (keyStatus) keyStatus.textContent = "Saved key: —";
    if (cacheHint) cacheHint.textContent = `Polygon: ${String(e)}`;
    if (openBtn) openBtn.classList.remove("primary");
    if (list) list.innerHTML = "";
  }
}

function parseJsonArraySafe(text) {
  try {
    const v = JSON.parse(String(text || "[]"));
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

function setCrossSelectedPaths(paths) {
  const el = $("valCrossSelected");
  if (!el) return;
  const uniq = [];
  const seen = new Set();
  for (const p of Array.isArray(paths) ? paths : []) {
    const s = String(p || "").trim();
    if (!s || seen.has(s)) continue;
    seen.add(s);
    uniq.push(s);
  }
  try {
    el.value = JSON.stringify(uniq);
  } catch {
    el.value = "[]";
  }
}

function getCrossSelectedPaths() {
  const el = $("valCrossSelected");
  const raw = el ? String(el.value || "[]") : "[]";
  const arr = parseJsonArraySafe(raw);
  const out = [];
  const seen = new Set();
  for (const p of arr) {
    const s = String(p || "").trim();
    if (!s || seen.has(s)) continue;
    seen.add(s);
    out.push(s);
  }
  return out;
}

function syncCrossSelectedFromUi() {
  const paths = [];
  for (const cb of document.querySelectorAll("#valCrossDatasets input[type=checkbox][data-csv-path]")) {
    if (!cb || !cb.checked) continue;
    const p = String(cb.dataset?.csvPath || "").trim();
    if (!p) continue;
    paths.push(p);
  }
  setCrossSelectedPaths(paths);
  updateCrossMeta("");
  updateValidateTabUi();
}

function getCrossSelectedDatasets() {
  const out = [];
  const cbs = [...document.querySelectorAll("#valCrossDatasets input[type=checkbox][data-csv-path]")];
  if (cbs.length) {
    for (const cb of cbs) {
      if (!cb || !cb.checked) continue;
      const p = String(cb.dataset?.csvPath || "").trim();
      if (!p) continue;
      out.push({ csv_path: p, label: String(cb.dataset?.label || "").trim() });
    }
    return out;
  }
  // Fallback: if the list isn't populated, use the persisted selection (paths only).
  for (const p of getCrossSelectedPaths()) {
    out.push({ csv_path: p, label: "" });
  }
  return out;
}

function crossTickerQuery() {
  return String($("valCrossSearch")?.value || "").trim().toLowerCase();
}

function crossFilterTokens() {
  const q = crossTickerQuery();
  if (!q) return [];
  return q.split(/[\s,]+/g).map((t) => String(t || "").trim()).filter((t) => !!t);
}

function updateCrossMeta(extra) {
  const meta = $("valCrossMeta");
  if (!meta) return;

  const items = [...document.querySelectorAll("#valCrossDatasets .listItem")];
  const total = items.length;
  const visible = items.filter((it) => it && it.style.display !== "none").length;
  const selected = getCrossSelectedPaths().length;

  const parts = [];
  parts.push(`selected ${selected}/${total}${visible !== total ? ` (showing ${visible})` : ""}`);
  const x = String(extra || "").trim();
  if (x) parts.push(x);
  meta.textContent = parts.join(" • ");

  const btnMatch = $("btnCrossSelectMatch");
  if (btnMatch) {
    const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};
    const market = String(ctx.market || "").trim();
    const tf = String(ctx.timeframe || "").trim();
    btnMatch.disabled = RUNNING || !market || !tf;
    btnMatch.title = (market && tf)
      ? "Select datasets that match the currently loaded dataset's market, adjusted/raw (stocks), and timeframe."
      : "Match current requires the loaded dataset to have market + timeframe metadata (e.g. loaded from Polygon).";
  }
}

function applyCrossTickerFilter() {
  const list = $("valCrossDatasets");
  if (!list) return;
  const tokens = crossFilterTokens();

  const items = [...list.querySelectorAll(".listItem")];
  for (const it of items) {
    const hay = String(it?.dataset?.search || "").toLowerCase();
    const ok = !tokens.length || tokens.every((t) => hay.includes(t));
    it.style.display = ok ? "" : "none";
  }
  updateCrossMeta(tokens.length ? `filter=${tokens.join(" ")}` : "");
}

function setCrossSelectionForVisible(fnKeepChecked) {
  const list = $("valCrossDatasets");
  if (!list) return;
  const cbs = [...list.querySelectorAll("input[type=checkbox][data-csv-path]")];
  for (const cb of cbs) {
    const item = cb?.closest(".listItem");
    if (!item || item.style.display === "none") continue;
    if (cb.disabled) continue;
    cb.checked = !!fnKeepChecked(cb, item);
  }
  syncCrossSelectedFromUi();
  saveSettingsSoon();
  updateCrossMeta("");
}

function selectAllVisibleCross() {
  setCrossSelectionForVisible(() => true);
}

function selectNoneVisibleCross() {
  setCrossSelectionForVisible(() => false);
}

function selectMatchCurrentCross() {
  const ctx = CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object" ? CURRENT_DATASET_CTX : {};
  const market = String(ctx.market || "").trim();
  if (!market) return;
  const tf = String(ctx.timeframe || "").trim().toLowerCase();
  if (!tf) return;
  const adjusted = ctx.adjusted === true ? true : (ctx.adjusted === false ? false : null);
  const curPath = String($("csvPath")?.value || "").trim();

  setCrossSelectionForVisible((cb, item) => {
    const p = String(cb?.dataset?.csvPath || "").trim();
    if (curPath && p && p === curPath) return false;

    const m = String(item?.dataset?.market || "").trim();
    if (!m || m !== market) return false;

    const tf2 = String(item?.dataset?.timeframe || "").trim().toLowerCase();
    if (!tf2 || tf2 !== tf) return false;

    if (market === "stocks" && adjusted != null) {
      const a = String(item?.dataset?.adjusted || "").trim();
      const want = adjusted ? "1" : "0";
      if (a !== want) return false;
    }
    return true;
  });
}

function setBatchSelectedPaths(paths) {
  const el = $("batchSelected");
  if (!el) return;
  const uniq = [];
  const seen = new Set();
  for (const p of Array.isArray(paths) ? paths : []) {
    const s = String(p || "").trim();
    if (!s || seen.has(s)) continue;
    seen.add(s);
    uniq.push(s);
  }
  try {
    el.value = JSON.stringify(uniq);
  } catch {
    el.value = "[]";
  }
}

function getBatchSelectedPaths() {
  const el = $("batchSelected");
  const raw = el ? String(el.value || "[]") : "[]";
  const arr = parseJsonArraySafe(raw);
  const out = [];
  const seen = new Set();
  for (const p of arr) {
    const s = String(p || "").trim();
    if (!s || seen.has(s)) continue;
    seen.add(s);
    out.push(s);
  }
  return out;
}

function syncBatchSelectedFromUi() {
  const selected = new Set(getBatchSelectedPaths());
  for (const cb of document.querySelectorAll("#batchDatasets input[type=checkbox][data-csv-path]")) {
    const p = String(cb?.dataset?.csvPath || "").trim();
    if (!p) continue;
    if (cb.checked) selected.add(p);
    else selected.delete(p);
  }
  setBatchSelectedPaths([...selected]);
}

function batchDatasetQuery() {
  return String($("batchDatasetSearch")?.value || "").trim().toLowerCase();
}

function updateBatchMeta(extra) {
  const meta = $("batchMeta");
  if (!meta) return;
  const total = Array.isArray(BATCH_DATASETS_CACHE) ? BATCH_DATASETS_CACHE.length : 0;
  const selected = getBatchSelectedPaths().length;
  const parts = [];
  parts.push(`selected ${selected}/${total}`);
  const x = String(extra || "").trim();
  if (x) parts.push(x);
  const last = LAST_BATCH_SUMMARY && typeof LAST_BATCH_SUMMARY === "object" ? LAST_BATCH_SUMMARY : null;
  const p = last ? String(last.path || "").trim() : "";
  if (p) parts.push(`summary=${fileBasename(p)}`);
  meta.textContent = parts.join(" • ");
}

function setBatchSummaryUi(summary, path) {
  const p = String(path || "").trim();
  const s = summary && typeof summary === "object" ? summary : null;
  LAST_BATCH_SUMMARY = s ? { path: p, summary: s } : null;

  if ($("batchOutput")) $("batchOutput").textContent = s ? pretty({ path: p || null, summary: s }) : "-";
  try {
    if (p) localStorage.setItem(LAST_BATCH_SUMMARY_KEY, p);
    else localStorage.removeItem(LAST_BATCH_SUMMARY_KEY);
  } catch {}

  updateBatchMeta("");
  syncBatchSummaryButtons();
}

function syncBatchSummaryButtons() {
  const p = String(LAST_BATCH_SUMMARY?.path || "").trim();
  const s = LAST_BATCH_SUMMARY?.summary && typeof LAST_BATCH_SUMMARY.summary === "object" ? LAST_BATCH_SUMMARY.summary : null;
  const failed = Number(s?.counts?.failed || s?.results?.failed?.length || 0);

  const reveal = $("btnBatchRevealSummary");
  if (reveal) reveal.disabled = RUNNING || !p;

  const retry = $("btnBatchRetryFailed");
  if (retry) retry.disabled = RUNNING || !s || failed <= 0;
}

async function loadLastBatchSummaryFromDisk() {
  const invoke = tauriInvoke();
  if (!invoke) return;
  let p = "";
  try {
    p = String(localStorage.getItem(LAST_BATCH_SUMMARY_KEY) || "").trim();
  } catch {
    p = "";
  }
  if (!p) return;
  try {
    const v = await invoke("load_run_artifact", { path: p });
    const kind = String(v?.kind || "").trim();
    if (kind !== "vectorbt_batch_summary") return;
    setBatchSummaryUi(v, p);
  } catch {}
}

function filterBatchDatasets(datasets, query) {
  const q = String(query || "").trim().toLowerCase();
  const xs = Array.isArray(datasets) ? datasets : [];
  if (!q) return xs;
  const terms = q.split(/\s+/g).map((t) => t.trim()).filter((t) => !!t);
  if (!terms.length) return xs;

  return xs.filter((ds) => {
    const market = String(ds?.market || "").trim();
    const sym = String(ds?.display_symbol || ds?.polygon_ticker || "").trim();
    const path = String(ds?.csv_path || "").trim();
    const tf = String(ds?.timeframe || "").trim();
    const hay = `${market} ${sym} ${path} tf=${tf}`.toLowerCase();
    return terms.every((t) => hay.includes(t));
  });
}

function renderBatchDatasetList() {
  const list = $("batchDatasets");
  if (!list) return;

  const datasets = Array.isArray(BATCH_DATASETS_CACHE) ? BATCH_DATASETS_CACHE : [];
  list.innerHTML = "";

  if (!datasets.length) {
    const empty = document.createElement("div");
    empty.className = "muted";
    empty.style.fontSize = "0.8rem";
    empty.textContent = "No datasets yet. Put CSVs in a repo-level 'data' folder, or use Polygon to download and cache symbols.";
    list.appendChild(empty);
    updateBatchMeta("");
    return;
  }

  const q = batchDatasetQuery();
  const items = filterBatchDatasets(datasets, q).sort((a, b) => {
    const am = String(a?.market || "");
    const bm = String(b?.market || "");
    const as = String(a?.display_symbol || a?.polygon_ticker || "");
    const bs = String(b?.display_symbol || b?.polygon_ticker || "");
    return (am + as).localeCompare(bm + bs);
  });

  if (!items.length) {
    const empty = document.createElement("div");
    empty.className = "muted";
    empty.style.fontSize = "0.8rem";
    empty.textContent = q ? "No matches for the current search." : "No datasets.";
    list.appendChild(empty);
    updateBatchMeta("");
    return;
  }

  const selected = new Set(getBatchSelectedPaths());

  const MAX_RENDER = items.length > 1000 ? 500 : items.length;
  const renderItems = items.slice(0, MAX_RENDER);
  if (renderItems.length < items.length) {
    const note = document.createElement("div");
    note.className = "muted";
    note.style.fontSize = "0.8rem";
    note.style.marginBottom = "0.55rem";
    note.textContent = `Showing first ${renderItems.length} of ${items.length} results. Refine search to narrow. (Selection still applies to all filtered items.)`;
    list.appendChild(note);
  }

  for (const ds of renderItems) {
    const path = String(ds?.csv_path || "").trim();
    const title = String(ds?.display_symbol || ds?.polygon_ticker || "").trim();
    const market = String(ds?.market || "").trim();
    const tfLabel = String(ds?.timeframe || "").trim() || "?";
    const minTs = fmtYmdUtc(ds?.min_ts);
    const maxTs = fmtYmdUtc(ds?.max_ts);
    const updated = fmtYmdUtc(ds?.last_updated_utc_ms);
    const rowsN = Number(ds?.row_count || 0);
    const rowsHint = rowsN > 0 ? rowsN.toLocaleString() : "—";
    const adj = market === "stocks" && !!ds?.adjusted;
    const marketLabel = market ? (market === "stocks" ? (adj ? `${market} (adj)` : `${market} (raw)`) : market) : "market";
    if (!path || !title) continue;

    const item = document.createElement("div");
    item.className = "listItem";

    const row = document.createElement("div");
    row.style.display = "flex";
    row.style.alignItems = "center";
    row.style.justifyContent = "space-between";
    row.style.gap = "0.75rem";

    const left = document.createElement("div");
    const t1 = document.createElement("div");
    t1.className = "mono";
    t1.textContent = title;
    const t2 = document.createElement("div");
    t2.className = "muted";
    t2.style.fontSize = "0.8rem";
    t2.textContent = `${marketLabel} • tf=${tfLabel} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}`;
    left.appendChild(t1);
    left.appendChild(t2);

    const right = document.createElement("div");
    right.style.display = "flex";
    right.style.alignItems = "center";
    right.style.gap = "0.5rem";

    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = selected.has(path);
    cb.disabled = RUNNING;
    cb.dataset.csvPath = path;
    cb.dataset.label = title;
    cb.addEventListener("change", () => {
      syncBatchSelectedFromUi();
      updateBatchMeta("");
      saveSettingsSoon();
    });

    row.addEventListener("click", (ev) => {
      const isToggle = ev?.target === cb;
      if (isToggle) return;
      if (RUNNING) return;
      cb.checked = !cb.checked;
      syncBatchSelectedFromUi();
      updateBatchMeta("");
      saveSettingsSoon();
    });

    right.appendChild(cb);
    row.appendChild(left);
    row.appendChild(right);
    item.appendChild(row);
    list.appendChild(item);
  }

  updateBatchMeta("");
}

async function initBatchDatasetList() {
  const list = $("batchDatasets");
  if (!list) return;

  const invoke = tauriInvoke();
  if (!invoke) {
    list.innerHTML = "";
    const msg = document.createElement("div");
    msg.className = "muted";
    msg.style.fontSize = "0.8rem";
    msg.textContent = "Batch list requires the Tauri backend.";
    list.appendChild(msg);
    updateBatchMeta("");
    return;
  }

  let poly = [];
  let polyErr = "";
  try {
    const info = await invoke("polygon_get_cache_info");
    poly = Array.isArray(info?.datasets) ? info.datasets : [];
  } catch (e) {
    polyErr = String(e);
    poly = [];
  }

  let localPaths = [];
  let localEntries = [];
  let localDir = "";
  let localTruncated = false;
  let localErr = "";
  try {
    const resp = await invoke("list_repo_data_csvs", { maxFiles: 20000, recursive: true, includeIdentity: true });
    localPaths = Array.isArray(resp?.csv_paths) ? resp.csv_paths : [];
    localEntries = Array.isArray(resp?.csv_entries) ? resp.csv_entries : [];
    localDir = String(resp?.dir || "").trim();
    localTruncated = !!resp?.truncated;
  } catch (e) {
    localErr = String(e);
    localPaths = [];
    localEntries = [];
  }

  const byPath = new Map();
  for (const ds of poly) {
    const p = String(ds?.csv_path || "").trim();
    if (!p) continue;
    byPath.set(p, ds);
  }
  const locals = localEntries.length ? localEntries : localPaths.map((p) => ({ csv_path: p }));
  for (const e of locals) {
    const p = String(e?.csv_path || "").trim();
    if (!p || byPath.has(p)) continue;
    const size = Number(e?.source_size_bytes ?? NaN);
    const mtime = Number(e?.source_mtime_utc_ms ?? NaN);
    const fp = String(e?.source_content_fingerprint || "").trim();
    const source_size_bytes = Number.isFinite(size) && size >= 0 ? size : undefined;
    const source_mtime_utc_ms = Number.isFinite(mtime) && mtime > 0 ? mtime : undefined;
    const source_content_fingerprint = fp || undefined;
    byPath.set(p, {
      source: "repo_data",
      market: "local",
      display_symbol: fileBasename(p) || p,
      csv_path: p,
      timeframe: "",
      source_size_bytes,
      source_mtime_utc_ms,
      source_content_fingerprint,
    });
  }

  BATCH_DATASETS_CACHE = [...byPath.values()];
  renderBatchDatasetList();

  const extras = [];
  if (localDir) extras.push(`local=${fileBasename(localDir) || "data"}`);
  if (localTruncated) extras.push("local truncated");
  if (polyErr && !poly.length) extras.push("polygon error");
  if (localErr && !localPaths.length) extras.push("local data error");
  updateBatchMeta(extras.join(" • "));
}

async function initCrossTickerList() {
  const list = $("valCrossDatasets");
  if (!list) return;

  const invoke = tauriInvoke();
  if (!invoke) {
    list.innerHTML = "";
    const msg = document.createElement("div");
    msg.className = "muted";
    msg.style.fontSize = "0.8rem";
    msg.textContent = "Cross-ticker list requires the Tauri backend.";
    list.appendChild(msg);
    updateCrossMeta("backend unavailable");
    return;
  }

  try {
    const info = await invoke("polygon_get_cache_info");
    const datasets = Array.isArray(info?.datasets) ? info.datasets : [];

    list.innerHTML = "";
    const items = [...datasets].sort((a, b) => {
      const am = String(a?.market || "");
      const bm = String(b?.market || "");
      const as = String(a?.display_symbol || a?.polygon_ticker || "");
      const bs = String(b?.display_symbol || b?.polygon_ticker || "");
      return (am + as).localeCompare(bm + bs);
    });

    if (!items.length) {
      const empty = document.createElement("div");
      empty.className = "muted";
      empty.style.fontSize = "0.8rem";
      empty.textContent = "No cached datasets yet. Use Polygon to download and cache a few symbols.";
      list.appendChild(empty);
      setCrossSelectedPaths([]);
      saveSettingsSoon();
      updateCrossMeta("no cached datasets");
      return;
    }

    const selected = new Set(getCrossSelectedPaths());

    for (const ds of items) {
      const path = String(ds?.csv_path || "").trim();
      const title = String(ds?.display_symbol || ds?.polygon_ticker || "").trim();
      const market = String(ds?.market || "").trim();
      const tfLabel = String(ds?.timeframe || "").trim() || "1m";
      const minTs = fmtYmdUtc(ds?.min_ts);
      const maxTs = fmtYmdUtc(ds?.max_ts);
      const updated = fmtYmdUtc(ds?.last_updated_utc_ms);
      const rowsN = Number(ds?.row_count || 0);
      const rowsHint = rowsN > 0 ? rowsN.toLocaleString() : "—";
      const adj = market === "stocks" && !!ds?.adjusted;
      const marketLabel = market ? (market === "stocks" ? (adj ? `${market} (adj)` : `${market} (raw)`) : market) : "market";
      if (!path || !title) continue;

      const item = document.createElement("div");
      item.className = "listItem";
      item.dataset.search = `${title} ${marketLabel} tf=${tfLabel} ${minTs} ${maxTs}`.toLowerCase();
      item.dataset.market = market;
      item.dataset.adjusted = adj ? "1" : "0";
      item.dataset.timeframe = tfLabel;

      const row = document.createElement("div");
      row.style.display = "flex";
      row.style.alignItems = "center";
      row.style.justifyContent = "space-between";
      row.style.gap = "0.75rem";

      const left = document.createElement("div");
      const t1 = document.createElement("div");
      t1.className = "mono";
      t1.textContent = title;
      const t2 = document.createElement("div");
      t2.className = "muted";
      t2.style.fontSize = "0.8rem";
      t2.textContent = `${marketLabel} • tf=${tfLabel} • ${minTs} → ${maxTs} • rows=${rowsHint} • updated=${updated}`;
      left.appendChild(t1);
      left.appendChild(t2);

      const right = document.createElement("div");
      right.style.display = "flex";
      right.style.alignItems = "center";
      right.style.gap = "0.5rem";

      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = selected.has(path);
      cb.disabled = RUNNING;
      cb.dataset.csvPath = path;
      cb.dataset.label = title;
      cb.addEventListener("change", () => {
        syncCrossSelectedFromUi();
        saveSettingsSoon();
      });

      row.addEventListener("click", (ev) => {
        const isToggle = ev?.target === cb;
        if (isToggle) return;
        if (RUNNING) return;
        cb.checked = !cb.checked;
        syncCrossSelectedFromUi();
        saveSettingsSoon();
      });

      right.appendChild(cb);
      row.appendChild(left);
      row.appendChild(right);
      item.appendChild(row);
      list.appendChild(item);
    }

    syncCrossSelectedFromUi();
    saveSettingsSoon();
    applyCrossTickerFilter();
  } catch (e) {
    list.innerHTML = "";
    const msg = document.createElement("div");
    msg.className = "muted";
    msg.style.fontSize = "0.8rem";
    msg.textContent = `Polygon: ${String(e)}`;
    list.appendChild(msg);
    updateCrossMeta("failed to load datasets");
  }
}

function updateDeviceIdEnabled() {
  const kind = strategyKind();
  const backend = $("backend")?.value || "CpuOnly";
  const isGpu = ["GpuOnly", "GpuKernel", "Auto"].includes(backend);
  const dev = $("deviceId");
  if (!dev) return;
  const s = LAST_GPU_STATUS && typeof LAST_GPU_STATUS === "object" ? LAST_GPU_STATUS : null;
  const gpuOk = !s ? true : (!!s?.built_with_cuda && !!s?.cuda_available);

  if (kind === "CompositeCpu") {
    dev.disabled = true;
    return;
  }
  if (kind === "CompositeMo") {
    const prescanOn = !!$("moPrescanEnable")?.checked;
    dev.disabled = RUNNING || !prescanOn || !gpuOk;
    return;
  }
  dev.disabled = RUNNING || !isGpu || !gpuOk;
}

function rememberDesiredDeviceIdFromUi() {
  const sel = $("deviceId");
  if (!sel) return;
  sel.dataset.desired = String(sel.value || "0");
}

function desiredDeviceId() {
  const sel = $("deviceId");
  const raw = String(sel?.dataset?.desired || sel?.value || "0").trim();
  const n = Number(raw);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.floor(n));
}

function formatBytesCompact(bytes) {
  const b = Number(bytes || 0);
  if (!Number.isFinite(b) || b <= 0) return "";
  const gib = b / (1024 ** 3);
  if (gib >= 1) return `${fmtFixed(gib, 1)} GiB`;
  const mib = b / (1024 ** 2);
  return `${fmtFixed(mib, 0)} MiB`;
}

function applyCudaBackendAvailabilityConstraints(status) {
  if (!isDoubleLikeKind(strategyKind())) return;
  const backendSel = $("backend");
  if (!backendSel) return;

  const builtCuda = !!status?.built_with_cuda;
  const builtKernel = !!status?.built_with_cuda_backtest_kernel;
  const cudaOk = !!status?.cuda_available;

  const disableGpuOnly = !builtCuda || !cudaOk;
  const disableGpuKernel = !builtCuda || !builtKernel || !cudaOk;

  for (const opt of backendSel.options) {
    if (opt.value === "GpuOnly") opt.disabled = disableGpuOnly;
    else if (opt.value === "GpuKernel") opt.disabled = disableGpuKernel;
  }

  const cur = String(backendSel.value || "CpuOnly");
  if ((cur === "GpuOnly" && disableGpuOnly) || (cur === "GpuKernel" && disableGpuKernel)) {
    backendSel.value = "Auto";
    applyBackendMaConstraints();
    updateDeviceIdEnabled();
  }
}

async function refreshCudaDevicesUi(opts) {
  const sel = $("deviceId");
  if (!sel) return;
  const invoke = tauriInvoke();
  if (!invoke) return;

  const status = opts?.status && typeof opts.status === "object" ? opts.status : null;
  const builtCuda = !!status?.built_with_cuda;
  const cudaOk = !!status?.cuda_available;

  if (!builtCuda || !cudaOk) {
    if (sel.options.length !== 1 || sel.options[0]?.value !== "0") {
      sel.innerHTML = "";
      const opt = document.createElement("option");
      opt.value = "0";
      opt.textContent = "Auto";
      sel.appendChild(opt);
    }
    if (!sel.dataset.desired) sel.dataset.desired = "0";
    sel.value = "0";
    return;
  }

  const force = opts?.force === true;
  const now = typeof performance !== "undefined" ? performance.now() : Date.now();

  let resp = null;
  if (!force && LAST_CUDA_DEVICES && (now - LAST_CUDA_DEVICES_AT_MS) < 2000) {
    resp = LAST_CUDA_DEVICES;
  } else {
    try {
      if (!force && CUDA_DEVICES_INFLIGHT) resp = await CUDA_DEVICES_INFLIGHT;
      else {
        CUDA_DEVICES_INFLIGHT = invoke("list_cuda_devices");
        resp = await CUDA_DEVICES_INFLIGHT;
      }
    } catch {
      resp = null;
    } finally {
      CUDA_DEVICES_INFLIGHT = null;
    }
    if (resp && typeof resp === "object") {
      LAST_CUDA_DEVICES = resp;
      LAST_CUDA_DEVICES_AT_MS = now;
    }
  }

  const devices = Array.isArray(resp?.devices) ? resp.devices : [];
  const recRaw = resp?.recommended_device_id;
  const recommended = recRaw == null ? null : Math.max(0, Math.floor(Number(recRaw)));

  if (!devices.length) {
    if (sel.options.length !== 1 || sel.options[0]?.value !== "0") {
      sel.innerHTML = "";
      const opt = document.createElement("option");
      opt.value = "0";
      opt.textContent = "Auto";
      sel.appendChild(opt);
    }
    if (!sel.dataset.desired) sel.dataset.desired = "0";
    sel.value = "0";
    return;
  }

  const desired = desiredDeviceId();
  const ids = devices.map((d) => Math.max(0, Math.floor(Number(d?.device_id ?? 0))));

  sel.innerHTML = "";
  for (const d of devices) {
    const id = Math.max(0, Math.floor(Number(d?.device_id ?? 0)));
    const name = String(d?.name || `GPU ${id}`).trim() || `GPU ${id}`;
    const mem = formatBytesCompact(d?.total_memory_bytes);
    const opt = document.createElement("option");
    opt.value = String(id);
    const parts = [];
    if (mem) parts.push(mem);
    parts.push(`#${id}`);
    opt.textContent = `${name} (${parts.join(" • ")})${recommended != null && id === recommended ? " (recommended)" : ""}`;
    sel.appendChild(opt);
  }

  let pick = ids.includes(desired) ? desired : null;
  if (pick == null && recommended != null && ids.includes(recommended)) pick = recommended;
  if (pick == null) pick = ids[0] || 0;

  sel.dataset.desired = String(pick);
  sel.value = String(pick);
}

async function refreshGpuStatusUi() {
  const el = $("gpuStatus");
  if (!el) return;

  const invoke = tauriInvoke();
  if (!invoke) {
    el.textContent = "GPU: (Tauri invoke not available)";
    return;
  }

  let status = null;
  try {
    const now = typeof performance !== "undefined" ? performance.now() : Date.now();
    if (LAST_GPU_STATUS && (now - LAST_GPU_STATUS_AT_MS) < 750) {
      status = LAST_GPU_STATUS;
    } else {
      status = await invoke("get_gpu_status");
      if (status && typeof status === "object") {
        LAST_GPU_STATUS = status;
        LAST_GPU_STATUS_AT_MS = now;
      }
    }
  } catch (e) {
    el.textContent = `GPU: status failed (${String(e)})`;
    return;
  }

  const s = LAST_GPU_STATUS && typeof LAST_GPU_STATUS === "object" ? LAST_GPU_STATUS : null;
  const builtCuda = !!s?.built_with_cuda;
  const builtKernel = !!s?.built_with_cuda_backtest_kernel;
  const cudaOk = !!s?.cuda_available;

  applyCudaBackendAvailabilityConstraints(s);
  await refreshCudaDevicesUi({ status: s });
  updateDeviceIdEnabled();

  const buildLabel = builtCuda ? (builtKernel ? "cuda+kernel" : "cuda") : "cpu-only build";
  const backendSel = String($("backend")?.value || "CpuOnly");
  const deviceId = Math.max(0, Math.floor(num($("deviceId")?.value, 0)));
  const lastUsed = String(CURRENT_RUN?.res?.backend_used || "").trim() || "-";

  const fastMa = String($("fastMaType")?.value || "").trim();
  const slowMa = String($("slowMaType")?.value || "").trim();
  const exitMode = String($("exitMode")?.value || "cross").trim() || "cross";
  const cooldownBars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
  const signalPersistenceBars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
  const trailingStopPct = parseRiskPctInput("trailingStopPct");
  const hasExecutionOverlays =
    cooldownBars > 0 || signalPersistenceBars > 1 || trailingStopPct != null;
  const kind = strategyKind();
  const kernelStrategyOk = isDoubleLikeKind(kind);
  const kernelOk = builtCuda
    && builtKernel
    && cudaOk
    && kernelStrategyOk
    && exitMode === "cross"
    && !hasExecutionOverlays
    && fastMa
    && slowMa
    && maSupportsKernel(fastMa)
    && maSupportsKernel(slowMa);
  const sweepOk = builtCuda && cudaOk && fastMa && slowMa && maSupportsCuda(fastMa) && maSupportsCuda(slowMa);

  const parts = [];
  parts.push(`build=${buildLabel}`);
  parts.push(`cuda_available=${builtCuda ? (cudaOk ? "true" : "false") : "n/a"}`);
  parts.push(`backend_sel=${backendSel}`);
  if (backendSel === "GpuOnly" || backendSel === "GpuKernel") parts.push(`device_id=${deviceId}`);
  if (lastUsed !== "-") parts.push(`last_used=${lastUsed}`);

  const wantsGpu = backendSel === "GpuOnly" || backendSel === "GpuKernel";
  let hint = "";
  if (!builtCuda && wantsGpu) {
    hint = " (rebuild with --features cuda)";
  } else if (builtCuda && !cudaOk && wantsGpu) {
    hint = " (CUDA runtime unavailable)";
  } else if (backendSel === "GpuKernel" && !kernelStrategyOk) {
    hint = " (kernel is available for Double MA, Composite Trend-Quality, Regime-Switch Hybrid, and Breakout Pulse)";
  } else if (backendSel === "GpuKernel" && exitMode !== "cross") {
    hint = " (kernel requires exit mode = cross)";
  } else if (backendSel === "GpuKernel" && hasExecutionOverlays) {
    hint = " (kernel does not support cooldown/persistence/trailing-stop overlays)";
  } else if (backendSel === "GpuKernel" && !kernelOk) {
    hint = " (kernel not available for selected MA types/build)";
  } else if (backendSel === "GpuOnly" && !sweepOk) {
    hint = " (GPU sweep not available for selected MA types/build)";
  }

  el.textContent = `GPU: ${parts.join(" | ")}${hint}`;
  updateRunPlanUi();
}

function buildTinyDoubleMaRequestForParity(backendObj) {
  const dataIdText = String($("dataId")?.textContent || "").trim();
  const data_id = dataIdText === "—" ? "" : dataIdText;
  if (!data_id || data_id === "-") throw new Error("Load CSV first (missing data id).");

  const meta = CURRENT_DATA_META;
  if (!meta || typeof meta !== "object") throw new Error("Load CSV first.");

  const fastMaType = String($("fastMaType")?.value || "").trim();
  const slowMaType = String($("slowMaType")?.value || "").trim();
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");

  const maSource = String($("maSource")?.value || "close");
  validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource);

  const f0 = Math.max(1, Math.floor(num($("fastStart")?.value, 5)));
  const s0raw = Math.max(1, Math.floor(num($("slowStart")?.value, 20)));
  const s0 = Math.max(s0raw, f0 + 1);
  const f1 = f0 + 2;
  const s1 = s0 + 3;
  const fast_range = [f0, f1, 1];
  const slow_range = [s0, s1, 1];

  const objective = String($("objective")?.value || "Sharpe");

  const strategy = readStrategyCfgFromUi();

  const req = {
    strategy_kind: "DoubleMa",
    backend: backendObj,
    data_id,
    starting_equity: readStartingEquityFromUi(),
    fast_range,
    slow_range,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    export_all_csv_path: null,
    fast_ma_params: resolvedMaParams("fast", fastMaType),
    slow_ma_params: resolvedMaParams("slow", slowMaType),
    strategy,
    objective,
    mode: "Grid",
    top_k: 8,
    include_all: false,
    heatmap_bins: 0,
  };

  // Validate that the tiny grid actually has combos.
  const fastPeriods = expandRangeU16Like(fast_range[0], fast_range[1], fast_range[2]);
  const slowPeriods = expandRangeU16Like(slow_range[0], slow_range[1], slow_range[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  if (!pairs) throw new Error("Parity grid has no valid combinations (fast must be < slow). Adjust ranges.");

  return req;
}

async function runGpuParityCheck() {
  const metaEl = $("gpuParityMeta");
  if (!metaEl) return;

  const invoke = tauriInvoke();
  if (!invoke) {
    metaEl.textContent = "Parity: (Tauri invoke not available)";
    return;
  }
  if (RUNNING) return;
  if (strategyKind() !== "DoubleMa") {
    metaEl.textContent = "Parity: switch Strategy to Double MA to run the check.";
    return;
  }

  metaEl.textContent = "Parity: checking GPU availability...";
  await refreshGpuStatusUi();
  const s = LAST_GPU_STATUS && typeof LAST_GPU_STATUS === "object" ? LAST_GPU_STATUS : null;
  const builtCuda = !!s?.built_with_cuda;
  const builtKernel = !!s?.built_with_cuda_backtest_kernel;
  const cudaOk = !!s?.cuda_available;
  if (!builtCuda) {
    metaEl.textContent = "Parity: unavailable (app not built with CUDA).";
    return;
  }
  if (!cudaOk) {
    metaEl.textContent = "Parity: unavailable (CUDA runtime not available).";
    return;
  }

  const device_id = Math.max(0, Math.floor(num($("deviceId")?.value, 0)));
  const fastMa = String($("fastMaType")?.value || "").trim();
  const slowMa = String($("slowMaType")?.value || "").trim();
  const exitMode = String($("exitMode")?.value || "cross").trim() || "cross";
  const cooldownBars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
  const signalPersistenceBars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
  const trailingStopPct = parseRiskPctInput("trailingStopPct");
  const hasExecutionOverlays =
    cooldownBars > 0 || signalPersistenceBars > 1 || trailingStopPct != null;

  let gpuBackend = null;
  if (builtKernel && exitMode === "cross" && !hasExecutionOverlays && fastMa && slowMa && maSupportsKernel(fastMa) && maSupportsKernel(slowMa)) {
    gpuBackend = "GpuKernel";
  } else if (fastMa && slowMa && maSupportsCuda(fastMa) && maSupportsCuda(slowMa)) {
    gpuBackend = "GpuOnly";
  }

  if (!gpuBackend) {
    metaEl.textContent = "Parity: unavailable (selected MAs do not support CUDA sweep/kernel).";
    return;
  }

  const cpuReq = buildTinyDoubleMaRequestForParity({ backend: "CpuOnly" });
  const gpuReq = buildTinyDoubleMaRequestForParity({ backend: gpuBackend, device_id });

  const tol = 1e-4;
  const wasRunning = RUNNING;
  setRunning(true);
  try {
    clearProgressUi();
    setStatus("Running CPU ↔ GPU parity check (tiny grid)...", "muted");

    setProgressUi(0, 1, "parity_cpu", "");
    const cpuRes = await invoke("run_double_ma_optimization", { req: cpuReq });
    setProgressUi(1, 1, "parity_cpu", "");

    setProgressUi(0, 1, gpuBackend === "GpuKernel" ? "parity_gpu_kernel" : "parity_gpu_sweep", "");
    const gpuRes = await invoke("run_double_ma_optimization", { req: gpuReq });
    setProgressUi(1, 1, gpuBackend === "GpuKernel" ? "parity_gpu_kernel" : "parity_gpu_sweep", "");

    const cm = cpuRes?.best_metrics && typeof cpuRes.best_metrics === "object" ? cpuRes.best_metrics : {};
    const gm = gpuRes?.best_metrics && typeof gpuRes.best_metrics === "object" ? gpuRes.best_metrics : {};
    const dp = Math.abs(Number(cm.pnl || 0) - Number(gm.pnl || 0));
    const ds = Math.abs(Number(cm.sharpe || 0) - Number(gm.sharpe || 0));
    const dd = Math.abs(Number(cm.max_dd || 0) - Number(gm.max_dd || 0));

    const cpuP = cpuRes?.best_params && typeof cpuRes.best_params === "object" ? cpuRes.best_params : {};
    const gpuP = gpuRes?.best_params && typeof gpuRes.best_params === "object" ? gpuRes.best_params : {};
    const sameParams = JSON.stringify(cpuP) === JSON.stringify(gpuP);

    const ok = dp <= tol && ds <= tol && dd <= tol;
    const paramsHint = sameParams ? "" : " (best_params differ; may be tie)";
    metaEl.textContent =
      `Parity: ${ok ? "OK" : "CHECK"} • gpu=${gpuBackend} • |Δpnl|=${fmt(dp, 6)} |Δsharpe|=${fmt(ds, 6)} |Δdd|=${fmt(dd, 6)} tol=${tol}${paramsHint}`;
    setStatus(ok ? "Parity check OK." : "Parity check completed (review diffs).", ok ? "ok" : "muted");
  } catch (e) {
    if (isCancelledError(e)) metaEl.textContent = "Parity: cancelled.";
    else metaEl.textContent = `Parity: failed (${String(e)})`;
    throw e;
  } finally {
    if (!wasRunning) clearProgressUi();
    setRunning(false);
    refreshGpuStatusUi();
  }
}

function strategyKind() {
  const sel = $("strategyKind");
  let v = String(sel?.value || "DoubleMa").trim() || "DoubleMa";

  // Migrate legacy CompositeCpu selection to full-parameter CompositeMo mode.
  if (v === "CompositeCpu") {
    v = "CompositeMo";
    if (sel) sel.value = "CompositeMo";
  }

  return v;
}

function normalizeStrategyKindRaw(kindRaw) {
  const k = String(kindRaw || "DoubleMa").trim() || "DoubleMa";
  if (k === "CompositeCpu") return "CompositeCpu";
  if (k === "CompositeMo") return "CompositeMo";
  if (k === "CompositeTrend") return "CompositeTrend";
  if (k === "RegimeHybrid") return "RegimeHybrid";
  if (k === "BreakoutMomentum") return "BreakoutMomentum";
  return "DoubleMa";
}

function isDoubleLikeKind(kind) {
  const k = String(kind || "").trim();
  return k === "DoubleMa" || k === "CompositeTrend" || k === "RegimeHybrid" || k === "BreakoutMomentum";
}

function isCompositeStrategy() {
  const k = strategyKind();
  return k === "CompositeCpu" || k === "CompositeMo" || k === "CompositeTrend";
}

function countF64RangeValues(start, end, step) {
  const a = Number(start);
  const b = Number(end);
  const s = Number(step);
  if (!Number.isFinite(a) || !Number.isFinite(b) || !Number.isFinite(s)) return 1;
  if (s === 0 || a === b) return 1;
  const lo = Math.min(a, b);
  const hi = Math.max(a, b);
  const st = Math.abs(s);
  if (!(st > 0) || !Number.isFinite(st)) return 1;
  const n = Math.floor((hi - lo) / st) + 1;
  return Math.max(1, Math.min(1_000_000, n));
}

function estimateMaParamVariantCount(side, maId) {
  const sweeps = resolvedMaParamSweeps(side, maId);
  if (!sweeps || typeof sweeps !== "object") return 1;
  let total = 1;
  for (const v of Object.values(sweeps)) {
    const arr = Array.isArray(v) ? v : null;
    if (!arr || arr.length < 3) continue;
    total *= countF64RangeValues(arr[0], arr[1], arr[2]);
    if (total > 1_000_000) return 1_000_000;
  }
  return Math.max(1, total);
}

function updatePairsPreviewUi() {
  const el = $("pairsPreview");
  if (!el) return;

  const fast = [
    Math.max(1, Math.floor(num($("fastStart")?.value, 5))),
    Math.max(1, Math.floor(num($("fastEnd")?.value, 30))),
    Math.floor(num($("fastStep")?.value, 1)),
  ];
  const slow = [
    Math.max(1, Math.floor(num($("slowStart")?.value, 20))),
    Math.max(1, Math.floor(num($("slowEnd")?.value, 120))),
    Math.floor(num($("slowStep")?.value, 1)),
  ];

  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);

  const fastMaType = String($("fastMaType")?.value || "").trim();
  const slowMaType = String($("slowMaType")?.value || "").trim();
  const fastVar = fastMaType ? estimateMaParamVariantCount("fast", fastMaType) : 1;
  const slowVar = slowMaType ? estimateMaParamVariantCount("slow", slowMaType) : 1;
  const variantPairs = Math.max(1, fastVar) * Math.max(1, slowVar);
  const combos = Number(pairs || 0) * variantPairs;

  const includeAll = String($("includeAll")?.value || "false") === "true";
  const tooManyIncludeAll = includeAll && combos > 2_000_000;

  const pairsText = pairs ? pairs.toLocaleString() : "0";
  const combosText = combos ? combos.toLocaleString() : "0";
  const varText = variantPairs > 1 ? ` • combos≈${combosText} (pairs×MA variants: ${fastVar}×${slowVar})` : "";
  el.textContent = tooManyIncludeAll
    ? `Pairs: ${pairsText} (fast < slow)${varText} • includeAll will be rejected above 2,000,000 combos`
    : `Pairs: ${pairsText} (fast < slow)${varText}`;
}

function runButtonLabelForKind(kind) {
  if (kind === "CompositeMo") return "Run optimization (Composite MO)";
  if (kind === "CompositeCpu") return "Run optimization (Composite CPU)";
  if (kind === "CompositeTrend") return "Run optimization (Composite Trend-Quality)";
  if (kind === "RegimeHybrid") return "Run optimization (Regime-Switch Hybrid)";
  if (kind === "BreakoutMomentum") return "Run optimization (Breakout Pulse)";
  return "Run optimization (Double MA)";
}

function appendRunReadinessBadge(host, label, value, tone, title) {
  if (!host) return;
  const badge = document.createElement("span");
  const cls = tone === "on" ? "on" : (tone === "warn" ? "warn" : "off");
  badge.className = `badge ${cls}`;
  badge.textContent = `${label}: ${value}`;
  if (title) badge.title = title;
  host.appendChild(badge);
}

function selectedValidationChecksSummary() {
  const checks = [];
  if (!!$("valWfEnable")?.checked) checks.push("WF");
  if (!!$("valPerturbEnable")?.checked) checks.push("perturb");
  if (!!$("valMcEnable")?.checked) checks.push("MC");
  if (!!$("valCscvEnable")?.checked) checks.push("CSCV");
  if (!!$("valCrossEnable")?.checked) {
    const n = getCrossSelectedDatasets().length;
    checks.push(n ? `cross(${n})` : "cross");
  }
  return checks;
}

function tryBuildRunRequestPreview() {
  const kind = strategyKind();
  try {
    if (kind === "CompositeCpu" || kind === "CompositeMo") return getRequestForFingerprint();
    return getDoubleMaRequest();
  } catch {
    return null;
  }
}

function updateRunReadinessUi() {
  const host = $("runReadiness");
  const hint = $("runReadinessHint");
  const btnRun = $("btnRun");
  const btnQaRun = $("btnQaRun");
  if (!host && !hint && !btnRun && !btnQaRun) return;

  const kind = strategyKind();
  const blockers = [];
  const warnings = [];
  const pushUnique = (arr, text) => {
    const s = String(text || "").trim();
    if (!s) return;
    if (!arr.includes(s)) arr.push(s);
  };

  if (btnRun) {
    btnRun.textContent = RUNNING ? "Running..." : runButtonLabelForKind(kind);
  }

  const dataIdText = String($("dataId")?.textContent || "").trim();
  const dataReady = !!dataIdText && dataIdText !== "—" && dataIdText !== "-";
  const meta = CURRENT_DATA_META && typeof CURRENT_DATA_META === "object" ? CURRENT_DATA_META : null;
  const fastMaType = String($("fastMaType")?.value || "").trim();
  const slowMaType = String($("slowMaType")?.value || "").trim();
  const maSource = String($("maSource")?.value || "close").trim() || "close";

  const fast = [
    Math.max(1, Math.floor(num($("fastStart")?.value, 5))),
    Math.max(1, Math.floor(num($("fastEnd")?.value, 30))),
    Math.floor(num($("fastStep")?.value, 1)),
  ];
  const slow = [
    Math.max(1, Math.floor(num($("slowStart")?.value, 20))),
    Math.max(1, Math.floor(num($("slowEnd")?.value, 120))),
    Math.floor(num($("slowStep")?.value, 1)),
  ];
  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  const includeAll = String($("includeAll")?.value || "false") === "true";

  if (!dataReady) pushUnique(blockers, "Load CSV first (missing data id).");
  if (!fastMaType) pushUnique(blockers, "Select a fast MA type.");
  if (!slowMaType) pushUnique(blockers, "Select a slow MA type.");
  if (!pairs) pushUnique(blockers, "No valid parameter combinations (fast must be < slow).");

  let backendValue = "CPU";
  let backendTone = "on";
  if (isDoubleLikeKind(kind)) {
    const backend = String($("backend")?.value || "CpuOnly").trim() || "CpuOnly";
    const exitMode = String($("exitMode")?.value || "cross").trim() || "cross";
    const cooldownBars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
    const signalPersistenceBars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
    const trailingStopPct = parseRiskPctInput("trailingStopPct");
    const hasExecutionOverlays =
      cooldownBars > 0 || signalPersistenceBars > 1 || trailingStopPct != null;
    const kernelStrategyOk = isDoubleLikeKind(kind);
    backendValue = backend;
    let backendOk = true;
    if (fastMaType && slowMaType) {
      if (backend === "CpuOnly") {
        backendOk = maSupportsCpu(fastMaType) && maSupportsCpu(slowMaType);
        if (!backendOk) pushUnique(blockers, "CPU backend does not support one of the selected MA types.");
      } else if (backend === "GpuOnly") {
        backendOk = maSupportsCuda(fastMaType) && maSupportsCuda(slowMaType);
        if (!backendOk) pushUnique(blockers, "GPU backend requires CUDA support for both selected MAs.");
      } else if (backend === "GpuKernel") {
        backendOk = kernelStrategyOk && maSupportsKernel(fastMaType) && maSupportsKernel(slowMaType);
        if (!kernelStrategyOk) {
          backendOk = false;
          pushUnique(blockers, "GPU (kernel) is available for Double MA, Composite Trend-Quality, Regime-Switch Hybrid, and Breakout Pulse only.");
        }
        if (exitMode !== "cross") {
          backendOk = false;
          pushUnique(blockers, "GPU (kernel) currently supports exit mode = cross only.");
        }
        if (hasExecutionOverlays) {
          backendOk = false;
          pushUnique(blockers, "GPU (kernel) does not support cooldown/persistence/trailing-stop overlays.");
        }
        if (!backendOk && exitMode === "cross" && kernelStrategyOk && !hasExecutionOverlays) {
          pushUnique(blockers, "GPU (kernel) does not support one of the selected MA types.");
        }
      } else {
        const okFast = maSupportsCpu(fastMaType) || maSupportsCuda(fastMaType) || maSupportsKernel(fastMaType);
        const okSlow = maSupportsCpu(slowMaType) || maSupportsCuda(slowMaType) || maSupportsKernel(slowMaType);
        backendOk = okFast && okSlow;
        if (!backendOk) pushUnique(blockers, "Auto backend cannot run the selected MA(s).");
      }
    }

    const s = LAST_GPU_STATUS && typeof LAST_GPU_STATUS === "object" ? LAST_GPU_STATUS : null;
    if (backend === "GpuOnly" || backend === "GpuKernel") {
      if (s) {
        if (!s.built_with_cuda || !s.cuda_available) {
          backendOk = false;
          pushUnique(blockers, "CUDA backend is unavailable on this machine.");
        }
        if (backend === "GpuKernel" && !s.built_with_cuda_backtest_kernel) {
          backendOk = false;
          pushUnique(blockers, "This build is missing cuda-backtest-kernel support.");
        }
      } else {
        pushUnique(warnings, "GPU status is unknown (use Refresh GPU status).");
      }
    }
    backendTone = backendOk ? "on" : "off";
  } else if (kind === "CompositeMo") {
    const prescanOn = !!$("moPrescanEnable")?.checked;
    backendValue = prescanOn ? "CPU + GPU pre-scan" : "CPU";
    if (prescanOn) {
      const requireGpu = !!$("moPrescanRequireGpu")?.checked;
      const s = LAST_GPU_STATUS && typeof LAST_GPU_STATUS === "object" ? LAST_GPU_STATUS : null;
      if (requireGpu) {
        if (s) {
          const ok = !!s.built_with_cuda && !!s.cuda_available;
          if (!ok) pushUnique(blockers, "Pre-scan requires a CUDA-capable GPU.");
          backendTone = ok ? "on" : "off";
        } else {
          backendTone = "warn";
          pushUnique(warnings, "Pre-scan requires GPU, but status is unknown.");
        }
      } else {
        backendTone = "warn";
      }
    }
  }

  if (dataReady && meta && fastMaType && slowMaType) {
    try {
      validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource);
    } catch (e) {
      pushUnique(blockers, String(e));
    }
    if ((kind === "CompositeCpu" || kind === "CompositeMo" || kind === "RegimeHybrid" || kind === "BreakoutMomentum") && (!meta.has_high || !meta.has_low)) {
      pushUnique(
        blockers,
        kind === "RegimeHybrid"
          ? "Regime-Switch Hybrid requires high/low candle fields."
          : (kind === "BreakoutMomentum"
            ? "Breakout Pulse requires high/low candle fields."
            : "Composite strategy requires high/low candle fields.")
      );
    }
    if (kind === "BreakoutMomentum" && !meta.has_volume) {
      pushUnique(blockers, "Breakout Pulse requires volume candle field.");
    }
  }

  const fastVar = fastMaType ? estimateMaParamVariantCount("fast", fastMaType) : 1;
  const slowVar = slowMaType ? estimateMaParamVariantCount("slow", slowMaType) : 1;
  const combos = Math.max(0, Math.floor(pairs * Math.max(1, fastVar) * Math.max(1, slowVar)));
  if (isDoubleLikeKind(kind) && includeAll && combos > 2_000_000) {
    pushUnique(blockers, `includeAll=true would return ${combos} rows; disable it or reduce ranges/sweeps.`);
  } else if (kind === "CompositeCpu" && includeAll && pairs > 2_000_000) {
    pushUnique(blockers, `includeAll=true would return ${pairs} rows; disable it or reduce ranges.`);
  }

  const sl = parseRiskPctInput("stopLossPct");
  const tp = parseRiskPctInput("takeProfitPct");
  const trail = parseRiskPctInput("trailingStopPct");
  const cooldownBars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
  const signalPersistenceBars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
  const slRaw = String($("stopLossPct")?.value || "").trim();
  const tpRaw = String($("takeProfitPct")?.value || "").trim();
  const trailRaw = String($("trailingStopPct")?.value || "").trim();
  if (slRaw && sl == null) pushUnique(warnings, "Stop-loss input is invalid and will be ignored.");
  if (tpRaw && tp == null) pushUnique(warnings, "Take-profit input is invalid and will be ignored.");
  if (trailRaw && trail == null) pushUnique(warnings, "Trailing-stop input is invalid and will be ignored.");
  if (!sl && !tp && !trail && cooldownBars === 0 && signalPersistenceBars <= 1) {
    pushUnique(warnings, "Risk/entry overlays are disabled.");
  }

  const supportsVal = kind === "CompositeCpu" || kind === "CompositeMo" || kind === "DoubleMa" || kind === "CompositeTrend" || kind === "RegimeHybrid" || kind === "BreakoutMomentum";
  const autoVal = supportsVal && !!$("valAutoRun")?.checked;
  const checks = selectedValidationChecksSummary();
  if (supportsVal && !autoVal) pushUnique(warnings, "Auto-run validation is disabled.");

  const previewReq = tryBuildRunRequestPreview();
  const workLabel = previewReq ? estimateOptimizationWorkLabel(previewReq) : "";
  const searchValue = workLabel || (pairs > 0 ? `${formatCompactNumber(pairs)} MA pairs` : "invalid range");
  const searchTone = pairs > 0 ? "on" : "off";
  const riskParts = [];
  if (sl) riskParts.push(`SL ${fmtFixed(sl * 100, 2)}%`);
  if (tp) riskParts.push(`TP ${fmtFixed(tp * 100, 2)}%`);
  if (trail) riskParts.push(`Trail ${fmtFixed(trail * 100, 2)}%`);
  if (cooldownBars > 0) riskParts.push(`CD ${cooldownBars}b`);
  if (signalPersistenceBars > 1) riskParts.push(`Persist ${signalPersistenceBars}b`);
  const riskValue = riskParts.length ? riskParts.join(" / ") : "off";
  const riskTone = riskParts.length ? "on" : "warn";
  const valValue = autoVal
    ? (checks.length ? `auto (${checks.join("+")})` : "auto")
    : "manual";
  const valTone = autoVal ? "on" : "warn";

  if (host) {
    host.innerHTML = "";
    appendRunReadinessBadge(host, "Data", dataReady ? "loaded" : "missing", dataReady ? "on" : "off");
    appendRunReadinessBadge(host, "Search", searchValue, searchTone);
    appendRunReadinessBadge(host, "Backend", backendValue, backendTone);
    appendRunReadinessBadge(host, "Risk", riskValue, riskTone);
    appendRunReadinessBadge(host, "Validation", valValue, valTone);
  }

  const canRun = !RUNNING && blockers.length === 0;
  LAST_RUN_READINESS = { can_run: canRun, blockers: blockers.slice(), warnings: warnings.slice() };

  if (btnRun) {
    btnRun.disabled = !canRun;
    btnRun.title = blockers.length ? blockers[0] : "";
  }
  if (btnQaRun) {
    btnQaRun.disabled = !canRun;
    btnQaRun.title = blockers.length ? blockers[0] : "";
  }

  if (hint) {
    if (RUNNING) hint.textContent = "Readiness: run in progress.";
    else if (blockers.length) hint.textContent = `Readiness: blocked — ${blockers[0]}`;
    else if (warnings.length) hint.textContent = `Readiness: ready — ${warnings[0]}`;
    else hint.textContent = "Readiness: ready.";
  }
}

function updateRunPlanUi() {
  const el = $("runPlanMeta");
  if (!el) return;

  const kind = strategyKind();
  const steps = [];
  const parts = [];

  if (kind === "CompositeMo") {
    const prescan = !!$("moPrescanEnable")?.checked;
    steps.push(prescan ? "Optimize (MO + GPU pre-scan)" : "Optimize (MO)");
  } else if (kind === "CompositeCpu") {
    steps.push("Optimize (Composite)");
  } else {
    const backend = String($("backend")?.value || "CpuOnly");
    const b = backend === "CpuOnly" ? "CPU" : (backend === "Auto" ? "Auto" : backend);
    steps.push(`Optimize (${b})`);
  }

  const supportsVal = kind === "CompositeCpu" || kind === "CompositeMo" || kind === "DoubleMa" || kind === "CompositeTrend" || kind === "RegimeHybrid" || kind === "BreakoutMomentum";
  const autoVal = !!$("valAutoRun")?.checked && supportsVal;
  const autoXlsx = false;
  const autoReveal = false;

  if (autoVal) steps.push("Validate (auto)");
  if (autoXlsx) steps.push("Export XLSX (auto)");
  if (autoReveal) steps.push("Reveal report (auto)");

  parts.push(`Plan: ${steps.join(" → ")}`);

  const objective = kind === "CompositeMo"
    ? `obj=max ${String($("objective")?.value || "Sharpe")} / min ${String($("moObjectiveMin")?.value || "MaxDrawdown")}`
    : `obj=${String($("objective")?.value || "Sharpe")}`;
  parts.push(objective);

  const sl = parseRiskPctInput("stopLossPct");
  const tp = parseRiskPctInput("takeProfitPct");
  const trail = parseRiskPctInput("trailingStopPct");
  const cooldownBars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
  const signalPersistenceBars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
  const exitMode = String($("exitMode")?.value || "cross").trim() || "cross";
  const kPeriod = Math.max(1, Math.floor(num($("kaufmanStopPeriod")?.value, 22)));
  const kMult = Math.max(0.0001, num($("kaufmanStopMult")?.value, 2.0));
  const kMa = String($("kaufmanStopMaType")?.value || "sma").trim() || "sma";
  const riskParts = [];
  if (sl) riskParts.push(`SL ${fmtFixed(sl * 100, 2)}%`);
  if (tp) riskParts.push(`TP ${fmtFixed(tp * 100, 2)}%`);
  if (trail) riskParts.push(`Trail ${fmtFixed(trail * 100, 2)}%`);
  if (cooldownBars > 0) riskParts.push(`cooldown=${cooldownBars}b`);
  if (signalPersistenceBars > 1) riskParts.push(`persist=${signalPersistenceBars}b`);
  const riskText = riskParts.length ? `risk=${riskParts.join(", ")}` : "risk=none";
  const exitText = exitMode === "kaufman_stop"
    ? `exit=Kaufman-stop (p=${kPeriod}, m=${fmtFixed(kMult, 2)}, ma=${kMa})`
    : "exit=cross";
  parts.push(`${riskText} • ${exitText}`);
  if (isDoubleLikeKind(kind)) {
    const minTrades = optNum($("dmConstraintMinTrades")?.value);
    const maxDd = optNum($("dmConstraintMaxDd")?.value);
    const cParts = [];
    if (Number.isFinite(minTrades) && minTrades > 0) cParts.push(`trades>=${Math.floor(minTrades)}`);
    if (Number.isFinite(maxDd) && maxDd >= 0) cParts.push(`dd<=${fmtFixed(maxDd * 100, 2)}%`);
    if (cParts.length) parts.push(`filters=${cParts.join(",")}`);
  }

  const req = tryBuildRunRequestPreview();
  const work = req ? estimateOptimizationWorkLabel(req) : "";
  if (work) parts.push(`work=${work}`);

  // Quick reminder when validate is off.
  if (supportsVal && !autoVal) {
    parts.push("tip: enable Auto-run validation for hands-off robustness");
  }

  el.textContent = parts.join(" • ");
  updateRunReadinessUi();
}

function updateValidateTabUi() {
  const btn = $("btnValidate");
  const hint = $("validationHint");
  if (!btn && !hint) return;

  const hasRun = !!(CURRENT_RUN?.req && CURRENT_RUN?.res);
  const runKind = hasRun ? kindFromReq(CURRENT_RUN.req) : "";
  const canValidate = hasRun && (runKind === "CompositeCpu" || runKind === "CompositeMo" || runKind === "DoubleMa" || runKind === "CompositeTrend" || runKind === "RegimeHybrid" || runKind === "BreakoutMomentum");

  if (btn) btn.disabled = RUNNING || !canValidate;

  if (!hint) return;
  if (RUNNING) {
    hint.textContent = "Validation: disabled while a task is running.";
    return;
  }
  if (!hasRun) {
    hint.textContent = "Run an optimization first to enable validation.";
    return;
  }
  if (!canValidate) {
    hint.textContent = "Validation is not supported for this run.";
    return;
  }

  const maxCandidates = Math.max(1, Math.floor(num($("valCandidates")?.value, 32)));
  const checks = selectedValidationChecksSummary();
  const checksText = checks.length ? ` • checks=${checks.join("+")}` : "";
  hint.textContent = `Ready: validate up to ${maxCandidates} candidates from Top/Pareto.${checksText}`;
}

function updateQuickActionsUi() {
  const btnReport = $("btnOpenLatestReport");
  const btnMeta = $("btnOpenLatestMetaReport");
  const meta = $("quickActionsMeta");
  if (!btnReport && !btnMeta && !meta) return;

  const invoke = tauriInvoke();
  const okInvoke = !!invoke;

  const reportPath = String(CURRENT_RUN?.report_xlsx_path || "").trim();
  const metaReportPath = String(LAST_META_RUN?.report_xlsx_path || "").trim();

  if (btnReport) btnReport.disabled = RUNNING || !okInvoke || !reportPath;
  if (btnMeta) btnMeta.disabled = RUNNING || !okInvoke || !metaReportPath;

  if (meta) {
    const parts = [];
    if (reportPath) parts.push(`report=${fileBasename(reportPath)}`);
    else parts.push("report=—");
    if (metaReportPath) parts.push(`meta=${fileBasename(metaReportPath)}`);
    else parts.push("meta=—");
    if (!okInvoke) parts.push("backend unavailable");
    meta.textContent = parts.join(" • ");
  }

  updateQuickBarUi();
}

function applyBackendSelectionConstraints() {
  updateStrategyUi();
}

function updateStrategyUi() {
  const kind = strategyKind();
  const isDoubleLike = isDoubleLikeKind(kind);
  const isDoubleMa = kind === "DoubleMa";
  const isCompTrend = kind === "CompositeTrend";
  const isRegimeHybrid = kind === "RegimeHybrid";
  const isBreakoutMomentum = kind === "BreakoutMomentum";
  const isCompCpu = kind === "CompositeCpu";
  const isCompMo = kind === "CompositeMo";
  const isComp = isCompCpu || isCompMo;
  const compModeSpec = compositeSignalModeSpec(readCompositeSignalMode());
  const exitMode = String($("exitMode")?.value || "cross").trim() || "cross";
  const useKaufmanExit = isDoubleLike && exitMode === "kaufman_stop";

  if (isCompMo && String(document.documentElement.dataset.workflowAdvanced || "").trim() !== "1") {
    setWorkflowAdvancedEnabled(true, { persist: true });
  }

  const hint = $("strategyHint");
  if (hint) {
    hint.textContent = isCompCpu
      ? "Legacy fixed-parameter mode. This selection is auto-upgraded to full composite optimization."
      : isCompTrend
      ? "Double MA with trend-quality regime filters (LinearReg angle + BB width + optional ROC/ADX confirmation)."
      : isRegimeHybrid
      ? "Hybrid: trend-follow in strong regimes, mean-reversion in range regimes (hysteresis-enabled ADX switching)."
      : isBreakoutMomentum
      ? "Breakout Pulse: MA crossover gated by BB breakout + MACD + CMO + ADOSC (optional ADX)."
      : isCompMo
      ? `Optimizes MA + HalfTrend + MACZ + OTT parameters. Current variant: ${compModeSpec.title}.`
      : "Fast double moving-average crossover (CPU / GPU sweep / GPU kernel).";
  }

  const compWrap = $("compositeParamsWrap");
  if (compWrap) compWrap.style.display = isCompCpu ? "block" : "none";

  const compSignalWrap = $("compositeSignalModeWrap");
  if (compSignalWrap) compSignalWrap.style.display = isComp ? "block" : "none";
  if ($("compositeSignalMode")) $("compositeSignalMode").disabled = RUNNING || !isComp;
  const compSignalDetails = $("compositeSignalModeDetails");
  if (compSignalDetails) {
    compSignalDetails.textContent = isComp
      ? `${compModeSpec.title}: ${compModeSpec.details}`
      : "";
  }

  const compMoWrap = $("compositeMoWrap");
  if (compMoWrap) compMoWrap.style.display = isCompMo ? "block" : "none";
  const trendWrap = $("trendQualityWrap");
  const showTrendWrap = isCompTrend || isRegimeHybrid;
  const allowTrendAdx = isCompTrend;
  if (trendWrap) trendWrap.style.display = showTrendWrap ? "block" : "none";
  const trendTitle = $("trendQualityTitle");
  if (trendTitle) {
    trendTitle.textContent = isRegimeHybrid
      ? "Trend regime detector (used by Regime-Switch Hybrid)"
      : "Trend-quality filters";
  }
  for (const id of [
    "trendLrPeriod",
    "trendLrMinDeg",
    "trendBbwPeriod",
    "trendBbwMult",
    "trendBbwMin",
    "trendRocPeriod",
    "trendUseRoc",
  ]) {
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING || !showTrendWrap;
  }
  if ($("trendUseAdx")) $("trendUseAdx").disabled = RUNNING || !allowTrendAdx;
  const trendUseAdx = allowTrendAdx && !!$("trendUseAdx")?.checked;
  if ($("trendAdxPeriod")) $("trendAdxPeriod").disabled = RUNNING || !allowTrendAdx || !trendUseAdx;
  if ($("trendAdxMin")) $("trendAdxMin").disabled = RUNNING || !allowTrendAdx || !trendUseAdx;
  const regimeWrap = $("regimeHybridParamsWrap");
  if (regimeWrap) regimeWrap.style.display = isRegimeHybrid ? "block" : "none";
  for (const id of [
    "regimeAdxPeriod",
    "regimeAdxEnter",
    "regimeAdxExit",
    "regimeRevEntryRel",
    "regimeRevExitRel",
  ]) {
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING || !isRegimeHybrid;
  }
  const breakoutWrap = $("breakoutMomentumWrap");
  if (breakoutWrap) breakoutWrap.style.display = isBreakoutMomentum ? "block" : "none";
  for (const id of [
    "breakoutBbPeriod",
    "breakoutBbMult",
    "breakoutConfirmBars",
    "breakoutMacdFast",
    "breakoutMacdSlow",
    "breakoutMacdSignal",
    "breakoutCmoPeriod",
    "breakoutCmoMinAbs",
    "breakoutAdoscShort",
    "breakoutAdoscLong",
    "breakoutUseAdx",
  ]) {
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING || !isBreakoutMomentum;
  }
  const breakoutUseAdx = isBreakoutMomentum && !!$("breakoutUseAdx")?.checked;
  if ($("breakoutAdxPeriod")) $("breakoutAdxPeriod").disabled = RUNNING || !isBreakoutMomentum || !breakoutUseAdx;
  if ($("breakoutAdxMin")) $("breakoutAdxMin").disabled = RUNNING || !isBreakoutMomentum || !breakoutUseAdx;

  const dmConstraintsWrap = $("dmConstraintsWrap");
  if (dmConstraintsWrap) dmConstraintsWrap.style.display = isDoubleLike ? "" : "none";
  for (const id of ["dmConstraintMinTrades", "dmConstraintMaxDd"]) {
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING || !isDoubleLike;
  }

  const valAuto = $("valAutoRun");
  if (valAuto) valAuto.disabled = RUNNING || !(isComp || isDoubleLike);
  const valAutoXlsx = $("valAutoExportXlsx");
  if (valAutoXlsx) {
    valAutoXlsx.checked = false;
    valAutoXlsx.disabled = true;
  }
  const valAutoReveal = $("valAutoRevealReport");
  if (valAutoReveal) valAutoReveal.disabled = RUNNING || !(isComp || isDoubleLike);
  const valAutoTop = $("valAutoExportTopN");
  if (valAutoTop) valAutoTop.disabled = true;

  const backendSel = $("backend");
  if (backendSel) {
    backendSel.disabled = RUNNING || isComp;
    for (const opt of backendSel.options) {
      opt.disabled = isComp ? (opt.value !== "CpuOnly") : false;
    }
    if (isComp) backendSel.value = "CpuOnly";
  }

  const modeSel = $("mode");
  if (modeSel) {
    modeSel.disabled = RUNNING || isComp;
    if (isComp && modeSel.value !== "Grid") modeSel.value = "Grid";
  }
  const vdpsActive = !!(modeSel && modeSel.value === "GpuVdps" && isDoubleLike && !isComp);
  const vdpsWrap = $("vdpsConfigWrap");
  if (vdpsWrap) vdpsWrap.style.display = vdpsActive ? "block" : "none";
  for (const id of [
    "vdpsEta",
    "vdpsRungs",
    "vdpsMinInitialPairs",
    "vdpsMaxSurvivorSeeds",
    "vdpsMaxPeriodCandidates",
    "vdpsRandomInjectionRatio",
    "vdpsEliteMutationRatio",
    "vdpsDifferentialRatio",
    "vdpsSeed",
    "vdpsDeterministic",
    "vdpsQualityCheckMaxPairs",
    "vdpsQualityCheckTopK",
  ]) {
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING || !vdpsActive;
  }

  const presetBtn = $("btnPresetKernelSmaAlma");
  if (presetBtn) presetBtn.disabled = RUNNING || isComp;

  const refreshGpuBtn = $("btnRefreshGpuStatus");
  if (refreshGpuBtn) refreshGpuBtn.disabled = RUNNING;
  const parityBtn = $("btnGpuParityCheck");
  if (parityBtn) parityBtn.disabled = RUNNING || !isDoubleMa;
  const parityMeta = $("gpuParityMeta");
  if (parityMeta && !isDoubleMa) {
    parityMeta.textContent = "Parity: (available for Double MA only)";
  } else if (parityMeta && isDoubleMa && String(parityMeta.textContent || "").includes("available for Double MA")) {
    parityMeta.textContent = "Parity: —";
  }

  const heatmapBins = $("heatmapBins");
  if (heatmapBins) {
    heatmapBins.disabled = RUNNING || !isDoubleLike;
  }

  const includeAllSel = $("includeAll");
  if (includeAllSel) {
    includeAllSel.disabled = RUNNING || isCompMo;
    if (isCompMo) includeAllSel.value = "false";
  }

  const topKInput = $("topK");
  if (topKInput) {
    topKInput.disabled = RUNNING || isCompMo;
  }

  const exportInput = $("exportAllCsvPath");
  if (exportInput) exportInput.disabled = RUNNING || isComp;
  const exportBtn = $("btnBrowseExportAll");
  if (exportBtn) exportBtn.disabled = RUNNING || isComp;
  if (isComp && exportInput) exportInput.value = "";

  const heatmapTabBtn = document.querySelector('#resultsTabBar button[data-tab="heatmap"]');
  if (heatmapTabBtn) {
    heatmapTabBtn.disabled = RUNNING || !isDoubleLike;
    heatmapTabBtn.title = isDoubleLike ? "" : "Heatmap is available for Double-MA-family strategies only.";
  }
  if (!isDoubleLike && RESULTS_TABS?.get() === "heatmap") {
    RESULTS_TABS.set("overview", { save: false, focus: false });
  }

  const exitModeEl = $("exitMode");
  if (exitModeEl) exitModeEl.disabled = RUNNING || !isDoubleLike;
  const kPeriodWrap = $("kaufmanStopPeriodWrap");
  const kMultWrap = $("kaufmanStopMultWrap");
  const kMaWrap = $("kaufmanStopMaTypeWrap");
  if (kPeriodWrap) kPeriodWrap.style.display = useKaufmanExit ? "" : "none";
  if (kMultWrap) kMultWrap.style.display = useKaufmanExit ? "" : "none";
  if (kMaWrap) kMaWrap.style.display = useKaufmanExit ? "" : "none";
  if ($("kaufmanStopPeriod")) $("kaufmanStopPeriod").disabled = RUNNING || !useKaufmanExit;
  if ($("kaufmanStopMult")) $("kaufmanStopMult").disabled = RUNNING || !useKaufmanExit;
  if ($("kaufmanStopMaType")) $("kaufmanStopMaType").disabled = RUNNING || !useKaufmanExit;
  if ($("allowFlip")) {
    $("allowFlip").disabled = RUNNING || useKaufmanExit;
    if (useKaufmanExit) {
      $("allowFlip").title = "Allow flip is ignored when Exit mode = Kaufman-stop.";
    } else {
      $("allowFlip").title = "Allow switching long↔short on opposite signals (when Long-only is off).";
    }
  }

  updateMoSamplerUi();
  updateDeviceIdEnabled();
  updateMetaGroupingUi();
  updateMetaHoldoutUi();
  updatePairsPreviewUi();
  updateRunPlanUi();
  updateValidateTabUi();
  updateQuickActionsUi();
  refreshGpuStatusUi();
  updateTopTableNoRunUi();
  updateVdpsTelemetryHint(CURRENT_RUN?.res || null);
  renderContextChips();
}

function updateMetaGroupingUi() {
  const mode = String($("metaGroupingMode")?.value || "clustered");
  const enableCluster = mode !== "exact";
  const intEl = $("metaClusterIntBucket");
  const floatEl = $("metaClusterFloatBucket");
  if (intEl) intEl.disabled = RUNNING || !enableCluster;
  if (floatEl) floatEl.disabled = RUNNING || !enableCluster;
  updateMetaHoldoutUi();
}

function updateMetaHoldoutUi() {
  const kind = String($("metaStrategyKind")?.value || "CompositeMo");
  const supportsHoldout = kind === "CompositeCpu" || kind === "CompositeMo" || kind === "DoubleMa" || kind === "CompositeTrend" || kind === "RegimeHybrid" || kind === "BreakoutMomentum";

  const holdoutTag = $("metaHoldoutTag");
  const evalEl = $("metaEvalHoldout");
  const modeEl = $("metaHoldoutMode");
  const repsEl = $("metaHoldoutReps");
  const note = $("metaHoldoutNote");

  if (evalEl) {
    evalEl.disabled = RUNNING || !supportsHoldout;
    if (!supportsHoldout && evalEl.checked) {
      evalEl.checked = false;
      saveSettingsSoon();
    }
  }

  const evalOn = supportsHoldout && !!evalEl?.checked;
  if (holdoutTag) holdoutTag.disabled = RUNNING || !evalOn;
  if (modeEl) modeEl.disabled = RUNNING || !evalOn;
  const groupingMode = String($("metaGroupingMode")?.value || "clustered");
  const clustered = groupingMode !== "exact";
  if (repsEl) repsEl.disabled = RUNNING || !evalOn || !clustered;

  if (note) {
    note.innerHTML = supportsHoldout
      ? 'Holdout runs (tagged <span class="mono">holdout</span>) are excluded only when holdout evaluation is enabled. Heavy mode uses iterative re-scoring, walk-forward, and (optionally) the Validation perturb/MC toggles.'
      : "Holdout evaluation is currently unavailable for this strategy.";
  }
}

const PRESET_PROFILES = {
  Quick: {
    name: "Quick",
    topK: 20,
    includeAll: false,
    heatmapBins: 0,
    valAutoRun: true,
    valAutoExportXlsx: false,
    valAutoRevealReport: false,
    valAutoExportTopN: 20,
    valCandidates: 16,
    valWfEnable: false,
    valWfWindow: 20000,
    valWfStep: 20000,
    valPerturbEnable: false,
    valPerturbIters: 8,
    valPerturbMin: 0.015,
    valPerturbMax: 0.06,
    valMcEnable: false,
    valMcSims: 500,
    valCscvEnable: false,
    valCscvSlices: 16,
    valCrossEnable: false,
    valCrossMax: 4,
  },
  Balanced: {
    name: "Balanced",
    topK: 20,
    includeAll: false,
    heatmapBins: 128,
    valAutoRun: true,
    valAutoExportXlsx: false,
    valAutoRevealReport: false,
    valAutoExportTopN: 20,
    valCandidates: 32,
    valWfEnable: false,
    valWfWindow: 20000,
    valWfStep: 20000,
    valPerturbEnable: true,
    valPerturbIters: 8,
    valPerturbMin: 0.015,
    valPerturbMax: 0.06,
    valMcEnable: true,
    valMcSims: 500,
    valCscvEnable: false,
    valCscvSlices: 16,
    valCrossEnable: false,
    valCrossMax: 4,
  },
  Deep: {
    name: "Deep",
    topK: 50,
    includeAll: false,
    heatmapBins: 256,
    valAutoRun: true,
    valAutoExportXlsx: false,
    valAutoRevealReport: false,
    valAutoExportTopN: 50,
    valCandidates: 64,
    valWfEnable: true,
    valWfWindow: 20000,
    valWfStep: 20000,
    valPerturbEnable: true,
    valPerturbIters: 16,
    valPerturbMin: 0.01,
    valPerturbMax: 0.08,
    valMcEnable: true,
    valMcSims: 2000,
    valCscvEnable: true,
    valCscvSlices: 16,
    valCrossEnable: true,
    valCrossMax: 8,
  },
};

const COST_PRESETS = {
  custom: { fee_bps: null, spread_bps: null, slippage_bps: null },
  none: { fee_bps: 0.0, spread_bps: 0.0, slippage_bps: 0.0 },
  stocks_liquid: { fee_bps: 0.10, spread_bps: 1.00, slippage_bps: 0.50 },
  fx_majors: { fee_bps: 0.00, spread_bps: 1.00, slippage_bps: 0.50 },
  crypto_spot_maker: { fee_bps: 2.00, spread_bps: 5.00, slippage_bps: 1.50 },
  crypto_spot_taker: { fee_bps: 7.50, spread_bps: 5.00, slippage_bps: 2.50 },
};

function readCostBpsFromUi() {
  const fee_bps = Math.max(0, num($("feeBps")?.value, 0));
  const spread_bps = Math.max(0, num($("spreadBps")?.value, 0));
  const slippage_bps = Math.max(0, num($("slippageBps")?.value, 0));
  return { fee_bps, spread_bps, slippage_bps };
}

function perSideBpsFromCostBps(cost) {
  const fee = Number(cost?.fee_bps || 0);
  const spread = Number(cost?.spread_bps || 0);
  const slip = Number(cost?.slippage_bps || 0);
  return fee + slip + (spread / 2);
}

function commissionFractionFromCostBps(cost) {
  const perSideBps = perSideBpsFromCostBps(cost);
  return perSideBps / 10_000;
}

function syncCostUi(opts) {
  const { fee_bps, spread_bps, slippage_bps } = readCostBpsFromUi();
  const perSideBps = perSideBpsFromCostBps({ fee_bps, spread_bps, slippage_bps });
  const comm = commissionFractionFromCostBps({ fee_bps, spread_bps, slippage_bps });

  const commEl = $("commission");
  if (commEl) commEl.value = String(Math.max(0, Math.min(0.999999, comm)));

  const hint = $("costHint");
  if (hint) {
    const rt = perSideBps * 2;
    hint.textContent = `Total: ${fmtFixed(perSideBps, 2)} bps/side (${fmtFixed(rt, 2)} bps round-trip)`;
  }

  if (opts?.save) saveSettingsSoon();
}

function applyCostPreset(presetId, opts) {
  const silent = !!opts?.silent;
  const id = String(presetId || "custom").trim() || "custom";
  const preset = COST_PRESETS[id] || COST_PRESETS.custom;

  if (preset.fee_bps != null && $("feeBps")) $("feeBps").value = String(preset.fee_bps);
  if (preset.spread_bps != null && $("spreadBps")) $("spreadBps").value = String(preset.spread_bps);
  if (preset.slippage_bps != null && $("slippageBps")) $("slippageBps").value = String(preset.slippage_bps);

  syncCostUi({ save: !silent });
}

function readStrategyCfgFromUi() {
  const kind = strategyKind();
  const { fee_bps, spread_bps, slippage_bps } = readCostBpsFromUi();
  const cost_preset = String($("costPreset")?.value || "custom").trim() || "custom";
  const starting_equity = readStartingEquityFromUi();
  const commission = commissionFractionFromCostBps({ fee_bps, spread_bps, slippage_bps });
  const eps_rel = num($("epsRel")?.value, 0);
  const stop_loss_pct = parseRiskPctInput("stopLossPct");
  const take_profit_pct = parseRiskPctInput("takeProfitPct");
  const trailing_stop_pct = parseRiskPctInput("trailingStopPct");
  const cooldown_bars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
  const signal_persistence_bars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
  const exit_mode = String($("exitMode")?.value || "cross").trim() || "cross";
  const kaufman_stop_period = Math.max(1, Math.floor(num($("kaufmanStopPeriod")?.value, 22)));
  const kaufman_stop_mult = Math.max(0.0001, num($("kaufmanStopMult")?.value, 2.0));
  const kaufman_stop_ma_type = String($("kaufmanStopMaType")?.value || "sma").trim() || "sma";
  const trend_quality_enable = kind === "CompositeTrend" || kind === "RegimeHybrid";
  const trend_quality_linearreg_period = Math.max(1, Math.floor(num($("trendLrPeriod")?.value, 34)));
  const trend_quality_angle_min_deg = Math.max(0.0001, num($("trendLrMinDeg")?.value, 2.0));
  const trend_quality_bbw_period = Math.max(1, Math.floor(num($("trendBbwPeriod")?.value, 20)));
  const trend_quality_bbw_mult = Math.max(0.0001, num($("trendBbwMult")?.value, 2.0));
  const trend_quality_bbw_min = Math.max(0, num($("trendBbwMin")?.value, 0.04));
  const trend_quality_roc_period = Math.max(1, Math.floor(num($("trendRocPeriod")?.value, 9)));
  const trend_quality_use_roc = !!$("trendUseRoc")?.checked;
  const trend_quality_use_adx = kind === "CompositeTrend" && !!$("trendUseAdx")?.checked;
  const trend_quality_adx_period = Math.max(1, Math.floor(num($("trendAdxPeriod")?.value, 14)));
  const trend_quality_adx_min = Math.max(0.0001, num($("trendAdxMin")?.value, 20.0));
  const regime_hybrid_enable = kind === "RegimeHybrid";
  const regime_hybrid_adx_period = Math.max(1, Math.floor(num($("regimeAdxPeriod")?.value, 14)));
  const regime_hybrid_adx_enter = Math.max(0.0001, num($("regimeAdxEnter")?.value, 22.0));
  const regime_hybrid_adx_exit = Math.max(0.0001, num($("regimeAdxExit")?.value, 18.0));
  const regime_hybrid_reversion_entry_rel = Math.max(0.000001, num($("regimeRevEntryRel")?.value, 0.0035));
  const regime_hybrid_reversion_exit_rel = Math.max(0, num($("regimeRevExitRel")?.value, 0.0012));
  const breakout_momentum_enable = kind === "BreakoutMomentum";
  const breakout_bbands_period = Math.max(1, Math.floor(num($("breakoutBbPeriod")?.value, 20)));
  const breakout_bbands_mult = Math.max(0.0001, num($("breakoutBbMult")?.value, 2.0));
  const breakout_confirm_bars = Math.max(1, Math.floor(num($("breakoutConfirmBars")?.value, 1)));
  const breakout_macd_fast_period = Math.max(1, Math.floor(num($("breakoutMacdFast")?.value, 12)));
  const breakout_macd_slow_period = Math.max(1, Math.floor(num($("breakoutMacdSlow")?.value, 26)));
  const breakout_macd_signal_period = Math.max(1, Math.floor(num($("breakoutMacdSignal")?.value, 9)));
  const breakout_cmo_period = Math.max(1, Math.floor(num($("breakoutCmoPeriod")?.value, 14)));
  const breakout_cmo_min_abs = Math.max(0, num($("breakoutCmoMinAbs")?.value, 20.0));
  const breakout_adosc_short_period = Math.max(1, Math.floor(num($("breakoutAdoscShort")?.value, 3)));
  const breakout_adosc_long_period = Math.max(1, Math.floor(num($("breakoutAdoscLong")?.value, 10)));
  const breakout_use_adx = kind === "BreakoutMomentum" && !!$("breakoutUseAdx")?.checked;
  const breakout_adx_period = Math.max(1, Math.floor(num($("breakoutAdxPeriod")?.value, 14)));
  const breakout_adx_min = Math.max(0.0001, num($("breakoutAdxMin")?.value, 20.0));
  if (!Number.isFinite(commission) || commission < 0 || commission >= 1) throw new Error("total cost must be in [0, 1).");
  if (!Number.isFinite(eps_rel) || eps_rel < 0) throw new Error("eps_rel must be >= 0.");
  if (!Number.isFinite(cooldown_bars) || cooldown_bars < 0 || cooldown_bars > 65535) throw new Error("cooldown bars must be in [0, 65535].");
  if (!Number.isFinite(signal_persistence_bars) || signal_persistence_bars < 1 || signal_persistence_bars > 65535) {
    throw new Error("signal persistence bars must be in [1, 65535].");
  }
  if (!(exit_mode === "cross" || exit_mode === "kaufman_stop")) throw new Error("exit mode must be 'cross' or 'kaufman_stop'.");
  if (!Number.isFinite(kaufman_stop_mult) || kaufman_stop_mult <= 0) throw new Error("kaufman-stop multiplier must be > 0.");
  if (!Number.isFinite(trend_quality_angle_min_deg) || trend_quality_angle_min_deg <= 0) throw new Error("trend min angle must be > 0.");
  if (!Number.isFinite(trend_quality_bbw_mult) || trend_quality_bbw_mult <= 0) throw new Error("trend BB width multiplier must be > 0.");
  if (!Number.isFinite(trend_quality_bbw_min) || trend_quality_bbw_min < 0) throw new Error("trend min BB width must be >= 0.");
  if (trend_quality_use_adx) {
    if (!Number.isFinite(trend_quality_adx_period) || trend_quality_adx_period <= 0) {
      throw new Error("trend ADX period must be > 0.");
    }
    if (!Number.isFinite(trend_quality_adx_min) || trend_quality_adx_min <= 0) {
      throw new Error("trend min ADX must be > 0.");
    }
  }
  if (regime_hybrid_enable) {
    if (!Number.isFinite(regime_hybrid_adx_enter) || regime_hybrid_adx_enter <= 0) throw new Error("regime hybrid ADX enter must be > 0.");
    if (!Number.isFinite(regime_hybrid_adx_exit) || regime_hybrid_adx_exit <= 0 || regime_hybrid_adx_exit > regime_hybrid_adx_enter) {
      throw new Error("regime hybrid ADX exit must be > 0 and <= ADX enter.");
    }
    if (!Number.isFinite(regime_hybrid_reversion_entry_rel) || regime_hybrid_reversion_entry_rel <= 0) {
      throw new Error("regime hybrid reversion entry must be > 0.");
    }
    if (!Number.isFinite(regime_hybrid_reversion_exit_rel) || regime_hybrid_reversion_exit_rel < 0 || regime_hybrid_reversion_exit_rel > regime_hybrid_reversion_entry_rel) {
      throw new Error("regime hybrid reversion exit must be >= 0 and <= reversion entry.");
    }
  }
  if (breakout_momentum_enable) {
    if (!Number.isFinite(breakout_bbands_mult) || breakout_bbands_mult <= 0) {
      throw new Error("breakout BB multiplier must be > 0.");
    }
    if (!Number.isFinite(breakout_cmo_min_abs) || breakout_cmo_min_abs < 0) {
      throw new Error("breakout min |CMO| must be >= 0.");
    }
    if (breakout_macd_fast_period >= breakout_macd_slow_period) {
      throw new Error("breakout MACD requires fast period < slow period.");
    }
    if (breakout_adosc_short_period >= breakout_adosc_long_period) {
      throw new Error("breakout ADOSC requires short period < long period.");
    }
    if (breakout_use_adx) {
      if (!Number.isFinite(breakout_adx_min) || breakout_adx_min <= 0) {
        throw new Error("breakout ADX min must be > 0.");
      }
    }
  }
  return {
    long_only: !!$("longOnly")?.checked,
    allow_flip: !!$("allowFlip")?.checked,
    trade_on_next_bar: !!$("tradeOnNext")?.checked,
    cost_preset,
    starting_equity,
    fee_bps,
    spread_bps,
    slippage_bps,
    commission,
    stop_loss_pct,
    take_profit_pct,
    trailing_stop_pct,
    cooldown_bars,
    signal_persistence_bars,
    exit_mode,
    kaufman_stop_period,
    kaufman_stop_mult,
    kaufman_stop_ma_type,
    trend_quality_enable,
    trend_quality_linearreg_period,
    trend_quality_angle_min_deg,
    trend_quality_bbw_period,
    trend_quality_bbw_mult,
    trend_quality_bbw_min,
    trend_quality_roc_period,
    trend_quality_use_roc,
    trend_quality_use_adx,
    trend_quality_adx_period,
    trend_quality_adx_min,
    regime_hybrid_enable,
    regime_hybrid_adx_period,
    regime_hybrid_adx_enter,
    regime_hybrid_adx_exit,
    regime_hybrid_reversion_entry_rel,
    regime_hybrid_reversion_exit_rel,
    breakout_momentum_enable,
    breakout_bbands_period,
    breakout_bbands_mult,
    breakout_confirm_bars,
    breakout_macd_fast_period,
    breakout_macd_slow_period,
    breakout_macd_signal_period,
    breakout_cmo_period,
    breakout_cmo_min_abs,
    breakout_adosc_short_period,
    breakout_adosc_long_period,
    breakout_use_adx,
    breakout_adx_period,
    breakout_adx_min,
    eps_rel,
  };
}

function applyPresetProfile(profileId, opts) {
  const silent = !!opts?.silent;
  const id = String(profileId || "").trim();
  const preset = PRESET_PROFILES[id] || PRESET_PROFILES.Balanced;

  const setVal = (elId, value) => {
    const el = $(elId);
    if (!el) return;
    el.value = String(value);
  };
  const setBool = (elId, checked) => {
    const el = $(elId);
    if (!el) return;
    el.checked = !!checked;
  };

  // Optimization defaults (safe; avoids changing MA choices/ranges).
  setVal("topK", preset.topK);
  setVal("includeAll", preset.includeAll ? "true" : "false");
  setVal("heatmapBins", preset.heatmapBins);

  // Validation defaults (Composite strategies only; safe to set regardless).
  setBool("valAutoRun", preset.valAutoRun);
  setBool("valAutoExportXlsx", preset.valAutoExportXlsx);
  setBool("valAutoRevealReport", preset.valAutoRevealReport);
  setVal("valAutoExportTopN", preset.valAutoExportTopN);
  setVal("valCandidates", preset.valCandidates);

  setBool("valWfEnable", preset.valWfEnable);
  setVal("valWfWindow", preset.valWfWindow);
  setVal("valWfStep", preset.valWfStep);

  setBool("valPerturbEnable", preset.valPerturbEnable);
  setVal("valPerturbIters", preset.valPerturbIters);
  setVal("valPerturbMin", preset.valPerturbMin);
  setVal("valPerturbMax", preset.valPerturbMax);

  setBool("valMcEnable", preset.valMcEnable);
  setVal("valMcSims", preset.valMcSims);

  setBool("valCscvEnable", preset.valCscvEnable);
  setVal("valCscvSlices", preset.valCscvSlices);

  // Cross-ticker: only enable if the user has any cached datasets selected.
  const crossAvail = getCrossSelectedDatasets().length > 0;
  setBool("valCrossEnable", preset.valCrossEnable && crossAvail);
  setVal("valCrossMax", preset.valCrossMax);

  updateStrategyUi();
  applyBackendSelectionConstraints();
  updateDeviceIdEnabled();
  applyBackendMaConstraints();
  refreshMaParamsUi();
  updateValidationExportButtons();
  saveSettingsSoon();

  if (!silent) {
    const crossNote = preset.valCrossEnable && !crossAvail ? " (cross-ticker skipped; no datasets selected)" : "";
    setStatus(`Applied preset: ${preset.name}${crossNote}`, "ok");
  }
}

function updateMoSamplerUi() {
  if (strategyKind() !== "CompositeMo") return;

  const sampler = ($("moSampler")?.value || "Nsga2").trim();
  const nsga = $("moNsga2Wrap");
  const rnd = $("moRandomWrap");
  const tpe = $("moTpeWrap");
  const qmc = $("moQmcWrap");
  if (nsga) nsga.style.display = sampler === "Nsga2" ? "block" : "none";
  if (rnd) rnd.style.display = sampler === "Random" ? "block" : "none";
  if (tpe) tpe.style.display = sampler === "Tpe" ? "block" : "none";
  if (qmc) qmc.style.display = sampler === "Qmc" ? "block" : "none";

  const prescanOn = !!$("moPrescanEnable")?.checked;
  const dis = RUNNING || !prescanOn;
  const setDis = (id) => {
    const el = $(id);
    if (el) el.disabled = dis;
  };
  setDis("moPrescanRequireGpu");
  setDis("moPrescanTopK");
  setDis("moPrescanWindow");
  setDis("moPrescanPredictiveTopK");
  setDis("moPrescanPredictiveStride");
  setDis("moPrescanObjectives");
  setDis("moPrescanMode");
  setDis("moPrescanMaPairs");

  updateMoRangeBuilderUi();
}

function updateMoRangeBuilderUi() {
  if (strategyKind() !== "CompositeMo") return;
  const useBuilder = !!$("moUseRangeBuilder")?.checked;
  const hint = $("moRangeBuilderHint");
  if (hint) {
    hint.textContent = useBuilder
      ? "Form ranges are converted into Composite MO search-space JSON at run time."
      : "Raw JSON search-space mode is active.";
  }

  const jsonDetails = $("moSearchSpaceJsonDetails");
  if (jsonDetails) jsonDetails.open = !useBuilder;

  const json = $("moSearchSpace");
  if (json) json.disabled = RUNNING || useBuilder;
  const btnReset = $("btnMoResetSearchSpace");
  if (btnReset) btnReset.disabled = RUNNING || useBuilder;

  const builderIds = [
    "moHtAmpLow", "moHtAmpHigh", "moHtAmpStep",
    "moHtDevLow", "moHtDevHigh", "moHtDevStep",
    "moHtAtrLow", "moHtAtrHigh", "moHtAtrStep",
    "moMaczFastLow", "moMaczFastHigh", "moMaczFastStep",
    "moMaczSlowLow", "moMaczSlowHigh", "moMaczSlowStep",
    "moMaczSignalLow", "moMaczSignalHigh", "moMaczSignalStep",
    "moMaczLengthZLow", "moMaczLengthZHigh", "moMaczLengthZStep",
    "moMaczLengthStdevLow", "moMaczLengthStdevHigh", "moMaczLengthStdevStep",
    "moMaczGammaLow", "moMaczGammaHigh", "moMaczGammaStep",
    "moMaczALow", "moMaczAHigh", "moMaczAStep",
    "moMaczBLow", "moMaczBHigh", "moMaczBStep",
    "moMaczUseLagMode",
    "moMaczUseVolumeMode",
    "moOttPeriodLow", "moOttPeriodHigh", "moOttPeriodStep",
    "moOttPercentLow", "moOttPercentHigh", "moOttPercentStep",
    "moOttMaTypes",
  ];
  for (const id of builderIds) {
    const el = $(id);
    if (!el) continue;
    el.disabled = RUNNING || !useBuilder;
  }
}

function readSettingsFromUi() {
  const obj = {};
  const fields = [
    "csvPath",
    "polygonMarket",
    "polygonSymbol",
    "polygonTimeframe",
    "polygonFrom",
    "polygonTo",
    "strategyKind",
    "presetProfile",
    "backend",
    "deviceId",
    "compareSelect",
    "fastStart",
    "fastEnd",
    "fastStep",
    "slowStart",
    "slowEnd",
    "slowStep",
    "fastMaType",
    "slowMaType",
    "maSource",
    "objective",
    "mode",
    "topK",
    "includeAll",
    "exportAllCsvPath",
    "heatmapBins",
    "heatmapColor",
    "heatmapRange",
    "heatmapShowTop",
    "heatmapShowCrosshair",
    "commission",
    "costPreset",
    "feeBps",
    "spreadBps",
    "slippageBps",
    "epsRel",
    "startingEquity",
    "stopLossPct",
    "takeProfitPct",
    "trailingStopPct",
    "cooldownBars",
    "signalPersistenceBars",
    "dmConstraintMinTrades",
    "dmConstraintMaxDd",
    "exitMode",
    "kaufmanStopPeriod",
    "kaufmanStopMult",
    "kaufmanStopMaType",
    "trendLrPeriod",
    "trendLrMinDeg",
    "trendBbwPeriod",
    "trendBbwMult",
    "trendBbwMin",
    "trendRocPeriod",
    "trendAdxPeriod",
    "trendAdxMin",
    "regimeAdxPeriod",
    "regimeAdxEnter",
    "regimeAdxExit",
    "regimeRevEntryRel",
    "regimeRevExitRel",
    "breakoutBbPeriod",
    "breakoutBbMult",
    "breakoutConfirmBars",
    "breakoutMacdFast",
    "breakoutMacdSlow",
    "breakoutMacdSignal",
    "breakoutCmoPeriod",
    "breakoutCmoMinAbs",
    "breakoutAdoscShort",
    "breakoutAdoscLong",
    "breakoutAdxPeriod",
    "breakoutAdxMin",
    "execMode",
    "execSymbol",
    "execQty",
    "execParamSource",
    "execManualFastLen",
    "execManualSlowLen",
    "execRiskStopLossPct",
    "execRiskTakeProfitPct",
    "halfTrendAmplitude",
    "halfTrendAtrPeriod",
    "halfTrendChannelDev",
    "maczFastLength",
    "maczSlowLength",
    "maczSignalLength",
    "maczLengthZ",
    "maczLengthStdev",
    "maczGamma",
    "maczA",
    "maczB",
    "ottPeriod",
    "ottPercent",
    "ottMaType",
    "filterMinPnl",
    "filterMinSharpe",
    "filterMaxDd",
    "filterMinTrades",
    "filterMinExposure",
    "paretoX",
    "paretoY",
    "moObjectiveMin",
    "moSampler",
    "moNsga2Seed",
    "moNsga2Population",
    "moNsga2Generations",
    "moNsga2CrossoverProb",
    "moNsga2SwappingProb",
    "moNsga2MutationProb",
    "moRandomSeed",
    "moRandomTrials",
    "moTpeSeed",
    "moTpeTrials",
    "moTpeStartupTrials",
    "moTpeCandidates",
    "moTpeGamma",
    "moTpeBatchSize",
    "moTpeMaxModelPoints",
    "moQmcSeed",
    "moQmcTrials",
    "moQmcSequence",
    "moConstraintMinTrades",
    "moConstraintMaxDd",
    "moConstraintMaxAbsExposure",
    "moPrescanTopK",
    "moPrescanWindow",
    "moPrescanPredictiveTopK",
    "moPrescanPredictiveStride",
    "moPrescanObjectives",
    "moPrescanMode",
    "moPrescanMaPairs",
    "moSearchSpace",
    "valCandidates",
    "valSeed",
    "valWfWindow",
    "valWfStep",
    "valPerturbIters",
    "valPerturbMin",
    "valPerturbMax",
    "valMcSims",
    "valCscvSlices",
    "valCrossMax",
    "valCrossSelected",
    "batchDatasetSearch",
    "batchTags",
    "batchSelected",
    "batchHoldoutPct",
    "batchHoldoutTag",
    "metaStrategyKind",
    "metaTimeframe",
    "metaRequiredTag",
    "metaMinSamples",
    "metaTopK",
    "metaHoldoutTag",
    "metaHoldoutMode",
    "metaHoldoutReps",
    "metaGroupingMode",
    "metaClusterIntBucket",
    "metaClusterFloatBucket",
    "metaStage4TopK",
    "metaStage4MinConf",
    "metaStage4MetaWeight",
    "metaStage4MlWeight",
    "metaStage4UncertaintyWeight",
    "metaStage4AcceptConf",
    "metaStage4ReviewConf",
    "metaStage4MaxAcceptUncertainty",
    "metaSavedSearch",
    "stage4SavedSearch",
    "metaCompareSelect",
  ];
  for (const id of fields) {
    const el = $(id);
    if (!el) continue;
    obj[id] = el.value;
  }
  const checks = [
    "longOnly",
    "allowFlip",
    "tradeOnNext",
    "autoRestoreDataset",
    "execDryRun",
    "execRememberKeys",
    "execLiveUseDelayed",
    "polygonStocksAdjusted",
    "maczUseVolume",
    "maczUseLag",
    "trendUseRoc",
    "trendUseAdx",
    "breakoutUseAdx",
    "moUseRangeBuilder",
    "moPrescanEnable",
    "moPrescanRequireGpu",
    "valAutoRun",
    "valAutoExportXlsx",
    "valAutoRevealReport",
    "valWfEnable",
    "valPerturbEnable",
    "valMcEnable",
    "valCscvEnable",
    "valCrossEnable",
    "metaEvalHoldout",
    "metaUseMlConfidence",
    "metaMlAutopilot",
    "metaStage4AutoTrain",
    "metaStage4RequireHealthy",
    "metaStage4FallbackToMeta",
    "batchAutoMeta",
    "batchSkipDone",
  ];
  for (const id of checks) {
    const el = $(id);
    if (!el) continue;
    obj[id] = !!el.checked;
  }
  obj.maParams = MA_PARAMS_STATE;
  obj.maParamSweeps = MA_PARAM_SWEEPS_STATE;
  return obj;
}

let SAVE_TIMER = null;
function saveSettingsSoon() {
  if (SAVE_TIMER) clearTimeout(SAVE_TIMER);
  SAVE_TIMER = setTimeout(() => {
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(readSettingsFromUi()));
    } catch {}
  }, 120);
}

function clearExecutionApiKeysFromStorage() {
  try {
    localStorage.removeItem(EXEC_KEYS_STORAGE_KEY);
  } catch {}
}

function saveExecutionApiKeysToStorage() {
  if (!$("execRememberKeys")?.checked) {
    clearExecutionApiKeysFromStorage();
    return;
  }
  const api_key = String($("execApiKey")?.value || "").trim();
  const api_secret = String($("execApiSecret")?.value || "").trim();
  try {
    localStorage.setItem(EXEC_KEYS_STORAGE_KEY, JSON.stringify({ api_key, api_secret }));
  } catch {}
}

function loadExecutionApiKeysFromStorage() {
  if (!$("execRememberKeys")?.checked) return;
  let payload = null;
  try {
    payload = JSON.parse(localStorage.getItem(EXEC_KEYS_STORAGE_KEY) || "null");
  } catch {
    payload = null;
  }
  if (!payload || typeof payload !== "object") return;
  if ($("execApiKey")) $("execApiKey").value = String(payload.api_key || "");
  if ($("execApiSecret")) $("execApiSecret").value = String(payload.api_secret || "");
}

function loadSettings() {
  let settings = null;
  try {
    settings = JSON.parse(localStorage.getItem(SETTINGS_KEY) || "null");
  } catch {
    settings = null;
  }
  if (!settings || typeof settings !== "object") return false;

  if (settings.maParams && typeof settings.maParams === "object") {
    MA_PARAMS_STATE = settings.maParams;
    if (!MA_PARAMS_STATE || typeof MA_PARAMS_STATE !== "object") MA_PARAMS_STATE = { fast: {}, slow: {} };
    if (!MA_PARAMS_STATE.fast || typeof MA_PARAMS_STATE.fast !== "object") MA_PARAMS_STATE.fast = {};
    if (!MA_PARAMS_STATE.slow || typeof MA_PARAMS_STATE.slow !== "object") MA_PARAMS_STATE.slow = {};
  }
  if (settings.maParamSweeps && typeof settings.maParamSweeps === "object") {
    MA_PARAM_SWEEPS_STATE = settings.maParamSweeps;
    if (!MA_PARAM_SWEEPS_STATE || typeof MA_PARAM_SWEEPS_STATE !== "object") MA_PARAM_SWEEPS_STATE = { fast: {}, slow: {} };
    if (!MA_PARAM_SWEEPS_STATE.fast || typeof MA_PARAM_SWEEPS_STATE.fast !== "object") MA_PARAM_SWEEPS_STATE.fast = {};
    if (!MA_PARAM_SWEEPS_STATE.slow || typeof MA_PARAM_SWEEPS_STATE.slow !== "object") MA_PARAM_SWEEPS_STATE.slow = {};
  }

  const legacyOffset = num(settings.almaOffset, NaN);
  const legacySigma = num(settings.almaSigma, NaN);
  if (Number.isFinite(legacyOffset) || Number.isFinite(legacySigma)) {
    for (const side of ["fast", "slow"]) {
      const st = ensureMaParamState(side, "alma");
      if (Number.isFinite(legacyOffset) && st.offset == null) st.offset = legacyOffset;
      if (Number.isFinite(legacySigma) && st.sigma == null) st.sigma = legacySigma;
    }
  }

  for (const [id, value] of Object.entries(settings)) {
    if (id === "maParams" || id === "maParamSweeps") continue;
    const el = $(id);
    if (!el) continue;
    if (el.type === "checkbox") el.checked = !!value;
    else {
      const raw = String(value);
      const v = id === "strategyKind" && raw === "CompositeCpu" ? "CompositeMo" : raw;
      el.value = v;
      if (id === "deviceId") el.dataset.desired = v;
    }
  }

  // Back-compat: if new controls are missing from stored settings, apply Balanced defaults.
  let patched = false;
  const d = PRESET_PROFILES.Balanced;
  const has = (id) => Object.prototype.hasOwnProperty.call(settings, id);
  const setDef = (id, value) => {
    if (has(id)) return;
    const el = $(id);
    if (!el) return;
    if (el.type === "checkbox") el.checked = !!value;
    else el.value = String(value);
    patched = true;
  };
  setDef("presetProfile", "Balanced");
  setDef("topK", d.topK);
  setDef("includeAll", d.includeAll ? "true" : "false");
  setDef("vdpsEta", 4);
  setDef("vdpsRungs", "0.0625,0.125,0.25,0.5,1.0");
  setDef("vdpsMinInitialPairs", 4096);
  setDef("vdpsMaxSurvivorSeeds", 256);
  setDef("vdpsMaxPeriodCandidates", 384);
  setDef("vdpsRandomInjectionRatio", 0.10);
  setDef("vdpsEliteMutationRatio", 0.50);
  setDef("vdpsDifferentialRatio", 0.40);
  setDef("vdpsDeterministic", true);
  setDef("vdpsSeed", 1);
  setDef("vdpsQualityCheckMaxPairs", 0);
  setDef("vdpsQualityCheckTopK", 20);
  setDef("heatmapBins", d.heatmapBins);
  setDef("heatmapColor", "cividis");
  setDef("heatmapRange", "q2_98");
  setDef("heatmapShowTop", "true");
  setDef("heatmapShowCrosshair", "true");
  setDef("valAutoRun", d.valAutoRun);
  setDef("valAutoExportXlsx", d.valAutoExportXlsx);
  setDef("valAutoRevealReport", d.valAutoRevealReport);
  setDef("valAutoExportTopN", d.valAutoExportTopN);
  setDef("valCandidates", d.valCandidates);
  setDef("valWfEnable", d.valWfEnable);
  setDef("valWfWindow", d.valWfWindow);
  setDef("valWfStep", d.valWfStep);
  setDef("valPerturbEnable", d.valPerturbEnable);
  setDef("valPerturbIters", d.valPerturbIters);
  setDef("valPerturbMin", d.valPerturbMin);
  setDef("valPerturbMax", d.valPerturbMax);
  setDef("valMcEnable", d.valMcEnable);
  setDef("valMcSims", d.valMcSims);
  setDef("valCscvEnable", d.valCscvEnable);
  setDef("valCscvSlices", d.valCscvSlices);
  setDef("valCrossEnable", d.valCrossEnable);
  setDef("valCrossMax", d.valCrossMax);
  setDef("metaHoldoutMode", "light");
  setDef("autoRestoreDataset", true);

  // Cost model migration: older settings used `commission` directly.
  setDef("costPreset", "custom");
  setDef("startingEquity", DEFAULT_STARTING_EQUITY);
  if (!has("feeBps") && $("feeBps")) {
    const comm = num(settings.commission, NaN);
    $("feeBps").value = String(Number.isFinite(comm) ? (Math.max(0, comm) * 10_000) : 0);
    patched = true;
  }
  setDef("spreadBps", 0);
  setDef("slippageBps", 0);
  syncCostUi({ save: false });
  loadExecutionApiKeysFromStorage();
  if (patched) saveSettingsSoon();
  updateDeviceIdEnabled();
  updatePolygonAdjustedUi();
  updateStrategyUi();
  applyBackendSelectionConstraints();
  return true;
}

async function restoreLastDatasetOnStartup() {
  const autoRestore = !!$("autoRestoreDataset")?.checked;
  if (!autoRestore) return;

  const path = String($("csvPath")?.value || "").trim();
  if (!path) return;

  const tfWanted = String($("dataTimeframe")?.value || "").trim();
  try {
    await loadCsv();
  } catch (e) {
    setStatus(`Startup restore skipped: ${String(e)}`, "muted");
    return;
  }

  if (tfWanted && $("dataTimeframe")) {
    try {
      $("dataTimeframe").value = tfWanted;
      await applySelectedDataTimeframe({ persist: false });
    } catch (e) {
      setStatus(`Timeframe restore skipped: ${String(e)}`, "muted");
    }
  }
}

function applySettingsObject(settings) {
  if (!settings || typeof settings !== "object") return;

  if (settings.maParams && typeof settings.maParams === "object") {
    MA_PARAMS_STATE = settings.maParams;
    if (!MA_PARAMS_STATE || typeof MA_PARAMS_STATE !== "object") MA_PARAMS_STATE = { fast: {}, slow: {} };
    if (!MA_PARAMS_STATE.fast || typeof MA_PARAMS_STATE.fast !== "object") MA_PARAMS_STATE.fast = {};
    if (!MA_PARAMS_STATE.slow || typeof MA_PARAMS_STATE.slow !== "object") MA_PARAMS_STATE.slow = {};
  }
  if (settings.maParamSweeps && typeof settings.maParamSweeps === "object") {
    MA_PARAM_SWEEPS_STATE = settings.maParamSweeps;
    if (!MA_PARAM_SWEEPS_STATE || typeof MA_PARAM_SWEEPS_STATE !== "object") MA_PARAM_SWEEPS_STATE = { fast: {}, slow: {} };
    if (!MA_PARAM_SWEEPS_STATE.fast || typeof MA_PARAM_SWEEPS_STATE.fast !== "object") MA_PARAM_SWEEPS_STATE.fast = {};
    if (!MA_PARAM_SWEEPS_STATE.slow || typeof MA_PARAM_SWEEPS_STATE.slow !== "object") MA_PARAM_SWEEPS_STATE.slow = {};
  }

  for (const [id, value] of Object.entries(settings)) {
    if (id === "maParams" || id === "maParamSweeps") continue;
    const el = $(id);
    if (!el) continue;
    if (el.type === "checkbox") el.checked = !!value;
    else {
      const raw = String(value);
      el.value = id === "strategyKind" && raw === "CompositeCpu" ? "CompositeMo" : raw;
    }
  }

  syncCostUi({ save: false });
  loadExecutionApiKeysFromStorage();
  updateStrategyUi();
  applyBackendSelectionConstraints();
  updateDeviceIdEnabled();
  updatePolygonAdjustedUi();
  applyBackendMaConstraints();
  refreshMaParamsUi();
  saveSettingsSoon();
}

function applyRequestToUi(req) {
  if (!req || typeof req !== "object") return;

  const kindRaw = String(req.strategy_kind || req.strategyKind || "DoubleMa");
  const kind = normalizeStrategyKindRaw(kindRaw);
  if ($("strategyKind")) $("strategyKind").value = kind === "CompositeCpu" ? "CompositeMo" : kind;
  updateStrategyUi();

  const backend = String(req?.backend?.backend || "CpuOnly");
  const deviceId = Number(req?.backend?.device_id || 0);
  const fast = Array.isArray(req.fast_range) ? req.fast_range : [0, 0, 0];
  const slow = Array.isArray(req.slow_range) ? req.slow_range : [0, 0, 0];

  const fastMa = kind === "CompositeMo" ? String((req.fast_ma_type || "sma")).trim() : String((req.fast_ma_types?.[0] || "sma")).trim();
  const slowMa = kind === "CompositeMo" ? String((req.slow_ma_type || "sma")).trim() : String((req.slow_ma_types?.[0] || "sma")).trim();

  if (isDoubleLikeKind(kind)) {
    if ($("backend")) $("backend").value = backend;
    if ($("deviceId")) {
      $("deviceId").value = String(deviceId);
      $("deviceId").dataset.desired = String(deviceId);
    }
  } else if (kind === "CompositeMo") {
    const prescan = req.gpu_prescan && typeof req.gpu_prescan === "object" ? req.gpu_prescan : null;
    if ($("backend")) $("backend").value = "CpuOnly";
    if ($("deviceId")) {
      const id = String(Number(prescan?.device_id || 0));
      $("deviceId").value = id;
      $("deviceId").dataset.desired = id;
    }
  } else {
    if ($("backend")) $("backend").value = "CpuOnly";
    if ($("deviceId")) {
      $("deviceId").value = "0";
      $("deviceId").dataset.desired = "0";
    }
  }

  if (kind === "CompositeMo") {
    const space = req.search_space && typeof req.search_space === "object" ? req.search_space : null;
    const findIntRange = (name) => {
      const params = Array.isArray(space?.params) ? space.params : [];
      const p = params.find((x) => String(x?.name || "").toLowerCase() === String(name).toLowerCase());
      const d = p?.domain && typeof p.domain === "object" ? p.domain : null;
      const r = d?.IntRange && typeof d.IntRange === "object" ? d.IntRange : null;
      if (!r) return null;
      return { low: Number(r.low), high: Number(r.high), step: Number(r.step ?? 1) };
    };
    const fr = findIntRange("fast_len");
    const sr = findIntRange("slow_len");
    if (fr) {
      if ($("fastStart")) $("fastStart").value = String(fr.low || 0);
      if ($("fastEnd")) $("fastEnd").value = String(fr.high || 0);
      if ($("fastStep")) $("fastStep").value = String(fr.step || 1);
    }
    if (sr) {
      if ($("slowStart")) $("slowStart").value = String(sr.low || 0);
      if ($("slowEnd")) $("slowEnd").value = String(sr.high || 0);
      if ($("slowStep")) $("slowStep").value = String(sr.step || 1);
    }
  } else {
    if ($("fastStart")) $("fastStart").value = String(Number(fast[0] || 0));
    if ($("fastEnd")) $("fastEnd").value = String(Number(fast[1] || 0));
    if ($("fastStep")) $("fastStep").value = String(Number(fast[2] || 0));

    if ($("slowStart")) $("slowStart").value = String(Number(slow[0] || 0));
    if ($("slowEnd")) $("slowEnd").value = String(Number(slow[1] || 0));
    if ($("slowStep")) $("slowStep").value = String(Number(slow[2] || 0));
  }

  if ($("fastMaType")) $("fastMaType").value = fastMa;
  if ($("slowMaType")) $("slowMaType").value = slowMa;

  if ($("maSource")) $("maSource").value = String(req.ma_source || "close");
  if ($("objective")) {
    $("objective").value = String(kind === "CompositeMo" ? (req?.objectives?.maximize || "Sharpe") : (req.objective || "Sharpe"));
  }
  if (kind === "CompositeMo" && $("moObjectiveMin")) {
    $("moObjectiveMin").value = String(req?.objectives?.minimize || "MaxDrawdown");
  }
  if ($("mode")) $("mode").value = ((kind === "CompositeCpu" || kind === "CompositeMo") ? "Grid" : String(req.mode || "Grid"));
  const vdps = req?.vdps && typeof req.vdps === "object" ? req.vdps : null;
  if ($("vdpsEta")) $("vdpsEta").value = String(Math.max(2, Math.floor(num(vdps?.eta, 4))));
  if ($("vdpsRungs")) {
    const rungs = Array.isArray(vdps?.rungs) ? vdps.rungs.map((v) => Number(v)).filter((v) => Number.isFinite(v) && v > 0) : [];
    $("vdpsRungs").value = rungs.length ? rungs.join(",") : "0.0625,0.125,0.25,0.5,1.0";
  }
  if ($("vdpsMinInitialPairs")) $("vdpsMinInitialPairs").value = String(Math.max(1, Math.floor(num(vdps?.min_initial_pairs, 4096))));
  if ($("vdpsMaxSurvivorSeeds")) $("vdpsMaxSurvivorSeeds").value = String(Math.max(8, Math.floor(num(vdps?.max_survivor_seeds, 256))));
  if ($("vdpsMaxPeriodCandidates")) $("vdpsMaxPeriodCandidates").value = String(Math.max(16, Math.floor(num(vdps?.max_period_candidates, 384))));
  if ($("vdpsRandomInjectionRatio")) $("vdpsRandomInjectionRatio").value = String(Math.min(1, Math.max(0, num(vdps?.random_injection_ratio, 0.10))));
  if ($("vdpsEliteMutationRatio")) $("vdpsEliteMutationRatio").value = String(Math.min(1, Math.max(0, num(vdps?.elite_mutation_ratio, 0.50))));
  if ($("vdpsDifferentialRatio")) $("vdpsDifferentialRatio").value = String(Math.min(1, Math.max(0, num(vdps?.differential_ratio, 0.40))));
  if ($("vdpsDeterministic")) $("vdpsDeterministic").checked = vdps?.deterministic !== false;
  if ($("vdpsSeed")) $("vdpsSeed").value = String(Math.max(0, Math.floor(num(vdps?.seed, 1))));
  if ($("vdpsQualityCheckMaxPairs")) $("vdpsQualityCheckMaxPairs").value = String(Math.max(0, Math.floor(num(vdps?.quality_check_max_pairs, 0))));
  if ($("vdpsQualityCheckTopK")) $("vdpsQualityCheckTopK").value = String(Math.max(1, Math.floor(num(vdps?.quality_check_top_k, 20))));

  if ($("topK")) $("topK").value = String(Number(req.top_k || 0));
  if ($("includeAll")) $("includeAll").value = (req.include_all ? "true" : "false");
  if ($("heatmapBins")) $("heatmapBins").value = String(Number(req.heatmap_bins || 0));
  if ($("exportAllCsvPath")) $("exportAllCsvPath").value = String(req.export_all_csv_path || "");

  const st = kind === "CompositeMo" ? (req.base_strategy || {}) : (req.strategy || {});
  const startingEquity = startingEquityFromReq(req);
  if ($("startingEquity")) $("startingEquity").value = String(startingEquity);
  const presetRaw = String(st.cost_preset || "").trim();
  const preset = COST_PRESETS[presetRaw] ? presetRaw : "custom";
  if ($("costPreset")) $("costPreset").value = preset;

  const comm = num(st.commission, 0);
  const feeBps = Number(st.fee_bps);
  const spreadBps = Number(st.spread_bps);
  const slipBps = Number(st.slippage_bps);
  const stopLossPct = Number(st.stop_loss_pct);
  const takeProfitPct = Number(st.take_profit_pct);
  const trailingStopPct = Number(st.trailing_stop_pct);
  const cooldownBars = Math.max(0, Math.floor(num(st.cooldown_bars, 0)));
  const signalPersistenceBars = Math.max(1, Math.floor(num(st.signal_persistence_bars, 1)));
  const exitMode = String(st.exit_mode || "cross").trim() || "cross";
  const kaufmanStopPeriod = Math.max(1, Math.floor(num(st.kaufman_stop_period, 22)));
  const kaufmanStopMult = num(st.kaufman_stop_mult, 2.0);
  const kaufmanStopMaType = String(st.kaufman_stop_ma_type || "sma").trim() || "sma";
  const trendLrPeriod = Math.max(1, Math.floor(num(st.trend_quality_linearreg_period, 34)));
  const trendLrMinDeg = num(st.trend_quality_angle_min_deg, 2.0);
  const trendBbwPeriod = Math.max(1, Math.floor(num(st.trend_quality_bbw_period, 20)));
  const trendBbwMult = num(st.trend_quality_bbw_mult, 2.0);
  const trendBbwMin = num(st.trend_quality_bbw_min, 0.04);
  const trendRocPeriod = Math.max(1, Math.floor(num(st.trend_quality_roc_period, 9)));
  const trendUseRoc = !!st.trend_quality_use_roc;
  const trendUseAdx = !!st.trend_quality_use_adx;
  const trendAdxPeriod = Math.max(1, Math.floor(num(st.trend_quality_adx_period, 14)));
  const trendAdxMin = num(st.trend_quality_adx_min, 20.0);
  const regimeAdxPeriod = Math.max(1, Math.floor(num(st.regime_hybrid_adx_period, 14)));
  const regimeAdxEnter = num(st.regime_hybrid_adx_enter, 22.0);
  const regimeAdxExit = num(st.regime_hybrid_adx_exit, 18.0);
  const regimeRevEntryRel = num(st.regime_hybrid_reversion_entry_rel, 0.0035);
  const regimeRevExitRel = num(st.regime_hybrid_reversion_exit_rel, 0.0012);
  const breakoutBbPeriod = Math.max(1, Math.floor(num(st.breakout_bbands_period, 20)));
  const breakoutBbMult = num(st.breakout_bbands_mult, 2.0);
  const breakoutConfirmBars = Math.max(1, Math.floor(num(st.breakout_confirm_bars, 1)));
  const breakoutMacdFast = Math.max(1, Math.floor(num(st.breakout_macd_fast_period, 12)));
  const breakoutMacdSlow = Math.max(1, Math.floor(num(st.breakout_macd_slow_period, 26)));
  const breakoutMacdSignal = Math.max(1, Math.floor(num(st.breakout_macd_signal_period, 9)));
  const breakoutCmoPeriod = Math.max(1, Math.floor(num(st.breakout_cmo_period, 14)));
  const breakoutCmoMinAbs = num(st.breakout_cmo_min_abs, 20.0);
  const breakoutAdoscShort = Math.max(1, Math.floor(num(st.breakout_adosc_short_period, 3)));
  const breakoutAdoscLong = Math.max(1, Math.floor(num(st.breakout_adosc_long_period, 10)));
  const breakoutUseAdx = !!st.breakout_use_adx;
  const breakoutAdxPeriod = Math.max(1, Math.floor(num(st.breakout_adx_period, 14)));
  const breakoutAdxMin = num(st.breakout_adx_min, 20.0);

  const hasBps =
    Number.isFinite(feeBps) && feeBps >= 0
    && Number.isFinite(spreadBps) && spreadBps >= 0
    && Number.isFinite(slipBps) && slipBps >= 0;

  if ($("feeBps")) $("feeBps").value = String(hasBps ? feeBps : Math.max(0, comm) * 10_000);
  if ($("spreadBps")) $("spreadBps").value = String(hasBps ? spreadBps : 0);
  if ($("slippageBps")) $("slippageBps").value = String(hasBps ? slipBps : 0);
  syncCostUi({ save: false });
  if ($("epsRel")) $("epsRel").value = String(num(st.eps_rel, 0));
  if ($("stopLossPct")) $("stopLossPct").value = Number.isFinite(stopLossPct) && stopLossPct > 0 ? String(stopLossPct) : "";
  if ($("takeProfitPct")) $("takeProfitPct").value = Number.isFinite(takeProfitPct) && takeProfitPct > 0 ? String(takeProfitPct) : "";
  if ($("trailingStopPct")) $("trailingStopPct").value = Number.isFinite(trailingStopPct) && trailingStopPct > 0 ? String(trailingStopPct) : "";
  if ($("cooldownBars")) $("cooldownBars").value = String(cooldownBars);
  if ($("signalPersistenceBars")) $("signalPersistenceBars").value = String(signalPersistenceBars);
  const dmConstraints = req.constraints && typeof req.constraints === "object" ? req.constraints : null;
  const dmMinTrades = Number(dmConstraints?.min_trades);
  const dmMaxDd = Number(dmConstraints?.max_drawdown);
  if ($("dmConstraintMinTrades")) $("dmConstraintMinTrades").value = Number.isFinite(dmMinTrades) && dmMinTrades >= 0 ? String(Math.floor(dmMinTrades)) : "";
  if ($("dmConstraintMaxDd")) $("dmConstraintMaxDd").value = Number.isFinite(dmMaxDd) && dmMaxDd >= 0 ? String(dmMaxDd) : "";
  if ($("exitMode")) $("exitMode").value = (exitMode === "kaufman_stop" ? "kaufman_stop" : "cross");
  if ($("kaufmanStopPeriod")) $("kaufmanStopPeriod").value = String(kaufmanStopPeriod);
  if ($("kaufmanStopMult")) $("kaufmanStopMult").value = String(Number.isFinite(kaufmanStopMult) && kaufmanStopMult > 0 ? kaufmanStopMult : 2.0);
  if ($("kaufmanStopMaType")) $("kaufmanStopMaType").value = kaufmanStopMaType;
  if ($("trendLrPeriod")) $("trendLrPeriod").value = String(trendLrPeriod);
  if ($("trendLrMinDeg")) $("trendLrMinDeg").value = String(Number.isFinite(trendLrMinDeg) && trendLrMinDeg > 0 ? trendLrMinDeg : 2.0);
  if ($("trendBbwPeriod")) $("trendBbwPeriod").value = String(trendBbwPeriod);
  if ($("trendBbwMult")) $("trendBbwMult").value = String(Number.isFinite(trendBbwMult) && trendBbwMult > 0 ? trendBbwMult : 2.0);
  if ($("trendBbwMin")) $("trendBbwMin").value = String(Number.isFinite(trendBbwMin) && trendBbwMin >= 0 ? trendBbwMin : 0.04);
  if ($("trendRocPeriod")) $("trendRocPeriod").value = String(trendRocPeriod);
  if ($("trendUseRoc")) $("trendUseRoc").checked = trendUseRoc;
  if ($("trendUseAdx")) $("trendUseAdx").checked = trendUseAdx;
  if ($("trendAdxPeriod")) $("trendAdxPeriod").value = String(trendAdxPeriod);
  if ($("trendAdxMin")) $("trendAdxMin").value = String(Number.isFinite(trendAdxMin) && trendAdxMin > 0 ? trendAdxMin : 20.0);
  if ($("regimeAdxPeriod")) $("regimeAdxPeriod").value = String(regimeAdxPeriod);
  if ($("regimeAdxEnter")) $("regimeAdxEnter").value = String(Number.isFinite(regimeAdxEnter) && regimeAdxEnter > 0 ? regimeAdxEnter : 22.0);
  if ($("regimeAdxExit")) $("regimeAdxExit").value = String(Number.isFinite(regimeAdxExit) && regimeAdxExit > 0 ? regimeAdxExit : 18.0);
  if ($("regimeRevEntryRel")) $("regimeRevEntryRel").value = String(Number.isFinite(regimeRevEntryRel) && regimeRevEntryRel > 0 ? regimeRevEntryRel : 0.0035);
  if ($("regimeRevExitRel")) $("regimeRevExitRel").value = String(Number.isFinite(regimeRevExitRel) && regimeRevExitRel >= 0 ? regimeRevExitRel : 0.0012);
  if ($("breakoutBbPeriod")) $("breakoutBbPeriod").value = String(breakoutBbPeriod);
  if ($("breakoutBbMult")) $("breakoutBbMult").value = String(Number.isFinite(breakoutBbMult) && breakoutBbMult > 0 ? breakoutBbMult : 2.0);
  if ($("breakoutConfirmBars")) $("breakoutConfirmBars").value = String(breakoutConfirmBars);
  if ($("breakoutMacdFast")) $("breakoutMacdFast").value = String(breakoutMacdFast);
  if ($("breakoutMacdSlow")) $("breakoutMacdSlow").value = String(breakoutMacdSlow);
  if ($("breakoutMacdSignal")) $("breakoutMacdSignal").value = String(breakoutMacdSignal);
  if ($("breakoutCmoPeriod")) $("breakoutCmoPeriod").value = String(breakoutCmoPeriod);
  if ($("breakoutCmoMinAbs")) $("breakoutCmoMinAbs").value = String(Number.isFinite(breakoutCmoMinAbs) && breakoutCmoMinAbs >= 0 ? breakoutCmoMinAbs : 20.0);
  if ($("breakoutAdoscShort")) $("breakoutAdoscShort").value = String(breakoutAdoscShort);
  if ($("breakoutAdoscLong")) $("breakoutAdoscLong").value = String(breakoutAdoscLong);
  if ($("breakoutUseAdx")) $("breakoutUseAdx").checked = breakoutUseAdx;
  if ($("breakoutAdxPeriod")) $("breakoutAdxPeriod").value = String(breakoutAdxPeriod);
  if ($("breakoutAdxMin")) $("breakoutAdxMin").value = String(Number.isFinite(breakoutAdxMin) && breakoutAdxMin > 0 ? breakoutAdxMin : 20.0);
  if ($("longOnly")) $("longOnly").checked = !!st.long_only;
  if ($("allowFlip")) $("allowFlip").checked = !!st.allow_flip;
  if ($("tradeOnNext")) $("tradeOnNext").checked = !!st.trade_on_next_bar;

  if (kind === "CompositeCpu" || kind === "CompositeMo") {
    const rawSignalMode = String(req.signal_mode || req.signalMode || "hard_gate").trim().toLowerCase();
    const signalMode = rawSignalMode === "entry_core_exit_overlay" || rawSignalMode === "trend_pullback_reversion"
      ? rawSignalMode
      : "hard_gate";
    if ($("compositeSignalMode")) $("compositeSignalMode").value = signalMode;
  }

  if (kind === "CompositeCpu") {
    const ht = req.halftrend && typeof req.halftrend === "object" ? req.halftrend : null;
    if (ht) {
      if ($("halfTrendAmplitude")) $("halfTrendAmplitude").value = String(Math.max(1, Math.floor(num(ht.amplitude, 2))));
      if ($("halfTrendAtrPeriod")) $("halfTrendAtrPeriod").value = String(Math.max(1, Math.floor(num(ht.atr_period, 100))));
      if ($("halfTrendChannelDev")) $("halfTrendChannelDev").value = String(num(ht.channel_deviation, 2.0));
    }

    const macz = req.macz && typeof req.macz === "object" ? req.macz : null;
    if (macz) {
      if ($("maczFastLength")) $("maczFastLength").value = String(Math.max(1, Math.floor(num(macz.fast_length, 12))));
      if ($("maczSlowLength")) $("maczSlowLength").value = String(Math.max(1, Math.floor(num(macz.slow_length, 25))));
      if ($("maczSignalLength")) $("maczSignalLength").value = String(Math.max(1, Math.floor(num(macz.signal_length, 9))));
      if ($("maczLengthZ")) $("maczLengthZ").value = String(Math.max(1, Math.floor(num(macz.lengthz, 20))));
      if ($("maczLengthStdev")) $("maczLengthStdev").value = String(Math.max(1, Math.floor(num(macz.length_stdev, 25))));
      if ($("maczGamma")) $("maczGamma").value = String(num(macz.gamma, 0.02));
      if ($("maczA")) $("maczA").value = String(num(macz.a, 1.0));
      if ($("maczB")) $("maczB").value = String(num(macz.b, 1.0));
      if ($("maczUseVolume")) $("maczUseVolume").checked = !!macz.use_volume;
      if ($("maczUseLag")) $("maczUseLag").checked = !!macz.use_lag;
    }

    const ott = req.ott && typeof req.ott === "object" ? req.ott : null;
    if (ott) {
      if ($("ottPeriod")) $("ottPeriod").value = String(Math.max(1, Math.floor(num(ott.period, 2))));
      if ($("ottPercent")) $("ottPercent").value = String(num(ott.percent, 1.4));
      if ($("ottMaType")) $("ottMaType").value = String(ott.ma_type || "VAR");
    }
  }

  if (kind === "CompositeMo") {
    const sampler = req.sampler && typeof req.sampler === "object" ? req.sampler : null;
    const samplerKind = String(sampler?.sampler || "Nsga2");
    if ($("moSampler")) $("moSampler").value = samplerKind;

    if (samplerKind === "Nsga2") {
      const cfg = sampler?.config && typeof sampler.config === "object" ? sampler.config : null;
      if (cfg) {
        if ($("moNsga2Seed")) $("moNsga2Seed").value = String(Number(cfg.seed ?? 1));
        if ($("moNsga2Population")) $("moNsga2Population").value = String(Number(cfg.population_size ?? 80));
        if ($("moNsga2Generations")) $("moNsga2Generations").value = String(Number(cfg.generations ?? 40));
        if ($("moNsga2CrossoverProb")) $("moNsga2CrossoverProb").value = String(num(cfg.crossover_prob, 0.9));
        if ($("moNsga2SwappingProb")) $("moNsga2SwappingProb").value = String(num(cfg.swapping_prob, 0.5));
        if ($("moNsga2MutationProb")) $("moNsga2MutationProb").value = (cfg.mutation_prob == null ? "" : String(num(cfg.mutation_prob, "")));
      }
    } else if (samplerKind === "Random") {
      const cfg = sampler?.config && typeof sampler.config === "object" ? sampler.config : null;
      if (cfg) {
        if ($("moRandomSeed")) $("moRandomSeed").value = String(Number(cfg.seed ?? 1));
        if ($("moRandomTrials")) $("moRandomTrials").value = String(Number(cfg.trials ?? 2000));
      }
    } else if (samplerKind === "Tpe") {
      const cfg = sampler?.config && typeof sampler.config === "object" ? sampler.config : null;
      if (cfg) {
        if ($("moTpeSeed")) $("moTpeSeed").value = String(Number(cfg.seed ?? 1));
        if ($("moTpeTrials")) $("moTpeTrials").value = String(Number(cfg.trials ?? 5000));
        if ($("moTpeStartupTrials")) $("moTpeStartupTrials").value = String(Number(cfg.startup_trials ?? 10));
        if ($("moTpeCandidates")) $("moTpeCandidates").value = String(Number(cfg.n_ei_candidates ?? 24));
        if ($("moTpeGamma")) $("moTpeGamma").value = String(num(cfg.gamma, 0.25));
        if ($("moTpeBatchSize")) $("moTpeBatchSize").value = String(Number(cfg.batch_size ?? 64));
        if ($("moTpeMaxModelPoints")) $("moTpeMaxModelPoints").value = String(Number(cfg.max_model_points ?? 2048));
      }
    } else if (samplerKind === "Qmc") {
      const cfg = sampler?.config && typeof sampler.config === "object" ? sampler.config : null;
      if (cfg) {
        if ($("moQmcSeed")) $("moQmcSeed").value = String(Number(cfg.seed ?? 1));
        if ($("moQmcTrials")) $("moQmcTrials").value = String(Number(cfg.trials ?? 5000));
        if ($("moQmcSequence")) $("moQmcSequence").value = String(cfg.sequence ?? "halton");
      }
    }

    const constraints = req.constraints && typeof req.constraints === "object" ? req.constraints : null;
    if ($("moConstraintMinTrades")) $("moConstraintMinTrades").value = constraints?.min_trades == null ? "" : String(Number(constraints.min_trades));
    if ($("moConstraintMaxDd")) $("moConstraintMaxDd").value = constraints?.max_drawdown == null ? "" : String(num(constraints.max_drawdown, ""));
    if ($("moConstraintMaxAbsExposure")) $("moConstraintMaxAbsExposure").value = constraints?.max_abs_exposure == null ? "" : String(num(constraints.max_abs_exposure, ""));

    const prescan = req.gpu_prescan && typeof req.gpu_prescan === "object" ? req.gpu_prescan : null;
    if ($("moPrescanEnable")) $("moPrescanEnable").checked = !!prescan;
    if ($("moPrescanRequireGpu")) $("moPrescanRequireGpu").checked = !!prescan?.require_gpu;
    if ($("moPrescanTopK")) $("moPrescanTopK").value = String(Number(prescan?.top_k ?? 128));
    if ($("moPrescanWindow")) $("moPrescanWindow").value = String(Number(prescan?.window ?? 8));
    if ($("moPrescanPredictiveTopK")) $("moPrescanPredictiveTopK").value = String(Number(prescan?.predictive_top_k ?? 12));
    if ($("moPrescanPredictiveStride")) $("moPrescanPredictiveStride").value = String(Number(prescan?.predictive_stride ?? 8));
    if ($("moPrescanObjectives")) $("moPrescanObjectives").value = String(prescan?.objectives ?? "both");
    if ($("moPrescanMode")) $("moPrescanMode").value = String(prescan?.mode ?? "Auto");
    if ($("moPrescanMaPairs")) {
      const pairs = Array.isArray(prescan?.ma_pairs) ? prescan.ma_pairs : [];
      $("moPrescanMaPairs").value = pairs
        .map((p) => `${String(p?.fast_ma_type || "").trim()},${String(p?.slow_ma_type || "").trim()}`)
        .filter((s) => s.trim())
        .join("\n");
    }

    if ($("moSearchSpace")) {
      const space = req.search_space && typeof req.search_space === "object" ? req.search_space : null;
      $("moSearchSpace").value = space ? JSON.stringify(space, null, 2) : "";
    }

    updateMoSamplerUi();
    updateDeviceIdEnabled();
  }

  const fastParams = req.fast_ma_params && typeof req.fast_ma_params === "object" ? req.fast_ma_params : null;
  const slowParams = req.slow_ma_params && typeof req.slow_ma_params === "object" ? req.slow_ma_params : null;
  if (fastParams) {
    const s = ensureMaParamState("fast", fastMa);
    for (const [k, v] of Object.entries(fastParams)) {
      const n = Number(v);
      if (Number.isFinite(n)) s[k] = n;
    }
  }
  if (slowParams) {
    const s = ensureMaParamState("slow", slowMa);
    for (const [k, v] of Object.entries(slowParams)) {
      const n = Number(v);
      if (Number.isFinite(n)) s[k] = n;
    }
  }

  const hasSweepField =
    Object.prototype.hasOwnProperty.call(req, "fast_ma_param_sweeps") ||
    Object.prototype.hasOwnProperty.call(req, "slow_ma_param_sweeps") ||
    Object.prototype.hasOwnProperty.call(req, "fastMaParamSweeps") ||
    Object.prototype.hasOwnProperty.call(req, "slowMaParamSweeps");
  const fastSweeps = (req.fast_ma_param_sweeps && typeof req.fast_ma_param_sweeps === "object")
    ? req.fast_ma_param_sweeps
    : ((req.fastMaParamSweeps && typeof req.fastMaParamSweeps === "object") ? req.fastMaParamSweeps : null);
  const slowSweeps = (req.slow_ma_param_sweeps && typeof req.slow_ma_param_sweeps === "object")
    ? req.slow_ma_param_sweeps
    : ((req.slowMaParamSweeps && typeof req.slowMaParamSweeps === "object") ? req.slowMaParamSweeps : null);

  if (hasSweepField) {
    if (!MA_PARAM_SWEEPS_STATE || typeof MA_PARAM_SWEEPS_STATE !== "object") MA_PARAM_SWEEPS_STATE = { fast: {}, slow: {} };
    if (!MA_PARAM_SWEEPS_STATE.fast || typeof MA_PARAM_SWEEPS_STATE.fast !== "object") MA_PARAM_SWEEPS_STATE.fast = {};
    if (!MA_PARAM_SWEEPS_STATE.slow || typeof MA_PARAM_SWEEPS_STATE.slow !== "object") MA_PARAM_SWEEPS_STATE.slow = {};
    MA_PARAM_SWEEPS_STATE.fast[fastMa] = {};
    MA_PARAM_SWEEPS_STATE.slow[slowMa] = {};
  }

  if (fastSweeps) {
    const s = ensureMaParamSweepState("fast", fastMa);
    for (const [k, v] of Object.entries(fastSweeps)) {
      const arr = Array.isArray(v) ? v : null;
      if (!arr || arr.length < 3) continue;
      const start = Number(arr[0]);
      const end = Number(arr[1]);
      const step = Number(arr[2]);
      if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step)) continue;
      s[k] = { enabled: true, start, end, step };
    }
  }
  if (slowSweeps) {
    const s = ensureMaParamSweepState("slow", slowMa);
    for (const [k, v] of Object.entries(slowSweeps)) {
      const arr = Array.isArray(v) ? v : null;
      if (!arr || arr.length < 3) continue;
      const start = Number(arr[0]);
      const end = Number(arr[1]);
      const step = Number(arr[2]);
      if (!Number.isFinite(start) || !Number.isFinite(end) || !Number.isFinite(step)) continue;
      s[k] = { enabled: true, start, end, step };
    }
  }

  updateStrategyUi();
  applyBackendSelectionConstraints();
  updateDeviceIdEnabled();
  applyBackendMaConstraints();
  refreshMaParamsUi();
  saveSettingsSoon();
}

function getRecentCsvPaths() {
  try {
    const v = JSON.parse(localStorage.getItem(RECENT_CSV_KEY) || "[]");
    if (!Array.isArray(v)) return [];
    return v
      .filter((x) => typeof x === "string")
      .map((x) => x.trim())
      .filter((x) => !!x);
  } catch {
    return [];
  }
}

function setRecentCsvPaths(paths) {
  try {
    localStorage.setItem(RECENT_CSV_KEY, JSON.stringify(paths));
  } catch {}
}

function renderRecentCsvDatalist(paths) {
  const dl = $("csvRecentList");
  if (!dl) return;
  dl.innerHTML = "";
  const items = Array.isArray(paths) ? paths : [];
  for (const p of items) {
    const opt = document.createElement("option");
    opt.value = String(p || "");
    dl.appendChild(opt);
  }
}

function addRecentCsvPath(path) {
  const p = String(path || "").trim();
  if (!p) return;
  const prev = getRecentCsvPaths();
  const next = [p, ...prev.filter((x) => x !== p)].slice(0, RECENT_CSV_MAX);
  setRecentCsvPaths(next);
  renderRecentCsvDatalist(next);
}

function getHistory() {
  try {
    const v = JSON.parse(localStorage.getItem(HISTORY_KEY) || "[]");
    return Array.isArray(v) ? v : [];
  } catch {
    return [];
  }
}

function setHistory(history) {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
  } catch {}
}

function populateHistorySelect(history, selectedTs) {
  const sel = $("historySelect");
  if (!sel) return;
  sel.innerHTML = "";

  const opt0 = document.createElement("option");
  opt0.value = "";
  opt0.textContent = history.length ? "Select a run..." : "No runs yet";
  sel.appendChild(opt0);

  for (const h of history) {
    const opt = document.createElement("option");
    opt.value = h.ts || "";
    opt.textContent = `${h.ts || "unknown"} | ${h.backend || ""} | ${h.pairs || 0} pairs`;
    sel.appendChild(opt);
  }
  sel.value = selectedTs || "";
}

function populateCompareSelect(history, selectedTs) {
  const sel = $("compareSelect");
  if (!sel) return;
  sel.innerHTML = "";

  const opt0 = document.createElement("option");
  opt0.value = "";
  const disk = Array.isArray(SAVED_RUNS) ? SAVED_RUNS : [];
  opt0.textContent = (history.length || disk.length) ? "Compare against..." : "No runs yet";
  sel.appendChild(opt0);

  if (disk.length) {
    const og = document.createElement("optgroup");
    og.label = "Saved runs (disk)";
    for (const r of disk) {
      const opt = document.createElement("option");
      opt.value = `disk:${r.path || ""}`;
      const hasVal = r.has_validation ? "val" : "-";
      const hasXlsx = r.report_xlsx_path ? "xlsx" : "-";
      opt.textContent = `${r.ts || "unknown"} | ${r.strategy_kind || ""} | ${hasVal}/${hasXlsx}`;
      og.appendChild(opt);
    }
    sel.appendChild(og);
  }

  if (history.length) {
    const og = document.createElement("optgroup");
    og.label = "Session history";
    for (const h of history) {
      const opt = document.createElement("option");
      opt.value = `hist:${h.ts || ""}`;
      opt.textContent = `${h.ts || "unknown"} | ${h.backend || ""} | ${h.pairs || 0} pairs`;
      og.appendChild(opt);
    }
    sel.appendChild(og);
  }

  let v = String(selectedTs || "").trim();
  if (v && !v.startsWith("hist:") && !v.startsWith("disk:")) {
    // Back-compat: older saved value was a raw `ts`.
    if (history.some((h) => String(h?.ts || "") === v)) v = `hist:${v}`;
  }
  sel.value = v;
}

function populateSavedRunsSelect(runs, selectedPath) {
  const sel = $("savedRunsSelect");
  if (!sel) return;
  sel.innerHTML = "";

  const items = Array.isArray(runs) ? runs : [];
  const opt0 = document.createElement("option");
  opt0.value = "";
  opt0.textContent = items.length ? "Select a saved run..." : "No saved runs yet";
  sel.appendChild(opt0);

  for (const r of items) {
    const opt = document.createElement("option");
    opt.value = r.path || "";
    const hasVal = r.has_validation ? "val" : "-";
    const hasXlsx = r.report_xlsx_path ? "xlsx" : "-";
    const kind = r.strategy_kind || "";
    const tagsN = Array.isArray(r.tags) ? r.tags.length : 0;
    const tagsHint = tagsN ? ` | tags=${tagsN}` : "";
    opt.textContent = `${r.ts || "unknown"} | ${kind} | ${hasVal}/${hasXlsx}${tagsHint}`;
    sel.appendChild(opt);
  }
  sel.value = selectedPath || "";
}

function savedRunsQuery() {
  return String($("savedRunsSearch")?.value || "").trim().toLowerCase();
}

function filterSavedRuns(runs, query) {
  const q = String(query || "").trim().toLowerCase();
  if (!q) return Array.isArray(runs) ? runs : [];
  const terms = q.split(/\s+/).filter((t) => !!t);
  if (!terms.length) return Array.isArray(runs) ? runs : [];

  const xs = Array.isArray(runs) ? runs : [];
  return xs.filter((r) => {
    const ts = String(r?.ts || "").toLowerCase();
    const kind = String(r?.strategy_kind || "").toLowerCase();
    const csv = String(r?.csv_path || "").toLowerCase();
    const tags = Array.isArray(r?.tags) ? r.tags.map((t) => String(t || "").toLowerCase()).join(" ") : "";
    const hay = `${ts} ${kind} ${csv} ${tags}`;
    return terms.every((t) => hay.includes(t));
  });
}

function renderSavedRunsSelect(selectedPath) {
  const cur = String(selectedPath || $("savedRunsSelect")?.value || "").trim();
  const filtered = filterSavedRuns(SAVED_RUNS, savedRunsQuery());
  populateSavedRunsSelect(filtered, cur);

  const meta = $("savedRunsMeta");
  if (meta) meta.textContent = `saved runs: ${filtered.length}/${SAVED_RUNS.length}`;
  syncSavedRunSelectionUi();
}

async function refreshSavedRuns(selectedPath) {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    const runs = await invoke("list_run_artifacts");
    SAVED_RUNS = Array.isArray(runs) ? runs : [];
    COMPARE_BASELINE_CACHE = {};
    SAVED_RUN_DETAIL_CACHE = {};
    CURRENT_SAVED_RUN_DETAIL_PATH = "";
    renderSavedRunsSelect(selectedPath);
    populateCompareSelect(getHistory(), $("compareSelect")?.value || "");
    updateAggregateSummaryCards();
  } catch (e) {
    const meta = $("savedRunsMeta");
    if (meta) meta.textContent = `failed to list saved runs: ${String(e)}`;
    updateAggregateSummaryCards();
  }
}

function populateMetaSavedSelect(items, selectedPath) {
  const sel = $("metaSavedSelect");
  if (!sel) return;
  sel.innerHTML = "";

  const rows = Array.isArray(items) ? items : [];
  const opt0 = document.createElement("option");
  opt0.value = "";
  opt0.textContent = rows.length ? "Select a saved meta run..." : "No meta runs yet";
  sel.appendChild(opt0);

  for (const r of rows) {
    const opt = document.createElement("option");
    opt.value = r.path || "";
    const kind = String(r.strategy_kind || "");
    const tf = String(r.timeframe || "");
    const groups = Number(r.groups || 0);
    const artifacts = Number(r.artifacts_used || 0);
    const holdout = Number(r.holdout_datasets || 0);
    const tagsN = Array.isArray(r.tags) ? r.tags.length : 0;
    const tagsHint = tagsN ? ` | tags=${tagsN}` : "";
    const tfHint = tf ? ` | ${tf}` : "";
    const holdoutHint = holdout ? ` | holdout=${holdout}` : "";
    opt.textContent = `${r.ts || "unknown"} | ${kind}${tfHint} | groups=${groups} | arts=${artifacts}${holdoutHint}${tagsHint}`;
    sel.appendChild(opt);
  }
  sel.value = selectedPath || "";
}

function metaSavedQuery() {
  return String($("metaSavedSearch")?.value || "").trim().toLowerCase();
}

function filterMetaSaved(items, query) {
  const q = String(query || "").trim().toLowerCase();
  if (!q) return Array.isArray(items) ? items : [];
  const terms = q.split(/\s+/).filter((t) => !!t);
  if (!terms.length) return Array.isArray(items) ? items : [];

  const xs = Array.isArray(items) ? items : [];
  return xs.filter((r) => {
    const ts = String(r?.ts || "").toLowerCase();
    const kind = String(r?.strategy_kind || "").toLowerCase();
    const tf = String(r?.timeframe || "").toLowerCase();
    const tag = String(r?.required_tag || "").toLowerCase();
    const tags = Array.isArray(r?.tags) ? r.tags.map((t) => String(t || "").toLowerCase()).join(" ") : "";
    const hay = `${ts} ${kind} ${tf} ${tag} ${tags}`;
    return terms.every((t) => hay.includes(t));
  });
}

function renderMetaSavedSelect(selectedPath) {
  const cur = String(selectedPath || $("metaSavedSelect")?.value || "").trim();
  const filtered = filterMetaSaved(SAVED_META, metaSavedQuery());
  populateMetaSavedSelect(filtered, cur);
  syncMetaSavedSelectionUi();
  populateMetaCompareSelect(SAVED_META, $("metaCompareSelect")?.value || "");
  renderMetaCompare();
}

async function refreshMetaSaved(selectedPath) {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    const runs = await invoke("list_meta_artifacts");
    SAVED_META = Array.isArray(runs) ? runs : [];
    META_BASELINE_CACHE = {};
    renderMetaSavedSelect(selectedPath);
    updateAggregateSummaryCards();
  } catch (e) {
    const meta = $("metaSavedMeta");
    if (meta) meta.textContent = `failed to list meta runs: ${String(e)}`;
    updateAggregateSummaryCards();
  }
}

function populateStage4SavedSelect(items, selectedPath) {
  const sel = $("stage4SavedSelect");
  if (!sel) return;
  sel.innerHTML = "";

  const rows = Array.isArray(items) ? items : [];
  const opt0 = document.createElement("option");
  opt0.value = "";
  opt0.textContent = rows.length ? "Select a saved Stage 4 run..." : "No Stage 4 runs yet";
  sel.appendChild(opt0);

  for (const r of rows) {
    const opt = document.createElement("option");
    opt.value = r.path || "";
    const kind = String(r.strategy_kind || "");
    const tf = String(r.timeframe || "");
    const selected = Number(r.groups_selected || 0);
    const scored = Number(r.groups_scored || 0);
    const considered = Number(r.groups_considered || 0);
    const minConf = Number(r.min_ml_confidence);
    const tagsN = Array.isArray(r.tags) ? r.tags.length : 0;
    const tfHint = tf ? ` | ${tf}` : "";
    const confHint = Number.isFinite(minConf) ? ` | min_ml=${fmtFixed(minConf, 2)}` : "";
    const tagsHint = tagsN ? ` | tags=${tagsN}` : "";
    opt.textContent =
      `${r.ts || "unknown"} | ${kind}${tfHint} | selected=${selected}/${scored}/${considered}${confHint}${tagsHint}`;
    sel.appendChild(opt);
  }
  sel.value = selectedPath || "";
}

function stage4SavedQuery() {
  return String($("stage4SavedSearch")?.value || "").trim().toLowerCase();
}

function filterStage4Saved(items, query) {
  const q = String(query || "").trim().toLowerCase();
  if (!q) return Array.isArray(items) ? items : [];
  const terms = q.split(/\s+/).filter((t) => !!t);
  if (!terms.length) return Array.isArray(items) ? items : [];

  const xs = Array.isArray(items) ? items : [];
  return xs.filter((r) => {
    const ts = String(r?.ts || "").toLowerCase();
    const kind = String(r?.strategy_kind || "").toLowerCase();
    const tf = String(r?.timeframe || "").toLowerCase();
    const src = String(r?.source_meta_json_path || "").toLowerCase();
    const tags = Array.isArray(r?.tags) ? r.tags.map((t) => String(t || "").toLowerCase()).join(" ") : "";
    const hay = `${ts} ${kind} ${tf} ${src} ${tags}`;
    return terms.every((t) => hay.includes(t));
  });
}

function renderStage4SavedSelect(selectedPath) {
  const cur = String(selectedPath || $("stage4SavedSelect")?.value || "").trim();
  const filtered = filterStage4Saved(SAVED_STAGE4, stage4SavedQuery());
  populateStage4SavedSelect(filtered, cur);
  syncStage4SavedSelectionUi();
}

async function refreshStage4Saved(selectedPath) {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    const runs = await invoke("list_stage4_artifacts");
    SAVED_STAGE4 = Array.isArray(runs) ? runs : [];
    renderStage4SavedSelect(selectedPath);
  } catch (e) {
    const meta = $("stage4SavedMeta");
    if (meta) meta.textContent = `failed to list Stage 4 runs: ${String(e)}`;
  }
}

function selectedStage4SavedEntry() {
  const path = String($("stage4SavedSelect")?.value || "").trim();
  if (!path) return null;
  return SAVED_STAGE4.find((r) => String(r?.path || "") === path) || null;
}

function syncStage4SavedSelectionUi() {
  updateStage4SavedMeta();
  const entry = selectedStage4SavedEntry();
  const tagsEl = $("stage4SavedTags");
  if (tagsEl) tagsEl.value = entry && Array.isArray(entry.tags) ? entry.tags.join(", ") : "";

  const btnLoad = $("btnLoadStage4Saved");
  const btnReveal = $("btnRevealStage4Saved");
  const btnDelete = $("btnDeleteStage4Saved");
  const btnSaveTags = $("btnSaveStage4SavedTags");
  const btnRefresh = $("btnRefreshStage4Saved");
  const has = !!entry;
  if (btnRefresh) btnRefresh.disabled = RUNNING;
  if (btnLoad) btnLoad.disabled = RUNNING || !has;
  if (btnReveal) btnReveal.disabled = RUNNING || !has;
  if (btnDelete) btnDelete.disabled = RUNNING || !has;
  if (btnSaveTags) btnSaveTags.disabled = RUNNING || !has;
  if (tagsEl) tagsEl.disabled = RUNNING || !has;
}

function updateStage4SavedMeta() {
  const meta = $("stage4SavedMeta");
  if (!meta) return;
  const entry = selectedStage4SavedEntry();
  if (!entry) {
    const filtered = filterStage4Saved(SAVED_STAGE4, stage4SavedQuery());
    meta.textContent = `saved Stage 4: ${filtered.length}/${SAVED_STAGE4.length}`;
    return;
  }
  const parts = [];
  if (entry.ts) parts.push(`ts=${entry.ts}`);
  if (entry.strategy_kind) parts.push(`strategy=${entry.strategy_kind}`);
  if (entry.timeframe) parts.push(`tf=${entry.timeframe}`);
  parts.push(
    `selected=${Number(entry.groups_selected || 0)}/${Number(entry.groups_scored || 0)}/${Number(entry.groups_considered || 0)}`
  );
  if (Number.isFinite(Number(entry.min_ml_confidence))) {
    parts.push(`min_ml=${fmtFixed(Number(entry.min_ml_confidence), 3)}`);
  }
  if (Number.isFinite(Number(entry.top_k))) parts.push(`top_k=${Number(entry.top_k)}`);
  parts.push(`model=${entry.model_healthy ? "healthy" : "unhealthy"}`);
  if (entry.source_meta_json_path) parts.push(`source=${fileBasename(entry.source_meta_json_path)}`);
  if (Array.isArray(entry.tags) && entry.tags.length) parts.push(`tags=${entry.tags.join(",")}`);
  if (entry.bytes != null) parts.push(`bytes=${entry.bytes}`);
  meta.textContent = parts.join(" • ");
}

function buildStage4ResponseFromArtifact(artifact, path) {
  const ts = String(artifact?.ts || "").trim() || new Date().toISOString();
  const result = artifact?.res && typeof artifact.res === "object" ? artifact.res : null;
  return {
    ts,
    stage4_json_path: String(path || "").trim(),
    stage4_json_bytes: 0,
    result,
  };
}

async function saveStage4SavedTags(path, tags) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Select a saved Stage 4 run first.");

  await invoke("update_artifact_tags", { path: p, tags: Array.isArray(tags) ? tags : [] });
  await refreshStage4Saved(p);
  setStatus("Saved Stage 4 tags.", "ok");
}

async function loadStage4Saved(path) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Select a saved Stage 4 run first.");

  const artifact = await invoke("load_run_artifact", { path: p });
  const kind = String(artifact?.kind || "");
  if (kind !== "vectorbt_meta_stage4_experimental") {
    throw new Error(`Not a Stage 4 artifact: kind='${kind}'`);
  }
  const resp = buildStage4ResponseFromArtifact(artifact, p);

  const result = resp?.result || {};
  const sourceMetaPath = String(result?.source_meta_json_path || "").trim();
  if (sourceMetaPath && String(LAST_META_RUN?.meta_json_path || "").trim() !== sourceMetaPath) {
    try {
      const metaArtifact = await invoke("load_run_artifact", { path: sourceMetaPath });
      if (String(metaArtifact?.kind || "") === "vectorbt_meta_selection") {
        const metaResp = buildMetaSelectionResponseFromArtifact(metaArtifact, sourceMetaPath);
        LAST_META_RUN = metaResp;
        renderMetaSelection(metaResp);
        updateValidationExportButtons();
        await maybeAutoTrainAndApplyMetaMl({ silent: true });
      }
    } catch {
      // Stage 4 can still be viewed even when source meta file was removed.
    }
  }

  LAST_META_STAGE4_RUN = resp;
  renderMetaStage4Output(resp);
  setMetaStage4Status(
    `Loaded Stage 4 ${resp.ts} · selected=${Number(result?.groups_selected || 0)} · source=${sourceMetaPath ? fileBasename(sourceMetaPath) : "—"}`
  );
  updateMetaStage4Buttons();
  RESULTS_TABS?.set("meta", { save: true, focus: false });
  setStatus(`Loaded Stage 4 run ${resp.ts}`, "muted");
}

async function revealSelectedStage4Saved() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const entry = selectedStage4SavedEntry();
  const p = String(entry?.path || "").trim();
  if (!p) return;
  await invoke("reveal_path", { path: p });
}

async function deleteSelectedStage4Saved() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const entry = selectedStage4SavedEntry();
  const p = String(entry?.path || "").trim();
  if (!p) return;
  await invoke("delete_run_artifact", { path: p });
  if (LAST_META_STAGE4_RUN && String(LAST_META_STAGE4_RUN.stage4_json_path || "") === p) {
    LAST_META_STAGE4_RUN = null;
    renderMetaStage4Output(null);
    setMetaStage4Status("—");
    updateMetaStage4Buttons();
  }
  await refreshStage4Saved("");
  setStatus("Deleted Stage 4 run.", "ok");
}

function selectedMetaSavedEntry() {
  const path = String($("metaSavedSelect")?.value || "").trim();
  if (!path) return null;
  return SAVED_META.find((r) => String(r?.path || "") === path) || null;
}

function syncMetaSavedSelectionUi() {
  updateMetaSavedMeta();
  const entry = selectedMetaSavedEntry();
  const tagsEl = $("metaSavedTags");
  if (tagsEl) tagsEl.value = entry && Array.isArray(entry.tags) ? entry.tags.join(", ") : "";

  const btnLoad = $("btnLoadMetaSaved");
  const btnOpen = $("btnOpenMetaSavedReport");
  const btnReveal = $("btnRevealMetaSaved");
  const btnDelete = $("btnDeleteMetaSaved");
  const btnSaveTags = $("btnSaveMetaSavedTags");

  const has = !!entry;
  if (btnLoad) btnLoad.disabled = RUNNING || !has;
  if (btnOpen) btnOpen.disabled = RUNNING || !has || !String(entry?.report_xlsx_path || "").trim();
  if (btnReveal) btnReveal.disabled = RUNNING || !has;
  if (btnDelete) btnDelete.disabled = RUNNING || !has;
  if (btnSaveTags) btnSaveTags.disabled = RUNNING || !has;
  if (tagsEl) tagsEl.disabled = RUNNING || !has;
}

function updateMetaSavedMeta() {
  const meta = $("metaSavedMeta");
  if (!meta) return;
  const entry = selectedMetaSavedEntry();
  if (!entry) {
    const filtered = filterMetaSaved(SAVED_META, metaSavedQuery());
    meta.textContent = `saved meta: ${filtered.length}/${SAVED_META.length}`;
    return;
  }
  const parts = [];
  if (entry.ts) parts.push(`ts=${entry.ts}`);
  if (entry.strategy_kind) parts.push(`strategy=${entry.strategy_kind}`);
  if (entry.timeframe) parts.push(`tf=${entry.timeframe}`);
  if (entry.required_tag) parts.push(`tag=${entry.required_tag}`);
  parts.push(`groups=${Number(entry.groups || 0)}`);
  parts.push(`artifacts=${Number(entry.artifacts_used || 0)}`);
  const holdout = Number(entry.holdout_datasets || 0);
  if (holdout) parts.push(`holdout=${holdout}`);
  if (Array.isArray(entry.tags) && entry.tags.length) parts.push(`tags=${entry.tags.join(",")}`);
  if (entry.bytes != null) parts.push(`bytes=${entry.bytes}`);
  meta.textContent = parts.join(" • ");
}

function buildMetaSelectionResponseFromArtifact(artifact, path) {
  const ts = String(artifact?.ts || "").trim() || new Date().toISOString();
  const report_xlsx_path = String(artifact?.report_xlsx_path || "").trim();
  const result = artifact?.res && typeof artifact.res === "object" ? artifact.res : null;
  return {
    ts,
    report_xlsx_path,
    report_xlsx_bytes: 0,
    meta_json_path: String(path || "").trim(),
    meta_json_bytes: 0,
    result,
  };
}

async function loadMetaSaved(path) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Select a saved meta run first.");

  const artifact = await invoke("load_run_artifact", { path: p });
  const kind = String(artifact?.kind || "");
  if (kind !== "vectorbt_meta_selection") throw new Error(`Not a meta artifact: kind='${kind}'`);
  const resp = buildMetaSelectionResponseFromArtifact(artifact, p);

  LAST_META_RUN = resp;
  renderMetaSelection(resp);
  updateValidationExportButtons();
  await maybeAutoTrainAndApplyMetaMl({ silent: true });
  RESULTS_TABS?.set("meta", { save: true, focus: false });
  renderMetaCompare();
  updateQuickActionsUi();
  setStatus(`Loaded meta run ${resp.ts}`, "muted");
}

async function openSelectedMetaSavedReport() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const entry = selectedMetaSavedEntry();
  const reportPath = String(entry?.report_xlsx_path || "").trim();
  if (!reportPath) {
    setStatus("No report path for this meta run.", "muted");
    return;
  }
  try {
    await invoke("open_path", { path: reportPath });
    setStatus("Opened report.", "ok");
  } catch {
    await invoke("reveal_path", { path: reportPath });
    setStatus("Revealed report.", "ok");
  }
}

async function revealSelectedMetaSaved() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const entry = selectedMetaSavedEntry();
  const p = String(entry?.path || "").trim();
  if (!p) return;
  await invoke("reveal_path", { path: p });
}

async function deleteSelectedMetaSaved() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const entry = selectedMetaSavedEntry();
  const p = String(entry?.path || "").trim();
  if (!p) return;
  await invoke("delete_run_artifact", { path: p });
  if (LAST_META_RUN && String(LAST_META_RUN.meta_json_path || "") === p) {
    LAST_META_RUN = null;
    renderMetaSelection(null);
    updateValidationExportButtons();
    updateQuickActionsUi();
  }
  await refreshMetaSaved("");
  setStatus("Deleted meta run.", "ok");
}

function populateMetaCompareSelect(items, selectedPath) {
  const sel = $("metaCompareSelect");
  if (!sel) return;
  sel.innerHTML = "";

  const rows = Array.isArray(items) ? items : [];
  const opt0 = document.createElement("option");
  opt0.value = "";
  opt0.textContent = rows.length ? "Compare against..." : "No meta runs yet";
  sel.appendChild(opt0);

  for (const r of rows) {
    const opt = document.createElement("option");
    opt.value = r.path || "";
    const kind = String(r.strategy_kind || "");
    const tf = String(r.timeframe || "");
    const groups = Number(r.groups || 0);
    const tfHint = tf ? ` | ${tf}` : "";
    opt.textContent = `${r.ts || "unknown"} | ${kind}${tfHint} | groups=${groups}`;
    sel.appendChild(opt);
  }
  sel.value = selectedPath || "";
}

function fmtPct(x, digits) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const d = Number.isFinite(digits) ? digits : 1;
  return (n * 100).toFixed(d) + "%";
}

async function loadMetaArtifactCached(path) {
  const p = String(path || "").trim();
  if (!p) return null;
  const cached = META_BASELINE_CACHE[p];
  if (cached) return cached;
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const artifact = await invoke("load_run_artifact", { path: p });
  META_BASELINE_CACHE[p] = artifact;
  return artifact;
}

async function renderMetaCompare() {
  const el = $("metaCompareSummary");
  if (!el) return;

  const cur = LAST_META_RUN?.result && typeof LAST_META_RUN.result === "object" ? LAST_META_RUN.result : null;
  const curGroups = Array.isArray(cur?.groups) ? cur.groups : [];
  if (!cur || !curGroups.length) {
    el.textContent = "—";
    return;
  }

  const baselinePath = String($("metaCompareSelect")?.value || "").trim();
  if (!baselinePath) {
    el.textContent = "Select a baseline meta run to compare.";
    return;
  }

  const seq = ++META_COMPARE_SEQ;
  try {
    const baselineArt = await loadMetaArtifactCached(baselinePath);
    if (seq !== META_COMPARE_SEQ) return;

    const base = baselineArt?.res && typeof baselineArt.res === "object" ? baselineArt.res : null;
    const baseGroups = Array.isArray(base?.groups) ? base.groups : [];
    if (!base || !baseGroups.length) {
      el.textContent = "Baseline meta run has no groups.";
      return;
    }

    const curTop = curGroups[0] || {};
    const baseTop = baseGroups[0] || {};

    const curKeys = new Set(curGroups.map((g) => String(g?.key || "")).filter((k) => !!k));
    const baseKeys = new Set(baseGroups.map((g) => String(g?.key || "")).filter((k) => !!k));
    let overlap = 0;
    for (const k of curKeys) if (baseKeys.has(k)) overlap++;
    const union = curKeys.size + baseKeys.size - overlap;
    const jaccard = union ? overlap / union : 0;

    const delta = (a, b) => {
      const x = Number(a);
      const y = Number(b);
      if (!Number.isFinite(x) || !Number.isFinite(y)) return "-";
      const d = x - y;
      return (d >= 0 ? "+" : "") + d.toFixed(6);
    };

    const curHold = curTop?.holdout && typeof curTop.holdout === "object" ? curTop.holdout : null;
    const baseHold = baseTop?.holdout && typeof baseTop.holdout === "object" ? baseTop.holdout : null;

    const lines = [];
    lines.push(`Current: groups=${curGroups.length} artifacts=${Number(cur.artifacts_used || 0)}/${Number(cur.artifacts_considered || 0)} runtime=${Number(cur.runtime_ms || 0)}ms`);
    lines.push(`Baseline: groups=${baseGroups.length} artifacts=${Number(base.artifacts_used || 0)}/${Number(base.artifacts_considered || 0)} runtime=${Number(base.runtime_ms || 0)}ms`);
    lines.push(`Overlap: ${overlap}/${Math.min(curKeys.size, baseKeys.size)} (Jaccard ${jaccard.toFixed(3)})`);
    lines.push("");
    lines.push("Top-1 deltas (current - baseline):");
    lines.push(`  meta_score: ${delta(curTop.meta_score, baseTop.meta_score)}`);
    lines.push(`  robust_p10: ${delta(curTop.robust_score_p10, baseTop.robust_score_p10)}`);
    lines.push(`  robust_med: ${delta(curTop.robust_score_median, baseTop.robust_score_median)}`);
    lines.push(`  dd_p90: ${delta(curTop.max_dd_p90, baseTop.max_dd_p90)}`);
    lines.push(`  pass_rate: ${fmtPct(Number(curTop.pass_rate || 0), 1)} vs ${fmtPct(Number(baseTop.pass_rate || 0), 1)}`);

    if (curHold || baseHold) {
      lines.push("");
      lines.push("Holdout (top-1):");
      if (curHold) lines.push(`  current: pass=${fmtPct(Number(curHold.pass_rate || 0), 1)} p10=${Number(curHold.score_p10 || 0).toFixed(6)} dd_p90=${Number(curHold.max_dd_p90 || 0).toFixed(6)}`);
      else lines.push("  current: -");
      if (baseHold) lines.push(`  baseline: pass=${fmtPct(Number(baseHold.pass_rate || 0), 1)} p10=${Number(baseHold.score_p10 || 0).toFixed(6)} dd_p90=${Number(baseHold.max_dd_p90 || 0).toFixed(6)}`);
      else lines.push("  baseline: -");
    }

    el.textContent = lines.join("\n");
  } catch (e) {
    if (seq !== META_COMPARE_SEQ) return;
    el.textContent = `Meta compare failed: ${String(e)}`;
  }
}

function selectedSavedRunEntry() {
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) return null;
  return SAVED_RUNS.find((r) => String(r?.path || "") === path) || null;
}

function strategyKindLabel(kind) {
  const k = String(kind || "").trim();
  if (k === "CompositeCpu") return "Composite CPU (legacy)";
  if (k === "CompositeMo") return "Composite MO";
  if (k === "CompositeTrend") return "Composite Trend-Quality";
  if (k === "RegimeHybrid") return "Regime-Switch Hybrid";
  if (k === "BreakoutMomentum") return "Breakout Pulse";
  return "Double MA";
}

function formatRangeLabel(range) {
  const r = range && typeof range === "object" ? range : null;
  if (!r) return "-";
  const start = Number(r.start);
  const end = Number(r.end);
  const step = Number(r.step);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return "-";
  const a = Math.floor(start);
  const b = Math.floor(end);
  if (Number.isFinite(step) && step > 0) return `${a}-${b} x${Math.floor(step)}`;
  return `${a}-${b}`;
}

function buildSavedRunReqSnapshot(req) {
  if (!req || typeof req !== "object") return "Request unavailable.";
  const kind = kindFromReq(req);
  const backend = String(req?.backend?.backend || "").trim() || "CpuOnly";
  const objective = String(req?.objective || req?.objectives?.maximize || "").trim() || "-";
  const fastTypes = Array.isArray(req?.fast_ma_types) ? req.fast_ma_types.map((x) => String(x || "")).filter((x) => !!x).join(", ") : "";
  const slowTypes = Array.isArray(req?.slow_ma_types) ? req.slow_ma_types.map((x) => String(x || "")).filter((x) => !!x).join(", ") : "";
  const maSource = String(req?.ma_source || "").trim() || "close";
  const lines = [];

  lines.push(`strategy: ${strategyKindLabel(kind)}`);
  lines.push(`backend: ${backend}`);
  lines.push(`objective: ${objective}`);
  lines.push(`ma source: ${maSource}`);
  lines.push(`fast range: ${formatRangeLabel(req?.fast_range)}`);
  lines.push(`slow range: ${formatRangeLabel(req?.slow_range)}`);
  lines.push(`fast ma: ${fastTypes || "-"}`);
  lines.push(`slow ma: ${slowTypes || "-"}`);

  const strategy = req?.strategy && typeof req.strategy === "object" ? req.strategy : null;
  if (strategy) {
    const flags = [];
    if (strategy.long_only === true) flags.push("long-only");
    if (strategy.allow_flip === true) flags.push("allow-flip");
    if (strategy.trade_on_next_bar === true) flags.push("trade-next");
    const commission = Number(strategy.commission || 0);
    const feeBps = Number(strategy.fee_bps);
    const spreadBps = Number(strategy.spread_bps);
    const slipBps = Number(strategy.slippage_bps);
    const stopLossPct = Number(strategy.stop_loss_pct);
    const takeProfitPct = Number(strategy.take_profit_pct);
    const trailingStopPct = Number(strategy.trailing_stop_pct);
    const cooldownBars = Math.max(0, Math.floor(num(strategy.cooldown_bars, 0)));
    const signalPersistenceBars = Math.max(1, Math.floor(num(strategy.signal_persistence_bars, 1)));
    const exitMode = String(strategy.exit_mode || "cross").trim() || "cross";
    const kPeriod = Math.max(1, Math.floor(num(strategy.kaufman_stop_period, 22)));
    const kMult = num(strategy.kaufman_stop_mult, 2.0);
    const kMa = String(strategy.kaufman_stop_ma_type || "sma").trim() || "sma";
    const eps = Number(strategy.eps_rel || 0);
    const hasBps =
      Number.isFinite(feeBps) && feeBps >= 0
      && Number.isFinite(spreadBps) && spreadBps >= 0
      && Number.isFinite(slipBps) && slipBps >= 0;
    const costText = hasBps
      ? `fee_bps=${fmtFixed(feeBps, 2)}, spread_bps=${fmtFixed(spreadBps, 2)}, slippage_bps=${fmtFixed(slipBps, 2)}`
      : `commission=${fmtFixed(commission, 4)}`;
    const riskParts = [];
    if (Number.isFinite(stopLossPct) && stopLossPct > 0) riskParts.push(`sl=${fmtFixed(stopLossPct, 4)}`);
    if (Number.isFinite(takeProfitPct) && takeProfitPct > 0) riskParts.push(`tp=${fmtFixed(takeProfitPct, 4)}`);
    if (Number.isFinite(trailingStopPct) && trailingStopPct > 0) riskParts.push(`trail=${fmtFixed(trailingStopPct, 4)}`);
    if (cooldownBars > 0) riskParts.push(`cooldown=${cooldownBars}`);
    if (signalPersistenceBars > 1) riskParts.push(`persist=${signalPersistenceBars}`);
    if (exitMode === "kaufman_stop") riskParts.push(`exit=kaufman_stop(p=${kPeriod},m=${fmtFixed(kMult, 2)},ma=${kMa})`);
    else riskParts.push("exit=cross");
    const riskText = riskParts.length ? `, ${riskParts.join(", ")}` : "";
    lines.push(`strategy params: ${costText}, eps_rel=${fmtFixed(eps, 4)}${riskText}${flags.length ? ` (${flags.join(",")})` : ""}`);
    if (kind === "BreakoutMomentum" && !!strategy.breakout_momentum_enable) {
      const bbPeriod = Math.max(1, Math.floor(num(strategy.breakout_bbands_period, 20)));
      const bbMult = num(strategy.breakout_bbands_mult, 2.0);
      const confirm = Math.max(1, Math.floor(num(strategy.breakout_confirm_bars, 1)));
      const macdFast = Math.max(1, Math.floor(num(strategy.breakout_macd_fast_period, 12)));
      const macdSlow = Math.max(1, Math.floor(num(strategy.breakout_macd_slow_period, 26)));
      const macdSignal = Math.max(1, Math.floor(num(strategy.breakout_macd_signal_period, 9)));
      const cmoPeriod = Math.max(1, Math.floor(num(strategy.breakout_cmo_period, 14)));
      const cmoMinAbs = num(strategy.breakout_cmo_min_abs, 20.0);
      const adoscShort = Math.max(1, Math.floor(num(strategy.breakout_adosc_short_period, 3)));
      const adoscLong = Math.max(1, Math.floor(num(strategy.breakout_adosc_long_period, 10)));
      const useAdx = !!strategy.breakout_use_adx;
      const adxPeriod = Math.max(1, Math.floor(num(strategy.breakout_adx_period, 14)));
      const adxMin = num(strategy.breakout_adx_min, 20.0);
      const adxText = useAdx ? `, adx=${adxPeriod}/${fmtFixed(adxMin, 2)}` : "";
      lines.push(
        `breakout filters: bb=${bbPeriod}x${fmtFixed(bbMult, 2)}, confirm=${confirm}, macd=${macdFast}/${macdSlow}/${macdSignal}, cmo=${cmoPeriod}|${fmtFixed(cmoMinAbs, 2)}, adosc=${adoscShort}/${adoscLong}${adxText}`
      );
    }
  }
  lines.push(`starting equity: ${fmtMoney(startingEquityFromReq(req), 2)}`);

  if (kind === "CompositeCpu" || kind === "CompositeMo") {
    const compact = formatCompositeParamsCompact({
      halftrend: req?.halftrend || null,
      macz: req?.macz || null,
      ott: req?.ott || null,
    });
    if (compact) lines.push(`composite filters: ${compact}`);
  }

  return lines.join("\n");
}

function buildSavedRunDetailFromArtifact(artifact) {
  const req = artifact?.run?.req && typeof artifact.run.req === "object" ? artifact.run.req : null;
  const datasetMeta = artifact?.dataset_meta && typeof artifact.dataset_meta === "object" ? artifact.dataset_meta : null;
  const strategyKind = req ? kindFromReq(req) : String(artifact?.strategy_kind || "").trim();
  const timeframe = String(datasetMeta?.timeframe || "").trim();
  const csvPath = String(artifact?.data?.csv_path || "").trim();
  const reportPath = String(artifact?.report_xlsx_path || "").trim();
  const hasValidation = !!artifact?.validation?.res;
  const tags = Array.isArray(artifact?.tags) ? artifact.tags.map((t) => String(t || "").trim()).filter((t) => !!t) : [];
  const paramsText = buildSavedRunReqSnapshot(req);
  return { strategyKind, timeframe, csvPath, reportPath, hasValidation, tags, paramsText };
}

function renderSavedRunDetailCard(entry, detail, opts) {
  const set = (id, text) => {
    const el = $(id);
    if (!el) return;
    el.textContent = String(text || "—");
  };

  if (!entry) {
    set("savedRunDetailTitle", "No saved run selected");
    set("savedRunDetailMeta", "Select a run to inspect metadata and settings.");
    set("savedRunDetailStrategy", "—");
    set("savedRunDetailDataset", "—");
    set("savedRunDetailTimeframe", "—");
    set("savedRunDetailValidation", "—");
    set("savedRunDetailReport", "—");
    set("savedRunDetailSize", "—");
    set("savedRunDetailTags", "—");
    set("savedRunDetailParams", "Select a saved run to inspect parameters.");
    return;
  }

  const ts = String(entry?.ts || "").trim() || "Selected run";
  const path = String(entry?.path || "").trim();
  const pathBase = path ? (fileBasename(path) || path) : "unknown";
  const strategyKind = String(detail?.strategyKind || entry?.strategy_kind || "").trim();
  const csvPath = String(detail?.csvPath || entry?.csv_path || "").trim();
  const dataset = csvPath ? (fileBasename(csvPath) || csvPath) : "—";
  const tf = String(detail?.timeframe || entry?.dataset_meta?.timeframe || "").trim();
  const hasValidation = (detail?.hasValidation != null) ? !!detail.hasValidation : !!entry?.has_validation;
  const reportPath = String(detail?.reportPath || entry?.report_xlsx_path || "").trim();
  const tags = Array.isArray(detail?.tags) ? detail.tags : (Array.isArray(entry?.tags) ? entry.tags : []);
  const sizeBytes = Number(entry?.bytes || 0);
  const size = formatBytesCompact(sizeBytes) || (sizeBytes > 0 ? `${sizeBytes} B` : "—");

  set("savedRunDetailTitle", ts);
  set("savedRunDetailMeta", `artifact: ${pathBase}`);
  set("savedRunDetailStrategy", strategyKind ? strategyKindLabel(strategyKind) : "—");
  set("savedRunDetailDataset", dataset);
  set("savedRunDetailTimeframe", tf || "—");
  set("savedRunDetailValidation", hasValidation ? "Available" : "Not saved");
  set("savedRunDetailReport", reportPath ? "XLSX available" : "Not exported");
  set("savedRunDetailSize", size);
  set("savedRunDetailTags", tags.length ? tags.join(", ") : "—");

  if (opts?.loading) {
    set("savedRunDetailParams", "Loading parameter snapshot...");
  } else {
    const paramsText = String(detail?.paramsText || "").trim();
    set("savedRunDetailParams", paramsText || "Request unavailable.");
  }
}

async function refreshSavedRunDetailCard(entry) {
  const seq = ++SAVED_RUN_DETAIL_SEQ;
  if (!entry) {
    renderSavedRunDetailCard(null, null, { loading: false });
    return;
  }

  const path = String(entry?.path || "").trim();
  if (!path) {
    renderSavedRunDetailCard(entry, null, { loading: false });
    return;
  }

  renderSavedRunDetailCard(entry, null, { loading: true });
  const cached = SAVED_RUN_DETAIL_CACHE[path];
  if (cached) {
    if (seq !== SAVED_RUN_DETAIL_SEQ) return;
    renderSavedRunDetailCard(entry, cached, { loading: false });
    return;
  }

  try {
    const artifact = await loadRunArtifactJson(path);
    const detail = buildSavedRunDetailFromArtifact(artifact);
    SAVED_RUN_DETAIL_CACHE[path] = detail;
    if (seq !== SAVED_RUN_DETAIL_SEQ) return;
    const current = selectedSavedRunEntry();
    if (!current || String(current?.path || "") !== path) return;
    renderSavedRunDetailCard(current, detail, { loading: false });
  } catch (e) {
    if (seq !== SAVED_RUN_DETAIL_SEQ) return;
    renderSavedRunDetailCard(entry, {
      paramsText: `Failed to load artifact:\n${String(e)}`,
    }, { loading: false });
  }
}

function syncSavedRunSelectionUi() {
  updateSavedRunsMeta();
  const tagsEl = $("savedRunsTags");
  const entry = selectedSavedRunEntry();
  const has = !!entry;
  const hasReport = !!String(entry?.report_xlsx_path || "").trim();
  if (tagsEl) {
    tagsEl.value = has && Array.isArray(entry.tags) ? entry.tags.join(", ") : "";
    tagsEl.disabled = RUNNING || !has;
  }

  const btnSaveTags = $("btnSaveSavedRunTags");
  const btnRefresh = $("btnRefreshSavedRuns");
  const btnLoad = $("btnLoadSavedRun");
  const btnApply = $("btnApplySavedRun");
  const btnRerun = $("btnRerunSavedRun");
  const btnOpenReport = $("btnOpenSavedRunReport");
  const btnReveal = $("btnRevealSavedRun");
  const btnDelete = $("btnDeleteSavedRun");
  const btnOpenDir = $("btnOpenRunsDir");

  if (btnSaveTags) btnSaveTags.disabled = RUNNING || !has;
  if (btnRefresh) btnRefresh.disabled = RUNNING;
  if (btnLoad) btnLoad.disabled = RUNNING || !has;
  if (btnApply) btnApply.disabled = RUNNING || !has;
  if (btnRerun) btnRerun.disabled = RUNNING || !has;
  if (btnOpenReport) btnOpenReport.disabled = RUNNING || !hasReport;
  if (btnReveal) btnReveal.disabled = RUNNING || !has;
  if (btnDelete) btnDelete.disabled = RUNNING || !has;
  if (btnOpenDir) btnOpenDir.disabled = RUNNING;

  const path = String(entry?.path || "").trim();
  if (!path) {
    CURRENT_SAVED_RUN_DETAIL_PATH = "";
    renderSavedRunDetailCard(null, null, { loading: false });
    return;
  }

  const cached = SAVED_RUN_DETAIL_CACHE[path];
  if (cached) renderSavedRunDetailCard(entry, cached, { loading: false });
  else renderSavedRunDetailCard(entry, null, { loading: true });

  if (!cached || path !== CURRENT_SAVED_RUN_DETAIL_PATH) {
    CURRENT_SAVED_RUN_DETAIL_PATH = path;
    refreshSavedRunDetailCard(entry);
  }
}

function updateSavedRunsMeta() {
  const meta = $("savedRunsMeta");
  if (!meta) return;
  const entry = selectedSavedRunEntry();
  if (!entry) {
    const filtered = filterSavedRuns(SAVED_RUNS, savedRunsQuery());
    meta.textContent = `saved runs: ${filtered.length}/${SAVED_RUNS.length}`;
    return;
  }
  const parts = [];
  if (entry.ts) parts.push(`ts=${entry.ts}`);
  if (entry.strategy_kind) parts.push(`strategy=${entry.strategy_kind}`);
  if (entry.has_validation) parts.push("validation=yes");
  if (entry.report_xlsx_path) parts.push("xlsx=yes");
  if (entry.csv_path) parts.push(`csv=${entry.csv_path}`);
  if (Array.isArray(entry.tags) && entry.tags.length) parts.push(`tags=${entry.tags.join(",")}`);
  if (entry.bytes != null) parts.push(`bytes=${entry.bytes}`);
  meta.textContent = parts.join(" • ");
}

function parseTagsInput(raw) {
  const s = String(raw || "");
  const parts = s
    .split(/[,;\n]+/g)
    .flatMap((p) => String(p || "").trim().split(/\s+/g))
    .map((p) => p.trim())
    .filter((p) => !!p);

  const seen = new Set();
  const out = [];
  for (const t of parts) {
    const k = t.toLowerCase();
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(t);
  }
  return out;
}

async function loadRunArtifactJson(path) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Missing run artifact path.");
  return await invoke("load_run_artifact", { path: p });
}

async function saveSavedRunTags(path, tags) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Select a saved run first.");

  await invoke("update_artifact_tags", { path: p, tags: Array.isArray(tags) ? tags : [] });
  await refreshSavedRuns(p);

  if (CURRENT_RUN?.artifact_path && String(CURRENT_RUN.artifact_path) === p) {
    CURRENT_RUN.tags = Array.isArray(tags) ? tags : [];
  }
  setStatus("Saved tags.", "ok");
}

async function saveMetaSavedTags(path, tags) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Select a saved meta run first.");

  await invoke("update_artifact_tags", { path: p, tags: Array.isArray(tags) ? tags : [] });
  await refreshMetaSaved(p);
  setStatus("Saved meta tags.", "ok");
}

async function loadSavedRun(path) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const p = String(path || "").trim();
  if (!p) throw new Error("Select a saved run first.");

  const artifact = await invoke("load_run_artifact", { path: p });
  const runReq = artifact?.run?.req || null;
  const runRes = artifact?.run?.res || null;
  const ts = String(artifact?.ts || "").trim() || new Date().toISOString();
  if (!runReq || !runRes) throw new Error("Saved run artifact is missing run.req/run.res.");

  CURRENT_RUN = { ts, req: runReq, res: runRes, artifact_path: p };
  if (artifact?.report_xlsx_path) CURRENT_RUN.report_xlsx_path = String(artifact.report_xlsx_path || "").trim();
  if (artifact?.data?.csv_path) CURRENT_RUN.csv_path = String(artifact.data.csv_path || "").trim();
  if (Array.isArray(artifact?.tags)) CURRENT_RUN.tags = artifact.tags.map((t) => String(t || "").trim()).filter((t) => !!t);

  clearValidationUi();
  $("output").textContent = pretty(runRes);
  renderRun(runRes, runReq);

  const valRes = artifact?.validation?.res || null;
  const valReq = artifact?.validation?.req || null;
  if (valRes) {
    CURRENT_VALIDATION = { ts, req: valReq, res: valRes };
    if ($("validationOutput")) $("validationOutput").textContent = pretty(valRes);
    if ($("validationMeta")) {
      const crossN = Array.isArray(valRes?.cross_ticker?.datasets) ? valRes.cross_ticker.datasets.length : 0;
      const crossSuffix = crossN ? ` • cross=${crossN}` : "";
      $("validationMeta").textContent = `validated ${valRes?.candidates?.length || 0} candidates in ${Number(valRes?.runtime_ms || 0)}ms (backend)${crossSuffix}`;
    }
    renderValidationTable(valRes);
    updateValidationExportButtons();
    if (CURRENT_RUN?.res && CURRENT_RUN?.req) renderBest(CURRENT_RUN.res, CURRENT_RUN.req);
  }

  updateSavedRunsMeta();
  updateValidateTabUi();
  updateQuickActionsUi();
  setStatus(`Loaded saved run ${ts}`, "muted");
}

async function applySavedRunToForm(path) {
  const artifact = await loadRunArtifactJson(path);
  const runReq = artifact?.run?.req || null;
  if (!runReq) throw new Error("Saved run artifact is missing run.req.");
  const csvPath = String(artifact?.data?.csv_path || "").trim();
  if (csvPath && $("csvPath")) $("csvPath").value = csvPath;
  applyRequestToUi(runReq);
  setStatus("Applied saved run settings to form.", "ok");
}

async function rerunSavedRun(path) {
  if (RUNNING) return;
  const artifact = await loadRunArtifactJson(path);
  const runReq = artifact?.run?.req || null;
  if (!runReq) throw new Error("Saved run artifact is missing run.req.");

  const csvPath = String(artifact?.data?.csv_path || "").trim();
  if (!csvPath) {
    throw new Error("Saved run is missing data.csv_path. Load a dataset first, then rerun.");
  }

  if ($("csvPath")) $("csvPath").value = csvPath;
  await loadCsv();
  const tf = String(artifact?.dataset_meta?.timeframe || "").trim();
  if (tf && $("dataTimeframe")) {
    $("dataTimeframe").value = tf;
    await applySelectedDataTimeframe();
  }
  applyRequestToUi(runReq);
  await runWorkflow();
}

async function openSelectedSavedRunReport() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const entry = selectedSavedRunEntry();
  const reportPath = String(entry?.report_xlsx_path || "").trim();
  if (!reportPath) {
    setStatus("No report path for this saved run yet (run validation + export XLSX).", "muted");
    return;
  }
  try {
    await invoke("open_path", { path: reportPath });
    setStatus("Opened report.", "ok");
  } catch {
    await invoke("reveal_path", { path: reportPath });
    setStatus("Revealed report.", "ok");
  }
}

function buildRunArtifactPayload(extra) {
  const ts = CURRENT_RUN?.ts || new Date().toISOString();
  const reportPath = String(extra?.report_xlsx_path || CURRENT_RUN?.report_xlsx_path || "").trim();
  const csvPath = String(extra?.csv_path || CURRENT_RUN?.csv_path || $("csvPath")?.value || "").trim();
  const tags = Array.isArray(CURRENT_RUN?.tags) ? CURRENT_RUN.tags : [];
  const datasetMeta = buildDatasetMetaForArtifact(csvPath);
  const config_fingerprint = computeConfigFingerprint(CURRENT_RUN?.req || null, CURRENT_VALIDATION?.req || null);
  return {
    version: 1,
    kind: "vectorbt_run_artifact",
    ts,
    config_fingerprint,
    run: { req: CURRENT_RUN?.req || null, res: CURRENT_RUN?.res || null },
    validation: CURRENT_VALIDATION ? { req: CURRENT_VALIDATION.req || null, res: CURRENT_VALIDATION.res || null } : null,
    report_xlsx_path: reportPath,
    data: { csv_path: csvPath },
    dataset_meta: datasetMeta,
    tags,
  };
}

async function persistCurrentRunArtifact(extra) {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (!CURRENT_RUN?.req || !CURRENT_RUN?.res) return;
  try {
    const artifact = buildRunArtifactPayload(extra);
    const resp = await invoke("save_run_artifact", { artifact });
    const path = String(resp?.path || "").trim();
    if (path) CURRENT_RUN.artifact_path = path;
    if (!BATCH_RUNNING) await refreshSavedRuns(path);
  } catch {}
}

function addHistoryEntry(req, res) {
  const history = getHistory();
  const ts = new Date().toISOString();
  const backend = req?.backend?.backend || "CpuOnly";
  const pairs = Number(res?.num_combos || res?.total_evaluations || 0);
  history.unshift({ ts, backend, pairs, req, res });
  while (history.length > 20) history.pop();
  setHistory(history);
  populateHistorySelect(history, ts);
  populateCompareSelect(history, $("compareSelect")?.value || "");
  const tags = Array.isArray(NEXT_RUN_TAGS) ? NEXT_RUN_TAGS : [];
  NEXT_RUN_TAGS = null;
  CURRENT_RUN = { ts, req, res, tags, csv_path: String($("csvPath")?.value || "").trim() };
  clearValidationUi();
}

function scoreOf(metrics, objective) {
  const m = metrics || {};
  const obj = String(objective || "Sharpe");
  if (obj === "Pnl") return Number(m.pnl || 0);
  if (obj === "Sharpe") return Number(m.sharpe || 0);
  if (obj === "Sortino") return Number(m.sortino || 0);
  if (obj === "Calmar") return Number(m.calmar || 0);
  if (obj === "Omega") return Number(m.omega || 0);
  if (obj === "Volatility") return -Number(m.volatility || 0);
  if (obj === "MaxDrawdown") return -Number(m.max_dd || 0);
  return Number(m.sharpe || 0);
}

function optNum(s) {
  const t = String(s ?? "").trim();
  if (!t) return NaN;
  const x = Number(t);
  return Number.isFinite(x) ? x : NaN;
}

function readTopFiltersFromUi() {
  const f = {
    minPnl: optNum($("filterMinPnl")?.value),
    minSharpe: optNum($("filterMinSharpe")?.value),
    maxDd: optNum($("filterMaxDd")?.value),
    minTrades: optNum($("filterMinTrades")?.value),
    minExposure: optNum($("filterMinExposure")?.value),
  };
  if (!Number.isFinite(f.minPnl)) delete f.minPnl;
  if (!Number.isFinite(f.minSharpe)) delete f.minSharpe;
  if (!Number.isFinite(f.maxDd)) delete f.maxDd;
  if (!Number.isFinite(f.minTrades)) delete f.minTrades;
  if (!Number.isFinite(f.minExposure)) delete f.minExposure;
  return f;
}

function applyTopFilters(rows, filters) {
  const f = filters || {};
  return rows.filter((row) => {
    const m = row.metrics || {};
    const pnl = Number(m.pnl || 0);
    const sharpe = Number(m.sharpe || 0);
    const dd = Number(m.max_dd || 0);
    const trades = Number(m.trades || 0);
    const expo = Number(m.exposure || 0);

    if (f.minPnl != null && pnl < f.minPnl) return false;
    if (f.minSharpe != null && sharpe < f.minSharpe) return false;
    if (f.maxDd != null && dd > f.maxDd) return false;
    if (f.minTrades != null && trades < f.minTrades) return false;
    if (f.minExposure != null && expo < f.minExposure) return false;
    return true;
  });
}

function collectTopRows(res) {
  const out = [];
  const pareto = Array.isArray(res?.pareto) ? res.pareto : null;
  if (pareto) {
    const pts = pareto.map((p) => ({
      params: (p && typeof p === "object" ? p.params : null) || {},
      metrics: (p && typeof p === "object" ? p.metrics : null) || {},
      maximize_value: Number(p?.maximize_value ?? NaN),
      minimize_value: Number(p?.minimize_value ?? NaN),
      violation: Number(p?.violation ?? 0),
    }));
    pts.sort((a, b) => {
      const aFeas = a.violation <= 0;
      const bFeas = b.violation <= 0;
      if (aFeas !== bFeas) return aFeas ? -1 : 1;
      if (a.maximize_value !== b.maximize_value) return b.maximize_value - a.maximize_value;
      if (a.minimize_value !== b.minimize_value) return a.minimize_value - b.minimize_value;
      return 0;
    });
    for (let i = 0; i < pts.length; i++) {
      const p = pts[i];
      out.push({
        rank: i + 1,
        params: p.params,
        metrics: p.metrics,
        mo: { maximize_value: p.maximize_value, minimize_value: p.minimize_value, violation: p.violation },
      });
    }
    return out;
  }
  const top = Array.isArray(res?.top) ? res.top : [];
  for (let i = 0; i < top.length; i++) {
    const [params, metrics] = top[i] || [];
    out.push({ rank: i + 1, params: params || {}, metrics: metrics || {} });
  }
  if (!out.length && res?.best_params && res?.best_metrics) {
    out.push({ rank: 1, params: res.best_params, metrics: res.best_metrics });
  }
  return out;
}

function sortRows(rows, key, dir, objective) {
  const d = dir === "desc" ? -1 : 1;
  const k = String(key || "rank");
  const obj = String(objective || "Sharpe");
  const cmpNum = (a, b) => (a === b ? 0 : a < b ? -1 : 1);

  return [...rows].sort((ra, rb) => {
    const pa = ra.params || {};
    const pb = rb.params || {};
    const ma = ra.metrics || {};
    const mb = rb.metrics || {};
    const moa = ra.mo || {};
    const mob = rb.mo || {};

    if (k === "rank") return d * cmpNum(ra.rank, rb.rank);
    if (k === "fast") {
      const sa = `${pa.fast_ma_type || ""}`.toLowerCase();
      const sb = `${pb.fast_ma_type || ""}`.toLowerCase();
      if (sa !== sb) return d * cmpNum(sa, sb);
      return d * cmpNum(Number(pa.fast_len || 0), Number(pb.fast_len || 0));
    }
    if (k === "slow") {
      const sa = `${pa.slow_ma_type || ""}`.toLowerCase();
      const sb = `${pb.slow_ma_type || ""}`.toLowerCase();
      if (sa !== sb) return d * cmpNum(sa, sb);
      return d * cmpNum(Number(pa.slow_len || 0), Number(pb.slow_len || 0));
    }
    if (k === "pnl" || k === "ret_pct") return d * cmpNum(Number(ma.pnl || 0), Number(mb.pnl || 0));
    if (k === "sharpe") return d * cmpNum(Number(ma.sharpe || 0), Number(mb.sharpe || 0));
    if (k === "max_dd") return d * cmpNum(Number(ma.max_dd || 0), Number(mb.max_dd || 0));
    if (k === "trades") return d * cmpNum(Number(ma.trades || 0), Number(mb.trades || 0));
    if (k === "exposure") return d * cmpNum(Number(ma.exposure || 0), Number(mb.exposure || 0));
    if (k === "maximize_value") return d * cmpNum(Number(moa.maximize_value ?? -Infinity), Number(mob.maximize_value ?? -Infinity));
    if (k === "minimize_value") return d * cmpNum(Number(moa.minimize_value ?? Infinity), Number(mob.minimize_value ?? Infinity));
    if (k === "violation") return d * cmpNum(Number(moa.violation ?? 0), Number(mob.violation ?? 0));
    if (k === "score") return d * cmpNum(scoreOf(ma, obj), scoreOf(mb, obj));
    return d * cmpNum(ra.rank, rb.rank);
  });
}

function paretoMetricValue(metrics, key) {
  const m = metrics || {};
  const k = String(key || "");
  if (k === "pnl") return Number(m.pnl || 0);
  if (k === "sharpe") return Number(m.sharpe || 0);
  if (k === "max_dd") return Number(m.max_dd || 0);
  if (k === "trades") return Number(m.trades || 0);
  if (k === "exposure") return Number(m.exposure || 0);
  return 0;
}

function paretoSign(key) {
  // Convert "lower is better" metrics into a "higher is better" score for Pareto dominance checks.
  const k = String(key || "");
  if (k === "max_dd") return -1;
  return 1;
}

function computePareto(rows, xKey, yKey) {
  const sx = paretoSign(xKey);
  const sy = paretoSign(yKey);
  const pts = rows.map((row) => {
    const m = row.metrics || {};
    const x = paretoMetricValue(m, xKey);
    const y = paretoMetricValue(m, yKey);
    return { row, x, y, tx: sx * x, ty: sy * y };
  });

  const front = [];
  for (let i = 0; i < pts.length; i++) {
    let dominated = false;
    for (let j = 0; j < pts.length; j++) {
      if (i === j) continue;
      const a = pts[j];
      const b = pts[i];
      if (a.tx >= b.tx && a.ty >= b.ty && (a.tx > b.tx || a.ty > b.ty)) {
        dominated = true;
        break;
      }
    }
    if (!dominated) front.push(pts[i]);
  }
  return { points: pts, front };
}

function renderPareto(rows) {
  const wrap = $("paretoWrap");
  const canvas = $("paretoCanvas");
  const meta = $("paretoMeta");
  if (!wrap || !canvas || !meta) return;
  const hint = $("paretoEmptyHint");

  const ptsRows = Array.isArray(rows) ? rows : [];
  if (!ptsRows.length) {
    wrap.style.display = "none";
    PARETO_POINTS_RENDERED = [];
    if (hint) {
      hint.textContent = CURRENT_RUN?.res
        ? "No Pareto points to render (try loosening Top filters)."
        : "Run an optimization to generate Pareto points.";
    }
    setHintVisible("paretoEmptyHint", true);
    return;
  }

  setHintVisible("paretoEmptyHint", false);
  wrap.style.display = "block";
  const xKey = $("paretoX")?.value || "max_dd";
  const yKey = $("paretoY")?.value || "pnl";
  const { points, front } = computePareto(ptsRows, xKey, yKey);

  meta.textContent = `points=${points.length} pareto=${front.length}`;

  const w = canvas.width || 1024;
  const h = canvas.height || 240;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, w, h);

  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
  for (const p of points) {
    if (Number.isFinite(p.x)) { if (p.x < minX) minX = p.x; if (p.x > maxX) maxX = p.x; }
    if (Number.isFinite(p.y)) { if (p.y < minY) minY = p.y; if (p.y > maxY) maxY = p.y; }
  }
  if (!Number.isFinite(minX) || !Number.isFinite(maxX) || !Number.isFinite(minY) || !Number.isFinite(maxY)) {
    PARETO_POINTS_RENDERED = [];
    return;
  }
  if (minX === maxX) { minX -= 1; maxX += 1; }
  if (minY === maxY) { minY -= 1; maxY += 1; }

  const padL = 36, padR = 10, padT = 10, padB = 22;
  const plotW = Math.max(1, w - padL - padR);
  const plotH = Math.max(1, h - padT - padB);
  const xOf = (x) => padL + ((x - minX) / (maxX - minX)) * plotW;
  const yOf = (y) => padT + (1 - (y - minY) / (maxY - minY)) * plotH;

  // Axes
  ctx.strokeStyle = "rgba(148, 163, 184, 0.35)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(padL + 0.5, padT + 0.5);
  ctx.lineTo(padL + 0.5, padT + plotH + 0.5);
  ctx.lineTo(padL + plotW + 0.5, padT + plotH + 0.5);
  ctx.stroke();

  // Points
  const frontKeys = new Set(front.map((p) => paramsKey(p.row.params)));
  PARETO_POINTS_RENDERED = [];

  for (const p of points) {
    const sx = xOf(p.x);
    const sy = yOf(p.y);
    const key = paramsKey(p.row.params);
    const isFront = frontKeys.has(key);
    ctx.fillStyle = isFront ? "rgba(56, 189, 248, 0.95)" : "rgba(148, 163, 184, 0.55)";
    const r = isFront ? 3 : 2;
    ctx.beginPath();
    ctx.arc(sx, sy, r, 0, Math.PI * 2);
    ctx.fill();
    PARETO_POINTS_RENDERED.push({ sx, sy, row: p.row, key });
  }

  // Selected highlight
  if (DRILL_SELECTED_KEY) {
    const hit = PARETO_POINTS_RENDERED.find((p) => p.key === DRILL_SELECTED_KEY);
    if (hit) {
      ctx.strokeStyle = "rgba(52, 211, 153, 0.95)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(hit.sx, hit.sy, 6, 0, Math.PI * 2);
      ctx.stroke();
    }
  }

  canvas.onclick = (ev) => {
    const rect = canvas.getBoundingClientRect();
    const cx = ((ev.clientX - rect.left) / rect.width) * w;
    const cy = ((ev.clientY - rect.top) / rect.height) * h;
    let best = null;
    let bestD2 = 1e18;
    for (const p of PARETO_POINTS_RENDERED) {
      const dx = p.sx - cx;
      const dy = p.sy - cy;
      const d2 = dx * dx + dy * dy;
      if (d2 < bestD2) { bestD2 = d2; best = p; }
    }
    if (best && bestD2 <= 12 * 12) {
      DRILL_SELECTED_KEY = best.key;
      syncHeatmapSelectionToDrillKey();
      requestDrilldown(best.row.params, undefined, {
        source: "pareto",
        row: best.row,
        metrics: best.row?.metrics || null,
      });
      renderTopTable(CURRENT_RUN?.res, CURRENT_RUN?.req);
      RESULTS_TABS?.set("drill", { save: false, focus: true });
    }
  };
}

function updateTopTableHeader(isMo, req) {
  const tr = document.querySelector("#topTable thead tr");
  if (!tr) return;

  const mode = isMo ? "mo" : "base";
  if (tr.getAttribute("data-mode") === mode) return;
  tr.setAttribute("data-mode", mode);

  if (!isMo) {
    tr.innerHTML = `
      <th data-key="rank" class="left">#</th>
      <th data-key="fast" class="left">Fast</th>
      <th data-key="slow" class="left">Slow</th>
      <th data-key="pnl">PnL</th>
      <th data-key="ret_pct">Ret %</th>
      <th data-key="sharpe">Sharpe</th>
      <th data-key="max_dd">MaxDD</th>
      <th data-key="trades">Trades</th>
      <th data-key="exposure">Expo</th>
      <th data-key="score">Score</th>
    `;
    return;
  }

  const maxObj = String(req?.objectives?.maximize || req?.objective || "maximize");
  const minObj = String(req?.objectives?.minimize || "?");
  tr.innerHTML = `
    <th data-key="rank" class="left">#</th>
    <th data-key="fast" class="left">Fast</th>
    <th data-key="slow" class="left">Slow</th>
    <th data-key="pnl">PnL</th>
    <th data-key="ret_pct">Ret %</th>
    <th data-key="sharpe">Sharpe</th>
    <th data-key="max_dd">MaxDD</th>
    <th data-key="trades">Trades</th>
    <th data-key="exposure">Expo</th>
    <th data-key="maximize_value">Max(${maxObj})</th>
    <th data-key="minimize_value">Min(${minObj})</th>
    <th data-key="violation">Viol</th>
  `;
}

function topTablePreviewReq() {
  const kind = strategyKind();
  if (kind === "CompositeMo") {
    const maximize = String($("objective")?.value || "Sharpe").trim() || "Sharpe";
    const minimize = String($("moObjectiveMin")?.value || "MaxDrawdown").trim() || "MaxDrawdown";
    return { objectives: { maximize, minimize }, objective: maximize };
  }
  const objective = String($("objective")?.value || "Sharpe").trim() || "Sharpe";
  return { objective };
}

function renderTopTablePlaceholder(text) {
  const tbody = $("topTableBody");
  if (!tbody) return;
  tbody.innerHTML = "";

  const kind = strategyKind();
  const isMo = kind === "CompositeMo";
  updateTopTableHeader(isMo, topTablePreviewReq());
  updateTopSortUi();

  const cols = isMo ? 12 : 10;
  const tr = document.createElement("tr");
  const td = document.createElement("td");
  td.colSpan = cols;
  td.className = "left muted";
  td.textContent = String(text || "").trim() || "—";
  tr.appendChild(td);
  tbody.appendChild(tr);
}

function renderTopTableSkeleton(opts) {
  const tbody = $("topTableBody");
  if (!tbody) return;
  tbody.innerHTML = "";

  const kind = strategyKind();
  const isMo = kind === "CompositeMo";
  updateTopTableHeader(isMo, topTablePreviewReq());
  updateTopSortUi();

  const cols = isMo ? 12 : 10;
  const rows = Math.max(4, Math.min(14, Math.floor(Number(opts?.rows ?? 8))));

  for (let r = 0; r < rows; r++) {
    const tr = document.createElement("tr");
    tr.className = "skeletonRow";
    for (let c = 0; c < cols; c++) {
      const td = document.createElement("td");
      if (c <= 2) td.className = "left";
      const line = document.createElement("div");
      line.className = "skeleton skeletonLine";
      const w = 40 + ((r * 13 + c * 29) % 55);
      line.style.width = `${w}%`;
      td.appendChild(line);
      tr.appendChild(td);
    }
    tbody.appendChild(tr);
  }
}

function updateTopTableNoRunUi() {
  const guard = $("topSelectionGuardrail");
  if (guard) {
    guard.style.display = "none";
    guard.textContent = "";
    guard.className = "muted badText";
  }
  if (CURRENT_RUN?.res) return;
  if (RUNNING) renderTopTableSkeleton({ rows: 9 });
  else renderTopTablePlaceholder("Run an optimization to populate Top results.");
}

function setTopSelectionGuardrail(msg, kind) {
  const el = $("topSelectionGuardrail");
  if (!el) return;
  const text = String(msg || "").trim();
  if (!text) {
    el.style.display = "none";
    el.textContent = "";
    el.className = "muted badText";
    return;
  }
  el.style.display = "block";
  el.textContent = text;
  const k = String(kind || "").trim();
  el.className = k === "ok" ? "muted" : "muted badText";
}

function updateTopSelectionGuardrail(req) {
  if (!CURRENT_RUN?.res) {
    setTopSelectionGuardrail("", "");
    return;
  }
  const hasRows = Array.isArray(TOP_ROWS_FILTERED) && TOP_ROWS_FILTERED.length > 0;
  if (!hasRows) {
    setTopSelectionGuardrail("", "");
    return;
  }

  const valRes = CURRENT_VALIDATION?.res || null;
  if (!valRes) {
    setTopSelectionGuardrail(
      "Selection-bias guardrail: these are optimization-only winners. Run Validation before treating them as robust.",
      "danger",
    );
    return;
  }

  const runKind = String(req?.strategy_kind || req?.strategyKind || "").trim() || "DoubleMa";
  setTopSelectionGuardrail(
    `Validation context loaded for ${runKind}. Final selection should still include holdout/meta confirmation.`,
    "ok",
  );
}

function updateTopSortUi() {
  const ths = document.querySelectorAll("#topTable thead th[data-key]");
  for (const th of ths) {
    const key = String(th.getAttribute("data-key") || "").trim();
    if (key && key === TOP_SORT.key) {
      th.setAttribute("data-sort", TOP_SORT.dir);
      th.setAttribute("aria-sort", TOP_SORT.dir === "asc" ? "ascending" : "descending");
    } else {
      th.removeAttribute("data-sort");
      th.removeAttribute("aria-sort");
    }
  }
}

function formatMaParamsInline(m, maxKeys) {
  const obj = m && typeof m === "object" ? m : null;
  if (!obj) return "";
  const keys = Object.keys(obj).sort();
  if (!keys.length) return "";
  const lim = Math.max(1, Math.floor(Number(maxKeys || 0) || 3));
  const parts = [];
  for (const k of keys.slice(0, lim)) {
    parts.push(`${k}=${fmt(obj[k], 4)}`);
  }
  const more = keys.length > lim ? ",…" : "";
  return ` {${parts.join(",")}${more}}`;
}

function renderTopTable(res, req) {
  const kindRaw = String(req?.strategy_kind || req?.strategyKind || "DoubleMa");
  const isMo = kindRaw === "CompositeMo" && Array.isArray(res?.pareto);
  updateTopTableHeader(isMo, req);
  updateTopSortUi();

  TOP_ROWS = collectTopRows(res);
  const obj = req?.objective || "Sharpe";
  const sorted = sortRows(TOP_ROWS, TOP_SORT.key, TOP_SORT.dir, obj);
  const filters = readTopFiltersFromUi();
  TOP_ROWS_FILTERED = applyTopFilters(sorted, filters);
  updateTopSelectionGuardrail(req);

  const meta = $("topFilterMeta");
  if (meta) {
    meta.textContent = `showing ${TOP_ROWS_FILTERED.length}/${sorted.length} rows`;
  }

  const tbody = $("topTableBody");
  if (!tbody) return;
  tbody.innerHTML = "";

  if (!TOP_ROWS_FILTERED.length) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = isMo ? 12 : 10;
    td.className = "left muted";
    td.textContent = sorted.length ? "No rows match the current filters." : "No rows.";
    tr.appendChild(td);
    tbody.appendChild(tr);
    renderPareto([]);
    return;
  }

  for (const row of TOP_ROWS_FILTERED) {
    const p = row.params || {};
    const m = row.metrics || {};
    const mo = row.mo || {};
    const key = paramsKey(p);
    const tr = document.createElement("tr");

    const tdRank = document.createElement("td");
    tdRank.textContent = String(row.rank);
    tdRank.className = "left mono";

    const tdFast = document.createElement("td");
    const fastExtra = formatMaParamsInline(p.fast_ma_params, 2);
    tdFast.textContent = `${p.fast_ma_type || "?"}(${p.fast_len || "?"})${fastExtra}`;
    tdFast.className = "left mono";
    if (fastExtra) tdFast.title = `Fast MA params${formatMaParamsInline(p.fast_ma_params, 99)}`;

    const tdSlow = document.createElement("td");
    const slowExtra = formatMaParamsInline(p.slow_ma_params, 2);
    tdSlow.textContent = `${p.slow_ma_type || "?"}(${p.slow_len || "?"})${slowExtra}`;
    tdSlow.className = "left mono";
    if (slowExtra) tdSlow.title = `Slow MA params${formatMaParamsInline(p.slow_ma_params, 99)}`;

    const tdPnl = document.createElement("td");
    tdPnl.textContent = fmt(m.pnl, 6);
    tdPnl.className = "mono";
    const pnl = Number(m.pnl ?? NaN);
    if (Number.isFinite(pnl)) {
      if (pnl > 0) tdPnl.classList.add("goodText");
      else if (pnl < 0) tdPnl.classList.add("badText");
    }

    const tdRetPct = document.createElement("td");
    tdRetPct.textContent = fmtPct(m.pnl, 2);
    tdRetPct.className = "mono";
    if (Number.isFinite(pnl)) {
      if (pnl > 0) tdRetPct.classList.add("goodText");
      else if (pnl < 0) tdRetPct.classList.add("badText");
    }

    const tdSharpe = document.createElement("td");
    tdSharpe.textContent = fmt(m.sharpe, 6);
    tdSharpe.className = "mono";
    const sharpe = Number(m.sharpe ?? NaN);
    if (Number.isFinite(sharpe)) {
      if (sharpe > 0) tdSharpe.classList.add("goodText");
      else if (sharpe < 0) tdSharpe.classList.add("badText");
    }

    const tdDd = document.createElement("td");
    tdDd.textContent = fmt(m.max_dd, 6);
    tdDd.className = "mono";

    const tdTrades = document.createElement("td");
    tdTrades.textContent = String(Number(m.trades || 0));
    tdTrades.className = "mono";

    const tdExpo = document.createElement("td");
    tdExpo.textContent = fmt(m.exposure, 4);
    tdExpo.className = "mono";

    const tdScore = document.createElement("td");
    if (!isMo) {
      tdScore.textContent = fmt(scoreOf(m, obj), 6);
      tdScore.className = "mono";
    }

    tr.appendChild(tdRank);
    tr.appendChild(tdFast);
    tr.appendChild(tdSlow);
    tr.appendChild(tdPnl);
    tr.appendChild(tdRetPct);
    tr.appendChild(tdSharpe);
    tr.appendChild(tdDd);
    tr.appendChild(tdTrades);
    tr.appendChild(tdExpo);
    if (!isMo) tr.appendChild(tdScore);

    if (isMo) {
      const tdMax = document.createElement("td");
      tdMax.textContent = fmt(mo.maximize_value, 6);
      tdMax.className = "mono";

      const tdMin = document.createElement("td");
      tdMin.textContent = fmt(mo.minimize_value, 6);
      tdMin.className = "mono";

      const tdViol = document.createElement("td");
      tdViol.textContent = fmt(mo.violation, 6);
      tdViol.className = "mono";

      tr.appendChild(tdMax);
      tr.appendChild(tdMin);
      tr.appendChild(tdViol);
    }

    if (key && key === DRILL_SELECTED_KEY) tr.classList.add("selected");
    tr.addEventListener("click", () => {
      for (const el of tbody.querySelectorAll("tr.selected")) el.classList.remove("selected");
      tr.classList.add("selected");
      DRILL_SELECTED_KEY = key;
      syncHeatmapSelectionToDrillKey();
      requestDrilldown(p, undefined, {
        source: "top",
        row,
        metrics: m,
      });
      RESULTS_TABS?.set("drill", { save: false, focus: true });
    });
    tbody.appendChild(tr);
  }

  renderPareto(TOP_ROWS_FILTERED);
}

function renderCompare() {
  const el = $("compareSummary");
  if (!el) return;

  const sel = String($("compareSelect")?.value || "").trim();
  if (!sel) {
    el.textContent = "-";
    return;
  }
  if (!CURRENT_RUN?.res) {
    el.textContent = "Select a current run to compare.";
    return;
  }

  COMPARE_SEQ = (COMPARE_SEQ + 1) >>> 0;
  const seq = COMPARE_SEQ;
  el.textContent = "Loading compare…";

  const stableStringify = (v) => {
    if (v == null) return "null";
    if (typeof v !== "object") {
      try { return JSON.stringify(v); } catch { return String(v); }
    }
    if (Array.isArray(v)) return `[${v.map(stableStringify).join(",")}]`;
    const keys = Object.keys(v).sort();
    return `{${keys.map((k) => `${JSON.stringify(k)}:${stableStringify(v[k])}`).join(",")}}`;
  };

  const truncate = (s, n) => {
    const t = String(s || "");
    const lim = Math.max(8, Math.floor(Number(n || 0) || 120));
    return t.length > lim ? `${t.slice(0, lim - 1)}…` : t;
  };

  const diffParams = (a, b, limit) => {
    const aa = a && typeof a === "object" ? a : {};
    const bb = b && typeof b === "object" ? b : {};
    const keys = Array.from(new Set([...Object.keys(aa), ...Object.keys(bb)])).sort();
    const out = [];
    for (const k of keys) {
      const av = stableStringify(aa[k]);
      const bv = stableStringify(bb[k]);
      if (av === bv) continue;
      out.push({ key: k, a: av, b: bv });
      if (out.length >= (limit || 12)) break;
    }
    return out;
  };

  const pickOptimizeBest = (res, req) => {
    const kind = String(req?.strategy_kind || req?.strategyKind || "DoubleMa");
    if (kind === "CompositeMo" && Array.isArray(res?.pareto) && res.pareto.length) {
      const pts = res.pareto;
      const best = [...pts].sort((a, b) => {
        const av = Number(a?.violation ?? 0);
        const bv = Number(b?.violation ?? 0);
        const aFeas = av <= 0;
        const bFeas = bv <= 0;
        if (aFeas !== bFeas) return aFeas ? -1 : 1;
        const aMax = Number(a?.maximize_value ?? -Infinity);
        const bMax = Number(b?.maximize_value ?? -Infinity);
        if (aMax !== bMax) return bMax - aMax;
        const aMin = Number(a?.minimize_value ?? Infinity);
        const bMin = Number(b?.minimize_value ?? Infinity);
        if (aMin !== bMin) return aMin - bMin;
        return 0;
      })[0];
      return { metrics: best?.metrics || {}, params: best?.params || {} };
    }
    return { metrics: res?.best_metrics || {}, params: res?.best_params || {} };
  };

  const pickRecommended = (valRes) => {
    if (!valRes) return null;
    const { rows } = computeValidationShortlist(valRes);
    return rows && rows.length ? (rows[0] || null) : null;
  };

  const resolveBaseline = async (value) => {
    const v = String(value || "").trim();
    if (!v) return null;
    const history = getHistory();
    if (v.startsWith("disk:")) {
      const path = v.slice(5);
      const cached = COMPARE_BASELINE_CACHE[path];
      const art = cached || (await loadRunArtifactJson(path));
      if (!cached) COMPARE_BASELINE_CACHE[path] = art;
      return {
        source: "disk",
        ts: String(art?.ts || "").trim(),
        req: art?.run?.req || null,
        res: art?.run?.res || null,
        validation: art?.validation?.res || null,
        report_xlsx_path: String(art?.report_xlsx_path || "").trim(),
        csv_path: String(art?.data?.csv_path || "").trim(),
        tags: Array.isArray(art?.tags) ? art.tags : [],
      };
    }
    const ts = v.startsWith("hist:") ? v.slice(5) : v;
    const h = history.find((x) => String(x?.ts || "") === ts) || null;
    if (!h) return null;
    return { source: "hist", ts: h.ts, req: h.req || null, res: h.res || null, validation: null, report_xlsx_path: "", csv_path: "", tags: [] };
  };

  (async () => {
    const base = await resolveBaseline(sel);
    const cur = CURRENT_RUN;
    const curVal = (CURRENT_VALIDATION && CURRENT_VALIDATION.ts === cur.ts) ? CURRENT_VALIDATION.res : null;

    if (seq !== COMPARE_SEQ) return;
    if (!base?.res || !base?.req) {
      el.textContent = "Baseline run not found (refresh saved runs or pick another baseline).";
      return;
    }

    const bReq = base.req || {};
    const bRes = base.res || {};
    const cReq = cur.req || {};
    const cRes = cur.res || {};

    const bBest = pickOptimizeBest(bRes, bReq);
    const cBest = pickOptimizeBest(cRes, cReq);

    const bm = bBest.metrics || {};
    const cm = cBest.metrics || {};
    const bp = bBest.params || {};
    const cp = cBest.params || {};

    const bObj = bReq.objective || (bReq?.objectives?.maximize || "Sharpe");
    const cObj = cReq.objective || (cReq?.objectives?.maximize || "Sharpe");
    const bScore = scoreOf(bm, bObj);
    const cScore = scoreOf(cm, cObj);

    const bRt = Number(bRes.runtime_ms || 0);
    const cRt = Number(cRes.runtime_ms || 0);
    const rtRatio = bRt > 0 && cRt > 0 ? (bRt / cRt) : 0;

    const delta = (a, b) => Number(a || 0) - Number(b || 0);

    const lines = [];
    const bKind = kindFromReq(bReq);
    const cKind = kindFromReq(cReq);
    const bCombos = Number(bRes.num_combos || bRes.total_evaluations || 0);
    const cCombos = Number(cRes.num_combos || cRes.total_evaluations || 0);
    lines.push(`baseline (${base.source}): ${base.ts} | strategy=${bKind} | combos=${bCombos} | runtime_ms=${bRt || 0}`);
    lines.push(`current:          ${cur.ts} | strategy=${cKind} | combos=${cCombos} | runtime_ms=${cRt || 0}${rtRatio ? ` | speedup=${rtRatio.toFixed(2)}x` : ""}`);
    if (bKind !== cKind) lines.push(`note: strategies differ (baseline=${bKind}, current=${cKind})`);
    lines.push("");

    lines.push("Optimization best:");
    lines.push(`  baseline: pnl=${fmt(bm.pnl, 6)} ret=${fmtPct(bm.pnl, 2)} sharpe=${fmt(bm.sharpe, 6)} dd=${fmt(bm.max_dd, 6)} trades=${Number(bm.trades || 0)} expo=${fmt(bm.exposure, 4)} score(${bObj})=${fmt(bScore, 6)}`);
    lines.push(`  current:  pnl=${fmt(cm.pnl, 6)} ret=${fmtPct(cm.pnl, 2)} sharpe=${fmt(cm.sharpe, 6)} dd=${fmt(cm.max_dd, 6)} trades=${Number(cm.trades || 0)} expo=${fmt(cm.exposure, 4)} score(${cObj})=${fmt(cScore, 6)}`);
    lines.push(`  deltas:   pnl=${fmt(delta(cm.pnl, bm.pnl), 6)} sharpe=${fmt(delta(cm.sharpe, bm.sharpe), 6)} dd=${fmt(delta(cm.max_dd, bm.max_dd), 6)} trades=${delta(cm.trades, bm.trades)} expo=${fmt(delta(cm.exposure, bm.exposure), 4)}`);
    if (bObj === cObj) lines.push(`  scoreΔ:   ${fmt(delta(cScore, bScore), 6)} (${cObj})`);
    lines.push("");

    const bRec = pickRecommended(base.validation);
    const cRec = pickRecommended(curVal);
    if (bRec || cRec) {
      lines.push("Validation recommended:");
      const fmtU = (x, d) => fmt(x, Number.isFinite(d) ? d : 4);
      const fmtPct = (x, d) => {
        const n = Number(x);
        if (!Number.isFinite(n)) return "-";
        return (n * 100).toFixed(Number.isFinite(d) ? d : 1) + "%";
      };
      const wfLine = (wf) => {
        if (!wf || typeof wf !== "object") return "";
        return `           wf: win=${Number(wf.windows || 0)} ` +
          `sh_med=${fmtU(wf.sharpe_median, 4)} sh_worst=${fmtU(wf.sharpe_worst, 4)} ` +
          `dd_worst=${fmtU(wf.max_dd_worst, 4)} pnl_worst=${fmtU(wf.pnl_worst, 6)}`;
      };
      const mcLine = (mc) => {
        if (!mc || typeof mc !== "object") return "";
        return `           mc: p+=${fmtPct(mc.prob_pnl_positive, 1)} pnl_p05=${fmtU(mc.pnl_p05, 6)} dd_p95=${fmtU(mc.max_dd_p95, 6)} sims=${Number(mc.simulations || 0)}`;
      };
      const perturbLine = (pt) => {
        if (!pt || typeof pt !== "object") return "";
        return `           perturb: sh_worst=${fmtU(pt.sharpe_worst, 4)} dd_worst=${fmtU(pt.max_dd_worst, 4)} pnl_worst=${fmtU(pt.pnl_worst, 6)} iters=${Number(pt.iterations || 0)}`;
      };
      const crossLine = (cross) => {
        if (!cross || typeof cross !== "object") return "";
        return `           cross: ds=${Number(cross.datasets || 0)} sh_worst=${fmtU(cross.sharpe_worst, 4)} dd_worst=${fmtU(cross.max_dd_worst, 4)} pnl_worst=${fmtU(cross.pnl_worst, 6)}`;
      };
      if (bRec) {
        const p = bRec.params || {};
        const m = bRec.in_sample || {};
        const cross = bRec.cross_ticker && typeof bRec.cross_ticker === "object" ? bRec.cross_ticker : null;
        lines.push(`  baseline: robust=${fmt(bRec.robust_score, 6)} ${bRec.robust_pareto ? "(robust pareto)" : ""}${cross ? ` | cross=${fmt(cross.score, 6)} ${bRec.cross_ticker_pareto ? "(cross pareto)" : ""}` : ""}`.trim());
        lines.push(`           in_sample: pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} | params=${truncate(stableStringify(p), 140)}`);
        const l1 = wfLine(bRec.walk_forward);
        const l2 = mcLine(bRec.trade_bootstrap);
        const l3 = perturbLine(bRec.perturb);
        const l4 = crossLine(cross);
        if (l1) lines.push(l1);
        if (l2) lines.push(l2);
        if (l3) lines.push(l3);
        if (l4) lines.push(l4);
      } else {
        lines.push("  baseline: (no validation saved)");
      }
      if (cRec) {
        const p = cRec.params || {};
        const m = cRec.in_sample || {};
        const cross = cRec.cross_ticker && typeof cRec.cross_ticker === "object" ? cRec.cross_ticker : null;
        lines.push(`  current:  robust=${fmt(cRec.robust_score, 6)} ${cRec.robust_pareto ? "(robust pareto)" : ""}${cross ? ` | cross=${fmt(cross.score, 6)} ${cRec.cross_ticker_pareto ? "(cross pareto)" : ""}` : ""}`.trim());
        lines.push(`           in_sample: pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} | params=${truncate(stableStringify(p), 140)}`);
        const l1 = wfLine(cRec.walk_forward);
        const l2 = mcLine(cRec.trade_bootstrap);
        const l3 = perturbLine(cRec.perturb);
        const l4 = crossLine(cross);
        if (l1) lines.push(l1);
        if (l2) lines.push(l2);
        if (l3) lines.push(l3);
        if (l4) lines.push(l4);
      } else {
        lines.push("  current:  (no validation loaded)");
      }
      if (bRec && cRec) {
        const bm = bRec.in_sample || {};
        const cm = cRec.in_sample || {};
        const bCross = bRec.cross_ticker && typeof bRec.cross_ticker === "object" ? bRec.cross_ticker : null;
        const cCross = cRec.cross_ticker && typeof cRec.cross_ticker === "object" ? cRec.cross_ticker : null;
        const bCrossScore = bCross ? Number(bCross.score || 0) : NaN;
        const cCrossScore = cCross ? Number(cCross.score || 0) : NaN;
        const crossDelta = (Number.isFinite(bCrossScore) && Number.isFinite(cCrossScore)) ? fmt(delta(cCrossScore, bCrossScore), 6) : "-";
        lines.push(
          `  deltas:   robust=${fmt(delta(cRec.robust_score, bRec.robust_score), 6)} cross=${crossDelta} ` +
          `pnl=${fmt(delta(cm.pnl, bm.pnl), 6)} sharpe=${fmt(delta(cm.sharpe, bm.sharpe), 6)} ` +
          `dd=${fmt(delta(cm.max_dd, bm.max_dd), 6)} trades=${delta(cm.trades, bm.trades)}`
        );

        const bWf = bRec.walk_forward && typeof bRec.walk_forward === "object" ? bRec.walk_forward : null;
        const cWf = cRec.walk_forward && typeof cRec.walk_forward === "object" ? cRec.walk_forward : null;
        if (bWf && cWf) {
          lines.push(
            `  wfΔ:      sh_med=${fmt(delta(cWf.sharpe_median, bWf.sharpe_median), 4)} ` +
            `sh_worst=${fmt(delta(cWf.sharpe_worst, bWf.sharpe_worst), 4)} ` +
            `dd_worst=${fmt(delta(cWf.max_dd_worst, bWf.max_dd_worst), 4)} ` +
            `pnl_worst=${fmt(delta(cWf.pnl_worst, bWf.pnl_worst), 6)}`
          );
        }

        const bMc = bRec.trade_bootstrap && typeof bRec.trade_bootstrap === "object" ? bRec.trade_bootstrap : null;
        const cMc = cRec.trade_bootstrap && typeof cRec.trade_bootstrap === "object" ? cRec.trade_bootstrap : null;
        if (bMc && cMc) {
          lines.push(
            `  mcΔ:      p+=${fmt(delta(cMc.prob_pnl_positive, bMc.prob_pnl_positive), 4)} ` +
            `pnl_p05=${fmt(delta(cMc.pnl_p05, bMc.pnl_p05), 6)} ` +
            `dd_p95=${fmt(delta(cMc.max_dd_p95, bMc.max_dd_p95), 6)}`
          );
        }

        const bPt = bRec.perturb && typeof bRec.perturb === "object" ? bRec.perturb : null;
        const cPt = cRec.perturb && typeof cRec.perturb === "object" ? cRec.perturb : null;
        if (bPt && cPt) {
          lines.push(
            `  perturbΔ: sh_worst=${fmt(delta(cPt.sharpe_worst, bPt.sharpe_worst), 4)} ` +
            `dd_worst=${fmt(delta(cPt.max_dd_worst, bPt.max_dd_worst), 4)} ` +
            `pnl_worst=${fmt(delta(cPt.pnl_worst, bPt.pnl_worst), 6)}`
          );
        }
      }
      lines.push("");
    }

    const aParams = (bRec?.params || bp);
    const bParams = (cRec?.params || cp);
    const diffs = diffParams(aParams, bParams, 12);
    if (diffs.length) {
      lines.push("Params diff (current - baseline):");
      for (const d of diffs) {
        lines.push(`  ${d.key}: ${truncate(d.a, 60)} -> ${truncate(d.b, 60)}`);
      }
      const totalKeys = new Set([...(aParams ? Object.keys(aParams) : []), ...(bParams ? Object.keys(bParams) : [])]).size;
      if (totalKeys > diffs.length) lines.push(`  … (showing ${diffs.length}/${totalKeys} keys)`);
      lines.push("");
    }

    if (seq !== COMPARE_SEQ) return;
    el.textContent = lines.join("\n").trim() || "-";
  })().catch((e) => {
    if (seq !== COMPARE_SEQ) return;
    el.textContent = `Compare failed: ${String(e)}`;
  });
}

function paramsKey(p) {
  const pp = p || {};
  const mapKey = (m) => {
    if (!m || typeof m !== "object") return "";
    const keys = Object.keys(m).sort();
    if (!keys.length) return "";
    const parts = [];
    for (const k of keys) {
      const v = Number(m[k]);
      parts.push(`${k}=${Number.isFinite(v) ? v.toPrecision(12) : String(m[k])}`);
    }
    return parts.join(",");
  };
  const f = mapKey(pp.fast_ma_params);
  const s = mapKey(pp.slow_ma_params);
  return `${pp.fast_ma_type || ""}|${pp.fast_len || 0}|${pp.slow_ma_type || ""}|${pp.slow_len || 0}|${f}|${s}`;
}

function desiredDrillBins() {
  const canvas = $("equityCanvas");
  if (!canvas) return 1024;
  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const w = Math.floor((rect.width || 0) * dpr);
  if (w < 64) return 1024;
  return Math.max(64, Math.min(4096, w));
}

function clearStabilityUi() {
  const wrap = $("stabilityWrap");
  if (wrap) wrap.style.display = "none";
  const status = $("stabilityStatus");
  if (status) {
    status.textContent = "-";
    status.className = "muted";
  }
  const meta = $("stabilityMeta");
  if (meta) meta.textContent = "-";
  const reject = $("rejectMeta");
  if (reject) reject.textContent = "-";
  const tbody = $("stabilityTableBody");
  if (tbody) tbody.innerHTML = "";
  const tableWrap = $("stabilityTableWrap");
  if (tableWrap) tableWrap.style.display = "none";
}

function setStabilityVisible(visible) {
  const wrap = $("stabilityWrap");
  if (!wrap) return;
  wrap.style.display = visible ? "block" : "none";
}

function setStabilityStatus(msg, kind) {
  const el = $("stabilityStatus");
  if (!el) return;
  el.textContent = msg;
  el.className = "muted " + (kind || "");
}

function setDrillVisible(visible) {
  const wrap = $("drillWrap");
  if (!wrap) return;
  wrap.style.display = visible ? "block" : "none";
  const hint = $("drillEmptyHint");
  if (hint && !visible) {
    hint.textContent = CURRENT_RUN?.res
      ? "Click a row in the Top tab (or a point in Pareto) to compute drilldown curves."
      : "Run an optimization, then click a row in the Top tab to compute drilldown curves.";
  }
  setHintVisible("drillEmptyHint", !visible);
  if (!visible) {
    clearStabilityUi();
    clearDrillTvChart();
  }
}

function setDrillStatus(msg, kind) {
  const el = $("drillStatus");
  if (!el) return;
  el.textContent = msg;
  el.className = "muted " + (kind || "");
}

function drillChartCssVar(name, fallback) {
  const v = String(getComputedStyle(document.documentElement).getPropertyValue(name) || "").trim();
  return v || fallback;
}

function drillChartThemeOptions() {
  const dark = String(document.documentElement.dataset.theme || "light").trim() === "dark";
  const text = drillChartCssVar("--text", dark ? "#e5e7eb" : "#0f172a");
  const border = drillChartCssVar("--border", dark ? "#334155" : "#e2e8f0");
  const panel = drillChartCssVar("--surface2", dark ? "#16213a" : "#f1f5f9");
  return {
    layout: {
      background: { color: panel },
      textColor: text,
      fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial, sans-serif",
      fontSize: 11,
    },
    grid: {
      vertLines: { color: border, visible: true },
      horzLines: { color: border, visible: true },
    },
    rightPriceScale: { borderColor: border },
    timeScale: {
      borderColor: border,
      rightOffset: 8,
      barSpacing: 7,
      enableConflation: true,
      conflationThresholdFactor: 0.75,
      precomputeConflationOnInit: false,
      maxBarSpacing: 40,
    },
    crosshair: { mode: 1 },
  };
}

function chartTimeToUnixSec(t) {
  if (t == null) return null;
  if (typeof t === "number") return Number.isFinite(t) ? Math.floor(t) : null;
  if (typeof t === "string") {
    const n = Number(t);
    return Number.isFinite(n) ? Math.floor(n) : null;
  }
  if (typeof t === "object") {
    const anyT = t;
    const ts = Number(anyT?.timestamp);
    if (Number.isFinite(ts)) return Math.floor(ts);
    const y = Number(anyT?.year);
    const m = Number(anyT?.month);
    const d = Number(anyT?.day);
    if (Number.isFinite(y) && Number.isFinite(m) && Number.isFinite(d)) {
      return Math.floor(Date.UTC(y, m - 1, d) / 1000);
    }
  }
  return null;
}

function fmtPrice(x) {
  const n = Number(x);
  if (!Number.isFinite(n)) return "-";
  const abs = Math.abs(n);
  const d = abs >= 100 ? 2 : (abs >= 1 ? 4 : 6);
  return n.toFixed(d);
}

function formatUtcTimeFromSec(sec) {
  const t = Number(sec);
  if (!Number.isFinite(t)) return "-";
  const d = new Date(t * 1000);
  const pad = (n) => String(Math.max(0, Math.floor(Number(n || 0)))).padStart(2, "0");
  const y = d.getUTCFullYear();
  const m = pad(d.getUTCMonth() + 1);
  const day = pad(d.getUTCDate());
  const h = pad(d.getUTCHours());
  const min = pad(d.getUTCMinutes());
  const s = pad(d.getUTCSeconds());
  return `${y}-${m}-${day} ${h}:${min}:${s} UTC`;
}

function formatUtcTimeShortFromSec(sec) {
  const t = Number(sec);
  if (!Number.isFinite(t)) return "-";
  const d = new Date(t * 1000);
  const pad = (n) => String(Math.max(0, Math.floor(Number(n || 0)))).padStart(2, "0");
  const y = d.getUTCFullYear();
  const m = pad(d.getUTCMonth() + 1);
  const day = pad(d.getUTCDate());
  const h = pad(d.getUTCHours());
  const min = pad(d.getUTCMinutes());
  return `${y}-${m}-${day} ${h}:${min}`;
}

function ensureDrillTvLegendEl(host) {
  if (!host) return null;
  let el = $("drillTvLegend");
  if (!el) {
    el = document.createElement("div");
    el.id = "drillTvLegend";
    el.className = "tvLegend mono";
    el.textContent = "Hover chart to inspect OHLC and indicators.";
    const parent = host.parentElement || host;
    parent.appendChild(el);
  }
  return el;
}

function formatDrillTradeEventSummary(ev) {
  const obj = ev && typeof ev === "object" ? ev : {};
  const entries = Array.isArray(obj.entries) ? obj.entries : [];
  const exits = Array.isArray(obj.exits) ? obj.exits : [];
  if (!entries.length && !exits.length) return "";

  const parts = [];

  if (entries.length) {
    let longs = 0;
    let shorts = 0;
    for (const e of entries) {
      const dir = Number(e?.dir || 0);
      if (dir >= 0) longs++;
      else shorts++;
    }
    if (entries.length === 1) parts.push(longs ? "Entry: Long" : "Entry: Short");
    else parts.push(`Entry: ${entries.length} (L${longs}/S${shorts})`);
  }

  if (exits.length) {
    if (exits.length === 1) {
      const e = exits[0] || {};
      const open = !!e.open;
      const pnl = Number(e.pnl || 0);
      const pnlText = Number.isFinite(pnl) ? `${pnl >= 0 ? "+" : ""}${(pnl * 100).toFixed(2)}%` : "";
      parts.push(open ? "Exit: Open" : (pnlText ? `Exit: ${pnlText}` : "Exit"));
    } else {
      let openN = 0;
      let sum = 0;
      let sumN = 0;
      for (const e of exits) {
        if (e?.open) openN++;
        const pnl = Number(e?.pnl);
        if (Number.isFinite(pnl)) {
          sum += pnl;
          sumN++;
        }
      }
      const avg = sumN ? `${(sum / sumN * 100).toFixed(2)}%` : "-";
      parts.push(`Exit: ${exits.length} (open=${openN}, avg=${avg})`);
    }
  }

  return parts.join(" | ");
}

function composeDrillTvLegendText(crosshairText, pinnedText) {
  const c = String(crosshairText || "").trim();
  const p = String(pinnedText || "").trim();
  if (c && p) return `${c}\n\n${p}`;
  if (p) return p;
  if (c) return c;
  return "Hover chart to inspect OHLC and indicators.";
}

function applyDrillTvLegendDisplay(opts) {
  const el = DRILL_TV_LEGEND_EL;
  if (!el) return;
  const text = composeDrillTvLegendText(DRILL_TV_LEGEND_LAST_TEXT, DRILL_TV_LEGEND_PINNED_TEXT);
  if (opts && opts.opacity != null) el.style.opacity = String(opts.opacity);
  el.textContent = text;
}

function setDrillTvLegendPinnedText(text) {
  DRILL_TV_LEGEND_PINNED_TEXT = String(text || "").trim();
  const opacity = DRILL_TV_LEGEND_PINNED_TEXT ? "1" : (DRILL_TV_LEGEND_LAST_TEXT ? "0.85" : "0.72");
  applyDrillTvLegendDisplay({ opacity });
}

function formatDurationSec(sec) {
  const s = Math.max(0, Math.floor(Number(sec || 0)));
  if (!Number.isFinite(s) || s <= 0) return "-";
  const days = Math.floor(s / 86400);
  const hours = Math.floor((s % 86400) / 3600);
  const mins = Math.floor((s % 3600) / 60);
  const parts = [];
  if (days) parts.push(`${days}d`);
  if (hours || days) parts.push(`${hours}h`);
  parts.push(`${mins}m`);
  return parts.join(" ");
}

function buildPinnedDrillTradeText(tr, tradeIdx, total, bars) {
  const t = tr && typeof tr === "object" ? tr : {};
  const dirVal = Number(t.direction || 0);
  const dir = dirVal > 0 ? "Long" : dirVal < 0 ? "Short" : "Flat";
  const pnl = Number(t.pnl);
  const pnlStr = Number.isFinite(pnl) ? `${pnl >= 0 ? "+" : ""}${(pnl * 100).toFixed(2)}%` : "-";
  const barsN = Number(t.bars);
  const barsStr = Number.isFinite(barsN) ? String(barsN) : "-";
  const open = !!t.open;
  const state = open ? "open" : "closed";

  const head = `Selected trade #${tradeIdx + 1}/${Math.max(1, Math.floor(Number(total || 0)))}: ${dir}  pnl=${pnlStr}  bars=${barsStr}  ${state}`;

  const entryIdx = Number(t.entry_t);
  const exitIdx = Number(t.exit_t);
  const lines = [head];

  if (!Array.isArray(bars) || !bars.length) {
    const entryPart = Number.isFinite(entryIdx) ? `Entry idx=${Math.floor(entryIdx)}` : "Entry idx=-";
    const exitPart = open ? "Exit=open" : (Number.isFinite(exitIdx) ? `Exit idx=${Math.floor(exitIdx)}` : "Exit idx=-");
    lines.push(`${entryPart}  ${exitPart}`);
    return lines.join("\n");
  }

  const entryBarIdx = mapTradeIndexToBarIndex(entryIdx, bars);
  const exitBarIdx = open ? (bars.length - 1) : mapTradeIndexToBarIndex(exitIdx, bars);
  const entryBar = entryBarIdx == null ? null : bars[entryBarIdx];
  const exitBar = exitBarIdx == null ? null : bars[exitBarIdx];

  const entryTime = entryBar ? Number(entryBar.time) : NaN;
  const exitTime = exitBar ? Number(exitBar.time) : NaN;
  const entryPx = entryBar ? Number(entryBar.close) : NaN;
  const exitPx = exitBar ? Number(exitBar.close) : NaN;

  const entryTimeStr = Number.isFinite(entryTime) ? `${formatUtcTimeShortFromSec(entryTime)} UTC` : "-";
  const exitTimeStr = Number.isFinite(exitTime) ? `${formatUtcTimeShortFromSec(exitTime)} UTC` : "-";

  const entryLine = `Entry idx=${Number.isFinite(entryIdx) ? Math.floor(entryIdx) : "-"}  ${entryTimeStr}  px~${Number.isFinite(entryPx) ? fmtPrice(entryPx) : "-"}`;
  let exitLine;
  if (open) {
    const dur = Number.isFinite(entryTime) && Number.isFinite(exitTime) ? formatDurationSec(exitTime - entryTime) : "-";
    exitLine = `Exit open (as of ${exitTimeStr})  px~${Number.isFinite(exitPx) ? fmtPrice(exitPx) : "-"}  dur=${dur}`;
  } else {
    const dur = Number.isFinite(entryTime) && Number.isFinite(exitTime) ? formatDurationSec(exitTime - entryTime) : "-";
    exitLine = `Exit  idx=${Number.isFinite(exitIdx) ? Math.floor(exitIdx) : "-"}  ${exitTimeStr}  px~${Number.isFinite(exitPx) ? fmtPrice(exitPx) : "-"}  dur=${dur}`;
  }
  lines.push(entryLine);
  lines.push(exitLine);

  if (Number.isFinite(entryPx) && entryPx > 0 && entryBarIdx != null && exitBarIdx != null) {
    const lo = Math.max(0, Math.min(entryBarIdx, exitBarIdx));
    const hi = Math.min(bars.length - 1, Math.max(entryBarIdx, exitBarIdx));
    let maxHigh = -Infinity;
    let minLow = Infinity;
    for (let i = lo; i <= hi; i++) {
      const b = bars[i] || {};
      const h = Number(b.high);
      const l = Number(b.low);
      if (Number.isFinite(h)) maxHigh = Math.max(maxHigh, h);
      if (Number.isFinite(l)) minLow = Math.min(minLow, l);
    }
    if (Number.isFinite(maxHigh) && Number.isFinite(minLow)) {
      const mfe = dirVal >= 0 ? (maxHigh - entryPx) / entryPx : (entryPx - minLow) / entryPx;
      const mae = dirVal >= 0 ? (minLow - entryPx) / entryPx : (entryPx - maxHigh) / entryPx;
      const mfeStr = Number.isFinite(mfe) ? `${mfe >= 0 ? "+" : ""}${(mfe * 100).toFixed(2)}%` : "-";
      const maeStr = Number.isFinite(mae) ? `${mae >= 0 ? "+" : ""}${(mae * 100).toFixed(2)}%` : "-";
      lines.push(`MFE~${mfeStr}  MAE~${maeStr}`);
    }
  }

  return lines.join("\n");
}

function updateDrillTvLegendFromCrosshair(param) {
  const el = DRILL_TV_LEGEND_EL;
  if (!el) return;

  const time = chartTimeToUnixSec(param?.time);
  const sd = param?.seriesData;
  const hasData = time != null && sd && typeof sd.get === "function";

  if (!hasData) {
    const opacity = DRILL_TV_LEGEND_PINNED_TEXT ? "1" : "0.72";
    applyDrillTvLegendDisplay({ opacity });
    return;
  }

  const candle = DRILL_TV_SERIES ? sd.get(DRILL_TV_SERIES) : null;
  const fast = DRILL_TV_FAST_SERIES ? sd.get(DRILL_TV_FAST_SERIES) : null;
  const slow = DRILL_TV_SLOW_SERIES ? sd.get(DRILL_TV_SLOW_SERIES) : null;
  const halftrend = DRILL_TV_HALF_SERIES ? sd.get(DRILL_TV_HALF_SERIES) : null;
  const macz = DRILL_TV_MACZ_SERIES ? sd.get(DRILL_TV_MACZ_SERIES) : null;

  const lines = [formatUtcTimeFromSec(time)];

  const o = Number(candle?.open);
  const h = Number(candle?.high);
  const l = Number(candle?.low);
  const c = Number(candle?.close);
  if (Number.isFinite(o) && Number.isFinite(h) && Number.isFinite(l) && Number.isFinite(c)) {
    lines.push(`O ${fmtPrice(o)}  H ${fmtPrice(h)}  L ${fmtPrice(l)}  C ${fmtPrice(c)}`);
  }

  const fv = Number(fast?.value);
  const sv = Number(slow?.value);
  if (Number.isFinite(fv) || Number.isFinite(sv)) {
    const fStr = Number.isFinite(fv) ? fmtPrice(fv) : "-";
    const sStr = Number.isFinite(sv) ? fmtPrice(sv) : "-";
    lines.push(`fast ${fStr}  slow ${sStr}`);
  }

  const htv = Number(halftrend?.value);
  if (Number.isFinite(htv)) {
    lines.push(`halftrend ${fmt(htv, 4)}`);
  }
  const mzv = Number(macz?.value);
  if (Number.isFinite(mzv)) {
    lines.push(`macz ${fmt(mzv, 6)}`);
  }

  const ev = DRILL_TV_TRADE_EVENTS_BY_TIME && typeof DRILL_TV_TRADE_EVENTS_BY_TIME.get === "function"
    ? DRILL_TV_TRADE_EVENTS_BY_TIME.get(time)
    : null;
  const evText = formatDrillTradeEventSummary(ev);
  if (evText) lines.push(evText);

  const crosshairText = lines.join("\n");
  DRILL_TV_LEGEND_LAST_TEXT = crosshairText;
  el.style.opacity = "1";
  el.textContent = composeDrillTvLegendText(crosshairText, DRILL_TV_LEGEND_PINNED_TEXT);
}

function bestEffortCurrentDrillBars() {
  const dataId = String(CURRENT_RUN?.req?.data_id || CURRENT_RUN?.req?.dataId || "").trim();
  if (dataId && dataId === DRILL_LAST_BARS_DATA_ID && Array.isArray(DRILL_LAST_BARS)) {
    return DRILL_LAST_BARS;
  }
  if (dataId && DRILL_PRICE_BARS_CACHE.has(dataId)) return DRILL_PRICE_BARS_CACHE.get(dataId);
  return null;
}

function computeTradeTableStart(total, selectedIdx) {
  const n = Math.max(1, Math.floor(Number(DRILL_TRADES_TABLE_MAX_ROWS || 200)));
  if (!Number.isFinite(total) || total <= n) return 0;
  const maxStart = Math.max(0, total - n);
  const sel = Math.floor(Number(selectedIdx));
  if (Number.isFinite(sel) && sel >= 0 && sel < total) {
    const half = Math.floor(n / 2);
    return Math.max(0, Math.min(maxStart, sel - half));
  }
  return maxStart;
}

function clearTradeSelection() {
  DRILL_SELECTED_TRADE_IDX = -1;
  setDrillTvLegendPinnedText("");
}

function applyTradeRowSelection() {
  const body = $("tradeTableBody");
  if (!body) return;
  for (const el of body.querySelectorAll("tr.selected")) el.classList.remove("selected");
  if (DRILL_SELECTED_TRADE_IDX < 0) return;
  const row = body.querySelector(`tr[data-trade-idx=\"${DRILL_SELECTED_TRADE_IDX}\"]`);
  if (row) row.classList.add("selected");
}

function scrollTradeRowIntoView() {
  const body = $("tradeTableBody");
  if (!body) return;
  if (DRILL_SELECTED_TRADE_IDX < 0) return;
  const row = body.querySelector(`tr[data-trade-idx=\"${DRILL_SELECTED_TRADE_IDX}\"]`);
  if (row && typeof row.scrollIntoView === "function") {
    row.scrollIntoView({ block: "center" });
  }
}

function focusDrillTvChartOnTradeRange(entryTimeSec, exitTimeSec) {
  const chart = DRILL_TV_CHART;
  const bars = bestEffortCurrentDrillBars();
  if (!chart || !bars || !bars.length) return;
  const ts = chart.timeScale && chart.timeScale();
  if (!ts || typeof ts.setVisibleRange !== "function") return;

  const start = Number(entryTimeSec);
  const end = exitTimeSec == null ? NaN : Number(exitTimeSec);
  if (!Number.isFinite(start)) return;
  const baseEnd = Number.isFinite(end) ? end : start;

  // Derive padding from the candle step (roughly 20 bars).
  const dt = bars.length > 2 ? Math.max(1, Math.round((bars[2].time - bars[0].time) / 2)) : 3600;
  const pad = Math.max(1, dt * 20);

  const minT = Number(bars[0].time);
  const maxT = Number(bars[bars.length - 1].time);
  const from = Math.max(minT, start - pad);
  const to = Math.min(maxT, baseEnd + pad);

  try {
    ts.setVisibleRange({ from, to });
  } catch {
    // ignore chart API mismatch
  }
}

function selectDrillTradeByIndex(idx, opts) {
  const curves = DRILL_LAST_CURVES && typeof DRILL_LAST_CURVES === "object" ? DRILL_LAST_CURVES : null;
  const trades = Array.isArray(curves?.trades) ? curves.trades : [];
  const total = trades.length;
  const i = Math.floor(Number(idx));
  if (!Number.isFinite(i) || i < 0 || i >= total) return;

  DRILL_SELECTED_TRADE_IDX = i;
  DRILL_TRADES_RANGE_START = computeTradeTableStart(total, i);

  const bars = bestEffortCurrentDrillBars();
  const tr = trades[i] || {};
  setDrillTvLegendPinnedText(buildPinnedDrillTradeText(tr, i, total, bars));
  renderTrades(curves, { bars, fromSelection: true });
  applyTradeRowSelection();
  if (opts?.scroll !== false) scrollTradeRowIntoView();

  if (bars) {
    const entryTime = mapTradeIndexToChartTime(tr?.entry_t, bars);
    const exitTime = tr?.open ? null : mapTradeIndexToChartTime(tr?.exit_t, bars);
    focusDrillTvChartOnTradeRange(entryTime, exitTime);
  }
}

function findNearestTradeEventTime(targetSec) {
  const map = DRILL_TV_TRADE_EVENTS_BY_TIME;
  if (!map || typeof map.get !== "function" || map.size === 0) return null;
  const t0 = Number(targetSec);
  if (!Number.isFinite(t0)) return null;
  if (map.has(t0)) return t0;
  let bestT = null;
  let bestD = Infinity;
  for (const [t] of map.entries()) {
    const tt = Number(t);
    if (!Number.isFinite(tt)) continue;
    const d = Math.abs(tt - t0);
    if (d < bestD) {
      bestD = d;
      bestT = tt;
    }
  }
  return bestT;
}

function pickTradeIndexFromEvent(ev) {
  const obj = ev && typeof ev === "object" ? ev : {};
  const exits = Array.isArray(obj.exits) ? obj.exits : [];
  const entries = Array.isArray(obj.entries) ? obj.entries : [];
  if (exits.length) {
    let best = exits[0];
    let bestAbs = Math.abs(Number(best?.pnl || 0));
    for (const e of exits) {
      const abs = Math.abs(Number(e?.pnl || 0));
      if (abs > bestAbs) {
        bestAbs = abs;
        best = e;
      }
    }
    const idx = Number(best?.idx);
    return Number.isFinite(idx) ? idx : null;
  }
  if (entries.length) {
    const idx = Number(entries[0]?.idx);
    return Number.isFinite(idx) ? idx : null;
  }
  return null;
}

function onDrillTvChartClick(param) {
  const time = chartTimeToUnixSec(param?.time);
  if (time == null) return;
  const hitT = findNearestTradeEventTime(time);
  if (hitT == null) return;
  const bars = bestEffortCurrentDrillBars();
  const dt = bars && bars.length > 2 ? Math.max(1, Math.round((bars[2].time - bars[0].time) / 2)) : 3600;
  const maxDist = dt * 3;
  if (Math.abs(hitT - time) > maxDist) return;
  const ev = DRILL_TV_TRADE_EVENTS_BY_TIME && typeof DRILL_TV_TRADE_EVENTS_BY_TIME.get === "function"
    ? DRILL_TV_TRADE_EVENTS_BY_TIME.get(hitT)
    : null;
  const idx = pickTradeIndexFromEvent(ev);
  if (idx == null) return;
  selectDrillTradeByIndex(idx, { scroll: true });
}

function ensureDrillTvAuxPanes() {
  if (!DRILL_TV_CHART || typeof DRILL_TV_CHART.addSeries !== "function") return;
  if (typeof DRILL_TV_CHART.addPane !== "function" || typeof DRILL_TV_CHART.panes !== "function") return;

  try {
    while (Array.isArray(DRILL_TV_CHART.panes()) && DRILL_TV_CHART.panes().length < 3) {
      DRILL_TV_CHART.addPane();
    }
  } catch {
    return;
  }

  const halfOpts = {
    color: "rgba(56, 189, 248, 0.95)",
    lineWidth: 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
  };
  const maczOpts = {
    color: "rgba(148, 163, 184, 0.9)",
    base: 0,
    lastValueVisible: false,
    priceLineVisible: false,
  };

  try {
    if (!DRILL_TV_HALF_SERIES && LineSeries) {
      DRILL_TV_HALF_SERIES = DRILL_TV_CHART.addSeries(LineSeries, halfOpts, 1);
      if (DRILL_TV_HALF_SERIES && typeof DRILL_TV_HALF_SERIES.createPriceLine === "function") {
        DRILL_TV_HALF_SERIES.createPriceLine({
          price: 0,
          color: "rgba(148, 163, 184, 0.35)",
          lineWidth: 1,
          lineStyle: 2,
          axisLabelVisible: false,
          title: "",
        });
      }
    }
  } catch {
    DRILL_TV_HALF_SERIES = null;
  }
  try {
    if (!DRILL_TV_MACZ_SERIES && HistogramSeries) {
      DRILL_TV_MACZ_SERIES = DRILL_TV_CHART.addSeries(HistogramSeries, maczOpts, 2);
    }
  } catch {
    DRILL_TV_MACZ_SERIES = null;
  }

  try {
    const panes = DRILL_TV_CHART.panes();
    if (Array.isArray(panes)) {
      if (panes[1] && typeof panes[1].setHeight === "function") panes[1].setHeight(96);
      if (panes[2] && typeof panes[2].setHeight === "function") panes[2].setHeight(120);
    }
  } catch {
    // optional pane sizing
  }
}

function ensureDrillTvChart() {
  const host = $("drillTvChart");
  if (!host) return null;
  if (DRILL_TV_CHART && DRILL_TV_SERIES) {
    ensureDrillTvAuxPanes();
    return { chart: DRILL_TV_CHART, series: DRILL_TV_SERIES, host };
  }

  const width = Math.max(240, Math.floor(host.clientWidth || 820));
  const height = Math.max(220, Math.floor(host.clientHeight || 360));
  DRILL_TV_CHART = createChart(host, { ...drillChartThemeOptions(), width, height });
  const candleOpts = {
    upColor: "rgba(22, 163, 74, 0.9)",
    downColor: "rgba(225, 29, 72, 0.9)",
    wickUpColor: "rgba(22, 163, 74, 0.9)",
    wickDownColor: "rgba(225, 29, 72, 0.9)",
    borderVisible: true,
    borderUpColor: "rgba(22, 163, 74, 1)",
    borderDownColor: "rgba(225, 29, 72, 1)",
  };
  if (typeof DRILL_TV_CHART.addSeries === "function" && CandlestickSeries) {
    try {
      DRILL_TV_SERIES = DRILL_TV_CHART.addSeries(CandlestickSeries, candleOpts);
    } catch {
      DRILL_TV_SERIES = null;
    }
  }
  if (!DRILL_TV_SERIES && typeof DRILL_TV_CHART.addCandlestickSeries === "function") {
    DRILL_TV_SERIES = DRILL_TV_CHART.addCandlestickSeries(candleOpts);
  }
  if (!DRILL_TV_SERIES) throw new Error("Lightweight Charts candlestick API unavailable.");
  const fastOpts = {
    color: "rgba(14, 165, 233, 0.95)",
    lineWidth: 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
  };
  const slowOpts = {
    color: "rgba(245, 158, 11, 0.95)",
    lineWidth: 2,
    crosshairMarkerVisible: false,
    lastValueVisible: false,
    priceLineVisible: false,
  };
  if (typeof DRILL_TV_CHART.addSeries === "function" && LineSeries) {
    try {
      DRILL_TV_FAST_SERIES = DRILL_TV_CHART.addSeries(LineSeries, fastOpts);
      DRILL_TV_SLOW_SERIES = DRILL_TV_CHART.addSeries(LineSeries, slowOpts);
    } catch {
      DRILL_TV_FAST_SERIES = null;
      DRILL_TV_SLOW_SERIES = null;
    }
  }
  if ((!DRILL_TV_FAST_SERIES || !DRILL_TV_SLOW_SERIES) && typeof DRILL_TV_CHART.addLineSeries === "function") {
    DRILL_TV_FAST_SERIES = DRILL_TV_FAST_SERIES || DRILL_TV_CHART.addLineSeries(fastOpts);
    DRILL_TV_SLOW_SERIES = DRILL_TV_SLOW_SERIES || DRILL_TV_CHART.addLineSeries(slowOpts);
  }
  DRILL_TV_MARKERS = createSeriesMarkers(DRILL_TV_SERIES, []);
  ensureDrillTvAuxPanes();

  if (typeof ResizeObserver !== "undefined") {
    DRILL_TV_RESIZE_OBSERVER = new ResizeObserver(() => {
      if (!DRILL_TV_CHART) return;
      const rect = host.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      DRILL_TV_CHART.applyOptions({
        width: Math.max(240, Math.floor(rect.width)),
        height: Math.max(220, Math.floor(rect.height)),
      });
    });
    DRILL_TV_RESIZE_OBSERVER.observe(host);
  }

  DRILL_TV_LEGEND_EL = ensureDrillTvLegendEl(host);
  applyDrillTvLegendDisplay();
  if (!DRILL_TV_LEGEND_SUBSCRIBED && typeof DRILL_TV_CHART.subscribeCrosshairMove === "function") {
    DRILL_TV_CHART.subscribeCrosshairMove((p) => updateDrillTvLegendFromCrosshair(p));
    DRILL_TV_LEGEND_SUBSCRIBED = true;
  }
  if (!DRILL_TV_CLICK_SUBSCRIBED && typeof DRILL_TV_CHART.subscribeClick === "function") {
    DRILL_TV_CHART.subscribeClick((p) => onDrillTvChartClick(p));
    DRILL_TV_CLICK_SUBSCRIBED = true;
  }
  if (!DRILL_TV_RANGE_SUBSCRIBED) {
    try {
      const ts = DRILL_TV_CHART.timeScale && DRILL_TV_CHART.timeScale();
      if (ts && typeof ts.subscribeVisibleLogicalRangeChange === "function") {
        ts.subscribeVisibleLogicalRangeChange(() => {
          scheduleDrillTvVisibleMarkerRefresh();
        });
        DRILL_TV_RANGE_SUBSCRIBED = true;
      }
    } catch {
      // optional in older chart API builds
    }
  }

  return { chart: DRILL_TV_CHART, series: DRILL_TV_SERIES, host };
}

function refreshDrillTvChartTheme() {
  if (!DRILL_TV_CHART) return;
  DRILL_TV_CHART.applyOptions(drillChartThemeOptions());
}

function clearDrillTvChart(msg = "Hover for OHLC/MA values; click a marker to focus a trade.") {
  const meta = $("drillTvChartMeta");
  if (meta && msg != null) meta.textContent = String(msg);
  if (DRILL_TV_SERIES) DRILL_TV_SERIES.setData([]);
  if (DRILL_TV_FAST_SERIES) DRILL_TV_FAST_SERIES.setData([]);
  if (DRILL_TV_SLOW_SERIES) DRILL_TV_SLOW_SERIES.setData([]);
  if (DRILL_TV_HALF_SERIES) DRILL_TV_HALF_SERIES.setData([]);
  if (DRILL_TV_MACZ_SERIES) DRILL_TV_MACZ_SERIES.setData([]);
  if (DRILL_TV_MARKERS && typeof DRILL_TV_MARKERS.setMarkers === "function") {
    DRILL_TV_MARKERS.setMarkers([]);
  }
  DRILL_TV_ALL_MARKERS = [];
  DRILL_TV_MARKER_LAST_KEY = "";
  if (DRILL_TV_MARKER_RAF) {
    if (typeof cancelAnimationFrame === "function") cancelAnimationFrame(DRILL_TV_MARKER_RAF);
    else clearTimeout(DRILL_TV_MARKER_RAF);
    DRILL_TV_MARKER_RAF = 0;
  }
  DRILL_TV_TRADE_EVENTS_BY_TIME = null;
  DRILL_TV_LEGEND_LAST_TEXT = "";
  DRILL_TV_LEGEND_PINNED_TEXT = "";
  applyDrillTvLegendDisplay({ opacity: "0.72" });
}

function normalizeChartTimeSec(ts) {
  const t = Number(ts);
  if (!Number.isFinite(t)) return NaN;
  const abs = Math.abs(t);
  if (abs >= 1e17) return Math.floor(t / 1e9); // nanoseconds -> seconds
  if (abs >= 1e14) return Math.floor(t / 1e6); // microseconds -> seconds
  if (abs >= 1e11) return Math.floor(t / 1e3); // milliseconds -> seconds
  return Math.floor(t);
}

function normalizePriceBarsForChart(rows) {
  let input = [];
  if (Array.isArray(rows)) input = rows;
  else if (rows && typeof rows === "object") {
    const anyRows = rows;
    if (Array.isArray(anyRows.bars)) input = anyRows.bars;
    else if (Array.isArray(anyRows.items)) input = anyRows.items;
    else if (Array.isArray(anyRows.data)) input = anyRows.data;
    else if (Array.isArray(anyRows.result)) input = anyRows.result;
    else if (anyRows.result && typeof anyRows.result === "object" && Array.isArray(anyRows.result.bars)) input = anyRows.result.bars;
  }

  const out = [];
  for (const r of input) {
    const t = normalizeChartTimeSec(r?.time ?? r?.timestamp ?? r?.ts ?? r?.t);
    const open = Number(r?.open ?? r?.o);
    const highRaw = Number(r?.high ?? r?.h);
    const lowRaw = Number(r?.low ?? r?.l);
    const close = Number(r?.close ?? r?.c);
    const high = Math.max(highRaw, open, close);
    const low = Math.min(lowRaw, open, close);
    if (!Number.isFinite(t) || !Number.isFinite(open) || !Number.isFinite(high) || !Number.isFinite(low) || !Number.isFinite(close)) continue;
    const fromIndex = Math.max(0, Math.floor(Number(r?.from_index ?? r?.fromIndex ?? r?.from ?? 0)));
    const toIndex = Math.max(fromIndex, Math.floor(Number(r?.to_index ?? r?.toIndex ?? r?.to ?? fromIndex)));
    out.push({ time: t, open, high, low, close, fromIndex, toIndex, volume: Number(r?.volume ?? r?.v ?? 0) });
  }
  if (out.length <= 1) return out;
  out.sort((a, b) => a.time - b.time);
  const deduped = [];
  for (const row of out) {
    const last = deduped[deduped.length - 1];
    if (last && row.time === last.time) deduped[deduped.length - 1] = row;
    else deduped.push(row);
  }
  return deduped;
}

function normalizeIndicatorBarsForChart(rows) {
  const input = Array.isArray(rows) ? rows : [];
  const out = [];
  for (const r of input) {
    const time = normalizeChartTimeSec(r?.time ?? r?.timestamp ?? r?.ts ?? r?.t);
    const fromIndex = Math.max(0, Math.floor(Number(r?.from_index ?? r?.fromIndex ?? r?.from ?? 0)));
    const toIndex = Math.max(fromIndex, Math.floor(Number(r?.to_index ?? r?.toIndex ?? r?.to ?? fromIndex)));
    const fastRaw = Number(r?.fast);
    const slowRaw = Number(r?.slow);
    const fast = Number.isFinite(fastRaw) ? fastRaw : null;
    const slow = Number.isFinite(slowRaw) ? slowRaw : null;
    if (!Number.isFinite(time)) continue;
    if (fast == null && slow == null) continue;
    out.push({ time, fromIndex, toIndex, fast, slow });
  }
  if (out.length <= 1) return out;
  out.sort((a, b) => a.time - b.time);
  const deduped = [];
  for (const row of out) {
    const last = deduped[deduped.length - 1];
    if (last && row.time === last.time) {
      deduped[deduped.length - 1] = {
        ...row,
        fast: row.fast != null ? row.fast : last.fast,
        slow: row.slow != null ? row.slow : last.slow,
      };
    } else {
      deduped.push(row);
    }
  }
  return deduped;
}

function normalizeCompositeIndicatorBarsForChart(rows) {
  const input = Array.isArray(rows) ? rows : [];
  const out = [];
  for (const r of input) {
    const time = normalizeChartTimeSec(r?.time ?? r?.timestamp ?? r?.ts ?? r?.t);
    const fromIndex = Math.max(0, Math.floor(Number(r?.from_index ?? r?.fromIndex ?? r?.from ?? 0)));
    const toIndex = Math.max(fromIndex, Math.floor(Number(r?.to_index ?? r?.toIndex ?? r?.to ?? fromIndex)));
    const halftrendRaw = Number(r?.halftrend);
    const maczRaw = Number(r?.macz_hist ?? r?.macz);
    const ottLineRaw = Number(r?.ott_line);
    const ottPriceRaw = Number(r?.ott_price);
    const halftrend = Number.isFinite(halftrendRaw) ? halftrendRaw : null;
    const macz = Number.isFinite(maczRaw) ? maczRaw : null;
    const ottLine = Number.isFinite(ottLineRaw) ? ottLineRaw : null;
    const ottPrice = Number.isFinite(ottPriceRaw) ? ottPriceRaw : null;
    if (!Number.isFinite(time)) continue;
    if (halftrend == null && macz == null && ottLine == null && ottPrice == null) continue;
    out.push({ time, fromIndex, toIndex, halftrend, macz, ottLine, ottPrice });
  }
  if (out.length <= 1) return out;
  out.sort((a, b) => a.time - b.time);
  const deduped = [];
  for (const row of out) {
    const last = deduped[deduped.length - 1];
    if (last && row.time === last.time) {
      deduped[deduped.length - 1] = {
        ...row,
        halftrend: row.halftrend != null ? row.halftrend : last.halftrend,
        macz: row.macz != null ? row.macz : last.macz,
        ottLine: row.ottLine != null ? row.ottLine : last.ottLine,
        ottPrice: row.ottPrice != null ? row.ottPrice : last.ottPrice,
      };
    } else {
      deduped.push(row);
    }
  }
  return deduped;
}

async function fetchDrillPriceBars(dataId) {
  const id = String(dataId || "").trim();
  if (!id) return [];
  if (DRILL_PRICE_BARS_CACHE.has(id)) return DRILL_PRICE_BARS_CACHE.get(id);
  const invoke = tauriInvoke();
  if (!invoke) return [];
  let raw;
  try {
    raw = await invoke("get_price_bars", { dataId: id, maxPoints: 8000 });
  } catch (firstErr) {
    try {
      raw = await invoke("get_price_bars", { data_id: id, max_points: 8000 });
    } catch {
      throw firstErr;
    }
  }
  const bars = normalizePriceBarsForChart(raw);
  DRILL_PRICE_BARS_CACHE.set(id, bars);
  return bars;
}

function mapTradeIndexToChartTime(idx, bars) {
  const x = Math.max(0, Math.floor(Number(idx || 0)));
  if (!Array.isArray(bars) || !bars.length) return null;
  let monotonicRanges = true;
  let prevFrom = -Infinity;
  for (const b of bars) {
    if (!Number.isFinite(b?.fromIndex) || !Number.isFinite(b?.toIndex) || b.fromIndex < prevFrom || b.toIndex < b.fromIndex) {
      monotonicRanges = false;
      break;
    }
    prevFrom = b.fromIndex;
  }
  if (!monotonicRanges) {
    let bestTime = bars[0].time;
    let bestDist = Infinity;
    for (const b of bars) {
      if (x >= b.fromIndex && x <= b.toIndex) return b.time;
      const d = x < b.fromIndex ? (b.fromIndex - x) : (x - b.toIndex);
      if (d < bestDist) {
        bestDist = d;
        bestTime = b.time;
      }
    }
    return bestTime;
  }
  let lo = 0;
  let hi = bars.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const b = bars[mid];
    if (x < b.fromIndex) hi = mid - 1;
    else if (x > b.toIndex) lo = mid + 1;
    else return b.time;
  }
  if (hi < 0) return bars[0].time;
  if (lo >= bars.length) return bars[bars.length - 1].time;
  const left = bars[hi];
  const right = bars[lo];
  return Math.abs(x - left.toIndex) <= Math.abs(right.fromIndex - x) ? left.time : right.time;
}

function mapTradeIndexToBarIndex(idx, bars) {
  const x = Math.max(0, Math.floor(Number(idx || 0)));
  if (!Array.isArray(bars) || !bars.length) return null;
  let monotonicRanges = true;
  let prevFrom = -Infinity;
  for (const b of bars) {
    if (!Number.isFinite(b?.fromIndex) || !Number.isFinite(b?.toIndex) || b.fromIndex < prevFrom || b.toIndex < b.fromIndex) {
      monotonicRanges = false;
      break;
    }
    prevFrom = b.fromIndex;
  }
  if (!monotonicRanges) {
    let bestI = 0;
    let bestDist = Infinity;
    for (let i = 0; i < bars.length; i++) {
      const b = bars[i];
      if (x >= b.fromIndex && x <= b.toIndex) return i;
      const d = x < b.fromIndex ? (b.fromIndex - x) : (x - b.toIndex);
      if (d < bestDist) {
        bestDist = d;
        bestI = i;
      }
    }
    return bestI;
  }
  let lo = 0;
  let hi = bars.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const b = bars[mid];
    if (x < b.fromIndex) hi = mid - 1;
    else if (x > b.toIndex) lo = mid + 1;
    else return mid;
  }
  if (hi < 0) return 0;
  if (lo >= bars.length) return bars.length - 1;
  const left = bars[hi];
  const right = bars[lo];
  return Math.abs(x - left.toIndex) <= Math.abs(right.fromIndex - x) ? hi : lo;
}

function buildDrillTradeMarkers(curves, bars) {
  const trades = Array.isArray(curves?.trades) ? curves.trades : [];
  if (!trades.length || !bars.length) return [];
  const compact = trades.length > 1200;
  const adaptiveMaxTrades = compact
    ? Math.max(900, Math.min(3200, Math.floor(bars.length * 0.5)))
    : Math.max(700, Math.min(6000, Math.floor(bars.length * 1.3)));
  const shown = trades.length > adaptiveMaxTrades
    ? trades.slice(trades.length - adaptiveMaxTrades)
    : trades;
  const perTradeMarkers = [];
  for (const tr of shown) {
    const dir = Number(tr?.direction || 0);
    const pnl = Number(tr?.pnl || 0);
    const entryTime = mapTradeIndexToChartTime(tr?.entry_t, bars);
    const exitTime = mapTradeIndexToChartTime(tr?.exit_t, bars);
    const entryMarker = entryTime != null
      ? {
        time: entryTime,
        position: dir >= 0 ? "belowBar" : "aboveBar",
        shape: dir >= 0 ? "arrowUp" : "arrowDown",
        color: dir >= 0 ? "rgba(14, 165, 233, 0.98)" : "rgba(168, 85, 247, 0.98)",
        text: compact ? "" : (dir >= 0 ? "Long in" : "Short in"),
      }
      : null;
    let exitMarker = null;
    if (exitTime != null) {
      const openTrade = !!tr?.open;
      const pnlText = Number.isFinite(pnl) ? `${pnl >= 0 ? "+" : ""}${(pnl * 100).toFixed(1)}%` : "";
      const color = openTrade
        ? "rgba(148, 163, 184, 0.95)"
        : (pnl >= 0 ? "rgba(22, 163, 74, 0.95)" : "rgba(225, 29, 72, 0.95)");
      exitMarker = {
        time: exitTime,
        position: dir >= 0 ? "aboveBar" : "belowBar",
        shape: "circle",
        color,
        text: compact ? "" : (openTrade ? "Open" : (pnlText ? `Exit ${pnlText}` : "Exit")),
      };
    }
    if (entryMarker || exitMarker) perTradeMarkers.push({ entryMarker, exitMarker });
  }
  if (!perTradeMarkers.length) return [];

  if (compact) {
    const targetTrades = Math.max(600, Math.min(1800, Math.floor(adaptiveMaxTrades * 0.6)));
    let sampled = perTradeMarkers;
    if (perTradeMarkers.length > targetTrades) {
      const stride = Math.ceil(perTradeMarkers.length / targetTrades);
      const out = [];
      for (let i = 0; i < perTradeMarkers.length; i += stride) out.push(perTradeMarkers[i]);
      const last = perTradeMarkers[perTradeMarkers.length - 1];
      if (out.length && out[out.length - 1] !== last) out.push(last);
      sampled = out.length > targetTrades ? out.slice(out.length - targetTrades) : out;
    }
    const merged = [];
    for (const pair of sampled) {
      if (pair.entryMarker) merged.push({ ...pair.entryMarker, text: "" });
      if (pair.exitMarker) merged.push({ ...pair.exitMarker, text: "" });
    }
    const uniq = new Map();
    for (const m of merged) {
      const key = `${m.time}|${m.position}|${m.shape}|${m.color}`;
      if (!uniq.has(key)) uniq.set(key, m);
    }
    const deduped = [...uniq.values()];
    deduped.sort((a, b) => Number(a.time) - Number(b.time));
    if (deduped.length > 3200) return deduped.slice(deduped.length - 3200);
    return deduped;
  }

  const markers = [];
  for (const pair of perTradeMarkers) {
    if (pair.entryMarker) markers.push(pair.entryMarker);
    if (pair.exitMarker) markers.push(pair.exitMarker);
  }
  markers.sort((a, b) => Number(a.time) - Number(b.time));
  if (markers.length > 8000) return markers.slice(markers.length - 8000);
  return markers;
}

function buildDrillTradeEventIndex(curves, bars) {
  const trades = Array.isArray(curves?.trades) ? curves.trades : [];
  if (!trades.length || !bars.length) return null;

  const compact = trades.length > 1200;
  const adaptiveMaxTrades = compact
    ? Math.max(900, Math.min(3200, Math.floor(bars.length * 0.5)))
    : Math.max(700, Math.min(6000, Math.floor(bars.length * 1.3)));
  const shown = trades.length > adaptiveMaxTrades
    ? trades.slice(trades.length - adaptiveMaxTrades)
    : trades;
  const shownStartIdx = Math.max(0, trades.length - shown.length);

  const out = new Map();

  for (let local = 0; local < shown.length; local++) {
    const tr = shown[local];
    const idx = shownStartIdx + local;
    const dir = Number(tr?.direction || 0);
    const pnl = Number(tr?.pnl || 0);
    const openTrade = !!tr?.open;

    const entryTime = mapTradeIndexToChartTime(tr?.entry_t, bars);
    const exitTime = mapTradeIndexToChartTime(tr?.exit_t, bars);

    if (entryTime != null) {
      const key = Number(entryTime);
      const cur = out.get(key) || { entries: [], exits: [] };
      cur.entries.push({ idx, dir });
      out.set(key, cur);
    }
    if (exitTime != null) {
      const key = Number(exitTime);
      const cur = out.get(key) || { entries: [], exits: [] };
      cur.exits.push({ idx, dir, pnl, open: openTrade });
      out.set(key, cur);
    }
  }

  return out;
}

function drillMarkerVisibleWindowFromLogicalRange() {
  const chart = DRILL_TV_CHART;
  const bars = Array.isArray(DRILL_LAST_BARS) ? DRILL_LAST_BARS : [];
  if (!chart || !bars.length) return null;
  const ts = chart.timeScale && chart.timeScale();
  if (!ts || typeof ts.getVisibleLogicalRange !== "function") return null;

  const lr = ts.getVisibleLogicalRange();
  if (!lr) return null;
  const fromRaw = Number(lr.from);
  const toRaw = Number(lr.to);
  if (!Number.isFinite(fromRaw) || !Number.isFinite(toRaw)) return null;
  if (toRaw < fromRaw) return null;

  const span = Math.max(1, Math.ceil(toRaw - fromRaw));
  const pad = Math.max(24, Math.min(420, Math.floor(span * 0.12)));
  const lo = Math.max(0, Math.floor(fromRaw) - pad);
  const hi = Math.min(bars.length - 1, Math.ceil(toRaw) + pad);
  if (hi < lo) return null;

  const fromTime = Number(bars[lo]?.time);
  const toTime = Number(bars[hi]?.time);
  if (!Number.isFinite(fromTime) || !Number.isFinite(toTime)) return null;
  return {
    fromTime,
    toTime,
    key: `${lo}:${hi}:${bars.length}`,
  };
}

function selectDrillMarkersForVisibleWindow(allMarkers, windowInfo) {
  const xs = Array.isArray(allMarkers) ? allMarkers : [];
  if (!xs.length) return [];
  if (!windowInfo) {
    return xs;
  }

  const fromTime = Number(windowInfo.fromTime);
  const toTime = Number(windowInfo.toTime);
  if (!Number.isFinite(fromTime) || !Number.isFinite(toTime) || toTime < fromTime) {
    return xs;
  }

  const out = [];
  for (const m0 of xs) {
    const m = m0 && typeof m0 === "object" ? m0 : null;
    if (!m) continue;
    const t = Number(m.time);
    if (!Number.isFinite(t) || t < fromTime || t > toTime) continue;
    out.push(m);
  }
  return out;
}

function applyDrillTvVisibleMarkers(opts) {
  if (!DRILL_TV_MARKERS || typeof DRILL_TV_MARKERS.setMarkers !== "function") return;
  const allMarkers = Array.isArray(DRILL_TV_ALL_MARKERS) ? DRILL_TV_ALL_MARKERS : [];
  if (!allMarkers.length) {
    DRILL_TV_MARKERS.setMarkers([]);
    DRILL_TV_MARKER_LAST_KEY = "empty";
    return;
  }

  const windowInfo = drillMarkerVisibleWindowFromLogicalRange();
  const visible = selectDrillMarkersForVisibleWindow(allMarkers, windowInfo);
  const key = windowInfo
    ? `${windowInfo.key}:${visible.length}:${allMarkers.length}`
    : `all:${visible.length}:${allMarkers.length}`;

  if (!opts?.force && key === DRILL_TV_MARKER_LAST_KEY) return;
  DRILL_TV_MARKERS.setMarkers(visible);
  DRILL_TV_MARKER_LAST_KEY = key;
}

function scheduleDrillTvVisibleMarkerRefresh(opts) {
  if (opts?.force) {
    if (DRILL_TV_MARKER_RAF) {
      if (typeof cancelAnimationFrame === "function") cancelAnimationFrame(DRILL_TV_MARKER_RAF);
      else clearTimeout(DRILL_TV_MARKER_RAF);
      DRILL_TV_MARKER_RAF = 0;
    }
    applyDrillTvVisibleMarkers({ force: true });
    return;
  }
  if (DRILL_TV_MARKER_RAF) return;
  const schedule = typeof requestAnimationFrame === "function"
    ? requestAnimationFrame
    : (cb) => setTimeout(cb, 16);
  DRILL_TV_MARKER_RAF = schedule(() => {
    DRILL_TV_MARKER_RAF = 0;
    applyDrillTvVisibleMarkers();
  });
}

function drillCurvesCacheKey(cmd, req) {
  const payload = { cmd: String(cmd || ""), req: canonicalizeJsonValue(req) };
  return `${payload.cmd}:${hashFNV1a64Hex(canonicalJsonString(payload))}`;
}

function drillCurvesCacheGet(key) {
  if (!DRILL_CURVES_CACHE.has(key)) return null;
  const hit = DRILL_CURVES_CACHE.get(key);
  // LRU: move to newest position.
  DRILL_CURVES_CACHE.delete(key);
  DRILL_CURVES_CACHE.set(key, hit);
  return hit;
}

function drillCurvesCacheSet(key, curves) {
  if (DRILL_CURVES_CACHE.has(key)) DRILL_CURVES_CACHE.delete(key);
  DRILL_CURVES_CACHE.set(key, curves);
  while (DRILL_CURVES_CACHE.size > DRILL_CURVES_CACHE_MAX) {
    const oldest = DRILL_CURVES_CACHE.keys().next();
    if (oldest.done) break;
    DRILL_CURVES_CACHE.delete(oldest.value);
  }
}

async function invokeDrilldownCached(cmd, req) {
  const key = drillCurvesCacheKey(cmd, req);
  const cached = drillCurvesCacheGet(key);
  if (cached) return { key, curves: cached, cacheHit: true, inflightHit: false };

  if (DRILL_CURVES_INFLIGHT.has(key)) {
    const curves = await DRILL_CURVES_INFLIGHT.get(key);
    return { key, curves, cacheHit: true, inflightHit: true };
  }

  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const promise = invoke(cmd, { req })
    .then((curves) => {
      drillCurvesCacheSet(key, curves);
      return curves;
    })
    .finally(() => {
      DRILL_CURVES_INFLIGHT.delete(key);
    });
  DRILL_CURVES_INFLIGHT.set(key, promise);
  const curves = await promise;
  return { key, curves, cacheHit: false, inflightHit: false };
}

function findTopRowByParamsKey(key) {
  const k = String(key || "").trim();
  if (!k) return null;
  const rowsFiltered = Array.isArray(TOP_ROWS_FILTERED) ? TOP_ROWS_FILTERED : [];
  for (const row of rowsFiltered) {
    if (paramsKey(row?.params) === k) return row;
  }
  const rowsAll = Array.isArray(TOP_ROWS) ? TOP_ROWS : [];
  for (const row of rowsAll) {
    if (paramsKey(row?.params) === k) return row;
  }
  return null;
}

function resolveDrillContext(params, opts) {
  const o = opts && typeof opts === "object" ? opts : {};
  const key = paramsKey(params);
  const row = (o.row && typeof o.row === "object") ? o.row : findTopRowByParamsKey(key);

  let metrics = o.metrics && typeof o.metrics === "object" ? o.metrics : null;
  if (!metrics && row?.metrics && typeof row.metrics === "object") metrics = row.metrics;
  if (!metrics && CURRENT_RUN?.res?.best_params && paramsKey(CURRENT_RUN.res.best_params) === key) {
    metrics = CURRENT_RUN.res.best_metrics && typeof CURRENT_RUN.res.best_metrics === "object"
      ? CURRENT_RUN.res.best_metrics
      : null;
  }

  const source = String(o.source || "").trim() || "manual";
  return {
    key,
    row,
    metrics,
    source,
    prefetch: !!o.prefetch,
    skipStability: !!o.skipStability,
  };
}

function formatDrillMetricsSummary(metrics) {
  const m = metrics && typeof metrics === "object" ? metrics : null;
  if (!m) return "";
  return `pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} expo=${fmt(m.exposure, 4)}`;
}

function formatDrillMoSummary(row) {
  const mo = row?.mo && typeof row.mo === "object" ? row.mo : null;
  if (!mo) return "";
  return `max=${fmt(mo.maximize_value, 6)} min=${fmt(mo.minimize_value, 6)} viol=${fmt(mo.violation, 6)}`;
}

function buildDrilldownInvocation(params, runReqOverride) {
  const runReq = runReqOverride || CURRENT_RUN?.req;
  const runDataId = String(runReq?.data_id || runReq?.dataId || "").trim();
  if (!runReq || !runDataId) return null;
  if (!params || !params.fast_ma_type || !params.slow_ma_type) return null;

  const bins = desiredDrillBins();
  const fastId = String(params.fast_ma_type || "").trim().toLowerCase();
  const slowId = String(params.slow_ma_type || "").trim().toLowerCase();
  const fastParams = (params.fast_ma_params && typeof params.fast_ma_params === "object")
    ? params.fast_ma_params
    : resolveRunMaParams(runReq, "fast", fastId);
  const slowParams = (params.slow_ma_params && typeof params.slow_ma_params === "object")
    ? params.slow_ma_params
    : resolveRunMaParams(runReq, "slow", slowId);
  const kindRaw = String(runReq.strategy_kind || runReq.strategyKind || "DoubleMa");
  const kind = normalizeStrategyKindRaw(kindRaw);

  const base = {
    data_id: runDataId,
    params: {
      fast_len: Number(params.fast_len || 0),
      slow_len: Number(params.slow_len || 0),
      fast_ma_type: String(params.fast_ma_type || ""),
      slow_ma_type: String(params.slow_ma_type || ""),
    },
    ma_source: String(runReq.ma_source || "close"),
    fast_ma_params: fastParams,
    slow_ma_params: slowParams,
    strategy: runReq.strategy || runReq.base_strategy || {},
    bins,
  };

  const cmd = isDoubleLikeKind(kind) ? "compute_double_ma_drilldown" : "compute_composite_cpu_drilldown";
  const req = kind === "CompositeCpu"
    ? {
      ...base,
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
      signal_mode: runReq.signal_mode || runReq.signalMode || "hard_gate",
    }
    : (kind === "CompositeMo"
      ? {
        ...base,
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        signal_mode: params.signal_mode || params.signalMode || runReq.signal_mode || runReq.signalMode || "hard_gate",
        strategy: params.strategy || base.strategy,
      }
      : {
        ...base,
        halftrend_filter: runReq.halftrend_filter || runReq.halftrendFilter || null,
      });

  return { runReq, kind, cmd, req };
}

async function renderDrillTvChart(curves, runReq, seq, context) {
  const meta = $("drillTvChartMeta");
  const req = runReq && typeof runReq === "object" ? runReq : null;
  const ctxMeta = context && typeof context === "object" ? context : {};
  const dataId = String(req?.data_id || req?.dataId || "").trim();
  if (!dataId) {
    clearDrillTvChart();
    return;
  }

  if (meta) meta.textContent = "Loading candles...";
  try {
    const bars = await fetchDrillPriceBars(dataId);
    if (seq !== DRILL_SEQ) return;
    if (!bars.length) {
      clearDrillTvChart("No valid OHLC bars available for chart rendering.");
      return;
    }

    DRILL_LAST_BARS = bars;
    DRILL_LAST_BARS_DATA_ID = dataId;

    const ctx = ensureDrillTvChart();
    if (!ctx) return;
    const candleData = bars.map((b) => ({
      time: b.time,
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
    }));
    if (!candleData.length) {
      clearDrillTvChart("No candle points generated from loaded bars.");
      return;
    }
    ctx.series.setData(candleData);
    const indicatorBars = normalizeIndicatorBarsForChart(curves?.indicator_bars);
    const fastData = [];
    const slowData = [];
    for (const b of indicatorBars) {
      if (Number.isFinite(b.fast)) fastData.push({ time: b.time, value: b.fast });
      if (Number.isFinite(b.slow)) slowData.push({ time: b.time, value: b.slow });
    }
    if (DRILL_TV_FAST_SERIES) DRILL_TV_FAST_SERIES.setData(fastData);
    if (DRILL_TV_SLOW_SERIES) DRILL_TV_SLOW_SERIES.setData(slowData);

    const compositeBars = normalizeCompositeIndicatorBarsForChart(curves?.composite_indicator_bars);
    const halftrendData = [];
    const maczData = [];
    for (const b of compositeBars) {
      if (Number.isFinite(b.halftrend)) halftrendData.push({ time: b.time, value: b.halftrend });
      if (Number.isFinite(b.macz)) {
        maczData.push({
          time: b.time,
          value: b.macz,
          color: b.macz >= 0 ? "rgba(16, 185, 129, 0.9)" : "rgba(244, 63, 94, 0.9)",
        });
      }
    }
    if (DRILL_TV_HALF_SERIES) DRILL_TV_HALF_SERIES.setData(halftrendData);
    if (DRILL_TV_MACZ_SERIES) DRILL_TV_MACZ_SERIES.setData(maczData);

    const markers = buildDrillTradeMarkers(curves, bars);
    DRILL_TV_TRADE_EVENTS_BY_TIME = buildDrillTradeEventIndex(curves, bars);
    renderTrades(curves, { bars });
    if (DRILL_TV_MARKERS && typeof DRILL_TV_MARKERS.setMarkers === "function") {
      // Marker rendering is viewport-windowed to reduce chart work on dense runs.
      DRILL_TV_ALL_MARKERS = markers;
      DRILL_TV_MARKER_LAST_KEY = "";
      scheduleDrillTvVisibleMarkerRefresh({ force: true });
    } else {
      DRILL_TV_MARKERS = createSeriesMarkers(ctx.series, []);
      DRILL_TV_ALL_MARKERS = markers;
      DRILL_TV_MARKER_LAST_KEY = "";
      scheduleDrillTvVisibleMarkerRefresh({ force: true });
    }

    // Seed the legend with the most recent candle so it isn't empty until you hover.
    if (DRILL_TV_LEGEND_EL && candleData.length) {
      const last = candleData[candleData.length - 1] || {};
      const time = chartTimeToUnixSec(last.time);
      if (time != null) {
        const lines = [formatUtcTimeFromSec(time)];
        const o = Number(last.open);
        const h = Number(last.high);
        const l = Number(last.low);
        const c = Number(last.close);
        if (Number.isFinite(o) && Number.isFinite(h) && Number.isFinite(l) && Number.isFinite(c)) {
          lines.push(`O ${fmtPrice(o)}  H ${fmtPrice(h)}  L ${fmtPrice(l)}  C ${fmtPrice(c)}`);
        }
        const fv = fastData.length ? Number(fastData[fastData.length - 1]?.value) : NaN;
        const sv = slowData.length ? Number(slowData[slowData.length - 1]?.value) : NaN;
        if (Number.isFinite(fv) || Number.isFinite(sv)) {
          const fStr = Number.isFinite(fv) ? fmtPrice(fv) : "-";
          const sStr = Number.isFinite(sv) ? fmtPrice(sv) : "-";
          lines.push(`fast ${fStr}  slow ${sStr}`);
        }
        const htv = halftrendData.length ? Number(halftrendData[halftrendData.length - 1]?.value) : NaN;
        const mzv = maczData.length ? Number(maczData[maczData.length - 1]?.value) : NaN;
        if (Number.isFinite(htv)) lines.push(`halftrend ${fmt(htv, 4)}`);
        if (Number.isFinite(mzv)) lines.push(`macz ${fmt(mzv, 6)}`);
        const ev = DRILL_TV_TRADE_EVENTS_BY_TIME && typeof DRILL_TV_TRADE_EVENTS_BY_TIME.get === "function"
          ? DRILL_TV_TRADE_EVENTS_BY_TIME.get(time)
          : null;
        const evText = formatDrillTradeEventSummary(ev);
        if (evText) lines.push(evText);
        const crosshairText = lines.join("\n");
        DRILL_TV_LEGEND_LAST_TEXT = crosshairText;
        DRILL_TV_LEGEND_EL.style.opacity = "0.85";
        DRILL_TV_LEGEND_EL.textContent = composeDrillTvLegendText(crosshairText, DRILL_TV_LEGEND_PINNED_TEXT);
      }
    }

    requestAnimationFrame(() => {
      if (!DRILL_TV_CHART) return;
      const rect = ctx.host.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        DRILL_TV_CHART.applyOptions({ width: Math.floor(rect.width), height: Math.floor(rect.height) });
      }
      DRILL_TV_CHART.timeScale().fitContent();
      scheduleDrillTvVisibleMarkerRefresh({ force: true });
    });

    if (meta) {
      const first = bars[0];
      const last = bars[bars.length - 1];
      const tradesN = Array.isArray(curves?.trades) ? curves.trades.length : 0;
      const indBarsN = indicatorBars.length;
      const compBarsN = compositeBars.length;
      const stats = curves?.trade_stats && typeof curves.trade_stats === "object" ? curves.trade_stats : null;
      const wr = Number(stats?.win_rate ?? NaN);
      const pf = Number(stats?.profit_factor ?? NaN);
      const tradeStatsPart = stats
        ? `closed=${Number(stats.closed_trades || 0)} wr=${Number.isFinite(wr) ? `${(wr * 100).toFixed(1)}%` : "-"} pf=${Number.isFinite(pf) ? (pf === Infinity ? "inf" : pf.toFixed(3)) : "-"}`
        : "";
      const metricPart = formatDrillMetricsSummary(ctxMeta.metrics);
      const moPart = formatDrillMoSummary(ctxMeta.row);
      const srcPart = ctxMeta.source ? `src=${ctxMeta.source}` : "";
      const cachePart = ctxMeta.cacheHit ? "cached" : "fresh";
      meta.textContent =
        `bars=${bars.length} idx=${first.fromIndex}..${last.toIndex} markers=${markers.length} trades=${tradesN} ind=${indBarsN} fast=${fastData.length} slow=${slowData.length}` +
        ` halftrend=${halftrendData.length} macz=${maczData.length}` +
        (compBarsN ? ` comp=${compBarsN}` : "") +
        (tradeStatsPart ? ` | ${tradeStatsPart}` : "") +
        (metricPart ? ` | ${metricPart}` : "") +
        (moPart ? ` | ${moPart}` : "") +
        (srcPart ? ` | ${srcPart}` : "") +
        ` | ${cachePart}`;
    }
  } catch (e) {
    if (seq !== DRILL_SEQ) return;
    if (meta) meta.textContent = `Chart load failed: ${String(e)}`;
  }
}

function drawMinMaxChart(canvas, minmax, opts) {
  const mm = Array.isArray(minmax) ? minmax : [];
  const bins = Math.floor(mm.length / 2);
  if (!canvas || bins <= 0) return;

  const height = Number(opts?.height || 160);
  canvas.width = bins;
  canvas.height = height;

  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, bins, height);

  let min = Infinity;
  let max = -Infinity;
  for (let i = 0; i < mm.length; i++) {
    const v = Number(mm[i]);
    if (!Number.isFinite(v)) continue;
    if (v < min) min = v;
    if (v > max) max = v;
  }
  if (!Number.isFinite(min) || !Number.isFinite(max)) return;
  if (min === max) {
    min -= 1;
    max += 1;
  }

  const pad = 6;
  const h = height;
  const scale = (h - 1 - 2 * pad) / (max - min);
  const yOf = (v) => pad + (max - v) * scale;

  if (Number.isFinite(opts?.baseline)) {
    const by = yOf(Number(opts.baseline));
    ctx.strokeStyle = "rgba(148, 163, 184, 0.45)";
    ctx.beginPath();
    ctx.moveTo(0, by + 0.5);
    ctx.lineTo(bins, by + 0.5);
    ctx.stroke();
  }

  ctx.strokeStyle = opts?.stroke || "rgba(52, 211, 153, 0.95)";
  ctx.lineWidth = 1;
  for (let x = 0; x < bins; x++) {
    const a = Number(mm[2 * x + 0]);
    const b = Number(mm[2 * x + 1]);
    if (!Number.isFinite(a) || !Number.isFinite(b)) continue;
    const y1 = yOf(a);
    const y2 = yOf(b);
    ctx.beginPath();
    ctx.moveTo(x + 0.5, y1);
    ctx.lineTo(x + 0.5, y2);
    ctx.stroke();
  }
}

function renderDrilldown(curves, params, context) {
  const c = curves || {};
  const eq = c.equity || {};
  const dd = c.drawdown || {};
  const ctxMeta = context && typeof context === "object" ? context : {};

  const bins = Number(eq.bins || 0);
  if (!bins) {
    setDrillVisible(false);
    return;
  }

  setDrillVisible(true);
  const p = params || {};
  const meta = $("drillMeta");
  if (meta) {
    const metricPart = formatDrillMetricsSummary(ctxMeta.metrics);
    const moPart = formatDrillMoSummary(ctxMeta.row);
    const srcPart = ctxMeta.source ? `src=${ctxMeta.source}` : "";
    meta.textContent =
      `${p.fast_ma_type || "?"}(${p.fast_len || "?"}) vs ${p.slow_ma_type || "?"}(${p.slow_len || "?"}) | ` +
      (metricPart ? `${metricPart} | ` : "") +
      (moPart ? `${moPart} | ` : "") +
      (srcPart ? `${srcPart} | ` : "") +
      `bins=${bins} start_t=${Number(eq.start_t || 0)} end_t=${Number(eq.end_t || 0)} (min/max per bin)`;
  }

  drawMinMaxChart($("equityCanvas"), eq.minmax, {
    stroke: "rgba(52, 211, 153, 0.95)",
    baseline: 1.0,
    height: 160,
  });
  drawMinMaxChart($("drawdownCanvas"), dd.minmax, {
    stroke: "rgba(251, 113, 133, 0.95)",
    baseline: 0.0,
    height: 160,
  });

  DRILL_LAST_CURVES = c;
  renderTrades(c, { bars: bestEffortCurrentDrillBars() });
}

function renderTrades(curves, opts) {
  const wrap = $("tradeStatsWrap");
  const statsEl = $("tradeStats");
  const body = $("tradeTableBody");
  if (!wrap || !statsEl || !body) return;

  const c = curves && typeof curves === "object" ? curves : {};
  const trades = Array.isArray(c?.trades) ? c.trades : [];
  const stats = c?.trade_stats && typeof c.trade_stats === "object" ? c.trade_stats : null;
  DRILL_LAST_CURVES = c;

  if (!trades.length && !stats) {
    wrap.style.display = "none";
    statsEl.textContent = "-";
    body.innerHTML = "";
    return;
  }

  wrap.style.display = "block";

  const priceBars = Array.isArray(opts?.bars) ? opts.bars : bestEffortCurrentDrillBars();
  const total = trades.length;
  const maxRows = Math.max(20, Math.floor(Number(DRILL_TRADES_TABLE_MAX_ROWS || 200)));

  // Maintain a sliding window so chart-clicked trades are visible.
  let start = Math.max(0, Math.floor(Number(DRILL_TRADES_RANGE_START || 0)));
  if (total <= maxRows) {
    start = 0;
  } else {
    const maxStart = Math.max(0, total - maxRows);
    start = Math.max(0, Math.min(maxStart, start));
    if (DRILL_SELECTED_TRADE_IDX >= 0 && DRILL_SELECTED_TRADE_IDX < total) {
      const endExcl = start + maxRows;
      if (DRILL_SELECTED_TRADE_IDX < start || DRILL_SELECTED_TRADE_IDX >= endExcl) {
        start = computeTradeTableStart(total, DRILL_SELECTED_TRADE_IDX);
      }
    } else if (start === 0) {
      start = maxStart;
    }
  }
  DRILL_TRADES_RANGE_START = start;

  if (stats) {
    const closed = Number(stats.closed_trades || 0);
    const open = Number(stats.open_trades || 0);
    const wins = Number(stats.wins || 0);
    const losses = Number(stats.losses || 0);
    const winRate = Number(stats.win_rate || 0);
    const avgWin = Number(stats.avg_win || 0);
    const avgLoss = Number(stats.avg_loss || 0);
    const pf = Number(stats.profit_factor);
    const exp = Number(stats.expectancy || 0);

    const pfStr = Number.isFinite(pf) ? (pf === Infinity ? "inf" : pf.toFixed(3)) : "-";
    const wrStr = Number.isFinite(winRate) ? `${(winRate * 100).toFixed(1)}%` : "-";
    const avgWinStr = Number.isFinite(avgWin) ? `${(avgWin * 100).toFixed(2)}%` : "-";
    const avgLossStr = Number.isFinite(avgLoss) ? `${(avgLoss * 100).toFixed(2)}%` : "-";
    const expStr = Number.isFinite(exp) ? `${(exp * 100).toFixed(2)}%` : "-";

    statsEl.textContent =
      `closed=${closed} open=${open} wins=${wins} losses=${losses} ` +
      `win_rate=${wrStr} avg_win=${avgWinStr} avg_loss=${avgLossStr} ` +
      `pf=${pfStr} expectancy=${expStr}`;
  } else {
    statsEl.textContent = `trades=${trades.length}`;
  }

  const shown = total ? trades.slice(start, Math.min(total, start + maxRows)) : [];
  const viewPart = total ? ` | view=${start + 1}..${start + shown.length}/${total}` : "";
  if (viewPart) statsEl.textContent = `${statsEl.textContent}${viewPart}`;

  body.innerHTML = "";
  for (let i = 0; i < shown.length; i++) {
    const t = shown[i] || {};
    const tradeIdx = start + i; // 0-based index into curves.trades
    const idx = tradeIdx + 1; // 1-based label
    const dirVal = Number(t.direction || 0);
    const dir = dirVal > 0 ? "Long" : dirVal < 0 ? "Short" : "Flat";
    const entry = Number(t.entry_t || 0);
    const barsN = Number(t.bars || 0);
    const pnl = Number(t.pnl || 0);
    const open = !!t.open;
    const exitRaw = Number(t.exit_t);
    const exit = open && (!Number.isFinite(exitRaw) || exitRaw <= 0) ? NaN : exitRaw;
    const state = open ? "open" : "closed";

    const pnlStr = Number.isFinite(pnl) ? `${(pnl * 100).toFixed(2)}%` : "-";

    const tr = document.createElement("tr");
    tr.dataset.tradeIdx = String(tradeIdx);
    if (tradeIdx === DRILL_SELECTED_TRADE_IDX) tr.classList.add("selected");

    const tdN = document.createElement("td");
    tdN.className = "left mono";
    tdN.textContent = String(idx);

    const tdDir = document.createElement("td");
    tdDir.className = "left mono";
    tdDir.textContent = dir;

    const tdEntry = document.createElement("td");
    tdEntry.className = "left mono";
    tdEntry.textContent = String(entry);
    const tdEntryTime = document.createElement("td");
    tdEntryTime.className = "left mono";
    tdEntryTime.textContent = "-";
    if (priceBars && priceBars.length) {
      const tt = mapTradeIndexToChartTime(entry, priceBars);
      if (tt != null) {
        tdEntry.title = formatUtcTimeFromSec(tt);
        tdEntryTime.textContent = formatUtcTimeShortFromSec(tt);
        tdEntryTime.title = formatUtcTimeFromSec(tt);
      }
    }

    const tdExit = document.createElement("td");
    tdExit.className = "left mono";
    tdExit.textContent = Number.isFinite(exit) ? String(exit) : "-";
    const tdExitTime = document.createElement("td");
    tdExitTime.className = "left mono";
    tdExitTime.textContent = "-";
    if (Number.isFinite(exit) && priceBars && priceBars.length) {
      const tt = mapTradeIndexToChartTime(exit, priceBars);
      if (tt != null) {
        tdExit.title = formatUtcTimeFromSec(tt);
        tdExitTime.textContent = formatUtcTimeShortFromSec(tt);
        tdExitTime.title = formatUtcTimeFromSec(tt);
      }
    }

    const tdBars = document.createElement("td");
    tdBars.className = "mono";
    tdBars.textContent = String(barsN);

    const tdPnl = document.createElement("td");
    tdPnl.className = "mono";
    tdPnl.textContent = pnlStr;
    if (Number.isFinite(pnl)) {
      if (pnl > 0) tdPnl.style.color = "rgba(52, 211, 153, 0.95)";
      else if (pnl < 0) tdPnl.style.color = "rgba(251, 113, 133, 0.95)";
    }

    const tdState = document.createElement("td");
    tdState.className = "left mono";
    tdState.textContent = state;

    tr.appendChild(tdN);
    tr.appendChild(tdDir);
    tr.appendChild(tdEntry);
    tr.appendChild(tdEntryTime);
    tr.appendChild(tdExit);
    tr.appendChild(tdExitTime);
    tr.appendChild(tdBars);
    tr.appendChild(tdPnl);
    tr.appendChild(tdState);

    tr.addEventListener("click", () => {
      selectDrillTradeByIndex(tradeIdx, { scroll: false });
    });

    body.appendChild(tr);
  }
}

function resolveRunMaParams(runReq, side, maId) {
  const asObj = (x) => (x && typeof x === "object" ? x : null);
  const legacyAlma = {
    offset: num(runReq?.alma_offset ?? runReq?.almaOffset, 0.85),
    sigma: num(runReq?.alma_sigma ?? runReq?.almaSigma, 6.0),
  };
  const id = String(maId || "").trim().toLowerCase();
  const direct = side === "fast"
    ? (asObj(runReq.fast_ma_params) || asObj(runReq.fastMaParams))
    : (asObj(runReq.slow_ma_params) || asObj(runReq.slowMaParams));
  if (direct) return direct;
  if (id !== "alma") return null;
  const hasLegacy = runReq?.alma_offset != null || runReq?.almaOffset != null || runReq?.alma_sigma != null || runReq?.almaSigma != null;
  return hasLegacy ? legacyAlma : null;
}

async function prefetchDrillRows(runReq, selectedKey, prefetchSeq) {
  const req = runReq && typeof runReq === "object" ? runReq : null;
  if (!req) return;
  const rowsBase = (Array.isArray(TOP_ROWS_FILTERED) && TOP_ROWS_FILTERED.length)
    ? TOP_ROWS_FILTERED
    : (Array.isArray(TOP_ROWS) ? TOP_ROWS : []);
  if (!rowsBase.length) return;

  const selected = String(selectedKey || "").trim();
  const unique = [];
  const seen = new Set();
  for (const row of rowsBase) {
    const key = paramsKey(row?.params);
    if (!key || seen.has(key) || key === selected) continue;
    seen.add(key);
    unique.push(row);
    if (unique.length >= DRILL_PREFETCH_TOP_N) break;
  }

  for (const row of unique) {
    if (prefetchSeq !== DRILL_PREFETCH_SEQ) return;
    try {
      await requestDrilldown(row?.params, req, {
        prefetch: true,
        source: "prefetch",
        row,
        metrics: row?.metrics || null,
        skipStability: true,
      });
    } catch {
      // ignore prefetch failures; foreground drilldown still works
    }
  }
}

function scheduleDrillPrefetch(runReq, selectedKey) {
  const req = runReq && typeof runReq === "object" ? runReq : null;
  if (!req) return;
  const prefetchSeq = ++DRILL_PREFETCH_SEQ;
  setTimeout(() => {
    if (prefetchSeq !== DRILL_PREFETCH_SEQ) return;
    void prefetchDrillRows(req, selectedKey, prefetchSeq);
  }, 0);
}

async function requestDrilldown(params, runReqOverride, opts) {
  const invocation = buildDrilldownInvocation(params, runReqOverride);
  if (!invocation) return;

  const { runReq, cmd, req } = invocation;
  const context = resolveDrillContext(params, opts);

  if (context.prefetch) {
    try {
      await invokeDrilldownCached(cmd, req);
    } catch {
      // ignore background prefetch failures
    }
    return;
  }

  const seq = ++DRILL_SEQ;
  setDrillVisible(true);
  clearStabilityUi();
  clearTradeSelection();
  DRILL_TRADES_RANGE_START = 0;
  DRILL_LAST_CURVES = null;
  DRILL_TV_TRADE_EVENTS_BY_TIME = null;
  const sourceTag = context.source && context.source !== "manual" ? ` (${context.source})` : "";
  setDrillStatus(`Computing drilldown…${sourceTag}`, "muted");

  try {
    const out = await invokeDrilldownCached(cmd, req);
    if (seq !== DRILL_SEQ) return;
    const renderContext = { ...context, cacheHit: out.cacheHit, requestKey: out.key };
    setDrillStatus(out.cacheHit ? "Done (cached)." : "Done.", out.cacheHit ? "muted" : "ok");
    renderDrilldown(out.curves, params, renderContext);
    void renderDrillTvChart(out.curves, runReq, seq, renderContext);
    if (!context.skipStability) requestLocalStability(params, runReq);
  } catch (e) {
    if (seq !== DRILL_SEQ) return;
    setDrillStatus(String(e), "danger");
    clearStabilityUi();
    clearDrillTvChart();
  }
}

function renderStability(res) {
  const wrap = $("stabilityWrap");
  const meta = $("stabilityMeta");
  const reject = $("rejectMeta");
  const tableWrap = $("stabilityTableWrap");
  const tbody = $("stabilityTableBody");
  if (!wrap || !meta || !tableWrap || !tbody) return;

  const obj = res && typeof res === "object" ? res : {};
  const base = obj.base && typeof obj.base === "object" ? obj.base : {};
  const pts = Array.isArray(obj.points) ? obj.points : [];
  const sum = obj.summary && typeof obj.summary === "object" ? obj.summary : {};

  setStabilityVisible(true);
  if (!pts.length) {
    setStabilityStatus("No stability data.", "muted");
    meta.textContent = "-";
    if (reject) reject.textContent = "-";
    tableWrap.style.display = "none";
    tbody.innerHTML = "";
    return;
  }

  setStabilityStatus("Done.", "ok");

  const objective = String(obj.objective || "").trim() || "?";
  const radius = Number(obj.radius || 0);
  const step = Number(obj.step || 0);
  const startT = Number(obj.start_t || 0);
  const baseScore = Number(sum.base || 0);
  const minScore = Number(sum.min || 0);
  const medScore = Number(sum.median || 0);
  const maxScore = Number(sum.max || 0);
  const peakDelta = baseScore - medScore;
  meta.textContent =
    `objective=${objective} radius=${radius} step=${step} pts=${pts.length} start_t=${startT} | ` +
    `score: base=${fmt(baseScore, 6)} median=${fmt(medScore, 6)} min=${fmt(minScore, 6)} max=${fmt(maxScore, 6)} peakΔ=${fmt(peakDelta, 6)}`;

  tableWrap.style.display = "block";
  tbody.innerHTML = "";

  const bf = Number(base.fast_len || 0);
  const bs = Number(base.slow_len || 0);
  const basePoint = pts.find((p) => Number(p?.fast_len || 0) === bf && Number(p?.slow_len || 0) === bs);
  renderRejectionChecklist(basePoint?.metrics || null);
  const sorted = [...pts].sort((a, b) => Number(b?.score || 0) - Number(a?.score || 0));
  for (const p of sorted) {
    const m = p?.metrics && typeof p.metrics === "object" ? p.metrics : {};
    const tr = document.createElement("tr");
    const fast = Number(p?.fast_len || 0);
    const slow = Number(p?.slow_len || 0);

    const tdFast = document.createElement("td");
    tdFast.className = "left mono";
    tdFast.textContent = String(fast);
    const tdSlow = document.createElement("td");
    tdSlow.className = "left mono";
    tdSlow.textContent = String(slow);

    const tdScore = document.createElement("td");
    tdScore.className = "mono";
    tdScore.textContent = fmt(p?.score, 6);
    if (Number.isFinite(p?.score)) {
      if (Number(p.score) >= baseScore) tdScore.style.color = "rgba(52, 211, 153, 0.95)";
      else tdScore.style.color = "rgba(148, 163, 184, 0.9)";
    }

    const tdPnl = document.createElement("td");
    tdPnl.className = "mono";
    tdPnl.textContent = fmt(m.pnl, 6);
    const tdSharpe = document.createElement("td");
    tdSharpe.className = "mono";
    tdSharpe.textContent = fmt(m.sharpe, 6);
    const tdDd = document.createElement("td");
    tdDd.className = "mono";
    tdDd.textContent = fmt(m.max_dd, 6);
    const tdTrades = document.createElement("td");
    tdTrades.className = "mono";
    tdTrades.textContent = String(Number(m.trades || 0));

    tr.appendChild(tdFast);
    tr.appendChild(tdSlow);
    tr.appendChild(tdScore);
    tr.appendChild(tdPnl);
    tr.appendChild(tdSharpe);
    tr.appendChild(tdDd);
    tr.appendChild(tdTrades);

    if (fast === bf && slow === bs) {
      tr.style.background = "rgba(56, 189, 248, 0.08)";
      tr.style.borderLeft = "3px solid rgba(56, 189, 248, 0.9)";
    }

    tbody.appendChild(tr);
  }
}

function renderRejectionChecklist(metrics) {
  const el = $("rejectMeta");
  if (!el) return;

  const m = metrics && typeof metrics === "object" ? metrics : {};
  const filters = readTopFiltersFromUi();

  const pnl = Number(m.pnl);
  const sharpe = Number(m.sharpe);
  const dd = Number(m.max_dd);
  const trades = Number(m.trades);
  const expo = Number(m.exposure);

  let any = false;
  let allOk = true;

  const spans = [];
  function add(ok, text) {
    any = true;
    if (!ok) allOk = false;
    const span = document.createElement("span");
    span.className = "badge " + (ok ? "on" : "off");
    span.textContent = text;
    spans.push(span);
  }

  if (filters.minPnl != null) {
    const ok = Number.isFinite(pnl) && pnl >= filters.minPnl;
    add(ok, `PnL≥${fmt(filters.minPnl, 6)} (${fmt(pnl, 6)})`);
  }
  if (filters.minSharpe != null) {
    const ok = Number.isFinite(sharpe) && sharpe >= filters.minSharpe;
    add(ok, `Sharpe≥${fmt(filters.minSharpe, 6)} (${fmt(sharpe, 6)})`);
  }
  if (filters.maxDd != null) {
    const ok = Number.isFinite(dd) && dd <= filters.maxDd;
    add(ok, `MaxDD≤${fmt(filters.maxDd, 6)} (${fmt(dd, 6)})`);
  }
  if (filters.minTrades != null) {
    const ok = Number.isFinite(trades) && trades >= filters.minTrades;
    add(ok, `Trades≥${String(filters.minTrades)} (${String(Number.isFinite(trades) ? trades : "-")})`);
  }
  if (filters.minExposure != null) {
    const ok = Number.isFinite(expo) && expo >= filters.minExposure;
    add(ok, `Expo≥${fmt(filters.minExposure, 6)} (${fmt(expo, 6)})`);
  }

  if (!any) {
    el.textContent = "No Top filters set (nothing to validate).";
    return;
  }

  el.innerHTML = "";
  const head = document.createElement("span");
  head.className = "badge " + (allOk ? "on" : "off");
  head.textContent = allOk ? "PASS" : "FAIL";
  el.appendChild(head);
  for (const s of spans) {
    el.appendChild(document.createTextNode(" "));
    el.appendChild(s);
  }
}

async function requestLocalStability(params, runReqOverride) {
  const invoke = tauriInvoke();
  if (!invoke) return;

  const runReq = runReqOverride || CURRENT_RUN?.req;
  if (!runReq || !runReq.data_id) return;
  if (!params || !params.fast_ma_type || !params.slow_ma_type) return;

  const seq = ++STABILITY_SEQ;
  setStabilityVisible(true);
  setStabilityStatus("Computing stability…", "muted");
  const meta = $("stabilityMeta");
  if (meta) meta.textContent = "-";
  const reject = $("rejectMeta");
  if (reject) reject.textContent = "-";
  const tableWrap = $("stabilityTableWrap");
  if (tableWrap) tableWrap.style.display = "none";

  const fastId = String(params.fast_ma_type || "").trim().toLowerCase();
  const slowId = String(params.slow_ma_type || "").trim().toLowerCase();
  const fastParams = (params.fast_ma_params && typeof params.fast_ma_params === "object")
    ? params.fast_ma_params
    : resolveRunMaParams(runReq, "fast", fastId);
  const slowParams = (params.slow_ma_params && typeof params.slow_ma_params === "object")
    ? params.slow_ma_params
    : resolveRunMaParams(runReq, "slow", slowId);

  const kindRaw = String(runReq.strategy_kind || runReq.strategyKind || "DoubleMa");
  const kind = normalizeStrategyKindRaw(kindRaw);
  const objective = String(runReq.objective || "Sharpe");

  const base = {
    data_id: runReq.data_id,
    params: {
      fast_len: Number(params.fast_len || 0),
      slow_len: Number(params.slow_len || 0),
      fast_ma_type: String(params.fast_ma_type || ""),
      slow_ma_type: String(params.slow_ma_type || ""),
    },
    ma_source: String(runReq.ma_source || "close"),
    fast_ma_params: fastParams,
    slow_ma_params: slowParams,
    strategy: runReq.strategy || runReq.base_strategy || {},
    objective,
    radius: 1,
    step: 1,
  };

  const cmd = isDoubleLikeKind(kind) ? "compute_double_ma_local_stability" : "compute_composite_cpu_local_stability";
  const req = kind === "CompositeCpu"
    ? {
      ...base,
      halftrend: runReq.halftrend,
      macz: runReq.macz || {},
      ott: runReq.ott || {},
    }
    : (kind === "CompositeMo"
      ? {
        ...base,
        halftrend: params.halftrend,
        macz: params.macz || {},
        ott: params.ott || {},
        strategy: params.strategy || base.strategy,
      }
      : {
        ...base,
        halftrend_filter: runReq.halftrend_filter || runReq.halftrendFilter || null,
      });

  let cacheKey = "";
  try { cacheKey = `${cmd}:${JSON.stringify(req)}`; } catch { cacheKey = `${cmd}:${Date.now()}`; }
  const cached = cacheKey ? STABILITY_CACHE.get(cacheKey) : null;
  if (cached) {
    if (seq !== STABILITY_SEQ) return;
    renderStability(cached);
    return;
  }

  try {
    const out = await invoke(cmd, { req });
    if (seq !== STABILITY_SEQ) return;
    if (cacheKey) STABILITY_CACHE.set(cacheKey, out);
    renderStability(out);
  } catch (e) {
    if (seq !== STABILITY_SEQ) return;
    setStabilityStatus(String(e), "danger");
  }
}

function clamp01(x) {
  return Math.max(0, Math.min(1, x));
}

function lerp(a, b, t) {
  return a + (b - a) * t;
}

function hexToRgb(hex) {
  const h = String(hex || "").trim().replace("#", "");
  const raw = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  const i = Number.parseInt(raw || "0", 16);
  return { r: (i >> 16) & 255, g: (i >> 8) & 255, b: i & 255 };
}

function makeHeatmapInterpolator(stops) {
  const S = (Array.isArray(stops) ? stops : [])
    .map((s) => ({ t: clamp01(Number(s?.t ?? 0)), rgb: hexToRgb(s?.color || "#000000") }))
    .sort((a, b) => a.t - b.t);
  return (t) => {
    const x = clamp01(Number(t ?? 0));
    for (let i = 0; i < S.length - 1; i++) {
      const a = S[i];
      const b = S[i + 1];
      if (x >= a.t && x <= b.t) {
        const u = (x - a.t) / Math.max(1e-12, (b.t - a.t));
        return {
          r: Math.round(lerp(a.rgb.r, b.rgb.r, u)),
          g: Math.round(lerp(a.rgb.g, b.rgb.g, u)),
          b: Math.round(lerp(a.rgb.b, b.rgb.b, u)),
        };
      }
    }
    const first = S[0]?.rgb || { r: 0, g: 0, b: 0 };
    const last = S[S.length - 1]?.rgb || first;
    return x < (S[0]?.t ?? 0) ? first : last;
  };
}

const HEATMAP_COLORMAPS = {
  cividis: makeHeatmapInterpolator([
    { t: 0.0, color: "#00224e" },
    { t: 0.25, color: "#2b3f66" },
    { t: 0.5, color: "#665c66" },
    { t: 0.75, color: "#a38f49" },
    { t: 1.0, color: "#ffd43b" },
  ]),
  viridis: makeHeatmapInterpolator([
    { t: 0.0, color: "#440154" },
    { t: 0.25, color: "#3b528b" },
    { t: 0.5, color: "#21918c" },
    { t: 0.75, color: "#5ec962" },
    { t: 1.0, color: "#fde725" },
  ]),
  blueOrange: makeHeatmapInterpolator([
    { t: 0.0, color: "#2b8cbe" },
    { t: 0.5, color: "#ffffff" },
    { t: 1.0, color: "#e34a33" },
  ]),
  redGreen: makeHeatmapInterpolator([
    { t: 0.0, color: "#b2182b" },
    { t: 0.5, color: "#f7f7f7" },
    { t: 1.0, color: "#1a9850" },
  ]),
};

const HEATMAP_LUT_CACHE = new Map();
function heatmapGetLut(name, size) {
  const n = String(name || "cividis");
  const s = Math.max(2, Math.floor(Number(size || 512)));
  const key = `${n}:${s}`;
  const cached = HEATMAP_LUT_CACHE.get(key);
  if (cached) return cached;

  const interp = HEATMAP_COLORMAPS[n] || HEATMAP_COLORMAPS.cividis;
  const lut = new Uint8Array(s * 4);
  for (let i = 0; i < s; i++) {
    const t = i / (s - 1);
    const { r, g, b } = interp(t);
    lut[i * 4 + 0] = r;
    lut[i * 4 + 1] = g;
    lut[i * 4 + 2] = b;
    lut[i * 4 + 3] = 255;
  }
  HEATMAP_LUT_CACHE.set(key, lut);
  return lut;
}

function quantileSorted(sorted, q) {
  const a = Array.isArray(sorted) ? sorted : [];
  if (!a.length) return NaN;
  const qq = clamp01(Number(q ?? 0));
  const idx = (a.length - 1) * qq;
  const lo = Math.floor(idx);
  const hi = Math.ceil(idx);
  if (lo === hi) return a[lo];
  return a[lo] + (a[hi] - a[lo]) * (idx - lo);
}

function heatmapBinPeriod(v, vMin, vMax, bins) {
  const b = Math.max(1, Math.floor(Number(bins || 0)));
  if (b <= 1) return 0;
  const minV = Math.floor(Number(vMin || 0));
  const maxV = Math.floor(Number(vMax || 0));
  const denom = Math.max(0, maxV - minV);
  if (denom === 0) return 0;
  const x = Math.max(0, Math.floor(Number(v || 0)) - minV);
  return Math.min(b - 1, Math.floor((x * (b - 1)) / denom));
}

function heatmapBinRange(bin, vMin, vMax, bins) {
  const b = Math.max(1, Math.floor(Number(bins || 0)));
  const minV = Math.floor(Number(vMin || 0));
  const maxV = Math.floor(Number(vMax || 0));
  const denom = Math.max(0, maxV - minV);
  if (b <= 1 || denom === 0) {
    const v = denom === 0 ? minV : maxV;
    return { lo: v, hi: v, center: v };
  }

  const bb = Math.max(0, Math.min(b - 1, Math.floor(Number(bin || 0))));
  const loX = Math.ceil((bb * denom) / (b - 1));
  const hiX = Math.ceil(((bb + 1) * denom) / (b - 1)) - 1;
  const lo = minV + Math.max(0, Math.min(denom, loX));
  const hi = minV + Math.max(0, Math.min(denom, Math.max(loX, hiX)));
  const center = Math.round((lo + hi) / 2);
  return { lo, hi, center };
}

function heatmapPrefsFromUi() {
  const color = String($("heatmapColor")?.value || "cividis").trim() || "cividis";
  const range = String($("heatmapRange")?.value || "q2_98").trim() || "q2_98";
  const showTop = String($("heatmapShowTop")?.value || "true").trim() !== "false";
  const showCrosshair = String($("heatmapShowCrosshair")?.value || "true").trim() !== "false";
  return { color, range, showTop, showCrosshair };
}

function formatBinRange(r) {
  if (!r || !Number.isFinite(Number(r.lo)) || !Number.isFinite(Number(r.hi))) return "?";
  if (r.lo === r.hi) return String(r.lo);
  return `${r.lo}..${r.hi}`;
}

function renderHeatmapOverlay() {
  const st = HEATMAP_STATE;
  if (!st || !st.overlay) return;
  const canvas = st.overlay;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const w = canvas.width;
  const h = canvas.height;
  const cell = Math.max(1, Number(st.cellPx || 1));
  const half = cell * 0.5;
  ctx.clearRect(0, 0, w, h);

  if (st.showTop && Array.isArray(st.topPoints)) {
    const maxMarkers = 200;
    let n = 0;
    for (const p of st.topPoints) {
      const x = p.sBin * cell + half;
      const y = p.fBin * cell + half;
      const isBest = p.rank === 1;
      ctx.strokeStyle = isBest ? "rgba(250, 204, 21, 0.95)" : "rgba(255, 255, 255, 0.85)";
      ctx.lineWidth = isBest
        ? Math.max(1.5, Math.min(3, cell * 0.35))
        : Math.max(1, Math.min(2, cell * 0.22));
      ctx.beginPath();
      ctx.arc(x, y, isBest ? Math.max(2, cell * 0.32) : Math.max(1.5, cell * 0.24), 0, Math.PI * 2);
      ctx.stroke();
      n++;
      if (n >= maxMarkers) break;
    }
  }

  if (HEATMAP_SELECTED && HEATMAP_SELECTED.idx != null) {
    const x = HEATMAP_SELECTED.sBin * cell;
    const y = HEATMAP_SELECTED.fBin * cell;
    ctx.strokeStyle = "rgba(52, 211, 153, 0.95)";
    ctx.lineWidth = Math.max(1, Math.min(2.5, cell * 0.22));
    ctx.strokeRect(x + 0.5, y + 0.5, Math.max(1, cell - 1), Math.max(1, cell - 1));
  }

  if (st.showCrosshair && HEATMAP_HOVER && HEATMAP_HOVER.idx != null) {
    const x = HEATMAP_HOVER.sBin * cell;
    const y = HEATMAP_HOVER.fBin * cell;
    ctx.save();
    ctx.strokeStyle = "rgba(255, 255, 255, 0.55)";
    ctx.lineWidth = 1;
    ctx.setLineDash([Math.max(4, cell * 0.8), Math.max(3, cell * 0.6)]);
    ctx.beginPath();
    ctx.moveTo(0, y + half);
    ctx.lineTo(w, y + half);
    ctx.moveTo(x + half, 0);
    ctx.lineTo(x + half, h);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.strokeStyle = "rgba(255, 255, 255, 0.95)";
    ctx.lineWidth = Math.max(1, Math.min(2, cell * 0.2));
    ctx.strokeRect(x + 0.5, y + 0.5, Math.max(1, cell - 1), Math.max(1, cell - 1));
    ctx.restore();
  }
}

function syncHeatmapSelectionToDrillKey() {
  const st = HEATMAP_STATE;
  if (!st || !st.keyToCell) return;
  const idx = st.keyToCell.get(DRILL_SELECTED_KEY);
  if (typeof idx === "number") {
    const fBin = Math.floor(idx / st.binsSlow);
    const sBin = idx - fBin * st.binsSlow;
    HEATMAP_SELECTED = { fBin, sBin, idx, key: DRILL_SELECTED_KEY };
  } else {
    HEATMAP_SELECTED = null;
  }
  renderHeatmapOverlay();
}

function exportHeatmapPng() {
  const base = $("heatmapCanvas");
  const overlay = $("heatmapOverlay");
  if (!base || !overlay) return;

  const w = base.width;
  const h = base.height;
  if (!w || !h) return;
  const tmp = document.createElement("canvas");
  tmp.width = w;
  tmp.height = h;
  const ctx = tmp.getContext("2d");
  if (!ctx) return;
  ctx.drawImage(base, 0, 0);
  ctx.drawImage(overlay, 0, 0);

  const url = tmp.toDataURL("image/png");
  const obj = String(CURRENT_RUN?.req?.objective || "heatmap").toLowerCase();
  const ts = CURRENT_RUN?.ts ? String(CURRENT_RUN.ts).replace(/[^\w-]+/g, "-") : "";
  const a = document.createElement("a");
  a.href = url;
  a.download = `heatmap_${obj}${ts ? "_" + ts : ""}.png`;
  a.click();
}

function renderHeatmap(res, req) {
  const wrap = $("heatmapWrap");
  const canvas = $("heatmapCanvas");
  const overlay = $("heatmapOverlay");
  const meta = $("heatmapMeta");
  const stack = $("heatmapCanvasStack");
  const tooltip = $("heatmapTooltip");
  const legend = $("heatmapLegend");
  const legMin = $("heatmapLegendMin");
  const legMid = $("heatmapLegendMid");
  const legMax = $("heatmapLegendMax");
  if (!wrap || !canvas || !overlay || !meta || !stack) return;
  const hint = $("heatmapEmptyHint");
  const kind = kindFromReq(req);
  if (!isDoubleLikeKind(kind)) {
    HEATMAP_STATE = null;
    HEATMAP_HOVER = null;
    HEATMAP_SELECTED = null;
    wrap.style.display = "none";
    if (hint) {
      hint.textContent = "Heatmap is available for Double-MA-family strategies only.";
    }
    setHintVisible("heatmapEmptyHint", true);
    return;
  }

  const hm = res?.heatmap;
  if (!hm || !Array.isArray(hm.values) || !hm.bins_fast || !hm.bins_slow) {
    HEATMAP_STATE = null;
    HEATMAP_HOVER = null;
    HEATMAP_SELECTED = null;
    wrap.style.display = "none";
    if (hint) {
      hint.textContent = CURRENT_RUN?.res
        ? "No heatmap in this result (set Heatmap bins > 0, then rerun)."
        : "Run an optimization with Heatmap bins > 0 to generate a heatmap.";
    }
    setHintVisible("heatmapEmptyHint", true);
    return;
  }

  const rawBinsFast = Number(hm.bins_fast);
  const rawBinsSlow = Number(hm.bins_slow);
  const rawValues = hm.values;
  if (!rawBinsFast || !rawBinsSlow || rawValues.length !== rawBinsFast * rawBinsSlow) {
    HEATMAP_STATE = null;
    HEATMAP_HOVER = null;
    HEATMAP_SELECTED = null;
    wrap.style.display = "none";
    if (hint) {
      hint.textContent = "Heatmap data is missing or malformed for this run.";
    }
    setHintVisible("heatmapEmptyHint", true);
    return;
  }

  const prefs = heatmapPrefsFromUi();
  const len = rawBinsFast * rawBinsSlow;
  const values = new Float64Array(len);
  const finite = [];

  let min = Infinity;
  let max = -Infinity;
  for (let i = 0; i < len; i++) {
    const v = rawValues[i];
    const n = typeof v === "number" && Number.isFinite(v) ? v : NaN;
    values[i] = n;
    if (Number.isFinite(n)) {
      finite.push(n);
      if (n < min) min = n;
      if (n > max) max = n;
    }
  }

  if (!Number.isFinite(min) || !Number.isFinite(max) || !finite.length) {
    HEATMAP_STATE = null;
    HEATMAP_HOVER = null;
    HEATMAP_SELECTED = null;
    wrap.style.display = "none";
    if (hint) {
      hint.textContent = "Heatmap values are not finite for this run.";
    }
    setHintVisible("heatmapEmptyHint", true);
    return;
  }

  let lo = min;
  let hi = max;
  if (prefs.range === "q2_98" && finite.length >= 8) {
    const sorted = [...finite].sort((a, b) => a - b);
    const qlo = quantileSorted(sorted, 0.02);
    const qhi = quantileSorted(sorted, 0.98);
    if (Number.isFinite(qlo) && Number.isFinite(qhi) && qlo < qhi) {
      lo = qlo;
      hi = qhi;
    }
  }
  if (lo === hi) {
    const eps = Math.max(1, Math.abs(hi)) * 1e-6;
    lo -= eps;
    hi += eps;
  }

  const rowHasData = new Array(rawBinsFast).fill(false);
  const colHasData = new Array(rawBinsSlow).fill(false);
  for (let idx = 0; idx < len; idx++) {
    const v = values[idx];
    if (!Number.isFinite(v)) continue;
    const fRaw = Math.floor(idx / rawBinsSlow);
    const sRaw = idx - fRaw * rawBinsSlow;
    rowHasData[fRaw] = true;
    colHasData[sRaw] = true;
  }

  const displayFastToRaw = [];
  const displaySlowToRaw = [];
  const rawFastToDisplay = new Int32Array(rawBinsFast);
  const rawSlowToDisplay = new Int32Array(rawBinsSlow);
  rawFastToDisplay.fill(-1);
  rawSlowToDisplay.fill(-1);

  for (let fRaw = 0; fRaw < rawBinsFast; fRaw++) {
    if (!rowHasData[fRaw]) continue;
    rawFastToDisplay[fRaw] = displayFastToRaw.length;
    displayFastToRaw.push(fRaw);
  }
  for (let sRaw = 0; sRaw < rawBinsSlow; sRaw++) {
    if (!colHasData[sRaw]) continue;
    rawSlowToDisplay[sRaw] = displaySlowToRaw.length;
    displaySlowToRaw.push(sRaw);
  }

  if (!displayFastToRaw.length || !displaySlowToRaw.length) {
    HEATMAP_STATE = null;
    HEATMAP_HOVER = null;
    HEATMAP_SELECTED = null;
    wrap.style.display = "none";
    if (hint) {
      hint.textContent = "Heatmap bins are empty for this run.";
    }
    setHintVisible("heatmapEmptyHint", true);
    return;
  }

  const binsFast = displayFastToRaw.length;
  const binsSlow = displaySlowToRaw.length;
  const compacted = binsFast !== rawBinsFast || binsSlow !== rawBinsSlow;
  const displayLen = binsFast * binsSlow;
  const displayValues = new Float64Array(displayLen);
  for (let i = 0; i < displayLen; i++) displayValues[i] = NaN;
  for (let f = 0; f < binsFast; f++) {
    const fRaw = displayFastToRaw[f];
    const srcOff = fRaw * rawBinsSlow;
    const dstOff = f * binsSlow;
    for (let s = 0; s < binsSlow; s++) {
      displayValues[dstOff + s] = values[srcOff + displaySlowToRaw[s]];
    }
  }

  wrap.style.display = "block";
  setHintVisible("heatmapEmptyHint", false);
  const obj = String(req?.objective || req?.objectives?.maximize || "Sharpe");
  const clipNote = prefs.range === "q2_98" ? " (clip 2–98%)" : "";
  const compactNote = compacted ? ` | compact=${binsFast}x${binsSlow} (raw ${rawBinsFast}x${rawBinsSlow})` : "";
  HEATMAP_META_BASE = `Objective heatmap (${obj}) | fast=${hm.fast_min}..${hm.fast_max} slow=${hm.slow_min}..${hm.slow_max} | score=${fmt(lo, 4)}..${fmt(hi, 4)}${clipNote}${compactNote}`;
  meta.textContent = HEATMAP_META_BASE;

  const maxBins = Math.max(1, binsFast, binsSlow);
  let cellPx = Math.floor(1200 / maxBins);
  if (!Number.isFinite(cellPx) || cellPx < 1) cellPx = 1;
  if (cellPx > 8) cellPx = 8;
  const capByDim = Math.max(1, Math.floor(3072 / maxBins));
  cellPx = Math.max(1, Math.min(cellPx, capByDim));

  canvas.width = binsSlow * cellPx;
  canvas.height = binsFast * cellPx;
  overlay.width = binsSlow * cellPx;
  overlay.height = binsFast * cellPx;
  const ctx = canvas.getContext("2d");
  const heatSrc = document.createElement("canvas");
  heatSrc.width = binsSlow;
  heatSrc.height = binsFast;
  const srcCtx = heatSrc.getContext("2d");
  if (!ctx || !srcCtx) return;
  const img = srcCtx.createImageData(binsSlow, binsFast);
  const lut = heatmapGetLut(prefs.color, 512);
  const lutSize = Math.max(1, (lut.length / 4) | 0);
  for (let idx = 0; idx < displayLen; idx++) {
    const v = displayValues[idx];
    const px = idx * 4;
    if (!Number.isFinite(v)) {
      img.data[px + 3] = 0; // transparent; show canvas background
      continue;
    }
    const t = clamp01((v - lo) / Math.max(1e-12, (hi - lo)));
    const i = Math.min(lutSize - 1, Math.max(0, Math.round(t * (lutSize - 1))));
    img.data[px + 0] = lut[i * 4 + 0];
    img.data[px + 1] = lut[i * 4 + 1];
    img.data[px + 2] = lut[i * 4 + 2];
    img.data[px + 3] = 255;
  }
  srcCtx.putImageData(img, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(heatSrc, 0, 0, binsSlow, binsFast, 0, 0, canvas.width, canvas.height);

  const fastBins = new Array(binsFast);
  const slowBins = new Array(binsSlow);
  for (let f = 0; f < binsFast; f++) fastBins[f] = heatmapBinRange(displayFastToRaw[f], hm.fast_min, hm.fast_max, rawBinsFast);
  for (let s = 0; s < binsSlow; s++) slowBins[s] = heatmapBinRange(displaySlowToRaw[s], hm.slow_min, hm.slow_max, rawBinsSlow);

  const topRows = Array.isArray(TOP_ROWS) && TOP_ROWS.length ? TOP_ROWS : collectTopRows(res);
  const maxTopRows = 2000;
  const rowsForHeatmap = topRows.length > maxTopRows ? topRows.slice(0, maxTopRows) : topRows;
  const topPoints = [];
  const topByCell = new Map();
  const keyToCell = new Map();
  for (const row of rowsForHeatmap) {
    const p = row?.params && typeof row.params === "object" ? row.params : {};
    const key = paramsKey(p);
    const fLen = Number(p.fast_len || 0);
    const sLen = Number(p.slow_len || 0);
    if (!Number.isFinite(fLen) || !Number.isFinite(sLen) || fLen <= 0 || sLen <= 0) continue;
    const fRawBin = heatmapBinPeriod(fLen, hm.fast_min, hm.fast_max, rawBinsFast);
    const sRawBin = heatmapBinPeriod(sLen, hm.slow_min, hm.slow_max, rawBinsSlow);
    const fBin = rawFastToDisplay[fRawBin];
    const sBin = rawSlowToDisplay[sRawBin];
    if (fBin < 0 || sBin < 0) continue;
    const idx = fBin * binsSlow + sBin;
    const pt = { fBin, sBin, idx, key, rank: Number(row.rank || 0), row };
    topPoints.push(pt);
    keyToCell.set(key, idx);
    const existing = topByCell.get(idx);
    if (!existing || (pt.rank > 0 && pt.rank < existing.rank)) topByCell.set(idx, pt);
  }

  HEATMAP_STATE = {
    hm,
    obj,
    binsFast,
    binsSlow,
    values: displayValues,
    lo,
    hi,
    fastBins,
    slowBins,
    topPoints,
    topByCell,
    keyToCell,
    overlay,
    stack,
    tooltip,
    meta,
    showTop: prefs.showTop,
    showCrosshair: prefs.showCrosshair,
    cellPx,
  };

  // Legend
  if (legend) {
    const lctx = legend.getContext("2d");
    if (lctx) {
      const lw = legend.width || 1;
      const lh = legend.height || 1;
      const lImg = lctx.createImageData(lw, lh);
      for (let x = 0; x < lw; x++) {
        const t = lw <= 1 ? 0 : x / (lw - 1);
        const i = Math.min(lutSize - 1, Math.max(0, Math.round(t * (lutSize - 1))));
        const r = lut[i * 4 + 0];
        const g = lut[i * 4 + 1];
        const b = lut[i * 4 + 2];
        for (let y = 0; y < lh; y++) {
          const px = (y * lw + x) * 4;
          lImg.data[px + 0] = r;
          lImg.data[px + 1] = g;
          lImg.data[px + 2] = b;
          lImg.data[px + 3] = 255;
        }
      }
      lctx.putImageData(lImg, 0, 0);
    }
  }
  if (legMin) legMin.textContent = fmt(lo, 6);
  if (legMid) legMid.textContent = fmt((lo + hi) / 2, 6);
  if (legMax) legMax.textContent = fmt(hi, 6);

  const hideTooltip = () => {
    if (tooltip) tooltip.style.display = "none";
  };
  const showTooltip = (xCss, yCss, html) => {
    if (!tooltip) return;
    tooltip.innerHTML = html;
    tooltip.style.display = "block";
    const pad = 10;
    const tw = tooltip.offsetWidth || 0;
    const th = tooltip.offsetHeight || 0;
    const maxX = Math.max(pad, (stack.clientWidth || 0) - tw - pad);
    const maxY = Math.max(pad, (stack.clientHeight || 0) - th - pad);
    tooltip.style.left = `${Math.max(pad, Math.min(maxX, xCss + 14))}px`;
    tooltip.style.top = `${Math.max(pad, Math.min(maxY, yCss + 14))}px`;
  };

  overlay.onmousemove = (ev) => {
    const rect = overlay.getBoundingClientRect();
    const xCss = ev.clientX - rect.left;
    const yCss = ev.clientY - rect.top;
    const sBin = Math.floor((xCss / rect.width) * binsSlow);
    const fBin = Math.floor((yCss / rect.height) * binsFast);
    if (sBin < 0 || fBin < 0 || sBin >= binsSlow || fBin >= binsFast) {
      HEATMAP_HOVER = null;
      meta.textContent = HEATMAP_META_BASE;
      hideTooltip();
      renderHeatmapOverlay();
      return;
    }

    const idx = fBin * binsSlow + sBin;
    const v = displayValues[idx];
    if (!Number.isFinite(v)) {
      HEATMAP_HOVER = null;
      meta.textContent = HEATMAP_META_BASE;
      hideTooltip();
      renderHeatmapOverlay();
      return;
    }

    const fr = fastBins[fBin];
    const sr = slowBins[sBin];
    const top = topByCell.get(idx) || null;
    const topLine = top
      ? `<div class="muted">Top in bin: #${top.rank} ${Number(top.row?.params?.fast_len || 0)}/${Number(top.row?.params?.slow_len || 0)}</div>`
      : `<div class="muted">Top in bin: none (increase Top K to explore)</div>`;

    HEATMAP_HOVER = { fBin, sBin, idx, v };
    meta.textContent = `${HEATMAP_META_BASE} | hover: fast=${formatBinRange(fr)} slow=${formatBinRange(sr)} score=${fmt(v, 6)}`;
    showTooltip(
      xCss,
      yCss,
      `<div class="mono">fast ${formatBinRange(fr)} · slow ${formatBinRange(sr)}</div>` +
      `<div class="mono">score ${fmt(v, 6)}</div>` +
      topLine +
      `<div class="muted">Click to drill down</div>`
    );
    renderHeatmapOverlay();
  };

  overlay.onmouseleave = () => {
    HEATMAP_HOVER = null;
    meta.textContent = HEATMAP_META_BASE;
    hideTooltip();
    renderHeatmapOverlay();
  };

  overlay.onclick = (ev) => {
    const rect = overlay.getBoundingClientRect();
    const xCss = ev.clientX - rect.left;
    const yCss = ev.clientY - rect.top;
    const sBin = Math.floor((xCss / rect.width) * binsSlow);
    const fBin = Math.floor((yCss / rect.height) * binsFast);
    if (sBin < 0 || fBin < 0 || sBin >= binsSlow || fBin >= binsFast) return;

    const idx = fBin * binsSlow + sBin;
    const exact = topByCell.get(idx) || null;
    let best = exact;
    let bestD2 = 1e18;
    if (!best && Array.isArray(topPoints) && topPoints.length) {
      for (const p of topPoints) {
        const dx = p.sBin - sBin;
        const dy = p.fBin - fBin;
        const d2 = dx * dx + dy * dy;
        if (d2 < bestD2) {
          bestD2 = d2;
          best = p;
        }
      }
      if (best && bestD2 > 9) best = null;
    }
    if (!best) {
      setStatus("No Top row near this cell. Try increasing Top K.", "muted");
      return;
    }

    DRILL_SELECTED_KEY = best.key;
    HEATMAP_SELECTED = { fBin: best.fBin, sBin: best.sBin, idx: best.idx, key: best.key };
    requestDrilldown(best.row.params, undefined, {
      source: "heatmap",
      row: best.row,
      metrics: best.row?.metrics || null,
    });
    renderTopTable(CURRENT_RUN?.res, CURRENT_RUN?.req);
    renderHeatmapOverlay();
    RESULTS_TABS?.set("drill", { save: false, focus: true });
  };

  syncHeatmapSelectionToDrillKey();
}

function renderBest(res, req) {
  const el = $("bestSummary");
  if (!el) return;
  const kindRaw = String(req?.strategy_kind || req?.strategyKind || "DoubleMa");
  const startEq = startingEquityFromReq(req);

  const metaRes = LAST_META_RUN?.result && typeof LAST_META_RUN.result === "object" ? LAST_META_RUN.result : null;
  const metaKind = String(metaRes?.strategy_kind || "").trim();
  const metaTop = Array.isArray(metaRes?.groups) && metaRes.groups.length ? metaRes.groups[0] : null;
  const metaHold = metaTop?.holdout && typeof metaTop.holdout === "object" ? metaTop.holdout : null;
  if (metaTop && metaHold && (!metaKind || metaKind === kindRaw)) {
    const p = metaTop.params && typeof metaTop.params === "object" ? metaTop.params : {};
    const title =
      `${String(p.fast_ma_type || "?").toUpperCase()}(${p.fast_len || "?"}) vs ` +
      `${String(p.slow_ma_type || "?").toUpperCase()}(${p.slow_len || "?"})`;
    const groups = Number(Array.isArray(metaRes?.groups) ? metaRes.groups.length : 0);
    const datasets = Number(metaHold.datasets || 0);
    el.textContent =
      `final selection: ${p.fast_ma_type || "?"}(${p.fast_len || "?"}) vs ${p.slow_ma_type || "?"}(${p.slow_len || "?"}) | ` +
      `meta=${fmt(metaTop.meta_score, 6)} robust_p10=${fmt(metaTop.robust_score_p10, 6)} dd_p90=${fmt(metaTop.max_dd_p90, 6)} | ` +
      `holdout pass=${fmtFixed(Number(metaHold.pass_rate || 0) * 100, 1)}% p10=${fmt(metaHold.score_p10, 6)} dd_p90=${fmt(metaHold.max_dd_p90, 6)} datasets=${datasets} groups=${groups}`;
    renderBestCard({
      kicker: "Final selection",
      title,
      meta: `meta+holdout · groups=${groups} · datasets=${datasets}`,
      stats: [
        { label: "Meta", value: fmt(metaTop.meta_score, 6) },
        { label: "Robust p10", value: fmt(metaTop.robust_score_p10, 6) },
        { label: "DD p90", value: fmt(metaTop.max_dd_p90, 6) },
        { label: "H pass", value: `${fmtFixed(Number(metaHold.pass_rate || 0) * 100, 1)}%` },
        { label: "H p10", value: fmt(metaHold.score_p10, 6) },
        { label: "H DD p90", value: fmt(metaHold.max_dd_p90, 6) },
      ],
    });
    return;
  }

  if (CURRENT_VALIDATION?.res && CURRENT_VALIDATION.ts === CURRENT_RUN?.ts) {
    const { rows, hasCross, usedFallback } = computeValidationShortlist(CURRENT_VALIDATION.res);
    const c = Array.isArray(rows) && rows.length ? rows[0] : null;
    if (c) {
      const p = c.params || {};
      const m = c.in_sample || {};
      const cross = c.cross_ticker && typeof c.cross_ticker === "object" ? c.cross_ticker : null;
      const crossPart = hasCross && cross ? ` cross=${fmt(cross.score, 6)}${c.cross_ticker_pareto ? "" : "(!)"}` : "";
      const fb = usedFallback ? " (fallback)" : "";
      const finalEq = finalEquityFromPnl(m.pnl, startEq);
      el.textContent =
        `recommended${fb}: ${p.fast_ma_type || "?"}(${p.fast_len || "?"}) vs ${p.slow_ma_type || "?"}(${p.slow_len || "?"}) | ` +
        `robust=${fmt(c.robust_score, 6)}${c.robust_pareto ? "" : "(!)"}${crossPart} | ` +
        `pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} final_eq=${fmtMoney(finalEq, 2)} ` +
        `sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} expo=${fmt(m.exposure, 4)}`;

      const title =
        `${String(p.fast_ma_type || "?").toUpperCase()}(${p.fast_len || "?"}) vs ` +
        `${String(p.slow_ma_type || "?").toUpperCase()}(${p.slow_len || "?"})`;
      const robustV = `${fmt(c.robust_score, 6)}${c.robust_pareto ? "" : " (!)"}`;
      const crossV = hasCross && cross ? `${fmt(cross.score, 6)}${c.cross_ticker_pareto ? "" : " (!)"}` : "";
      const pnl = Number(m.pnl ?? NaN);
      renderBestCard({
        kicker: usedFallback ? "Recommended (fallback)" : "Recommended",
        title,
        meta: `validation shortlist · candidates=${Array.isArray(rows) ? rows.length : 0}${hasCross ? " · cross-ticker" : ""} · holdout via Meta`,
        stats: [
          { label: "Robust", value: robustV },
          ...(crossV ? [{ label: "Cross", value: crossV }] : []),
          { label: "PnL", value: fmt(m.pnl, 6), tone: Number.isFinite(pnl) ? (pnl > 0 ? "good" : (pnl < 0 ? "bad" : "")) : "" },
          { label: "Return %", value: fmtPct(m.pnl, 2), tone: Number.isFinite(pnl) ? (pnl > 0 ? "good" : (pnl < 0 ? "bad" : "")) : "" },
          { label: "Final Eq", value: fmtMoney(finalEq, 2) },
          { label: "Sharpe", value: fmt(m.sharpe, 6) },
          { label: "MaxDD", value: fmt(m.max_dd, 6) },
          { label: "Trades", value: String(Number(m.trades || 0)) },
          { label: "Exposure", value: fmt(m.exposure, 4) },
        ],
      });
      return;
    }
  }

  if (kindRaw === "CompositeMo" && Array.isArray(res?.pareto)) {
    const pts = res.pareto;
    if (!pts.length) {
      el.textContent = "pareto: 0 points";
      renderBestCard({ kicker: "Pareto", title: "0 points", meta: "—", stats: [] });
      return;
    }
    const best = [...pts].sort((a, b) => {
      const av = Number(a?.violation ?? 0);
      const bv = Number(b?.violation ?? 0);
      const aFeas = av <= 0;
      const bFeas = bv <= 0;
      if (aFeas !== bFeas) return aFeas ? -1 : 1;
      const aMax = Number(a?.maximize_value ?? -Infinity);
      const bMax = Number(b?.maximize_value ?? -Infinity);
      if (aMax !== bMax) return bMax - aMax;
      const aMin = Number(a?.minimize_value ?? Infinity);
      const bMin = Number(b?.minimize_value ?? Infinity);
      if (aMin !== bMin) return aMin - bMin;
      return 0;
    })[0];

    const p = best?.params || {};
    const m = best?.metrics || {};
    const maxObj = String(req?.objectives?.maximize || req?.objective || "?");
    const minObj = String(req?.objectives?.minimize || "?");
    const maxV = Number(best?.maximize_value ?? NaN);
    const minV = Number(best?.minimize_value ?? NaN);
    const viol = Number(best?.violation ?? NaN);
    const runtimeMs = Number(res?.runtime_ms || 0);
    const evals = Number(res?.total_evaluations || 0);
    const finalEq = finalEquityFromPnl(m.pnl, startEq);
    el.textContent =
      `pareto best: ${p.fast_ma_type || "?"}(${p.fast_len || "?"}) vs ${p.slow_ma_type || "?"}(${p.slow_len || "?"}) | ` +
      `pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} final_eq=${fmtMoney(finalEq, 2)} ` +
      `sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} expo=${fmt(m.exposure, 4)} | ` +
      `maximize(${maxObj})=${fmt(maxV, 6)} minimize(${minObj})=${fmt(minV, 6)} viol=${fmt(viol, 6)} | ` +
      `pareto=${pts.length} evals=${evals} runtime_ms=${runtimeMs}`;

    const title =
      `${String(p.fast_ma_type || "?").toUpperCase()}(${p.fast_len || "?"}) vs ` +
      `${String(p.slow_ma_type || "?").toUpperCase()}(${p.slow_len || "?"})`;
    const pnl = Number(m.pnl ?? NaN);
    renderBestCard({
      kicker: "Pareto best",
      title,
      meta: `pareto=${pts.length} · evals=${evals} · runtime=${runtimeMs}ms`,
      stats: [
        { label: "PnL", value: fmt(m.pnl, 6), tone: Number.isFinite(pnl) ? (pnl > 0 ? "good" : (pnl < 0 ? "bad" : "")) : "" },
        { label: "Return %", value: fmtPct(m.pnl, 2), tone: Number.isFinite(pnl) ? (pnl > 0 ? "good" : (pnl < 0 ? "bad" : "")) : "" },
        { label: "Final Eq", value: fmtMoney(finalEq, 2) },
        { label: "Sharpe", value: fmt(m.sharpe, 6) },
        { label: "MaxDD", value: fmt(m.max_dd, 6) },
        { label: "Trades", value: String(Number(m.trades || 0)) },
        { label: "Exposure", value: fmt(m.exposure, 4) },
        { label: `Max (${maxObj})`, value: fmt(maxV, 6) },
        { label: `Min (${minObj})`, value: fmt(minV, 6) },
        { label: "Violation", value: fmt(viol, 6), tone: Number.isFinite(viol) && viol > 0 ? "bad" : "" },
      ],
    });
    return;
  }

  const p = res?.best_params || {};
  const m = res?.best_metrics || {};
  const obj = req?.objective || "-";
  const backendUsed = String(res?.backend_used || "-");
  const modeUsed = String(res?.mode_used || "-");
  const combos = Number(res?.num_combos || 0);
  const runtimeMs = Number(res?.runtime_ms || 0);
  const vdps = res?.vdps && typeof res.vdps === "object" ? res.vdps : null;
  const vdpsMeta = vdps
    ? ` vdps_eval=${Number(vdps.evaluated_pairs_total || 0)} vdps_ff_saved=${Number(vdps.full_fidelity_pairs_saved || 0)}`
    : "";
  const s = fmt(scoreOf(m, obj), 6);
  const finalEq = finalEquityFromPnl(m.pnl, startEq);
  el.textContent =
    `best: ${p.fast_ma_type || "?"}(${p.fast_len || "?"}) vs ${p.slow_ma_type || "?"}(${p.slow_len || "?"}) | ` +
    `pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} final_eq=${fmtMoney(finalEq, 2)} ` +
    `sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} expo=${fmt(m.exposure, 4)} score(${obj})=${s} | ` +
    `backend=${backendUsed} mode=${modeUsed} combos=${combos} runtime_ms=${runtimeMs}${vdpsMeta}`;

  const title =
    `${String(p.fast_ma_type || "?").toUpperCase()}(${p.fast_len || "?"}) vs ` +
    `${String(p.slow_ma_type || "?").toUpperCase()}(${p.slow_len || "?"})`;
  const pnl = Number(m.pnl ?? NaN);
  renderBestCard({
    kicker: "Best",
    title,
    meta: `backend=${backendUsed} · mode=${modeUsed} · combos=${combos} · runtime=${runtimeMs}ms`,
    stats: [
      { label: "PnL", value: fmt(m.pnl, 6), tone: Number.isFinite(pnl) ? (pnl > 0 ? "good" : (pnl < 0 ? "bad" : "")) : "" },
      { label: "Return %", value: fmtPct(m.pnl, 2), tone: Number.isFinite(pnl) ? (pnl > 0 ? "good" : (pnl < 0 ? "bad" : "")) : "" },
      { label: "Final Eq", value: fmtMoney(finalEq, 2) },
      { label: "Sharpe", value: fmt(m.sharpe, 6) },
      { label: "MaxDD", value: fmt(m.max_dd, 6) },
      { label: "Trades", value: String(Number(m.trades || 0)) },
      { label: "Exposure", value: fmt(m.exposure, 4) },
      { label: `Score (${obj})`, value: s },
    ],
  });
}

function renderRun(res, req) {
  const kindRaw = String(req?.strategy_kind || req?.strategyKind || "DoubleMa");
  let best = res?.best_params;
  let bestMetrics = res?.best_metrics || null;
  let bestMo = null;
  if (!best && kindRaw === "CompositeMo" && Array.isArray(res?.pareto) && res.pareto.length) {
    const pts = res.pareto;
    const bestPt = [...pts].sort((a, b) => {
      const av = Number(a?.violation ?? 0);
      const bv = Number(b?.violation ?? 0);
      const aFeas = av <= 0;
      const bFeas = bv <= 0;
      if (aFeas !== bFeas) return aFeas ? -1 : 1;
      const aMax = Number(a?.maximize_value ?? -Infinity);
      const bMax = Number(b?.maximize_value ?? -Infinity);
      if (aMax !== bMax) return bMax - aMax;
      const aMin = Number(a?.minimize_value ?? Infinity);
      const bMin = Number(b?.minimize_value ?? Infinity);
      if (aMin !== bMin) return aMin - bMin;
      return 0;
    })[0];
    best = bestPt?.params || null;
    bestMetrics = bestPt?.metrics || null;
    bestMo = bestPt ? {
      maximize_value: Number(bestPt.maximize_value ?? NaN),
      minimize_value: Number(bestPt.minimize_value ?? NaN),
      violation: Number(bestPt.violation ?? NaN),
    } : null;
  }
  DRILL_SELECTED_KEY = best ? paramsKey(best) : "";
  renderBest(res, req);
  renderTopTable(res, req);
  renderHeatmap(res, req);
  renderCompare();
  updateVdpsTelemetryHint(res);
  if (best) {
    requestDrilldown(best, req, {
      source: "best",
      metrics: bestMetrics,
      row: bestMo ? { params: best, metrics: bestMetrics, mo: bestMo } : null,
    });
    scheduleDrillPrefetch(req, DRILL_SELECTED_KEY);
  } else {
    setDrillVisible(false);
  }
  updateOverviewEmptyUi();
  renderContextChips();
}

function parseVdpsRungsInput(raw) {
  const text = String(raw || "").trim();
  if (!text) return [];
  const parts = text.split(/[,\s]+/g).map((s) => Number(s)).filter((v) => Number.isFinite(v));
  const out = [];
  for (const v of parts) {
    if (v > 0) out.push(Math.min(1, Math.max(0.000001, v)));
  }
  out.sort((a, b) => a - b);
  const dedup = [];
  for (const v of out) {
    if (!dedup.length || Math.abs(dedup[dedup.length - 1] - v) > 1e-12) dedup.push(v);
  }
  return dedup;
}

function readVdpsConfigFromUi(mode) {
  if (mode !== "GpuVdps") return null;
  const eta = Math.floor(num($("vdpsEta")?.value, 4));
  if (!Number.isFinite(eta) || eta < 2 || eta > 32) throw new Error("VDPS eta must be in [2, 32].");

  const rungs = parseVdpsRungsInput($("vdpsRungs")?.value);
  if (!rungs.length) throw new Error("VDPS rungs must include at least one positive fraction.");

  const minInitialPairs = Math.floor(num($("vdpsMinInitialPairs")?.value, 4096));
  if (!Number.isFinite(minInitialPairs) || minInitialPairs < 1) {
    throw new Error("VDPS min initial pairs must be >= 1.");
  }

  const maxSurvivorSeeds = Math.floor(num($("vdpsMaxSurvivorSeeds")?.value, 256));
  if (!Number.isFinite(maxSurvivorSeeds) || maxSurvivorSeeds < 8) {
    throw new Error("VDPS max survivor seeds must be >= 8.");
  }

  const maxPeriodCandidates = Math.floor(num($("vdpsMaxPeriodCandidates")?.value, 384));
  if (!Number.isFinite(maxPeriodCandidates) || maxPeriodCandidates < 16) {
    throw new Error("VDPS max period candidates must be >= 16.");
  }

  const randomInjectionRatio = num($("vdpsRandomInjectionRatio")?.value, 0.10);
  if (!Number.isFinite(randomInjectionRatio) || randomInjectionRatio < 0 || randomInjectionRatio > 1) {
    throw new Error("VDPS random injection ratio must be in [0, 1].");
  }
  const eliteMutationRatio = num($("vdpsEliteMutationRatio")?.value, 0.50);
  if (!Number.isFinite(eliteMutationRatio) || eliteMutationRatio < 0 || eliteMutationRatio > 1) {
    throw new Error("VDPS elite mutation ratio must be in [0, 1].");
  }
  const differentialRatio = num($("vdpsDifferentialRatio")?.value, 0.40);
  if (!Number.isFinite(differentialRatio) || differentialRatio < 0 || differentialRatio > 1) {
    throw new Error("VDPS differential ratio must be in [0, 1].");
  }

  const seed = Math.max(0, Math.floor(num($("vdpsSeed")?.value, 1)));
  const deterministic = !!$("vdpsDeterministic")?.checked;
  const qualityCheckMaxPairs = Math.floor(num($("vdpsQualityCheckMaxPairs")?.value, 0));
  if (!Number.isFinite(qualityCheckMaxPairs) || qualityCheckMaxPairs < 0) {
    throw new Error("VDPS quality check max pairs must be >= 0.");
  }
  const qualityCheckTopK = Math.floor(num($("vdpsQualityCheckTopK")?.value, 20));
  if (!Number.isFinite(qualityCheckTopK) || qualityCheckTopK < 1) {
    throw new Error("VDPS quality check top-K must be >= 1.");
  }

  return {
    eta,
    rungs,
    min_initial_pairs: minInitialPairs,
    max_survivor_seeds: maxSurvivorSeeds,
    max_period_candidates: maxPeriodCandidates,
    random_injection_ratio: randomInjectionRatio,
    elite_mutation_ratio: eliteMutationRatio,
    differential_ratio: differentialRatio,
    deterministic,
    seed,
    quality_check_max_pairs: qualityCheckMaxPairs,
    quality_check_top_k: qualityCheckTopK,
  };
}

function updateVdpsTelemetryHint(res) {
  const el = $("vdpsTelemetryHint");
  if (!el) return;
  const vdps = res?.vdps && typeof res.vdps === "object" ? res.vdps : null;
  if (!vdps) {
    el.textContent = "VDPS telemetry appears after a run.";
    return;
  }
  const evald = Number(vdps.evaluated_pairs_total || 0);
  const full = Number(vdps.full_grid_pairs || 0);
  const saved = Number(vdps.full_fidelity_pairs_saved || 0);
  const ff = Number(vdps.full_fidelity_pairs_evaluated || 0);
  const rungs = Array.isArray(vdps.rung_stats) ? vdps.rung_stats.length : 0;
  const rt = Number(vdps.runtime_ms_total || 0);
  const elite = Number(vdps.elite_mutation_ratio || 0);
  const diff = Number(vdps.differential_ratio || 0);
  const rand = Number(vdps.random_injection_ratio || 0);
  const qcOverlap = Number(vdps.quality_check_top_k_overlap);
  const qcRegret = Number(vdps.quality_check_best_score_regret);
  const qcPairs = Number(vdps.quality_check_baseline_pairs || 0);
  const qcMs = Number(vdps.quality_check_baseline_runtime_ms || 0);
  const qcText =
    Number.isFinite(qcOverlap) && Number.isFinite(qcRegret) && qcPairs > 0
      ? ` quality(overlap=${qcOverlap.toFixed(2)}, regret=${qcRegret.toFixed(6)}, baseline_pairs=${qcPairs}, baseline_ms=${qcMs})`
      : "";
  el.textContent = `VDPS: rungs=${rungs} evals=${evald} full_grid=${full} full_fidelity=${ff} saved=${saved} runtime=${rt}ms mix(e/d/r)=${elite.toFixed(2)}/${diff.toFixed(2)}/${rand.toFixed(2)}${qcText}`;
}

function getDoubleMaRequest() {
  const reqKind = strategyKind();
  const dataIdText = $("dataId").textContent.trim();
  const dataId = dataIdText === "—" ? "" : dataIdText;
  if (!dataId || dataId === "-") throw new Error("Load CSV first (missing data id).");

  const backend = $("backend").value;
  const deviceId = Number($("deviceId").value || 0);

  const fast = [
    Number($("fastStart").value),
    Number($("fastEnd").value),
    Number($("fastStep").value),
  ];
  const slow = [
    Number($("slowStart").value),
    Number($("slowEnd").value),
    Number($("slowStep").value),
  ];

  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  if (!pairs) throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = ($("fastMaType")?.value || "").trim();
  const slowMaType = ($("slowMaType")?.value || "").trim();
  const exitMode = String($("exitMode")?.value || "cross").trim() || "cross";
  const cooldownBars = Math.max(0, Math.floor(num($("cooldownBars")?.value, 0)));
  const signalPersistenceBars = Math.max(1, Math.floor(num($("signalPersistenceBars")?.value, 1)));
  const trailingStopPct = parseRiskPctInput("trailingStopPct");
  const hasExecutionOverlays =
    cooldownBars > 0 || signalPersistenceBars > 1 || trailingStopPct != null;
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");

  if (backend === "CpuOnly") {
    if (!maSupportsCpu(fastMaType) || !maSupportsCpu(slowMaType)) {
      throw new Error("CPU backend does not support one of the selected MA types. Choose a supported MA or switch backend.");
    }
  }

  if (backend === "GpuOnly") {
    if (!maSupportsCuda(fastMaType) || !maSupportsCuda(slowMaType)) {
      throw new Error("GPU backend requires CUDA support for both selected MAs. Choose a supported MA or switch backend.");
    }
  }

  if (backend === "GpuKernel") {
    if (!isDoubleLikeKind(reqKind)) {
      throw new Error("GPU (kernel) is available for Double MA, Composite Trend-Quality, Regime-Switch Hybrid, and Breakout Pulse only.");
    }
    if (exitMode !== "cross") {
      throw new Error("GPU (kernel) currently supports exit mode = cross only.");
    }
    if (hasExecutionOverlays) {
      throw new Error("GPU (kernel) does not support cooldown/persistence/trailing-stop overlays.");
    }
    if (!maSupportsKernel(fastMaType) || !maSupportsKernel(slowMaType)) {
      throw new Error("GPU (kernel) does not support one of the selected MA types. Choose a supported MA or switch backend.");
    }
  } else if (backend === "GpuOnly") {
    if (!maSupportsCuda(fastMaType) || !maSupportsCuda(slowMaType)) {
      throw new Error("GPU (MA sweep) does not support the selected MA(s). Switch backend or choose a supported MA.");
    }
  } else if (backend === "CpuOnly") {
    if (!maSupportsCpu(fastMaType) || !maSupportsCpu(slowMaType)) {
      throw new Error("CPU backend does not support the selected MA(s). Switch backend or choose a supported MA.");
    }
  } else if (backend === "Auto") {
    const okFast = maSupportsCpu(fastMaType) || maSupportsCuda(fastMaType) || maSupportsKernel(fastMaType);
    const okSlow = maSupportsCpu(slowMaType) || maSupportsCuda(slowMaType) || maSupportsKernel(slowMaType);
    if (!okFast || !okSlow) {
      throw new Error("Auto backend cannot run the selected MA(s). Choose a period-based, single-output MA that supports CPU or CUDA.");
    }
  }

  const maSource = $("maSource").value;
  validateCandleFieldRequirements(CURRENT_DATA_META, fastMaType, slowMaType, maSource);
  if (reqKind === "RegimeHybrid" || reqKind === "BreakoutMomentum") {
    const meta = CURRENT_DATA_META;
    if (!meta || !meta.has_high || !meta.has_low) {
      throw new Error(
        reqKind === "BreakoutMomentum"
          ? "Breakout Pulse requires high/low candle fields in the loaded dataset."
          : "Regime-Switch Hybrid requires high/low candle fields in the loaded dataset."
      );
    }
    if (reqKind === "BreakoutMomentum" && !meta.has_volume) {
      throw new Error("Breakout Pulse requires volume candle field in the loaded dataset.");
    }
  }

  const objective = $("objective").value;
  const mode = $("mode").value;

  const topK = Number($("topK").value || 0);
  const includeAll = $("includeAll").value === "true";

  const fastVar = estimateMaParamVariantCount("fast", fastMaType);
  const slowVar = estimateMaParamVariantCount("slow", slowMaType);
  const combos = pairs * fastVar * slowVar;
  if (includeAll && combos > 2_000_000) {
    throw new Error(`includeAll=true would return ${combos} rows; disable it or reduce ranges/sweeps.`);
  }

  const exportAllCsvPath = ($("exportAllCsvPath")?.value || "").trim();
  if (exportAllCsvPath) {
    if (includeAll) throw new Error("exportAllCsvPath requires includeAll=false (stream export avoids huge RAM usage).");
    if (mode !== "Grid") throw new Error("exportAllCsvPath requires mode=Grid.");
    if (backend !== "GpuKernel") throw new Error("exportAllCsvPath is currently supported only for the GPU (kernel) backend.");
    if (fastVar * slowVar > 1) throw new Error("exportAllCsvPath is not supported when optimizing MA parameters. Disable MA param sweeps or export.");
  }

  const heatmapBins = Math.max(0, Math.floor(num($("heatmapBins")?.value, 0)));
  if (heatmapBins > 512) throw new Error("heatmapBins must be <= 512.");
  if (mode === "CoarseToFine") {
    if (includeAll) throw new Error("Coarse→Fine mode requires includeAll=false.");
    if (heatmapBins > 0) throw new Error("Coarse→Fine mode currently requires heatmapBins=0.");
    if (exportAllCsvPath) throw new Error("Coarse→Fine mode cannot be used with exportAllCsvPath.");
  }
  if (mode === "GpuVdps") {
    if (!isDoubleLikeKind(reqKind)) {
      throw new Error(
        "GPU VDPS mode is available for Double MA, Composite Trend-Quality, Regime-Switch Hybrid, and Breakout Pulse only."
      );
    }
    if (includeAll) throw new Error("GpuVdps mode requires includeAll=false.");
    if (heatmapBins > 0) throw new Error("GpuVdps mode currently requires heatmapBins=0.");
    if (exportAllCsvPath) throw new Error("GpuVdps mode does not support exportAllCsvPath.");
    if (backend === "CpuOnly") {
      throw new Error("GpuVdps mode requires backend=Auto/GpuOnly/GpuKernel with CUDA available.");
    }
    if (hasExecutionOverlays) {
      throw new Error("GpuVdps mode does not support cooldown/persistence/trailing-stop overlays.");
    }
  }
  const vdps = readVdpsConfigFromUi(mode);

  const strategy = readStrategyCfgFromUi();
  const minTrades = optNum($("dmConstraintMinTrades")?.value);
  const maxDd = optNum($("dmConstraintMaxDd")?.value);
  let constraints = null;
  if (Number.isFinite(minTrades) || Number.isFinite(maxDd)) {
    constraints = {};
    if (Number.isFinite(minTrades)) {
      if (minTrades < 0) throw new Error("Min trades must be >= 0.");
      constraints.min_trades = Math.floor(minTrades);
    }
    if (Number.isFinite(maxDd)) {
      if (maxDd < 0) throw new Error("Max drawdown constraint must be >= 0.");
      constraints.max_drawdown = maxDd;
    }
  }

  const backendObj =
    backend === "GpuOnly" || backend === "GpuKernel" || backend === "Auto"
      ? { backend, device_id: deviceId }
      : { backend: "CpuOnly" };

  return {
    strategy_kind: reqKind,
    backend: backendObj,
    data_id: dataId,
    starting_equity: readStartingEquityFromUi(),
    fast_range: fast,
    slow_range: slow,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    export_all_csv_path: exportAllCsvPath || null,
    fast_ma_params: resolvedMaParams("fast", fastMaType),
    slow_ma_params: resolvedMaParams("slow", slowMaType),
    fast_ma_param_sweeps: resolvedMaParamSweeps("fast", fastMaType),
    slow_ma_param_sweeps: resolvedMaParamSweeps("slow", slowMaType),
    strategy,
    objective,
    mode,
    vdps,
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
    constraints,
  };
}

function defaultCompositeMoSearchSpace() {
  return {
    params: [
      { name: "halftrend_amplitude", domain: { IntRange: { low: 2, high: 20, step: 1, log: false } } },
      { name: "halftrend_channel_deviation", domain: { FloatRange: { low: 0.5, high: 5.0, log: false } } },
      { name: "halftrend_atr_period", domain: { IntRange: { low: 20, high: 200, step: 1, log: false } } },

      { name: "macz_fast_length", domain: { IntRange: { low: 5, high: 30, step: 1, log: false } } },
      { name: "macz_slow_length", domain: { IntRange: { low: 10, high: 60, step: 1, log: false } } },
      { name: "macz_signal_length", domain: { IntRange: { low: 3, high: 30, step: 1, log: false } } },
      { name: "macz_lengthz", domain: { IntRange: { low: 5, high: 50, step: 1, log: false } } },
      { name: "macz_length_stdev", domain: { IntRange: { low: 5, high: 80, step: 1, log: false } } },
      { name: "macz_a", domain: { FloatRange: { low: -2.0, high: 2.0, log: false } } },
      { name: "macz_b", domain: { FloatRange: { low: -2.0, high: 2.0, log: false } } },
      { name: "macz_gamma", domain: { FloatRange: { low: 0.0, high: 0.2, log: false } } },
      { name: "macz_use_lag", domain: "Bool" },
      { name: "macz_use_volume", domain: "Bool" },

      { name: "ott_period", domain: { IntRange: { low: 1, high: 50, step: 1, log: false } } },
      { name: "ott_percent", domain: { FloatRange: { low: 0.1, high: 5.0, log: false } } },
      {
        name: "ott_ma_type",
        domain: { Categorical: { choices: ["VAR", "EMA", "SMA", "WMA", "TMA", "WWMA", "ZLEMA", "TSF"] } },
      },

      { name: "strategy_commission", domain: { FloatRange: { low: 0.0, high: 0.001, log: false } } },
      { name: "strategy_eps_rel", domain: { FloatRange: { low: 0.0, high: 0.005, log: false } } },
      { name: "strategy_long_only", domain: "Bool" },
      { name: "strategy_allow_flip", domain: "Bool" },
      { name: "strategy_trade_on_next_bar", domain: "Bool" },
    ],
  };
}

function parseMoRangeInt(lowId, highId, stepId, label) {
  const low = Math.floor(num($(lowId)?.value, NaN));
  const high = Math.floor(num($(highId)?.value, NaN));
  const step = Math.floor(num($(stepId)?.value, NaN));
  if (!Number.isFinite(low) || !Number.isFinite(high) || !Number.isFinite(step)) {
    throw new Error(`${label}: low/high/step must be finite integers.`);
  }
  if (step < 1) throw new Error(`${label}: step must be >= 1.`);
  if (low > high) throw new Error(`${label}: low must be <= high.`);
  return { IntRange: { low, high, step, log: false } };
}

function parseMoRangeFloat(lowId, highId, stepId, label) {
  const low = num($(lowId)?.value, NaN);
  const high = num($(highId)?.value, NaN);
  const step = num($(stepId)?.value, NaN);
  if (!Number.isFinite(low) || !Number.isFinite(high) || !Number.isFinite(step)) {
    throw new Error(`${label}: low/high/step must be finite numbers.`);
  }
  if (step <= 0) throw new Error(`${label}: step must be > 0.`);
  if (low > high) throw new Error(`${label}: low must be <= high.`);

  if (low === high) return { Categorical: { choices: [low] } };

  const count = Math.floor((high - low) / step) + 1;
  if (Number.isFinite(count) && count >= 2 && count <= 400) {
    const choices = [];
    for (let i = 0; i < count; i++) {
      const v = low + i * step;
      if (v > high + step * 1e-9) break;
      choices.push(Number(v.toFixed(10)));
    }
    const last = choices.length ? choices[choices.length - 1] : NaN;
    if (choices.length >= 2 && Number.isFinite(last) && Math.abs(last - high) > step * 0.55) {
      choices.push(Number(high.toFixed(10)));
    }
    if (choices.length >= 2) return { Categorical: { choices } };
  }

  return { FloatRange: { low, high, log: false } };
}

function parseMoBoolDomain(modeId) {
  const mode = String($(modeId)?.value || "both").trim().toLowerCase();
  if (mode === "true") return { Categorical: { choices: [true] } };
  if (mode === "false") return { Categorical: { choices: [false] } };
  return "Bool";
}

function parseMoOttMaTypeDomain() {
  const raw = String($("moOttMaTypes")?.value || "");
  const choices = raw
    .split(/[,\s]+/g)
    .map((v) => String(v || "").trim().toUpperCase())
    .filter((v) => !!v);
  const dedup = [];
  const seen = new Set();
  for (const c of choices) {
    if (seen.has(c)) continue;
    seen.add(c);
    dedup.push(c);
  }
  if (!dedup.length) {
    throw new Error("OTT MA types: enter at least one value (comma-separated).");
  }
  return { Categorical: { choices: dedup } };
}

function buildCompositeMoSearchSpaceFromUi() {
  return {
    params: [
      { name: "halftrend_amplitude", domain: parseMoRangeInt("moHtAmpLow", "moHtAmpHigh", "moHtAmpStep", "HalfTrend amplitude") },
      { name: "halftrend_channel_deviation", domain: parseMoRangeFloat("moHtDevLow", "moHtDevHigh", "moHtDevStep", "HalfTrend channel deviation") },
      { name: "halftrend_atr_period", domain: parseMoRangeInt("moHtAtrLow", "moHtAtrHigh", "moHtAtrStep", "HalfTrend ATR period") },

      { name: "macz_fast_length", domain: parseMoRangeInt("moMaczFastLow", "moMaczFastHigh", "moMaczFastStep", "MACZ fast length") },
      { name: "macz_slow_length", domain: parseMoRangeInt("moMaczSlowLow", "moMaczSlowHigh", "moMaczSlowStep", "MACZ slow length") },
      { name: "macz_signal_length", domain: parseMoRangeInt("moMaczSignalLow", "moMaczSignalHigh", "moMaczSignalStep", "MACZ signal length") },
      { name: "macz_lengthz", domain: parseMoRangeInt("moMaczLengthZLow", "moMaczLengthZHigh", "moMaczLengthZStep", "MACZ lengthz") },
      { name: "macz_length_stdev", domain: parseMoRangeInt("moMaczLengthStdevLow", "moMaczLengthStdevHigh", "moMaczLengthStdevStep", "MACZ length stdev") },
      { name: "macz_gamma", domain: parseMoRangeFloat("moMaczGammaLow", "moMaczGammaHigh", "moMaczGammaStep", "MACZ gamma") },
      { name: "macz_a", domain: parseMoRangeFloat("moMaczALow", "moMaczAHigh", "moMaczAStep", "MACZ a") },
      { name: "macz_b", domain: parseMoRangeFloat("moMaczBLow", "moMaczBHigh", "moMaczBStep", "MACZ b") },
      { name: "macz_use_lag", domain: parseMoBoolDomain("moMaczUseLagMode") },
      { name: "macz_use_volume", domain: parseMoBoolDomain("moMaczUseVolumeMode") },

      { name: "ott_period", domain: parseMoRangeInt("moOttPeriodLow", "moOttPeriodHigh", "moOttPeriodStep", "OTT period") },
      { name: "ott_percent", domain: parseMoRangeFloat("moOttPercentLow", "moOttPercentHigh", "moOttPercentStep", "OTT percent") },
      { name: "ott_ma_type", domain: parseMoOttMaTypeDomain() },

      { name: "strategy_commission", domain: { FloatRange: { low: 0.0, high: 0.001, log: false } } },
      { name: "strategy_eps_rel", domain: { FloatRange: { low: 0.0, high: 0.005, log: false } } },
      { name: "strategy_long_only", domain: "Bool" },
      { name: "strategy_allow_flip", domain: "Bool" },
      { name: "strategy_trade_on_next_bar", domain: "Bool" },
    ],
  };
}

function parseCompositeMoSearchSpace() {
  const el = $("moSearchSpace");
  if (!el) return defaultCompositeMoSearchSpace();
  const raw = String(el.value || "").trim();
  if (!raw) {
    const def = defaultCompositeMoSearchSpace();
    el.value = JSON.stringify(def, null, 2);
    return def;
  }
  let obj = null;
  try {
    obj = JSON.parse(raw);
  } catch (e) {
    throw new Error(`Invalid Composite MO search space JSON: ${String(e)}`);
  }
  if (!obj || typeof obj !== "object") throw new Error("Composite MO search space JSON must be an object.");
  if (!Array.isArray(obj.params)) throw new Error("Composite MO search space JSON must contain a params array.");
  return obj;
}

function normalizeCompositeMoSearchSpace(space, fastDomain, slowDomain) {
  const paramsIn = Array.isArray(space?.params) ? space.params : [];
  const params = [];
  const seen = new Set();
  for (const p of paramsIn) {
    if (!p || typeof p !== "object") continue;
    const name = String(p.name || "").trim();
    if (!name) continue;
    const key = name.toLowerCase();
    if (key === "fast_len" || key === "slow_len") continue;
    if (seen.has(key)) continue;
    if (p.domain == null) continue;
    params.push({ name, domain: p.domain });
    seen.add(key);
  }
  return {
    params: [
      { name: "fast_len", domain: fastDomain },
      { name: "slow_len", domain: slowDomain },
      ...params,
    ],
  };
}

function parseCompositeMoPrescanMaPairs(text) {
  const raw = String(text || "");
  const lines = raw.split(/\r?\n/);
  const out = [];
  for (const line of lines) {
    const t = String(line || "").trim();
    if (!t || t.startsWith("#")) continue;
    const parts = t.split(",").map((s) => s.trim()).filter((s) => !!s);
    if (parts.length !== 2) {
      throw new Error(`Invalid MA pair line "${line}". Use "fast,slow" (one pair per line).`);
    }
    out.push({ fast_ma_type: parts[0], slow_ma_type: parts[1] });
  }
  return out.length ? out : null;
}

// ----------------------------
// Distributed (LAN) UI helpers
// ----------------------------

const DIST_SETTINGS_KEY = "vta_dist_settings_v1";
let DIST_SAVE_TIMER = null;

function parseLines(text) {
  const raw = String(text || "");
  return raw
    .split(/\r?\n/)
    .map((s) => String(s || "").trim())
    .filter((s) => !!s && !s.startsWith("#"));
}

function distRandomHex(bytes) {
  const n = Math.max(1, Math.floor(Number(bytes || 0)));
  if (typeof crypto?.getRandomValues === "function") {
    const buf = new Uint8Array(n);
    crypto.getRandomValues(buf);
    return Array.from(buf)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
  }
  let s = "";
  for (let i = 0; i < n; i++) s += Math.floor(Math.random() * 256).toString(16).padStart(2, "0");
  return s;
}

function distGenToken() {
  return distRandomHex(24);
}

function distRewritePath(p, fromPrefix, toPrefix) {
  const path = String(p || "");
  const from = String(fromPrefix || "").trim();
  const to = String(toPrefix || "").trim();
  if (!from || !to) return path;
  const a = path.toLowerCase();
  const b = from.toLowerCase();
  if (!a.startsWith(b)) return path;
  return to + path.slice(from.length);
}

function distRewritePaths(paths, fromPrefix, toPrefix) {
  const out = [];
  for (const p of Array.isArray(paths) ? paths : []) {
    const s = String(p || "").trim();
    if (!s) continue;
    out.push(distRewritePath(s, fromPrefix, toPrefix));
  }
  return out;
}

function saveDistSettingsSoon() {
  if (DIST_SAVE_TIMER) clearTimeout(DIST_SAVE_TIMER);
  DIST_SAVE_TIMER = setTimeout(() => {
    DIST_SAVE_TIMER = null;
    const st = {
      worker_bind: String($("distWorkerBindAddr")?.value || "").trim(),
      worker_token: String($("distWorkerToken")?.value || "").trim(),
      leader_token: String($("distLeaderToken")?.value || "").trim(),
      leader_workers: String($("distLeaderWorkers")?.value || ""),
      leader_mode: String($("distLeaderMode")?.value || "assets"),
      assets_csvs: String($("distAssetsCsvs")?.value || ""),
      path_from: String($("distPathFrom")?.value || ""),
      path_to: String($("distPathTo")?.value || ""),
      tile_csv: String($("distTileCsvPath")?.value || ""),
      tiles_per_worker: Number($("distTilesPerWorker")?.value || 4),
    };
    try {
      localStorage.setItem(DIST_SETTINGS_KEY, JSON.stringify(st));
    } catch {
      // ignore
    }
  }, 200);
}

function loadDistSettings() {
  try {
    const raw = String(localStorage.getItem(DIST_SETTINGS_KEY) || "");
    if (!raw) return null;
    const st = JSON.parse(raw);
    return st && typeof st === "object" ? st : null;
  } catch {
    return null;
  }
}

function applyDistSettings(st) {
  const obj = st && typeof st === "object" ? st : null;
  if (!obj) return;
  if ($("distWorkerBindAddr") && obj.worker_bind != null) $("distWorkerBindAddr").value = String(obj.worker_bind || "");
  if ($("distWorkerToken") && obj.worker_token != null) $("distWorkerToken").value = String(obj.worker_token || "");
  if ($("distLeaderToken") && obj.leader_token != null) $("distLeaderToken").value = String(obj.leader_token || "");
  if ($("distLeaderWorkers") && obj.leader_workers != null) $("distLeaderWorkers").value = String(obj.leader_workers || "");
  if ($("distLeaderMode") && obj.leader_mode != null) $("distLeaderMode").value = String(obj.leader_mode || "assets");
  if ($("distAssetsCsvs") && obj.assets_csvs != null) $("distAssetsCsvs").value = String(obj.assets_csvs || "");
  if ($("distPathFrom") && obj.path_from != null) $("distPathFrom").value = String(obj.path_from || "");
  if ($("distPathTo") && obj.path_to != null) $("distPathTo").value = String(obj.path_to || "");
  if ($("distTileCsvPath") && obj.tile_csv != null) $("distTileCsvPath").value = String(obj.tile_csv || "");
  if ($("distTilesPerWorker") && obj.tiles_per_worker != null) $("distTilesPerWorker").value = String(Number(obj.tiles_per_worker || 4));
}

function updateDistModeUi() {
  const mode = String($("distLeaderMode")?.value || "assets");
  const assets = $("distAssetsWrap");
  const tiles = $("distTilesWrap");
  if (assets) assets.style.display = mode === "assets" ? "" : "none";
  if (tiles) tiles.style.display = mode === "tiles" ? "" : "none";
  const hint = $("distLeaderModeHint");
  if (hint) {
    hint.textContent =
      mode === "tiles"
        ? "Splits one Grid run into tiles (heatmap off). Load the dataset locally; provide a shared/UNC path for workers."
        : "Runs one optimization per CSV. Load any CSV locally to define the settings template; provide shared/UNC paths for workers.";
  }
}

async function refreshDistWorkerStatus() {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    const st = await invoke("distributed_worker_server_status");
    const running = !!st?.running;
    const bind = String(st?.bind_addr || st?.bindAddr || "").trim();
    const el = $("distWorkerStatus");
    if (el) el.textContent = running ? `Worker: running on ${bind}` : "Worker: stopped";
  } catch (e) {
    const el = $("distWorkerStatus");
    if (el) el.textContent = `Worker: error (${String(e)})`;
  }
}

async function startDistWorkerServer() {
  const invoke = tauriInvoke();
  if (!invoke) return;
  const bind_addr = String($("distWorkerBindAddr")?.value || "").trim() || "127.0.0.1:48991";
  const auth_token = String($("distWorkerToken")?.value || "").trim();
  if (!auth_token) throw new Error("Worker auth token is required.");
  const st = await invoke("distributed_start_worker_server", { cfg: { bind_addr, auth_token } });
  const running = !!st?.running;
  const bind = String(st?.bind_addr || st?.bindAddr || "").trim();
  const el = $("distWorkerStatus");
  if (el) el.textContent = running ? `Worker: running on ${bind}` : "Worker: stopped";
}

async function stopDistWorkerServer() {
  const invoke = tauriInvoke();
  if (!invoke) return;
  await invoke("distributed_stop_worker_server");
  await refreshDistWorkerStatus();
}

function renderDistributedAssetsSummary(resp, req) {
  const out = $("distOutput");
  if (!out) return;
  const results = Array.isArray(resp?.results) ? resp.results : [];
  const objective = String(req?.objective || "Sharpe");
  const ok = [];
  const bad = [];
  for (const r of results) {
    const csv = String(r?.csv_path || "").trim();
    const err = String(r?.error || "").trim();
    const res = r?.result || null;
    if (err) bad.push({ csv, err });
    else if (res?.best_params && res?.best_metrics) {
      const p = res.best_params;
      const m = res.best_metrics;
      const score = Number(scoreOf(m, objective) || 0);
      ok.push({ csv, p, m, score });
    }
  }
  ok.sort((a, b) => b.score - a.score);

  const lines = [];
  lines.push(`assets: ok=${ok.length} failed=${bad.length} cancelled=${!!resp?.cancelled}`);
  lines.push(`objective=${objective}`);
  lines.push("");
  lines.push("top (by objective):");
  const maxRows = 50;
  for (let i = 0; i < Math.min(maxRows, ok.length); i++) {
    const r = ok[i];
    const p = r.p || {};
    const m = r.m || {};
    lines.push(
      `${String(i + 1).padStart(3, " ")} score=${fmt(r.score, 6)} pnl=${fmt(m.pnl, 6)} ret=${fmtPct(m.pnl, 2)} sharpe=${fmt(m.sharpe, 6)} dd=${fmt(m.max_dd, 6)} trades=${Number(m.trades || 0)} | ` +
      `${p.fast_ma_type || "?"}(${p.fast_len || "?"}) vs ${p.slow_ma_type || "?"}(${p.slow_len || "?"}) | ${r.csv}`
    );
  }
  if (bad.length) {
    lines.push("");
    lines.push("errors (first 25):");
    for (let i = 0; i < Math.min(25, bad.length); i++) {
      lines.push(`- ${bad[i].csv}: ${bad[i].err}`);
    }
  }
  out.textContent = lines.join("\n");
}

async function runDistributedAssets() {
  if (RUNNING) return;
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (String($("strategyKind")?.value || "DoubleMa") !== "DoubleMa") {
    throw new Error("Distributed v1 supports Double MA only. Switch Strategy to 'Double MA crossover'.");
  }

  const req = getDoubleMaRequest();
  const auth_token = String($("distLeaderToken")?.value || $("distWorkerToken")?.value || "").trim();
  if (!auth_token) throw new Error("Auth token is required.");

  const worker_urls = parseLines($("distLeaderWorkers")?.value || "");
  if (!worker_urls.length) throw new Error("Add at least one worker URL.");

  let csv_paths = parseLines($("distAssetsCsvs")?.value || "");
  if (!csv_paths.length) throw new Error("Add at least one CSV path.");

  const from = String($("distPathFrom")?.value || "");
  const to = String($("distPathTo")?.value || "");
  csv_paths = distRewritePaths(csv_paths, from, to);

  const dist_req = { worker_urls, auth_token, csv_paths, req };

  try {
    clearProgressUi();
    if ($("distOutput")) $("distOutput").textContent = "Starting distributed assets run...";
    setStatus("Distributed assets: starting...", "muted");
    setRunning(true);

    const resp = await invoke("distributed_run_double_ma_assets", { dist_req });
    renderDistributedAssetsSummary(resp, req);

    const ok = Number(resp?.successes || 0);
    const fail = Number(resp?.failures || 0);
    const cancelled = !!resp?.cancelled;
    if (cancelled) setStatus(`Distributed assets: cancelled (${Number(resp?.completed || 0)}/${Number(resp?.total || 0)}).`, "muted");
    else if (fail) setStatus(`Distributed assets: done (${ok} ok, ${fail} failed).`, "danger");
    else setStatus(`Distributed assets: done (${ok} ok).`, "ok");
  } finally {
    setRunning(false);
  }
}

async function runDistributedTiles() {
  if (RUNNING) return;
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (String($("strategyKind")?.value || "DoubleMa") !== "DoubleMa") {
    throw new Error("Distributed v1 supports Double MA only. Switch Strategy to 'Double MA crossover'.");
  }

  const req = getDoubleMaRequest();
  const auth_token = String($("distLeaderToken")?.value || $("distWorkerToken")?.value || "").trim();
  if (!auth_token) throw new Error("Auth token is required.");

  const worker_urls = parseLines($("distLeaderWorkers")?.value || "");
  if (!worker_urls.length) throw new Error("Add at least one worker URL.");

  let csv_path = String($("distTileCsvPath")?.value || "").trim();
  if (!csv_path) throw new Error("Dataset CSV path is required (shared/UNC path).");

  const from = String($("distPathFrom")?.value || "");
  const to = String($("distPathTo")?.value || "");
  csv_path = distRewritePath(csv_path, from, to);

  const tiles_per_worker = Math.max(1, Math.floor(Number($("distTilesPerWorker")?.value || 4)));
  const dist_req = { worker_urls, auth_token, csv_path, req, tiles_per_worker };

  try {
    clearProgressUi();
    if ($("distOutput")) $("distOutput").textContent = "Starting tiled distributed run...";
    setStatus("Distributed tiles: starting...", "muted");
    setRunning(true);

    const resp = await invoke("distributed_run_double_ma_tiled", { dist_req });
    const cancelled = !!resp?.cancelled;
    const result = resp?.result || null;
    const errs = Array.isArray(resp?.errors) ? resp.errors : [];

    if (cancelled) {
      if ($("distOutput")) $("distOutput").textContent = `Cancelled (${Number(resp?.tiles_completed || 0)}/${Number(resp?.tiles_total || 0)} tiles).`;
      setStatus("Distributed tiles: cancelled.", "muted");
      return;
    }
    if (!result) {
      if ($("distOutput")) $("distOutput").textContent = errs.length ? errs.join("\n") : "Tiled run failed (no result).";
      setStatus("Distributed tiles: failed.", "danger");
      return;
    }

    if ($("distOutput")) $("distOutput").textContent = `Tiled run complete (${Number(resp?.tiles_completed || 0)}/${Number(resp?.tiles_total || 0)} tiles). Result loaded into Results.`;
    renderRun(result, req);
    setStatus("Distributed tiles: done.", "ok");
  } finally {
    setRunning(false);
  }
}

async function initDistributedProgressListener() {
  const listen = tauriListen();
  if (!listen) return;
  try {
    await listen("distributed_progress", (event) => {
      if (!RUNNING) return;
      const p = event?.payload || {};
      const mode = String(p.mode || "");
      const completed = Number(p.completed || 0);
      const total = Number(p.total || 0);
      const ok = Number(p.successes || 0);
      const fail = Number(p.failures || 0);
      const last = String(p.last || "").trim();

      const label = mode === "tiles" ? "Distributed · Tiles" : "Distributed · Assets";
      const suffix = [`ok=${ok}`, `err=${fail}`, last].filter((s) => !!String(s || "").trim()).join(" · ");
      setStatus(`${label} ${completed}/${total}${suffix ? ` · ${suffix}` : ""}`, "muted");
      setProgressUi(completed, total, mode === "tiles" ? "distributed_tiles" : "distributed_assets", suffix);
    });
  } catch {
    // optional
  }
}

function initDistributedUi() {
  const panel = $("distLeaderMode");
  if (!panel) return;

  applyDistSettings(loadDistSettings());
  updateDistModeUi();
  refreshDistWorkerStatus();

  $("distLeaderMode")?.addEventListener("change", () => {
    updateDistModeUi();
    saveDistSettingsSoon();
  });

  const watchIds = [
    "distWorkerBindAddr",
    "distWorkerToken",
    "distLeaderToken",
    "distLeaderWorkers",
    "distAssetsCsvs",
    "distPathFrom",
    "distPathTo",
    "distTileCsvPath",
    "distTilesPerWorker",
  ];
  for (const id of watchIds) {
    const el = $(id);
    if (!el) continue;
    el.addEventListener("input", saveDistSettingsSoon);
    el.addEventListener("change", saveDistSettingsSoon);
  }

  $("btnDistGenToken")?.addEventListener("click", () => {
    const tok = distGenToken();
    if ($("distWorkerToken")) $("distWorkerToken").value = tok;
    if ($("distLeaderToken")) $("distLeaderToken").value = tok;
    saveDistSettingsSoon();
    setStatus("Generated distributed auth token.", "muted");
  });

  $("btnDistStartWorker")?.addEventListener("click", async () => {
    if (RUNNING) return;
    try {
      await startDistWorkerServer();
      saveDistSettingsSoon();
      setStatus("Worker server started.", "ok");
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });
  $("btnDistStopWorker")?.addEventListener("click", async () => {
    if (RUNNING) return;
    try {
      await stopDistWorkerServer();
      setStatus("Worker server stopped.", "muted");
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });
  $("btnDistWorkerStatus")?.addEventListener("click", async () => {
    if (RUNNING) return;
    await refreshDistWorkerStatus();
  });

  $("btnDistFillFromPolygon")?.addEventListener("click", async () => {
    if (RUNNING) return;
    const invoke = tauriInvoke();
    if (!invoke) return;
    try {
      const info = await invoke("polygon_get_cache_info");
      const ds = Array.isArray(info?.datasets) ? info.datasets : [];
      const paths = ds.map((d) => String(d?.csv_path || "").trim()).filter((p) => !!p);
      if ($("distAssetsCsvs")) $("distAssetsCsvs").value = paths.join("\n");
      saveDistSettingsSoon();
      setStatus(`Loaded ${paths.length} Polygon CSV paths (local paths; use rewrite or UNC).`, "muted");
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });

  $("btnDistUseCurrentCsv")?.addEventListener("click", () => {
    if ($("distTileCsvPath")) $("distTileCsvPath").value = String($("csvPath")?.value || "").trim();
    saveDistSettingsSoon();
  });

  $("btnDistRunAssets")?.addEventListener("click", async () => {
    try {
      await runDistributedAssets();
    } catch (e) {
      if (isCancelledError(e)) setStatus("Distributed assets: cancelled.", "muted");
      else setStatus(String(e), "danger");
    }
  });
  $("btnDistRunTiles")?.addEventListener("click", async () => {
    try {
      await runDistributedTiles();
    } catch (e) {
      if (isCancelledError(e)) setStatus("Distributed tiles: cancelled.", "muted");
      else setStatus(String(e), "danger");
    }
  });

  $("btnDistCancel")?.addEventListener("click", async () => {
    if (!RUNNING) return;
    try {
      setStatus("Cancel requested... (may take a moment to stop safely)", "muted");
      await requestCancelCurrentRun();
    } catch (e) {
      setStatus(String(e), "danger");
    }
  });
}

function getCompositeMoRequest() {
  const dataIdText = $("dataId").textContent.trim();
  const dataId = dataIdText === "—" ? "" : dataIdText;
  if (!dataId || dataId === "-") throw new Error("Load CSV first (missing data id).");

  const meta = CURRENT_DATA_META;
  if (!meta || typeof meta !== "object") throw new Error("Load CSV first.");
  if (!meta.has_high || !meta.has_low) {
    throw new Error("Composite strategy requires high/low candle fields. Load a CSV with high+low columns.");
  }

  const fast = [
    Number($("fastStart").value),
    Number($("fastEnd").value),
    Number($("fastStep").value),
  ];
  const slow = [
    Number($("slowStart").value),
    Number($("slowEnd").value),
    Number($("slowStep").value),
  ];

  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  if (!pairs) throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = ($("fastMaType")?.value || "").trim();
  const slowMaType = ($("slowMaType")?.value || "").trim();
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");

  if (!maSupportsCpu(fastMaType) || !maSupportsCpu(slowMaType)) {
    throw new Error("Composite strategy is CPU-only and does not support one of the selected MA types.");
  }

  const maSource = $("maSource").value;
  validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource);

  const baseStrategy = readStrategyCfgFromUi();
  const signal_mode = readCompositeSignalMode();

  const maximize = $("objective").value;
  const minimize = $("moObjectiveMin")?.value || "MaxDrawdown";

  const samplerKind = String($("moSampler")?.value || "Nsga2");
  let sampler = null;
  if (samplerKind === "Random") {
    const seed = Math.max(0, Math.floor(num($("moRandomSeed")?.value, 1)));
    const trials = Math.max(1, Math.floor(num($("moRandomTrials")?.value, 2000)));
    if (trials > 200_000) throw new Error("Random trials too large (max 200000).");
    sampler = { sampler: "Random", config: { seed, trials } };
  } else if (samplerKind === "Tpe") {
    const seed = Math.max(0, Math.floor(num($("moTpeSeed")?.value, 1)));
    const trials = Math.max(1, Math.floor(num($("moTpeTrials")?.value, 5000)));
    const startup_trials = Math.max(0, Math.floor(num($("moTpeStartupTrials")?.value, 10)));
    const n_ei_candidates = Math.max(1, Math.floor(num($("moTpeCandidates")?.value, 24)));
    const gamma = num($("moTpeGamma")?.value, 0.25);
    const batch_size = Math.max(1, Math.floor(num($("moTpeBatchSize")?.value, 64)));
    const max_model_points = Math.max(32, Math.floor(num($("moTpeMaxModelPoints")?.value, 2048)));
    if (trials > 200_000) throw new Error("TPE trials too large (max 200000).");
    if (!Number.isFinite(gamma) || gamma <= 0 || gamma >= 1) throw new Error("TPE gamma must be in (0, 1).");
    if (n_ei_candidates > 512) throw new Error("TPE EI candidates too large (max 512).");
    sampler = {
      sampler: "Tpe",
      config: { seed, trials, startup_trials, n_ei_candidates, gamma, batch_size, max_model_points },
    };
  } else if (samplerKind === "Qmc") {
    const seed = Math.max(0, Math.floor(num($("moQmcSeed")?.value, 1)));
    const trials = Math.max(1, Math.floor(num($("moQmcTrials")?.value, 5000)));
    const sequence = String($("moQmcSequence")?.value || "sobol").trim().toLowerCase() || "sobol";
    if (trials > 200_000) throw new Error("QMC trials too large (max 200000).");
    sampler = { sampler: "Qmc", config: { seed, trials, sequence } };
  } else {
    const seed = Math.max(0, Math.floor(num($("moNsga2Seed")?.value, 1)));
    const population_size = Math.max(2, Math.floor(num($("moNsga2Population")?.value, 80)));
    const generations = Math.max(1, Math.floor(num($("moNsga2Generations")?.value, 40)));
    const crossover_prob = num($("moNsga2CrossoverProb")?.value, 0.9);
    const swapping_prob = num($("moNsga2SwappingProb")?.value, 0.5);
    const mutRaw = String($("moNsga2MutationProb")?.value || "").trim();
    const mutation_prob = mutRaw ? num(mutRaw, NaN) : null;
    if (!Number.isFinite(crossover_prob) || crossover_prob < 0 || crossover_prob > 1) throw new Error("NSGA-II crossover_prob must be in [0, 1].");
    if (!Number.isFinite(swapping_prob) || swapping_prob < 0 || swapping_prob > 1) throw new Error("NSGA-II swapping_prob must be in [0, 1].");
    if (mutation_prob != null && (!Number.isFinite(mutation_prob) || mutation_prob < 0 || mutation_prob > 1)) {
      throw new Error("NSGA-II mutation_prob must be blank or in [0, 1].");
    }
    const evals = population_size * (generations + 1);
    if (evals > 200_000) throw new Error(`NSGA-II budget too large (${evals} evals; max 200000).`);
    sampler = {
      sampler: "Nsga2",
      config: {
        seed,
        population_size,
        generations,
        crossover_prob,
        swapping_prob,
        mutation_prob: mutation_prob == null ? null : mutation_prob,
      },
    };
  }

  const minTrades = optNum($("moConstraintMinTrades")?.value);
  const maxDd = optNum($("moConstraintMaxDd")?.value);
  const maxAbsExpo = optNum($("moConstraintMaxAbsExposure")?.value);
  let constraints = null;
  if (Number.isFinite(minTrades) || Number.isFinite(maxDd) || Number.isFinite(maxAbsExpo)) {
    constraints = {};
    if (Number.isFinite(minTrades)) {
      if (minTrades < 0) throw new Error("Min trades must be >= 0.");
      constraints.min_trades = Math.floor(minTrades);
    }
    if (Number.isFinite(maxDd)) {
      if (maxDd < 0) throw new Error("Max drawdown constraint must be >= 0.");
      constraints.max_drawdown = maxDd;
    }
    if (Number.isFinite(maxAbsExpo)) {
      if (maxAbsExpo < 0) throw new Error("Max abs exposure constraint must be >= 0.");
      constraints.max_abs_exposure = maxAbsExpo;
    }
  }

  let gpu_prescan = null;
  const prescanOn = !!$("moPrescanEnable")?.checked;
  if (prescanOn) {
    const device_id = Math.max(0, Math.floor(num($("deviceId")?.value, 0)));
    const top_k = Math.max(1, Math.floor(num($("moPrescanTopK")?.value, 128)));
    const window = Math.max(0, Math.floor(num($("moPrescanWindow")?.value, 8)));
    const predictive_top_k = Math.max(0, Math.floor(num($("moPrescanPredictiveTopK")?.value, 12)));
    const predictive_stride = Math.max(1, Math.min(65535, Math.floor(num($("moPrescanPredictiveStride")?.value, 8))));
    const objectives = String($("moPrescanObjectives")?.value || "both").trim();
    const mode = String($("moPrescanMode")?.value || "Auto").trim();
    const ma_pairs = parseCompositeMoPrescanMaPairs($("moPrescanMaPairs")?.value || "");
    gpu_prescan = {
      top_k,
      window,
      predictive_top_k,
      predictive_stride,
      device_id,
      require_gpu: !!$("moPrescanRequireGpu")?.checked,
      objectives,
      ma_pairs,
      mode,
    };
  }

  const useRangeBuilder = !!$("moUseRangeBuilder")?.checked;
  const space = useRangeBuilder ? buildCompositeMoSearchSpaceFromUi() : parseCompositeMoSearchSpace();
  if (useRangeBuilder && $("moSearchSpace")) {
    $("moSearchSpace").value = JSON.stringify(space, null, 2);
  }
  const rawFastStep = Math.floor(num($("fastStep")?.value, 1));
  const rawSlowStep = Math.floor(num($("slowStep")?.value, 1));
  const fastStep = Math.max(1, rawFastStep);
  const slowStep = Math.max(1, rawSlowStep);
  const fastLow = Math.floor(fast[0]);
  const slowLow = Math.floor(slow[0]);
  const fastHigh = rawFastStep === 0 ? fastLow : Math.floor(fast[1]);
  const slowHigh = rawSlowStep === 0 ? slowLow : Math.floor(slow[1]);
  const fastDomain = { IntRange: { low: fastLow, high: fastHigh, step: fastStep, log: false } };
  const slowDomain = { IntRange: { low: slowLow, high: slowHigh, step: slowStep, log: false } };
  const search_space = normalizeCompositeMoSearchSpace(space, fastDomain, slowDomain);

  return {
    strategy_kind: "CompositeMo",
    data_id: dataId,
    starting_equity: readStartingEquityFromUi(),
    fast_ma_type: fastMaType,
    slow_ma_type: slowMaType,
    ma_source: maSource,
    fast_ma_params: resolvedMaParams("fast", fastMaType),
    slow_ma_params: resolvedMaParams("slow", slowMaType),
    base_strategy: baseStrategy,
    signal_mode,
    strategy: baseStrategy,
    search_space,
    objective: maximize,
    objectives: { maximize, minimize },
    sampler,
    constraints,
    gpu_prescan,
    mode: null,
  };
}

function getCompositeRequest() {
  const dataIdText = $("dataId").textContent.trim();
  const dataId = dataIdText === "—" ? "" : dataIdText;
  if (!dataId || dataId === "-") throw new Error("Load CSV first (missing data id).");

  const meta = CURRENT_DATA_META;
  if (!meta || typeof meta !== "object") throw new Error("Load CSV first.");
  if (!meta.has_high || !meta.has_low) {
    throw new Error("Composite strategy requires high/low candle fields. Load a CSV with high+low columns.");
  }

  const fast = [
    Number($("fastStart").value),
    Number($("fastEnd").value),
    Number($("fastStep").value),
  ];
  const slow = [
    Number($("slowStart").value),
    Number($("slowEnd").value),
    Number($("slowStep").value),
  ];

  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  if (!pairs) throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = ($("fastMaType")?.value || "").trim();
  const slowMaType = ($("slowMaType")?.value || "").trim();
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");

  if (!maSupportsCpu(fastMaType) || !maSupportsCpu(slowMaType)) {
    throw new Error("Composite strategy is CPU-only and does not support one of the selected MA types.");
  }

  const maSource = $("maSource").value;
  validateCandleFieldRequirements(meta, fastMaType, slowMaType, maSource);

  const amp = Math.floor(num($("halfTrendAmplitude")?.value, 2));
  const atrP = Math.floor(num($("halfTrendAtrPeriod")?.value, 100));
  const dev = num($("halfTrendChannelDev")?.value, 2.0);
  if (!Number.isFinite(amp) || amp < 1 || amp > 65535) throw new Error("HalfTrend amplitude must be in [1, 65535].");
  if (!Number.isFinite(atrP) || atrP < 1 || atrP > 65535) throw new Error("HalfTrend ATR period must be in [1, 65535].");
  if (!Number.isFinite(dev) || dev < 0) throw new Error("HalfTrend channel deviation must be finite and >= 0.");
  const halftrend = { amplitude: amp, atr_period: atrP, channel_deviation: dev };

  const maczFast = Math.floor(num($("maczFastLength")?.value, 12));
  const maczSlow = Math.floor(num($("maczSlowLength")?.value, 25));
  const maczSig = Math.floor(num($("maczSignalLength")?.value, 9));
  const maczLz = Math.floor(num($("maczLengthZ")?.value, 20));
  const maczLsd = Math.floor(num($("maczLengthStdev")?.value, 25));
  const maczGamma = num($("maczGamma")?.value, 0.02);
  const maczA = num($("maczA")?.value, 1.0);
  const maczB = num($("maczB")?.value, 1.0);
  const maczUseVolume = !!$("maczUseVolume")?.checked;
  const maczUseLag = !!$("maczUseLag")?.checked;

  if (!Number.isFinite(maczFast) || maczFast < 1) throw new Error("MACZ fast_length must be >= 1.");
  if (!Number.isFinite(maczSlow) || maczSlow < 1) throw new Error("MACZ slow_length must be >= 1.");
  if (maczFast >= maczSlow) throw new Error("MACZ requires fast_length < slow_length.");
  if (!Number.isFinite(maczSig) || maczSig < 1) throw new Error("MACZ signal_length must be >= 1.");
  if (!Number.isFinite(maczLz) || maczLz < 1) throw new Error("MACZ lengthz must be >= 1.");
  if (!Number.isFinite(maczLsd) || maczLsd < 1) throw new Error("MACZ length_stdev must be >= 1.");
  if (!Number.isFinite(maczA) || maczA < -2 || maczA > 2) throw new Error("MACZ a must be in [-2, 2].");
  if (!Number.isFinite(maczB) || maczB < -2 || maczB > 2) throw new Error("MACZ b must be in [-2, 2].");
  if (!Number.isFinite(maczGamma) || maczGamma < 0 || maczGamma >= 1) throw new Error("MACZ gamma must be in [0, 1).");
  if (maczUseVolume && !meta.has_volume) throw new Error("MACZ use_volume requires volume column in the CSV.");

  const macz = {
    fast_length: maczFast,
    slow_length: maczSlow,
    signal_length: maczSig,
    lengthz: maczLz,
    length_stdev: maczLsd,
    a: maczA,
    b: maczB,
    use_lag: maczUseLag,
    gamma: maczGamma,
    use_volume: maczUseVolume,
  };

  const ottPeriod = Math.floor(num($("ottPeriod")?.value, 2));
  const ottPercent = num($("ottPercent")?.value, 1.4);
  const ottMaType = String($("ottMaType")?.value || "VAR").trim();
  if (!Number.isFinite(ottPeriod) || ottPeriod < 1) throw new Error("OTT period must be >= 1.");
  if (!Number.isFinite(ottPercent) || ottPercent < 0) throw new Error("OTT percent must be finite and >= 0.");
  if (!ottMaType) throw new Error("OTT MA type is required.");
  const ott = { period: ottPeriod, percent: ottPercent, ma_type: ottMaType };

  const objective = $("objective").value;
  const topK = Number($("topK").value || 0);
  const includeAll = $("includeAll").value === "true";
  if (includeAll && pairs > 2_000_000) {
    throw new Error(`includeAll=true would return ${pairs} rows; disable it or reduce ranges.`);
  }

  const heatmapBins = 0;

  const strategy = readStrategyCfgFromUi();
  const signal_mode = readCompositeSignalMode();

  return {
    strategy_kind: "CompositeCpu",
    backend: { backend: "CpuOnly" },
    data_id: dataId,
    starting_equity: readStartingEquityFromUi(),
    fast_range: fast,
    slow_range: slow,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    fast_ma_params: resolvedMaParams("fast", fastMaType),
    slow_ma_params: resolvedMaParams("slow", slowMaType),
    strategy,
    signal_mode,
    halftrend,
    macz,
    ott,
    objective,
    mode: "Grid",
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
  };
}

function getRequest() {
  const kind = strategyKind();
  return kind === "CompositeCpu" ? getCompositeRequest() : (kind === "CompositeMo" ? getCompositeMoRequest() : getDoubleMaRequest());
}

function kindFromReq(req) {
  const kindRaw = String(req?.strategy_kind || req?.strategyKind || "DoubleMa");
  return normalizeStrategyKindRaw(kindRaw);
}

function setExecStatus(msg, kind) {
  const el = $("execStatusHint");
  if (!el) return;
  el.textContent = String(msg || "-");
  el.classList.remove("goodText", "badText");
  if (kind === "ok") el.classList.add("goodText");
  else if (kind === "danger") el.classList.add("badText");
}

function setExecLiveStatus(msg, kind) {
  const el = $("execLiveStatusHint");
  if (!el) return;
  el.textContent = String(msg || "-");
  el.classList.remove("goodText", "badText");
  if (kind === "ok") el.classList.add("goodText");
  else if (kind === "danger") el.classList.add("badText");
}

function setExecRunnerStatus(msg, kind) {
  const el = $("execRunnerStatusHint");
  if (!el) return;
  el.textContent = String(msg || "-");
  el.classList.remove("goodText", "badText");
  if (kind === "ok") el.classList.add("goodText");
  else if (kind === "danger") el.classList.add("badText");
}

function setExecOutput(value) {
  const el = $("execOutput");
  if (!el) return;
  if (typeof value === "string") {
    el.textContent = value;
    return;
  }
  try {
    el.textContent = pretty(value);
  } catch {
    el.textContent = String(value);
  }
}

function normalizeExecSymbolKey(s) {
  return String(s || "").toUpperCase().replace(/[^A-Z0-9]/g, "");
}

function execOrderTimeSec(order) {
  const o = order && typeof order === "object" ? order : {};
  const picks = [o.filled_at, o.submitted_at, o.updated_at, o.canceled_at, o.created_at];
  for (const raw of picks) {
    if (!raw) continue;
    if (typeof raw === "number") {
      const abs = Math.abs(raw);
      if (!Number.isFinite(abs) || abs <= 0) continue;
      if (abs >= 1e12) return Math.floor(raw / 1000);
      return Math.floor(raw);
    }
    const ms = Date.parse(String(raw));
    if (Number.isFinite(ms)) return Math.floor(ms / 1000);
  }
  return NaN;
}

function fmtExecOrderTime(order) {
  const sec = execOrderTimeSec(order);
  return Number.isFinite(sec) ? formatUtcTimeShortFromSec(sec) : "-";
}

function shortExecOrderId(id) {
  const s = String(id || "").trim();
  if (!s) return "-";
  if (s.length <= 12) return s;
  return `${s.slice(0, 8)}…`;
}

function nearestExecBarTimeSec(targetSec, bars) {
  if (!Array.isArray(bars) || !bars.length) return null;
  const t = Number(targetSec);
  if (!Number.isFinite(t)) return null;
  let lo = 0;
  let hi = bars.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const mt = Number(bars[mid]?.time);
    if (!Number.isFinite(mt)) return null;
    if (t < mt) hi = mid - 1;
    else if (t > mt) lo = mid + 1;
    else return mt;
  }
  if (hi < 0) return Number(bars[0]?.time);
  if (lo >= bars.length) return Number(bars[bars.length - 1]?.time);
  const lt = Number(bars[hi]?.time);
  const rt = Number(bars[lo]?.time);
  return Math.abs(t - lt) <= Math.abs(rt - t) ? lt : rt;
}

function buildExecOrderMarkers(snapshot, bars) {
  const orders = Array.isArray(snapshot?.orders) ? snapshot.orders : [];
  if (!orders.length || !Array.isArray(bars) || !bars.length) return [];
  const out = [];
  for (const o0 of orders) {
    const o = o0 && typeof o0 === "object" ? o0 : {};
    const sec = execOrderTimeSec(o);
    const time = nearestExecBarTimeSec(sec, bars);
    if (!Number.isFinite(time)) continue;

    const side = String(o.side || "").trim().toLowerCase();
    const status = String(o.status || "").trim().toLowerCase();
    const isBuy = side === "buy";
    const isSell = side === "sell";
    const isCancelled = status.includes("cancel");
    const isRejected = status.includes("reject");
    const isFilled = status === "filled" || status === "partially_filled";
    const isOpen = status === "new" || status === "accepted" || status === "pending_new";

    let color = "rgba(148, 163, 184, 0.9)";
    if (isRejected) color = "rgba(245, 158, 11, 0.95)";
    else if (isCancelled) color = "rgba(100, 116, 139, 0.9)";
    else if (isFilled && isBuy) color = "rgba(22, 163, 74, 0.95)";
    else if (isFilled && isSell) color = "rgba(225, 29, 72, 0.95)";
    else if (isOpen) color = "rgba(2, 132, 199, 0.95)";
    else if (isBuy) color = "rgba(22, 163, 74, 0.95)";
    else if (isSell) color = "rgba(225, 29, 72, 0.95)";

    const qtyRaw = Number(o.filled_qty ?? o.qty ?? NaN);
    const qty = Number.isFinite(qtyRaw) ? qtyRaw : NaN;
    const qtyText = Number.isFinite(qty) ? String(qty) : "-";
    const statusText = status || "unknown";

    out.push({
      time,
      position: isBuy ? "belowBar" : "aboveBar",
      shape: isFilled ? (isBuy ? "arrowUp" : "arrowDown") : "circle",
      color,
      text: `${isBuy ? "B" : (isSell ? "S" : "?")} ${qtyText} ${statusText}`,
      size: isFilled ? 2 : 1,
    });
  }
  out.sort((a, b) => Number(a.time) - Number(b.time));
  if (out.length > 4000) return out.slice(out.length - 4000);
  return out;
}

function execCurrentSymbolKey() {
  const fromCfg = normalizeExecSymbolKey(EXEC_LIVE_STREAM_CFG?.symbol || "");
  if (fromCfg) return fromCfg;
  const fromUi = normalizeExecSymbolKey($("execSymbol")?.value || "");
  if (fromUi) return fromUi;
  return normalizeExecSymbolKey(CURRENT_DATASET_CTX?.symbol || "");
}

function execMergedBars() {
  const map = new Map();
  const base = Array.isArray(EXEC_TV_BASE_BARS) ? EXEC_TV_BASE_BARS : [];
  for (const b0 of base) {
    const t = Number(b0?.time);
    if (!Number.isFinite(t)) continue;
    map.set(t, {
      time: t,
      open: Number(b0.open),
      high: Number(b0.high),
      low: Number(b0.low),
      close: Number(b0.close),
    });
  }
  for (const [t0, b0] of EXEC_TV_LIVE_BARS.entries()) {
    const t = Number(t0);
    if (!Number.isFinite(t)) continue;
    const b = b0 && typeof b0 === "object" ? b0 : {};
    map.set(t, {
      time: t,
      open: Number(b.open),
      high: Number(b.high),
      low: Number(b.low),
      close: Number(b.close),
    });
  }
  const out = [...map.values()]
    .filter((b) => Number.isFinite(b.time) && Number.isFinite(b.open) && Number.isFinite(b.high) && Number.isFinite(b.low) && Number.isFinite(b.close))
    .sort((a, b) => a.time - b.time);
  if (out.length > 12000) return out.slice(out.length - 12000);
  return out;
}

function rebuildExecBaseTimeSet() {
  const out = new Set();
  const base = Array.isArray(EXEC_TV_BASE_BARS) ? EXEC_TV_BASE_BARS : [];
  for (const b0 of base) {
    const t = Number(b0?.time);
    if (!Number.isFinite(t)) continue;
    out.add(t);
  }
  EXEC_TV_BASE_TIME_SET = out;
}

function recomputeExecMergedCount() {
  const baseN = EXEC_TV_BASE_TIME_SET && typeof EXEC_TV_BASE_TIME_SET.size === "number"
    ? EXEC_TV_BASE_TIME_SET.size
    : 0;
  let merged = baseN;
  for (const t0 of EXEC_TV_LIVE_BARS.keys()) {
    const t = Number(t0);
    if (!Number.isFinite(t)) continue;
    if (!EXEC_TV_BASE_TIME_SET.has(t)) merged++;
  }
  EXEC_TV_MERGED_COUNT = Math.max(0, merged);
}

function updateExecChartMeta(snapshot, opts) {
  const meta = $("execTvChartMeta");
  if (!meta) return;
  const baseN = EXEC_TV_BASE_TIME_SET && typeof EXEC_TV_BASE_TIME_SET.size === "number"
    ? EXEC_TV_BASE_TIME_SET.size
    : (Array.isArray(EXEC_TV_BASE_BARS) ? EXEC_TV_BASE_BARS.length : 0);
  const liveN = EXEC_TV_LIVE_BARS.size;
  const mergedN = Number.isFinite(Number(opts?.mergedCount))
    ? Math.max(0, Math.floor(Number(opts.mergedCount)))
    : Math.max(0, Math.floor(Number(EXEC_TV_MERGED_COUNT || 0)));
  const markersN = Number.isFinite(Number(opts?.markerCount))
    ? Math.max(0, Math.floor(Number(opts.markerCount)))
    : Math.max(0, Math.floor(Number(EXEC_TV_MARKER_COUNT || 0)));
  const ordersN = Array.isArray(snapshot?.orders) ? snapshot.orders.length : 0;
  meta.textContent = `bars=${mergedN} (base=${baseN} live=${liveN}) orders=${ordersN} markers=${markersN}`;
}

function tryExecSeriesIncrementalUpdate(candle) {
  if (!EXEC_TV_SERIES || typeof EXEC_TV_SERIES.update !== "function") return false;
  const c = candle && typeof candle === "object" ? candle : null;
  if (!c) return false;
  try {
    EXEC_TV_SERIES.update(c);
    return true;
  } catch {
    return false;
  }
}

function updateExecChartSeries(snapshot, opts) {
  if (!EXEC_TV_SERIES) return;
  const merged = execMergedBars();
  EXEC_TV_SERIES.setData(merged);
  EXEC_TV_MERGED_COUNT = merged.length;
  const markers = buildExecOrderMarkers(snapshot, merged);
  EXEC_TV_MARKER_COUNT = markers.length;
  if (EXEC_TV_MARKERS && typeof EXEC_TV_MARKERS.setMarkers === "function") {
    EXEC_TV_MARKERS.setMarkers(markers);
  }
  updateExecChartMeta(snapshot, { mergedCount: merged.length, markerCount: markers.length });
  if (opts?.fit) {
    try {
      EXEC_TV_CHART?.timeScale?.().fitContent?.();
    } catch {}
  }
}

function clearExecLiveBars(opts) {
  EXEC_TV_LIVE_BARS.clear();
  recomputeExecMergedCount();
  if (opts?.refresh && EXEC_TV_SERIES) {
    updateExecChartSeries(EXEC_LAST_SNAPSHOT, { fit: false });
  } else {
    updateExecChartMeta(EXEC_LAST_SNAPSHOT);
  }
}

function execCandleFromLivePayload(payload) {
  const p = payload && typeof payload === "object" ? payload : {};
  const openMs = Number(p.time_open_unix_ms);
  const tSec = Math.floor(openMs / 1000);
  const open = Number(p.open);
  const high = Number(p.high);
  const low = Number(p.low);
  const close = Number(p.close);
  if (!Number.isFinite(tSec) || !Number.isFinite(open) || !Number.isFinite(high) || !Number.isFinite(low) || !Number.isFinite(close)) {
    return null;
  }
  return { time: tSec, open, high, low, close };
}

function onExecLiveBarEvent(payload) {
  const p = payload && typeof payload === "object" ? payload : {};
  const evSym = normalizeExecSymbolKey(p.symbol || "");
  const curSym = execCurrentSymbolKey();
  if (curSym && evSym && curSym !== evSym) return;

  const candle = execCandleFromLivePayload(p);
  if (!candle) return;
  const t = Number(candle.time);
  const hadLive = EXEC_TV_LIVE_BARS.has(t);
  const hadBase = EXEC_TV_BASE_TIME_SET.has(t);
  EXEC_TV_LIVE_BARS.set(t, candle);
  if (!hadLive && !hadBase) {
    EXEC_TV_MERGED_COUNT = Math.max(0, Number(EXEC_TV_MERGED_COUNT || 0) + 1);
  }
  if (EXEC_TV_SERIES) {
    // Fast path for realtime bars; fall back to full merge on out-of-order updates.
    const updated = tryExecSeriesIncrementalUpdate(candle);
    if (!updated) {
      updateExecChartSeries(EXEC_LAST_SNAPSHOT, { fit: false });
    } else {
      updateExecChartMeta(EXEC_LAST_SNAPSHOT);
    }
  } else {
    updateExecChartMeta(EXEC_LAST_SNAPSHOT);
  }
  const when = formatUtcTimeShortFromSec(Number(candle.time));
  const delayed = !!p.delayed;
  setExecLiveStatus(`Live candle: ${String(p.symbol || "-")} @ ${when}${delayed ? " (delayed)" : ""}`, "ok");
}

function onExecLiveStatusEvent(payload) {
  const p = payload && typeof payload === "object" ? payload : {};
  const status = String(p.status || "").trim().toLowerCase();
  const message = String(p.message || "").trim();
  const symbol = String(p.symbol || "").trim() || String(EXEC_LIVE_STREAM_CFG?.symbol || "").trim();
  if (status === "error") {
    EXEC_LIVE_STREAM_ACTIVE = false;
    setExecLiveStatus(`Live candles error (${symbol || "-"})${message ? `: ${message}` : ""}`, "danger");
    return;
  }
  if (status === "closed") {
    EXEC_LIVE_STREAM_ACTIVE = false;
    setExecLiveStatus(`Live candles stopped (${symbol || "-"})${message ? `: ${message}` : ""}`, "muted");
    return;
  }
  if (status === "subscribed" || status === "connected" || status === "auth_success") {
    EXEC_LIVE_STREAM_ACTIVE = true;
    setExecLiveStatus(`Live candles active (${symbol || "-"})${message ? `: ${message}` : ""}`, "ok");
    return;
  }
  setExecLiveStatus(`Live candles: ${status || "status"}${message ? ` (${message})` : ""}`, "muted");
}

function onExecRunnerEvent(payload) {
  const p = payload && typeof payload === "object" ? payload : {};
  const status = String(p.status || "").trim().toLowerCase();
  const message = String(p.message || "").trim();
  const symbol = String(p.symbol || "").trim() || String($("execSymbol")?.value || "").trim().toUpperCase();
  const execution = p.execution && typeof p.execution === "object" ? p.execution : null;

  if (status === "started") EXEC_RUNNER_ACTIVE = true;
  else if (status === "stopped") EXEC_RUNNER_ACTIVE = false;

  if (execution) {
    setExecOutput(execution);
    if (execution.order_submitted) {
      refreshExecutionSnapshotUi({ showOutput: false, silentStatus: true }).catch(() => {});
    }
  }

  if (status === "error") {
    setExecRunnerStatus(`Auto-runner error (${symbol || "-"})${message ? `: ${message}` : ""}`, "danger");
    setExecStatus(`Auto-runner error: ${message || "execution failed"}`, "danger");
    return;
  }

  if (status === "order_submitted") {
    setExecRunnerStatus(`Auto-runner submitted order (${symbol || "-"})${message ? `: ${message}` : ""}`, "ok");
    setExecStatus(`Auto-runner submitted order for ${symbol || "-"}.`, "ok");
    return;
  }

  if (status === "triggered") {
    setExecRunnerStatus(`Auto-runner triggered (${symbol || "-"})${message ? `: ${message}` : ""}`, "muted");
    return;
  }

  if (status === "signal_processed") {
    setExecRunnerStatus(`Auto-runner processed signal (${symbol || "-"})${message ? `: ${message}` : ""}`, "muted");
    return;
  }

  if (status === "started") {
    setExecRunnerStatus(`Auto-runner active (${symbol || "-"})${message ? `: ${message}` : ""}`, "ok");
    return;
  }

  if (status === "stopped") {
    setExecRunnerStatus(`Auto-runner stopped${message ? `: ${message}` : ""}`, "muted");
    return;
  }

  if (status === "skipped_busy") {
    setExecRunnerStatus(`Auto-runner waiting (${symbol || "-"})`, "muted");
    return;
  }

  setExecRunnerStatus(`Auto-runner: ${status || "status"}${message ? ` (${message})` : ""}`, "muted");
}

async function initExecLiveStreamListeners() {
  if (EXEC_LIVE_LISTENERS_READY) return;
  const listen = tauriListen();
  if (!listen) return;
  EXEC_LIVE_LISTENERS_READY = true;
  try {
    await listen("polygon_live_bar", (event) => {
      try {
        onExecLiveBarEvent(event?.payload);
      } catch {}
    });
  } catch {}
  try {
    await listen("polygon_live_status", (event) => {
      try {
        onExecLiveStatusEvent(event?.payload);
      } catch {}
    });
  } catch {}
  try {
    await listen("live_execution_runner_event", (event) => {
      try {
        onExecRunnerEvent(event?.payload);
      } catch {}
    });
  } catch {}
}

function ensureExecTvChart() {
  const host = $("execTvChart");
  if (!host) return null;
  if (EXEC_TV_CHART && EXEC_TV_SERIES) return { chart: EXEC_TV_CHART, series: EXEC_TV_SERIES, host };

  const width = Math.max(240, Math.floor(host.clientWidth || 820));
  const height = Math.max(220, Math.floor(host.clientHeight || 300));
  EXEC_TV_CHART = createChart(host, { ...drillChartThemeOptions(), width, height });
  const candleOpts = {
    upColor: "rgba(22, 163, 74, 0.9)",
    downColor: "rgba(225, 29, 72, 0.9)",
    wickUpColor: "rgba(22, 163, 74, 0.9)",
    wickDownColor: "rgba(225, 29, 72, 0.9)",
    borderVisible: true,
    borderUpColor: "rgba(22, 163, 74, 1)",
    borderDownColor: "rgba(225, 29, 72, 1)",
  };
  if (typeof EXEC_TV_CHART.addSeries === "function" && CandlestickSeries) {
    try {
      EXEC_TV_SERIES = EXEC_TV_CHART.addSeries(CandlestickSeries, candleOpts);
    } catch {
      EXEC_TV_SERIES = null;
    }
  }
  if (!EXEC_TV_SERIES && typeof EXEC_TV_CHART.addCandlestickSeries === "function") {
    EXEC_TV_SERIES = EXEC_TV_CHART.addCandlestickSeries(candleOpts);
  }
  if (!EXEC_TV_SERIES) throw new Error("Lightweight Charts candlestick API unavailable.");
  EXEC_TV_MARKERS = createSeriesMarkers(EXEC_TV_SERIES, []);

  if (typeof ResizeObserver !== "undefined") {
    EXEC_TV_RESIZE_OBSERVER = new ResizeObserver(() => {
      if (!EXEC_TV_CHART) return;
      const rect = host.getBoundingClientRect();
      if (!rect.width || !rect.height) return;
      EXEC_TV_CHART.applyOptions({
        width: Math.max(240, Math.floor(rect.width)),
        height: Math.max(220, Math.floor(rect.height)),
      });
    });
    EXEC_TV_RESIZE_OBSERVER.observe(host);
  }

  return { chart: EXEC_TV_CHART, series: EXEC_TV_SERIES, host };
}

function refreshExecTvChartTheme() {
  if (!EXEC_TV_CHART) return;
  EXEC_TV_CHART.applyOptions(drillChartThemeOptions());
}

function clearExecTvChart(msg) {
  const meta = $("execTvChartMeta");
  if (meta && msg != null) meta.textContent = String(msg);
  EXEC_TV_BASE_BARS = [];
  EXEC_TV_BASE_TIME_SET = new Set();
  EXEC_TV_LIVE_BARS.clear();
  EXEC_TV_MARKER_COUNT = 0;
  EXEC_TV_MERGED_COUNT = 0;
  if (EXEC_TV_SERIES) EXEC_TV_SERIES.setData([]);
  if (EXEC_TV_MARKERS && typeof EXEC_TV_MARKERS.setMarkers === "function") {
    EXEC_TV_MARKERS.setMarkers([]);
  }
}

function renderExecOrdersTable(snapshot) {
  const body = $("execOrdersTableBody");
  if (!body) return;
  body.innerHTML = "";
  const ordersRaw = Array.isArray(snapshot?.orders) ? snapshot.orders : [];
  const orders = [...ordersRaw].sort((a, b) => {
    const ta = execOrderTimeSec(a);
    const tb = execOrderTimeSec(b);
    if (!Number.isFinite(ta) && !Number.isFinite(tb)) return 0;
    if (!Number.isFinite(ta)) return 1;
    if (!Number.isFinite(tb)) return -1;
    return tb - ta;
  });
  if (!orders.length) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 7;
    td.className = "left muted";
    td.textContent = "No broker orders for current symbol.";
    tr.appendChild(td);
    body.appendChild(tr);
    return;
  }
  for (const o0 of orders.slice(0, 200)) {
    const o = o0 && typeof o0 === "object" ? o0 : {};
    const tr = document.createElement("tr");

    const sideRaw = String(o.side || "").trim().toLowerCase();
    const isBuy = sideRaw === "buy";
    const isSell = sideRaw === "sell";

    const tdTime = document.createElement("td");
    tdTime.className = "left mono";
    tdTime.textContent = fmtExecOrderTime(o);

    const tdSide = document.createElement("td");
    tdSide.className = `left ${isBuy ? "goodText" : (isSell ? "badText" : "")}`.trim();
    tdSide.textContent = sideRaw || "-";

    const tdStatus = document.createElement("td");
    tdStatus.className = "left";
    tdStatus.textContent = String(o.status || "-");

    const tdQty = document.createElement("td");
    tdQty.className = "mono";
    tdQty.textContent = Number.isFinite(Number(o.qty)) ? String(Number(o.qty)) : "-";

    const tdFilled = document.createElement("td");
    tdFilled.className = "mono";
    tdFilled.textContent = Number.isFinite(Number(o.filled_qty)) ? String(Number(o.filled_qty)) : "-";

    const tdPx = document.createElement("td");
    tdPx.className = "mono";
    tdPx.textContent = Number.isFinite(Number(o.filled_avg_price)) ? fmtPrice(Number(o.filled_avg_price)) : "-";

    const tdId = document.createElement("td");
    tdId.className = "left mono";
    const id = String(o.id || "").trim();
    tdId.textContent = shortExecOrderId(id);
    if (id) tdId.title = id;

    tr.appendChild(tdTime);
    tr.appendChild(tdSide);
    tr.appendChild(tdStatus);
    tr.appendChild(tdQty);
    tr.appendChild(tdFilled);
    tr.appendChild(tdPx);
    tr.appendChild(tdId);
    body.appendChild(tr);
  }
}

function renderExecPositionMeta(snapshot) {
  const el = $("execPositionMeta");
  if (!el) return;
  const pos = snapshot?.position && typeof snapshot.position === "object" ? snapshot.position : null;
  if (!pos || !Number.isFinite(Number(pos.qty)) || Math.abs(Number(pos.qty)) < 1e-9) {
    const sym = String(snapshot?.symbol || "").trim();
    el.textContent = sym ? `Position: flat (${sym})` : "Position: flat";
    return;
  }
  const qty = Number(pos.qty);
  const side = String(pos.side || (qty >= 0 ? "long" : "short")).trim();
  const avg = Number(pos.avg_entry_price);
  const cur = Number(pos.current_price);
  const upl = Number(pos.unrealized_pl);
  const uplpc = Number(pos.unrealized_plpc);
  const parts = [
    `Position ${String(pos.symbol || snapshot?.symbol || "").trim() || "-"}`,
    `qty=${qty}`,
    `side=${side || "-"}`,
  ];
  if (Number.isFinite(avg)) parts.push(`avg=${fmtPrice(avg)}`);
  if (Number.isFinite(cur)) parts.push(`px=${fmtPrice(cur)}`);
  if (Number.isFinite(upl)) parts.push(`uPnL=${upl >= 0 ? "+" : ""}${upl.toFixed(2)}`);
  if (Number.isFinite(uplpc)) parts.push(`uPnL%=${uplpc >= 0 ? "+" : ""}${(uplpc * 100).toFixed(2)}%`);
  el.textContent = parts.join(" • ");
}

async function renderExecSnapshotChart(snapshot) {
  const meta = $("execTvChartMeta");
  const dataIdText = String($("dataId")?.textContent || "").trim();
  const dataId = dataIdText === "—" ? "" : dataIdText;
  if (!dataId || dataId === "-") {
    clearExecTvChart("Load a dataset to render execution chart.");
    return;
  }

  const bars = await fetchDrillPriceBars(dataId);
  if (!bars.length) {
    clearExecTvChart("No valid OHLC bars available for chart rendering.");
    return;
  }

  const ctx = ensureExecTvChart();
  if (!ctx) return;
  const candleData = bars.map((b) => ({
    time: b.time,
    open: b.open,
    high: b.high,
    low: b.low,
    close: b.close,
  }));
  EXEC_TV_BASE_BARS = candleData;
  rebuildExecBaseTimeSet();
  recomputeExecMergedCount();
  updateExecChartSeries(snapshot, { fit: true });
}

async function refreshExecutionSnapshotUi(opts) {
  const invoke = tauriInvoke();
  if (!invoke) {
    setExecStatus("Tauri invoke is unavailable.", "danger");
    return;
  }
  if (RUNNING || EXEC_BUSY) return;

  const mode = String($("execMode")?.value || "paper").trim().toLowerCase() === "live" ? "live" : "paper";
  const alpaca_api_key = String($("execApiKey")?.value || "").trim();
  const alpaca_api_secret = String($("execApiSecret")?.value || "").trim();
  if (!alpaca_api_key || !alpaca_api_secret) {
    setExecStatus("API key + secret are required to refresh broker orders.", "danger");
    return;
  }

  let symbol = String($("execSymbol")?.value || "").trim().toUpperCase();
  if (!symbol) {
    const fromCtx = String(CURRENT_DATASET_CTX?.symbol || "").trim().toUpperCase();
    if (fromCtx) {
      symbol = fromCtx;
      if ($("execSymbol")) $("execSymbol").value = symbol;
    }
  }

  setExecBusy(true);
  if (!opts?.silentStatus) setExecStatus(`Refreshing ${mode} orders...`, "muted");
  try {
    const resp = await invoke("get_live_execution_snapshot", {
      req: {
        mode,
        alpaca_api_key,
        alpaca_api_secret,
        symbol,
        limit: 200,
      },
    });
    EXEC_LAST_SNAPSHOT = resp;
    if (opts?.showOutput !== false) setExecOutput(resp);
    renderExecOrdersTable(resp);
    renderExecPositionMeta(resp);
    await renderExecSnapshotChart(resp);
    saveExecutionApiKeysToStorage();
    const n = Array.isArray(resp?.orders) ? resp.orders.length : 0;
    if (!opts?.silentStatus) {
      setExecStatus(`Loaded ${n} order${n === 1 ? "" : "s"} for ${symbol || "all symbols"}.`, "ok");
    }
  } catch (e) {
    const msg = String(e);
    setExecStatus(`Execution snapshot failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
}

function resolveExecLiveStreamSymbol() {
  let symbol = String($("execSymbol")?.value || "").trim().toUpperCase();
  if (!symbol) {
    const fromCtx = String(CURRENT_DATASET_CTX?.symbol || "").trim().toUpperCase();
    if (fromCtx) {
      symbol = fromCtx;
      if ($("execSymbol")) $("execSymbol").value = symbol;
    }
  }
  if (!symbol) throw new Error("Execution symbol is required for live candles (e.g. AAPL).");
  return symbol;
}

async function startExecLiveCandleStream() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const symbol = resolveExecLiveStreamSymbol();
  const market = String(CURRENT_DATASET_CTX?.market || "stocks").trim().toLowerCase() || "stocks";
  const delayed = !!$("execLiveUseDelayed")?.checked;
  if (market !== "stocks") {
    throw new Error("Live websocket candles currently support stocks only.");
  }
  setExecLiveStatus(`Starting ${delayed ? "delayed " : ""}live candles for ${symbol}...`, "muted");
  const resp = await invoke("polygon_start_live_candle_stream", {
    req: { symbol, market, delayed },
  });
  EXEC_LIVE_STREAM_ACTIVE = true;
  EXEC_LIVE_STREAM_CFG = {
    symbol: String(resp?.symbol || symbol).trim().toUpperCase(),
    market: String(resp?.market || market).trim().toLowerCase(),
    delayed: !!resp?.delayed,
    channel: String(resp?.channel || "").trim(),
  };
  clearExecLiveBars({ refresh: true });
  const channel = String(resp?.channel || "").trim();
  setExecLiveStatus(
    `Live candles started for ${EXEC_LIVE_STREAM_CFG.symbol}${channel ? ` (${channel})` : ""}${EXEC_LIVE_STREAM_CFG.delayed ? " [delayed]" : ""}.`,
    "ok",
  );
}

async function stopExecLiveCandleStream() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const resp = await invoke("polygon_stop_live_candle_stream");
  EXEC_LIVE_STREAM_ACTIVE = false;
  EXEC_LIVE_STREAM_CFG = null;
  const stopped = !!resp?.stopped;
  setExecLiveStatus(stopped ? "Live candles stopped." : "Live candles were already stopped.", "muted");
}

async function refreshExecLiveCandleStreamStatusUi(opts) {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    const resp = await invoke("polygon_live_candle_stream_status");
    const active = !!resp?.active;
    const stream = resp?.stream && typeof resp.stream === "object" ? resp.stream : null;
    EXEC_LIVE_STREAM_ACTIVE = active;
    EXEC_LIVE_STREAM_CFG = stream
      ? {
        symbol: String(stream.symbol || "").trim().toUpperCase(),
        market: String(stream.market || "").trim().toLowerCase(),
        delayed: !!stream.delayed,
      }
      : null;
    if (opts?.silent) return;
    if (active && EXEC_LIVE_STREAM_CFG?.symbol) {
      setExecLiveStatus(
        `Live candles active for ${EXEC_LIVE_STREAM_CFG.symbol}${EXEC_LIVE_STREAM_CFG.delayed ? " [delayed]" : ""}.`,
        "ok",
      );
    } else {
      setExecLiveStatus("Live candles: off", "muted");
    }
  } catch (e) {
    if (!opts?.silent) setExecLiveStatus(`Live status unavailable: ${String(e)}`, "danger");
  }
}

function summarizeExecRunnerStatus(st) {
  const s = st && typeof st === "object" ? st : {};
  const active = !!s.active;
  const cfg = s.config && typeof s.config === "object" ? s.config : null;
  if (!active || !cfg) return "Auto-runner: off";
  const mode = String(cfg.mode || "paper").toLowerCase();
  const dry = cfg.dry_run ? "dry-run" : "broker";
  const sym = String(cfg.symbol || "-").trim() || "-";
  const runs = Number(s.run_count || 0);
  const errs = Number(s.error_count || 0);
  const skips = Number(s.skip_count || 0);
  const inflight = !!s.inflight;
  return `Auto-runner: on (${sym}, ${mode}, ${dry}) runs=${runs} skips=${skips} errs=${errs}${inflight ? " [in-flight]" : ""}`;
}

async function refreshExecRunnerStatusUi(opts) {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    const st = await invoke("live_execution_runner_status");
    EXEC_RUNNER_STATUS = st;
    EXEC_RUNNER_ACTIVE = !!st?.active;
    if (!opts?.silent) {
      setExecRunnerStatus(summarizeExecRunnerStatus(st), EXEC_RUNNER_ACTIVE ? "ok" : "muted");
    }
  } catch (e) {
    if (!opts?.silent) setExecRunnerStatus(`Auto-runner status unavailable: ${String(e)}`, "danger");
  }
}

async function startExecLiveRunner() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  let payload = null;
  try {
    payload = buildExecutionInvokePayload();
  } catch (e) {
    throw new Error(String(e));
  }

  const req = payload.req;
  if (!req.dry_run && (!req.alpaca_api_key || !req.alpaca_api_secret)) {
    throw new Error("API key + secret are required unless dry-run is enabled.");
  }
  if (!EXEC_LIVE_STREAM_ACTIVE) {
    throw new Error("Start live candles first. Auto-runner triggers on Polygon live bars.");
  }

  if (!req.dry_run) {
    const modeLabel = req.mode === "live" ? "LIVE" : "PAPER";
    const confirmText =
      `Start auto-runner in ${modeLabel} mode?\n` +
      `Symbol: ${req.symbol}\n` +
      `Qty: ${req.qty}\n` +
      `This will place orders on each new live bar when signal changes.`;
    if (!confirm(confirmText)) {
      setExecRunnerStatus("Auto-runner start cancelled.", "muted");
      return;
    }
  }

  setExecRunnerStatus("Starting auto-runner...", "muted");
  const st = await invoke("start_live_execution_runner", { req });
  EXEC_RUNNER_STATUS = st;
  EXEC_RUNNER_ACTIVE = !!st?.active;
  setExecRunnerStatus(summarizeExecRunnerStatus(st), EXEC_RUNNER_ACTIVE ? "ok" : "muted");
  setExecStatus(`Auto-runner started for ${String(req.symbol || "-")}.`, "ok");
  saveExecutionApiKeysToStorage();
}

async function stopExecLiveRunner() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const resp = await invoke("stop_live_execution_runner");
  EXEC_RUNNER_STATUS = resp?.status || null;
  EXEC_RUNNER_ACTIVE = !!resp?.status?.active;
  setExecRunnerStatus(summarizeExecRunnerStatus(resp?.status), EXEC_RUNNER_ACTIVE ? "ok" : "muted");
  if (resp?.stopped) setExecStatus("Auto-runner stopped.", "muted");
  else setExecStatus("Auto-runner was already off.", "muted");
}

function updateExecParamSourceUi() {
  const src = String($("execParamSource")?.value || "selected").trim().toLowerCase();
  const manual = src === "manual";
  const disabled = !manual || RUNNING || EXEC_BUSY;
  for (const id of ["execManualFastLen", "execManualSlowLen"]) {
    const el = $(id);
    if (el) el.disabled = disabled;
  }
}

function setExecBusy(busy) {
  EXEC_BUSY = !!busy;
  const disabled = RUNNING || EXEC_BUSY;
  for (const id of ["btnExecTestConnection", "btnExecRunOnce", "btnExecClearKeys", "btnExecRefresh", "btnExecLiveStart", "btnExecLiveStop", "btnExecRunnerStart", "btnExecRunnerStop"]) {
    const el = $(id);
    if (el) el.disabled = disabled;
  }
  updateExecParamSourceUi();
}

function parseExecutionRiskPct(inputId, label) {
  const raw = String($(inputId)?.value || "").trim();
  if (!raw) return null;
  const v = Number(raw);
  if (!Number.isFinite(v) || v <= 0) throw new Error(`${label} must be finite and > 0.`);
  const frac = v > 1 ? v / 100.0 : v;
  if (!Number.isFinite(frac) || frac <= 0 || frac >= 1) throw new Error(`${label} must be in (0, 1) or a percent in (0, 100).`);
  return frac;
}

function getExecutionDataIdFromUi() {
  const dataIdText = String($("dataId")?.textContent || "").trim();
  const data_id = dataIdText === "—" ? "" : dataIdText;
  if (!data_id || data_id === "-") throw new Error("Load CSV first (missing data id).");
  return data_id;
}

function parseExecutionManualLen(inputId, fallbackInputId, label) {
  const raw = String($(inputId)?.value || "").trim();
  const base = raw ? Number(raw) : Number($(fallbackInputId)?.value);
  const x = Math.floor(base);
  if (!Number.isFinite(x) || x < 1 || x > 65535) {
    throw new Error(`${label} must be an integer in [1, 65535].`);
  }
  return x;
}

function buildManualExecutionStrategySpec() {
  const data_id = getExecutionDataIdFromUi();
  const meta = CURRENT_DATA_META;
  if (!meta || typeof meta !== "object") throw new Error("Load CSV first.");

  const fast_ma_type = String($("fastMaType")?.value || "").trim();
  const slow_ma_type = String($("slowMaType")?.value || "").trim();
  if (!fast_ma_type) throw new Error("Select a fast MA type.");
  if (!slow_ma_type) throw new Error("Select a slow MA type.");

  const fast_len = parseExecutionManualLen("execManualFastLen", "fastStart", "Manual fast length");
  const slow_len = parseExecutionManualLen("execManualSlowLen", "slowStart", "Manual slow length");
  if (fast_len >= slow_len) throw new Error("Manual parameters require fast length < slow length.");

  const ma_source = String($("maSource")?.value || "close").trim() || "close";
  validateCandleFieldRequirements(meta, fast_ma_type, slow_ma_type, ma_source);

  const fast_ma_params = resolvedMaParams("fast", fast_ma_type);
  const slow_ma_params = resolvedMaParams("slow", slow_ma_type);
  const strategy = readStrategyCfgFromUi();
  const signal_mode = readCompositeSignalMode();
  const bins = desiredDrillBins();
  const params = { fast_len, slow_len, fast_ma_type, slow_ma_type };
  const uiKind = strategyKind();

  if (isDoubleLikeKind(uiKind)) {
    if (!maSupportsCpu(fast_ma_type) || !maSupportsCpu(slow_ma_type)) {
      throw new Error("Execution currently requires CPU-supporting MA types for manual Double MA mode.");
    }
    return {
      strategy_kind: uiKind,
      strategy: {
        kind: "double_ma",
        req: {
          data_id,
          params,
          ma_source,
          fast_ma_params,
          slow_ma_params,
          strategy,
          bins,
          halftrend_filter: null,
        },
      },
    };
  }

  if (!meta.has_high || !meta.has_low) {
    throw new Error("Composite execution requires high/low candle fields in the loaded dataset.");
  }
  if (!maSupportsCpu(fast_ma_type) || !maSupportsCpu(slow_ma_type)) {
    throw new Error("Composite execution is CPU-only and does not support one of the selected MA types.");
  }

  const amp = Math.floor(num($("halfTrendAmplitude")?.value, 2));
  const atrP = Math.floor(num($("halfTrendAtrPeriod")?.value, 100));
  const dev = num($("halfTrendChannelDev")?.value, 2.0);
  if (!Number.isFinite(amp) || amp < 1 || amp > 65535) throw new Error("HalfTrend amplitude must be in [1, 65535].");
  if (!Number.isFinite(atrP) || atrP < 1 || atrP > 65535) throw new Error("HalfTrend ATR period must be in [1, 65535].");
  if (!Number.isFinite(dev) || dev < 0) throw new Error("HalfTrend channel deviation must be finite and >= 0.");
  const halftrend = { amplitude: amp, atr_period: atrP, channel_deviation: dev };

  const maczFast = Math.floor(num($("maczFastLength")?.value, 12));
  const maczSlow = Math.floor(num($("maczSlowLength")?.value, 25));
  const maczSig = Math.floor(num($("maczSignalLength")?.value, 9));
  const maczLz = Math.floor(num($("maczLengthZ")?.value, 20));
  const maczLsd = Math.floor(num($("maczLengthStdev")?.value, 25));
  const maczGamma = num($("maczGamma")?.value, 0.02);
  const maczA = num($("maczA")?.value, 1.0);
  const maczB = num($("maczB")?.value, 1.0);
  const maczUseVolume = !!$("maczUseVolume")?.checked;
  const maczUseLag = !!$("maczUseLag")?.checked;

  if (!Number.isFinite(maczFast) || maczFast < 1) throw new Error("MACZ fast_length must be >= 1.");
  if (!Number.isFinite(maczSlow) || maczSlow < 1) throw new Error("MACZ slow_length must be >= 1.");
  if (maczFast >= maczSlow) throw new Error("MACZ requires fast_length < slow_length.");
  if (!Number.isFinite(maczSig) || maczSig < 1) throw new Error("MACZ signal_length must be >= 1.");
  if (!Number.isFinite(maczLz) || maczLz < 1) throw new Error("MACZ lengthz must be >= 1.");
  if (!Number.isFinite(maczLsd) || maczLsd < 1) throw new Error("MACZ length_stdev must be >= 1.");
  if (!Number.isFinite(maczA) || maczA < -2 || maczA > 2) throw new Error("MACZ a must be in [-2, 2].");
  if (!Number.isFinite(maczB) || maczB < -2 || maczB > 2) throw new Error("MACZ b must be in [-2, 2].");
  if (!Number.isFinite(maczGamma) || maczGamma < 0 || maczGamma >= 1) throw new Error("MACZ gamma must be in [0, 1).");
  if (maczUseVolume && !meta.has_volume) throw new Error("MACZ use_volume requires volume column in the CSV.");

  const macz = {
    fast_length: maczFast,
    slow_length: maczSlow,
    signal_length: maczSig,
    lengthz: maczLz,
    length_stdev: maczLsd,
    a: maczA,
    b: maczB,
    use_lag: maczUseLag,
    gamma: maczGamma,
    use_volume: maczUseVolume,
  };

  const ottPeriod = Math.floor(num($("ottPeriod")?.value, 2));
  const ottPercent = num($("ottPercent")?.value, 1.4);
  const ottMaType = String($("ottMaType")?.value || "VAR").trim();
  if (!Number.isFinite(ottPeriod) || ottPeriod < 1) throw new Error("OTT period must be >= 1.");
  if (!Number.isFinite(ottPercent) || ottPercent < 0) throw new Error("OTT percent must be finite and >= 0.");
  if (!ottMaType) throw new Error("OTT MA type is required.");
  const ott = { period: ottPeriod, percent: ottPercent, ma_type: ottMaType };

  return {
    strategy_kind: uiKind === "CompositeMo" ? "CompositeMo" : "CompositeCpu",
    strategy: {
      kind: "composite_cpu",
      req: {
        data_id,
        params,
        ma_source,
        fast_ma_params,
        slow_ma_params,
        strategy,
        signal_mode,
        halftrend,
        macz,
        ott,
        bins,
      },
    },
  };
}

function resolveExecutionTargetRow(paramSource) {
  const source = String(paramSource || "selected").trim().toLowerCase();
  const rowsFiltered = Array.isArray(TOP_ROWS_FILTERED) ? TOP_ROWS_FILTERED : [];
  const rowsAll = Array.isArray(TOP_ROWS) ? TOP_ROWS : [];
  const fallback = rowsFiltered[0] || rowsAll[0] || null;

  if (source === "best") {
    const best = CURRENT_RUN?.res?.best_params;
    if (best && typeof best === "object") {
      const key = paramsKey(best);
      const row = findTopRowByParamsKey(key);
      if (row) return { row, source: "best" };
      return {
        row: {
          rank: 1,
          params: best,
          metrics: CURRENT_RUN?.res?.best_metrics && typeof CURRENT_RUN.res.best_metrics === "object"
            ? CURRENT_RUN.res.best_metrics
            : {},
        },
        source: "best",
      };
    }
    if (fallback) return { row: fallback, source: "best_top_fallback" };
    throw new Error("No Top rows available. Run an optimization first.");
  }

  const selectedKey = String(DRILL_SELECTED_KEY || "").trim();
  if (!selectedKey) {
    throw new Error("No Top row selected. Click a row in the Top tab or switch Parameter source to Best row.");
  }
  const selected = findTopRowByParamsKey(selectedKey);
  if (!selected) {
    throw new Error("Selected Top row is unavailable (filters changed). Select a row again or use Best row.");
  }
  return { row: selected, source: "selected" };
}

function buildExecutionInvokePayload() {
  const mode = String($("execMode")?.value || "paper").trim().toLowerCase() === "live" ? "live" : "paper";
  const dry_run = !!$("execDryRun")?.checked;
  const paramSource = String($("execParamSource")?.value || "selected").trim().toLowerCase() || "selected";
  const qtyRaw = Number($("execQty")?.value);
  if (!Number.isFinite(qtyRaw) || qtyRaw <= 0) throw new Error("Execution quantity must be finite and > 0.");
  const qty = Math.round(qtyRaw * 10_000) / 10_000;
  if (!Number.isFinite(qty) || qty <= 0) throw new Error("Execution quantity is too small.");

  let symbol = String($("execSymbol")?.value || "").trim().toUpperCase();
  if (!symbol) {
    const fromCtx = String(CURRENT_DATASET_CTX?.symbol || "").trim().toUpperCase();
    if (fromCtx) {
      symbol = fromCtx;
      if ($("execSymbol")) $("execSymbol").value = symbol;
    }
  }
  if (!symbol) throw new Error("Execution symbol is required (e.g. AAPL).");

  const risk = {
    stop_loss_pct: parseExecutionRiskPct("execRiskStopLossPct", "Risk override SL"),
    take_profit_pct: parseExecutionRiskPct("execRiskTakeProfitPct", "Risk override TP"),
  };

  let strategy = null;
  let strategy_kind = "";
  let source = paramSource;
  let params_key = "";
  if (paramSource === "manual") {
    const manual = buildManualExecutionStrategySpec();
    strategy = manual.strategy;
    strategy_kind = manual.strategy_kind;
    source = "manual_ui";
  } else {
    if (!CURRENT_RUN?.req || !CURRENT_RUN?.res) {
      throw new Error("No optimization run loaded. Use Manual parameter source or run an optimization.");
    }
    const picked = resolveExecutionTargetRow(paramSource);
    const row = picked.row;
    source = picked.source;
    const params = row?.params && typeof row.params === "object" ? row.params : null;
    if (!params) throw new Error("Unable to resolve execution parameters from Top row.");

    const drill = buildDrilldownInvocation(params, CURRENT_RUN.req);
    if (!drill?.req) throw new Error("Unable to build strategy request from selected parameters.");
    strategy = isDoubleLikeKind(drill.kind)
      ? { kind: "double_ma", req: drill.req }
      : { kind: "composite_cpu", req: drill.req };
    strategy_kind = drill.kind;
    params_key = paramsKey(params);
  }

  const req = {
    symbol,
    qty,
    mode,
    dry_run,
    risk,
    alpaca_api_key: String($("execApiKey")?.value || "").trim(),
    alpaca_api_secret: String($("execApiSecret")?.value || "").trim(),
    strategy,
  };

  return {
    req,
    meta: {
      mode,
      dry_run,
      symbol,
      qty,
      source,
      strategy_kind,
      params_key,
    },
  };
}

function parseValidationProbabilityGate(inputId) {
  const raw = String($(inputId)?.value ?? "").trim();
  if (!raw) return null;
  const v = Number(raw);
  if (!Number.isFinite(v)) return null;
  return Math.max(0, Math.min(1, v));
}

function getValidationRequest(opts) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  if (!CURRENT_RUN?.req || !CURRENT_RUN?.res) throw new Error("Run an optimization first.");

  const kind = kindFromReq(CURRENT_RUN.req);

  const dataId = String(CURRENT_RUN.req.data_id || "").trim();
  if (!dataId) throw new Error("Missing data id (run an optimization first).");

  const maxCandidates = Math.max(1, Math.floor(num($("valCandidates")?.value, 32)));
  const seed = Math.max(0, Math.floor(num($("valSeed")?.value, 1)));
  const min_psr = parseValidationProbabilityGate("valMinPsr");
  const min_dsr = parseValidationProbabilityGate("valMinDsr");

  const useFiltered = opts?.useFiltered !== false;
  const rowsFiltered = Array.isArray(TOP_ROWS_FILTERED) ? TOP_ROWS_FILTERED : [];
  const rowsAll = Array.isArray(TOP_ROWS) ? TOP_ROWS : [];
  const baseRows = (useFiltered && rowsFiltered.length) ? rowsFiltered : rowsAll;
  if (!baseRows.length) throw new Error("No Top rows available. Go to the Top tab and run an optimization.");

  const candidatesRaw = baseRows.slice(0, maxCandidates);

  const candidates = candidatesRaw.map((row) => {
    const p = row?.params && typeof row.params === "object" ? row.params : {};

    const fast_ma_type = String(p.fast_ma_type || (CURRENT_RUN.req.fast_ma_types?.[0] || "")).trim();
    const slow_ma_type = String(p.slow_ma_type || (CURRENT_RUN.req.slow_ma_types?.[0] || "")).trim();
    const fast_len = Math.max(0, Math.floor(Number(p.fast_len || 0)));
    const slow_len = Math.max(0, Math.floor(Number(p.slow_len || 0)));

    if (isDoubleLikeKind(kind)) {
      const fast_ma_params = (p.fast_ma_params && typeof p.fast_ma_params === "object") ? p.fast_ma_params : null;
      const slow_ma_params = (p.slow_ma_params && typeof p.slow_ma_params === "object") ? p.slow_ma_params : null;
      const strategy = CURRENT_RUN.req.strategy || CURRENT_RUN.req.base_strategy || {};
      const halftrend_filter = CURRENT_RUN.req.halftrend_filter || CURRENT_RUN.req.halftrendFilter || null;
      return {
        fast_ma_type,
        slow_ma_type,
        fast_len,
        slow_len,
        fast_ma_params: fast_ma_params || undefined,
        slow_ma_params: slow_ma_params || undefined,
        strategy,
        halftrend_filter: halftrend_filter || undefined,
      };
    }

    const halftrend = kind === "CompositeMo"
      ? (p.halftrend && typeof p.halftrend === "object" ? p.halftrend : null)
      : (CURRENT_RUN.req.halftrend && typeof CURRENT_RUN.req.halftrend === "object" ? CURRENT_RUN.req.halftrend : null);
    const signal_mode = kind === "CompositeMo"
      ? String(p.signal_mode || p.signalMode || CURRENT_RUN.req.signal_mode || CURRENT_RUN.req.signalMode || "hard_gate")
      : String(CURRENT_RUN.req.signal_mode || CURRENT_RUN.req.signalMode || "hard_gate");

    return {
      fast_ma_type,
      slow_ma_type,
      fast_len,
      slow_len,
      halftrend: halftrend || undefined,
      macz: kind === "CompositeMo" ? (p.macz || {}) : (CURRENT_RUN.req.macz || {}),
      ott: kind === "CompositeMo" ? (p.ott || {}) : (CURRENT_RUN.req.ott || {}),
      signal_mode,
      strategy: kind === "CompositeMo" ? (p.strategy || CURRENT_RUN.req.strategy || {}) : (CURRENT_RUN.req.strategy || {}),
    };
  }).filter((c) => {
    if (!c.fast_ma_type || !c.slow_ma_type || c.fast_len <= 0 || c.slow_len <= 0) return false;
    if (isDoubleLikeKind(kind)) return true;
    return !!c.halftrend;
  });

  if (!candidates.length) throw new Error("No valid candidates found (try lowering filters or increasing Candidates).");

  const walk_forward = $("valWfEnable")?.checked
    ? {
      window_bars: Math.max(64, Math.floor(num($("valWfWindow")?.value, 20000))),
      step_bars: Math.max(1, Math.floor(num($("valWfStep")?.value, 20000))),
    }
    : null;

  const perturb = $("valPerturbEnable")?.checked
    ? {
      iterations: Math.max(0, Math.floor(num($("valPerturbIters")?.value, 8))),
      pct_min: Math.max(0, num($("valPerturbMin")?.value, 0.015)),
      pct_max: Math.max(0, num($("valPerturbMax")?.value, 0.06)),
      flip_bool_prob: 0.02,
    }
    : null;

  const trade_bootstrap = $("valMcEnable")?.checked
    ? { simulations: Math.max(0, Math.floor(num($("valMcSims")?.value, 500))) }
    : null;

  const cscv_pbo = $("valCscvEnable")?.checked
    ? (() => {
      let slices = Math.max(4, Math.floor(num($("valCscvSlices")?.value, 16)));
      if ((slices & 1) === 1) slices = Math.max(4, slices - 1);
      return { slices, max_combinations: 50000, threshold: 0.0 };
    })()
    : null;

  const cross_ticker = $("valCrossEnable")?.checked
    ? (() => {
      const max_datasets = Math.max(0, Math.floor(num($("valCrossMax")?.value, 6)));
      if (max_datasets <= 0) return null;
      const datasets = getCrossSelectedDatasets()
        .map((d) => ({
          csv_path: String(d?.csv_path || "").trim(),
          label: String(d?.label || "").trim(),
        }))
        .filter((d) => !!d.csv_path)
        .slice(0, max_datasets);
      return datasets.length ? { max_datasets, datasets } : null;
    })()
    : null;

  return {
    data_id: dataId,
    ma_source: String(CURRENT_RUN.req.ma_source || "close"),
    fast_ma_params: CURRENT_RUN.req.fast_ma_params || null,
    slow_ma_params: CURRENT_RUN.req.slow_ma_params || null,
    candidates,
    walk_forward,
    perturb,
    trade_bootstrap,
    cross_ticker,
    cscv_pbo,
    seed,
    max_candidates: maxCandidates,
    min_psr,
    min_dsr,
  };
}

function clearValidationUi() {
  CURRENT_VALIDATION = null;
  const meta = $("validationMeta");
  if (meta) meta.textContent = "-";
  const meta2 = $("validationShortlistMeta");
  if (meta2) meta2.textContent = "-";
  const out = $("validationOutput");
  if (out) out.textContent = "-";
  const tbody = $("validationTableBody");
  if (tbody) tbody.innerHTML = "";
  const wrap = $("validationTableWrap");
  if (wrap) wrap.style.display = "none";
  updateValidationExportButtons();
}

function validationCandidates(res) {
  const xs = Array.isArray(res?.candidates) ? res.candidates : [];
  return xs.filter((c) => c && typeof c === "object");
}

function computeValidationShortlist(res) {
  const CSCV_RANK_FRAC_MIN = 0.55;
  const candidates = validationCandidates(res);
  const minPsr = parseValidationProbabilityGate("valMinPsr");
  const minDsr = parseValidationProbabilityGate("valMinDsr");
  const hasProbGate = minPsr != null || minDsr != null;
  const passProbabilityGates = (c) => {
    if (!hasProbGate) return true;
    const psr = Number(c?.psr);
    const dsr = Number(c?.dsr);
    if (minPsr != null && (!Number.isFinite(psr) || psr < minPsr)) return false;
    if (minDsr != null && (!Number.isFinite(dsr) || dsr < minDsr)) return false;
    return true;
  };
  const hasCross = candidates.some((c) => c.cross_ticker && typeof c.cross_ticker === "object");
  const strict = candidates.filter((c) => {
    if (!c.robust_pareto) return false;
    if (hasCross) return !!c.cross_ticker_pareto;
    return true;
  });

  const cscv = res?.cscv_pbo && typeof res.cscv_pbo === "object" ? res.cscv_pbo : null;
  const cscvCombos = cscv ? Number(cscv.combinations || 0) : 0;
  const cscvThreshold = cscv ? Number(cscv.threshold || 0) : 0;
  const hasCscv = cscvCombos > 0 && candidates.some((c) => c.cscv_pbo && typeof c.cscv_pbo === "object");

  const baseRows = strict.length ? strict : candidates;
  const usedFallback = !strict.length;

  const passCscv = (c) => {
    const st = c?.cscv_pbo && typeof c.cscv_pbo === "object" ? c.cscv_pbo : null;
    if (!st) return false;
    const sel = Number(st.is_selected_frac || 0);
    const rk = Number(st.oos_rank_frac_p50 || 0);
    const m = Number(st.oos_metric_p50 || NaN);
    if (!Number.isFinite(sel) || sel <= 0) return false;
    if (!Number.isFinite(rk) || rk < CSCV_RANK_FRAC_MIN) return false;
    if (!Number.isFinite(m) || m < cscvThreshold) return false;
    return true;
  };

  const probFiltered = hasProbGate ? baseRows.filter(passProbabilityGates) : baseRows;
  const cscvFiltered = hasCscv ? probFiltered.filter(passCscv) : probFiltered;
  const usedCscvFallback = hasCscv && baseRows.length && !cscvFiltered.length;
  const pickedRows = hasProbGate ? cscvFiltered : (usedCscvFallback ? baseRows : cscvFiltered);

  const sortByFinal = (a, b) => {
    const ar = Number(a?.robust_score || 0);
    const br = Number(b?.robust_score || 0);
    if (br !== ar) return br - ar;

    const aSt = a?.cscv_pbo && typeof a.cscv_pbo === "object" ? a.cscv_pbo : null;
    const bSt = b?.cscv_pbo && typeof b.cscv_pbo === "object" ? b.cscv_pbo : null;
    const aRk = Number(aSt?.oos_rank_frac_p50 || 0);
    const bRk = Number(bSt?.oos_rank_frac_p50 || 0);
    if (bRk !== aRk) return bRk - aRk;

    const aSel = Number(aSt?.is_selected_frac || 0);
    const bSel = Number(bSt?.is_selected_frac || 0);
    if (bSel !== aSel) return bSel - aSel;

    const aM = Number(aSt?.oos_metric_p50 || 0);
    const bM = Number(bSt?.oos_metric_p50 || 0);
    if (bM !== aM) return bM - aM;

    return 0;
  };

  const rows = [...pickedRows].sort(sortByFinal);
  return {
    candidates,
    rows,
    strictBase: strict,
    hasCross,
    usedFallback,
    hasCscv,
    cscvCombos,
    cscvThreshold,
    cscvRankMin: CSCV_RANK_FRAC_MIN,
    usedCscvFallback,
    minPsr,
    minDsr,
    hasProbGate,
    probPassN: probFiltered.length,
  };
}

function renderValidationTable(res) {
  const wrap = $("validationTableWrap");
  const tbody = $("validationTableBody");
  const meta = $("validationShortlistMeta");
  if (!wrap || !tbody) return;

  const {
    candidates,
    rows,
    strictBase,
    hasCross,
    usedFallback,
    hasCscv,
    cscvCombos,
    cscvThreshold,
    cscvRankMin,
    usedCscvFallback,
    minPsr,
    minDsr,
    hasProbGate,
    probPassN,
  } = computeValidationShortlist(res);

  tbody.innerHTML = "";
  if (!rows.length) {
    wrap.style.display = "none";
    if (meta) meta.textContent = "No validation result yet.";
    return;
  }

  wrap.style.display = "block";

  const robustParetoN = candidates.filter((c) => !!c.robust_pareto).length;
  const crossParetoN = hasCross ? candidates.filter((c) => !!c.cross_ticker_pareto).length : 0;
  const baseRule = hasCross ? "robust_pareto && cross_ticker_pareto" : "robust_pareto";
  const probRule = hasProbGate
    ? `prob(${minPsr != null ? `psr>=${fmtFixed(minPsr, 2)}` : ""}${minPsr != null && minDsr != null ? "," : ""}${minDsr != null ? `dsr>=${fmtFixed(minDsr, 2)}` : ""})`
    : "";
  const cscvRule = hasCscv ? `cscv(oos_rank_p50>=${cscvRankMin}, oos_sharpe_p50>=${cscvThreshold})` : "";
  const rule = [baseRule, probRule, cscvRule].filter((s) => !!s).join(" && ");
  const baseFallbackNote = usedFallback ? " • (fallback: top robust score)" : "";
  const cscvFallbackNote = usedCscvFallback && !hasProbGate ? " • (cscv fallback)" : "";
  if (meta) {
    const crossNote = hasCross ? ` • cross_pareto=${crossParetoN}/${candidates.length}` : "";
    const probNote = hasProbGate ? ` • prob_pass=${probPassN}/${strictBase.length ? strictBase.length : candidates.length}` : "";
    const cscvNote = hasCscv ? ` • cscv=${cscvCombos} combos` : "";
    const baseN = strictBase.length ? strictBase.length : candidates.length;
    const cscvPassN = rows.length;
    const cscvPassNote = hasCscv ? ` • cscv_pass=${cscvPassN}/${baseN}` : "";
    meta.textContent = `shortlist=${rows.length}/${candidates.length} • robust_pareto=${robustParetoN}/${candidates.length}${crossNote}${probNote}${cscvNote}${cscvPassNote} • rule=${rule}${baseFallbackNote}${cscvFallbackNote}`;
  }

  for (let i = 0; i < rows.length; i++) {
    const c = rows[i] || {};
    const p = c.params || {};
    const m = c.in_sample || {};
    const cross = c.cross_ticker && typeof c.cross_ticker === "object" ? c.cross_ticker : null;

    const tr = document.createElement("tr");

    const tdRank = document.createElement("td");
    tdRank.textContent = String(i + 1);
    tdRank.className = "left mono";

    const tdFast = document.createElement("td");
    tdFast.textContent = `${p.fast_ma_type || "?"}(${p.fast_len || "?"})`;
    tdFast.className = "left mono";

    const tdSlow = document.createElement("td");
    tdSlow.textContent = `${p.slow_ma_type || "?"}(${p.slow_len || "?"})`;
    tdSlow.className = "left mono";

    const tdRobust = document.createElement("td");
    tdRobust.textContent = fmt(c.robust_score, 6);
    tdRobust.className = "mono";

    const tdPsr = document.createElement("td");
    tdPsr.textContent = c.psr != null ? fmt(c.psr, 4) : "-";
    tdPsr.className = "mono";

    const tdDsr = document.createElement("td");
    tdDsr.textContent = c.dsr != null ? fmt(c.dsr, 4) : "-";
    tdDsr.className = "mono";

    const tdCscv = document.createElement("td");
    const cscv = c.cscv_pbo && typeof c.cscv_pbo === "object" ? c.cscv_pbo : null;
    const cscvRank = cscv ? Number(cscv.oos_rank_frac_p50 || 0) : NaN;
    tdCscv.textContent = cscv ? fmtPct(cscvRank, 1) : "-";
    tdCscv.className = "mono";

    const tdRobustPareto = document.createElement("td");
    tdRobustPareto.className = "left";
    const b1 = document.createElement("span");
    b1.className = "badge " + (c.robust_pareto ? "on" : "off");
    b1.textContent = c.robust_pareto ? "robust" : "—";
    tdRobustPareto.appendChild(b1);

    const tdCross = document.createElement("td");
    tdCross.textContent = cross ? fmt(cross.score, 6) : "-";
    tdCross.className = "mono";

    const tdCrossPareto = document.createElement("td");
    tdCrossPareto.className = "left";
    const b2 = document.createElement("span");
    const crossPareto = !!c.cross_ticker_pareto;
    b2.className = "badge " + (hasCross ? (crossPareto ? "on" : "off") : "off");
    b2.textContent = hasCross ? (crossPareto ? "cross" : "—") : "—";
    tdCrossPareto.appendChild(b2);

    const tdPnl = document.createElement("td");
    tdPnl.textContent = fmt(m.pnl, 6);
    tdPnl.className = "mono";

    const tdSharpe = document.createElement("td");
    tdSharpe.textContent = fmt(m.sharpe, 6);
    tdSharpe.className = "mono";

    const tdDd = document.createElement("td");
    tdDd.textContent = fmt(m.max_dd, 6);
    tdDd.className = "mono";

    const tdTrades = document.createElement("td");
    tdTrades.textContent = String(Number(m.trades || 0));
    tdTrades.className = "mono";

    tr.appendChild(tdRank);
    tr.appendChild(tdFast);
    tr.appendChild(tdSlow);
    tr.appendChild(tdRobust);
    tr.appendChild(tdPsr);
    tr.appendChild(tdDsr);
    tr.appendChild(tdCscv);
    tr.appendChild(tdRobustPareto);
    tr.appendChild(tdCross);
    tr.appendChild(tdCrossPareto);
    tr.appendChild(tdPnl);
    tr.appendChild(tdSharpe);
    tr.appendChild(tdDd);
    tr.appendChild(tdTrades);

    tbody.appendChild(tr);
  }
}

function csvEscape(v) {
  const s = String(v ?? "");
  if (/[",\n\r]/.test(s)) return `"${s.replaceAll("\"", "\"\"")}"`;
  return s;
}

function exportValidationShortlistCsv() {
  if (!CURRENT_VALIDATION?.res) {
    setStatus("No validation result to export yet.", "danger");
    return;
  }
  const { rows } = computeValidationShortlist(CURRENT_VALIDATION.res);
  if (!rows.length) {
    setStatus("No validated rows to export.", "muted");
    return;
  }

  const invoke = tauriInvoke();
  if (invoke) {
    const ts = CURRENT_RUN?.ts || new Date().toISOString();
    invoke("build_validation_shortlist_csv", { validation_res: CURRENT_VALIDATION.res })
      .then((csv) => {
        downloadText(`vectorbt_validation_shortlist_${ts}.csv`, String(csv || ""), "text/csv");
        setStatus("Exported validation shortlist CSV.", "ok");
      })
      .catch((e) => {
        setStatus(`Failed to export validation shortlist CSV: ${String(e)}`, "danger");
      });
    return;
  }

  const lines = [
    "rank,fast_ma,fast_len,slow_ma,slow_len,robust_score,robust_pareto,cross_score,cross_pareto,cscv_selected_frac,cscv_oos_rank_frac_p50,cscv_oos_sharpe_p50,psr,dsr,in_sample_pnl,in_sample_sharpe,in_sample_max_dd,in_sample_trades,params_json",
  ];
  for (let i = 0; i < rows.length; i++) {
    const c = rows[i] || {};
    const p = c.params || {};
    const m = c.in_sample || {};
    const cross = c.cross_ticker && typeof c.cross_ticker === "object" ? c.cross_ticker : null;
    const cscv = c.cscv_pbo && typeof c.cscv_pbo === "object" ? c.cscv_pbo : null;
    const paramsJson = (() => {
      try { return JSON.stringify(p); } catch { return "{}"; }
    })();
    lines.push([
      i + 1,
      csvEscape(p.fast_ma_type || ""),
      Number(p.fast_len || 0),
      csvEscape(p.slow_ma_type || ""),
      Number(p.slow_len || 0),
      Number(c.robust_score || 0),
      c.robust_pareto ? 1 : 0,
      cross ? Number(cross.score || 0) : "",
      c.cross_ticker_pareto ? 1 : 0,
      cscv ? Number(cscv.is_selected_frac || 0) : "",
      cscv ? Number(cscv.oos_rank_frac_p50 || 0) : "",
      cscv ? Number(cscv.oos_metric_p50 || 0) : "",
      c.psr != null ? Number(c.psr) : "",
      c.dsr != null ? Number(c.dsr) : "",
      Number(m.pnl || 0),
      Number(m.sharpe || 0),
      Number(m.max_dd || 0),
      Number(m.trades || 0),
      csvEscape(paramsJson),
    ].join(","));
  }

  const ts = CURRENT_RUN?.ts || new Date().toISOString();
  downloadText(`vectorbt_validation_shortlist_${ts}.csv`, lines.join("\n"), "text/csv");
}

function exportValidationJson() {
  if (!CURRENT_VALIDATION?.res) {
    setStatus("No validation result to export yet.", "danger");
    return;
  }
  const ts = CURRENT_RUN?.ts || new Date().toISOString();
  downloadText(`vectorbt_validation_${ts}.json`, pretty(CURRENT_VALIDATION.res), "application/json");
}

function exportReportJson() {
  if (!CURRENT_RUN?.res) {
    setStatus("No run result to export yet.", "danger");
    return;
  }
  const ts = CURRENT_RUN?.ts || new Date().toISOString();
  const payload = {
    version: 1,
    kind: "vectorbt_report",
    ts,
    data: {
      meta: (CURRENT_DATA_META && typeof CURRENT_DATA_META === "object") ? CURRENT_DATA_META : null,
      ctx: (CURRENT_DATASET_CTX && typeof CURRENT_DATASET_CTX === "object") ? CURRENT_DATASET_CTX : null,
    },
    run: { req: CURRENT_RUN.req || null, res: CURRENT_RUN.res || null },
    validation: CURRENT_VALIDATION ? { req: CURRENT_VALIDATION.req || null, res: CURRENT_VALIDATION.res || null } : null,
  };
  downloadText(`vectorbt_report_${ts}.json`, JSON.stringify(payload, null, 2), "application/json");
}

async function exportReportXlsx() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const path = String(LAST_META_RUN?.report_xlsx_path || "").trim();
  if (!path) {
    setStatus("Run Meta Analysis first. XLSX generation is consolidated in Stage 3.", "muted");
    return;
  }

  const wasRunning = RUNNING;
  setProgressUi(0, 1, "export_xlsx", "");
  setStatus("Opening Stage 3 XLSX report...", "muted");
  await invoke("open_path", { path });
  setProgressUi(1, 1, "export_xlsx", "");
  setStatus(`Opened Stage 3 XLSX report: ${path}`, "ok");
  updateQuickActionsUi();
  if (path && !!$("valAutoRevealReport")?.checked) {
    try {
      setProgressUi(0, 1, "reveal_report", "");
      await invoke("reveal_path", { path });
      setProgressUi(1, 1, "reveal_report", "");
    } catch (e) {
      setStatus(`Reveal failed: ${String(e)}`, "danger");
    }
  }
  if (!wasRunning) clearProgressUi();
}

async function runValidation(opts) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");

  const req = getValidationRequest(opts);
  const runKind = kindFromReq(CURRENT_RUN?.req);
  const cmd = isDoubleLikeKind(runKind) ? "run_double_ma_validation" : "run_composite_validation";
  setActiveProgressContext(buildValidationProgressContext(req, runKind));
  setRunning(true);
  try {
    setProgressUi(0, 1, isDoubleLikeKind(runKind) ? "double_ma_validate" : "composite_validate", "");
    setStatus("Running validation...", "muted");
    const t0 = performance.now();
    const res = await invoke(cmd, { req });
    const dt = performance.now() - t0;
    setStatus(`Validation done in ${dt.toFixed(0)}ms (frontend)`, "ok");
    $("output").textContent = pretty(res);
    if ($("validationOutput")) $("validationOutput").textContent = pretty(res);
    if ($("validationMeta")) {
      const crossN = Array.isArray(res?.cross_ticker?.datasets) ? res.cross_ticker.datasets.length : 0;
      const crossSuffix = crossN ? ` • cross=${crossN}` : "";
      const cscv = res?.cscv_pbo && typeof res.cscv_pbo === "object" ? res.cscv_pbo : null;
      const cscvCombos = cscv ? Number(cscv.combinations || 0) : 0;
      const cscvSkip = cscv ? String(cscv.skipped_reason || "").trim() : "";
      const cscvSuffix = cscv && !cscvSkip && cscvCombos > 0
        ? ` • pbo=${fmt(Number(cscv.pbo || 0), 3)} (cs=${cscvCombos})`
        : (cscvSkip ? " • cscv skipped" : "");
      $("validationMeta").textContent = `validated ${res?.candidates?.length || 0} candidates in ${Number(res?.runtime_ms || 0)}ms (backend)${crossSuffix}${cscvSuffix}`;
    }
    CURRENT_VALIDATION = { ts: CURRENT_RUN?.ts || new Date().toISOString(), req, res };
    renderValidationTable(res);
    if (CURRENT_RUN?.res && CURRENT_RUN?.req) renderBest(CURRENT_RUN.res, CURRENT_RUN.req);
    updateValidationExportButtons();
    // Persist validation into the run artifact so Meta selection can aggregate without requiring XLSX export.
    await persistCurrentRunArtifact();
    if (opts?.switchTab !== false) {
      RESULTS_TABS?.set("validate", { save: true, focus: false });
    }
    return res;
  } finally {
    setActiveProgressContext(null);
    setRunning(false);
  }
}

async function loadCsv() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const path = $("csvPath").value.trim();
  if (!path) throw new Error("Enter a CSV path.");
  setStatus("Loading data...", "muted");
  const resp = await invoke("load_price_data", { path });
  const id = String(resp?.id || "").trim();
  const meta = resp?.meta && typeof resp.meta === "object" ? resp.meta : null;
  const cacheHit = !!resp?.cache_hit;
  const cachePath = String(resp?.cache_path || "").trim();
  CURRENT_BASE_DATA_ID = id;
  CURRENT_BASE_DATA_META = meta;
  const baseTf = String(meta?.timeframe || "").trim() || null;
  CURRENT_BASE_DATASET_CTX = {
    source: "csv",
    label: fileBasename(path),
    timeframe: baseTf,
    base_timeframe: baseTf,
    derived_timeframe: baseTf,
    resample_applied: false,
    resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
    resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
  };
  CURRENT_DATA_META = meta;
  CURRENT_DATASET_CTX = { ...CURRENT_BASE_DATASET_CTX };
  clearDrillCaches({ clearBars: true });
  $("dataId").textContent = id;
  renderDataMeta(meta);
  applyMaSourceConstraints();
  applyBackendMaConstraints();
  setStatus(cacheHit ? "Loaded (binary cache)." : (cachePath ? "Loaded (CSV parsed + cached)." : "Loaded."), "ok");
  $("output").textContent = pretty(resp);
  addRecentCsvPath(path);
  saveSettingsSoon();
  updateOverviewEmptyUi();
  updateRunPlanUi();
  if (EXEC_LAST_SNAPSHOT) {
    try {
      await renderExecSnapshotChart(EXEC_LAST_SNAPSHOT);
    } catch {}
  } else {
    clearExecTvChart("Load a dataset and refresh orders to overlay broker events.");
  }
}

async function runOptimization(opts) {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  const req = getRequest();
  setActiveProgressContext(buildOptimizationProgressContext(req));
  setRunning(true);
  try {
    setStatus("Running optimization...", "muted");
    startOptimizationProgressHeartbeat(req);
    const t0 = performance.now();
    const kindRaw = String(req.strategy_kind || req.strategyKind || "DoubleMa");
    const kind = normalizeStrategyKindRaw(kindRaw);
    const cmd = kind === "CompositeCpu"
      ? "run_composite_cpu_optimization"
      : (kind === "CompositeMo" ? "run_composite_mo_optimization" : "run_double_ma_optimization");
    const res = await invoke(cmd, { req });
    const dt = performance.now() - t0;
    setStatus(`Done in ${dt.toFixed(0)}ms (frontend)`, "ok");
    $("output").textContent = pretty(res);
    addHistoryEntry(req, res);
    await persistCurrentRunArtifact();
    renderRun(res, req);
    refreshGpuStatusUi();
    if (opts?.switchTab !== false) {
      RESULTS_TABS?.set("top", { save: false, focus: false });
    }
    return res;
  } finally {
    stopOptimizationProgressHeartbeat();
    setActiveProgressContext(null);
    setRunning(false);
  }
}

async function runWorkflow(opts) {
  const res = await runOptimization(opts);
  const autoVal = !!$("valAutoRun")?.checked;
  if (autoVal) {
    try {
      await runValidation({ useFiltered: false, switchTab: false });
      if (opts?.switchTab !== false) {
        RESULTS_TABS?.set("overview", { save: true, focus: false });
      }
    } catch (e) {
      if (isCancelledError(e)) setStatus("Auto-validation cancelled.", "muted");
      else setStatus(`Auto-validation failed: ${String(e)}`, "danger");
    }
  }
  return res;
}

function defaultBatchTag() {
  const d = new Date();
  const pad = (n) => String(Math.max(0, Math.floor(Number(n || 0)))).padStart(2, "0");
  const y = d.getUTCFullYear();
  const m = pad(d.getUTCMonth() + 1);
  const day = pad(d.getUTCDate());
  const h = pad(d.getUTCHours());
  const min = pad(d.getUTCMinutes());
  const s = pad(d.getUTCSeconds());
  return `batch-${y}${m}${day}-${h}${min}${s}`;
}

function dedupTags(tags) {
  const xs = Array.isArray(tags) ? tags : [];
  const seen = new Set();
  const out = [];
  for (const t of xs) {
    const s = String(t || "").trim();
    if (!s) continue;
    const k = s.toLowerCase();
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(s);
  }
  return out;
}

function hashFNV1a32(text) {
  let h = 2166136261;
  const s = String(text || "");
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

function canonicalizeJsonValue(v) {
  if (v == null) return null;
  const t = typeof v;
  if (t === "number") return Number.isFinite(v) ? v : null;
  if (t === "string" || t === "boolean") return v;
  if (Array.isArray(v)) return v.map(canonicalizeJsonValue);
  if (t !== "object") return null;
  const out = {};
  const keys = Object.keys(v).sort();
  for (const k of keys) {
    const vv = v[k];
    if (vv === undefined) continue;
    out[k] = canonicalizeJsonValue(vv);
  }
  return out;
}

function canonicalJsonString(v) {
  return JSON.stringify(canonicalizeJsonValue(v));
}

function hashFNV1a64Hex(text) {
  const s = String(text || "");
  if (typeof BigInt !== "function") {
    const h32 = hashFNV1a32(s);
    return h32.toString(16).padStart(8, "0");
  }
  let h = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (let i = 0; i < s.length; i++) {
    h ^= BigInt(s.charCodeAt(i));
    h = (h * prime) & 0xffffffffffffffffn;
  }
  return h.toString(16).padStart(16, "0");
}

function stripRunReqForFingerprint(runReq) {
  const req = runReq && typeof runReq === "object" ? runReq : null;
  if (!req) return null;
  const out = canonicalizeJsonValue(req);
  if (!out || typeof out !== "object") return null;
  delete out.data_id;
  delete out.dataId;
  return out;
}

function stripValidationReqForFingerprint(valReq) {
  const req = valReq && typeof valReq === "object" ? valReq : null;
  if (!req) return null;
  const out = canonicalizeJsonValue(req);
  if (!out || typeof out !== "object") return null;
  delete out.data_id;
  delete out.dataId;
  delete out.candidates;
  const cross = out.cross_ticker && typeof out.cross_ticker === "object" ? out.cross_ticker : null;
  if (cross && Array.isArray(cross.datasets)) {
    const datasets = cross.datasets
      .map((d) => ({ csv_path: String(d?.csv_path || "").trim() }))
      .filter((d) => !!d.csv_path)
      .sort((a, b) => a.csv_path.localeCompare(b.csv_path));
    cross.datasets = datasets;
  }
  return out;
}

function computeConfigFingerprint(runReq, valReq) {
  const payload = {
    version: 1,
    run_req: stripRunReqForFingerprint(runReq),
    validation_req: stripValidationReqForFingerprint(valReq),
  };
  const canon = canonicalJsonString(payload);
  const hex = hashFNV1a64Hex(canon);
  return `cfg1-fnv1a64-${hex}`;
}

function getCompositeRequestForFingerprint() {
  const fast = [
    Number($("fastStart")?.value),
    Number($("fastEnd")?.value),
    Number($("fastStep")?.value),
  ];
  const slow = [
    Number($("slowStart")?.value),
    Number($("slowEnd")?.value),
    Number($("slowStep")?.value),
  ];

  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  if (!pairs) throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = String($("fastMaType")?.value || "").trim();
  const slowMaType = String($("slowMaType")?.value || "").trim();
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");
  if (!maSupportsCpu(fastMaType) || !maSupportsCpu(slowMaType)) {
    throw new Error("Composite strategy is CPU-only and does not support one of the selected MA types.");
  }

  const maSource = String($("maSource")?.value || "close");

  const amp = Math.floor(num($("halfTrendAmplitude")?.value, 2));
  const atrP = Math.floor(num($("halfTrendAtrPeriod")?.value, 100));
  const dev = num($("halfTrendChannelDev")?.value, 2.0);
  if (!Number.isFinite(amp) || amp < 1 || amp > 65535) throw new Error("HalfTrend amplitude must be in [1, 65535].");
  if (!Number.isFinite(atrP) || atrP < 1 || atrP > 65535) throw new Error("HalfTrend ATR period must be in [1, 65535].");
  if (!Number.isFinite(dev) || dev < 0) throw new Error("HalfTrend channel deviation must be finite and >= 0.");
  const halftrend = { amplitude: amp, atr_period: atrP, channel_deviation: dev };

  const maczFast = Math.floor(num($("maczFastLength")?.value, 12));
  const maczSlow = Math.floor(num($("maczSlowLength")?.value, 25));
  const maczSig = Math.floor(num($("maczSignalLength")?.value, 9));
  const maczLz = Math.floor(num($("maczLengthZ")?.value, 20));
  const maczLsd = Math.floor(num($("maczLengthStdev")?.value, 25));
  const maczGamma = num($("maczGamma")?.value, 0.02);
  const maczA = num($("maczA")?.value, 1.0);
  const maczB = num($("maczB")?.value, 1.0);
  const maczUseVolume = !!$("maczUseVolume")?.checked;
  const maczUseLag = !!$("maczUseLag")?.checked;

  if (!Number.isFinite(maczFast) || maczFast < 1) throw new Error("MACZ fast_length must be >= 1.");
  if (!Number.isFinite(maczSlow) || maczSlow < 1) throw new Error("MACZ slow_length must be >= 1.");
  if (maczFast >= maczSlow) throw new Error("MACZ requires fast_length < slow_length.");
  if (!Number.isFinite(maczSig) || maczSig < 1) throw new Error("MACZ signal_length must be >= 1.");
  if (!Number.isFinite(maczLz) || maczLz < 1) throw new Error("MACZ lengthz must be >= 1.");
  if (!Number.isFinite(maczLsd) || maczLsd < 1) throw new Error("MACZ length_stdev must be >= 1.");
  if (!Number.isFinite(maczA) || maczA < -2 || maczA > 2) throw new Error("MACZ a must be in [-2, 2].");
  if (!Number.isFinite(maczB) || maczB < -2 || maczB > 2) throw new Error("MACZ b must be in [-2, 2].");
  if (!Number.isFinite(maczGamma) || maczGamma < 0 || maczGamma >= 1) throw new Error("MACZ gamma must be in [0, 1).");

  const macz = {
    fast_length: maczFast,
    slow_length: maczSlow,
    signal_length: maczSig,
    lengthz: maczLz,
    length_stdev: maczLsd,
    a: maczA,
    b: maczB,
    use_lag: maczUseLag,
    gamma: maczGamma,
    use_volume: maczUseVolume,
  };

  const ottPeriod = Math.floor(num($("ottPeriod")?.value, 2));
  const ottPercent = num($("ottPercent")?.value, 1.4);
  const ottMaType = String($("ottMaType")?.value || "VAR").trim();
  if (!Number.isFinite(ottPeriod) || ottPeriod < 1) throw new Error("OTT period must be >= 1.");
  if (!Number.isFinite(ottPercent) || ottPercent < 0) throw new Error("OTT percent must be finite and >= 0.");
  if (!ottMaType) throw new Error("OTT MA type is required.");
  const ott = { period: ottPeriod, percent: ottPercent, ma_type: ottMaType };

  const objective = String($("objective")?.value || "Sharpe");
  const topK = Number($("topK")?.value || 0);
  const includeAll = $("includeAll")?.value === "true";
  if (includeAll && pairs > 2_000_000) {
    throw new Error(`includeAll=true would return ${pairs} rows; disable it or reduce ranges.`);
  }

  const heatmapBins = Math.max(0, Math.floor(num($("heatmapBins")?.value, 0)));
  if (heatmapBins > 512) throw new Error("heatmapBins must be <= 512.");

  const strategy = readStrategyCfgFromUi();
  const signal_mode = readCompositeSignalMode();

  return {
    strategy_kind: "CompositeCpu",
    backend: { backend: "CpuOnly" },
    data_id: "__cfg__",
    starting_equity: readStartingEquityFromUi(),
    fast_range: fast,
    slow_range: slow,
    fast_ma_types: [fastMaType],
    slow_ma_types: [slowMaType],
    ma_source: maSource,
    fast_ma_params: resolvedMaParams("fast", fastMaType),
    slow_ma_params: resolvedMaParams("slow", slowMaType),
    strategy,
    signal_mode,
    halftrend,
    macz,
    ott,
    objective,
    mode: "Grid",
    top_k: topK,
    include_all: includeAll,
    heatmap_bins: heatmapBins,
  };
}

function getCompositeMoRequestForFingerprint() {
  const fast = [
    Number($("fastStart")?.value),
    Number($("fastEnd")?.value),
    Number($("fastStep")?.value),
  ];
  const slow = [
    Number($("slowStart")?.value),
    Number($("slowEnd")?.value),
    Number($("slowStep")?.value),
  ];

  const fastPeriods = expandRangeU16Like(fast[0], fast[1], fast[2]);
  const slowPeriods = expandRangeU16Like(slow[0], slow[1], slow[2]);
  const pairs = countPairsFastLtSlow(fastPeriods, slowPeriods);
  if (!pairs) throw new Error("No valid parameter combinations (fast must be < slow).");

  const fastMaType = String($("fastMaType")?.value || "").trim();
  const slowMaType = String($("slowMaType")?.value || "").trim();
  if (!fastMaType) throw new Error("Select a fast MA type.");
  if (!slowMaType) throw new Error("Select a slow MA type.");

  if (!maSupportsCpu(fastMaType) || !maSupportsCpu(slowMaType)) {
    throw new Error("Composite strategy is CPU-only and does not support one of the selected MA types.");
  }

  const maSource = String($("maSource")?.value || "close");

  const baseStrategy = readStrategyCfgFromUi();
  const signal_mode = readCompositeSignalMode();

  const maximize = String($("objective")?.value || "Sharpe");
  const minimize = String($("moObjectiveMin")?.value || "MaxDrawdown");

  const samplerKind = String($("moSampler")?.value || "Nsga2");
  let sampler = null;
  if (samplerKind === "Random") {
    const seed = Math.max(0, Math.floor(num($("moRandomSeed")?.value, 1)));
    const trials = Math.max(1, Math.floor(num($("moRandomTrials")?.value, 2000)));
    if (trials > 200_000) throw new Error("Random trials too large (max 200000).");
    sampler = { sampler: "Random", config: { seed, trials } };
  } else if (samplerKind === "Tpe") {
    const seed = Math.max(0, Math.floor(num($("moTpeSeed")?.value, 1)));
    const trials = Math.max(1, Math.floor(num($("moTpeTrials")?.value, 5000)));
    const startup_trials = Math.max(0, Math.floor(num($("moTpeStartupTrials")?.value, 10)));
    const n_ei_candidates = Math.max(1, Math.floor(num($("moTpeCandidates")?.value, 24)));
    const gamma = num($("moTpeGamma")?.value, 0.25);
    const batch_size = Math.max(1, Math.floor(num($("moTpeBatchSize")?.value, 64)));
    const max_model_points = Math.max(32, Math.floor(num($("moTpeMaxModelPoints")?.value, 2048)));
    sampler = { sampler: "Tpe", config: { seed, trials, startup_trials, n_ei_candidates, gamma, batch_size, max_model_points } };
  } else if (samplerKind === "Qmc") {
    const seed = Math.max(0, Math.floor(num($("moQmcSeed")?.value, 1)));
    const trials = Math.max(1, Math.floor(num($("moQmcTrials")?.value, 5000)));
    const sequence = String($("moQmcSequence")?.value || "sobol").trim().toLowerCase() || "sobol";
    sampler = { sampler: "Qmc", config: { seed, trials, sequence } };
  } else {
    const seed = Math.max(0, Math.floor(num($("moNsga2Seed")?.value, 1)));
    const population = Math.max(4, Math.floor(num($("moNsga2Population")?.value, 64)));
    const generations = Math.max(1, Math.floor(num($("moNsga2Generations")?.value, 40)));
    const crossover_prob = Math.max(0, Math.min(1, num($("moNsga2CrossoverProb")?.value, 0.9)));
    const swapping_prob = Math.max(0, Math.min(1, num($("moNsga2SwappingProb")?.value, 0.5)));
    const mutation_prob = Math.max(0, Math.min(1, num($("moNsga2MutationProb")?.value, 0.3)));
    sampler = { sampler: "Nsga2", config: { seed, population, generations, crossover_prob, swapping_prob, mutation_prob } };
  }

  const minTrades = optNum($("moConstraintMinTrades")?.value);
  const maxDd = optNum($("moConstraintMaxDd")?.value);
  const maxAbsExpo = optNum($("moConstraintMaxAbsExposure")?.value);
  let constraints = null;
  if (Number.isFinite(minTrades) || Number.isFinite(maxDd) || Number.isFinite(maxAbsExpo)) {
    constraints = {};
    if (Number.isFinite(minTrades)) {
      if (minTrades < 0) throw new Error("Min trades must be >= 0.");
      constraints.min_trades = Math.floor(minTrades);
    }
    if (Number.isFinite(maxDd)) {
      if (maxDd < 0) throw new Error("Max drawdown constraint must be >= 0.");
      constraints.max_drawdown = maxDd;
    }
    if (Number.isFinite(maxAbsExpo)) {
      if (maxAbsExpo < 0) throw new Error("Max abs exposure constraint must be >= 0.");
      constraints.max_abs_exposure = maxAbsExpo;
    }
  }

  let gpu_prescan = null;
  const prescanOn = !!$("moPrescanEnable")?.checked;
  if (prescanOn) {
    const device_id = Math.max(0, Math.floor(num($("deviceId")?.value, 0)));
    const top_k = Math.max(1, Math.floor(num($("moPrescanTopK")?.value, 128)));
    const window = Math.max(0, Math.floor(num($("moPrescanWindow")?.value, 8)));
    const predictive_top_k = Math.max(0, Math.floor(num($("moPrescanPredictiveTopK")?.value, 12)));
    const predictive_stride = Math.max(1, Math.min(65535, Math.floor(num($("moPrescanPredictiveStride")?.value, 8))));
    const objectives = String($("moPrescanObjectives")?.value || "both").trim();
    const mode = String($("moPrescanMode")?.value || "Auto").trim();
    const ma_pairs = parseCompositeMoPrescanMaPairs($("moPrescanMaPairs")?.value || "");
    gpu_prescan = { top_k, window, predictive_top_k, predictive_stride, device_id, require_gpu: !!$("moPrescanRequireGpu")?.checked, objectives, ma_pairs, mode };
  }

  const useRangeBuilder = !!$("moUseRangeBuilder")?.checked;
  const space = useRangeBuilder ? buildCompositeMoSearchSpaceFromUi() : parseCompositeMoSearchSpace();
  const rawFastStep = Math.floor(num($("fastStep")?.value, 1));
  const rawSlowStep = Math.floor(num($("slowStep")?.value, 1));
  const fastStep = Math.max(1, rawFastStep);
  const slowStep = Math.max(1, rawSlowStep);
  const fastLow = Math.floor(fast[0]);
  const slowLow = Math.floor(slow[0]);
  const fastHigh = rawFastStep === 0 ? fastLow : Math.floor(fast[1]);
  const slowHigh = rawSlowStep === 0 ? slowLow : Math.floor(slow[1]);
  const fastDomain = { IntRange: { low: fastLow, high: fastHigh, step: fastStep, log: false } };
  const slowDomain = { IntRange: { low: slowLow, high: slowHigh, step: slowStep, log: false } };
  const search_space = normalizeCompositeMoSearchSpace(space, fastDomain, slowDomain);

  return {
    strategy_kind: "CompositeMo",
    data_id: "__cfg__",
    starting_equity: readStartingEquityFromUi(),
    fast_ma_type: fastMaType,
    slow_ma_type: slowMaType,
    ma_source: maSource,
    fast_ma_params: resolvedMaParams("fast", fastMaType),
    slow_ma_params: resolvedMaParams("slow", slowMaType),
    base_strategy: baseStrategy,
    signal_mode,
    strategy: baseStrategy,
    search_space,
    objective: maximize,
    objectives: { maximize, minimize },
    sampler,
    constraints,
    gpu_prescan,
    mode: null,
  };
}

function getValidationReqForFingerprintFromUi() {
  const kind = strategyKind();
  if (kind !== "CompositeCpu" && kind !== "CompositeMo") return null;

  const max_candidates = Math.max(1, Math.floor(num($("valCandidates")?.value, 32)));
  const seed = Math.max(0, Math.floor(num($("valSeed")?.value, 1)));

  const walk_forward = $("valWfEnable")?.checked
    ? {
      window_bars: Math.max(64, Math.floor(num($("valWfWindow")?.value, 20000))),
      step_bars: Math.max(1, Math.floor(num($("valWfStep")?.value, 20000))),
    }
    : null;

  const perturb = $("valPerturbEnable")?.checked
    ? {
      iterations: Math.max(0, Math.floor(num($("valPerturbIters")?.value, 8))),
      pct_min: Math.max(0, num($("valPerturbMin")?.value, 0.015)),
      pct_max: Math.max(0, num($("valPerturbMax")?.value, 0.06)),
      flip_bool_prob: 0.02,
    }
    : null;

  const trade_bootstrap = $("valMcEnable")?.checked
    ? { simulations: Math.max(0, Math.floor(num($("valMcSims")?.value, 500))) }
    : null;

  const cscv_pbo = $("valCscvEnable")?.checked
    ? (() => {
      let slices = Math.max(4, Math.floor(num($("valCscvSlices")?.value, 16)));
      if ((slices & 1) === 1) slices = Math.max(4, slices - 1);
      return { slices, max_combinations: 50000, threshold: 0.0 };
    })()
    : null;

  const cross_ticker = $("valCrossEnable")?.checked
    ? (() => {
      const max_datasets = Math.max(0, Math.floor(num($("valCrossMax")?.value, 6)));
      if (max_datasets <= 0) return null;
      const datasets = getCrossSelectedPaths()
        .map((csv_path) => ({ csv_path, label: "" }))
        .slice(0, max_datasets);
      return datasets.length ? { max_datasets, datasets } : null;
    })()
    : null;

  const fastMaType = String($("fastMaType")?.value || "").trim();
  const slowMaType = String($("slowMaType")?.value || "").trim();
  const maSource = String($("maSource")?.value || "close");

  return {
    data_id: "__cfg__",
    ma_source: maSource,
    fast_ma_params: resolvedMaParams("fast", fastMaType) || null,
    slow_ma_params: resolvedMaParams("slow", slowMaType) || null,
    candidates: [],
    walk_forward,
    perturb,
    trade_bootstrap,
    cross_ticker,
    cscv_pbo,
    seed,
    max_candidates,
  };
}

function getRequestForFingerprint() {
  const kind = strategyKind();
  if (kind === "CompositeCpu") return getCompositeRequestForFingerprint();
  if (kind === "CompositeMo") return getCompositeMoRequestForFingerprint();
  return null;
}

function splitHoldoutPaths(paths, holdoutPct, seed) {
  const xs = Array.isArray(paths) ? paths.map((p) => String(p || "").trim()).filter((p) => !!p) : [];
  const pct = Number.isFinite(Number(holdoutPct)) ? Math.max(0, Math.min(50, Number(holdoutPct))) : 0;
  if (!xs.length || pct <= 0) return { train: xs, holdout: [] };

  const n = xs.length;
  const k = Math.max(0, Math.min(n, Math.floor(n * (pct / 100))));
  if (k <= 0) return { train: xs, holdout: [] };
  if (k >= n) return { train: [], holdout: xs };

  const seed32 = hashFNV1a32(String(seed ?? 0));
  const scored = xs.map((p) => ({ p, s: (hashFNV1a32(p) ^ seed32) >>> 0 }));
  scored.sort((a, b) => a.s - b.s);

  const holdout = scored.slice(0, k).map((x) => x.p);
  const train = scored.slice(k).map((x) => x.p);
  return { train, holdout };
}

async function runBatchWorkflow() {
  const invoke = tauriInvoke();
  if (!invoke) throw new Error("Tauri invoke not available (window.__TAURI__ missing).");
  if (RUNNING) return;

  if (!isCompositeStrategy()) {
    setStatus("Batch runs currently support Composite strategies only (Composite [CPU] or Composite [CPU + MO]).", "danger");
    return;
  }

  const selectedPaths = getBatchSelectedPaths();
  if (!selectedPaths.length) {
    setStatus("Select at least 1 dataset in Meta → Batch runs first.", "danger");
    return;
  }

  const holdoutPctRaw = Math.floor(Number($("batchHoldoutPct")?.value || 0));
  const holdout_pct = Number.isFinite(holdoutPctRaw) ? Math.max(0, Math.min(50, holdoutPctRaw)) : 0;
  const holdoutTagRaw = String($("batchHoldoutTag")?.value || "").trim();
  const holdout_tag = holdoutTagRaw || "holdout";
  const auto_meta = !!$("batchAutoMeta")?.checked;
  const skip_done = !!$("batchSkipDone")?.checked;

  const prevAutoVal = !!$("valAutoRun")?.checked;
  const prevAutoXlsx = !!$("valAutoExportXlsx")?.checked;
  const prevAutoReveal = !!$("valAutoRevealReport")?.checked;

  setRunning(true);
  BATCH_RUNNING = true;
  BATCH_STOP_REQUESTED = false;
  if ($("btnBatchStop")) $("btnBatchStop").disabled = false;
  try {
    setBatchSummaryUi(null, "");
    // Hands-off defaults for batch: validate, but don't pop open files for every dataset.
    if ($("valAutoRun")) $("valAutoRun").checked = true;
    if ($("valAutoRevealReport")) $("valAutoRevealReport").checked = false;
    const requireXlsx = false;

    let baseTags = parseTagsInput($("batchTags")?.value || "");
    if (!baseTags.length) {
      const t = defaultBatchTag();
      baseTags = [t];
      if ($("batchTags")) $("batchTags").value = t;
      if ($("metaRequiredTag") && !String($("metaRequiredTag")?.value || "").trim()) $("metaRequiredTag").value = t;
    }
    saveSettingsSoon();

    if (!Array.isArray(BATCH_DATASETS_CACHE) || !BATCH_DATASETS_CACHE.length) {
      const merged = new Map();
      try {
        const info = await invoke("polygon_get_cache_info");
        for (const ds of Array.isArray(info?.datasets) ? info.datasets : []) {
          const p = String(ds?.csv_path || "").trim();
          if (!p) continue;
          merged.set(p, ds);
        }
      } catch {}
      try {
        const resp = await invoke("list_repo_data_csvs", { maxFiles: 20000, recursive: true, includeIdentity: true });
        const localPaths = Array.isArray(resp?.csv_paths) ? resp.csv_paths : [];
        const localEntries = Array.isArray(resp?.csv_entries) ? resp.csv_entries : [];
        const locals = localEntries.length ? localEntries : localPaths.map((p) => ({ csv_path: p }));
        for (const e of locals) {
          const p = String(e?.csv_path || "").trim();
          if (!p || merged.has(p)) continue;
          const size = Number(e?.source_size_bytes ?? NaN);
          const mtime = Number(e?.source_mtime_utc_ms ?? NaN);
          const fp = String(e?.source_content_fingerprint || "").trim();
          const source_size_bytes = Number.isFinite(size) && size >= 0 ? size : undefined;
          const source_mtime_utc_ms = Number.isFinite(mtime) && mtime > 0 ? mtime : undefined;
          const source_content_fingerprint = fp || undefined;
          merged.set(p, {
            source: "repo_data",
            market: "local",
            display_symbol: fileBasename(p) || p,
            csv_path: p,
            timeframe: "",
            source_size_bytes,
            source_mtime_utc_ms,
            source_content_fingerprint,
          });
        }
      } catch {}
      BATCH_DATASETS_CACHE = [...merged.values()];
    }

    const byPath = new Map();
    for (const ds of Array.isArray(BATCH_DATASETS_CACHE) ? BATCH_DATASETS_CACHE : []) {
      const p = String(ds?.csv_path || "").trim();
      if (!p) continue;
      byPath.set(p, ds);
    }

    const tfOverride = String($("dataTimeframe")?.value || "").trim();
    const hasTfOverride = !!tfOverride;
    const selectedTimeframes = new Set();
    for (const p0 of selectedPaths) {
      const p = String(p0 || "").trim();
      if (!p) continue;
      const ds = byPath.get(p) || null;
      const tf = String(ds?.timeframe || "").trim();
      if (tf) selectedTimeframes.add(tf);
    }
    if (!hasTfOverride && selectedTimeframes.size > 1) {
      const tfs = [...selectedTimeframes].sort().join(", ");
      setStatus(`Batch requires a single timeframe across selected datasets (found: ${tfs}). Filter by 'tf=1m' (or similar), or set Data → Timeframe to force resampling.`, "danger");
      return;
    }

    let expectedTimeframe = hasTfOverride
      ? tfOverride
      : (selectedTimeframes.size === 1 ? [...selectedTimeframes][0] : "");

    const batchTs = new Date().toISOString();
    const batchStart = performance.now();
    const n = selectedPaths.length;
    const primary_tag = String(baseTags[0] || "").trim();
    const split = splitHoldoutPaths(selectedPaths, holdout_pct, primary_tag || "0");
    const holdoutSet = new Set(Array.isArray(split.holdout) ? split.holdout : []);
    const expectedKind = strategyKind() === "CompositeCpu" ? "CompositeCpu" : "CompositeMo";
    const expectedFingerprint = computeConfigFingerprint(getRequestForFingerprint(), getValidationReqForFingerprintFromUi());
    const batchTimeframes = new Set();
    if (expectedTimeframe) batchTimeframes.add(expectedTimeframe);
    const doneLoose = new Set(); // csv_path + timeframe
    const doneStrict = new Set(); // csv_path + timeframe + source identity (fp preferred, else size/mtime)
    const doneKeyLoose = (csv_path, tf) => `${csv_path}@@tf=${tf || ""}`;
    const doneKeyStrictFp = (csv_path, tf, fp) => `${csv_path}@@tf=${tf || ""}@@fp=${fp}`;
    const doneKeyStrictMeta = (csv_path, tf, size, mtime) => `${csv_path}@@tf=${tf || ""}@@id=${size}@${mtime}`;
    if (skip_done && primary_tag) {
      try {
        const runs = await invoke("list_run_artifacts");
        for (const r of Array.isArray(runs) ? runs : []) {
          const strat = String(r?.strategy_kind || "").trim();
          if (strat !== expectedKind) continue;
          const fp = String(r?.config_fingerprint || "").trim();
          if (!fp || fp !== expectedFingerprint) continue;
          const tags = Array.isArray(r?.tags) ? r.tags.map((t) => String(t || "").toLowerCase()) : [];
          if (!tags.includes(primary_tag.toLowerCase())) continue;
          const csv = String(r?.csv_path || "").trim();
          if (!csv) continue;
          const rTf = String(r?.timeframe || "").trim();
          if (expectedTimeframe) {
            if (!rTf) continue;
            if (rTf !== expectedTimeframe) continue;
          }
          const hasVal = !!r?.has_validation;
          const hasXlsx = !!String(r?.report_xlsx_path || "").trim();
          if (!hasVal || (requireXlsx && !hasXlsx)) continue;

          const tfKey = expectedTimeframe || rTf || "";
          doneLoose.add(doneKeyLoose(csv, tfKey));

          const runFp = String(r?.source_content_fingerprint || "").trim();
          if (runFp) {
            doneStrict.add(doneKeyStrictFp(csv, tfKey, runFp));
            continue;
          }
          const size = Number(r?.source_size_bytes ?? NaN);
          const mtime = Number(r?.source_mtime_utc_ms ?? NaN);
          const hasId = Number.isFinite(size) && size >= 0 && Number.isFinite(mtime) && mtime > 0;
          if (hasId) doneStrict.add(doneKeyStrictMeta(csv, tfKey, size, mtime));
        }
      } catch {}
    }
    let ok = 0;
    let skipped = 0;
    let cancelled = false;
    const failures = [];
    const okRows = [];
    const skippedRows = [];
    const failedRows = [];

    for (let i = 0; i < n; i++) {
      if (BATCH_STOP_REQUESTED) break;
      const path = String(selectedPaths[i] || "").trim();
      if (!path) continue;

      const isHoldout = holdoutSet.has(path);
      const ds = byPath.get(path) || null;
      const label = String(ds?.display_symbol || ds?.polygon_ticker || fileBasename(path) || path).trim();

      const hoHint = holdoutSet.size ? ` • holdout=${holdoutSet.size}/${n}` : "";
      updateBatchMeta(`running ${i + 1}/${n}: ${label || "dataset"}${isHoldout ? " [holdout]" : ""}${hoHint}`);

      const tfKey = expectedTimeframe || "";
      const looseKey = doneKeyLoose(path, tfKey);
      const curFp = String(ds?.source_content_fingerprint || "").trim();
      const hasCurFp = !!curFp;
      const curSize = Number(ds?.source_size_bytes ?? NaN);
      const curMtime = Number(ds?.source_mtime_utc_ms ?? NaN);
      const hasCurId = Number.isFinite(curSize) && curSize >= 0 && Number.isFinite(curMtime) && curMtime > 0;
      const strictKey = hasCurFp
        ? doneKeyStrictFp(path, tfKey, curFp)
        : (hasCurId ? doneKeyStrictMeta(path, tfKey, curSize, curMtime) : "");
      const isDone = strictKey ? doneStrict.has(strictKey) : doneLoose.has(looseKey);

      if (isDone) {
        skipped += 1;
        updateBatchMeta(`skipping ${label || "dataset"} (already completed for ${primary_tag})`);
        skippedRows.push({ csv_path: path, label, is_holdout: isHoldout });
        continue;
      }

      if ($("csvPath")) $("csvPath").value = path;
      try {
        await loadCsv();
      } catch (e) {
        failures.push({ label, error: `load: ${String(e)}` });
        failedRows.push({ csv_path: path, label, is_holdout: isHoldout, error: `load: ${String(e)}` });
        setStatus(`Batch: failed to load ${label}: ${String(e)}`, "danger");
        continue;
      }

      const dsSource = String(ds?.source || "").trim();
      const isRepoData = dsSource === "repo_data";
      if (ds && !isRepoData) {
        const market = String(ds?.market || "").trim() || null;
        const adjFlag = market === "stocks" ? (!!ds?.adjusted) : null;
        const sym = String(ds?.display_symbol || ds?.polygon_ticker || label).trim();
        const baseTf = String(CURRENT_BASE_DATA_META?.timeframe || ds?.timeframe || "").trim() || null;
        CURRENT_BASE_DATASET_CTX = {
          source: "polygon",
          label: sym || label || fileBasename(path) || path,
          symbol: sym || null,
          market,
          adjusted: adjFlag,
          timeframe: baseTf,
          base_timeframe: baseTf,
          derived_timeframe: baseTf,
          resample_applied: false,
          resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
          resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
        };
        CURRENT_DATASET_CTX = { ...CURRENT_BASE_DATASET_CTX };
      }

      if (hasTfOverride) {
        try {
          await applySelectedDataTimeframe({ allowWhileRunning: true });
        } catch (e) {
          failures.push({ label, error: `resample: ${String(e)}` });
          failedRows.push({ csv_path: path, label, is_holdout: isHoldout, error: `resample: ${String(e)}` });
          setStatus(`Batch: failed to resample ${label}: ${String(e)}`, "danger");
          continue;
        }
      }

      const effTf = String(CURRENT_DATASET_CTX?.timeframe || CURRENT_DATA_META?.timeframe || "").trim();
      if (!expectedTimeframe && effTf) {
        expectedTimeframe = effTf;
        batchTimeframes.add(effTf);
      }
      if (expectedTimeframe && effTf && effTf !== expectedTimeframe) {
        failures.push({ label, error: `timeframe mismatch: got ${effTf}, expected ${expectedTimeframe}` });
        failedRows.push({ csv_path: path, label, is_holdout: isHoldout, error: `timeframe mismatch: got ${effTf}, expected ${expectedTimeframe}` });
        const hint = hasTfOverride ? "" : " Set Data → Timeframe to force resampling.";
        setStatus(`Batch: timeframe mismatch on ${label}: got ${effTf}, expected ${expectedTimeframe}.${hint}`, "danger");
        continue;
      }
      if (effTf) batchTimeframes.add(effTf);

      const extraTags = isHoldout && holdout_pct > 0 ? [holdout_tag] : [];
      const perRunTags = dedupTags([...baseTags, label, ...extraTags].filter((t) => !!String(t || "").trim()));
      NEXT_RUN_TAGS = perRunTags;

      try {
        await runWorkflow({ switchTab: false });
        ok += 1;
        okRows.push({
          csv_path: path,
          label,
          is_holdout: isHoldout,
          ts: String(CURRENT_RUN?.ts || "").trim() || null,
          artifact_path: String(CURRENT_RUN?.artifact_path || "").trim() || null,
          report_xlsx_path: String(CURRENT_RUN?.report_xlsx_path || "").trim() || null,
        });
      } catch (e) {
        if (isCancelledError(e)) {
          cancelled = true;
          setStatus("Batch cancelled.", "muted");
          break;
        }
        NEXT_RUN_TAGS = null;
        failures.push({ label, error: `run: ${String(e)}` });
        failedRows.push({ csv_path: path, label, is_holdout: isHoldout, error: `run: ${String(e)}` });
        setStatus(`Batch: failed on ${label}: ${String(e)}`, "danger");
      }
    }

    await refreshSavedRuns();
    const failed = failures.length;
    const skipHint = skipped ? ` • skipped=${skipped}` : "";
    const msg = `Batch done: ok=${ok}/${n}${skipHint}${failed ? ` • failed=${failed}` : ""}`;
    setStatus(msg, failed ? "danger" : "ok");
    updateBatchMeta(failed ? `${msg} • first=${failures[0]?.label}` : msg);

    const summary = {
      version: 1,
      kind: "vectorbt_batch_summary",
      ts: batchTs,
      strategy_kind: expectedKind,
      timeframe: batchTimeframes.size === 1 ? [...batchTimeframes][0] : null,
      batch_tag: primary_tag || null,
      config_fingerprint: expectedFingerprint,
      tags: baseTags,
      holdout_pct,
      holdout_tag,
      selected_paths: selectedPaths,
      holdout_paths: [...holdoutSet],
      counts: { total: n, ok, skipped, failed },
      results: {
        ok: okRows,
        skipped: skippedRows,
        failed: failedRows,
        cancelled,
      },
      runtime_ms: Math.max(0, Math.round(performance.now() - batchStart)),
    };
    try {
      const resp = await invoke("save_batch_summary", { summary });
      const p = String(resp?.path || "").trim();
      setBatchSummaryUi(summary, p);
    } catch (e) {
      setBatchSummaryUi(summary, "");
      setStatus(`Saved batch runs, but failed to write summary JSON: ${String(e)}`, "danger");
    }

    if (auto_meta && !cancelled) {
      try {
        // Ensure meta defaults match this batch.
        const kind = strategyKind();
        if ($("metaStrategyKind")) {
          $("metaStrategyKind").value = kind === "CompositeCpu"
            ? "CompositeCpu"
            : (kind === "CompositeTrend"
              ? "CompositeTrend"
              : (kind === "RegimeHybrid"
                ? "RegimeHybrid"
                : (kind === "BreakoutMomentum" ? "BreakoutMomentum" : "CompositeMo")));
        }
        const tf = batchTimeframes.size === 1 ? String([...batchTimeframes][0] || "").trim() : "";
        if (tf && $("metaTimeframe") && !String($("metaTimeframe")?.value || "").trim()) $("metaTimeframe").value = tf;
        if ($("metaRequiredTag")) $("metaRequiredTag").value = primary_tag || String($("metaRequiredTag")?.value || "").trim();
        if ($("metaHoldoutTag")) $("metaHoldoutTag").value = holdout_tag;
        if ($("metaEvalHoldout")) $("metaEvalHoldout").checked = holdoutSet.size > 0;
        saveSettingsSoon();

        // Stop controls no longer apply; meta is a separate phase.
        BATCH_RUNNING = false;
        BATCH_STOP_REQUESTED = false;
        if ($("btnBatchStop")) $("btnBatchStop").disabled = true;

        await runMetaSelection({ allowWhileRunning: true });
      } catch (e) {
        if (isCancelledError(e)) setStatus("Auto-meta cancelled.", "muted");
        else setStatus(`Auto-meta failed: ${String(e)}`, "danger");
      }
    }
  } finally {
    BATCH_RUNNING = false;
    BATCH_STOP_REQUESTED = false;
    if ($("btnBatchStop")) $("btnBatchStop").disabled = true;

    if ($("valAutoRun")) $("valAutoRun").checked = prevAutoVal;
    if ($("valAutoExportXlsx")) $("valAutoExportXlsx").checked = prevAutoXlsx;
    if ($("valAutoRevealReport")) $("valAutoRevealReport").checked = prevAutoReveal;
    saveSettingsSoon();
    setRunning(false);
  }
}

$("btnLoad").addEventListener("click", async () => {
  try {
    await loadCsv();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnApplyTimeframe")?.addEventListener("click", async () => {
  try {
    await applySelectedDataTimeframe();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnBrowseCsv")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    return;
  }
  try {
    const picked = await invoke("pick_csv_file");
    const path = String(picked || "").trim();
    if (!path) return;
    $("csvPath").value = path;
    saveSettingsSoon();
    await loadCsv();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnPolygonOpen")?.addEventListener("click", async () => {
  if (RUNNING) return;
  openPolygonModal();
  await initPolygonUi();
});

$("btnPolygonClose")?.addEventListener("click", closePolygonModal);
$("polygonModalBackdrop")?.addEventListener("click", closePolygonModal);
document.addEventListener("keydown", (e) => {
  if (e.defaultPrevented) return;
  if (e.key !== "Escape") return;
  const modal = $("polygonModal");
  if (!modal || !modal.classList.contains("open")) return;
  closePolygonModal();
});

$("btnPolygonSaveKey")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    return;
  }
  try {
    const key = String($("polygonApiKey")?.value || "").trim();
    if (!key) {
      setStatus("Enter your Polygon API key first.", "danger");
      return;
    }
    await invoke("polygon_set_api_key", { apiKey: key });
    if ($("polygonApiKey")) $("polygonApiKey").value = "";
    setStatus("Saved Polygon API key.", "ok");
    await initPolygonUi();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnPolygonClearKey")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    return;
  }
  try {
    await invoke("polygon_clear_api_key");
    setStatus("Cleared Polygon API key.", "muted");
    await initPolygonUi();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("execRememberKeys")?.addEventListener("change", () => {
  if ($("execRememberKeys")?.checked) saveExecutionApiKeysToStorage();
  else clearExecutionApiKeysFromStorage();
  saveSettingsSoon();
});

$("execParamSource")?.addEventListener("change", () => {
  updateExecParamSourceUi();
  saveSettingsSoon();
});

for (const id of ["execApiKey", "execApiSecret"]) {
  const el = $(id);
  if (!el) continue;
  el.addEventListener("input", () => {
    if ($("execRememberKeys")?.checked) saveExecutionApiKeysToStorage();
  });
  el.addEventListener("change", () => {
    if ($("execRememberKeys")?.checked) saveExecutionApiKeysToStorage();
  });
}

$("btnExecClearKeys")?.addEventListener("click", () => {
  if (RUNNING || EXEC_BUSY) return;
  if ($("execApiKey")) $("execApiKey").value = "";
  if ($("execApiSecret")) $("execApiSecret").value = "";
  clearExecutionApiKeysFromStorage();
  EXEC_LAST_SNAPSHOT = null;
  renderExecOrdersTable(null);
  renderExecPositionMeta(null);
  clearExecTvChart("Load a dataset and refresh orders to overlay broker events.");
  setExecOutput("-");
  setExecStatus("Cleared execution API keys.", "muted");
});

$("btnExecRefresh")?.addEventListener("click", async () => {
  try {
    await refreshExecutionSnapshotUi();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnExecLiveStart")?.addEventListener("click", async () => {
  if (RUNNING || EXEC_BUSY) return;
  setExecBusy(true);
  try {
    await startExecLiveCandleStream();
  } catch (e) {
    const msg = String(e);
    setExecLiveStatus(`Live start failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
});

$("btnExecLiveStop")?.addEventListener("click", async () => {
  if (RUNNING || EXEC_BUSY) return;
  setExecBusy(true);
  try {
    await stopExecLiveCandleStream();
  } catch (e) {
    const msg = String(e);
    setExecLiveStatus(`Live stop failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
});

$("btnExecRunnerStart")?.addEventListener("click", async () => {
  if (RUNNING || EXEC_BUSY) return;
  setExecBusy(true);
  try {
    await startExecLiveRunner();
  } catch (e) {
    const msg = String(e);
    setExecRunnerStatus(`Auto-runner start failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
});

$("btnExecRunnerStop")?.addEventListener("click", async () => {
  if (RUNNING || EXEC_BUSY) return;
  setExecBusy(true);
  try {
    await stopExecLiveRunner();
  } catch (e) {
    const msg = String(e);
    setExecRunnerStatus(`Auto-runner stop failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
});

$("btnExecTestConnection")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    setExecStatus("Tauri invoke is unavailable.", "danger");
    return;
  }
  if (RUNNING || EXEC_BUSY) return;

  const mode = String($("execMode")?.value || "paper").trim().toLowerCase() === "live" ? "live" : "paper";
  const alpaca_api_key = String($("execApiKey")?.value || "").trim();
  const alpaca_api_secret = String($("execApiSecret")?.value || "").trim();
  if (!alpaca_api_key || !alpaca_api_secret) {
    setExecStatus("API key + secret are required for account test.", "danger");
    return;
  }

  setExecBusy(true);
  setExecStatus(`Testing Alpaca ${mode} account...`, "muted");
  try {
    const resp = await invoke("test_alpaca_connection", {
      req: { mode, alpaca_api_key, alpaca_api_secret },
    });
    setExecOutput(resp);
    const acct = String(resp?.account_number || resp?.account_id || "").trim();
    const st = String(resp?.status || "").trim();
    const suffix = `${acct ? ` account=${acct}` : ""}${st ? ` status=${st}` : ""}`;
    setExecStatus(`Alpaca ${mode} connection OK.${suffix}`, "ok");
    setStatus(`Alpaca ${mode} connection OK.`, "ok");
    saveExecutionApiKeysToStorage();
  } catch (e) {
    const msg = String(e);
    setExecOutput(msg);
    setExecStatus(`Connection test failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
});

$("btnExecRunOnce")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    setExecStatus("Tauri invoke is unavailable.", "danger");
    return;
  }
  if (RUNNING || EXEC_BUSY) return;

  let payload = null;
  try {
    payload = buildExecutionInvokePayload();
  } catch (e) {
    const msg = String(e);
    setExecStatus(msg, "danger");
    setExecOutput(msg);
    return;
  }

  const req = payload.req;
  const meta = payload.meta || {};
  if (!req.dry_run && (!req.alpaca_api_key || !req.alpaca_api_secret)) {
    setExecStatus("API key + secret are required unless dry-run is enabled.", "danger");
    return;
  }

  if (!req.dry_run) {
    const modeLabel = req.mode === "live" ? "LIVE" : "PAPER";
    const strategyLabel = strategyKindLabel(meta.strategy_kind);
    const confirmText =
      `Submit ${modeLabel} order now?\n` +
      `Symbol: ${req.symbol}\n` +
      `Qty: ${req.qty}\n` +
      `Strategy: ${strategyLabel}\n` +
      `Row source: ${meta.source || "selected"}`;
    if (!confirm(confirmText)) {
      setExecStatus("Execution cancelled.", "muted");
      return;
    }
  }

  setExecBusy(true);
  setExecStatus(
    `Executing signal (${req.mode}${req.dry_run ? ", dry-run" : ""}, ${meta.strategy_kind || "strategy"})...`,
    "muted",
  );
  let shouldAutoRefreshSnapshot = false;
  try {
    const resp = await invoke("execute_strategy_signal_once", { req });
    setExecOutput(resp);
    const action = String(resp?.action || "hold").trim() || "hold";
    const submitted = !!resp?.order_submitted;
    const orderId = String(resp?.order_id || "").trim();
    const warningCount = Array.isArray(resp?.warnings) ? resp.warnings.length : 0;
    let msg = submitted
      ? `Submitted ${action.toUpperCase()} order${orderId ? ` (id=${orderId})` : ""}.`
      : `Signal processed: action=${action}.`;
    if (req.dry_run) msg += " Dry-run only.";
    if (warningCount) msg += ` Warnings=${warningCount}.`;
    setExecStatus(msg, submitted ? "ok" : "muted");
    setStatus(msg, submitted ? "ok" : "muted");
    saveExecutionApiKeysToStorage();
    shouldAutoRefreshSnapshot = submitted && !req.dry_run;
  } catch (e) {
    const msg = String(e);
    setExecOutput(msg);
    setExecStatus(`Execution failed: ${msg}`, "danger");
    setStatus(msg, "danger");
  } finally {
    setExecBusy(false);
  }
  if (shouldAutoRefreshSnapshot) {
    await refreshExecutionSnapshotUi({ showOutput: false, silentStatus: true });
  }
});

$("btnPolygonDownloadLoad")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    return;
  }
  if (RUNNING) return;
  ensurePolygonDateDefaults();

  const market = String($("polygonMarket")?.value || "stocks").trim();
  const adjusted = market === "stocks" ? !!$("polygonStocksAdjusted")?.checked : false;
  const symbol = String($("polygonSymbol")?.value || "").trim();
  const timeframe = String($("polygonTimeframe")?.value || "1m").trim();
  const from = String($("polygonFrom")?.value || "").trim();
  const to = String($("polygonTo")?.value || "").trim();

  if (!symbol) {
    setStatus("Enter a symbol first (e.g. AAPL or btc/usd).", "danger");
    return;
  }

  setRunning(true);
  try {
    const tfLabel = timeframe || "1m";
    setStatus(tfLabel === "1m" ? "Downloading 1m bars from Polygon..." : `Downloading 1m bars from Polygon (deriving ${tfLabel})...`, "muted");
    const resp = await invoke("polygon_download_1m_and_load", { req: { market, symbol, timeframe, from, to, adjusted } });

    const path = String(resp?.csv_path || "").trim();
    if (path && $("csvPath")) $("csvPath").value = path;

    const id = String(resp?.id || "").trim();
    const meta = resp?.meta && typeof resp.meta === "object" ? resp.meta : null;
    const baseId = String(resp?.base_load?.id || "").trim() || id;
    const baseMeta = resp?.base_load?.meta && typeof resp.base_load.meta === "object" ? resp.base_load.meta : meta;
    const baseTf = String(baseMeta?.timeframe || "1m").trim() || "1m";
    const curTf = String(tfLabel || baseTf).trim() || baseTf;
    CURRENT_BASE_DATA_ID = baseId;
    CURRENT_BASE_DATA_META = baseMeta;
    CURRENT_BASE_DATASET_CTX = {
      source: "polygon",
      label: symbol,
      symbol,
      market,
      adjusted,
      timeframe: baseTf,
      base_timeframe: baseTf,
      derived_timeframe: baseTf,
      resample_applied: false,
      resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
      resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
    };
    CURRENT_DATA_META = meta;
    CURRENT_DATASET_CTX = {
      ...CURRENT_BASE_DATASET_CTX,
      timeframe: curTf,
      base_timeframe: baseTf,
      derived_timeframe: curTf,
      resample_applied: !!(baseTf && curTf && baseTf !== curTf),
      resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
      resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
    };
    clearDrillCaches({ clearBars: true });
    $("dataId").textContent = id || "—";
    renderDataMeta(meta);
    applyMaSourceConstraints();
    applyBackendMaConstraints();
    updateOverviewEmptyUi();

    const dlBars = Number(resp?.downloaded_bars || 0);
    const usedCacheOnly = !!resp?.used_cache_only;
    const msg = usedCacheOnly ? "Loaded from cache." : `Downloaded ${dlBars} new bar${dlBars === 1 ? "" : "s"} and loaded.`;
    setStatus(msg, "ok");
    $("output").textContent = pretty(resp);

    if (path) addRecentCsvPath(path);
    saveSettingsSoon();
    if (EXEC_LAST_SNAPSHOT) {
      try {
        await renderExecSnapshotChart(EXEC_LAST_SNAPSHOT);
      } catch {}
    } else {
      clearExecTvChart("Load a dataset and refresh orders to overlay broker events.");
    }
    await initPolygonUi();
    await initCrossTickerList();
    closePolygonModal();
  } catch (e) {
    setStatus(String(e), "danger");
  } finally {
    setRunning(false);
  }
});

$("polygonMarket")?.addEventListener("change", () => {
  updatePolygonAdjustedUi();
  saveSettingsSoon();
});

$("btnBrowseExportAll")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) {
    setStatus("Tauri invoke not available (window.__TAURI__ missing).", "danger");
    return;
  }
  try {
    const picked = await invoke("pick_save_csv", { defaultName: "all_pairs.csv" });
    const path = String(picked || "").trim();
    if (!path) return;
    $("exportAllCsvPath").value = path;
    saveSettingsSoon();
    setStatus("Selected export path.", "muted");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnPresetKernelSmaAlma")?.addEventListener("click", () => {
  if ($("backend")) $("backend").value = "GpuKernel";
  applyBackendSelectionConstraints();
  updateDeviceIdEnabled();

  if ($("maSource")) $("maSource").value = "close";

  if ($("fastMaType") && MA_BY_ID["sma"]) $("fastMaType").value = "sma";
  if ($("slowMaType") && MA_BY_ID["alma"]) $("slowMaType").value = "alma";

  applyMaSourceConstraints();
  applyBackendMaConstraints();
  refreshMaParamsUi();
  refreshGpuStatusUi();
  saveSettingsSoon();
  setStatus("Applied preset: GPU kernel (SMA/ALMA)", "muted");
});

$("btnRefreshGpuStatus")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await refreshGpuStatusUi();
    setStatus("Refreshed GPU status.", "muted");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnGpuParityCheck")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await runGpuParityCheck();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Parity cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnClear").addEventListener("click", () => {
  clearProgressUi();
  $("dataId").textContent = "—";
  CURRENT_DATA_META = null;
  CURRENT_DATASET_CTX = null;
  CURRENT_BASE_DATA_ID = "";
  CURRENT_BASE_DATA_META = null;
  CURRENT_BASE_DATASET_CTX = null;
  renderDataMeta(null);
  applyMaSourceConstraints();
  applyBackendMaConstraints();
  $("output").textContent = "{}";
  if ($("validationMeta")) $("validationMeta").textContent = "-";
  if ($("validationOutput")) $("validationOutput").textContent = "-";
  $("bestSummary").textContent = "-";
  resetBestCardUi();
  $("topTableBody").innerHTML = "";
  if ($("historySelect")) $("historySelect").value = "";
  CURRENT_RUN = null;
  TOP_ROWS = [];
  TOP_ROWS_FILTERED = [];
  PARETO_POINTS_RENDERED = [];
  updateTopTableNoRunUi();
  const meta = $("topFilterMeta");
  if (meta) meta.textContent = "-";
  const pareto = $("paretoWrap");
  if (pareto) pareto.style.display = "none";
  setHintVisible("paretoEmptyHint", true);
  clearDrillCaches({ clearBars: true });
  setDrillVisible(false);
  EXEC_LAST_SNAPSHOT = null;
  renderExecOrdersTable(null);
  renderExecPositionMeta(null);
  clearExecTvChart("Load a dataset and refresh orders to overlay broker events.");
  $("heatmapWrap").style.display = "none";
  setHintVisible("heatmapEmptyHint", true);
  renderCompare();
  setStatus("Ready.", "");
  updateValidateTabUi();
  updateQuickActionsUi();
  updateOverviewEmptyUi();
});

$("btnRun").addEventListener("click", async () => {
  try {
    await runWorkflow();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnCancel").addEventListener("click", async () => {
  try {
    $("btnCancel").disabled = true;
    if (BATCH_RUNNING) {
      BATCH_STOP_REQUESTED = true;
      updateBatchMeta("stop requested");
    }
    setStatus("Cancel requested... (may take a moment to stop safely)", "muted");
    await requestCancelCurrentRun();
  } catch (e) {
    setStatus(String(e), "danger");
  } finally {
    if ($("btnCancel")) $("btnCancel").disabled = !RUNNING;
  }
});

$("btnValidate")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await runValidation();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Validation cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnOpenReportsDir")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    setStatus("Opening reports folder...", "muted");
    const dir = await invoke("open_reports_dir");
    setStatus(`Opened reports folder: ${String(dir || "").trim()}`, "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnOpenLatestReport")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  const path = String(CURRENT_RUN?.report_xlsx_path || "").trim();
  if (!path) {
    setStatus("No report yet (run validation + export XLSX).", "muted");
    return;
  }
  try {
    await invoke("open_path", { path });
    setStatus("Opened report.", "ok");
  } catch {
    await invoke("reveal_path", { path });
    setStatus("Revealed report.", "ok");
  } finally {
    updateQuickActionsUi();
  }
});

$("btnOpenLatestMetaReport")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  const path = String(LAST_META_RUN?.report_xlsx_path || "").trim();
  if (!path) {
    setStatus("No meta report yet (run meta analysis).", "muted");
    return;
  }
  try {
    await invoke("open_path", { path });
    setStatus("Opened meta report.", "ok");
  } catch {
    await invoke("reveal_path", { path });
    setStatus("Revealed meta report.", "ok");
  } finally {
    updateQuickActionsUi();
  }
});

$("btnCrossRefresh")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await initCrossTickerList();
    setStatus("Refreshed cross-ticker list.", "muted");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("valCrossSearch")?.addEventListener("input", () => {
  applyCrossTickerFilter();
});
$("valCrossSearch")?.addEventListener("change", () => {
  applyCrossTickerFilter();
});

$("btnCrossSelectAll")?.addEventListener("click", () => {
  if (RUNNING) return;
  selectAllVisibleCross();
  setStatus("Cross-ticker: selected all (filtered view).", "muted");
});
$("btnCrossSelectNone")?.addEventListener("click", () => {
  if (RUNNING) return;
  selectNoneVisibleCross();
  setStatus("Cross-ticker: cleared selection (filtered view).", "muted");
});
$("btnCrossSelectMatch")?.addEventListener("click", () => {
  if (RUNNING) return;
  selectMatchCurrentCross();
  setStatus("Cross-ticker: selected datasets matching current dataset.", "muted");
});

$("strategyKind")?.addEventListener("change", () => {
  if (strategyKind() === "CompositeMo") {
    setWorkflowAdvancedEnabled(true, { persist: true });
    CONTROLS_TABS?.set("advanced", { save: true, focus: false });
  }
  updateStrategyUi();
  applyBackendMaConstraints();
  refreshMaParamsUi();
  saveSettingsSoon();
});

$("compositeSignalMode")?.addEventListener("change", () => {
  updateStrategyUi();
  updateRunPlanUi();
  renderContextChips();
});

$("trendUseAdx")?.addEventListener("change", () => {
  updateStrategyUi();
  updateRunPlanUi();
  renderContextChips();
  saveSettingsSoon();
});

$("breakoutUseAdx")?.addEventListener("change", () => {
  updateStrategyUi();
  updateRunPlanUi();
  renderContextChips();
  saveSettingsSoon();
});

$("btnApplyPreset")?.addEventListener("click", () => {
  if (RUNNING) return;
  applyPresetProfile($("presetProfile")?.value || "Balanced");
});

$("backend").addEventListener("change", () => {
  updateDeviceIdEnabled();
  applyBackendSelectionConstraints();
  applyBackendMaConstraints();
  refreshGpuStatusUi();
  saveSettingsSoon();
  renderContextChips();
});
$("deviceId").addEventListener("change", () => {
  rememberDesiredDeviceIdFromUi();
  refreshGpuStatusUi();
  saveSettingsSoon();
  renderContextChips();
});
$("metaGroupingMode")?.addEventListener("change", updateMetaGroupingUi);
$("metaStrategyKind")?.addEventListener("change", updateMetaHoldoutUi);
$("metaEvalHoldout")?.addEventListener("change", updateMetaHoldoutUi);
$("metaHoldoutMode")?.addEventListener("change", () => {
  updateMetaHoldoutUi();
  saveSettingsSoon();
});

$("moSampler")?.addEventListener("change", () => {
  updateMoSamplerUi();
  saveSettingsSoon();
});

$("moPrescanEnable")?.addEventListener("change", () => {
  updateMoSamplerUi();
  updateDeviceIdEnabled();
  updateRunPlanUi();
  saveSettingsSoon();
});

$("moUseRangeBuilder")?.addEventListener("change", () => {
  updateMoRangeBuilderUi();
  saveSettingsSoon();
});

$("btnMoResetSearchSpace")?.addEventListener("click", () => {
  const el = $("moSearchSpace");
  if (!el) return;
  el.value = JSON.stringify(defaultCompositeMoSearchSpace(), null, 2);
  saveSettingsSoon();
  setStatus("Reset Composite MO search space.", "muted");
});

for (const id of ["csvPath", "objective", "moObjectiveMin", "mode", "maSource"]) {
  const el = $(id);
  if (!el) continue;
  const onChange = () => renderContextChips();
  el.addEventListener("change", onChange);
  el.addEventListener("input", onChange);
}

for (const id of [
  "csvPath",
  "dataTimeframe",
  "autoRestoreDataset",
  "polygonMarket",
  "polygonStocksAdjusted",
  "polygonSymbol",
  "polygonTimeframe",
  "polygonFrom",
  "polygonTo",
  "strategyKind",
  "presetProfile",
  "compareSelect",
  "fastMaType",
  "slowMaType",
  "fastStart",
  "fastEnd",
  "fastStep",
  "slowStart",
  "slowEnd",
  "slowStep",
  "maSource",
  "objective",
  "mode",
  "vdpsEta",
  "vdpsRungs",
  "vdpsMinInitialPairs",
  "vdpsMaxSurvivorSeeds",
  "vdpsMaxPeriodCandidates",
  "vdpsRandomInjectionRatio",
  "vdpsEliteMutationRatio",
  "vdpsDifferentialRatio",
  "vdpsDeterministic",
  "vdpsSeed",
  "vdpsQualityCheckMaxPairs",
  "vdpsQualityCheckTopK",
  "topK",
  "includeAll",
  "exportAllCsvPath",
  "heatmapBins",
  "commission",
  "costPreset",
  "feeBps",
  "spreadBps",
  "slippageBps",
  "epsRel",
  "startingEquity",
  "stopLossPct",
  "takeProfitPct",
  "trailingStopPct",
  "cooldownBars",
  "signalPersistenceBars",
  "dmConstraintMinTrades",
  "dmConstraintMaxDd",
  "exitMode",
  "kaufmanStopPeriod",
  "kaufmanStopMult",
  "kaufmanStopMaType",
  "trendLrPeriod",
  "trendLrMinDeg",
  "trendBbwPeriod",
  "trendBbwMult",
  "trendBbwMin",
  "trendRocPeriod",
  "trendUseRoc",
  "trendUseAdx",
  "trendAdxPeriod",
  "trendAdxMin",
  "regimeAdxPeriod",
  "regimeAdxEnter",
  "regimeAdxExit",
  "regimeRevEntryRel",
  "regimeRevExitRel",
  "breakoutBbPeriod",
  "breakoutBbMult",
  "breakoutConfirmBars",
  "breakoutMacdFast",
  "breakoutMacdSlow",
  "breakoutMacdSignal",
  "breakoutCmoPeriod",
  "breakoutCmoMinAbs",
  "breakoutAdoscShort",
  "breakoutAdoscLong",
  "breakoutUseAdx",
  "breakoutAdxPeriod",
  "breakoutAdxMin",
  "execMode",
  "execSymbol",
  "execQty",
  "execParamSource",
  "execManualFastLen",
  "execManualSlowLen",
  "execRiskStopLossPct",
  "execRiskTakeProfitPct",
  "execDryRun",
  "execRememberKeys",
  "execLiveUseDelayed",
    "longOnly",
    "allowFlip",
    "tradeOnNext",
    "compositeSignalMode",
    "halfTrendAmplitude",
  "halfTrendAtrPeriod",
  "halfTrendChannelDev",
  "maczFastLength",
  "maczSlowLength",
  "maczSignalLength",
  "maczLengthZ",
  "maczLengthStdev",
  "maczGamma",
  "maczA",
  "maczB",
  "maczUseVolume",
  "maczUseLag",
  "ottPeriod",
  "ottPercent",
  "ottMaType",
  "filterMinPnl",
  "filterMinSharpe",
  "filterMaxDd",
  "filterMinTrades",
  "filterMinExposure",
  "paretoX",
  "paretoY",
  "moObjectiveMin",
  "moSampler",
  "moNsga2Seed",
  "moNsga2Population",
  "moNsga2Generations",
  "moNsga2CrossoverProb",
  "moNsga2SwappingProb",
  "moNsga2MutationProb",
  "moRandomSeed",
  "moRandomTrials",
  "moTpeSeed",
  "moTpeTrials",
  "moTpeStartupTrials",
  "moTpeCandidates",
  "moTpeGamma",
  "moTpeBatchSize",
  "moTpeMaxModelPoints",
  "moQmcSeed",
  "moQmcTrials",
  "moQmcSequence",
  "moConstraintMinTrades",
  "moConstraintMaxDd",
  "moConstraintMaxAbsExposure",
  "moPrescanEnable",
  "moPrescanRequireGpu",
  "moPrescanTopK",
  "moPrescanWindow",
  "moPrescanPredictiveTopK",
  "moPrescanPredictiveStride",
  "moPrescanObjectives",
  "moPrescanMode",
  "moPrescanMaPairs",
  "moHtAmpLow",
  "moHtAmpHigh",
  "moHtAmpStep",
  "moHtDevLow",
  "moHtDevHigh",
  "moHtDevStep",
  "moHtAtrLow",
  "moHtAtrHigh",
  "moHtAtrStep",
  "moMaczFastLow",
  "moMaczFastHigh",
  "moMaczFastStep",
  "moMaczSlowLow",
  "moMaczSlowHigh",
  "moMaczSlowStep",
  "moMaczSignalLow",
  "moMaczSignalHigh",
  "moMaczSignalStep",
  "moMaczLengthZLow",
  "moMaczLengthZHigh",
  "moMaczLengthZStep",
  "moMaczLengthStdevLow",
  "moMaczLengthStdevHigh",
  "moMaczLengthStdevStep",
  "moMaczGammaLow",
  "moMaczGammaHigh",
  "moMaczGammaStep",
  "moMaczALow",
  "moMaczAHigh",
  "moMaczAStep",
  "moMaczBLow",
  "moMaczBHigh",
  "moMaczBStep",
  "moMaczUseLagMode",
  "moMaczUseVolumeMode",
  "moOttPeriodLow",
  "moOttPeriodHigh",
  "moOttPeriodStep",
  "moOttPercentLow",
  "moOttPercentHigh",
  "moOttPercentStep",
  "moOttMaTypes",
  "moUseRangeBuilder",
  "moSearchSpace",
  "valCandidates",
  "valSeed",
  "valAutoRun",
  "valAutoExportXlsx",
  "valAutoRevealReport",
  "valAutoExportTopN",
  "valWfEnable",
  "valWfWindow",
  "valWfStep",
  "valPerturbEnable",
  "valPerturbIters",
  "valPerturbMin",
  "valPerturbMax",
  "valMcEnable",
  "valMcSims",
  "valCscvEnable",
  "valCscvSlices",
  "valCrossEnable",
  "valCrossMax",
  "valCrossSelected",
  "metaStrategyKind",
  "metaTimeframe",
  "metaRequiredTag",
  "metaMinSamples",
  "metaTopK",
  "metaHoldoutTag",
  "metaEvalHoldout",
  "metaHoldoutReps",
  "metaGroupingMode",
  "metaClusterIntBucket",
  "metaClusterFloatBucket",
  "metaStage4TopK",
  "metaStage4MinConf",
  "metaStage4MetaWeight",
  "metaStage4MlWeight",
  "metaStage4UncertaintyWeight",
  "metaStage4AcceptConf",
  "metaStage4ReviewConf",
  "metaStage4MaxAcceptUncertainty",
  "metaStage4AutoTrain",
  "metaStage4RequireHealthy",
  "metaStage4FallbackToMeta",
  "metaSavedSearch",
  "stage4SavedSearch",
  "metaCompareSelect",
]) {
  const el = $(id);
  if (!el) continue;
  el.addEventListener("change", saveSettingsSoon);
  el.addEventListener("input", saveSettingsSoon);
}

// Execution costs UI: keep derived commission and hint in sync.
$("costPreset")?.addEventListener("change", () => {
  const id = String($("costPreset")?.value || "custom").trim() || "custom";
  if (id !== "custom") applyCostPreset(id, { silent: false });
  else syncCostUi({ save: false });
});
for (const id of ["feeBps", "spreadBps", "slippageBps"]) {
  const el = $(id);
  if (!el) continue;
  const onChange = () => {
    const presetEl = $("costPreset");
    if (presetEl && presetEl.value !== "custom") presetEl.value = "custom";
    syncCostUi({ save: false });
  };
  el.addEventListener("change", onChange);
  el.addEventListener("input", onChange);
}

for (const id of ["fastStart", "fastEnd", "fastStep", "slowStart", "slowEnd", "slowStep", "includeAll"]) {
  const el = $(id);
  if (!el) continue;
  el.addEventListener("change", updatePairsPreviewUi);
  el.addEventListener("input", updatePairsPreviewUi);
}

for (const id of [
  "strategyKind",
  "backend",
  "deviceId",
  "fastStart",
  "fastEnd",
  "fastStep",
  "slowStart",
  "slowEnd",
  "slowStep",
  "fastMaType",
  "slowMaType",
  "maSource",
  "objective",
  "moObjectiveMin",
  "mode",
  "vdpsEta",
  "vdpsRungs",
  "vdpsMinInitialPairs",
  "vdpsMaxSurvivorSeeds",
  "vdpsMaxPeriodCandidates",
  "vdpsRandomInjectionRatio",
  "vdpsEliteMutationRatio",
  "vdpsDifferentialRatio",
  "vdpsDeterministic",
  "vdpsSeed",
  "vdpsQualityCheckMaxPairs",
  "vdpsQualityCheckTopK",
  "includeAll",
  "startingEquity",
  "stopLossPct",
  "takeProfitPct",
  "trailingStopPct",
  "cooldownBars",
  "signalPersistenceBars",
  "dmConstraintMinTrades",
  "dmConstraintMaxDd",
  "exitMode",
  "kaufmanStopPeriod",
  "kaufmanStopMult",
  "kaufmanStopMaType",
  "trendLrPeriod",
  "trendLrMinDeg",
  "trendBbwPeriod",
  "trendBbwMult",
  "trendBbwMin",
  "trendRocPeriod",
  "trendUseRoc",
  "trendUseAdx",
  "trendAdxPeriod",
  "trendAdxMin",
  "regimeAdxPeriod",
  "regimeAdxEnter",
  "regimeAdxExit",
  "regimeRevEntryRel",
  "regimeRevExitRel",
  "breakoutBbPeriod",
  "breakoutBbMult",
  "breakoutConfirmBars",
  "breakoutMacdFast",
  "breakoutMacdSlow",
  "breakoutMacdSignal",
  "breakoutCmoPeriod",
  "breakoutCmoMinAbs",
  "breakoutAdoscShort",
  "breakoutAdoscLong",
  "breakoutUseAdx",
  "breakoutAdxPeriod",
  "breakoutAdxMin",
  "moSampler",
  "moPrescanEnable",
  "moPrescanRequireGpu",
  "moPrescanTopK",
  "moPrescanWindow",
  "moPrescanPredictiveTopK",
  "moPrescanPredictiveStride",
  "moPrescanObjectives",
  "moPrescanMode",
  "moPrescanMaPairs",
  "valAutoRun",
  "valAutoExportXlsx",
  "valAutoRevealReport",
  "valWfEnable",
  "valPerturbEnable",
  "valMcEnable",
  "valCscvEnable",
  "valCrossEnable",
]) {
  const el = $(id);
  if (!el) continue;
  el.addEventListener("change", updateRunPlanUi);
  el.addEventListener("input", updateRunPlanUi);
}

for (const id of ["valCandidates", "valWfEnable", "valPerturbEnable", "valMcEnable", "valCscvEnable", "valCrossEnable"]) {
  const el = $(id);
  if (!el) continue;
  el.addEventListener("change", updateValidateTabUi);
  el.addEventListener("input", updateValidateTabUi);
}

for (const id of [
  "filterMinPnl",
  "filterMinSharpe",
  "filterMaxDd",
  "filterMinTrades",
  "filterMinExposure",
  "paretoX",
  "paretoY",
]) {
  const el = $(id);
  if (!el) continue;
  const rerender = () => {
    if (CURRENT_RUN) renderTopTable(CURRENT_RUN.res, CURRENT_RUN.req);
  };
  el.addEventListener("change", rerender);
  el.addEventListener("input", rerender);
}

$("btnClearTopFilters")?.addEventListener("click", () => {
  for (const id of ["filterMinPnl", "filterMinSharpe", "filterMaxDd", "filterMinTrades", "filterMinExposure"]) {
    const el = $(id);
    if (el) el.value = "";
  }
  saveSettingsSoon();
  if (CURRENT_RUN) renderTopTable(CURRENT_RUN.res, CURRENT_RUN.req);
});

$("topTable")?.addEventListener("click", (ev) => {
  const th = ev?.target instanceof Element ? ev.target.closest("#topTable th[data-key]") : null;
  if (!th) return;
  const key = th.getAttribute("data-key") || "rank";
  if (TOP_SORT.key === key) TOP_SORT.dir = TOP_SORT.dir === "asc" ? "desc" : "asc";
  else {
    TOP_SORT.key = key;
    TOP_SORT.dir = (key === "rank" || key === "minimize_value" || key === "violation") ? "asc" : "desc";
  }
  if (CURRENT_RUN) renderRun(CURRENT_RUN.res, CURRENT_RUN.req);
});

$("btnExportJson")?.addEventListener("click", () => {
  if (!CURRENT_RUN?.res) {
    setStatus("No result to export yet.", "danger");
    return;
  }
  downloadText(`vectorbt_run_${CURRENT_RUN.ts}.json`, pretty(CURRENT_RUN.res), "application/json");
});

$("btnExportCsv")?.addEventListener("click", () => {
  if (!CURRENT_RUN?.res) {
    setStatus("No result to export yet.", "danger");
    return;
  }

  const obj = CURRENT_RUN.req?.objective || "Sharpe";
  const startEq = startingEquityFromReq(CURRENT_RUN?.req);
  const baseRows = Array.isArray(TOP_ROWS_FILTERED) ? TOP_ROWS_FILTERED : TOP_ROWS;
  const sorted = sortRows(baseRows, TOP_SORT.key, TOP_SORT.dir, obj);
  const lines = ["rank,fast_ma,fast_len,slow_ma,slow_len,pnl,pnl_pct,final_equity,sharpe,max_dd,trades,exposure,net_exposure,score,params_json"];
  for (const row of sorted) {
    const p = row.params || {};
    const m = row.metrics || {};
    const score = scoreOf(m, obj);
    const pnl = Number(m.pnl || 0);
    const finalEq = finalEquityFromPnl(pnl, startEq);
    const paramsJson = (() => {
      try { return JSON.stringify(p); } catch { return "{}"; }
    })();
    lines.push([
      row.rank,
      csvEscape(p.fast_ma_type || ""),
      p.fast_len || 0,
      csvEscape(p.slow_ma_type || ""),
      p.slow_len || 0,
      pnl,
      pnl * 100,
      Number(finalEq || 0),
      Number(m.sharpe || 0),
      Number(m.max_dd || 0),
      Number(m.trades || 0),
      Number(m.exposure || 0),
      Number(m.net_exposure || 0),
      Number(score || 0),
      csvEscape(paramsJson),
    ].join(","));
  }
  downloadText(`vectorbt_top_${CURRENT_RUN.ts}.csv`, lines.join("\n"), "text/csv");
});

$("btnHeatmapExportPng")?.addEventListener("click", () => {
  try {
    exportHeatmapPng();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

for (const id of ["heatmapColor", "heatmapRange", "heatmapShowTop", "heatmapShowCrosshair"]) {
  $(id)?.addEventListener("change", () => {
    saveSettingsSoon();
    if (CURRENT_RUN) renderHeatmap(CURRENT_RUN.res, CURRENT_RUN.req);
  });
}

$("btnExportValidationCsv")?.addEventListener("click", () => {
  exportValidationShortlistCsv();
});

$("btnExportValidationJson")?.addEventListener("click", () => {
  exportValidationJson();
});

$("btnExportReportJson")?.addEventListener("click", () => {
  exportReportJson();
});

$("btnExportReportXlsx")?.addEventListener("click", async () => {
  try {
    await exportReportXlsx();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnClearHistory")?.addEventListener("click", () => {
  setHistory([]);
  populateHistorySelect([], "");
  populateCompareSelect([], "");
  CURRENT_RUN = null;
  clearValidationUi();
  clearDrillCaches({ clearBars: false });
  setDrillVisible(false);
  renderCompare();
  setStatus("History cleared.", "muted");
});

$("btnApplyHistory")?.addEventListener("click", () => {
  if (!CURRENT_RUN?.req) {
    setStatus("Select a run from history first.", "muted");
    return;
  }
  applyRequestToUi(CURRENT_RUN.req);
  setStatus("Applied history run settings to the form.", "ok");
});

$("btnExportPreset")?.addEventListener("click", () => {
  const preset = readSettingsFromUi();
  const ts = new Date().toISOString().replaceAll(":", "-");
  const payload = { version: 1, kind: "vectorbt_preset", ts, preset };
  downloadText(`vectorbt_preset_${ts}.json`, JSON.stringify(payload, null, 2), "application/json");
  setStatus("Exported preset.", "ok");
});

$("btnImportPreset")?.addEventListener("click", () => {
  $("presetFile")?.click();
});

$("presetFile")?.addEventListener("change", async () => {
  const file = $("presetFile")?.files?.[0];
  if (!file) return;
  try {
    const text = await file.text();
    const obj = JSON.parse(text);
    const preset = obj?.preset && typeof obj.preset === "object" ? obj.preset : obj;
    applySettingsObject(preset);
    setStatus("Imported preset.", "ok");
  } catch (e) {
    setStatus(`Failed to import preset: ${String(e)}`, "danger");
  } finally {
    try { $("presetFile").value = ""; } catch {}
  }
});

$("historySelect")?.addEventListener("change", () => {
  const ts = $("historySelect").value || "";
  const history = getHistory();
  const entry = history.find((h) => h.ts === ts);
  if (!entry) return;
  CURRENT_RUN = { ts: entry.ts, req: entry.req, res: entry.res };
  clearValidationUi();
  $("output").textContent = pretty(entry.res);
  renderRun(entry.res, entry.req);
  setStatus(`Loaded run ${entry.ts}`, "muted");
});

$("savedRunsSelect")?.addEventListener("change", () => {
  syncSavedRunSelectionUi();
});

$("savedRunsSearch")?.addEventListener("input", () => {
  renderSavedRunsSelect();
});

$("btnSaveSavedRunTags")?.addEventListener("click", async () => {
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved run first.", "muted");
    return;
  }
  try {
    const tags = parseTagsInput($("savedRunsTags")?.value || "");
    await saveSavedRunTags(path, tags);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRefreshSavedRuns")?.addEventListener("click", async () => {
  await refreshSavedRuns();
  setStatus("Refreshed saved runs.", "muted");
});

$("btnOpenRunsDir")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  try {
    setStatus("Opening runs folder...", "muted");
    const dir = await invoke("open_runs_dir");
    setStatus(`Opened runs folder: ${String(dir || "").trim()}`, "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRevealSavedRun")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved run first.", "muted");
    return;
  }
  try {
    await invoke("reveal_path", { path });
    setStatus("Revealed saved run.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnDeleteSavedRun")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved run first.", "muted");
    return;
  }
  if (!confirm("Delete this saved run artifact file?")) return;
  try {
    await invoke("delete_run_artifact", { path });
    await refreshSavedRuns("");
    setStatus("Deleted saved run.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnLoadSavedRun")?.addEventListener("click", async () => {
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved run first.", "muted");
    return;
  }
  try {
    await loadSavedRun(path);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnApplySavedRun")?.addEventListener("click", async () => {
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved run first.", "muted");
    return;
  }
  try {
    await applySavedRunToForm(path);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRerunSavedRun")?.addEventListener("click", async () => {
  const path = String($("savedRunsSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved run first.", "muted");
    return;
  }
  try {
    await rerunSavedRun(path);
  } catch (e) {
    if (isCancelledError(e)) setStatus("Cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnOpenSavedRunReport")?.addEventListener("click", async () => {
  try {
    await openSelectedSavedRunReport();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRunBatch")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await runBatchWorkflow();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Batch cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnBatchStop")?.addEventListener("click", () => {
  if (!BATCH_RUNNING) return;
  BATCH_STOP_REQUESTED = true;
  updateBatchMeta("stop after current");
  setStatus("Batch will stop after the current dataset.", "muted");
});

$("btnBatchRevealSummary")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  const p = String(LAST_BATCH_SUMMARY?.path || "").trim();
  if (!p) {
    setStatus("No batch summary saved yet.", "muted");
    return;
  }
  try {
    await invoke("reveal_path", { path: p });
    setStatus("Revealed batch summary.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnBatchRetryFailed")?.addEventListener("click", async () => {
  if (RUNNING) return;
  const summary = LAST_BATCH_SUMMARY?.summary && typeof LAST_BATCH_SUMMARY.summary === "object" ? LAST_BATCH_SUMMARY.summary : null;
  if (!summary) {
    setStatus("No batch summary loaded.", "muted");
    return;
  }
  const failed = Number(summary?.counts?.failed || summary?.results?.failed?.length || 0);
  if (failed <= 0) {
    setStatus("No failures to retry.", "muted");
    return;
  }
  try {
    const tags = Array.isArray(summary.tags) ? summary.tags.join(", ") : "";
    if ($("batchTags")) $("batchTags").value = tags;
    if ($("batchHoldoutPct")) $("batchHoldoutPct").value = String(Math.floor(Number(summary.holdout_pct || 0)));
    if ($("batchHoldoutTag")) $("batchHoldoutTag").value = String(summary.holdout_tag || "holdout");
    if ($("batchSkipDone")) $("batchSkipDone").checked = true;

    const sel = Array.isArray(summary.selected_paths) ? summary.selected_paths : [];
    setBatchSelectedPaths(sel);
    renderBatchDatasetList();
    updateBatchMeta(`retrying failed (tag=${String(summary.batch_tag || "").trim() || "batch"})`);
    saveSettingsSoon();
    await runBatchWorkflow();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Retry cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnBatchRefresh")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await initBatchDatasetList();
    setStatus("Refreshed batch dataset list.", "muted");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnBatchSelectAll")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    if (!Array.isArray(BATCH_DATASETS_CACHE) || !BATCH_DATASETS_CACHE.length) {
      await initBatchDatasetList();
    }
    const q = batchDatasetQuery();
    const items = filterBatchDatasets(Array.isArray(BATCH_DATASETS_CACHE) ? BATCH_DATASETS_CACHE : [], q);
    const paths = [];
    for (const ds of items) {
      const p = String(ds?.csv_path || "").trim();
      if (!p) continue;
      paths.push(p);
    }
    setBatchSelectedPaths(paths);
    renderBatchDatasetList();
    updateBatchMeta("");
    saveSettingsSoon();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnBatchSelectNone")?.addEventListener("click", () => {
  if (RUNNING) return;
  setBatchSelectedPaths([]);
  renderBatchDatasetList();
  updateBatchMeta("");
  saveSettingsSoon();
});

$("batchDatasetSearch")?.addEventListener("input", () => {
  renderBatchDatasetList();
  updateBatchMeta("");
  saveSettingsSoon();
});

$("batchTags")?.addEventListener("input", saveSettingsSoon);
$("batchHoldoutPct")?.addEventListener("input", saveSettingsSoon);
$("batchHoldoutTag")?.addEventListener("input", saveSettingsSoon);
$("batchAutoMeta")?.addEventListener("change", saveSettingsSoon);
$("batchSkipDone")?.addEventListener("change", saveSettingsSoon);

$("btnRunMeta")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await runMetaSelection();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Meta cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnOpenMetaReport")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  const path = String(LAST_META_RUN?.report_xlsx_path || "").trim();
  if (!path) {
    setStatus("No meta report path yet. Run meta analysis first.", "muted");
    return;
  }
  try {
    await invoke("open_path", { path });
    setStatus("Opened meta report.", "ok");
  } catch {
    await invoke("reveal_path", { path });
    setStatus("Revealed meta report.", "ok");
  }
});

$("btnRevealMetaFiles")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  const p1 = String(LAST_META_RUN?.report_xlsx_path || "").trim();
  const p2 = String(LAST_META_RUN?.meta_json_path || "").trim();
  const path = p1 || p2;
  if (!path) {
    setStatus("No meta files yet. Run meta analysis first.", "muted");
    return;
  }
  try {
    await invoke("reveal_path", { path });
    setStatus("Revealed meta files.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnMetaMlRefresh")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await refreshMetaMlStatus();
    setStatus("Refreshed ML confidence status.", "muted");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnMetaMlTrain")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await trainMetaMlModel();
    if (metaMlEnabled() && LAST_META_RUN?.result?.groups?.length) {
      try {
        await applyMetaMlConfidenceToLastMeta({ silent: true });
      } catch {}
    }
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnMetaMlApply")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await applyMetaMlConfidenceToLastMeta();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnMetaMlDelete")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  if (!confirm("Delete the saved ML confidence model for this scope?")) return;
  const scope = metaMlScopeFromUi();
  if (!String(scope.timeframe || "").trim()) {
    setStatus("Timeframe required (set Meta timeframe filter or run meta once).", "muted");
    return;
  }
  try {
    await invoke("delete_ml_confidence_model", { scope });
    LAST_META_ML_STATUS = null;
    setMetaMlStatus("Deleted model.");
    updateMetaMlButtons();
    setStatus("Deleted ML confidence model.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRunMetaStage4")?.addEventListener("click", async () => {
  if (RUNNING) return;
  try {
    await runMetaStage4Experimental();
  } catch (e) {
    if (isCancelledError(e)) setStatus("Stage 4 cancelled.", "muted");
    else setStatus(String(e), "danger");
  }
});

$("btnRevealMetaStage4")?.addEventListener("click", async () => {
  const invoke = tauriInvoke();
  if (!invoke) return;
  if (RUNNING) return;
  const path = String(LAST_META_STAGE4_RUN?.stage4_json_path || "").trim();
  if (!path) {
    setStatus("No Stage 4 artifact saved yet.", "muted");
    return;
  }
  try {
    await invoke("reveal_path", { path });
    setStatus("Revealed Stage 4 artifact.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("metaUseMlConfidence")?.addEventListener("change", async () => {
  saveSettingsSoon();
  updateMetaMlButtons();
  if (metaMlEnabled()) {
    try {
      await maybeAutoTrainAndApplyMetaMl({ silent: true });
    } catch {}
  } else if (LAST_META_RUN?.result?.groups?.length) {
    // If disabling ML sorting, re-render in original order (meta_score order from backend).
    renderMetaSelection(LAST_META_RUN);
  }
});

$("metaMlAutopilot")?.addEventListener("change", async () => {
  saveSettingsSoon();
  updateMetaMlButtons();
  if (metaMlEnabled() && metaMlAutopilotEnabled()) {
    try {
      await maybeAutoTrainAndApplyMetaMl({ silent: true });
    } catch {}
  }
});

$("metaSavedSelect")?.addEventListener("change", () => {
  syncMetaSavedSelectionUi();
});

$("metaSavedSearch")?.addEventListener("input", () => {
  renderMetaSavedSelect();
  saveSettingsSoon();
});

$("btnSaveMetaSavedTags")?.addEventListener("click", async () => {
  const path = String($("metaSavedSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved meta run first.", "muted");
    return;
  }
  try {
    const tags = parseTagsInput($("metaSavedTags")?.value || "");
    await saveMetaSavedTags(path, tags);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRefreshMetaSaved")?.addEventListener("click", async () => {
  await refreshMetaSaved();
  setStatus("Refreshed saved meta runs.", "muted");
});

$("btnLoadMetaSaved")?.addEventListener("click", async () => {
  const path = String($("metaSavedSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved meta run first.", "muted");
    return;
  }
  try {
    await loadMetaSaved(path);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnOpenMetaSavedReport")?.addEventListener("click", async () => {
  try {
    await openSelectedMetaSavedReport();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRevealMetaSaved")?.addEventListener("click", async () => {
  try {
    await revealSelectedMetaSaved();
    setStatus("Revealed meta run.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnDeleteMetaSaved")?.addEventListener("click", async () => {
  const entry = selectedMetaSavedEntry();
  if (!entry) {
    setStatus("Select a saved meta run first.", "muted");
    return;
  }
  if (!confirm("Delete this saved meta artifact file?")) return;
  try {
    await deleteSelectedMetaSaved();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("stage4SavedSelect")?.addEventListener("change", () => {
  syncStage4SavedSelectionUi();
});

$("stage4SavedSearch")?.addEventListener("input", () => {
  renderStage4SavedSelect();
  saveSettingsSoon();
});

$("btnSaveStage4SavedTags")?.addEventListener("click", async () => {
  const path = String($("stage4SavedSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved Stage 4 run first.", "muted");
    return;
  }
  try {
    const tags = parseTagsInput($("stage4SavedTags")?.value || "");
    await saveStage4SavedTags(path, tags);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRefreshStage4Saved")?.addEventListener("click", async () => {
  await refreshStage4Saved();
  setStatus("Refreshed saved Stage 4 runs.", "muted");
});

$("btnLoadStage4Saved")?.addEventListener("click", async () => {
  const path = String($("stage4SavedSelect")?.value || "").trim();
  if (!path) {
    setStatus("Select a saved Stage 4 run first.", "muted");
    return;
  }
  try {
    await loadStage4Saved(path);
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnRevealStage4Saved")?.addEventListener("click", async () => {
  try {
    await revealSelectedStage4Saved();
    setStatus("Revealed Stage 4 run.", "ok");
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("btnDeleteStage4Saved")?.addEventListener("click", async () => {
  const entry = selectedStage4SavedEntry();
  if (!entry) {
    setStatus("Select a saved Stage 4 run first.", "muted");
    return;
  }
  if (!confirm("Delete this saved Stage 4 artifact file?")) return;
  try {
    await deleteSelectedStage4Saved();
  } catch (e) {
    setStatus(String(e), "danger");
  }
});

$("metaCompareSelect")?.addEventListener("change", () => {
  renderMetaCompare();
});

$("compareSelect")?.addEventListener("change", () => {
  renderCompare();
  saveSettingsSoon();
});

$("btnSetBaseline")?.addEventListener("click", () => {
  if (!CURRENT_RUN?.ts) {
    setStatus("Run an optimization or select a run from history first.", "muted");
    return;
  }
  const sel = $("compareSelect");
  if (!sel) return;
  if (CURRENT_RUN?.artifact_path) sel.value = `disk:${String(CURRENT_RUN.artifact_path || "").trim()}`;
  else sel.value = `hist:${CURRENT_RUN.ts}`;
  saveSettingsSoon();
  renderCompare();
});

$("btnClearBaseline")?.addEventListener("click", () => {
  const sel = $("compareSelect");
  if (!sel) return;
  sel.value = "";
  saveSettingsSoon();
  renderCompare();
});

initThemeUi();
initWindowEffectUi();
initUiMotionPrefs();
initSettingsWorkspaceUi();

CONTROLS_TABS = initTabSystem({
  barId: "controlsTabBar",
  panelsId: "controlsTabPanels",
  storageKey: CONTROLS_TAB_KEY,
  defaultTab: "data",
});
RESULTS_TABS = initTabSystem({
  barId: "resultsTabBar",
  panelsId: "resultsTabPanels",
  storageKey: RESULTS_TAB_KEY,
  defaultTab: "overview",
});

initWorkflowNav();
initWorkflowAdvancedToggle();
initAggregateTabUi();
initWorkspaceNav();

initControlsSplitter();

initOverviewEmptyUi();
updateOverviewEmptyUi();
initBestCardUi();
initQuickBarUi();
initCmdPaletteUi();
initGlobalShortcuts();

const SETTINGS_LOADED = loadSettings();
if (!SETTINGS_LOADED) {
  applyPresetProfile($("presetProfile")?.value || "Balanced", { silent: true });
}
syncCostUi({ save: false });
updatePolygonAdjustedUi();
if ($("moSearchSpace") && !String($("moSearchSpace").value || "").trim()) {
  $("moSearchSpace").value = JSON.stringify(defaultCompositeMoSearchSpace(), null, 2);
}
renderRecentCsvDatalist(getRecentCsvPaths());
renderDataMeta(null);
applyMaSourceConstraints();
const hist0 = getHistory();
populateHistorySelect(hist0, "");
populateCompareSelect(hist0, $("compareSelect")?.value || "");
updateAggregateSummaryCards();
refreshSavedRuns();
refreshMetaSaved();
refreshStage4Saved();
updateDeviceIdEnabled();
updateStrategyUi();
updateExecParamSourceUi();
renderExecOrdersTable(null);
renderExecPositionMeta(null);
clearExecTvChart("Load a dataset and refresh orders to overlay broker events.");
setExecLiveStatus("Live candles: off", "muted");
setExecRunnerStatus("Auto-runner: off", "muted");

initMaSelects();
initProgressListener();
initExecLiveStreamListeners();
initDistributedProgressListener();
initSampleCsvs();
initMarketDataDirUi();
initPolygonUi();
initCrossTickerList();
initBatchDatasetList();
initDistributedUi();
loadLastBatchSummaryFromDisk();
refreshExecLiveCandleStreamStatusUi({ silent: false });
refreshExecRunnerStatusUi({ silent: false });
setTimeout(() => {
  restoreLastDatasetOnStartup().catch((e) => {
    setStatus(`Startup restore skipped: ${String(e)}`, "muted");
  });
}, 0);
