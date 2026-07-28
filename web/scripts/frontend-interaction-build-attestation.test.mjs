import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  BUILD_ATTESTATION_KIND,
  BUILD_ATTESTATION_PROFILE,
  FILE_SET_HASH_FRAMING,
  atomicWriteJson,
  computeDistSnapshot,
  createImmutableDistSnapshot,
  createBuildAttestation,
  readBoundedFilePublication,
  resolveBoundedOutputPath,
  validateBuildAttestation,
  validateImmutableDistSnapshot,
} from "./frontend-interaction-build-attestation.mjs";

function legacyDelimiterAmbiguousHash(entries) {
  const hash = crypto.createHash("sha256");
  for (const [logicalPath, bytes] of entries) {
    hash.update(logicalPath);
    hash.update("\0");
    hash.update(bytes);
    hash.update("\0");
  }
  return hash.digest("hex");
}

function fakeSnapshot(seed = "a") {
  return {
    sha256: seed.repeat(64),
    path_manifest_sha256: "b".repeat(64),
    file_count: 2,
    total_bytes: 10,
    newest_mtime_utc: "2026-01-01T00:00:00.000Z",
    paths: ["index.html", "src/main.ts"],
  };
}

function fakeAttestation(now) {
  return {
    schema_version: 1,
    attestation_kind: BUILD_ATTESTATION_KIND,
    build_profile: BUILD_ATTESTATION_PROFILE,
    generated_at: now.toISOString(),
    build_command: "npm.cmd --prefix web run build",
    source_inputs: fakeSnapshot("a"),
    dist: {
      ...fakeSnapshot("c"),
      index_html_sha256: "d".repeat(64),
    },
    toolchain: {
      node_version: process.version,
      npm_version: "definitely-not-current",
    },
  };
}

function createTempWebRoot() {
  const webRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-build-"),
  );
  fs.mkdirSync(path.join(webRoot, "src"), { recursive: true });
  fs.mkdirSync(path.join(webRoot, "public"), { recursive: true });
  fs.mkdirSync(path.join(webRoot, "dist"), { recursive: true });
  fs.writeFileSync(path.join(webRoot, "src", "main.ts"), "export {}\n");
  fs.writeFileSync(path.join(webRoot, "index.html"), "<main></main>\n");
  fs.writeFileSync(path.join(webRoot, "package.json"), "{}\n");
  fs.writeFileSync(
    path.join(webRoot, "dist", "index.html"),
    "<main>built</main>\n",
  );
  fs.writeFileSync(
    path.join(webRoot, "dist", "app.js"),
    "console.log('built')\n",
  );
  return webRoot;
}

test("fresh exact source and dist attestation validates", () => {
  const current = new Date();
  const webRoot = createTempWebRoot();
  const attestation = createBuildAttestation({
    webRoot,
    now: current,
  });
  const result = validateBuildAttestation({
    attestation,
    webRoot,
    maxAgeMinutes: 240,
    now: current,
  });
  assert.equal(result.status, "passed");
  assert.deepEqual(result.issues, []);
});

test("source or dist drift invalidates an otherwise fresh attestation", () => {
  const current = new Date();
  const webRoot = createTempWebRoot();
  const attestation = createBuildAttestation({
    webRoot,
    now: current,
  });
  fs.writeFileSync(
    path.join(webRoot, "src", "main.ts"),
    "export const changed = true\n",
  );
  fs.writeFileSync(
    path.join(webRoot, "dist", "app.js"),
    "console.log('changed build')\n",
  );
  const result = validateBuildAttestation({
    attestation,
    webRoot,
    maxAgeMinutes: 240,
    now: current,
  });
  assert.equal(result.status, "failed");
  assert.ok(
    result.issues.includes("source_inputs.sha256 does not match current files"),
  );
  assert.ok(result.issues.includes("dist.sha256 does not match current files"));
});

test("stale build attestation fails closed before file comparison", () => {
  const now = new Date("2026-07-28T12:00:00.000Z");
  const attestation = fakeAttestation(new Date("2026-07-28T01:00:00.000Z"));
  const result = validateBuildAttestation({
    attestation,
    webRoot: path.join(os.tmpdir(), "missing-vectorgrid-web-root"),
    maxAgeMinutes: 240,
    now,
  });
  assert.equal(result.status, "failed");
  assert.ok(result.issues.includes("build attestation is stale"));
});

