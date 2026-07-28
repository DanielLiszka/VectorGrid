#!/usr/bin/env node

import { createHash, randomUUID } from "node:crypto";
import { lstat, mkdir, rename, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const POLYGON_PRODUCTION_BASE_URL = "https://api.polygon.io";
export const ALPACA_PAPER_BASE_URL = "https://paper-api.alpaca.markets";

const DEFAULT_TIMEOUT_MS = 12_000;
const MAX_RESPONSE_BYTES = 512 * 1024;
const POLYGON_KEY_ENV_NAMES = ["POLYGON_API_KEY", "POLYGON_KEY"];
const ALPACA_KEY_ENV_NAMES = [
  "APCA_API_KEY_ID",
  "ALPACA_API_KEY",
  "ALPACA_KEY_ID",
];
const ALPACA_SECRET_ENV_NAMES = [
  "APCA_API_SECRET_KEY",
  "ALPACA_API_SECRET",
  "ALPACA_SECRET_KEY",
];
const NETWORK_ERROR_CODES = new Set([
  "EAI_AGAIN",
  "ECONNREFUSED",
  "ECONNRESET",
  "EHOSTUNREACH",
  "ENETUNREACH",
  "ENOTFOUND",
  "ETIMEDOUT",
  "UND_ERR_CONNECT_TIMEOUT",
  "UND_ERR_HEADERS_TIMEOUT",
  "UND_ERR_SOCKET",
]);
const SAFE_SYMBOL = /^[A-Z][A-Z0-9.-]{0,14}$/u;

function nowIso(now = () => new Date()) {
  return now().toISOString();
}

function safeMessage(value) {
  return String(value ?? "")
    .replaceAll(/[\r\n\t]+/gu, " ")
    .replaceAll(/\s{2,}/gu, " ")
    .trim()
    .slice(0, 320);
}

function sha256Text(value) {
  return createHash("sha256").update(String(value), "utf8").digest("hex");
}

function hashedRequestId(response, payload) {
  const requestId =
    response.headers.get("x-request-id") ??
    (typeof payload?.request_id === "string" ? payload.request_id : "");
  return requestId.trim() ? sha256Text(requestId.trim()) : null;
}

function isLoopbackHostname(hostname) {
  const normalized = hostname.toLowerCase();
  return (
    normalized === "localhost" ||
    normalized === "127.0.0.1" ||
    normalized === "[::1]" ||
    normalized === "::1"
  );
}

export function validateProviderBaseUrl(
  provider,
  rawBaseUrl,
  { allowLoopbackMocks = false } = {},
) {
  let url;
  try {
    url = new URL(rawBaseUrl);
  } catch {
    throw new Error(`${provider} base URL is invalid.`);
  }

  if (url.username || url.password || url.search || url.hash) {
    throw new Error(
      `${provider} base URL must not include credentials, query parameters, or a fragment.`,
    );
  }
  if (url.pathname !== "/" && url.pathname !== "") {
    throw new Error(`${provider} base URL must not include a path.`);
  }

  if (allowLoopbackMocks && isLoopbackHostname(url.hostname)) {
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      throw new Error(`${provider} loopback mock URL must use HTTP or HTTPS.`);
    }
    return url;
  }

  if (url.protocol !== "https:" || url.port) {
    throw new Error(
      `${provider} provider URL must use HTTPS with its default port.`,
    );
  }

  const expectedHost =
    provider === "polygon" ? "api.polygon.io" : "paper-api.alpaca.markets";
  if (url.hostname.toLowerCase() !== expectedHost) {
    const environment =
      provider === "alpaca"
        ? "the Alpaca paper host; live-broker hosts are forbidden"
        : "the pinned Polygon API host";
    throw new Error(`${provider} preflight is restricted to ${environment}.`);
  }
  return url;
}

function normalizeCredential(raw, label) {
  const value = String(raw ?? "").trim();
  if (!value) {
    return "";
  }
  if (
    value.length < 8 ||
    value.length > 8192 ||
    /[\u0000-\u001f\u007f]/u.test(value)
  ) {
    throw new Error(
      `${label} is implausibly short, contains unsupported control characters, or is too long.`,
    );
  }
  return value;
}

