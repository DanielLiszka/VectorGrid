import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "./runtimePinia";
import * as dataService from "../services/dataService";
import * as settingsService from "../services/settingsService";
import { useDataStore } from "./dataStore";
import { useSettingsStore } from "./settingsStore";
import { useWorkflowStore } from "./workflowStore";

vi.mock("../services/dataService", () => ({
  browseMarketDataDir: vi.fn(async () => ""),
  getPolygonCacheInfo: vi.fn(async () => ({
    cacheDir: "C:/polygon",
    hasApiKey: false,
    datasets: [],
  })),
  openMarketDataDir: vi.fn(async () => ({
    path: "C:/default/polygon-cache",
    openedWith: "open_path",
  })),
  openPolygon: vi.fn(async () => undefined),
  resetMarketDataDir: vi.fn(async () => ({
    defaultRootDir: "C:/default",
    configuredRootDir: "",
    effectiveRootDir: "C:/default",
    polygonCacheDir: "C:/default/polygon-cache",
    hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
  })),
  setMarketDataDir: vi.fn(async () => ({
    defaultRootDir: "C:/default",
    configuredRootDir: "C:/custom",
    effectiveRootDir: "C:/custom",
    polygonCacheDir: "C:/custom/polygon-cache",
    hint: "Using custom root: C:/custom · Polygon cache: C:/custom/polygon-cache",
  })),
}));

vi.mock("../services/settingsService", () => ({
  ARTIFACT_RETENTION_FAMILY_ORDER: [
    "run",
    "meta",
    "stage5",
    "mlPolicyEval",
    "batchSummary",
    "campaignSummary",
    "optionsExpression",
  ],
  clearArtifactRetentionSettings: vi.fn(async () => undefined),
  getArtifactRetentionInfo: vi.fn(async () => ({
    defaults: {
      run: 200,
      meta: 100,
      stage5: 100,
      mlPolicyEval: 100,
      batchSummary: 100,
      campaignSummary: 100,
      optionsExpression: 100,
    },
    configured: {
      run: null,
      meta: null,
      stage5: null,
      mlPolicyEval: null,
      batchSummary: null,
      campaignSummary: null,
      optionsExpression: null,
    },
    envOverrides: {
      run: null,
      meta: null,
      stage5: null,
      mlPolicyEval: null,
      batchSummary: null,
      campaignSummary: null,
      optionsExpression: null,
    },
    effective: {
      run: 200,
      meta: 100,
      stage5: 100,
      mlPolicyEval: 100,
      batchSummary: 100,
      campaignSummary: 100,
      optionsExpression: 100,
    },
  })),
  openReportsDir: vi.fn(async () => undefined),
  openRunsDir: vi.fn(async () => undefined),
  setArtifactRetentionSettings: vi.fn(async () => undefined),
  setWindowEffect: vi.fn(async () => undefined),
}));

const browseMarketDataDirMock = vi.mocked(dataService.browseMarketDataDir);
const openMarketDataDirMock = vi.mocked(dataService.openMarketDataDir);
const openPolygonMock = vi.mocked(dataService.openPolygon);
const resetMarketDataDirMock = vi.mocked(dataService.resetMarketDataDir);
const setMarketDataDirMock = vi.mocked(dataService.setMarketDataDir);

const clearArtifactRetentionSettingsMock = vi.mocked(
  settingsService.clearArtifactRetentionSettings,
);
const getArtifactRetentionInfoMock = vi.mocked(
  settingsService.getArtifactRetentionInfo,
);
const openReportsDirMock = vi.mocked(settingsService.openReportsDir);
const openRunsDirMock = vi.mocked(settingsService.openRunsDir);
const setArtifactRetentionSettingsMock = vi.mocked(
  settingsService.setArtifactRetentionSettings,
);
const setWindowEffectMock = vi.mocked(settingsService.setWindowEffect);

interface LegacyField {
  value?: string;
  checked?: boolean;
  type?: string;
  dispatchEvent?: (event: Event) => boolean;
}

interface DocumentHarness {
  dataset: Record<string, string>;
  fields: Record<string, LegacyField>;
}

function installDocumentHarness(): DocumentHarness {
  const dataset: Record<string, string> = {};
  const fields: Record<string, LegacyField> = {
    marketDataDir: {
      value: "",
      dispatchEvent: () => true,
    },
    marketDataMoveExisting: {
      type: "checkbox",
      checked: true,
      dispatchEvent: () => true,
    },
  };
  vi.stubGlobal("document", {
    documentElement: { dataset },
    getElementById: (id: string) => fields[id] ?? null,
  });
  return { dataset, fields };
}

