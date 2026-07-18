import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { bindRuntimePinia } from "./runtimePinia";
import {
  applyTimeframe,
  browseCsv,
  clearPolygonApiKey,
  type DataLoadResult,
  type DataSampleCsvSummary,
  getMarketDataDirInfo,
  getPolygonCacheInfo,
  type MarketDataDirInfoSummary,
  type MarketDataDirOpenResult,
  type PolygonCacheInfoSummary,
  type PolygonCredentialStatusSummary,
  type PolygonDatasetSummary,
  browseMarketDataDir,
  clearData,
  closePolygon,
  downloadPolygonDataset,
  listSampleCsvs,
  loadCsv,
  loadSampleCsv,
  openMarketDataDir,
  openPolygon,
  repairPolygonDataset,
  resetMarketDataDir,
  setPolygonApiKey,
  setMarketDataDir,
  topOffPolygonDataset,
  validatePolygonDataset,
} from "../services/dataService";
import { DATA_STARTUP_SNAPSHOT_KEY } from "../utils/core/storageKeys";
import { readStorageJson, writeStorageJson } from "../utils/storage";
import { TauriRuntimeError } from "../tauri/runtimeError";
import { useDataStore } from "./dataStore";
import { useWorkflowArtifactsStore } from "./workflowArtifactsStore";

function installStorageHarness(
  seed: Record<string, string> = {},
): Map<string, string> {
  const state = new Map<string, string>(Object.entries(seed));
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => (state.has(key) ? String(state.get(key)) : null),
    setItem: (key: string, value: string) => {
      state.set(String(key), String(value));
    },
    removeItem: (key: string) => {
      state.delete(String(key));
    },
    clear: () => {
      state.clear();
    },
  });
  return state;
}

vi.mock("../services/dataService", () => ({
  applyTimeframe: vi.fn(async () => ({
    path: "",
    dataId: "data-1@15m",
    cacheHit: false,
    cachePath: "",
    loadSource: "derived_fresh_resample",
    meta: {
      len: 100,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000,
      maxTs: 1_700_100_000,
      timeframe: "15m",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    },
  })),
  browseCsv: vi.fn(async () => null),
  browseMarketDataDir: vi.fn(async () => ""),
  clearPolygonApiKey: vi.fn(async () => ({
    cacheDir: "C:/default/polygon-cache",
    hasApiKey: false,
    credentialStatus: {
      hasApiKey: false,
      state: "missing",
      storageBackend: "missing",
      degraded: false,
      message: "",
    },
    datasets: [],
  })),
  clearData: vi.fn(async () => undefined),
  closePolygon: vi.fn(async () => undefined),
  downloadPolygonDataset: vi.fn(async () => ({
    csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
    downloadedBars: 1500,
    usedCacheOnly: false,
    load: {
      path: "C:/default/polygon-cache/AAPL_1m.csv",
      dataId: "poly-1",
      cacheHit: false,
      cachePath: "C:/cache/poly-1.bin",
      loadSource: "derived_fresh_resample",
      meta: {
        len: 1500,
        hasOpen: true,
        hasHigh: true,
        hasLow: true,
        hasClose: true,
        hasVolume: true,
        minTs: 1_700_000_000,
        maxTs: 1_700_500_000,
        timeframe: "1m",
        sourceSizeBytes: 123,
        sourceMtimeUtcMs: 456,
      },
    },
  })),
  getMarketDataDirInfo: vi.fn(async () => ({
    defaultRootDir: "C:/default",
    configuredRootDir: "",
    effectiveRootDir: "C:/default",
    polygonCacheDir: "C:/default/polygon-cache",
    hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
  })),
  getPolygonCacheInfo: vi.fn(async () => ({
    cacheDir: "C:/default/polygon-cache",
    hasApiKey: true,
    credentialStatus: {
      hasApiKey: true,
      state: "keyring",
      storageBackend: "keyring",
      degraded: false,
      message: "",
    },
    datasets: [
      {
        market: "stocks",
        displaySymbol: "AAPL",
        polygonTicker: "AAPL",
        timeframe: "1m",
        csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
        minTs: 1700000000,
        maxTs: 1700005000,
        lastUpdatedUtcMs: 1700005000000,
        rowCount: 1500,
        adjusted: true,
        integrity: null,
      },
    ],
  })),
  listSampleCsvs: vi.fn(async () => []),
  loadCsv: vi.fn(async () => ({
    path: "C:/tmp/aapl.csv",
    dataId: "data-1",
    cacheHit: false,
    cachePath: "C:/cache/data-1.bin",
    loadSource: "csv_parsed_and_cached",
    meta: {
      len: 500,
      hasOpen: true,
      hasHigh: true,
      hasLow: true,
      hasClose: true,
      hasVolume: true,
      minTs: 1_700_000_000,
      maxTs: 1_700_500_000,
      timeframe: "1m",
      sourceSizeBytes: 123,
      sourceMtimeUtcMs: 456,
    },
  })),
  loadSampleCsv: vi.fn(async () => null),
  openMarketDataDir: vi.fn(async () => ({
    path: "C:/default/polygon-cache",
    openedWith: "open_path",
  })),
  openPolygon: vi.fn(async () => undefined),
  repairPolygonDataset: vi.fn(async () => ({
    market: "stocks",
    displaySymbol: "AAPL",
    polygonTicker: "AAPL",
    timeframe: "1m",
    csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
    minTs: 1700000000,
    maxTs: 1700005000,
    lastUpdatedUtcMs: 1700005000000,
    rowCount: 1500,
    adjusted: true,
    integrity: {
      checkedUtcMs: 1700005000000,
      parseErrors: 0,
      duplicates: 0,
      outOfOrder: 0,
      sameDayGapsGt2Min: 0,
    },
  })),
  topOffPolygonDataset: vi.fn(async () => ({
    csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
    downloadedBars: 12,
    usedCacheOnly: false,
    baseLoad: {
      path: "C:/default/polygon-cache/AAPL_1m.csv",
      dataId: "poly-topoff-base",
      cacheHit: false,
      cachePath: "C:/cache/poly-topoff-base.bin",
      loadSource: "csv_parsed_and_cached",
      meta: {
        len: 1512,
        hasOpen: true,
        hasHigh: true,
        hasLow: true,
        hasClose: true,
        hasVolume: true,
        minTs: 1_700_000_000,
        maxTs: 1_700_600_000,
        timeframe: "1m",
        sourceSizeBytes: 123,
        sourceMtimeUtcMs: 789,
      },
    },
    load: {
      path: "C:/default/polygon-cache/AAPL_1m.csv",
      dataId: "poly-topoff",
      cacheHit: false,
      cachePath: "C:/cache/poly-topoff.bin",
      loadSource: "derived_fresh_resample",
      meta: {
        len: 101,
        hasOpen: true,
        hasHigh: true,
        hasLow: true,
        hasClose: true,
        hasVolume: true,
        minTs: 1_700_000_000,
        maxTs: 1_700_600_000,
        timeframe: "15m",
        sourceSizeBytes: 123,
        sourceMtimeUtcMs: 789,
      },
    },
  })),
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
  setPolygonApiKey: vi.fn(async () => ({
    cacheDir: "C:/default/polygon-cache",
    hasApiKey: true,
    credentialStatus: {
      hasApiKey: true,
      state: "keyring",
      storageBackend: "keyring",
      degraded: false,
      message: "",
    },
    datasets: [],
  })),
  validatePolygonDataset: vi.fn(async () => ({
    market: "stocks",
    displaySymbol: "AAPL",
    polygonTicker: "AAPL",
    timeframe: "1m",
    csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
    minTs: 1700000000,
    maxTs: 1700005000,
    lastUpdatedUtcMs: 1700005000000,
    rowCount: 1500,
    adjusted: true,
    integrity: {
      checkedUtcMs: 1700005000000,
      parseErrors: 0,
      duplicates: 1,
      outOfOrder: 0,
      sameDayGapsGt2Min: 0,
    },
  })),
}));