test("profile and toolchain drift fail closed", () => {
  const now = new Date("2026-07-28T12:00:00.000Z");
  const attestation = fakeAttestation(now);
  attestation.build_profile = "other-profile";
  attestation.toolchain.node_version = "v0.0.0";
  const result = validateBuildAttestation({
    attestation,
    webRoot: path.join(os.tmpdir(), "missing-vectorgrid-web-root"),
    maxAgeMinutes: 240,
    now,
  });
  assert.ok(
    result.issues.some((issue) => issue.includes(BUILD_ATTESTATION_PROFILE)),
  );
  assert.ok(
    result.issues.includes(
      "toolchain.node_version does not match the measurement runtime",
    ),
  );
  assert.ok(
    result.issues.includes(
      "toolchain.npm_version does not match the measurement runtime",
    ),
  );
});

test("bounded output rejects escape and existing targets", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-path-"),
  );
  fs.mkdirSync(path.join(repoRoot, "output"), { recursive: true });
  assert.throws(
    () =>
      resolveBoundedOutputPath({
        repoRoot,
        candidatePath: path.join(repoRoot, "outside.json"),
      }),
    /must remain below/,
  );
  const existing = path.join(repoRoot, "output", "existing.json");
  fs.writeFileSync(existing, "{}");
  assert.throws(
    () =>
      resolveBoundedOutputPath({
        repoRoot,
        candidatePath: existing,
        mustNotExist: true,
      }),
    /refusing existing output target/,
  );
});

test("atomic new write refuses replacement", () => {
  const directory = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-atomic-"),
  );
  const target = path.join(directory, "summary.json");
  atomicWriteJson(target, { status: "passed" });
  assert.deepEqual(JSON.parse(fs.readFileSync(target, "utf8")), {
    status: "passed",
  });
  assert.throws(
    () => atomicWriteJson(target, { status: "overwritten" }),
    /refusing existing output target/,
  );
});

test("explicit attestation replacement remains atomic", () => {
  const directory = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-attestation-replace-"),
  );
  const target = path.join(directory, "attestation.json");
  atomicWriteJson(target, { generation: 1 });
  atomicWriteJson(target, { generation: 2 }, { replace: true });
  assert.deepEqual(JSON.parse(fs.readFileSync(target, "utf8")), {
    generation: 2,
  });
  assert.deepEqual(fs.readdirSync(directory), ["attestation.json"]);
});

test("failed JSON serialization leaves no partial output or temp file", () => {
  const directory = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-atomic-failure-"),
  );
  const target = path.join(directory, "summary.json");
  const circular = {};
  circular.self = circular;
  assert.throws(() => atomicWriteJson(target, circular), /circular/i);
  assert.equal(fs.existsSync(target), false);
  assert.deepEqual(fs.readdirSync(directory), []);
});

test("bounded output rejects an output-root junction", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-root-junction-"),
  );
  const outside = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-outside-"),
  );
  fs.symlinkSync(outside, path.join(repoRoot, "output"), "junction");
  assert.throws(
    () =>
      resolveBoundedOutputPath({
        repoRoot,
        candidatePath: path.join(repoRoot, "output", "summary.json"),
      }),
    /ordinary directory|reparse alias/i,
  );
});

test("parent junction swap is rejected before any external JSON is written", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-parent-swap-"),
  );
  const safeParent = path.join(repoRoot, "output", "run");
  const movedParent = path.join(repoRoot, "output", "run-original");
  const outside = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-swap-outside-"),
  );
  fs.mkdirSync(safeParent, { recursive: true });
  const target = path.join(safeParent, "summary.json");
  resolveBoundedOutputPath({ repoRoot, candidatePath: target });
  fs.renameSync(safeParent, movedParent);
  fs.symlinkSync(outside, safeParent, "junction");
  assert.throws(
    () => atomicWriteJson(target, { status: "must-not-escape" }, { repoRoot }),
    /reparse|alias/i,
  );
  assert.equal(fs.existsSync(path.join(outside, "summary.json")), false);
});