function resolveCredentialAliases(env, names, label) {
  const populated = names
    .map((name) => [name, normalizeCredential(env[name], label)])
    .filter(([, value]) => value);
  const distinct = new Set(populated.map(([, value]) => value));
  if (distinct.size > 1) {
    throw new Error(
      `${label} is configured with conflicting values across supported environment variables.`,
    );
  }
  return {
    value: populated[0]?.[1] ?? "",
    sources: populated.map(([name]) => name),
  };
}

export function resolveProviderCredentials(env = process.env) {
  const polygon = resolveCredentialAliases(
    env,
    POLYGON_KEY_ENV_NAMES,
    "Polygon API key",
  );
  const alpacaKey = resolveCredentialAliases(
    env,
    ALPACA_KEY_ENV_NAMES,
    "Alpaca API key ID",
  );
  const alpacaSecret = resolveCredentialAliases(
    env,
    ALPACA_SECRET_ENV_NAMES,
    "Alpaca API secret",
  );

  return {
    polygon: {
      apiKey: polygon.value,
      sources: polygon.sources,
    },
    alpaca: {
      apiKey: alpacaKey.value,
      apiSecret: alpacaSecret.value,
      keySources: alpacaKey.sources,
      secretSources: alpacaSecret.sources,
    },
  };
}

function credentialPresence(provider, credentials) {
  if (provider === "polygon") {
    return {
      api_key_present: Boolean(credentials.apiKey),
      source_names: [...credentials.sources],
    };
  }
  return {
    api_key_present: Boolean(credentials.apiKey),
    api_secret_present: Boolean(credentials.apiSecret),
    key_source_names: [...credentials.keySources],
    secret_source_names: [...credentials.secretSources],
  };
}

function credentialState(provider, credentials) {
  if (provider === "polygon") {
    return credentials.apiKey ? "complete" : "absent";
  }
  if (Boolean(credentials.apiKey) !== Boolean(credentials.apiSecret)) {
    return "incomplete";
  }
  return credentials.apiKey ? "complete" : "absent";
}

function providerResultBase(provider, credentials, environment, host) {
  return {
    provider,
    environment,
    endpoint_host: host,
    status: "skipped",
    reason_code: "not_run",
    message: "",
    credentials: credentialPresence(provider, credentials),
    network_attempted: false,
    request_method: "GET",
    checks: [],
  };
}

function isNetworkUnavailable(error) {
  if (error?.name === "AbortError" || error?.name === "TimeoutError") {
    return true;
  }
  let current = error;
  for (let depth = 0; current && depth < 5; depth += 1) {
    if (NETWORK_ERROR_CODES.has(String(current.code ?? "").toUpperCase())) {
      return true;
    }
    current = current.cause;
  }
  return false;
}

function httpFailureReason(status) {
  if (status === 401 || status === 403) {
    return "authentication_or_entitlement_rejected";
  }
  if (status === 429) {
    return "rate_limited";
  }
  if (status >= 500) {
    return "provider_error";
  }
  return "unexpected_http_status";
}

