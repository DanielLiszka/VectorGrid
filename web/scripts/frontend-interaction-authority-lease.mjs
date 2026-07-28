#!/usr/bin/env node
import crypto from "node:crypto";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  isSha256,
  readBoundedFilePublication,
} from "./frontend-interaction-build-attestation.mjs";

export const AUTHORITY_LEASE_KIND = "frontend-interaction-authority-lease";
export const AUTHORITY_MUTEX_NAME =
  "Local\\ta_desktop_demo_frontend_interaction_authority";
export const BENCHMARK_MUTEX_NAME = "Local\\ta_desktop_demo_benchmark_gate";
export const BENCHMARK_LEASE_PATH = "output/qa/benchmark-lock/active.json";
export const BENCHMARK_LEASE_SCHEMA =
  "ta_desktop_demo_benchmark_lock_active_lease_v1";
export const AUTHORITY_WRAPPER_PATH =
  "scripts/run_frontend_interaction_latency.ps1";
export const AUTHORITY_LOCK_CONTEXT =
  "scoped warm mocked-browser click-to-painted-stable latency";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(scriptDir, "..", "..");

function sha256Bytes(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function runFixedPowerShell(script, extraEnv = {}, timeout = 5_000) {
  if (process.platform !== "win32") {
    return {
      status: null,
      stdout: "",
      stderr: "Windows PowerShell is required",
      error: null,
    };
  }
  const encoded = Buffer.from(script, "utf16le").toString("base64");
  return spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-EncodedCommand",
      encoded,
    ],
    {
      encoding: "utf8",
      env: { ...process.env, ...extraEnv },
      timeout,
      windowsHide: true,
    },
  );
}

function probeNamedMutex(mutexName) {
  const result = runFixedPowerShell(
    `
    $ErrorActionPreference = "Stop"
    $mutex = [System.Threading.Mutex]::new(
      $false,
      [string]$env:VECTORGRID_AUTHORITY_MUTEX_PROBE_NAME
    )
    try {
      try {
        $acquired = $mutex.WaitOne(0)
      } catch [System.Threading.AbandonedMutexException] {
        try { $mutex.ReleaseMutex() | Out-Null } catch {}
        exit 3
      }
      if ($acquired) {
        $mutex.ReleaseMutex() | Out-Null
        exit 2
      }
      exit 0
    } finally {
      $mutex.Dispose()
    }
  `,
    {
      VECTORGRID_AUTHORITY_MUTEX_PROBE_NAME: mutexName,
    },
  );
  return {
    held_by_another_process: result.status === 0,
    exit_code: result.status,
    error: result.error?.message || String(result.stderr || "").trim() || null,
  };
}

export function probeAuthorityMutex() {
  return probeNamedMutex(AUTHORITY_MUTEX_NAME);
}

export function probeBenchmarkMutex() {
  return probeNamedMutex(BENCHMARK_MUTEX_NAME);
}

function queryProcessAncestry(startPid) {
  if (!Number.isInteger(startPid) || startPid <= 0) return [];
  const result = runFixedPowerShell(
    `
    $ErrorActionPreference = "Stop"
    $current = [int]$env:VECTORGRID_AUTHORITY_ANCESTRY_START_PID
    $seen = [System.Collections.Generic.HashSet[int]]::new()
    $rows = @()
    for ($depth = 0; $depth -lt 64 -and $current -gt 0; $depth += 1) {
      if (-not $seen.Add($current)) {
        throw "process ancestry contained a cycle"
      }
      $row = Get-CimInstance Win32_Process -Filter "ProcessId = $current"
      if ($null -eq $row) {
        break
      }
      $process = Get-Process -Id $current -ErrorAction Stop
      $rows += [pscustomobject]@{
        pid = $current
        parent_pid = [int]$row.ParentProcessId
        started_at = $process.StartTime.ToUniversalTime().ToString("o")
      }
      $current = [int]$row.ParentProcessId
    }
    [Console]::Out.Write(
      (ConvertTo-Json -InputObject @($rows) -Compress)
    )
  `,
    {
      VECTORGRID_AUTHORITY_ANCESTRY_START_PID: String(startPid),
    },
  );
  if (result.status !== 0) {
    throw new Error(
      `process ancestry probe failed closed: ${
        result.error?.message ||
        String(result.stderr || "").trim() ||
        `exit ${result.status}`
      }`,
    );
  }
  const parsed = JSON.parse(String(result.stdout || "[]"));
  return Array.isArray(parsed) ? parsed : [parsed];
}

