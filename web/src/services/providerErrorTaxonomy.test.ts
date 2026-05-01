import { describe, expect, it } from "vitest";
import {
  classifyPolygonProviderError,
  formatPolygonProviderErrorMessage,
} from "./providerErrorTaxonomy";

describe("providerErrorTaxonomy", () => {
  it("classifies common Polygon provider failures", () => {
    expect(
      classifyPolygonProviderError("Polygon API request failed 429").kind,
    ).toBe("rate_limited");
    expect(
      classifyPolygonProviderError("HTTP status: 403 not entitled").kind,
    ).toBe("entitlement_or_plan_limit");
    expect(
      classifyPolygonProviderError("api key rejected by provider").kind,
    ).toBe("auth_missing_or_invalid");
    expect(
      classifyPolygonProviderError("Polygon did not recognize ticker").kind,
    ).toBe("symbol_not_found");
    expect(
      classifyPolygonProviderError("Polygon returned no aggregate bars").kind,
    ).toBe("empty_valid_window");
    expect(
      classifyPolygonProviderError("Polygon returned an unexpected payload").kind,
    ).toBe("provider_schema_changed");
    expect(classifyPolygonProviderError("request timed out").kind).toBe(
      "network_transient",
    );
    expect(classifyPolygonProviderError("cache corrupt at row 12").kind).toBe(
      "local_cache_corrupt",
    );
  });

  it("formats guidance while preserving unknown redacted text", () => {
    expect(formatPolygonProviderErrorMessage("HTTP 429", "fallback")).toContain(
      "rate limit reached",
    );
    expect(formatPolygonProviderErrorMessage("custom [redacted]", "fallback")).toBe(
      "custom [redacted]",
    );
    expect(formatPolygonProviderErrorMessage("", "fallback")).toBe("fallback");
  });
});