const applyTimeframeMock = vi.mocked(applyTimeframe);
const clearDataMock = vi.mocked(clearData);
const loadCsvMock = vi.mocked(loadCsv);
const loadSampleCsvMock = vi.mocked(loadSampleCsv);
const browseCsvMock = vi.mocked(browseCsv);
const openPolygonMock = vi.mocked(openPolygon);
const closePolygonMock = vi.mocked(closePolygon);
const getMarketDataDirInfoMock = vi.mocked(getMarketDataDirInfo);
const listSampleCsvsMock = vi.mocked(listSampleCsvs);
const browseMarketDataDirMock = vi.mocked(browseMarketDataDir);
const clearPolygonApiKeyMock = vi.mocked(clearPolygonApiKey);
const setMarketDataDirMock = vi.mocked(setMarketDataDir);
const resetMarketDataDirMock = vi.mocked(resetMarketDataDir);
const openMarketDataDirMock = vi.mocked(openMarketDataDir);
const getPolygonCacheInfoMock = vi.mocked(getPolygonCacheInfo);
const setPolygonApiKeyMock = vi.mocked(setPolygonApiKey);
const downloadPolygonDatasetMock = vi.mocked(downloadPolygonDataset);
const topOffPolygonDatasetMock = vi.mocked(topOffPolygonDataset);
const validatePolygonDatasetMock = vi.mocked(validatePolygonDataset);
const repairPolygonDatasetMock = vi.mocked(repairPolygonDataset);

const defaultMarketDataDirInfo: MarketDataDirInfoSummary = {
  defaultRootDir: "C:/default",
  configuredRootDir: "",
  effectiveRootDir: "C:/default",
  polygonCacheDir: "C:/default/polygon-cache",
  hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
};

const defaultMarketDataOpenResult: MarketDataDirOpenResult = {
  path: "C:/default/polygon-cache",
  openedWith: "open_path",
};

const defaultSampleRows: DataSampleCsvSummary[] = [
  { label: "aapl.csv", path: "C:/samples/aapl.csv", disabled: false },
  { label: "msft.csv", path: "C:/samples/msft.csv", disabled: false },
];

const defaultLoadResult: DataLoadResult = {
  path: "C:/tmp/aapl.csv",
  dataId: "data-1",
  cacheHit: false,
  cachePath: "C:/cache/data-1.bin",
  loadSource: "csv_parsed_and_cached",
  meta: {
    len: 500,
    hasOpen: true,
    hasHigh: true,
    hasLow: true,
    hasClose: true,
    hasVolume: true,
    minTs: 1_700_000_000,
    maxTs: 1_700_500_000,
    timeframe: "1m",
    sourceSizeBytes: 123,
    sourceMtimeUtcMs: 456,
  },
};

const defaultPolygonDataset: PolygonDatasetSummary = {
  market: "stocks",
  displaySymbol: "AAPL",
  polygonTicker: "AAPL",
  timeframe: "1m",
  csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
  minTs: 1_700_000_000,
  maxTs: 1_700_500_000,
  lastUpdatedUtcMs: 1_700_005_000_000,
  rowCount: 1500,
  adjusted: true,
  integrity: null,
};

const defaultPolygonCredentialStatus: PolygonCredentialStatusSummary = {
  hasApiKey: true,
  state: "keyring",
  storageBackend: "keyring",
  degraded: false,
  message: "",
};

const missingPolygonCredentialStatus: PolygonCredentialStatusSummary = {
  hasApiKey: false,
  state: "missing",
  storageBackend: "missing",
  degraded: false,
  message: "",
};

const defaultPolygonCacheInfo: PolygonCacheInfoSummary = {
  cacheDir: "C:/default/polygon-cache",
  hasApiKey: true,
  credentialStatus: defaultPolygonCredentialStatus,
  datasets: [{ ...defaultPolygonDataset }],
};

async function flushAsync(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

function createDeferred<T = void>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((nextResolve) => {
    resolve = nextResolve;
  });
  return { promise, resolve };
}