function probeLeaseWriteSharingViolation(leasePath) {
  const result = runFixedPowerShell(
    `
    $ErrorActionPreference = "Stop"
    try {
      $stream = [System.IO.FileStream]::new(
        [string]$env:VECTORGRID_AUTHORITY_LEASE_PROBE_PATH,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::ReadWrite,
        (
          [System.IO.FileShare]::ReadWrite -bor
          [System.IO.FileShare]::Delete
        )
      )
      $stream.Dispose()
      [Console]::Out.Write("0")
    } catch {
      $exception = $_.Exception
      while ($null -ne $exception.InnerException) {
        $exception = $exception.InnerException
      }
      $win32Code = $exception.HResult -band 0xffff
      [Console]::Out.Write("$win32Code")
    }
  `,
    {
      VECTORGRID_AUTHORITY_LEASE_PROBE_PATH: leasePath,
    },
  );
  const win32Error = Number(String(result.stdout || "").trim());
  return {
    sharing_violation: result.status === 0 && win32Error === 32,
    win32_error_code: Number.isInteger(win32Error) ? win32Error : null,
    probe_exit_code: result.status,
    error: result.error?.message || String(result.stderr || "").trim() || null,
  };
}

function probeBenchmarkLeaseHolder({ leasePath, ownerPid, ownerStartedAt }) {
  const result = runFixedPowerShell(
    `
    $ErrorActionPreference = "Stop"
    . (Join-Path (
      [string]$env:VECTORGRID_BENCHMARK_LOCK_MODULE_ROOT
    ) "scripts\\benchmark_lock.ps1")
    Assert-BenchmarkLockLeaseHolder \`
      -Path ([string]$env:VECTORGRID_BENCHMARK_LEASE_PATH) \`
      -OwnerPid ([int]$env:VECTORGRID_BENCHMARK_OWNER_PID) \`
      -OwnerStartedAt ([string]$env:VECTORGRID_BENCHMARK_OWNER_STARTED_AT)
    [Console]::Out.Write("verified")
  `,
    {
      VECTORGRID_BENCHMARK_LOCK_MODULE_ROOT: defaultRepoRoot,
      VECTORGRID_BENCHMARK_LEASE_PATH: leasePath,
      VECTORGRID_BENCHMARK_OWNER_PID: String(ownerPid),
      VECTORGRID_BENCHMARK_OWNER_STARTED_AT: String(ownerStartedAt),
    },
    7_500,
  );
  return {
    verified:
      result.status === 0 && String(result.stdout || "").trim() === "verified",
    exit_code: result.status,
    error: result.error?.message || String(result.stderr || "").trim() || null,
  };
}