test("Windows ADS, reserved names, and hardlink publications fail closed", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-special-path-"),
  );
  fs.mkdirSync(path.join(repoRoot, "output"), { recursive: true });
  if (process.platform === "win32") {
    assert.throws(
      () =>
        resolveBoundedOutputPath({
          repoRoot,
          candidatePath: "\\\\?\\C:\\outside\\report.json",
        }),
      /device-namespace/i,
    );
    assert.throws(
      () =>
        resolveBoundedOutputPath({
          repoRoot,
          candidatePath: path.join(repoRoot, "output", "report:stream.json"),
        }),
      /unsafe output component/i,
    );
    assert.throws(
      () =>
        resolveBoundedOutputPath({
          repoRoot,
          candidatePath: path.join(repoRoot, "output", "CON", "report.json"),
        }),
      /unsafe output component/i,
    );
  }
  const original = path.join(repoRoot, "output", "original.json");
  const hardlink = path.join(repoRoot, "output", "linked.json");
  fs.writeFileSync(original, "{}");
  fs.linkSync(original, hardlink);
  assert.throws(
    () =>
      readBoundedFilePublication({
        repoRoot,
        candidatePath: hardlink,
      }),
    /hardlink|single-link/i,
  );

  const dangling = path.join(repoRoot, "output", "dangling.json");
  try {
    fs.symlinkSync(path.join(repoRoot, "missing.json"), dangling, "file");
    assert.throws(
      () =>
        resolveBoundedOutputPath({
          repoRoot,
          candidatePath: dangling,
        }),
      /reparse/i,
    );
  } catch (error) {
    if (
      !error ||
      typeof error !== "object" ||
      !["EPERM", "EACCES", "UNKNOWN"].includes(String(error.code))
    ) {
      throw error;
    }
  }
});

test("no-replace publication preserves a competitor sentinel", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-competitor-"),
  );
  const target = path.join(repoRoot, "output", "summary.json");
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, "competitor sentinel");
  assert.throws(
    () => atomicWriteJson(target, { status: "passed" }, { repoRoot }),
    /refusing existing output target/i,
  );
  assert.equal(fs.readFileSync(target, "utf8"), "competitor sentinel");
});

test("authority publication writes only an unpredictable temporary inode", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-private-write-"),
  );
  const target = path.join(repoRoot, "output", "summary.json");
  const originalOpenSync = fs.openSync;
  const originalWriteFileSync = fs.writeFileSync;
  let reservedPath = "";
  fs.openSync = function interceptedOpen(filePath, flags, ...rest) {
    if (String(filePath).includes("summary.json") && flags === "wx") {
      reservedPath = String(filePath);
    }
    return originalOpenSync.call(this, filePath, flags, ...rest);
  };
  fs.writeFileSync = function interceptedWrite(handle, bytes, ...rest) {
    if (reservedPath) {
      assert.notEqual(reservedPath, target);
      assert.match(reservedPath, /\.tmp-\d+-[a-f0-9-]+$/i);
      assert.equal(fs.existsSync(target), false);
    }
    return originalWriteFileSync.call(this, handle, bytes, ...rest);
  };
  try {
    atomicWriteJson(target, { status: "passed" }, { repoRoot });
  } finally {
    fs.openSync = originalOpenSync;
    fs.writeFileSync = originalWriteFileSync;
  }
  assert.equal(JSON.parse(fs.readFileSync(target, "utf8")).status, "passed");
  assert.equal(fs.statSync(target).nlink, 1);
});

test("immutable dist snapshot is flat, content-exact, and independent of live dist", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-dist-snapshot-"),
  );
  const webRoot = path.join(repoRoot, "web");
  fs.mkdirSync(webRoot);
  fs.mkdirSync(path.join(webRoot, "dist", "assets"), { recursive: true });
  fs.writeFileSync(
    path.join(webRoot, "dist", "index.html"),
    "<main>attested</main>",
  );
  fs.writeFileSync(
    path.join(webRoot, "dist", "assets", "app.js"),
    "globalThis.attested = true;",
  );
  const attestation = createBuildAttestation({
    webRoot: (() => {
      fs.mkdirSync(path.join(webRoot, "src"));
      fs.mkdirSync(path.join(webRoot, "public"));
      fs.writeFileSync(path.join(webRoot, "src", "main.ts"), "export {}");
      fs.writeFileSync(path.join(webRoot, "index.html"), "<main></main>");
      fs.writeFileSync(path.join(webRoot, "package.json"), "{}");
      return webRoot;
    })(),
  });
  const snapshotRoot = path.join(
    repoRoot,
    "output",
    "qa",
    "frontend-interaction-latency",
    "dist-snapshots",
    "0123456789abcdef0123456789abcdef",
  );
  const created = createImmutableDistSnapshot({
    repoRoot,
    webRoot,
    snapshotRoot,
    expectedDistSha256: attestation.dist.sha256,
  });
  assert.equal(created.snapshot.sha256, attestation.dist.sha256);
  assert.equal(fs.existsSync(path.join(snapshotRoot, "dist")), false);
  assert.ok(
    fs
      .readdirSync(snapshotRoot, { withFileTypes: true })
      .every((entry) => entry.isFile()),
  );

  fs.writeFileSync(
    path.join(webRoot, "dist", "index.html"),
    "<main>live dist mutated</main>",
  );
  const verified = validateImmutableDistSnapshot({
    repoRoot,
    snapshotRoot,
    expectedDistSha256: attestation.dist.sha256,
  });
  assert.equal(
    verified.entries.get("index.html").toString("utf8"),
    "<main>attested</main>",
  );
});

