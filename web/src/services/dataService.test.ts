import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import {
  applyTimeframe,
  browseCsv,
  browseMarketDataDir,
  clearData,
  clearPolygonApiKey,
  closePolygon,
  downloadPolygonDataset,
  getPolygonCacheInfo,
  getMarketDataDirInfo,
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
} from "./dataService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

const invokeCommandMock = vi.mocked(invokeCommand);

describe("dataService market-data directory actions", () => {
  beforeEach(() => {
    invokeCommandMock.mockReset();
  });

  it("normalizes market-data directory info and hint text", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      default_root_dir: "C:/default",
      configured_root_dir: "C:/custom",
      effective_root_dir: "C:/effective",
      polygon_cache_dir: "C:/effective/polygon-cache",
    });

    const info = await getMarketDataDirInfo();

    expect(invokeCommandMock).toHaveBeenCalledWith("get_market_data_dir_info");
    expect(info.defaultRootDir).toBe("C:/default");
    expect(info.configuredRootDir).toBe("C:/custom");
    expect(info.effectiveRootDir).toBe("C:/effective");
    expect(info.polygonCacheDir).toBe("C:/effective/polygon-cache");
    expect(info.hint).toContain("Using custom root: C:/custom");
  });

  it("returns trimmed picked folder path", async () => {
    invokeCommandMock.mockResolvedValueOnce(" C:/cache ");

    const path = await browseMarketDataDir();

    expect(invokeCommandMock).toHaveBeenCalledWith("pick_folder");
    expect(path).toBe("C:/cache");
  });

  it("picks CSV and loads via load_price_data command", async () => {
    invokeCommandMock
      .mockResolvedValueOnce(" C:/sample.csv ")
      .mockResolvedValueOnce({
        id: "data-1",
        meta: {
          len: 123,
          has_open: true,
          has_high: true,
          has_low: true,
          has_close: true,
          has_volume: true,
          min_ts: 1_700_000_000,
          max_ts: 1_700_000_999,
          timeframe: "1m",
        },
        cache_hit: false,
        cache_path: "C:/cache/a.bin",
        load_source: "csv_parsed_and_cached",
      });

    const result = await browseCsv();

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "pick_csv_file");
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "load_price_data", {
      path: "C:/sample.csv",
    });
    expect(result?.path).toBe("C:/sample.csv");
    expect(result?.dataId).toBe("data-1");
    expect(result?.meta.timeframe).toBe("1m");
    expect(result?.loadSource).toBe("csv_parsed_and_cached");
  });

  it("returns null when the native CSV picker is cancelled", async () => {
    invokeCommandMock.mockResolvedValueOnce(null);

    const result = await browseCsv();

    expect(invokeCommandMock).toHaveBeenCalledTimes(1);
    expect(invokeCommandMock).toHaveBeenCalledWith("pick_csv_file");
    expect(result).toBeNull();
  });

  it("surfaces native CSV picker failures", async () => {
    invokeCommandMock.mockRejectedValueOnce(new Error("picker unavailable"));

    await expect(browseCsv()).rejects.toThrow("picker unavailable");
    expect(invokeCommandMock).toHaveBeenCalledTimes(1);
    expect(invokeCommandMock).toHaveBeenCalledWith("pick_csv_file");
  });

  it("loads CSV directly and normalizes response shape", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      id: "data-2",
      meta: {
        len: "250",
        has_open: 1,
        has_high: true,
        has_low: true,
        has_close: true,
        has_volume: 0,
        min_ts: "1700000010",
        max_ts: "1700000510",
        timeframe: "5m",
      },
      cache_hit: true,
      cache_path: "C:/cache/data-2.bin",
      load_source: "binary_cache",
    });

    const result = await loadCsv(" C:/raw.csv ");

    expect(invokeCommandMock).toHaveBeenCalledWith("load_price_data", {
      path: "C:/raw.csv",
    });
    expect(result.path).toBe("C:/raw.csv");
    expect(result.dataId).toBe("data-2");
    expect(result.cacheHit).toBe(true);
    expect(result.loadSource).toBe("binary_cache");
    expect(result.meta.len).toBe(250);
    expect(result.meta.hasVolume).toBe(false);
    expect(result.meta.timeframe).toBe("5m");
  });

  it("keeps data IPC boundary parsing tolerant for malformed, partial, and extra-field payloads", async () => {
    invokeCommandMock
      .mockResolvedValueOnce("not an array")
      .mockResolvedValueOnce({
        id: "partial-data",
        extra_backend_field: { ignored: true },
      })
      .mockResolvedValueOnce("not an object");

    await expect(listSampleCsvs()).resolves.toEqual([]);

    const partial = await loadCsv(" C:/partial.csv ");
    expect(partial).toEqual(
      expect.objectContaining({
        path: "C:/partial.csv",
        dataId: "partial-data",
        cacheHit: false,
        cachePath: "",
        loadSource: "csv_parsed_cache_write_skipped",
      }),
    );
    expect(partial.meta).toEqual({
      len: 0,
      hasOpen: false,
      hasHigh: false,
      hasLow: false,
      hasClose: false,
      hasVolume: false,
      minTs: 0,
      maxTs: 0,
      timeframe: "",
      sourceSizeBytes: null,
      sourceMtimeUtcMs: null,
    });

    const info = await getMarketDataDirInfo();
    expect(info.defaultRootDir).toBe("");
    expect(info.hint).toContain("Using default root:");
  });

  it("applies timeframe using base data id", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      id: "data-2@15m",
      meta: { len: 100, timeframe: "15m", has_close: true },
      cache_hit: false,
      cache_path: "",
      load_source: "derived_fresh_resample",
    });

    const result = await applyTimeframe("15m", "data-2");

    expect(invokeCommandMock).toHaveBeenCalledWith("resample_price_data", {
      baseDataId: "data-2",
      timeframe: "15m",
    });
    expect(result.dataId).toBe("data-2@15m");
    expect(result.meta.timeframe).toBe("15m");
    expect(result.loadSource).toBe("derived_fresh_resample");
  });

  it("rejects applyTimeframe when base data id or timeframe is missing", async () => {
    await expect(applyTimeframe("15m", "")).rejects.toThrow(
      "Load a dataset first (missing base data id).",
    );
    await expect(applyTimeframe("   ", "data-2")).rejects.toThrow(
      "Choose a timeframe first.",
    );
  });

  it("loads sample CSV by index using list_sample_csvs command", async () => {
    invokeCommandMock
      .mockResolvedValueOnce([{ path: "C:/a.csv" }, { path: " C:/b.csv " }])
      .mockResolvedValueOnce({
        id: "data-9",
        meta: { len: 42, timeframe: "1m", has_close: true },
        cache_hit: false,
        cache_path: "",
        load_source: "csv_parsed_cache_write_skipped",
      });

    const result = await loadSampleCsv(1);

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "list_sample_csvs");
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "load_price_data", {
      path: "C:/b.csv",
    });
    expect(result?.dataId).toBe("data-9");
    expect(result?.loadSource).toBe("csv_parsed_cache_write_skipped");
  });

  it("infers base-load reuse when legacy responses omit load_source", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      id: "data-legacy-base",
      meta: { len: 42, timeframe: "1m", has_close: true },
      cache_hit: true,
      cache_path: "",
    });

    const result = await loadCsv(" C:/legacy.csv ");

    expect(result.dataId).toBe("data-legacy-base");
    expect(result.loadSource).toBe("in_memory_base_reuse");
  });

  it("infers derived reuse when legacy resample responses omit load_source", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      id: "data-legacy@15m",
      meta: { len: 21, timeframe: "15m", has_close: true },
      cache_hit: true,
      cache_path: "",
    });

    const result = await applyTimeframe("15m", "data-legacy");

    expect(result.dataId).toBe("data-legacy@15m");
    expect(result.loadSource).toBe("derived_in_memory_reuse");
  });

  it("lists sample CSV entries with normalized labels and paths", async () => {
    invokeCommandMock.mockResolvedValueOnce([
      { path: " C:/samples/aapl.csv " },
      {
        label: "MSFT sample",
        path: "C:/samples/msft.csv",
        source_kind: "bundled_synthetic",
        timeframe: "1m",
        row_count: "160",
        range: "relative 0m -> 2h 39m",
        description: "Synthetic sample",
        license: "Redistributable",
        research_note: "Research only",
      },
      { label: "bad", path: "   " },
    ]);

    const rows = await listSampleCsvs();

    expect(invokeCommandMock).toHaveBeenCalledWith("list_sample_csvs");
    expect(rows).toEqual([
      {
        label: "aapl.csv",
        path: "C:/samples/aapl.csv",
        disabled: false,
        sourceKind: "",
        timeframe: "",
        rowCount: null,
        range: "",
        description: "",
        license: "",
        researchNote: "",
        detail: "",
      },
      {
        label: "MSFT sample",
        path: "C:/samples/msft.csv",
        disabled: false,
        sourceKind: "bundled_synthetic",
        timeframe: "1m",
        rowCount: 160,
        range: "relative 0m -> 2h 39m",
        description: "Synthetic sample",
        license: "Redistributable",
        researchNote: "Research only",
        detail:
          "Synthetic sample · 1m candles · 160 rows · relative 0m -> 2h 39m · Research only",
      },
    ]);
  });

  it("sets and resets market-data folder via direct commands", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        default_root_dir: "C:/default",
        configured_root_dir: "C:/custom",
        effective_root_dir: "C:/custom",
        polygon_cache_dir: "C:/custom/polygon-cache",
      })
      .mockResolvedValueOnce({
        default_root_dir: "C:/default",
        configured_root_dir: "",
        effective_root_dir: "C:/default",
        polygon_cache_dir: "C:/default/polygon-cache",
      });

    const setInfo = await setMarketDataDir(" C:/custom ", true);
    const resetInfo = await resetMarketDataDir();

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "set_market_data_dir",
      {
        dir: "C:/custom",
        moveExisting: true,
      },
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      2,
      "clear_market_data_dir",
    );
    expect(setInfo.configuredRootDir).toBe("C:/custom");
    expect(resetInfo.configuredRootDir).toBe("");
  });

  it("invokes clear_loaded_data for a loaded base dataset id", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      cleared: true,
      removed_ids: ["data-1"],
    });

    await clearData(" data-1 ");

    expect(invokeCommandMock).toHaveBeenCalledWith("clear_loaded_data", {
      baseDataId: "data-1",
    });
  });

  it("opens market-data directory and falls back to reveal_path", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        default_root_dir: "C:/default",
        configured_root_dir: "",
        effective_root_dir: "C:/default",
        polygon_cache_dir: "C:/default/polygon-cache",
      })
      .mockRejectedValueOnce(new Error("open failed"))
      .mockResolvedValueOnce(undefined);

    const result = await openMarketDataDir();

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "get_market_data_dir_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "open_path", {
      path: "C:/default/polygon-cache",
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(3, "reveal_path", {
      path: "C:/default/polygon-cache",
    });
    expect(result).toEqual({
      path: "C:/default/polygon-cache",
      openedWith: "reveal_path",
    });
  });

  it("opens the effective market-data root when Polygon cache path is unavailable", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        default_root_dir: "C:/default",
        configured_root_dir: "C:/custom",
        effective_root_dir: "C:/custom",
        polygon_cache_dir: "",
      })
      .mockResolvedValueOnce(undefined);

    const result = await openMarketDataDir();

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "get_market_data_dir_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "open_path", {
      path: "C:/custom",
    });
    expect(result).toEqual({
      path: "C:/custom",
      openedWith: "open_path",
    });
  });

  it("rejects when market-data directory path is empty", async () => {
    await expect(setMarketDataDir("   ", true)).rejects.toThrow(
      "Enter a market data folder path first.",
    );
  });

  it("keeps polygon open/close actions as non-throwing Vue workflow hooks", async () => {
    await openPolygon();
    await closePolygon();
    expect(invokeCommandMock).not.toHaveBeenCalled();
  });

  it("manages Polygon cache info and API key commands", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        cache_dir: "C:/polygon",
        has_api_key: true,
        credential_status: {
          has_api_key: true,
          state: "json_file_fallback",
          storage_backend: "json_file_fallback",
          degraded: true,
          message: "legacy file fallback",
        },
        datasets: [
          {
            csv_path: "C:/polygon/AAPL_1m.csv",
            display_symbol: "AAPL",
            polygon_ticker: "AAPL",
            timeframe: "1m",
            row_count: 10,
          },
        ],
      })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        cache_dir: "C:/polygon",
        has_api_key: true,
        datasets: [],
      })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({
        cache_dir: "C:/polygon",
        has_api_key: false,
        datasets: [],
      });

    const info = await getPolygonCacheInfo();
    const saved = await setPolygonApiKey(" secret ");
    const cleared = await clearPolygonApiKey();

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "polygon_get_cache_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      2,
      "polygon_set_api_key",
      { apiKey: "secret" },
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      3,
      "polygon_get_cache_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      4,
      "polygon_clear_api_key",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      5,
      "polygon_get_cache_info",
    );
    expect(info.hasApiKey).toBe(true);
    expect(info.credentialStatus).toEqual({
      hasApiKey: true,
      state: "json_file_fallback",
      storageBackend: "json_file_fallback",
      degraded: true,
      message: "legacy file fallback",
    });
    expect(saved.hasApiKey).toBe(true);
    expect(cleared.hasApiKey).toBe(false);
  });

  it("normalizes Polygon cache datasets with manifest defaults and integrity details", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      cache_dir: "C:/polygon",
      has_api_key: false,
      datasets: [
        {
          market: "stocks",
          display_symbol: "AAPL",
          polygon_ticker: "AAPL",
          csv_path: "C:/polygon/AAPL_1m.csv",
          row_count: "10",
          adjusted: 0,
          integrity: {
            parse_errors: 1,
            duplicates: 2,
            out_of_order: 3,
            same_day_gaps_gt_2min: 4,
            checked_utc_ms: 1700000000000,
          },
        },
      ],
    });

    const info = await getPolygonCacheInfo();

    expect(info.hasApiKey).toBe(false);
    expect(info.credentialStatus).toEqual({
      hasApiKey: false,
      state: "missing",
      storageBackend: "missing",
      degraded: false,
      message: "",
    });
    expect(info.datasets).toEqual([
      {
        market: "stocks",
        displaySymbol: "AAPL",
        polygonTicker: "AAPL",
        timeframe: "1m",
        csvPath: "C:/polygon/AAPL_1m.csv",
        minTs: 0,
        maxTs: 0,
        lastUpdatedUtcMs: 0,
        rowCount: 10,
        adjusted: false,
        integrity: {
          checkedUtcMs: 1700000000000,
          parseErrors: 1,
          duplicates: 2,
          outOfOrder: 3,
          sameDayGapsGt2Min: 4,
        },
      },
    ]);
  });

  it("downloads, tops off, validates, and repairs Polygon datasets via backend commands", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/AAPL_1m.csv",
        downloaded_bars: 1234,
        used_cache_only: false,
        base_load: {
          id: "poly-1-base",
          meta: { len: 1234, timeframe: "1m", has_close: true },
          cache_hit: false,
          cache_path: "C:/cache/poly-1-base.bin",
          load_source: "csv_parsed_and_cached",
        },
        id: "poly-1",
        meta: {
          len: 96,
          timeframe: "15m",
          has_close: true,
        },
        cache_hit: false,
        cache_path: "C:/cache/poly-1.bin",
        load_source: "derived_fresh_resample",
      })
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/AAPL_1m.csv",
        downloaded_bars: 12,
        used_cache_only: false,
        base_load: {
          id: "poly-2-base",
          meta: { len: 1246, timeframe: "1m", has_close: true },
          cache_hit: false,
          cache_path: "C:/cache/poly-2-base.bin",
          load_source: "csv_parsed_and_cached",
        },
        id: "poly-2",
        meta: {
          len: 97,
          timeframe: "15m",
          has_close: true,
        },
        cache_hit: false,
        cache_path: "C:/cache/poly-2.bin",
        load_source: "derived_fresh_resample",
      })
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/AAPL_1m.csv",
        display_symbol: "AAPL",
        polygon_ticker: "AAPL",
        timeframe: "1m",
        row_count: 1234,
        integrity: {
          parse_errors: 0,
          duplicates: 1,
          out_of_order: 0,
          same_day_gaps_gt_2min: 0,
          checked_utc_ms: 1700000000000,
        },
      })
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/AAPL_1m.csv",
        display_symbol: "AAPL",
        polygon_ticker: "AAPL",
        timeframe: "1m",
        row_count: 1234,
        integrity: {
          parse_errors: 0,
          duplicates: 0,
          out_of_order: 0,
          same_day_gaps_gt_2min: 0,
          checked_utc_ms: 1700000000000,
        },
      });

    const download = await downloadPolygonDataset({
      market: "stocks",
      symbol: "aapl",
      from: "2024-01-01",
      to: "2024-01-31",
      adjusted: true,
      timeframe: "15m",
    });
    const toppedOff = await topOffPolygonDataset({
      csvPath: " C:/polygon/AAPL_1m.csv ",
      timeframe: "15m",
    });
    const validated = await validatePolygonDataset(" C:/polygon/AAPL_1m.csv ");
    const repaired = await repairPolygonDataset(" C:/polygon/AAPL_1m.csv ");

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "polygon_download_1m_and_load",
      {
        req: {
          market: "stocks",
          symbol: "AAPL",
          from: "2024-01-01",
          to: "2024-01-31",
          adjusted: true,
          timeframe: "15m",
        },
      },
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      2,
      "polygon_top_off_dataset",
      {
        req: {
          csv_path: "C:/polygon/AAPL_1m.csv",
          timeframe: "15m",
        },
      },
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      3,
      "polygon_validate_dataset",
      { csvPath: "C:/polygon/AAPL_1m.csv" },
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      4,
      "polygon_repair_dataset",
      { csvPath: "C:/polygon/AAPL_1m.csv" },
    );
    expect(download.downloadedBars).toBe(1234);
    expect(download.baseLoad.dataId).toBe("poly-1-base");
    expect(download.baseLoad.loadSource).toBe("csv_parsed_and_cached");
    expect(download.load.dataId).toBe("poly-1");
    expect(download.load.meta.timeframe).toBe("15m");
    expect(download.load.loadSource).toBe("derived_fresh_resample");
    expect(toppedOff.downloadedBars).toBe(12);
    expect(toppedOff.baseLoad.dataId).toBe("poly-2-base");
    expect(toppedOff.load.dataId).toBe("poly-2");
    expect(validated.integrity?.duplicates).toBe(1);
    expect(repaired.integrity?.duplicates).toBe(0);
  });

  it("rejects malformed or inverted Polygon download date ranges before invoking backend", async () => {
    await expect(
      downloadPolygonDataset({
        market: "stocks",
        symbol: "AAPL",
        from: "2024-02-31",
        to: "2024-03-01",
        adjusted: true,
        timeframe: "1m",
      }),
    ).rejects.toThrow("Polygon dates must use YYYY-MM-DD.");

    await expect(
      downloadPolygonDataset({
        market: "stocks",
        symbol: "AAPL",
        from: "2024-03-01",
        to: "2024-02-01",
        adjusted: true,
        timeframe: "1m",
      }),
    ).rejects.toThrow("Polygon end date must be on or after the start date.");

    expect(invokeCommandMock).not.toHaveBeenCalled();
  });

  it("rejects malformed Polygon download and top-off responses", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        csv_path: "",
        downloaded_bars: 0,
        used_cache_only: false,
        base_load: {
          id: "",
          meta: { len: 0, timeframe: "1m", has_close: true },
          cache_hit: false,
          cache_path: "",
          load_source: "csv_parsed_and_cached",
        },
        id: "",
        meta: { len: 0, timeframe: "1m", has_close: true },
        cache_hit: false,
        cache_path: "",
        load_source: "csv_parsed_and_cached",
      })
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/AAPL_1m.csv",
        downloaded_bars: 0,
        used_cache_only: false,
        base_load: {
          id: "",
          meta: { len: 0, timeframe: "1m", has_close: true },
          cache_hit: false,
          cache_path: "",
          load_source: "csv_parsed_and_cached",
        },
        id: "",
        meta: { len: 0, timeframe: "1m", has_close: true },
        cache_hit: false,
        cache_path: "",
        load_source: "csv_parsed_and_cached",
      });

    await expect(
      downloadPolygonDataset({
        market: "stocks",
        symbol: "AAPL",
        from: "2024-01-01",
        to: "2024-01-02",
        adjusted: true,
        timeframe: "1m",
      }),
    ).rejects.toThrow("Polygon download returned an invalid csv path.");

    await expect(
      topOffPolygonDataset({
        csvPath: "C:/polygon/AAPL_1m.csv",
        timeframe: "1m",
      }),
    ).rejects.toThrow("Polygon top-off returned an invalid base dataset.");
  });

  it("rejects empty Polygon dataset paths for top-off, validate, and repair actions", async () => {
    await expect(topOffPolygonDataset({ csvPath: "   " })).rejects.toThrow(
      "Select a Polygon dataset first.",
    );
    await expect(validatePolygonDataset("   ")).rejects.toThrow(
      "Select a Polygon dataset first.",
    );
    await expect(repairPolygonDataset("   ")).rejects.toThrow(
      "Select a Polygon dataset first.",
    );
  });

  it("preserves native ticker and market-specific request shapes for Polygon downloads", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/BTCUSD_1m.csv",
        downloaded_bars: 15,
        used_cache_only: true,
        base_load: {
          id: "crypto-base",
          meta: { len: 15, timeframe: "1m", has_close: true },
          cache_hit: true,
          cache_path: "C:/cache/crypto.bin",
          load_source: "binary_cache",
        },
        id: "crypto-base",
        meta: {
          len: 15,
          timeframe: "1m",
          has_close: true,
        },
        cache_hit: true,
        cache_path: "C:/cache/crypto.bin",
        load_source: "binary_cache",
      })
      .mockResolvedValueOnce({
        csv_path: "C:/polygon/EURUSD_1m.csv",
        downloaded_bars: 12,
        used_cache_only: false,
        base_load: {
          id: "fx-base",
          meta: { len: 12, timeframe: "1m", has_close: true },
          cache_hit: false,
          cache_path: "C:/cache/fx.bin",
          load_source: "csv_parsed_and_cached",
        },
        id: "fx-base",
        meta: {
          len: 12,
          timeframe: "1m",
          has_close: true,
        },
        cache_hit: false,
        cache_path: "C:/cache/fx.bin",
        load_source: "csv_parsed_and_cached",
      });

    await downloadPolygonDataset({
      market: "crypto",
      symbol: "x:btcusd",
      from: "2024-01-01",
      to: "2024-01-02",
      adjusted: false,
      timeframe: "",
    });
    await downloadPolygonDataset({
      market: "fx",
      symbol: "eur/usd",
      from: "2024-01-01",
      to: "2024-01-02",
      adjusted: false,
      timeframe: "1m",
    });

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "polygon_download_1m_and_load",
      {
        req: {
          market: "crypto",
          symbol: "X:BTCUSD",
          from: "2024-01-01",
          to: "2024-01-02",
          adjusted: false,
          timeframe: "1m",
        },
      },
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      2,
      "polygon_download_1m_and_load",
      {
        req: {
          market: "fx",
          symbol: "EUR/USD",
          from: "2024-01-01",
          to: "2024-01-02",
          adjusted: false,
          timeframe: "1m",
        },
      },
    );
  });
});
