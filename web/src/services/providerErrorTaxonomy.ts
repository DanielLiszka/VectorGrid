export type PolygonProviderErrorKind =
  | "auth_missing_or_invalid"
  | "entitlement_or_plan_limit"
  | "rate_limited"
  | "market_closed_or_no_session"
  | "empty_valid_window"
  | "symbol_not_found"
  | "provider_schema_changed"
  | "network_transient"
  | "local_cache_corrupt"
  | "unknown_provider_error";

export interface PolygonProviderErrorClassification {
  kind: PolygonProviderErrorKind;
  rawText: string;
}

function normalizeProviderErrorText(value: unknown): string {
  return String(value || "").trim();
}

export function classifyPolygonProviderError(
  raw: unknown,
): PolygonProviderErrorClassification {
  const rawText = normalizeProviderErrorText(raw);
  const lower = rawText.toLowerCase();

  if (
    lower.includes("rate limit") ||
    lower.includes("too many requests") ||
    lower.includes("429")
  ) {
    return { kind: "rate_limited", rawText };
  }

  if (
    lower.includes("not entitled") ||
    lower.includes("entitlement") ||
    lower.includes("subscription") ||
    lower.includes("plan limit") ||
    lower.includes("403")
  ) {
    return { kind: "entitlement_or_plan_limit", rawText };
  }

  if (
    lower.includes("api key") &&
    (lower.includes("missing") ||
      lower.includes("not configured") ||
      lower.includes("invalid") ||
      lower.includes("rejected"))
  ) {
    return { kind: "auth_missing_or_invalid", rawText };
  }

  if (lower.includes("401") || lower.includes("unauthorized")) {
    return { kind: "auth_missing_or_invalid", rawText };
  }

  if (
    lower.includes("did not recognize") ||
    lower.includes("ticker not found") ||
    lower.includes("symbol not found") ||
    lower.includes("unknown ticker") ||
    lower.includes("404")
  ) {
    return { kind: "symbol_not_found", rawText };
  }

  if (
    lower.includes("no aggregate bars") ||
    lower.includes("no bars") ||
    lower.includes("no data for that range") ||
    lower.includes("empty valid window")
  ) {
    return { kind: "empty_valid_window", rawText };
  }

  if (
    lower.includes("market closed") ||
    lower.includes("no session") ||
    lower.includes("eligible trades")
  ) {
    return { kind: "market_closed_or_no_session", rawText };
  }

  if (
    lower.includes("unexpected payload") ||
    lower.includes("unexpected response") ||
    lower.includes("returned an unexpected") ||
    lower.includes("schema")
  ) {
    return { kind: "provider_schema_changed", rawText };
  }

  if (
    lower.includes("cache corrupt") ||
    lower.includes("corrupt cache") ||
    lower.includes("parse error") ||
    lower.includes("out of order")
  ) {
    return { kind: "local_cache_corrupt", rawText };
  }

  if (
    lower.includes("server error") ||
    lower.includes("temporary server error") ||
    lower.includes("temporarily unavailable") ||
    lower.includes("timeout") ||
    lower.includes("timed out") ||
    lower.includes("network") ||
    lower.includes("connection") ||
    lower.includes("502") ||
    lower.includes("503") ||
    lower.includes("504")
  ) {
    return { kind: "network_transient", rawText };
  }

  return { kind: "unknown_provider_error", rawText };
}

export function polygonProviderErrorGuidance(
  kind: PolygonProviderErrorKind,
): string {
  switch (kind) {
    case "auth_missing_or_invalid":
      return "Polygon API authentication failed. Check the saved API key and credential storage state before retrying.";
    case "entitlement_or_plan_limit":
      return "Polygon key or plan does not allow this request. Check subscription entitlements, symbol, endpoint, and requested history.";
    case "rate_limited":
      return "Polygon rate limit reached. Wait and retry, reduce the date range, or load an already cached dataset.";
    case "market_closed_or_no_session":
      return "Polygon found no eligible market session for that request. Check the market calendar, ticker, and selected interval.";
    case "empty_valid_window":
      return "Polygon returned no bars for that request. Check the ticker, market, date range, subscription recency, and whether the interval had eligible trades.";
    case "symbol_not_found":
      return "Polygon did not recognize the requested ticker. Check symbol format, market, and whether the instrument is supported by the selected endpoint.";
    case "provider_schema_changed":
      return "Polygon returned an unexpected response shape. Refresh provider data later and keep the raw error for schema-contract investigation.";
    case "network_transient":
      return "Polygon returned a temporary network or server error. Retry later or reduce the request size.";
    case "local_cache_corrupt":
      return "Local Polygon cache data appears inconsistent. Validate or repair the cached dataset before retrying.";
    case "unknown_provider_error":
    default:
      return "";
  }
}

export function formatPolygonProviderErrorMessage(
  raw: unknown,
  fallback: string,
): string {
  const classification = classifyPolygonProviderError(raw);
  if (
    classification.rawText.toLowerCase().includes("to date must be >= from date")
  ) {
    return "Polygon end date must be on or after the start date.";
  }
  if (
    classification.rawText.toLowerCase().includes("bad request") ||
    classification.rawText.toLowerCase().includes("400")
  ) {
    return "Polygon rejected the request parameters. Check symbol format, market, date range, and timeframe.";
  }
  const guidance = polygonProviderErrorGuidance(classification.kind);
  if (guidance) return guidance;
  return classification.rawText || fallback;
}
