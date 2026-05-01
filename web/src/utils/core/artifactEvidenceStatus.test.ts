import { describe, expect, it } from "vitest";
import {
  artifactCompatibilityAllowsReplay,
  artifactCompatibilityLabel,
  artifactCompatibilityMetaToken,
  canOpenPersistedReport,
  formatReportStateLabel,
  formatReportStateMetaToken,
  formatReportStateShortLabel,
  replayClassLabel,
  replayClassSummary,
  normalizeArtifactCompatibility,
  resolveReportState,
} from "./artifactEvidenceStatus";

describe("artifactEvidenceStatus", () => {
  it("resolves saved-run report states without treating unknown as available", () => {
    expect(resolveReportState("", true)).toBe("not_exported");
    expect(resolveReportState("C:/runs/aapl.xlsx", true)).toBe("available");
    expect(resolveReportState("C:/runs/aapl.xlsx", false)).toBe(
      "missing_on_disk",
    );
    expect(resolveReportState("C:/runs/aapl.xlsx", null)).toBe("unknown");

    expect(formatReportStateLabel("available")).toBe("XLSX available");
    expect(formatReportStateLabel("missing_on_disk")).toBe("Missing on disk");
    expect(formatReportStateLabel("unknown")).toBe("Availability unverified");
    expect(formatReportStateLabel("not_exported")).toBe("Not exported");
    expect(formatReportStateShortLabel("unknown")).toBe("xlsx unverified");
    expect(formatReportStateMetaToken("unknown")).toBe("xlsx=unknown");
  });

  it("allows manual open attempts only when a path exists and is not known missing", () => {
    expect(canOpenPersistedReport("", null)).toBe(false);
    expect(canOpenPersistedReport("C:/runs/aapl.xlsx", true)).toBe(true);
    expect(canOpenPersistedReport("C:/runs/aapl.xlsx", null)).toBe(true);
    expect(canOpenPersistedReport("C:/runs/aapl.xlsx", false)).toBe(false);
  });

  it("formats replay classes and reasons", () => {
    expect(replayClassLabel("exact_evidence_review")).toBe(
      "Exact evidence review",
    );
    expect(
      replayClassSummary({
        replayClass: "exact_evidence_review",
        replayReasons: ["saved_run_result_payload_present"],
      }),
    ).toBe("Exact evidence review - saved run result payload present");
    expect(replayClassSummary({ replayClass: "", replayReasons: [] })).toBe(
      "-",
    );
  });

  it("normalizes artifact schema compatibility and blocks future replay", () => {
    const legacy = normalizeArtifactCompatibility(null);
    expect(legacy.status).toBe("unknown");
    expect(artifactCompatibilityAllowsReplay(legacy)).toBe(true);

    const supported = normalizeArtifactCompatibility({
      artifact_schema_version: 1,
      status: "supported",
      replay_blocked: false,
    });
    expect(artifactCompatibilityLabel(supported)).toBe("Schema v1 supported");
    expect(artifactCompatibilityMetaToken(supported)).toBe("schema=v1");
    expect(artifactCompatibilityAllowsReplay(supported)).toBe(true);

    const future = normalizeArtifactCompatibility({
      artifactSchemaVersion: 2,
      status: "future_unsupported",
      reason: "newer than supported",
    });
    expect(artifactCompatibilityLabel(future)).toBe("Schema v2 review-only");
    expect(artifactCompatibilityMetaToken(future)).toBe("schema=review-only");
    expect(artifactCompatibilityAllowsReplay(future)).toBe(false);
  });
});
