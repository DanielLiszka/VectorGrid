import assert from "node:assert/strict";
import { createServer } from "node:http";
import { after, before, test } from "node:test";

import {
  runProviderSandboxPreflight,
  validateProviderBaseUrl,
} from "./provider_sandbox_preflight.mjs";

const POLYGON_KEY = "polygon_fake_secret_for_offline_test";
const ALPACA_KEY = "alpaca_fake_key_for_offline_test";
const ALPACA_SECRET = "alpaca_fake_secret_for_offline_test";

let server;
let baseUrl;
let requests;
let mode;

before(async () => {
  requests = [];
  mode = "success";
  server = createServer((request, response) => {
    const url = new URL(request.url, "http://127.0.0.1");
    let body;
    requests.push({
      method: request.method,
      pathname: url.pathname,
      searchParams: Object.fromEntries(url.searchParams),
      alpacaKey: request.headers["apca-api-key-id"],
      alpacaSecret: request.headers["apca-api-secret-key"],
    });

    if (mode === "disconnect") {
      request.socket.destroy();
      return;
    }
    if (mode === "rate-limit") {
      response.writeHead(429, {
        "content-type": "application/json",
        "x-request-id": "rate-limited-request",
      });
      response.end(
        JSON.stringify({
          status: "ERROR",
          error: `credential ${POLYGON_KEY} exceeded its limit`,
        }),
      );
      return;
    }
    if (mode === "redirect") {
      response.writeHead(302, {
        location: "/redirect-target-that-must-not-be-followed",
      });
      response.end();
      return;
    }
    if (mode === "oversized") {
      response.writeHead(200, {
        "content-type": "application/json",
        "content-length": String(512 * 1024 + 1),
      });
      response.end("{}");
      return;
    }

    if (url.pathname === "/v3/reference/tickers") {
      body = {
        status: "OK",
        request_id: "polygon-request-id",
        results: [{ ticker: url.searchParams.get("ticker"), market: "stocks" }],
      };
    } else if (url.pathname === "/v2/account") {
      body = {
        id: "paper-account-id-not-for-evidence",
        account_number: "PA123456",
        status: "ACTIVE",
        trading_blocked: false,
        account_blocked: false,
      };
    } else if (url.pathname === "/v2/clock") {
      body = {
        is_open: mode === "closed-clock" ? false : true,
        next_open: "2026-07-27T09:30:00-04:00",
        next_close: "2026-07-25T16:00:00-04:00",
      };
    } else if (url.pathname === "/v2/orders") {
      body = [
        {
          id: "paper-order-id-not-for-evidence",
          client_order_id: "paper-client-id-not-for-evidence",
        },
      ];
    } else {
      response.writeHead(404, { "content-type": "application/json" });
      response.end(JSON.stringify({ error: "not found" }));
      return;
    }
    response.writeHead(200, {
      "content-type": "application/json",
      "x-request-id": `${url.pathname}-request-id`,
    });
    response.end(JSON.stringify(body));
  });
  await new Promise((resolveListen, rejectListen) => {
    server.once("error", rejectListen);
    server.listen(0, "127.0.0.1", resolveListen);
  });
  const address = server.address();
  baseUrl = `http://127.0.0.1:${address.port}`;
});

after(async () => {
  await new Promise((resolveClose, rejectClose) => {
    server.close((error) => (error ? rejectClose(error) : resolveClose()));
  });
});

function envWithCredentials() {
  return {
    POLYGON_API_KEY: POLYGON_KEY,
    APCA_API_KEY_ID: ALPACA_KEY,
    APCA_API_SECRET_KEY: ALPACA_SECRET,
  };
}

function serialized(summary) {
  return JSON.stringify(summary);
}

test("offline mocks prove only safe GET routes and evidence excludes secrets and payload identity", async () => {
  mode = "success";
  requests.length = 0;
  const summary = await runProviderSandboxPreflight({
    allowNetwork: true,
    requireEvidence: true,
    env: envWithCredentials(),
    polygonBaseUrl: baseUrl,
    alpacaPaperBaseUrl: baseUrl,
    allowLoopbackMocks: true,
  });

  assert.equal(summary.status, "passed");
  assert.equal(summary.required_evidence_satisfied, true);
  assert.deepEqual(
    requests.map((request) => [request.method, request.pathname]),
    [
      ["GET", "/v3/reference/tickers"],
      ["GET", "/v2/account"],
      ["GET", "/v2/clock"],
      ["GET", "/v2/orders"],
    ],
  );
  assert.equal(requests[0].searchParams.apiKey, POLYGON_KEY);
  assert.equal(requests[1].alpacaKey, ALPACA_KEY);
  assert.equal(requests[1].alpacaSecret, ALPACA_SECRET);

  const evidence = serialized(summary);
  for (const forbidden of [
    POLYGON_KEY,
    ALPACA_KEY,
    ALPACA_SECRET,
    "paper-account-id-not-for-evidence",
    "PA123456",
    "paper-order-id-not-for-evidence",
    "paper-client-id-not-for-evidence",
  ]) {
    assert.equal(evidence.includes(forbidden), false, forbidden);
  }
  assert.equal(summary.safety_contract.order_submission_allowed, false);
  assert.equal(summary.safety_contract.alpaca_live_host_allowed, false);
});

test("network opt-in is mandatory even when credentials are present", async () => {
  requests.length = 0;
  const summary = await runProviderSandboxPreflight({
    providers: "alpaca",
    env: envWithCredentials(),
  });
  assert.equal(summary.status, "skipped");
  assert.equal(summary.providers[0].reason_code, "network_opt_in_required");
  assert.equal(summary.providers[0].network_attempted, false);
  assert.equal(requests.length, 0);
});