test("snapshot validation rejects post-seal mutation and writable aliases", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-dist-tamper-"),
  );
  const webRoot = createTempWebRoot();
  const attestation = createBuildAttestation({ webRoot });
  const snapshotRoot = path.join(
    repoRoot,
    "output",
    "qa",
    "frontend-interaction-latency",
    "dist-snapshots",
    "fedcba9876543210fedcba9876543210",
  );
  createImmutableDistSnapshot({
    repoRoot,
    webRoot,
    snapshotRoot,
    expectedDistSha256: attestation.dist.sha256,
  });
  const manifest = JSON.parse(
    fs.readFileSync(path.join(snapshotRoot, "manifest.json"), "utf8"),
  );
  const appObject = manifest.dist.files.find(
    (entry) => entry.path === "dist/app.js",
  );
  const appPath = path.join(snapshotRoot, `object-${appObject.sha256}.bin`);
  fs.chmodSync(appPath, 0o644);
  fs.writeFileSync(appPath, "tampered");
  assert.throws(
    () =>
      validateImmutableDistSnapshot({
        repoRoot,
        snapshotRoot,
        expectedDistSha256: attestation.dist.sha256,
      }),
    /writable|does not match/i,
  );
});

test("dist snapshot capture rejects hardlinked source files", () => {
  const repoRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-latency-dist-hardlink-"),
  );
  const webRoot = createTempWebRoot();
  const linkedPath = path.join(webRoot, "dist", "linked.js");
  fs.linkSync(path.join(webRoot, "dist", "app.js"), linkedPath);
  assert.throws(() => createBuildAttestation({ webRoot }), /hardlink/i);
});

test("length-prefixed file framing rejects the legacy delimiter-shift collision", () => {
  const originalWebRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-dist-framing-original-"),
  );
  const substitutedWebRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "vectorgrid-dist-framing-substituted-"),
  );
  for (const webRoot of [originalWebRoot, substitutedWebRoot]) {
    fs.mkdirSync(path.join(webRoot, "dist", "assets"), { recursive: true });
    fs.writeFileSync(
      path.join(webRoot, "dist", "index.html"),
      "<main>same index</main>",
    );
  }
  const secondLogicalPath = "dist/assets/b.bin";
  const originalA = Buffer.from("X");
  const originalB = Buffer.concat([
    Buffer.from("Y\0"),
    Buffer.from(secondLogicalPath),
    Buffer.from("\0Z"),
  ]);
  const substitutedA = Buffer.concat([
    Buffer.from("X\0"),
    Buffer.from(secondLogicalPath),
    Buffer.from("\0Y"),
  ]);
  const substitutedB = Buffer.from("Z");
  fs.writeFileSync(
    path.join(originalWebRoot, "dist", "assets", "a.bin"),
    originalA,
  );
  fs.writeFileSync(
    path.join(originalWebRoot, "dist", "assets", "b.bin"),
    originalB,
  );
  fs.writeFileSync(
    path.join(substitutedWebRoot, "dist", "assets", "a.bin"),
    substitutedA,
  );
  fs.writeFileSync(
    path.join(substitutedWebRoot, "dist", "assets", "b.bin"),
    substitutedB,
  );

  const indexBytes = Buffer.from("<main>same index</main>");
  const originalLegacy = legacyDelimiterAmbiguousHash([
    ["dist/assets/a.bin", originalA],
    ["dist/assets/b.bin", originalB],
    ["dist/index.html", indexBytes],
  ]);
  const substitutedLegacy = legacyDelimiterAmbiguousHash([
    ["dist/assets/a.bin", substitutedA],
    ["dist/assets/b.bin", substitutedB],
    ["dist/index.html", indexBytes],
  ]);
  assert.equal(originalLegacy, substitutedLegacy);

  const original = computeDistSnapshot(originalWebRoot);
  const substituted = computeDistSnapshot(substitutedWebRoot);
  assert.equal(
    original.hash_framing,
    "uint64be-length-prefixed-path-size-sha256-manifest-v1",
  );
  assert.equal(original.hash_framing, FILE_SET_HASH_FRAMING);
  assert.equal(original.file_count, substituted.file_count);
  assert.equal(original.total_bytes, substituted.total_bytes);
  assert.equal(original.path_manifest_sha256, substituted.path_manifest_sha256);
  assert.equal(original.index_html_sha256, substituted.index_html_sha256);
  assert.notEqual(original.sha256, substituted.sha256);
  assert.notDeepEqual(original.files, substituted.files);
});
