import { describe, expect, it } from "vitest";
import { buildValidationEvidenceSummary } from "./validationEvidenceSummary";

describe("validationEvidenceSummary", () => {
  it("returns a not-run summary when no validation result is present", () => {
    expect(
      buildValidationEvidenceSummary({
        validationReq: {
          backend: { backend: "Auto" },
          walk_forward: { window: 100 },
        },
        datasetMeta: { dataset_id: "dataset-1" },
      }),
    ).toMatchObject({
      status: "not_run",
      summaryLine: "Validation evidence not run.",
      requestedBackend: "Auto",
      reportState: "not_generated",
      datasetIdentity: "dataset-1",
    });
  });

  it("summarizes available and missing requested modules", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        backend: { backend: "Gpu" },
        walk_forward: { window: 200 },
        cscv_pbo: { slices: 16 },
        spa: { slices: 16 },
        trade_bootstrap: { simulations: 500 },
      },
      validationRes: {
        requested_backend: "Gpu",
        backend_used: "Gpu",
        backend_route: "GpuKernel",
        walk_forward: { windows: 4 },
        cscv_pbo: { pbo: 0.12 },
        spa: { skipped_reason: "insufficient slices" },
      },
      datasetMeta: { source_content_fingerprint: "sha256:abc" },
      reportPath: "C:/reports/validation.xlsx",
      replayClass: "exact_evidence_review",
    });

    expect(summary.status).toBe("incomplete");
    expect(summary.summaryLine).toBe(
      "Validation evidence incomplete; 2/4 requested modules available; route Gpu.",
    );
    expect(summary.reportState).toBe("generated");
    expect(summary.replayClass).toBe("exact_evidence_review");
    expect(summary.datasetIdentity).toBe("sha256:abc");
    expect(
      summary.modules.map((module) => [
        module.id,
        module.availability,
        module.detail,
      ]),
    ).toEqual([
      ["walk_forward", "available", "result present"],
      ["reopt_walk_forward", "not_requested", "not requested"],
      ["cscv_pbo", "available", "result present"],
      ["spa", "skipped", "insufficient slices"],
      ["monte_carlo", "unavailable", "requested but result missing"],
      ["cross_ticker", "not_requested", "not requested"],
    ]);
  });

  it("keeps optional module absence distinct from missing evidence", () => {
    const summary = buildValidationEvidenceSummary({
      validationReq: {
        backend: { backend: "CpuOnly" },
      },
      validationRes: {
        backend_used: "Cpu",
        backend_route: "CpuOnly",
      },
    });

    expect(summary.status).toBe("available");
    expect(summary.summaryLine).toBe(
      "Validation evidence available; no optional evidence modules requested; route Cpu.",
    );
    expect(
      summary.modules.every(
        (module) => module.availability === "not_requested",
      ),
    ).toBe(true);
  });
});