describe("data store actions", () => {
  let pinia: ReturnType<typeof createPinia>;

  beforeEach(() => {
    vi.unstubAllGlobals();
    installStorageHarness();
    localStorage.clear();
    pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);
    useDataStore(pinia).applyLegacySnapshot({});
    applyTimeframeMock.mockClear();
    clearDataMock.mockClear();
    loadCsvMock.mockClear();
    loadSampleCsvMock.mockClear();
    browseCsvMock.mockClear();
    openPolygonMock.mockClear();
    closePolygonMock.mockClear();
    getMarketDataDirInfoMock.mockClear();
    listSampleCsvsMock.mockClear();
    browseMarketDataDirMock.mockClear();
    clearPolygonApiKeyMock.mockClear();
    setMarketDataDirMock.mockClear();
    resetMarketDataDirMock.mockClear();
    openMarketDataDirMock.mockClear();
    getPolygonCacheInfoMock.mockClear();
    setPolygonApiKeyMock.mockClear();
    downloadPolygonDatasetMock.mockClear();
    topOffPolygonDatasetMock.mockClear();
    validatePolygonDatasetMock.mockClear();
    repairPolygonDatasetMock.mockClear();

    applyTimeframeMock.mockResolvedValue({
      ...defaultLoadResult,
      path: "",
      dataId: "data-1@15m",
      cacheHit: false,
      cachePath: "",
      meta: { ...defaultLoadResult.meta, len: 90, timeframe: "15m" },
    });
    clearDataMock.mockResolvedValue(undefined);
    loadCsvMock.mockResolvedValue({ ...defaultLoadResult });
    loadSampleCsvMock.mockResolvedValue({
      ...defaultLoadResult,
      path: "C:/samples/10k.csv",
    });
    browseCsvMock.mockResolvedValue({
      ...defaultLoadResult,
      path: "C:/picked.csv",
    });
    openPolygonMock.mockResolvedValue(undefined);
    closePolygonMock.mockResolvedValue(undefined);
    getMarketDataDirInfoMock.mockResolvedValue(defaultMarketDataDirInfo);
    listSampleCsvsMock.mockResolvedValue(defaultSampleRows);
    browseMarketDataDirMock.mockResolvedValue("");
    clearPolygonApiKeyMock.mockResolvedValue({
      ...defaultPolygonCacheInfo,
      hasApiKey: false,
      credentialStatus: missingPolygonCredentialStatus,
      datasets: [],
    });
    setMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo);
    resetMarketDataDirMock.mockResolvedValue(defaultMarketDataDirInfo);
    openMarketDataDirMock.mockResolvedValue(defaultMarketDataOpenResult);
    getPolygonCacheInfoMock.mockResolvedValue({ ...defaultPolygonCacheInfo });
    setPolygonApiKeyMock.mockResolvedValue({
      ...defaultPolygonCacheInfo,
      datasets: [],
    });
    downloadPolygonDatasetMock.mockResolvedValue({
      csvPath: defaultPolygonDataset.csvPath,
      downloadedBars: 1500,
      usedCacheOnly: false,
      baseLoad: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-1-base",
      },
      load: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-1",
        meta: { ...defaultLoadResult.meta, timeframe: "15m", len: 100 },
      },
    });
    topOffPolygonDatasetMock.mockResolvedValue({
      csvPath: defaultPolygonDataset.csvPath,
      downloadedBars: 12,
      usedCacheOnly: false,
      baseLoad: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-topoff-base",
      },
      load: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-topoff",
        meta: { ...defaultLoadResult.meta, timeframe: "15m", len: 101 },
      },
    });
    validatePolygonDatasetMock.mockResolvedValue({
      ...defaultPolygonDataset,
      integrity: {
        checkedUtcMs: 1_700_005_000_000,
        parseErrors: 0,
        duplicates: 1,
        outOfOrder: 0,
        sameDayGapsGt2Min: 0,
      },
    });
    repairPolygonDatasetMock.mockResolvedValue({
      ...defaultPolygonDataset,
      integrity: {
        checkedUtcMs: 1_700_005_000_000,
        parseErrors: 0,
        duplicates: 0,
        outOfOrder: 0,
        sameDayGapsGt2Min: 0,
      },
    });
  });

  it("updates Vue data state without legacy DOM mirroring", () => {
    const store = useDataStore();
    store.setInputField("csvPath", "C:/tmp/aapl.csv");
    store.setSelectField("dataTimeframe", "5m");
    store.setCheckboxField("autoRestoreDataset", true);

    expect(store.csvPath).toBe("C:/tmp/aapl.csv");
    expect(store.dataTimeframe).toBe("5m");
    expect(store.autoRestoreDataset).toBe(true);
    expect(store.source).toBe("vue");
  });

  it("loads CSV and applies Vue-owned dataset state", async () => {
    const store = useDataStore();

    store.loadCsv(" C:/tmp/msft.csv ");
    await flushAsync();

    expect(loadCsvMock).toHaveBeenCalledWith("C:/tmp/msft.csv");
    expect(store.baseDataId).toBe("data-1");
    expect(store.dataId).toBe("data-1");
    expect(store.datasetLabel).toBe("aapl.csv");
    expect(store.datasetContext).toContain("source=csv");
    expect(store.datasetContext).toContain("load=csv parsed + cached");
    expect(store.dataLen).toBe("500");
    expect(store.baseCsvPath).toBe("C:/tmp/aapl.csv");
    expect(store.baseSourceKind).toBe("csv");
    expect(store.baseSourceSizeBytes).toBe(123);
    expect(store.baseSourceMtimeUtcMs).toBe(456);
    expect(store.canApplyTimeframe).toBe(true);
    const snapshot = readStorageJson<Record<string, unknown>>(
      DATA_STARTUP_SNAPSHOT_KEY,
    );
    expect(snapshot?.baseCsvPath).toBe("C:/tmp/aapl.csv");
    expect(snapshot?.baseSourceSizeBytes).toBe(123);
    expect(snapshot?.baseSourceMtimeUtcMs).toBe(456);
  });

  it("applies timeframe using base data id and supports reset to base timeframe", async () => {
    const store = useDataStore();
    store.loadCsv("C:/tmp/aapl.csv");
    await flushAsync();

    store.applyTimeframe("15m");
    await flushAsync();

    expect(applyTimeframeMock).toHaveBeenCalledWith("15m", "data-1");
    expect(store.baseDataId).toBe("data-1");
    expect(store.dataId).toBe("data-1@15m");
    expect(store.dataTimeframe).toBe("15m");
    expect(store.datasetRange).toContain("15m");
    expect(store.dataTimeframeHint).toContain("no extra downloads");
    expect(store.baseCsvPath).toBe("C:/tmp/aapl.csv");
    expect(store.baseDataMeta?.timeframe).toBe("1m");
    expect(store.baseSourceSizeBytes).toBe(123);
    expect(store.baseSourceMtimeUtcMs).toBe(456);
    expect(
      readStorageJson<Record<string, unknown>>(DATA_STARTUP_SNAPSHOT_KEY)
        ?.selectedTimeframe,
    ).toBe("15m");

    store.applyTimeframe("");
    await flushAsync();

    expect(store.dataId).toBe("data-1");
    expect(store.dataTimeframe).toBe("");
    expect(store.datasetRange).toContain("1m");
    expect(store.baseDataId).toBe("data-1");
    expect(store.baseCsvPath).toBe("C:/tmp/aapl.csv");
    expect(store.dataTimeframeHint).toContain(
      "Choose a higher timeframe to derive candles",
    );
  });

  it("renders relative-timestamp datasets without epoch date placeholders", async () => {
    loadCsvMock.mockResolvedValueOnce({
      ...defaultLoadResult,
      path: "C:/tmp/benchmark.csv",
      dataId: "data-relative",
      meta: {
        ...defaultLoadResult.meta,
        minTs: 0,
        maxTs: 199_999_000,
        timeframe: "1s",
      },
    });

    const store = useDataStore();
    store.loadCsv("C:/tmp/benchmark.csv");
    await flushAsync();

    expect(store.datasetRange).toContain("1s");
    expect(store.datasetRange).toContain("relative 0ms");
    expect(store.datasetRange).not.toContain("1970");
    expect(store.dataTimeframeHint).toContain("Relative timeline detected");
  });

  it("clears the browse CSV busy status when the picker is cancelled", async () => {
    browseCsvMock.mockResolvedValueOnce(null);
    const store = useDataStore();

    store.browseCsv();
    expect(store.statusLine).toBe("Loading data...");
    await flushAsync();

    expect(store.statusLine).toBe("CSV selection cancelled.");
    expect(store.canBrowseCsv).toBe(true);
  });

  it("surfaces browse CSV picker failures", async () => {
    browseCsvMock.mockRejectedValueOnce(new Error("picker unavailable"));
    const store = useDataStore();

    store.browseCsv();
    await flushAsync();

    expect(store.statusLine).toContain("picker unavailable");
    expect(store.canBrowseCsv).toBe(true);
  });

  it("dispatches market-data directory and sample actions through data service", async () => {
    const store = useDataStore();
    store.browseCsv();
    store.openPolygon();
    store.closePolygon();
    store.setMarketDataDir(" C:/cache ", true);
    store.browseMarketDataDir();
    store.resetMarketDataDir();
    store.openMarketDataDir();
    store.loadSampleCsv(2, "");
    store.loadSampleCsv(1, " C:/samples/aapl.csv ");
    await flushAsync();

    expect(browseCsvMock).toHaveBeenCalled();
    expect(openPolygonMock).toHaveBeenCalled();
    expect(closePolygonMock).toHaveBeenCalled();
    expect(setMarketDataDirMock).toHaveBeenNthCalledWith(1, "C:/cache", true);
    expect(browseMarketDataDirMock).toHaveBeenCalled();
    expect(resetMarketDataDirMock).toHaveBeenCalled();
    expect(openMarketDataDirMock).toHaveBeenCalled();
    expect(loadSampleCsvMock).toHaveBeenNthCalledWith(1, 2, "");
    expect(loadSampleCsvMock).toHaveBeenNthCalledWith(
      2,
      1,
      " C:/samples/aapl.csv ",
    );
  });

  it("opens, refreshes, and closes the Vue-native Polygon workflow", async () => {
    const store = useDataStore();

    store.openPolygon();
    await flushAsync();
    expect(store.polygonOpen).toBe(true);
    expect(openPolygonMock).toHaveBeenCalled();
    expect(getPolygonCacheInfoMock).toHaveBeenCalled();
    expect(store.polygonHasApiKey).toBe(true);
    expect(store.polygonDatasets.length).toBe(1);

    store.refreshPolygon();
    await flushAsync();
    expect(getPolygonCacheInfoMock).toHaveBeenCalledTimes(2);

    store.closePolygon();
    await flushAsync();
    expect(store.polygonOpen).toBe(false);
    expect(closePolygonMock).toHaveBeenCalled();
  });

  it("formats browser-runtime Polygon cache failures without the runtime class prefix", async () => {
    getPolygonCacheInfoMock.mockRejectedValueOnce(
      new TauriRuntimeError({
        code: "TAURI_INVOKE_FAILED",
        operation: "polygon_get_cache_info",
        message:
          "Desktop runtime unavailable for polygon get cache info. Open this action in the Tauri desktop app.",
      }),
    );
    const store = useDataStore();

    store.refreshPolygon();
    await flushAsync();
    await flushAsync();

    expect(store.statusLine).toContain(
      "Desktop runtime unavailable for polygon get cache info.",
    );
    expect(store.statusLine).not.toContain("TauriRuntimeError:");
  });

  it("falls back to the first cached dataset when refresh drops the prior selection", async () => {
    const store = useDataStore();
    getPolygonCacheInfoMock
      .mockResolvedValueOnce({
        ...defaultPolygonCacheInfo,
        datasets: [
          defaultPolygonDataset,
          {
            ...defaultPolygonDataset,
            displaySymbol: "MSFT",
            polygonTicker: "MSFT",
            csvPath: "C:/default/polygon-cache/MSFT_1m.csv",
          },
        ],
      })
      .mockResolvedValueOnce({
        ...defaultPolygonCacheInfo,
        datasets: [defaultPolygonDataset],
      });

    store.openPolygon();
    await flushAsync();
    store.selectPolygonDataset("C:/default/polygon-cache/MSFT_1m.csv");

    store.refreshPolygon();
    await flushAsync();

    expect(store.polygonSelectedPath).toBe(defaultPolygonDataset.csvPath);
    expect(store.polygonSelectedMeta).toContain("AAPL");
  });

  it("saves API keys and downloads/loads Polygon data through the store", async () => {
    const store = useDataStore();
    store.setInputField("polygonApiKey", "secret");
    store.setInputField("polygonSymbol", "aapl");
    store.setInputField("polygonFrom", "2024-01-01");
    store.setInputField("polygonTo", "2024-01-31");
    store.setSelectField("polygonTimeframe", "15m");

    store.savePolygonApiKey();
    await flushAsync();
    expect(setPolygonApiKeyMock).toHaveBeenCalledWith("secret");
    expect(store.polygonHasApiKey).toBe(true);

    store.downloadPolygonDataset();
    await flushAsync();
    await flushAsync();
    expect(downloadPolygonDatasetMock).toHaveBeenCalledWith({
      market: "stocks",
      symbol: "AAPL",
      from: "2024-01-01",
      to: "2024-01-31",
      adjusted: true,
      timeframe: "15m",
    });
    expect(store.dataId).toBe("poly-1");
    expect(store.baseDataId).toBe("poly-1-base");
    expect(store.dataTimeframe).toBe("15m");
    expect(store.datasetContext).toContain("source=polygon");
    expect(store.baseSourceKind).toBe("polygon");
  });

  it("blocks Polygon downloads when the date range is inverted", async () => {
    const store = useDataStore();
    store.setInputField("polygonSymbol", "aapl");
    store.setInputField("polygonFrom", "2024-02-01");
    store.setInputField("polygonTo", "2024-01-01");

    store.downloadPolygonDataset();
    await flushAsync();

    expect(downloadPolygonDatasetMock).not.toHaveBeenCalled();
    expect(store.statusLine).toContain("end date must be on or after");
  });

  it("normalizes Polygon provider failures into user-facing guidance", async () => {
    const store = useDataStore();
    store.setInputField("polygonApiKey", "poly_secret_123");
    store.setInputField("polygonSymbol", "aapl");
    store.setInputField("polygonFrom", "2024-01-01");
    store.setInputField("polygonTo", "2024-01-31");
    downloadPolygonDatasetMock.mockRejectedValueOnce(
      new Error("Polygon API request failed 429: apiKey=poly_secret_123"),
    );

    store.downloadPolygonDataset();
    await flushAsync();

    expect(store.statusLine).toContain("Polygon rate limit reached");
    expect(store.statusLine).toContain("reduce the date range");
    expect(store.statusLine).not.toContain("poly_secret_123");
  });

  it("redacts typed Polygon API keys from surfaced action failures", async () => {
    const store = useDataStore();
    store.setInputField("polygonApiKey", "poly_secret_123");
    setPolygonApiKeyMock.mockRejectedValueOnce(
      new Error("provider rejected apiKey=poly_secret_123"),
    );

    store.savePolygonApiKey();
    await flushAsync();

    expect(store.statusLine).toContain("[redacted]");
    expect(store.statusLine).not.toContain("poly_secret_123");
  });

  it("surfaces cache reuse and unchecked integrity for downloaded Polygon datasets", async () => {
    const store = useDataStore();
    downloadPolygonDatasetMock.mockResolvedValueOnce({
      csvPath: "C:/default/polygon-cache/AAPL_1m.csv",
      downloadedBars: 0,
      usedCacheOnly: true,
      baseLoad: {
        ...defaultLoadResult,
        path: "C:/default/polygon-cache/AAPL_1m.csv",
        dataId: "poly-cache-base",
        meta: { ...defaultLoadResult.meta, timeframe: "1m", len: 5000 },
      },
      load: {
        ...defaultLoadResult,
        path: "C:/default/polygon-cache/AAPL_1m.csv",
        dataId: "poly-cache-base",
        meta: { ...defaultLoadResult.meta, timeframe: "1m", len: 5000 },
      },
    });

    store.setInputField("polygonSymbol", "aapl");
    store.setInputField("polygonFrom", "2024-01-01");
    store.setInputField("polygonTo", "2024-01-31");

    store.downloadPolygonDataset();
    await flushAsync();
    await flushAsync();

    expect(store.statusLine).toContain("cache reused");
    expect(store.statusLine).toContain("AAPL");
    expect(store.polygonSelectedMeta).toContain("Integrity: unchecked");
    expect(store.polygonSelectedMeta).toContain("adjusted");
  });

  it("auto-tops off a matching cached Polygon dataset when re-downloading the same cached coverage", async () => {
    const store = useDataStore();
    store.openPolygon();
    await flushAsync();

    store.polygonDatasets = [
      {
        ...defaultPolygonDataset,
        minTs: Date.UTC(2024, 0, 1, 14, 30),
        maxTs: Date.UTC(2024, 0, 31, 21, 0),
      },
    ];
    store.polygonSelectedPath = defaultPolygonDataset.csvPath;
    store.setInputField("polygonSymbol", "aapl");
    store.setInputField("polygonFrom", "2024-01-01");
    store.setInputField("polygonTo", "2024-01-31");
    store.setSelectField("polygonTimeframe", "15m");

    store.downloadPolygonDataset();
    await flushAsync();
    await flushAsync();

    expect(topOffPolygonDatasetMock).toHaveBeenCalledWith({
      csvPath: defaultPolygonDataset.csvPath,
      timeframe: "15m",
    });
    expect(downloadPolygonDatasetMock).not.toHaveBeenCalled();
    expect(store.baseDataId).toBe("poly-topoff-base");
    expect(store.dataId).toBe("poly-topoff");
    expect(store.dataTimeframe).toBe("15m");
  });

  it("validates and repairs the selected Polygon dataset", async () => {
    const store = useDataStore();
    store.openPolygon();
    await flushAsync();

    validatePolygonDatasetMock.mockResolvedValueOnce({
      ...defaultPolygonDataset,
      rowCount: 1600,
      minTs: 1699999000,
      maxTs: 1700006000,
      integrity: {
        checkedUtcMs: 1700006000000,
        parseErrors: 0,
        duplicates: 1,
        outOfOrder: 0,
        sameDayGapsGt2Min: 0,
      },
    });
    repairPolygonDatasetMock.mockResolvedValueOnce({
      ...defaultPolygonDataset,
      rowCount: 1600,
      minTs: 1699999000,
      maxTs: 1700006000,
      integrity: {
        checkedUtcMs: 1700007000000,
        parseErrors: 0,
        duplicates: 0,
        outOfOrder: 0,
        sameDayGapsGt2Min: 0,
      },
    });

    store.validatePolygonDataset();
    await flushAsync();
    expect(validatePolygonDatasetMock).toHaveBeenCalledWith(
      defaultPolygonDataset.csvPath,
    );
    expect(store.polygonSelectedMeta).toContain("dup=1");
    expect(store.polygonDatasets[0]?.rowCount).toBe(1600);
    expect(store.polygonCacheMeta).toContain("1 cached dataset");

    store.repairPolygonDataset();
    await flushAsync();
    expect(repairPolygonDatasetMock).toHaveBeenCalledWith(
      defaultPolygonDataset.csvPath,
    );
    expect(store.polygonSelectedMeta).toContain("dup=0");
    expect(store.polygonSelectedPath).toBe(defaultPolygonDataset.csvPath);

    store.clearPolygonApiKey();
    await flushAsync();
    expect(clearPolygonApiKeyMock).toHaveBeenCalled();
    expect(store.polygonHasApiKey).toBe(false);
  });

  it("tops off the selected Polygon dataset and preserves the active derived timeframe", async () => {
    const store = useDataStore();
    store.openPolygon();
    await flushAsync();

    store.applyLoadResult(
      {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-1-base",
      },
      { treatAsBase: true, source: "polygon" },
    );
    store.applyLoadResult(
      {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-1",
        meta: { ...defaultLoadResult.meta, timeframe: "15m", len: 100 },
      },
      { treatAsBase: false, source: "polygon" },
    );

    store.topOffPolygonDataset();
    await flushAsync();
    await flushAsync();

    expect(topOffPolygonDatasetMock).toHaveBeenCalledWith({
      csvPath: defaultPolygonDataset.csvPath,
      timeframe: "15m",
    });
    expect(store.baseDataId).toBe("poly-topoff-base");
    expect(store.dataId).toBe("poly-topoff");
    expect(store.dataTimeframe).toBe("15m");
  });

  it("keeps no-new-bars status text visible when Polygon top-off finds nothing new", async () => {
    const store = useDataStore();
    store.openPolygon();
    await flushAsync();

    topOffPolygonDatasetMock.mockResolvedValueOnce({
      csvPath: defaultPolygonDataset.csvPath,
      downloadedBars: 0,
      usedCacheOnly: false,
      baseLoad: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-topoff-empty-base",
      },
      load: {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-topoff-empty-base",
      },
    });

    store.topOffPolygonDataset();
    await flushAsync();
    await flushAsync();

    expect(store.statusLine).toContain("no new bars available");
    expect(store.statusLine).toContain("Loaded Polygon dataset");
  });

  it("tops off a non-active cached dataset using 1m and keeps it selected after refresh", async () => {
    const store = useDataStore();
    const otherDataset = {
      ...defaultPolygonDataset,
      displaySymbol: "MSFT",
      polygonTicker: "MSFT",
      csvPath: "C:/default/polygon-cache/MSFT_1m.csv",
    };

    store.openPolygon();
    await flushAsync();
    store.polygonDatasets = [defaultPolygonDataset, otherDataset];
    store.selectPolygonDataset(otherDataset.csvPath);
    store.applyLoadResult(
      {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-1-base",
      },
      { treatAsBase: true, source: "polygon" },
    );
    store.applyLoadResult(
      {
        ...defaultLoadResult,
        path: defaultPolygonDataset.csvPath,
        dataId: "poly-1",
        meta: { ...defaultLoadResult.meta, timeframe: "15m", len: 100 },
      },
      { treatAsBase: false, source: "polygon" },
    );

    topOffPolygonDatasetMock.mockResolvedValueOnce({
      csvPath: otherDataset.csvPath,
      downloadedBars: 5,
      usedCacheOnly: false,
      baseLoad: {
        ...defaultLoadResult,
        path: otherDataset.csvPath,
        dataId: "poly-msft-base",
      },
      load: {
        ...defaultLoadResult,
        path: otherDataset.csvPath,
        dataId: "poly-msft-base",
      },
    });
    getPolygonCacheInfoMock.mockResolvedValueOnce({
      ...defaultPolygonCacheInfo,
      datasets: [otherDataset, defaultPolygonDataset],
    });

    store.topOffPolygonDataset();
    await flushAsync();
    await flushAsync();
    await flushAsync();

    expect(topOffPolygonDatasetMock).toHaveBeenCalledWith({
      csvPath: otherDataset.csvPath,
      timeframe: "1m",
    });
    expect(store.polygonSelectedPath).toBe(otherDataset.csvPath);
    expect(store.statusLine).toContain("bars downloaded");
  });

  it("hydrates startup data using Vue-owned data bootstrap services", async () => {
    const store = useDataStore();

    await store.hydrateStartupState();

    expect(getMarketDataDirInfoMock).toHaveBeenCalled();
    expect(listSampleCsvsMock).toHaveBeenCalled();
    expect(getPolygonCacheInfoMock).not.toHaveBeenCalled();
    expect(store.marketDataDir).toBe("");
    expect(store.marketDataDirHint).toContain("Using default root");
    expect(store.sampleCsvVisible).toBe(true);
    expect(store.sampleCsvOptions).toEqual(defaultSampleRows);
    expect(store.polygonCacheDir).toBe(
      defaultMarketDataDirInfo.polygonCacheDir,
    );
  });

  it("can hydrate only the critical startup dataset-readiness path", async () => {
    const store = useDataStore();

    await store.hydrateStartupState({ includeDeferred: false });

    expect(getMarketDataDirInfoMock).toHaveBeenCalled();
    expect(listSampleCsvsMock).not.toHaveBeenCalled();
    expect(store.sampleCsvVisible).toBe(false);
    expect(store.sampleCsvOptions).toEqual([]);
    expect(store.polygonCacheDir).toBe(
      defaultMarketDataDirInfo.polygonCacheDir,
    );
  });

  it("restores the startup dataset snapshot without eagerly scanning Polygon cache", async () => {
    const store = useDataStore();
    writeStorageJson(DATA_STARTUP_SNAPSHOT_KEY, {
      autoRestoreDataset: true,
      baseCsvPath: "C:/tmp/aapl.csv",
      selectedTimeframe: "15m",
      source: "csv",
      baseSourceSizeBytes: 123,
      baseSourceMtimeUtcMs: 456,
      savedAtUnixMs: Date.now(),
    });

    await store.hydrateStartupState();

    expect(loadCsvMock).toHaveBeenCalledWith("C:/tmp/aapl.csv");
    expect(applyTimeframeMock).toHaveBeenCalledWith("15m", "data-1");
    expect(getPolygonCacheInfoMock).not.toHaveBeenCalled();
    expect(store.baseDataId).toBe("data-1");
    expect(store.dataId).toBe("data-1@15m");
    expect(store.dataTimeframe).toBe("15m");
    expect(store.baseSourceSizeBytes).toBe(123);
    expect(store.baseSourceMtimeUtcMs).toBe(456);
  });

  it("reuses the same critical startup hydration promise across concurrent callers", async () => {
    const store = useDataStore();
    const deferred = createDeferred<MarketDataDirInfoSummary>();
    getMarketDataDirInfoMock.mockReturnValueOnce(deferred.promise);

    const first = store.hydrateStartupState({ includeDeferred: false });
    const second = store.hydrateStartupState({ includeDeferred: false });

    expect(getMarketDataDirInfoMock).toHaveBeenCalledTimes(1);

    deferred.resolve({
      defaultRootDir: "C:/default",
      configuredRootDir: "",
      effectiveRootDir: "C:/default",
      polygonCacheDir: "C:/default/polygon-cache",
      hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
    });

    await Promise.all([first, second]);
    expect(store.marketDataDirHint).toContain("Using default root: C:/default");
  });

  it("warns when startup restore finds the same path with different source identity", async () => {
    const store = useDataStore();
    loadCsvMock.mockResolvedValueOnce({
      ...defaultLoadResult,
      meta: {
        ...defaultLoadResult.meta,
        sourceSizeBytes: 999,
        sourceMtimeUtcMs: 888,
      },
    });
    writeStorageJson(DATA_STARTUP_SNAPSHOT_KEY, {
      autoRestoreDataset: true,
      baseCsvPath: "C:/tmp/aapl.csv",
      selectedTimeframe: "15m",
      source: "csv",
      baseSourceSizeBytes: 123,
      baseSourceMtimeUtcMs: 456,
      savedAtUnixMs: Date.now(),
    });

    await store.hydrateStartupState({ includeDeferred: false });

    expect(loadCsvMock).toHaveBeenCalledWith("C:/tmp/aapl.csv");
    expect(applyTimeframeMock).toHaveBeenCalledWith("15m", "data-1");
    expect(store.baseDataId).toBe("data-1");
    expect(store.dataId).toBe("data-1@15m");
    expect(store.statusLine).toContain("Source changed since last session");
    expect(store.statusLine).toContain("size changed");
    expect(store.statusLine).toContain("mtime changed");
  });

  it("shows startup restore pending when legacy hydration sees an auto-restore snapshot", () => {
    const store = useDataStore();
    writeStorageJson(DATA_STARTUP_SNAPSHOT_KEY, {
      autoRestoreDataset: true,
      baseCsvPath: "C:/tmp/aapl.csv",
      selectedTimeframe: "",
      source: "csv",
      baseSourceSizeBytes: 123,
      baseSourceMtimeUtcMs: 456,
      savedAtUnixMs: Date.now(),
    });

    store.applyLegacySnapshot({
      autoRestoreDataset: true,
    });

    expect(store.statusLine).toBe("Startup restore pending.");
    expect(store.dataId).toBe("—");
    expect(store.baseDataId).toBe("");
  });

  it("surfaces restore failure instead of hiding it behind a ready state", async () => {
    const store = useDataStore();
    loadCsvMock.mockRejectedValueOnce(new Error("missing csv"));
    writeStorageJson(DATA_STARTUP_SNAPSHOT_KEY, {
      autoRestoreDataset: true,
      baseCsvPath: "C:/tmp/missing.csv",
      selectedTimeframe: "15m",
      source: "csv",
      baseSourceSizeBytes: 123,
      baseSourceMtimeUtcMs: 456,
      savedAtUnixMs: Date.now(),
    });

    await store.hydrateStartupState({ includeDeferred: false });

    expect(store.statusLine).toContain("Startup restore failed:");
    expect(store.statusLine).toContain("missing csv");
    expect(store.dataId).toBe("—");
    expect(store.baseDataId).toBe("");
    expect(store.startupHydratedOnce).toBe(true);
  });

  it("waits for critical startup hydration before loading deferred sample affordances", async () => {
    const store = useDataStore();
    const deferred = createDeferred<MarketDataDirInfoSummary>();
    getMarketDataDirInfoMock.mockReturnValueOnce(deferred.promise);

    const hydration = store.hydrateStartupState();

    expect(getMarketDataDirInfoMock).toHaveBeenCalledTimes(1);
    expect(listSampleCsvsMock).not.toHaveBeenCalled();
    expect(store.startupHydratedOnce).toBe(false);
    expect(store.startupDeferredHydratedOnce).toBe(false);

    deferred.resolve({
      defaultRootDir: "C:/default",
      configuredRootDir: "",
      effectiveRootDir: "C:/default",
      polygonCacheDir: "C:/default/polygon-cache",
      hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
    });

    await hydration;

    expect(store.startupHydratedOnce).toBe(true);
    expect(store.startupDeferredHydratedOnce).toBe(true);
    expect(listSampleCsvsMock).toHaveBeenCalledTimes(1);
    expect(store.sampleCsvVisible).toBe(true);
    expect(store.sampleCsvOptions).toEqual(defaultSampleRows);
  });

  it("shares the same critical startup work when deferred hydration overlaps a critical-only caller", async () => {
    const store = useDataStore();
    const deferred = createDeferred<MarketDataDirInfoSummary>();
    getMarketDataDirInfoMock.mockReturnValueOnce(deferred.promise);

    const bootCritical = store.hydrateStartupState({ includeDeferred: false });
    const panelFollowup = store.hydrateStartupState();

    expect(getMarketDataDirInfoMock).toHaveBeenCalledTimes(1);
    expect(listSampleCsvsMock).not.toHaveBeenCalled();

    deferred.resolve({
      defaultRootDir: "C:/default",
      configuredRootDir: "",
      effectiveRootDir: "C:/default",
      polygonCacheDir: "C:/default/polygon-cache",
      hint: "Using default root: C:/default · Polygon cache: C:/default/polygon-cache",
    });

    await bootCritical;
    expect(store.startupHydratedOnce).toBe(true);

    await panelFollowup;

    expect(store.startupDeferredHydratedOnce).toBe(true);
    expect(listSampleCsvsMock).toHaveBeenCalledTimes(1);
    expect(store.sampleCsvVisible).toBe(true);
    expect(store.sampleCsvOptions).toEqual(defaultSampleRows);
  });

  it("does not eagerly scan Polygon cache when stale polygon-open state is restored during startup hydration", async () => {
    const store = useDataStore();
    store.applyLegacySnapshot({
      polygonOpen: true,
      autoRestoreDataset: true,
    });
    writeStorageJson(DATA_STARTUP_SNAPSHOT_KEY, {
      autoRestoreDataset: true,
      baseCsvPath: "C:/tmp/aapl.csv",
      selectedTimeframe: "",
      source: "csv",
      baseSourceSizeBytes: 123,
      baseSourceMtimeUtcMs: 456,
      savedAtUnixMs: Date.now(),
    });

    await store.hydrateStartupState({ includeDeferred: false });

    expect(loadCsvMock).toHaveBeenCalledWith("C:/tmp/aapl.csv");
    expect(getPolygonCacheInfoMock).not.toHaveBeenCalled();
    expect(store.polygonOpen).toBe(true);
  });

  it("rewrites Polygon startup snapshot paths when moving the cache root with moveExisting=true", async () => {
    const store = useDataStore();
    const oldPolygonPath = "C:/default/polygon-cache/AAPL_1m.csv";
    const nextPolygonPath = "C:/custom/polygon-cache/AAPL_1m.csv";
    setMarketDataDirMock.mockResolvedValueOnce({
      defaultRootDir: "C:/default",
      configuredRootDir: "C:/custom",
      effectiveRootDir: "C:/custom",
      polygonCacheDir: "C:/custom/polygon-cache",
      hint: "Using custom root: C:/custom · Polygon cache: C:/custom/polygon-cache",
    });

    store.applyLoadResult(
      {
        ...defaultLoadResult,
        path: oldPolygonPath,
        dataId: "poly-1-base",
      },
      { treatAsBase: true, source: "polygon" },
    );
    store.polygonCacheDir = "C:/default/polygon-cache";
    store.polygonSelectedPath = oldPolygonPath;
    store.recentCsvPaths = [oldPolygonPath, "C:/tmp/manual.csv"];

    store.setMarketDataDir("C:/custom", true);
    await flushAsync();

    expect(setMarketDataDirMock).toHaveBeenCalledWith("C:/custom", true);
    expect(store.baseCsvPath).toBe(nextPolygonPath);
    expect(store.csvPath).toBe(nextPolygonPath);
    expect(store.polygonSelectedPath).toBe(nextPolygonPath);
    expect(store.recentCsvPaths).toContain(nextPolygonPath);
    expect(store.recentCsvPaths).toContain("C:/tmp/manual.csv");
    expect(
      readStorageJson<Record<string, unknown>>(DATA_STARTUP_SNAPSHOT_KEY)
        ?.baseCsvPath,
    ).toBe(nextPolygonPath);
  });

  it("keeps Polygon startup snapshot paths unchanged when moveExisting=false", async () => {
    const store = useDataStore();
    const oldPolygonPath = "C:/default/polygon-cache/AAPL_1m.csv";
    setMarketDataDirMock.mockResolvedValueOnce({
      defaultRootDir: "C:/default",
      configuredRootDir: "C:/custom",
      effectiveRootDir: "C:/custom",
      polygonCacheDir: "C:/custom/polygon-cache",
      hint: "Using custom root: C:/custom · Polygon cache: C:/custom/polygon-cache",
    });

    store.applyLoadResult(
      {
        ...defaultLoadResult,
        path: oldPolygonPath,
        dataId: "poly-1-base",
      },
      { treatAsBase: true, source: "polygon" },
    );
    store.polygonCacheDir = "C:/default/polygon-cache";
    store.polygonSelectedPath = oldPolygonPath;

    store.setMarketDataDir("C:/custom", false);
    await flushAsync();

    expect(store.baseCsvPath).toBe(oldPolygonPath);
    expect(store.csvPath).toBe(oldPolygonPath);
    expect(store.polygonSelectedPath).toBe("");
    expect(
      readStorageJson<Record<string, unknown>>(DATA_STARTUP_SNAPSHOT_KEY)
        ?.baseCsvPath,
    ).toBe(oldPolygonPath);
  });

  it("clears data state while still calling data service clear hook", async () => {
    const store = useDataStore();
    const artifacts = useWorkflowArtifactsStore(pinia);
    store.loadCsv("C:/tmp/aapl.csv");
    await flushAsync();
    artifacts.applyVueRuntimePayload({
      currentRun: {
        ts: "2026-04-09T10:00:00Z",
        artifactPath: "C:/runs/run.json",
        strategyKind: "DoubleMa",
      },
      currentValidation: {
        ts: "2026-04-09T10:05:00Z",
        artifactPath: "C:/runs/validation.json",
      },
    });

    store.clearData();
    await flushAsync();

    expect(clearDataMock).toHaveBeenCalledWith("data-1");
    expect(store.dataId).toBe("—");
    expect(store.baseDataId).toBe("");
    expect(store.csvPath).toBe("");
    expect(store.datasetLabel).toBe("—");
    expect(store.dataFields).toEqual([]);
    expect(store.canApplyTimeframe).toBe(false);
    expect(artifacts.currentRun).toBeNull();
    expect(artifacts.currentValidation).toBeNull();
    expect(
      readStorageJson<Record<string, unknown>>(DATA_STARTUP_SNAPSHOT_KEY)
        ?.baseCsvPath,
    ).toBe("");
  });
});
