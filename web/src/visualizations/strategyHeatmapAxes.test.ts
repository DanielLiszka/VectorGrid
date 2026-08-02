import { describe, expect, it } from "vitest";
import {
  isStandaloneStrategyHeatmap,
  resolveStrategyHeatmapAxes,
  STANDALONE_HEATMAP_3D_UNAVAILABLE_REASON,
} from "./strategyHeatmapAxes";

describe("strategyHeatmapAxes", () => {
  it.each([
    ["Asr", "Squeeze period", "Trend / ER / Z-score periods"],
    ["Vcc", "Squeeze / momentum periods", "Trend period"],
    ["Vemr", "Z-score / BB periods", "ATR period"],
    ["Rocb", "Channel period", "ATR period"],
    ["Vcet", "VIDYA short period", "VIDYA long / Chandelier periods"],
    ["Mkcs", "StdDev period", "Breakout lookback"],
    ["Kqsr", "Fast oscillator period floor", "Slow oscillator period floor"],
    ["Carm", "Fast indicator period floor", "Slow indicator period floor"],
    [
      "Kvdf",
      "FVG / fast DTI period floors",
      "Keltner / DevStop / DTI period floors",
    ],
    ["Cdmc", "DPO period floor", "Cycle / decycler period floors"],
  ])(
    "describes the %s grid using strategy-local parameter semantics",
    (strategyKind, fastAxisLabel, slowAxisLabel) => {
      const axes = resolveStrategyHeatmapAxes({ strategy_kind: strategyKind });

      expect(axes).toMatchObject({
        fastAxisLabel,
        slowAxisLabel,
        supports3dSlices: false,
        threeDimensionalUnavailableReason:
          STANDALONE_HEATMAP_3D_UNAVAILABLE_REASON,
      });
      expect(axes.fastAxisDescription).not.toHaveLength(0);
      expect(axes.slowAxisDescription).not.toHaveLength(0);
      expect(isStandaloneStrategyHeatmap(strategyKind)).toBe(true);
    },
  );

  it("honors VCC mapping flags and calls fully disconnected coordinates unmapped", () => {
    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Vcc",
        vcc: {
          map_fast_to_squeeze_period: false,
          map_fast_to_momentum_period: true,
          map_slow_to_trend_period: false,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "Momentum period",
      slowAxisLabel: "Slow grid (unmapped)",
      slowAxisDescription: expect.stringContaining(
        "does not alter any mapped strategy periods",
      ),
    });

    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Vcc",
        vcc: {
          map_fast_to_squeeze_period: false,
          map_fast_to_momentum_period: false,
          map_slow_to_trend_period: false,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "Fast grid (unmapped)",
      slowAxisLabel: "Slow grid (unmapped)",
    });
  });

  it("honors selective mappings for each configurable standalone family", () => {
    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Asr",
        asr: {
          map_fast_to_squeeze_period: false,
          map_slow_to_trend_period: false,
          map_slow_to_er_period: true,
          map_slow_to_z_period: false,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "Fast grid (unmapped)",
      slowAxisLabel: "ER period",
    });

    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Vemr",
        vemr: {
          map_fast_to_z_period: "false",
          map_fast_to_bb_period: true,
          map_slow_to_atr_period: 0,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "BB period",
      slowAxisLabel: "Slow grid (unmapped)",
    });

    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Rocb",
        rocb: {
          map_fast_to_channel_period: false,
          map_slow_to_atr_period: true,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "Fast grid (unmapped)",
      slowAxisLabel: "ATR period",
    });

    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Vcet",
        vcet: {
          map_fast_to_vidya_short_period: true,
          map_slow_to_vidya_long_period: false,
          map_slow_to_ce_period: true,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "VIDYA short period",
      slowAxisLabel: "Chandelier period",
    });

    expect(
      resolveStrategyHeatmapAxes({
        strategy_kind: "Mkcs",
        mkcs: {
          map_fast_to_stddev_period: false,
          map_slow_to_breakout_lookback: false,
        },
      }),
    ).toMatchObject({
      fastAxisLabel: "Fast grid (unmapped)",
      slowAxisLabel: "Slow grid (unmapped)",
    });
  });

  it("uses MA-length semantics and retains 3D support for double-MA families", () => {
    expect(
      resolveStrategyHeatmapAxes({
        strategyKind: "DynamicDoubleMa",
        fast_ma_types: ["alma"],
        slowMaType: "ema",
      }),
    ).toEqual({
      fastAxisLabel: "Fast ALMA length",
      slowAxisLabel: "Slow EMA length",
      fastAxisDescription: "Fast ALMA length used by each grid column.",
      slowAxisDescription: "Slow EMA length used by each grid row.",
      supports3dSlices: true,
      threeDimensionalUnavailableReason: "",
    });
    expect(isStandaloneStrategyHeatmap("DynamicDoubleMa")).toBe(false);
  });
});