const defaultMarketDataDirInfo = {
  defaultRootDir: "C:/default",
  configuredRootDir: "",
  effectiveRootDir: "C:/default",
  polygonCacheDir: "C:/default/polygon-cache",
  hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
};

const defaultArtifactRetentionInfo = {
  defaults: {
    run: 200,
    meta: 100,
    stage5: 100,
    mlPolicyEval: 100,
    batchSummary: 100,
    campaignSummary: 100,
    optionsExpression: 100,
  },
  configured: {
    run: null,
    meta: null,
    stage5: null,
    mlPolicyEval: null,
    batchSummary: null,
    campaignSummary: null,
    optionsExpression: null,
  },
  envOverrides: {
    run: null,
    meta: null,
    stage5: null,
    mlPolicyEval: null,
    batchSummary: null,
    campaignSummary: null,
    optionsExpression: null,
  },
  effective: {
    run: 200,
    meta: 100,
    stage5: 100,
    mlPolicyEval: 100,
    batchSummary: 100,
    campaignSummary: 100,
    optionsExpression: 100,
  },
};

describe("settings store actions", () => {
  let harness: DocumentHarness;
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    vi.stubGlobal("navigator", {
      userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
    });
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    vi.clearAllMocks();
    browseMarketDataDirMock.mockResolvedValue("");
    setMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo);
    resetMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo);
    openMarketDataDirMock.mockResolvedValue({
      path: "C:/default/polygon-cache",
      openedWith: "open_path",
    });
    getArtifactRetentionInfoMock.mockResolvedValue(
      defaultArtifactRetentionInfo,
    );
    setArtifactRetentionSettingsMock.mockResolvedValue(
      undefined as never,
    );
    clearArtifactRetentionSettingsMock.mockResolvedValue(
      undefined as never,
    );
    harness = installDocumentHarness();
  });

  it("applies settings field updates without legacy settings action dispatch", () => {
    const store = useSettingsStore();
    store.setInputField("settingsMarketDataDir", " C:/cache ");
    store.setSelectField("settingsThemeSelect", "dark");
    store.setSelectField("settingsEffectSelect", "mica");
    store.setSelectField("settingsHelpMode", "off");
    store.setSelectField("settingsDensityMode", "dense");
    store.setSelectField("settingsDesktopNotifications", "important");
    store.setCheckboxField("settingsWorkflowAdvanced", true);
    store.setCheckboxField("settingsControlsCollapsed", true);
    store.setCheckboxField("settingsMoveExisting", false);

    expect(store.marketDataDir).toBe("C:/cache");
    expect(harness.fields.marketDataDir?.value).toBe("");
    expect(store.theme).toBe("dark");
    expect(harness.dataset.themePref).toBe("dark");
    expect(harness.dataset.windowEffect).toBe("mica");
    expect(store.helpMode).toBe("off");
    expect(harness.dataset.helpMode).toBe("off");
    expect(store.densityMode).toBe("dense");
    expect(harness.dataset.density).toBe("dense");
    expect(store.desktopNotificationMode).toBe("important");
    expect(store.workflowAdvanced).toBe(true);
    expect(harness.dataset.workflowAdvanced).toBe("1");
    expect(store.controlsCollapsed).toBe(true);
    expect(harness.dataset.controlsCollapsed).toBe("1");
    expect(store.moveExisting).toBe(false);
    expect(harness.fields.marketDataMoveExisting?.checked).toBe(true);
    expect(setWindowEffectMock).toHaveBeenCalledWith("mica", true);

    store.setSelectField("settingsEffectSelect", "none");

    expect(store.effect).toBe("none");
    expect(harness.dataset.windowEffect).toBeUndefined();
    expect(setWindowEffectMock).toHaveBeenLastCalledWith("none", true);
  });

  it("treats unchanged settings selections and checkboxes as no-ops", () => {
    const store = useSettingsStore();
    store.setSelectField("settingsThemeSelect", "dark");
    store.setSelectField("settingsEffectSelect", "mica");
    store.setSelectField("settingsHelpMode", "concise");
    store.setSelectField("settingsDensityMode", "dense");
    store.setSelectField("settingsDesktopNotifications", "important");
    store.setCheckboxField("settingsWorkflowAdvanced", true);
    store.setCheckboxField("settingsControlsCollapsed", true);
    store.setCheckboxField("settingsMoveExisting", false);

    const updatedAt = store.updatedAtUnixMs;
    setWindowEffectMock.mockClear();

    store.setSelectField("settingsThemeSelect", "dark");
    store.setSelectField("settingsEffectSelect", "mica");
    store.setSelectField("settingsHelpMode", "concise");
    store.setSelectField("settingsDensityMode", "dense");
    store.setSelectField("settingsDesktopNotifications", "important");
    store.setCheckboxField("settingsWorkflowAdvanced", true);
    store.setCheckboxField("settingsControlsCollapsed", true);
    store.setCheckboxField("settingsMoveExisting", false);

    expect(store.updatedAtUnixMs).toBe(updatedAt);
    expect(setWindowEffectMock).not.toHaveBeenCalled();
    expect(store.theme).toBe("dark");
    expect(store.effect).toBe("mica");
    expect(store.helpMode).toBe("concise");
    expect(store.densityMode).toBe("dense");
    expect(store.desktopNotificationMode).toBe("important");
    expect(store.workflowAdvanced).toBe(true);
    expect(store.controlsCollapsed).toBe(true);
    expect(store.moveExisting).toBe(false);
  });

  it("dispatches market-data directory actions via data service", () => {
    const store = useSettingsStore();
    store.browseMarketDataDir();
    store.setMarketDataDir(" C:/md ", true);
    store.resetMarketDataDir();
    store.openMarketDataDir();

    expect(browseMarketDataDirMock).toHaveBeenCalled();
    expect(setMarketDataDirMock).toHaveBeenCalledWith("C:/md", true);
    expect(resetMarketDataDirMock).toHaveBeenCalled();
    expect(openMarketDataDirMock).toHaveBeenCalled();
  });

  it("dispatches utility and navigation actions", () => {
    const store = useSettingsStore();
    const workflowStore = useWorkflowStore();

    store.openWorkflowData();
    store.openRunsDir();
    store.openReportsDir();
    store.openPolygonModal();
    expect(workflowStore.workspace).toBe("workflow");
    expect(workflowStore.workflow).toBe("data");

    store.gotoLibrary();
    expect(workflowStore.workspace).toBe("library");
    expect(workflowStore.workflow).toBe("history");

    store.gotoAggregate();
    expect(workflowStore.workspace).toBe("aggregate");
    expect(workflowStore.workflow).toBe("settings");

    expect(openPolygonMock).toHaveBeenCalled();
    expect(openRunsDirMock).toHaveBeenCalled();
    expect(openReportsDirMock).toHaveBeenCalled();
  });

  it("loads backend-owned artifact-retention info without using local bootstrap state", async () => {
    const settingsStore = useSettingsStore();

    await settingsStore.loadArtifactRetentionInfo();

    expect(getArtifactRetentionInfoMock).toHaveBeenCalledTimes(1);
    expect(settingsStore.artifactRetentionLoaded).toBe(true);
    expect(settingsStore.artifactRetentionLoading).toBe(false);
    expect(settingsStore.artifactRetentionInfo.configured.run).toBeNull();
    expect(settingsStore.artifactRetentionInfo.effective.meta).toBe(100);
    expect(settingsStore.artifactRetentionStatusLine).toContain(
      "Loaded artifact retention settings",
    );
  });

  it("saves artifact-retention settings through the backend service response", async () => {
    const settingsStore = useSettingsStore();
    await settingsStore.loadArtifactRetentionInfo();

    setArtifactRetentionSettingsMock.mockResolvedValueOnce({
      ...defaultArtifactRetentionInfo,
      configured: {
        ...defaultArtifactRetentionInfo.configured,
        run: 80,
        stage5: 55,
      },
      envOverrides: {
        ...defaultArtifactRetentionInfo.envOverrides,
        meta: 12,
      },
      effective: {
        ...defaultArtifactRetentionInfo.effective,
        run: 80,
        meta: 12,
        stage5: 55,
      },
    });

    await settingsStore.saveArtifactRetentionSettings({
      run: 80,
      meta: null,
      stage5: 55,
      mlPolicyEval: null,
      batchSummary: null,
      campaignSummary: null,
      optionsExpression: null,
    });

    expect(setArtifactRetentionSettingsMock).toHaveBeenCalledWith({
      run: 80,
      meta: null,
      stage5: 55,
      mlPolicyEval: null,
      batchSummary: null,
      campaignSummary: null,
      optionsExpression: null,
    });
    expect(getArtifactRetentionInfoMock).toHaveBeenCalledTimes(1);
    expect(settingsStore.artifactRetentionInfo.configured.run).toBe(80);
    expect(settingsStore.artifactRetentionInfo.effective.meta).toBe(12);
    expect(settingsStore.artifactRetentionStatusLine).toContain(
      "Saved artifact retention settings",
    );
  });

  it("clears persisted artifact-retention settings and restores defaults from the backend response", async () => {
    const settingsStore = useSettingsStore();
    settingsStore.applyArtifactRetentionInfo({
      ...defaultArtifactRetentionInfo,
      configured: {
        ...defaultArtifactRetentionInfo.configured,
        run: 90,
        batchSummary: 11,
      },
      effective: {
        ...defaultArtifactRetentionInfo.effective,
        run: 90,
        batchSummary: 11,
      },
    });

    clearArtifactRetentionSettingsMock.mockResolvedValueOnce(
      defaultArtifactRetentionInfo,
    );

    await settingsStore.clearArtifactRetentionSettings();

    expect(clearArtifactRetentionSettingsMock).toHaveBeenCalledTimes(1);
    expect(getArtifactRetentionInfoMock).toHaveBeenCalledTimes(0);
    expect(settingsStore.artifactRetentionInfo.configured.run).toBeNull();
    expect(settingsStore.artifactRetentionInfo.effective.run).toBe(200);
    expect(settingsStore.artifactRetentionStatusLine).toContain(
      "Reset artifact retention settings to defaults",
    );
  });

  it("creates, applies, duplicates, renames, and deletes custom perspectives", () => {
    const settingsStore = useSettingsStore();
    const workflowStore = useWorkflowStore();

    workflowStore.selectWorkflowStep("analyze");
    workflowStore.selectResultsTab("top");
    workflowStore.setSplitFraction(0.41, { persist: false });
    settingsStore.setSelectField("settingsDensityMode", "dense");

    settingsStore.createPerspective("Review focus");
    const created = settingsStore.perspectives.find(
      (entry) => entry.name === "Review focus",
    );
    expect(created).toBeTruthy();
    expect(created?.snapshot.resultsTab).toBe("top");
    expect(created?.snapshot.splitFraction).toBeCloseTo(0.41, 5);
    expect(created?.snapshot.densityMode).toBe("dense");

    workflowStore.selectWorkspace("library");
    settingsStore.setSelectField("settingsDensityMode", "comfortable");
    settingsStore.applyPerspective(String(created?.id || ""));

    expect(workflowStore.workspace).toBe("workflow");
    expect(workflowStore.resultsTab).toBe("top");
    expect(workflowStore.splitFraction).toBeCloseTo(0.41, 5);
    expect(settingsStore.densityMode).toBe("dense");

    settingsStore.duplicatePerspective(String(created?.id || ""));
    const duplicate = settingsStore.perspectives.find(
      (entry) => entry.name === "Review focus Copy",
    );
    expect(duplicate).toBeTruthy();

    settingsStore.renamePerspective(
      String(duplicate?.id || ""),
      "Review focus alt",
    );
    expect(
      settingsStore.perspectives.some(
        (entry) => entry.name === "Review focus alt",
      ),
    ).toBe(true);

    settingsStore.deletePerspective(String(duplicate?.id || ""));
    expect(
      settingsStore.perspectives.some((entry) => entry.id === duplicate?.id),
    ).toBe(false);
  });

  it("protects built-in perspectives from rename and delete while allowing overwrite in place", () => {
    const settingsStore = useSettingsStore();
    const workflowStore = useWorkflowStore();
    const builtinBefore = settingsStore.perspectives.find(
      (entry) => entry.id === "builtin_optimize",
    );

    expect(builtinBefore).toBeTruthy();
    expect(builtinBefore?.builtIn).toBe(true);

    settingsStore.renamePerspective("builtin_optimize", "Renamed builtin");
    settingsStore.deletePerspective("builtin_optimize");

    const builtinProtected = settingsStore.perspectives.find(
      (entry) => entry.id === "builtin_optimize",
    );
    expect(builtinProtected?.name).toBe(builtinBefore?.name);
    expect(
      settingsStore.perspectives.some(
        (entry) => entry.id === "builtin_optimize",
      ),
    ).toBe(true);

    workflowStore.selectWorkflowStep("analyze", { persist: false });
    workflowStore.selectResultsTab("top");
    workflowStore.setSplitFraction(0.41, { persist: false });
    settingsStore.setSelectField("settingsDensityMode", "dense");

    settingsStore.overwritePerspective("builtin_optimize");

    const builtinAfter = settingsStore.perspectives.find(
      (entry) => entry.id === "builtin_optimize",
    );
    expect(builtinAfter?.name).toBe(builtinBefore?.name);
    expect(builtinAfter?.builtIn).toBe(true);
    expect(builtinAfter?.snapshot.workflowStep).toBe("analyze");
    expect(builtinAfter?.snapshot.resultsTab).toBe("top");
    expect(builtinAfter?.snapshot.splitFraction).toBeCloseTo(0.41, 5);
    expect(builtinAfter?.snapshot.densityMode).toBe("dense");
    expect(settingsStore.statusLine).toContain("Updated perspective:");
  });

  it("overwrites an existing perspective snapshot and preserves built-in protection rules", () => {
    const settingsStore = useSettingsStore();
    const workflowStore = useWorkflowStore();

    settingsStore.selectPerspective("builtin_optimize");
    workflowStore.selectWorkflowStep("analyze");
    workflowStore.selectResultsTab("pareto");
    workflowStore.setSplitFraction(0.52, { persist: false });
    settingsStore.setSelectField("settingsDensityMode", "dense");

    settingsStore.overwritePerspective("builtin_optimize");

    const overwritten = settingsStore.perspectives.find(
      (entry) => entry.id === "builtin_optimize",
    );
    expect(overwritten?.snapshot.workspace).toBe("workflow");
    expect(overwritten?.snapshot.workflowStep).toBe("analyze");
    expect(overwritten?.snapshot.resultsTab).toBe("pareto");
    expect(overwritten?.snapshot.splitFraction).toBeCloseTo(0.52, 5);
    expect(overwritten?.snapshot.densityMode).toBe("dense");
    expect(settingsStore.statusLine).toContain(
      "Updated perspective: Optimize.",
    );

    const beforeRename = overwritten?.name;
    const beforeDeleteCount = settingsStore.perspectives.length;
    settingsStore.renamePerspective("builtin_optimize", "Should Not Rename");
    settingsStore.deletePerspective("builtin_optimize");

    const builtinAfterProtection = settingsStore.perspectives.find(
      (entry) => entry.id === "builtin_optimize",
    );
    expect(builtinAfterProtection?.name).toBe(beforeRename);
    expect(settingsStore.perspectives).toHaveLength(beforeDeleteCount);
  });

  it("syncs market-data fields from data store and applies runtime mutability flags", () => {
    const settingsStore = useSettingsStore();
    const dataStore = useDataStore();

    dataStore.applyLegacySnapshot({
      marketDataDir: "C:/market-root",
      marketDataMoveExisting: false,
      marketDataDirHint: "Using custom root: C:/market-root",
      canBrowseMarketDataDir: true,
      canSetMarketDataDir: true,
      canResetMarketDataDir: true,
      canOpenMarketDataDir: true,
    });

    harness.dataset.running = "1";
    settingsStore.syncFromDataStore();
    expect(settingsStore.marketDataDir).toBe("C:/market-root");
    expect(settingsStore.moveExisting).toBe(false);
    expect(settingsStore.marketDataDirHint).toBe(
      "Using custom root: C:/market-root",
    );
    expect(settingsStore.canBrowseMarketDataDir).toBe(false);
    expect(settingsStore.canOpenRunsDir).toBe(false);
    expect(settingsStore.canOpenReportsDir).toBe(false);

    harness.dataset.running = "";
    settingsStore.syncFromDataStore();
    expect(settingsStore.canBrowseMarketDataDir).toBe(true);
    expect(settingsStore.canSetMarketDataDir).toBe(true);
    expect(settingsStore.canResetMarketDataDir).toBe(true);
    expect(settingsStore.canOpenMarketDataDir).toBe(true);
    expect(settingsStore.canOpenRunsDir).toBe(true);
    expect(settingsStore.canOpenReportsDir).toBe(true);
  });
});
