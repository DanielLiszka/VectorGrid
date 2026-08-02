import { describe, expect, it } from "vitest";
import {
  MAX_SAFE_VCET_SEARCH_SEED,
  buildVcetSearchConfigFromDraft,
  createDefaultVcetSearchDraft,
  normalizeVcetCandidateDesignMode,
  normalizeVcetSearchConfigRequest,
} from "./vcetSearchRequestBuilder";

describe("vcetSearchRequestBuilder", () => {
  it("matches the backend's deterministic default multidimensional design", () => {
    const result = buildVcetSearchConfigFromDraft(
      createDefaultVcetSearchDraft(),
    );

    expect(result.config).toEqual({
      budget: 2048,
      seed: 0,
      vidya_short_period: { min: 4, max: 48, step: 2 },
      vidya_long_period: { min: 12, max: 160, step: 4 },
      vidya_alpha: { min: 0.05, max: 0.8, quantum: 0.05 },
      ce_period: { min: 10, max: 80, step: 2 },
      ce_mult: { min: 1.5, max: 5, quantum: 0.25 },
      ce_use_close: { values: [false, true] },
      require_vidya_slope: { values: [false, true] },
      max_trade_bars: { min: 30, max: 480, step: 30 },
    });
    expect(result.validPeriodPairs).toBeGreaterThan(0n);
    expect(result.validSearchSpaceSize).toBeGreaterThan(2048n);
    expect(result.effectiveBudget).toBe(2048);
    expect(result.summary).toContain(
      "Deterministic Halton–Cranley–Patterson design",
    );
    expect(result.summary).toContain("seed 0");
  });

  it("keeps a requested budget above the finite space while reporting the effective count", () => {
    const draft = createDefaultVcetSearchDraft();
    Object.assign(draft, {
      budget: "10",
      vidyaShortMin: "6",
      vidyaShortMax: "6",
      vidyaShortStep: "1",
      vidyaLongMin: "24",
      vidyaLongMax: "24",
      vidyaLongStep: "1",
      vidyaAlphaMin: "0.2",
      vidyaAlphaMax: "0.2",
      vidyaAlphaQuantum: "0.1",
      cePeriodMin: "20",
      cePeriodMax: "20",
      cePeriodStep: "1",
      ceMultMin: "3",
      ceMultMax: "3",
      ceMultQuantum: "0.5",
      ceUseClose: "true",
      requireVidyaSlope: "false",
      maxTradeBarsMin: "120",
      maxTradeBarsMax: "120",
      maxTradeBarsStep: "1",
    });

    const result = buildVcetSearchConfigFromDraft(draft);
    expect(result.config.budget).toBe(10);
    expect(result.validSearchSpaceSize).toBe(1n);
    expect(result.effectiveBudget).toBe(1);
    expect(result.summary).toContain(
      "Deterministic exhaustive lexicographic design",
    );
  });

  it.each([
    ["zero budget", { budget: "0" }, "budget must be within"],
    ["unsafe seed", { seed: String(MAX_SAFE_VCET_SEARCH_SEED + 1) }, "seed must be within"],
    ["short below two", { vidyaShortMin: "1" }, "short period minimum"],
    ["unaligned integer range", { cePeriodMax: "81" }, "endpoints must align"],
    ["alpha above one", { vidyaAlphaMax: "1.1" }, "alpha maximum"],
    ["overprecision", { vidyaAlphaQuantum: "0.0000001" }, "within [0.000001"],
    ["unaligned float range", { ceMultMax: "5.1" }, "endpoints must align"],
    ["empty bool choice", { ceUseClose: "" }, "close-source choice"],
  ])("rejects %s", (_label, patch, message) => {
    expect(() =>
      buildVcetSearchConfigFromDraft(
        {
          ...createDefaultVcetSearchDraft(),
          ...patch,
        } as unknown as ReturnType<typeof createDefaultVcetSearchDraft>,
      ),
    ).toThrow(message);
  });

  it("rejects a period design with no long period above a short period", () => {
    expect(() =>
      buildVcetSearchConfigFromDraft({
        ...createDefaultVcetSearchDraft(),
        vidyaShortMin: "20",
        vidyaShortMax: "30",
        vidyaShortStep: "2",
        vidyaLongMin: "4",
        vidyaLongMax: "12",
        vidyaLongStep: "2",
      }),
    ).toThrow("no valid period pairs");
  });

  it("validates and canonicalizes an injected backend-shaped config", () => {
    const defaults = buildVcetSearchConfigFromDraft(
      createDefaultVcetSearchDraft(),
    ).config;
    const normalized = normalizeVcetSearchConfigRequest({
      ...defaults,
      budget: 32,
      seed: 42,
      ce_use_close: { values: [true, false, true] },
      require_vidya_slope: { values: [true] },
    });

    expect(normalized.budget).toBe(32);
    expect(normalized.seed).toBe(42);
    expect(normalized.ce_use_close.values).toEqual([false, true]);
    expect(normalized.require_vidya_slope.values).toEqual([true]);
  });

  it("rejects unknown injected fields instead of sending a backend-invalid request", () => {
    const defaults = buildVcetSearchConfigFromDraft(
      createDefaultVcetSearchDraft(),
    ).config;
    expect(() =>
      normalizeVcetSearchConfigRequest({ ...defaults, sequence: "halton" }),
    ).toThrow('unsupported field "sequence"');
    expect(() =>
      normalizeVcetSearchConfigRequest({
        ...defaults,
        ce_mult: { ...defaults.ce_mult, step: 0.25 },
      }),
    ).toThrow('unsupported field "step"');
  });

  it("normalizes the candidate-design mode fail-safe to the legacy grid", () => {
    expect(normalizeVcetCandidateDesignMode("halton")).toBe("halton");
    expect(normalizeVcetCandidateDesignMode("period_grid")).toBe(
      "period_grid",
    );
    expect(normalizeVcetCandidateDesignMode("unexpected")).toBe(
      "period_grid",
    );
  });
});