test("missing credentials skip clearly, but an incomplete Alpaca pair fails closed", async () => {
  const absent = await runProviderSandboxPreflight({
    providers: "all",
    env: {},
  });
  assert.equal(absent.status, "skipped");
  assert.equal(absent.selected_provider_evidence_complete, false);
  assert.equal(absent.required_evidence_satisfied, false);
  assert.deepEqual(
    absent.providers.map((provider) => provider.reason_code),
    ["credentials_absent", "credentials_absent"],
  );

  const incomplete = await runProviderSandboxPreflight({
    providers: "alpaca",
    env: { APCA_API_KEY_ID: ALPACA_KEY },
  });
  assert.equal(incomplete.status, "failed");
  assert.equal(incomplete.providers[0].reason_code, "credentials_incomplete");
  assert.equal(serialized(incomplete).includes(ALPACA_KEY), false);
});

test("strict evidence mode fails when a selected provider is skipped", async () => {
  const summary = await runProviderSandboxPreflight({
    providers: "polygon",
    requireEvidence: true,
    env: {},
  });
  assert.equal(summary.status, "failed");
  assert.equal(summary.required_evidence_satisfied, false);
  assert.equal(summary.providers[0].status, "skipped");
});

test("live Alpaca and non-pinned provider hosts are rejected before fetch", () => {
  assert.throws(
    () =>
      validateProviderBaseUrl("alpaca", "https://api.alpaca.markets", {
        allowLoopbackMocks: false,
      }),
    /paper host/u,
  );
  assert.throws(
    () =>
      validateProviderBaseUrl("polygon", "http://api.polygon.io", {
        allowLoopbackMocks: false,
      }),
    /must use HTTPS/u,
  );
  assert.throws(
    () =>
      validateProviderBaseUrl("polygon", "https://example.com", {
        allowLoopbackMocks: false,
      }),
    /pinned Polygon API host/u,
  );
});

test("rate limiting fails closed and redacts a provider body containing the key", async () => {
  mode = "rate-limit";
  requests.length = 0;
  const summary = await runProviderSandboxPreflight({
    providers: "polygon",
    allowNetwork: true,
    env: envWithCredentials(),
    polygonBaseUrl: baseUrl,
    allowLoopbackMocks: true,
  });
  assert.equal(summary.status, "failed");
  assert.equal(summary.providers[0].reason_code, "rate_limited");
  assert.equal(serialized(summary).includes(POLYGON_KEY), false);
});

test("disconnect is a clear skip and a subsequent reconnect can pass", async () => {
  mode = "disconnect";
  const offline = await runProviderSandboxPreflight({
    providers: "polygon",
    allowNetwork: true,
    env: envWithCredentials(),
    polygonBaseUrl: baseUrl,
    allowLoopbackMocks: true,
    timeoutMs: 500,
  });
  assert.equal(offline.status, "skipped");
  assert.equal(offline.providers[0].reason_code, "network_unavailable");

  mode = "success";
  const recovered = await runProviderSandboxPreflight({
    providers: "polygon",
    allowNetwork: true,
    requireEvidence: true,
    env: envWithCredentials(),
    polygonBaseUrl: baseUrl,
    allowLoopbackMocks: true,
  });
  assert.equal(recovered.status, "passed");
});

test("a closed market clock remains valid connectivity evidence without opening an order route", async () => {
  mode = "closed-clock";
  requests.length = 0;
  const summary = await runProviderSandboxPreflight({
    providers: "alpaca",
    allowNetwork: true,
    requireEvidence: true,
    env: envWithCredentials(),
    alpacaPaperBaseUrl: baseUrl,
    allowLoopbackMocks: true,
  });
  assert.equal(summary.status, "passed");
  assert.equal(summary.providers[0].safe_account_summary.market_open, false);
  assert.equal(
    requests.some((request) => request.method !== "GET"),
    false,
  );
});

test("conflicting credential aliases fail before any network request", async () => {
  await assert.rejects(
    runProviderSandboxPreflight({
      providers: "polygon",
      allowNetwork: true,
      env: {
        POLYGON_API_KEY: "first-fake-key",
        POLYGON_KEY: "different-fake-key",
      },
      polygonBaseUrl: baseUrl,
      allowLoopbackMocks: true,
    }),
    /conflicting values/u,
  );
});

test("redirects are rejected without forwarding provider credentials", async () => {
  mode = "redirect";
  requests.length = 0;
  const summary = await runProviderSandboxPreflight({
    providers: "polygon",
    allowNetwork: true,
    env: envWithCredentials(),
    polygonBaseUrl: baseUrl,
    allowLoopbackMocks: true,
  });
  assert.equal(summary.status, "failed");
  assert.equal(requests.length, 1);
  assert.equal(requests[0].pathname, "/v3/reference/tickers");
});

test("advertised oversized responses fail before provider payload persistence", async () => {
  mode = "oversized";
  requests.length = 0;
  const summary = await runProviderSandboxPreflight({
    providers: "polygon",
    allowNetwork: true,
    env: envWithCredentials(),
    polygonBaseUrl: baseUrl,
    allowLoopbackMocks: true,
  });
  assert.equal(summary.status, "failed");
  assert.match(summary.providers[0].message, /size limit/u);
  assert.equal(summary.safety_contract.provider_payloads_persisted, false);
});