async function readBoundedJson(response) {
  const advertisedLength = Number(
    response.headers.get("content-length") ?? "0",
  );
  if (
    Number.isFinite(advertisedLength) &&
    advertisedLength > MAX_RESPONSE_BYTES
  ) {
    await response.body?.cancel().catch(() => {});
    throw new Error("provider response exceeded the preflight size limit");
  }
  if (!response.body) {
    throw new Error("provider response did not contain a JSON body");
  }
  const reader = response.body.getReader();
  const chunks = [];
  let totalBytes = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      totalBytes += value.byteLength;
      if (totalBytes > MAX_RESPONSE_BYTES) {
        await reader.cancel();
        throw new Error("provider response exceeded the preflight size limit");
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const bytes = new Uint8Array(totalBytes);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  const contentType = response.headers.get("content-type") ?? "";
  if (!contentType.toLowerCase().includes("json")) {
    throw new Error("provider response was not JSON");
  }
  try {
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    throw new Error("provider response contained invalid JSON");
  }
}

async function fetchJson(
  url,
  { headers = {}, timeoutMs, fetchImpl, provider, endpointLabel },
) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  const startedAt = performance.now();
  try {
    const response = await fetchImpl(url, {
      method: "GET",
      headers: {
        accept: "application/json",
        "user-agent": "VectorGrid-Provider-Sandbox-QA/1",
        ...headers,
      },
      redirect: "error",
      signal: controller.signal,
    });
    const durationMs = Math.max(0, Math.round(performance.now() - startedAt));
    if (!response.ok) {
      await response.body?.cancel().catch(() => {});
      const failure = new Error(
        `${provider} ${endpointLabel} returned HTTP ${response.status}.`,
      );
      failure.reasonCode = httpFailureReason(response.status);
      failure.httpStatus = response.status;
      failure.durationMs = durationMs;
      throw failure;
    }
    const payload = await readBoundedJson(response);
    return { response, payload, durationMs };
  } catch (error) {
    if (isNetworkUnavailable(error)) {
      const unavailable = new Error(
        `${provider} network connectivity was unavailable or timed out.`,
      );
      unavailable.reasonCode = "network_unavailable";
      unavailable.networkUnavailable = true;
      throw unavailable;
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

function failureResult(result, error) {
  const networkUnavailable = Boolean(error?.networkUnavailable);
  return {
    ...result,
    status: networkUnavailable ? "skipped" : "failed",
    reason_code: String(error?.reasonCode ?? "preflight_failed"),
    message: safeMessage(
      error?.message ?? `${result.provider} preflight failed without details.`,
    ),
    ...(Number.isInteger(error?.httpStatus)
      ? { http_status: error.httpStatus }
      : {}),
  };
}

async function runPolygonPreflight({
  credentials,
  allowNetwork,
  baseUrl,
  allowLoopbackMocks,
  timeoutMs,
  symbol,
  fetchImpl,
}) {
  const validatedBase = validateProviderBaseUrl("polygon", baseUrl, {
    allowLoopbackMocks,
  });
  let result = providerResultBase(
    "polygon",
    credentials,
    "market-data-read-only",
    validatedBase.host,
  );
  const state = credentialState("polygon", credentials);
  if (state === "absent") {
    return {
      ...result,
      reason_code: "credentials_absent",
      message:
        "Polygon preflight skipped because no supported API-key environment variable is set.",
    };
  }
  if (!allowNetwork) {
    return {
      ...result,
      reason_code: "network_opt_in_required",
      message:
        "Polygon preflight skipped; rerun with the explicit credentialed-network opt-in.",
    };
  }

  const url = new URL("/v3/reference/tickers", validatedBase);
  url.searchParams.set("ticker", symbol);
  url.searchParams.set("active", "true");
  url.searchParams.set("limit", "1");
  url.searchParams.set("apiKey", credentials.apiKey);
  result = {
    ...result,
    network_attempted: true,
    endpoint_path: "/v3/reference/tickers",
  };
  try {
    const { response, payload, durationMs } = await fetchJson(url, {
      timeoutMs,
      fetchImpl,
      provider: "Polygon",
      endpointLabel: "reference-ticker request",
    });
    if (
      payload?.status &&
      !["OK", "DELAYED"].includes(String(payload.status).toUpperCase())
    ) {
      const error = new Error(
        "Polygon returned a non-success payload status for the read-only reference request.",
      );
      error.reasonCode = "unexpected_payload";
      throw error;
    }
    const results = Array.isArray(payload?.results) ? payload.results : [];
    const matched = results.some(
      (entry) =>
        String(entry?.ticker ?? "")
          .trim()
          .toUpperCase() === symbol.toUpperCase(),
    );
    if (!matched) {
      const error = new Error(
        "Polygon reference response did not contain the requested ticker.",
      );
      error.reasonCode = "unexpected_payload";
      throw error;
    }
    return {
      ...result,
      status: "passed",
      reason_code: "read_only_connectivity_verified",
      message:
        "Polygon credentials and the pinned read-only reference endpoint were verified.",
      http_status: response.status,
      duration_ms: durationMs,
      provider_request_id_sha256: hashedRequestId(response, payload),
      result_count: results.length,
      checks: [
        "pinned_https_host",
        "get_only",
        "reference_ticker_json",
        "requested_ticker_present",
      ],
    };
  } catch (error) {
    return failureResult(result, error);
  }
}

function validateAlpacaAccount(payload) {
  const status = String(payload?.status ?? "")
    .trim()
    .toUpperCase();
  if (status !== "ACTIVE") {
    const error = new Error(
      "Alpaca paper account did not report ACTIVE status.",
    );
    error.reasonCode = "paper_account_not_tradeable";
    throw error;
  }
  if (payload?.trading_blocked === true || payload?.account_blocked === true) {
    const error = new Error("Alpaca paper account is blocked from trading.");
    error.reasonCode = "paper_account_not_tradeable";
    throw error;
  }
  return {
    account_status: status,
    trading_blocked: payload?.trading_blocked === true,
    account_blocked: payload?.account_blocked === true,
  };
}

function validateAlpacaClock(payload) {
  if (typeof payload?.is_open !== "boolean") {
    const error = new Error(
      "Alpaca paper clock response did not include a boolean is_open field.",
    );
    error.reasonCode = "unexpected_payload";
    throw error;
  }
  return {
    market_open: payload.is_open,
    next_open_present: Boolean(String(payload?.next_open ?? "").trim()),
    next_close_present: Boolean(String(payload?.next_close ?? "").trim()),
  };
}

async function runAlpacaPreflight({
  credentials,
  allowNetwork,
  baseUrl,
  allowLoopbackMocks,
  timeoutMs,
  fetchImpl,
}) {
  const validatedBase = validateProviderBaseUrl("alpaca", baseUrl, {
    allowLoopbackMocks,
  });
  let result = providerResultBase(
    "alpaca",
    credentials,
    "paper-only",
    validatedBase.host,
  );
  const state = credentialState("alpaca", credentials);
  if (state === "incomplete") {
    return {
      ...result,
      status: "failed",
      reason_code: "credentials_incomplete",
      message:
        "Alpaca paper preflight requires both API key ID and API secret; only one is present.",
    };
  }
  if (state === "absent") {
    return {
      ...result,
      reason_code: "credentials_absent",
      message:
        "Alpaca paper preflight skipped because no supported credential pair is set.",
    };
  }
  if (!allowNetwork) {
    return {
      ...result,
      reason_code: "network_opt_in_required",
      message:
        "Alpaca paper preflight skipped; rerun with the explicit credentialed-network opt-in.",
    };
  }

  const headers = {
    "APCA-API-KEY-ID": credentials.apiKey,
    "APCA-API-SECRET-KEY": credentials.apiSecret,
  };
  const requests = [
    {
      label: "account",
      path: "/v2/account",
      validate: validateAlpacaAccount,
    },
    {
      label: "clock",
      path: "/v2/clock",
      validate: validateAlpacaClock,
    },
    {
      label: "order-reconciliation snapshot",
      path: "/v2/orders?status=all&limit=1&direction=desc&nested=true",
      validate(payload) {
        if (!Array.isArray(payload)) {
          const error = new Error(
            "Alpaca paper order snapshot response was not an array.",
          );
          error.reasonCode = "unexpected_payload";
          throw error;
        }
        return { returned_order_count: payload.length };
      },
    },
  ];
  result = {
    ...result,
    network_attempted: true,
    endpoint_paths: ["/v2/account", "/v2/clock", "/v2/orders"],
  };
  const endpointEvidence = [];
  const safeSummary = {};
  try {
    for (const request of requests) {
      const url = new URL(request.path, validatedBase);
      const { response, payload, durationMs } = await fetchJson(url, {
        headers,
        timeoutMs,
        fetchImpl,
        provider: "Alpaca paper",
        endpointLabel: request.label,
      });
      Object.assign(safeSummary, request.validate(payload));
      endpointEvidence.push({
        label: request.label,
        method: "GET",
        path: url.pathname,
        http_status: response.status,
        duration_ms: durationMs,
        provider_request_id_sha256: hashedRequestId(response, payload),
      });
    }
    return {
      ...result,
      status: "passed",
      reason_code: "paper_read_only_connectivity_verified",
      message:
        "Alpaca paper credentials, account/clock reads, and a read-only order reconciliation snapshot were verified.",
      endpoint_evidence: endpointEvidence,
      safe_account_summary: safeSummary,
      checks: [
        "pinned_paper_https_host",
        "get_only",
        "active_unblocked_paper_account",
        "clock_json",
        "read_only_order_snapshot",
      ],
    };
  } catch (error) {
    return failureResult(
      {
        ...result,
        endpoint_evidence: endpointEvidence,
      },
      error,
    );
  }
}

function selectedProviders(raw) {
  const normalized = String(raw ?? "all")
    .trim()
    .toLowerCase();
  if (normalized === "all") {
    return ["polygon", "alpaca"];
  }
  if (normalized === "polygon" || normalized === "alpaca") {
    return [normalized];
  }
  throw new Error("providers must be one of: all, polygon, alpaca.");
}

function summaryStatus(providerResults, requireEvidence) {
  if (providerResults.some((item) => item.status === "failed")) {
    return "failed";
  }
  if (
    requireEvidence &&
    providerResults.some((item) => item.status !== "passed")
  ) {
    return "failed";
  }
  return providerResults.some((item) => item.status === "passed")
    ? "passed"
    : "skipped";
}

function assertEvidenceContainsNoSecrets(summary, secretValues) {
  const serialized = JSON.stringify(summary);
  for (const secret of secretValues) {
    if (secret && serialized.includes(secret)) {
      throw new Error(
        "Refusing to persist provider evidence because a credential value was detected.",
      );
    }
  }
}

export async function runProviderSandboxPreflight({
  providers = "all",
  allowNetwork = false,
  requireEvidence = false,
  timeoutMs = DEFAULT_TIMEOUT_MS,
  polygonSymbol = "AAPL",
  polygonBaseUrl = POLYGON_PRODUCTION_BASE_URL,
  alpacaPaperBaseUrl = ALPACA_PAPER_BASE_URL,
  allowLoopbackMocks = false,
  env = process.env,
  fetchImpl = globalThis.fetch,
  now = () => new Date(),
} = {}) {
  if (typeof fetchImpl !== "function") {
    throw new Error(
      "This preflight requires a Node runtime with fetch support.",
    );
  }
  if (!Number.isInteger(timeoutMs) || timeoutMs < 250 || timeoutMs > 120_000) {
    throw new Error("timeoutMs must be an integer between 250 and 120000.");
  }
  const symbol = String(polygonSymbol).trim().toUpperCase();
  if (!SAFE_SYMBOL.test(symbol)) {
    throw new Error("polygonSymbol must be a conservative uppercase ticker.");
  }

  const selected = selectedProviders(providers);
  const credentials = resolveProviderCredentials(env);
  const results = [];
  for (const provider of selected) {
    if (provider === "polygon") {
      results.push(
        await runPolygonPreflight({
          credentials: credentials.polygon,
          allowNetwork,
          baseUrl: polygonBaseUrl,
          allowLoopbackMocks,
          timeoutMs,
          symbol,
          fetchImpl,
        }),
      );
    } else {
      results.push(
        await runAlpacaPreflight({
          credentials: credentials.alpaca,
          allowNetwork,
          baseUrl: alpacaPaperBaseUrl,
          allowLoopbackMocks,
          timeoutMs,
          fetchImpl,
        }),
      );
    }
  }

  const status = summaryStatus(results, requireEvidence);
  const summary = {
    schema_version: 1,
    report_kind: "provider-sandbox-preflight",
    generated_at_utc: nowIso(now),
    status,
    credentialed_network_opt_in: Boolean(allowNetwork),
    credentialed_evidence_required: Boolean(requireEvidence),
    selected_provider_evidence_complete: results.every(
      (item) => item.status === "passed",
    ),
    required_evidence_satisfied: results.every(
      (item) => item.status === "passed",
    ),
    safety_contract: {
      request_methods: ["GET"],
      alpaca_environment: "paper-only",
      alpaca_live_host_allowed: false,
      order_submission_allowed: false,
      provider_payloads_persisted: false,
      credential_values_persisted: false,
      redirects_allowed: false,
    },
    providers: results,
  };
  assertEvidenceContainsNoSecrets(summary, [
    credentials.polygon.apiKey,
    credentials.alpaca.apiKey,
    credentials.alpaca.apiSecret,
  ]);
  return summary;
}

async function atomicWriteJson(path, value) {
  const absolutePath = resolve(path);
  await mkdir(dirname(absolutePath), { recursive: true });
  try {
    const target = await lstat(absolutePath);
    if (target.isSymbolicLink() || !target.isFile()) {
      throw new Error(
        "Provider evidence target must be a regular file when it already exists.",
      );
    }
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
  }
  const stagedPath = `${absolutePath}.tmp-${process.pid}-${randomUUID()}`;
  try {
    await writeFile(stagedPath, `${JSON.stringify(value, null, 2)}\n`, {
      encoding: "utf8",
      flag: "wx",
    });
    await rename(stagedPath, absolutePath);
  } finally {
    await rm(stagedPath, { force: true }).catch(() => {});
  }
  return absolutePath;
}

function parseCliArgs(argv) {
  const options = {
    providers: "all",
    allowNetwork: false,
    requireEvidence: false,
    timeoutMs: DEFAULT_TIMEOUT_MS,
    polygonSymbol: "AAPL",
    out: "",
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const nextValue = () => {
      index += 1;
      if (index >= argv.length || argv[index].startsWith("--")) {
        throw new Error(`${arg} requires a value.`);
      }
      return argv[index];
    };
    switch (arg) {
      case "--providers":
        options.providers = nextValue();
        break;
      case "--allow-network":
        options.allowNetwork = true;
        break;
      case "--require-evidence":
        options.requireEvidence = true;
        break;
      case "--timeout-ms":
        options.timeoutMs = Number(nextValue());
        break;
      case "--polygon-symbol":
        options.polygonSymbol = nextValue();
        break;
      case "--out":
        options.out = nextValue();
        break;
      case "--help":
      case "-h":
        options.help = true;
        break;
      default:
        throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return options;
}

function printUsage() {
  process.stdout.write(`VectorGrid provider sandbox preflight

Usage:
  node scripts/provider_sandbox_preflight.mjs [options]

Options:
  --providers <all|polygon|alpaca>  Provider selection (default: all)
  --allow-network                   Explicitly allow credentialed safe GET requests
  --require-evidence                Fail unless every selected provider passes
  --timeout-ms <250..120000>        Per-request timeout (default: 12000)
  --polygon-symbol <ticker>         Read-only Polygon reference probe (default: AAPL)
  --out <path>                      Evidence JSON path
  --help                            Show this help

Credentials are read only from supported environment variables and must never be
passed on the command line. Alpaca is pinned to paper-api.alpaca.markets. This
harness has no order-submission code and sends GET requests only.
`);
}

async function main() {
  let options;
  try {
    options = parseCliArgs(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`${safeMessage(error?.message)}\n`);
    printUsage();
    process.exitCode = 2;
    return;
  }
  if (options.help) {
    printUsage();
    return;
  }

  const defaultOut = resolve(
    "output",
    "qa",
    "provider-sandbox",
    `${new Date().toISOString().replaceAll(/[:.]/gu, "")}-${randomUUID()}`,
    "provider-sandbox-summary.json",
  );
  const outPath = options.out ? resolve(options.out) : defaultOut;
  try {
    const summary = await runProviderSandboxPreflight(options);
    const writtenPath = await atomicWriteJson(outPath, summary);
    process.stdout.write(
      `Provider sandbox preflight ${summary.status}; evidence written to ${writtenPath}\n`,
    );
    for (const provider of summary.providers) {
      process.stdout.write(
        `- ${provider.provider}: ${provider.status} (${provider.reason_code})\n`,
      );
    }
    process.exitCode = summary.status === "failed" ? 1 : 0;
  } catch (error) {
    const failure = {
      schema_version: 1,
      report_kind: "provider-sandbox-preflight",
      generated_at_utc: new Date().toISOString(),
      status: "failed",
      reason_code: "harness_error",
      message: safeMessage(error?.message ?? "Provider preflight failed."),
      safety_contract: {
        request_methods: ["GET"],
        alpaca_environment: "paper-only",
        order_submission_allowed: false,
        credential_values_persisted: false,
      },
    };
    await atomicWriteJson(outPath, failure).catch(() => {});
    process.stderr.write(`${failure.message}\n`);
    process.exitCode = 1;
  }
}

if (
  process.argv[1] &&
  fileURLToPath(import.meta.url) === resolve(process.argv[1])
) {
  await main();
}