export function verifyAuthorityLease({
  repoRoot = defaultRepoRoot,
  leasePath,
  expectedNonce,
  expectedRunId,
  expectedParentPid = process.ppid,
  now = new Date(),
  maxAgeMinutes = 30,
} = {}) {
  const issues = [];
  const boundedMaxAge = Number(maxAgeMinutes);
  if (!Number.isFinite(boundedMaxAge) || boundedMaxAge <= 0) {
    issues.push("authority lease max age must be a positive number");
  }
  let ancestry = [];
  try {
    ancestry = queryProcessAncestry(expectedParentPid);
  } catch (error) {
    issues.push(error instanceof Error ? error.message : String(error));
  }
  let publication = null;
  let lease = null;
  let benchmarkLeasePublication = null;
  let benchmarkLeasePayload = null;
  try {
    publication = readBoundedFilePublication({
      repoRoot,
      candidatePath: leasePath,
      expectedPrefix: "output/qa/frontend-interaction-latency/leases/",
    });
    lease = JSON.parse(publication.bytes.toString("utf8"));
  } catch (error) {
    issues.push(error instanceof Error ? error.message : String(error));
  }
  if (!lease || typeof lease !== "object" || Array.isArray(lease)) {
    issues.push("authority lease must contain a JSON object");
  } else {
    if (lease.schema_version !== 1) {
      issues.push("authority lease schema_version must equal 1");
    }
    if (lease.lease_kind !== AUTHORITY_LEASE_KIND) {
      issues.push(`authority lease kind must equal ${AUTHORITY_LEASE_KIND}`);
    }
    if (lease.mutex_name !== AUTHORITY_MUTEX_NAME) {
      issues.push(
        `authority lease mutex_name must equal ${AUTHORITY_MUTEX_NAME}`,
      );
    }
    if (lease.benchmark_mutex_name !== BENCHMARK_MUTEX_NAME) {
      issues.push(
        `authority lease benchmark_mutex_name must equal ${BENCHMARK_MUTEX_NAME}`,
      );
    }
    if (lease.wrapper_path !== AUTHORITY_WRAPPER_PATH) {
      issues.push(
        `authority lease wrapper_path must equal ${AUTHORITY_WRAPPER_PATH}`,
      );
    }
    if (lease.run_id !== expectedRunId) {
      issues.push("authority lease run_id does not match this run");
    }
    if (
      typeof expectedNonce !== "string" ||
      !/^[a-f0-9]{32}$/.test(expectedNonce) ||
      lease.nonce !== expectedNonce
    ) {
      issues.push("authority lease nonce does not match this run");
    }
    if (
      !Number.isInteger(lease.wrapper_pid) ||
      lease.wrapper_pid !== expectedParentPid
    ) {
      issues.push("authority lease wrapper must be the direct parent process");
    }
    const wrapperRow = ancestry.find(
      (entry) => entry?.pid === lease.wrapper_pid,
    );
    if (!wrapperRow || wrapperRow.started_at !== lease.wrapper_started_at) {
      issues.push("authority lease wrapper process identity does not match");
    }
    if (
      lease.mutex_owner_pid !== lease.wrapper_pid ||
      lease.mutex_owner_started_at !== lease.wrapper_started_at ||
      lease.mutex_ownership_mode !== "acquired"
    ) {
      issues.push("authority lease mutex owner metadata is invalid");
    }
    const mutexOwnerRow = ancestry.find(
      (entry) => entry?.pid === lease.mutex_owner_pid,
    );
    if (
      !mutexOwnerRow ||
      mutexOwnerRow.started_at !== lease.mutex_owner_started_at
    ) {
      issues.push(
        "authority lease mutex owner is not the recorded live ancestor",
      );
    }
    if (
      !Number.isInteger(lease.benchmark_mutex_owner_pid) ||
      lease.benchmark_mutex_ownership_mode !== "acquired"
    ) {
      issues.push("benchmark mutex owner metadata is invalid");
    }
    const benchmarkOwnerRow = ancestry.find(
      (entry) => entry?.pid === lease.benchmark_mutex_owner_pid,
    );
    if (
      !benchmarkOwnerRow ||
      benchmarkOwnerRow.started_at !== lease.benchmark_mutex_owner_started_at
    ) {
      issues.push("benchmark mutex owner is not the recorded live ancestor");
    }
    if (lease.benchmark_mutex_owner_pid !== lease.wrapper_pid) {
      issues.push(
        "the directly acquired benchmark mutex must name the wrapper",
      );
    }
    if (
      !Number.isFinite(Date.parse(String(lease.wrapper_started_at || ""))) ||
      !Number.isFinite(
        Date.parse(String(lease.mutex_owner_started_at || "")),
      ) ||
      !Number.isFinite(
        Date.parse(String(lease.benchmark_mutex_owner_started_at || "")),
      )
    ) {
      issues.push("authority lease process start identities are invalid");
    }
    if (lease.wrapper_context !== AUTHORITY_LOCK_CONTEXT) {
      issues.push("authority lease wrapper context is not canonical");
    }
    if (
      typeof lease.benchmark_lock_context !== "string" ||
      lease.benchmark_lock_context.trim().length === 0
    ) {
      issues.push("authority lease benchmark-lock context is empty");
    }
    if (!isSha256(lease.benchmark_lock_token_sha256)) {
      issues.push("authority lease benchmark-lock token hash is invalid");
    }
    if (lease.benchmark_lease_path !== BENCHMARK_LEASE_PATH) {
      issues.push(
        `authority lease benchmark_lease_path must equal ${BENCHMARK_LEASE_PATH}`,
      );
    }
    if (!isSha256(lease.benchmark_lease_sha256)) {
      issues.push("authority lease benchmark lease hash is invalid");
    }
    if (
      typeof lease.benchmark_lease_identity !== "string" ||
      !/^[a-f0-9]{8}:[a-f0-9]{16}$/i.test(lease.benchmark_lease_identity)
    ) {
      issues.push("authority lease benchmark file identity is invalid");
    }
    if (lease.benchmark_lease_write_probe_win32_error_code !== 32) {
      issues.push(
        "wrapper did not observe exact sharing violation 32 on the canonical benchmark lease",
      );
    }
    const environmentToken =
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN || "";
    const environmentContext =
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_CONTEXT || "";
    if (
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_HELD !== "1" ||
      environmentContext !== lease.benchmark_lock_context ||
      !/^[a-f0-9]{32}$/.test(environmentToken) ||
      sha256Bytes(Buffer.from(environmentToken)) !==
        lease.benchmark_lock_token_sha256 ||
      String(process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_PID || "") !==
        String(lease.benchmark_mutex_owner_pid) ||
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_OWNER_STARTED_AT !==
        lease.benchmark_mutex_owner_started_at ||
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_TOKEN_SHA256 !==
        lease.benchmark_lock_token_sha256 ||
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_SHA256 !==
        lease.benchmark_lease_sha256 ||
      process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_MUTEX_NAME !==
        BENCHMARK_MUTEX_NAME
    ) {
      issues.push(
        "inherited benchmark-lock environment does not match the lease",
      );
    }
    try {
      benchmarkLeasePublication = readBoundedFilePublication({
        repoRoot,
        candidatePath: lease.benchmark_lease_path,
        expectedPrefix: "output/qa/benchmark-lock/",
        expectedSha256: lease.benchmark_lease_sha256,
      });
      const expectedLeasePath = path.resolve(repoRoot, BENCHMARK_LEASE_PATH);
      const environmentLeasePath = path.resolve(
        String(process.env.TA_DESKTOP_DEMO_BENCHMARK_LOCK_LEASE_PATH || ""),
      );
      if (
        (process.platform === "win32"
          ? environmentLeasePath.toLowerCase()
          : environmentLeasePath) !==
        (process.platform === "win32"
          ? expectedLeasePath.toLowerCase()
          : expectedLeasePath)
      ) {
        issues.push(
          "benchmark-lock environment does not name the canonical lease path",
        );
      }
      benchmarkLeasePayload = JSON.parse(
        benchmarkLeasePublication.bytes.toString("utf8"),
      );
      if (
        benchmarkLeasePayload.schema !== BENCHMARK_LEASE_SCHEMA ||
        benchmarkLeasePayload.owner_pid !== lease.benchmark_mutex_owner_pid ||
        benchmarkLeasePayload.owner_started_at !==
          lease.benchmark_mutex_owner_started_at ||
        benchmarkLeasePayload.context !== lease.benchmark_lock_context ||
        benchmarkLeasePayload.token_sha256 !==
          lease.benchmark_lock_token_sha256 ||
        benchmarkLeasePayload.mutex_name !== BENCHMARK_MUTEX_NAME ||
        benchmarkLeasePayload.file_identity !==
          lease.benchmark_lease_identity ||
        benchmarkLeasePayload.released !== false
      ) {
        issues.push(
          "canonical benchmark lease contents do not match the live ownership chain",
        );
      }
    } catch (error) {
      issues.push(
        `canonical benchmark lease: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
    const createdMs = Date.parse(String(lease.created_at || ""));
    const nowMs = now.getTime();
    if (
      !Number.isFinite(createdMs) ||
      createdMs > nowMs + 60_000 ||
      !Number.isFinite(boundedMaxAge) ||
      nowMs - createdMs > boundedMaxAge * 60_000
    ) {
      issues.push("authority lease timestamp is stale or invalid");
    }
  }

  let writeProbe = {
    sharing_violation: false,
    win32_error_code: null,
    probe_exit_code: null,
  };
  if (publication) {
    if ((publication.stat.mode & 0o200) === 0) {
      issues.push(
        "authority lease is read-only instead of protected by a live handle",
      );
    }
    writeProbe = probeLeaseWriteSharingViolation(publication.path);
    if (!writeProbe.sharing_violation) {
      issues.push(
        "authority lease does not have an exact live write-sharing violation",
      );
    }
    try {
      const afterProbe = readBoundedFilePublication({
        repoRoot,
        candidatePath: publication.path,
        expectedPrefix: "output/qa/frontend-interaction-latency/leases/",
        expectedSha256: publication.sha256,
      });
      if (
        afterProbe.stat.dev !== publication.stat.dev ||
        afterProbe.stat.ino !== publication.stat.ino
      ) {
        issues.push("authority lease identity changed during verification");
      }
    } catch (error) {
      issues.push(error instanceof Error ? error.message : String(error));
    }
  }

  const mutexProbe = probeAuthorityMutex();
  if (!mutexProbe.held_by_another_process) {
    issues.push(
      "authority mutex is not held by a live external owner during verification",
    );
  }
  const benchmarkMutexProbe = probeBenchmarkMutex();
  if (!benchmarkMutexProbe.held_by_another_process) {
    issues.push(
      "repository benchmark mutex is not held by a live external owner during verification",
    );
  }
  const benchmarkLeaseWriteProbe = benchmarkLeasePublication
    ? probeLeaseWriteSharingViolation(benchmarkLeasePublication.path)
    : {
        sharing_violation: false,
        win32_error_code: null,
        probe_exit_code: null,
      };
  if (!benchmarkLeaseWriteProbe.sharing_violation) {
    issues.push(
      "canonical benchmark lease does not have exact live sharing violation 32",
    );
  }
  let benchmarkLeaseHolderProbe = {
    verified: false,
    exit_code: null,
    error: null,
  };
  if (benchmarkLeasePublication && lease) {
    benchmarkLeaseHolderProbe = probeBenchmarkLeaseHolder({
      leasePath: benchmarkLeasePublication.path,
      ownerPid: lease.benchmark_mutex_owner_pid,
      ownerStartedAt: lease.benchmark_mutex_owner_started_at,
    });
    if (!benchmarkLeaseHolderProbe.verified) {
      issues.push(
        `canonical benchmark lease holder identity is not authoritative: ${
          benchmarkLeaseHolderProbe.error ||
          `exit ${benchmarkLeaseHolderProbe.exit_code}`
        }`,
      );
    }
  }
  if (benchmarkLeasePublication) {
    try {
      readBoundedFilePublication({
        repoRoot,
        candidatePath: benchmarkLeasePublication.path,
        expectedPrefix: "output/qa/benchmark-lock/",
        expectedSha256: benchmarkLeasePublication.sha256,
      });
    } catch (error) {
      issues.push(
        `canonical benchmark lease revalidation: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
    const finalHolderProbe = probeBenchmarkLeaseHolder({
      leasePath: benchmarkLeasePublication.path,
      ownerPid: lease?.benchmark_mutex_owner_pid,
      ownerStartedAt: lease?.benchmark_mutex_owner_started_at,
    });
    if (!finalHolderProbe.verified) {
      issues.push(
        `canonical benchmark lease holder identity changed during verification: ${
          finalHolderProbe.error || `exit ${finalHolderProbe.exit_code}`
        }`,
      );
    }
  }

  const nonceSha256 =
    typeof expectedNonce === "string"
      ? sha256Bytes(Buffer.from(expectedNonce))
      : null;
  return {
    status: issues.length === 0 ? "passed" : "failed",
    verified: issues.length === 0,
    issues: [...new Set(issues)].sort(),
    lease_path: publication?.relative_path ?? null,
    lease_sha256: publication?.sha256 ?? null,
    nonce_sha256: isSha256(nonceSha256) ? nonceSha256 : null,
    owner_pid: lease?.wrapper_pid ?? null,
    owner_started_at: lease?.wrapper_started_at ?? null,
    mutex_owner_pid: lease?.mutex_owner_pid ?? null,
    mutex_owner_started_at: lease?.mutex_owner_started_at ?? null,
    mutex_ownership_mode: lease?.mutex_ownership_mode ?? null,
    mutex_name: lease?.mutex_name ?? null,
    benchmark_mutex_owner_pid: lease?.benchmark_mutex_owner_pid ?? null,
    benchmark_mutex_owner_started_at:
      lease?.benchmark_mutex_owner_started_at ?? null,
    benchmark_mutex_ownership_mode:
      lease?.benchmark_mutex_ownership_mode ?? null,
    benchmark_mutex_name: lease?.benchmark_mutex_name ?? null,
    benchmark_lease_path: benchmarkLeasePublication?.relative_path ?? null,
    benchmark_lease_sha256: benchmarkLeasePublication?.sha256 ?? null,
    benchmark_lease_identity: benchmarkLeasePayload?.file_identity ?? null,
    benchmark_lease_write_probe_win32_error_code:
      benchmarkLeaseWriteProbe.win32_error_code,
    benchmark_lease_write_probe_exit_code:
      benchmarkLeaseWriteProbe.probe_exit_code,
    benchmark_lease_holder_verified: benchmarkLeaseHolderProbe.verified,
    benchmark_lease_holder_probe_exit_code: benchmarkLeaseHolderProbe.exit_code,
    context: lease?.benchmark_lock_context ?? null,
    wrapper_context: lease?.wrapper_context ?? null,
    token_sha256: lease?.benchmark_lock_token_sha256 ?? null,
    created_at: lease?.created_at ?? null,
    write_probe_win32_error_code: writeProbe.win32_error_code,
    write_probe_exit_code: writeProbe.probe_exit_code,
    mutex_probe_exit_code: mutexProbe.exit_code,
    benchmark_mutex_probe_exit_code: benchmarkMutexProbe.exit_code,
  };
}

function parseArgs(argv) {
  const parsed = {
    command: argv[0] || "",
    leasePath: "",
    expectedNonce: "",
    expectedRunId: "",
    repoRoot: defaultRepoRoot,
    maxAgeMinutes: 30,
  };
  for (let index = 1; index < argv.length; index += 1) {
    const arg = argv[index];
    const value = argv[index + 1];
    if (
      [
        "--lease",
        "--nonce",
        "--run-id",
        "--repo-root",
        "--max-age-minutes",
      ].includes(arg)
    ) {
      if (!value) throw new Error(`${arg} requires a value`);
      index += 1;
      if (arg === "--lease") parsed.leasePath = value;
      if (arg === "--nonce") parsed.expectedNonce = value;
      if (arg === "--run-id") parsed.expectedRunId = value;
      if (arg === "--repo-root") parsed.repoRoot = value;
      if (arg === "--max-age-minutes") {
        parsed.maxAgeMinutes = Number(value);
      }
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.command === "mutex-probe") {
    const result = probeAuthorityMutex();
    console.log(JSON.stringify(result));
    if (!result.held_by_another_process) process.exitCode = 1;
    return;
  }
  if (args.command === "benchmark-mutex-probe") {
    const result = probeBenchmarkMutex();
    console.log(JSON.stringify(result));
    if (!result.held_by_another_process) process.exitCode = 1;
    return;
  }
  if (args.command !== "probe") {
    throw new Error(
      "expected command: probe, mutex-probe, or benchmark-mutex-probe",
    );
  }
  const result = verifyAuthorityLease({
    repoRoot: args.repoRoot,
    leasePath: args.leasePath,
    expectedNonce: args.expectedNonce,
    expectedRunId: args.expectedRunId,
    maxAgeMinutes: args.maxAgeMinutes,
  });
  console.log(JSON.stringify(result));
  if (result.status !== "passed") process.exitCode = 1;
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(
      error instanceof Error ? error.stack || error.message : String(error),
    );
    process.exitCode = 1;
  }
}
