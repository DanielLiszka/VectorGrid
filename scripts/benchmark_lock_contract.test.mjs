import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptsDirectory = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.dirname(scriptsDirectory);
const hostScript = path.join(
  scriptsDirectory,
  "support",
  "benchmark-lock-contract-host.ps1",
);
const childScript = path.join(
  scriptsDirectory,
  "support",
  "benchmark-lock-contract-child.ps1",
);
const lockScript = path.join(scriptsDirectory, "benchmark_lock.ps1");
const isWindows = process.platform === "win32";

function createIsolatedWorkspace() {
  const isolatedRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-benchmark-lock-contract-"),
  );
  const isolatedScripts = path.join(isolatedRoot, "scripts");
  const isolatedSupport = path.join(isolatedScripts, "support");
  fs.mkdirSync(isolatedSupport, { recursive: true });
  fs.copyFileSync(lockScript, path.join(isolatedScripts, "benchmark_lock.ps1"));
  fs.copyFileSync(
    hostScript,
    path.join(isolatedSupport, "benchmark-lock-contract-host.ps1"),
  );
  fs.copyFileSync(
    childScript,
    path.join(isolatedSupport, "benchmark-lock-contract-child.ps1"),
  );
  return isolatedRoot;
}

function runContract(mode) {
  const isolatedRoot = createIsolatedWorkspace();
  const isolatedHost = path.join(
    isolatedRoot,
    "scripts",
    "support",
    "benchmark-lock-contract-host.ps1",
  );
  let result;
  try {
    result = spawnSync(
      "powershell",
      [
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        isolatedHost,
        "-Mode",
        mode,
        "-RepoRoot",
        isolatedRoot,
      ],
      {
        cwd: isolatedRoot,
        encoding: "utf8",
        timeout: 30_000,
        maxBuffer: 4 * 1024 * 1024,
        windowsHide: true,
      },
    );
  } finally {
    const expectedPrefix = path.join(
      path.resolve(os.tmpdir()),
      "vectorgrid-benchmark-lock-contract-",
    );
    const resolvedIsolatedRoot = path.resolve(isolatedRoot);
    assert.ok(resolvedIsolatedRoot.startsWith(expectedPrefix));
    try {
      fs.rmSync(resolvedIsolatedRoot, { recursive: true, force: true });
    } catch {
      // A killed fixture may leave handles briefly alive; the isolated temp
      // workspace is deliberately safe to retain in that failure mode.
    }
  }
  assert.equal(
    result.status,
    0,
    `contract ${mode} failed\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
  );
  const prefix = "BENCHMARK_LOCK_CONTRACT_JSON:";
  const lines = result.stdout
    .split(/\r?\n/u)
    .filter((line) => line.startsWith(prefix));
  assert.equal(
    lines.length,
    1,
    `contract ${mode} emitted ${lines.length} result lines\n${result.stdout}`,
  );
  const report = JSON.parse(lines[0].slice(prefix.length));
  assert.equal(report.ok, true, `contract ${mode} failed: ${report.error}`);
  return report;
}

test(
  "direct owner publishes one fixed live lease that a child can inherit",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("valid");
    assert.equal(report.child.ok, true);
    assert.equal(report.child.acquired, false);
    assert.equal(report.child.inherited, true);
    assert.equal(report.child.write_probe_win32_error_code, 32);
    assert.match(
      report.lease_path.replaceAll("\\", "/"),
      /\/output\/qa\/benchmark-lock\/active\.json$/u,
    );
    assert.deepEqual(report.child.before, report.child.during);
    assert.deepEqual(report.child.before, report.child.after);
    assert.equal(report.lease_exists_after_release, true);
    assert.equal(report.stale_sha256, report.active_sha256);
  },
);

test(
  "forged environment hashes fail even while the real lease is live",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("forged-environment");
    assert.equal(report.child.ok, false);
    assert.match(report.child.error, /lease hash does not match/u);
  },
);

test(
  "environment markers and a held mutex cannot replace the live file lease",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("forged-unheld");
    assert.equal(report.child.ok, false);
    assert.match(report.child.error, /reported 0/u);
    assert.doesNotMatch(report.child.error, /reported 32/u);
  },
);

test(
  "a live unrelated owner is rejected because it is not an ancestor",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("unrelated-owner");
    assert.equal(report.attempt.ok, false);
    assert.match(report.attempt.error, /not a current ancestor/u);
  },
);

test(
  "a dead owner leaves only non-authoritative stale lease content",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("dead-owner");
    assert.equal(report.attempt.ok, false);
    assert.match(report.attempt.error, /reported 0/u);
  },
);

test(
  "PID reuse is rejected by exact process creation identity",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("pid-start-mismatch");
    assert.equal(report.child.ok, false);
    assert.match(report.child.error, /creation identity does not match/u);
  },
);

test(
  "ACL write denial is error 5 and never accepted as sharing violation 32",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("acl-denied");
    assert.equal(report.probe_code, 5);
    assert.equal(report.child.ok, false);
    assert.match(report.child.error, /reported 5/u);
  },
);

test(
  "release restores every prior environment value and permits a fresh acquisition",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("release");
    assert.equal(report.lease_exists_after_release, true);
    assert.deepEqual(report.restored, report.seeded);
    assert.equal(report.second.ok, true);
    assert.equal(report.second.acquired, true);
    assert.equal(report.second.inherited, false);
    assert.notEqual(report.second.acquisition_id, report.first_acquisition_id);
    assert.equal(report.second.lease_identity, report.first_identity);
  },
);

test(
  "nested child and grandchild inheritance preserve the root environment",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("nested");
    assert.equal(report.child.ok, true);
    assert.equal(report.child.inherited, true);
    assert.equal(report.child.grandchild.ok, true);
    assert.equal(report.child.grandchild.inherited, true);
    assert.deepEqual(report.root_environment, report.after_environment);
    assert.deepEqual(report.child.before, report.child.during);
    assert.deepEqual(report.child.before, report.child.after);
    assert.deepEqual(
      report.child.grandchild.before,
      report.child.grandchild.after,
    );
  },
);

test(
  "inherited Exit fails if the parent releases authority during child work",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("parent-release-during-child");
    assert.equal(report.child_entered_inherited, true);
    assert.equal(report.child_exit.ok, false);
    assert.equal(report.child_exit.exit_revalidated, false);
    assert.match(report.child_exit.error, /reported 0/u);
  },
);

test(
  "Restart Manager binds the fixed lease handle to the claimed owner",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("split-holder");
    assert.notEqual(report.actual_holder_pid, report.claimed_owner_pid);
    assert.equal(report.claimed_owner_held_read_handle, true);
    assert.equal(report.child.ok, false);
    assert.match(report.child.error, /holder identity does not match/u);
    assert.match(report.child.error, /observed:/u);
    assert.match(
      report.child.error,
      new RegExp(String(report.claimed_owner_pid)),
    );
    assert.match(
      report.child.error,
      new RegExp(String(report.actual_holder_pid)),
    );
  },
);

test(
  "the fixed file lease serializes contenders using different Local mutexes",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("fixed-contention");
    assert.notEqual(report.root_mutex, report.contender_mutex);
    assert.equal(report.contender.ok, false);
    assert.match(report.contender.error, /fixed canonical benchmark lease/u);
    assert.match(report.contender.error, /Win32 error 32/u);
  },
);

test(
  "hard-linked active leases fail closed without deleting the canonical file",
  { skip: !isWindows, concurrency: false },
  () => {
    const report = runContract("hardlink-rejected");
    assert.equal(report.attempt.ok, false);
    assert.match(report.attempt.error, /exactly one hard-link name/u);
    assert.equal(report.active_exists, true);
    assert.ok(
      Object.values(report.after_failure_environment).every(
        (value) => value === null,
      ),
    );
    assert.equal(report.recovery.ok, true);
    assert.equal(report.recovery.acquired, true);
  },
);

test("static contract keeps containment checks around the first write", () => {
  const source = fs.readFileSync(lockScript, "utf8");
  assert.match(source, /Join-Path \$leaseDirectory "active\.json"/u);
  assert.match(source, /OPEN_ALWAYS/u);
  assert.match(source, /FILE_FLAG_OPEN_REPARSE_POINT/u);
  assert.match(source, /GetLockingProcessesBounded/u);
  assert.match(source, /GetFinalPathNameByHandle/u);
  assert.match(
    source,
    /if \(-not \$Handle\.acquired\)[\s\S]*Assert-BenchmarkLockInheritedAuthority/u,
  );
  assert.doesNotMatch(source, /Remove-BenchmarkLockLeaseByIdentity/u);

  const activeOpen = source.indexOf(
    "$leaseStream = Open-BenchmarkLockActiveLease",
  );
  const openedHandlePathCheck = source.indexOf(
    "Assert-BenchmarkLockHandlePath",
    activeOpen,
  );
  const preWriteIdentity = source.indexOf("$preWritePathIdentity =");
  const truncate = source.indexOf("$leaseStream.SetLength(0)");
  const flush = source.indexOf("$leaseStream.Flush($true)");
  const postWriteIdentity = source.indexOf("$postWritePathIdentity =");
  assert.ok(activeOpen >= 0);
  assert.ok(openedHandlePathCheck > activeOpen);
  assert.ok(preWriteIdentity > openedHandlePathCheck);
  assert.ok(preWriteIdentity >= 0);
  assert.ok(truncate > preWriteIdentity);
  assert.ok(flush > truncate);
  assert.ok(postWriteIdentity > flush);
});
