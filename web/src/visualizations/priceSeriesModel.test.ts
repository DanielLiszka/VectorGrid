import { describe, expect, it } from "vitest";
import {
  normalizeChartTimeSec,
  normalizeCompositeIndicatorBars,
  normalizeIndexedPriceBars,
  normalizeMovingAverageIndicatorBars,
  normalizePriceBars,
  normalizeStrategyIndicatorBundle,
} from "./priceSeriesModel";

describe("priceSeriesModel", () => {
  it("normalizes second, millisecond, microsecond, and nanosecond timestamps", () => {
    expect(normalizeChartTimeSec(1_744_500_000)).toBe(1_744_500_000);
    expect(normalizeChartTimeSec(1_744_500_000_123)).toBe(1_744_500_000);
    expect(normalizeChartTimeSec(1_744_500_000_123_000)).toBe(1_744_500_000);
    expect(normalizeChartTimeSec(1_744_500_000_123_000_000)).toBe(
      1_744_500_000,
    );
    expect(Number.isNaN(normalizeChartTimeSec("bad"))).toBe(true);
  });

  it("normalizes, sorts, clamps wicks, and dedupes price bars", () => {
    const bars = normalizePriceBars([
      { t: 20, o: 10, h: 10.5, l: 9.5, c: 11 },
      { t: 10, o: 8, h: 8.2, l: 7.5, c: 7.8 },
      { t: 20, o: 12, h: 11, l: 13, c: 12.5 },
      { t: "bad", o: 1, h: 1, l: 1, c: 1 },
    ]);

    expect(bars).toEqual([
      { time: 10, open: 8, high: 8.2, low: 7.5, close: 7.8 },
      { time: 20, open: 12, high: 12.5, low: 12, close: 12.5 },
    ]);
  });

  it("normalizes indexed drill bars with index aliases", () => {
    const bars = normalizeIndexedPriceBars([
      {
        timestamp: 30,
        open: 20,
        high: 21,
        low: 19,
        close: 20.5,
        volume: "42",
        fromIndex: "5",
        to: "8",
      },
    ]);

    expect(bars[0]).toMatchObject({
      time: 30,
      volume: 42,
      fromIndex: 5,
      toIndex: 8,
    });
  });

  it("dedupes already-sorted bars without disturbing chronological order", () => {
    expect(
      normalizePriceBars([
        { t: 10, o: 8, h: 8.2, l: 7.5, c: 7.8 },
        { t: 20, o: 10, h: 10.5, l: 9.5, c: 11 },
        { t: 20, o: 12, h: 11, l: 13, c: 12.5 },
        { t: 30, o: 13, h: 14, l: 12, c: 13.5 },
      ]),
    ).toEqual([
      { time: 10, open: 8, high: 8.2, low: 7.5, close: 7.8 },
      { time: 20, open: 12, high: 12.5, low: 12, close: 12.5 },
      { time: 30, open: 13, high: 14, low: 12, close: 13.5 },
    ]);
  });

  it("normalizes moving average and composite indicator bars", () => {
    expect(
      normalizeMovingAverageIndicatorBars([
        { time: 3, fast: "10.5", slow: null },
        { time: 1, fast: "bad", slow: "20" },
        { time: 3, fast: "11.5", slow: "19.5" },
        { time: 2, fast: "bad", slow: "bad" },
      ]),
    ).toEqual([
      { time: 1, fast: null, slow: 20 },
      { time: 3, fast: 11.5, slow: 19.5 },
    ]);

    expect(
      normalizeCompositeIndicatorBars([
        { time: 2, halftrend: "1", macz_hist: "-0.2", ott_line: "100.8" },
        { time: 1, halftrend: null, macz: "0.3" },
        { time: 2, halftrend: "0", macz_hist: "0.4", ott_line: "101.2" },
      ]),
    ).toEqual([
      { time: 1, halftrend: null, macz: 0.3, ottLine: null, ottPrice: null },
      { time: 2, halftrend: 0, macz: 0.4, ottLine: 101.2, ottPrice: null },
    ]);
  });

  it("normalizes standalone strategy series in shared-bar alignment", () => {
    const bundle = normalizeStrategyIndicatorBundle({
      strategy_kind: "  Asr  ",
      aggregation: "last_finite",
      bars: [
        { time: 30, from_index: 2, to_index: 2 },
        { time: 10, from_index: 0, to_index: 0 },
        { time: 20, from_index: 1, to_index: 1 },
        { time: 20, from_index: 10, to_index: 12 },
        { time: "invalid", from_index: 4, to_index: 4 },
      ],
      series: [
        {
          id: " ASR_ZSCORE ",
          label: "Z-score",
          kind: "Line",
          pane_id: " ASR_ZSCORE ",
          scale_id: " ASR_ZSCORE ",
          unit: "zscore",
          default_visible: true,
          values: [3, 1, 2, 2.5, 999],
          reference_lines: [
            { value: -1.5, label: "Long entry" },
            { value: 1.5, label: "Short entry" },
          ],
        },
        {
          id: "asr_optional",
          label: "Optional diagnostic",
          kind: "histogram",
          pane_id: "asr_diagnostic",
          scale_id: "asr_state",
          unit: "state",
          default_visible: false,
          values: [null, null, null, null, null],
        },
      ],
    });

    expect(bundle).toMatchObject({
      strategyKind: "Asr",
      aggregation: "last_finite",
      bars: [
        { time: 10, fromIndex: 0, toIndex: 0 },
        { time: 20, fromIndex: 10, toIndex: 12 },
        { time: 30, fromIndex: 2, toIndex: 2 },
      ],
    });
    expect(bundle?.series[0]).toEqual({
      id: "asr_zscore",
      label: "Z-score",
      kind: "line",
      paneId: "asr_zscore",
      scaleId: "asr_zscore",
      unit: "z_score",
      defaultVisible: true,
      data: [
        { time: 10, value: 1 },
        { time: 20, value: 2.5 },
        { time: 30, value: 3 },
      ],
      finitePointCount: 3,
      referenceLines: [
        { value: -1.5, label: "Long entry" },
        { value: 1.5, label: "Short entry" },
      ],
    });
    expect(bundle?.series[1]).toMatchObject({
      id: "asr_optional",
      defaultVisible: false,
      finitePointCount: 0,
      data: [{ time: 10 }, { time: 20 }, { time: 30 }],
      referenceLines: [],
    });
  });

  it("omits malformed individual strategy series without disturbing valid siblings", () => {
    const baseSeries = {
      label: "Valid",
      kind: "line",
      pane_id: "price",
      scale_id: "price",
      unit: "price",
      default_visible: true,
      values: [100, null],
    };
    const bundle = normalizeStrategyIndicatorBundle({
      strategy_kind: "Rocb",
      aggregation: "last_finite",
      bars: [
        { time: 10, from_index: 0, to_index: 0 },
        { time: 20, from_index: 1, to_index: 1 },
      ],
      series: [
        { ...baseSeries, id: "valid" },
        { ...baseSeries, id: "bad_kind", kind: "area" },
        { ...baseSeries, id: "bad_length", values: [100] },
        { ...baseSeries, id: "bad_value", values: [100, "101"] },
        {
          ...baseSeries,
          id: "bad_reference",
          reference_lines: [{ value: Number.NaN, label: "Invalid" }],
        },
        { ...baseSeries, id: "duplicate" },
        { ...baseSeries, id: " DUPLICATE " },
      ],
    });

    expect(bundle?.series).toHaveLength(1);
    expect(bundle?.series[0]).toMatchObject({
      id: "valid",
      finitePointCount: 1,
      data: [{ time: 10, value: 100 }, { time: 20 }],
    });
  });

  it("accepts camel-case field aliases while preserving strict wire values", () => {
    const bundle = normalizeStrategyIndicatorBundle({
      strategyKind: "Vcet",
      aggregation: "last_finite",
      bars: [
        {
          time: 1_744_500_000_123,
          fromIndex: 4,
          toIndex: 7,
        },
      ],
      series: [
        {
          id: "vcet_vidya",
          label: "VIDYA",
          kind: "line",
          paneId: "price",
          scaleId: "price",
          unit: "price",
          defaultVisible: true,
          values: [101.25],
          referenceLines: [{ value: 100, label: "Anchor" }],
        },
      ],
    });

    expect(bundle?.bars).toEqual([
      {
        time: 1_744_500_000,
        fromIndex: 4,
        toIndex: 7,
      },
    ]);
    expect(bundle?.series[0]?.referenceLines).toEqual([
      { value: 100, label: "Anchor" },
    ]);
  });

  it("rejects malformed bundle framing or a bundle with no usable series", () => {
    const validSeries = {
      id: "series",
      label: "Series",
      kind: "line",
      pane_id: "price",
      scale_id: "price",
      unit: "price",
      default_visible: true,
      values: [1],
    };
    expect(
      normalizeStrategyIndicatorBundle({
        strategy_kind: "Asr",
        aggregation: "unsupported",
        bars: [{ time: 1, from_index: 0, to_index: 0 }],
        series: [validSeries],
      }),
    ).toBeNull();
    expect(
      normalizeStrategyIndicatorBundle({
        strategy_kind: "Asr",
        aggregation: "last_finite",
        bars: [{ time: "invalid", from_index: 0, to_index: 0 }],
        series: [validSeries],
      }),
    ).toBeNull();
    expect(
      normalizeStrategyIndicatorBundle({
        strategy_kind: "Asr",
        aggregation: "last_finite",
        bars: [{ time: 1, from_index: 0, to_index: 0 }],
        series: [{ ...validSeries, kind: "unsupported" }],
      }),
    ).toBeNull();
  });
});
