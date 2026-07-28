#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const BUILD_ATTESTATION_KIND = "frontend-production-build-attestation";
export const BUILD_ATTESTATION_PROFILE = "vectorgrid-web-production-build-v2";
export const IMMUTABLE_DIST_SNAPSHOT_KIND = "frontend-immutable-dist-snapshot";
export const IMMUTABLE_DIST_OBJECT_LAYOUT = "flat-content-addressed-objects-v1";
export const FILE_SET_HASH_FRAMING =
  "uint64be-length-prefixed-path-size-sha256-manifest-v1";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const defaultWebRoot = path.resolve(scriptDir, "..");
const defaultRepoRoot = path.resolve(defaultWebRoot, "..");

function sha256Bytes(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function updateLengthPrefixed(hash, bytes) {
  const payload = Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes);
  const length = Buffer.alloc(8);
  length.writeBigUInt64BE(BigInt(payload.length));
  hash.update(length);
  hash.update(payload);
}

function createFileSetHashes() {
  const contentHash = crypto.createHash("sha256");
  const pathHash = crypto.createHash("sha256");
  updateLengthPrefixed(
    contentHash,
    Buffer.from(`content:${FILE_SET_HASH_FRAMING}`),
  );
  updateLengthPrefixed(pathHash, Buffer.from(`paths:${FILE_SET_HASH_FRAMING}`));
  return { contentHash, pathHash };
}

function updateFileSetHashes(contentHash, pathHash, logicalPath, bytes) {
  const pathBytes = Buffer.from(logicalPath, "utf8");
  const sizeBytes = Buffer.from(String(bytes.length), "ascii");
  const digestBytes = Buffer.from(sha256Bytes(bytes), "ascii");
  updateLengthPrefixed(pathHash, pathBytes);
  updateLengthPrefixed(contentHash, pathBytes);
  updateLengthPrefixed(contentHash, sizeBytes);
  updateLengthPrefixed(contentHash, digestBytes);
}

export function isSha256(value) {
  return typeof value === "string" && /^[a-f0-9]{64}$/.test(value);
}

function lstatIfPresent(filePath) {
  try {
    return fs.lstatSync(filePath);
  } catch (error) {
    if (
      error &&
      typeof error === "object" &&
      "code" in error &&
      ["ENOENT", "ENOTDIR"].includes(String(error.code))
    ) {
      return null;
    }
    throw error;
  }
}

export function readStableSingleLinkFile(filePath, description) {
  const linkStat = fs.lstatSync(filePath);
  if (linkStat.isSymbolicLink() || !linkStat.isFile()) {
    throw new Error(`${description} must be an ordinary file: ${filePath}`);
  }
  if (linkStat.nlink !== 1) {
    throw new Error(`${description} must not be a hardlink: ${filePath}`);
  }

  const handle = fs.openSync(filePath, "r");
  let before;
  let after;
  let bytes;
  try {
    before = fs.fstatSync(handle);
    if (!before.isFile() || before.nlink !== 1) {
      throw new Error(`${description} must be a single-link file: ${filePath}`);
    }
    bytes = fs.readFileSync(handle);
    after = fs.fstatSync(handle);
  } finally {
    fs.closeSync(handle);
  }
  if (
    before.dev !== after.dev ||
    before.ino !== after.ino ||
    before.size !== after.size ||
    before.mtimeMs !== after.mtimeMs
  ) {
    throw new Error(`${description} changed while being read: ${filePath}`);
  }
  const currentLink = fs.lstatSync(filePath);
  if (
    currentLink.isSymbolicLink() ||
    !currentLink.isFile() ||
    currentLink.nlink !== 1
  ) {
    throw new Error(`${description} path became an alias: ${filePath}`);
  }
  const current = fs.statSync(filePath);
  if (
    current.dev !== after.dev ||
    current.ino !== after.ino ||
    current.size !== after.size ||
    current.mtimeMs !== after.mtimeMs
  ) {
    throw new Error(`${description} path identity changed: ${filePath}`);
  }
  return { bytes, stat: after };
}

export function computeCanonicalFileSetSnapshot(root, relativePaths) {
  const resolvedRoot = path.resolve(root);
  if (!Array.isArray(relativePaths) || relativePaths.length === 0) {
    throw new Error("canonical file set requires at least one path");
  }
  const files = [];
  const seen = new Set();
  for (const relativePath of relativePaths) {
    if (
      typeof relativePath !== "string" ||
      relativePath.includes("\\") ||
      !relativePath ||
      path.isAbsolute(relativePath)
    ) {
      throw new Error(`canonical file-set path is unsafe: ${relativePath}`);
    }
    const absolutePath = path.resolve(resolvedRoot, relativePath);
    const normalizedRelative = path
      .relative(resolvedRoot, absolutePath)
      .replaceAll("\\", "/");
    if (
      normalizedRelative === ".." ||
      normalizedRelative.startsWith("../") ||
      normalizedRelative !== relativePath ||
      seen.has(normalizedRelative)
    ) {
      throw new Error(
        `canonical file-set path escaped or was duplicated: ${relativePath}`,
      );
    }
    seen.add(normalizedRelative);
    files.push(absolutePath);
  }
  return snapshotFiles(resolvedRoot, files);
}

function assertOrdinaryFileBelowRoot(root, filePath, description) {
  const resolvedRoot = path.resolve(root);
  const resolvedFile = path.resolve(filePath);
  const relativePath = path.relative(resolvedRoot, resolvedFile);
  if (
    !relativePath ||
    relativePath === ".." ||
    relativePath.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relativePath)
  ) {
    throw new Error(`${description} escaped its root: ${filePath}`);
  }
  let cursor = resolvedRoot;
  const segments = relativePath.split(path.sep);
  for (let index = 0; index < segments.length; index += 1) {
    cursor = path.join(cursor, segments[index]);
    const stat = fs.lstatSync(cursor);
    if (stat.isSymbolicLink()) {
      throw new Error(`${description} contains a reparse point: ${cursor}`);
    }
    if (index < segments.length - 1 && !stat.isDirectory()) {
      throw new Error(`${description} parent is not a directory: ${cursor}`);
    }
  }
  const realRoot = fs.realpathSync.native(resolvedRoot);
  const realFile = fs.realpathSync.native(resolvedFile);
  const caseInsensitive = process.platform === "win32";
  const comparableRoot = caseInsensitive ? realRoot.toLowerCase() : realRoot;
  const comparableFile = caseInsensitive ? realFile.toLowerCase() : realFile;
  if (!comparableFile.startsWith(`${comparableRoot}${path.sep}`)) {
    throw new Error(`${description} resolves outside its root: ${filePath}`);
  }
}

function walkRegularFiles(root, current, files) {
  const currentStat = lstatIfPresent(current);
  if (!currentStat) return;
  if (currentStat.isSymbolicLink() || !currentStat.isDirectory()) {
    throw new Error(
      `build attestation directory must be ordinary: ${path.relative(root, current)}`,
    );
  }
  for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
    const absolutePath = path.join(current, entry.name);
    const entryStat = fs.lstatSync(absolutePath);
    if (entry.isSymbolicLink() || entryStat.isSymbolicLink()) {
      throw new Error(
        `build attestation refuses symbolic-link input: ${path.relative(root, absolutePath)}`,
      );
    }
    if (entryStat.isDirectory()) {
      walkRegularFiles(root, absolutePath, files);
    } else if (entryStat.isFile()) {
      files.push(absolutePath);
    }
  }
}

function snapshotFiles(root, files) {
  const uniqueFiles = [...new Set(files.map((file) => path.resolve(file)))]
    .filter((file) => fs.existsSync(file))
    .sort((left, right) => left.localeCompare(right));
  if (uniqueFiles.length === 0) {
    throw new Error(`cannot attest an empty file set under ${root}`);
  }

  const contentHash = crypto.createHash("sha256");
  const pathHash = crypto.createHash("sha256");
  let newestMtimeMs = 0;
  let totalBytes = 0;
  const relativePaths = [];
  const fileManifest = [];
  for (const file of uniqueFiles) {
    const linkStat = fs.lstatSync(file);
    if (linkStat.isSymbolicLink()) {
      throw new Error(
        `build attestation refuses symbolic-link input: ${path.relative(root, file)}`,
      );
    }
    if (!linkStat.isFile()) continue;
    const relativePath = path.relative(root, file).replaceAll("\\", "/");
    if (
      relativePath === ".." ||
      relativePath.startsWith("../") ||
      path.isAbsolute(relativePath)
    ) {
      throw new Error(`attested file escaped root: ${file}`);
    }
    if (linkStat.nlink !== 1) {
      throw new Error(
        `build attestation refuses hardlink input: ${path.relative(root, file)}`,
      );
    }
    assertOrdinaryFileBelowRoot(root, file, "build attestation input");
    const { stat, bytes } = readStableSingleLinkFile(
      file,
      "build attestation input",
    );
    assertOrdinaryFileBelowRoot(root, file, "build attestation input");
    relativePaths.push(relativePath);
    newestMtimeMs = Math.max(newestMtimeMs, stat.mtimeMs);
    totalBytes += bytes.length;
    updateFileSetHashes(contentHash, pathHash, relativePath, bytes);
    fileManifest.push({
      path: relativePath,
      size: bytes.length,
      sha256: sha256Bytes(bytes),
    });
  }
  if (relativePaths.length === 0) {
    throw new Error(`cannot attest an empty regular-file set under ${root}`);
  }
  return {
    sha256: contentHash.digest("hex"),
    path_manifest_sha256: pathHash.digest("hex"),
    file_count: relativePaths.length,
    total_bytes: totalBytes,
    newest_mtime_utc: new Date(newestMtimeMs).toISOString(),
    hash_framing: FILE_SET_HASH_FRAMING,
    paths: relativePaths,
    files: fileManifest,
  };
}

export function computeBuildInputSnapshot(webRoot = defaultWebRoot) {
  const resolvedWebRoot = path.resolve(webRoot);
  const files = [];
  for (const directoryName of ["src", "public"]) {
    walkRegularFiles(
      resolvedWebRoot,
      path.join(resolvedWebRoot, directoryName),
      files,
    );
  }
  for (const fileName of [
    "index.html",
    "package-lock.json",
    "package.json",
    "vite.config.ts",
    "scripts/check-release-dist.mjs",
    "scripts/clean-dist.mjs",
  ]) {
    files.push(path.join(resolvedWebRoot, fileName));
  }
  for (const entry of fs.readdirSync(resolvedWebRoot, {
    withFileTypes: true,
  })) {
    if (
      entry.isFile() &&
      (entry.name.startsWith("tsconfig") || entry.name.startsWith(".env"))
    ) {
      files.push(path.join(resolvedWebRoot, entry.name));
    }
  }
  return snapshotFiles(resolvedWebRoot, files);
}

export function computeDistSnapshot(webRoot = defaultWebRoot) {
  const resolvedWebRoot = path.resolve(webRoot);
  const distRoot = path.join(resolvedWebRoot, "dist");
  return loadDistByteSnapshot(distRoot).snapshot;
}

function snapshotDistEntries(entries, newestMtimeUtc) {
  const { contentHash, pathHash } = createFileSetHashes();
  const logicalPaths = [];
  const fileManifest = [];
  let totalBytes = 0;
  const sortedEntries = [...entries.entries()].sort(([left], [right]) =>
    left.localeCompare(right),
  );
  for (const [relativePath, bytes] of sortedEntries) {
    const logicalPath = `dist/${relativePath}`;
    updateFileSetHashes(contentHash, pathHash, logicalPath, bytes);
    logicalPaths.push(logicalPath);
    fileManifest.push({
      path: logicalPath,
      size: bytes.length,
      sha256: sha256Bytes(bytes),
    });
    totalBytes += bytes.length;
  }
  const indexBytes = entries.get("index.html");
  if (!indexBytes) {
    throw new Error("distribution index.html is missing");
  }
  return {
    sha256: contentHash.digest("hex"),
    path_manifest_sha256: pathHash.digest("hex"),
    file_count: logicalPaths.length,
    total_bytes: totalBytes,
    newest_mtime_utc: newestMtimeUtc,
    hash_framing: FILE_SET_HASH_FRAMING,
    paths: logicalPaths,
    files: fileManifest,
    index_html_sha256: sha256Bytes(indexBytes),
  };
}

export function loadDistByteSnapshot(
  distRoot,
  { requireReadOnly = false } = {},
) {
  const resolvedDistRoot = path.resolve(distRoot);
  const rootStat = fs.lstatSync(resolvedDistRoot);
  if (!rootStat.isDirectory() || rootStat.isSymbolicLink()) {
    throw new Error(
      `distribution root must be an ordinary directory: ${resolvedDistRoot}`,
    );
  }
  if (requireReadOnly && (rootStat.mode & 0o222) !== 0) {
    throw new Error(
      `immutable distribution directory is writable: ${resolvedDistRoot}`,
    );
  }

  const files = [];
  walkRegularFiles(resolvedDistRoot, resolvedDistRoot, files);
  files.sort((left, right) => left.localeCompare(right));
  if (files.length === 0) {
    throw new Error(
      `cannot snapshot an empty distribution: ${resolvedDistRoot}`,
    );
  }

  const entries = new Map();
  let newestMtimeMs = 0;
  for (const filePath of files) {
    const relativePath = path
      .relative(resolvedDistRoot, filePath)
      .replaceAll("\\", "/");
    if (
      !relativePath ||
      relativePath === ".." ||
      relativePath.startsWith("../") ||
      path.isAbsolute(relativePath)
    ) {
      throw new Error(`distribution file escaped its root: ${filePath}`);
    }
    assertOrdinaryFileBelowRoot(
      resolvedDistRoot,
      filePath,
      "distribution input",
    );
    const { bytes, stat } = readStableSingleLinkFile(
      filePath,
      "distribution input",
    );
    assertOrdinaryFileBelowRoot(
      resolvedDistRoot,
      filePath,
      "distribution input",
    );
    if (requireReadOnly && (stat.mode & 0o222) !== 0) {
      throw new Error(`immutable distribution file is writable: ${filePath}`);
    }
    entries.set(relativePath, Buffer.from(bytes));
    newestMtimeMs = Math.max(newestMtimeMs, stat.mtimeMs);
  }
  return {
    entries,
    snapshot: snapshotDistEntries(
      entries,
      new Date(newestMtimeMs).toISOString(),
    ),
  };
}

function npmVersion() {
  const command =
    process.platform === "win32" ? process.env.ComSpec || "cmd.exe" : "npm";
  const args =
    process.platform === "win32"
      ? ["/d", "/s", "/c", "npm.cmd --version"]
      : ["--version"];
  const result = spawnSync(command, args, {
    encoding: "utf8",
    windowsHide: true,
  });
  if (result.status !== 0) return null;
  const version = String(result.stdout || "").trim();
  return version || null;
}

export function createBuildAttestation({
  webRoot = defaultWebRoot,
  expectedSourceSha256 = "",
  now = new Date(),
} = {}) {
  const sourceInputs = computeBuildInputSnapshot(webRoot);
  if (expectedSourceSha256 && sourceInputs.sha256 !== expectedSourceSha256) {
    throw new Error(
      `frontend build inputs changed during build: expected ${expectedSourceSha256}, observed ${sourceInputs.sha256}`,
    );
  }
  return {
    schema_version: 1,
    attestation_kind: BUILD_ATTESTATION_KIND,
    build_profile: BUILD_ATTESTATION_PROFILE,
    generated_at: now.toISOString(),
    build_command: "npm.cmd --prefix web run build",
    source_inputs: sourceInputs,
    dist: computeDistSnapshot(webRoot),
    toolchain: {
      node_version: process.version,
      npm_version: npmVersion(),
    },
  };
}

export function validateBuildAttestation({
  attestation,
  webRoot = defaultWebRoot,
  maxAgeMinutes,
  now = new Date(),
} = {}) {
  const issues = [];
  if (!attestation || typeof attestation !== "object") {
    return { status: "failed", issues: ["attestation must be an object"] };
  }
  if (attestation.schema_version !== 1) {
    issues.push("schema_version must equal 1");
  }
  if (attestation.attestation_kind !== BUILD_ATTESTATION_KIND) {
    issues.push(`attestation_kind must equal ${BUILD_ATTESTATION_KIND}`);
  }
  if (attestation.build_profile !== BUILD_ATTESTATION_PROFILE) {
    issues.push(`build_profile must equal ${BUILD_ATTESTATION_PROFILE}`);
  }
  if (attestation.build_command !== "npm.cmd --prefix web run build") {
    issues.push("build_command is not the fixed production web build");
  }
  const generatedMs = Date.parse(String(attestation.generated_at || ""));
  const nowMs = now.getTime();
  const boundedMaxAge = Number(maxAgeMinutes);
  if (!Number.isFinite(generatedMs)) {
    issues.push("generated_at must be an ISO timestamp");
  } else {
    if (generatedMs > nowMs + 60_000) {
      issues.push("generated_at is unreasonably in the future");
    }
    if (
      !Number.isFinite(boundedMaxAge) ||
      boundedMaxAge <= 0 ||
      nowMs - generatedMs > boundedMaxAge * 60_000
    ) {
      issues.push("build attestation is stale");
    }
  }

  let currentSource = null;
  let currentDist = null;
  try {
    currentSource = computeBuildInputSnapshot(webRoot);
    currentDist = computeDistSnapshot(webRoot);
  } catch (error) {
    issues.push(error instanceof Error ? error.message : String(error));
  }
  for (const [name, current, recorded] of [
    ["source_inputs", currentSource, attestation.source_inputs],
    ["dist", currentDist, attestation.dist],
  ]) {
    if (!current || !recorded || typeof recorded !== "object") {
      issues.push(`${name} snapshot is missing`);
      continue;
    }
    for (const field of [
      "sha256",
      "path_manifest_sha256",
      "file_count",
      "total_bytes",
      "hash_framing",
    ]) {
      if (current[field] !== recorded[field]) {
        issues.push(`${name}.${field} does not match current files`);
      }
    }
    for (const field of ["paths", "files"]) {
      if (JSON.stringify(current[field]) !== JSON.stringify(recorded[field])) {
        issues.push(`${name}.${field} does not match current files`);
      }
    }
  }
  if (
    currentDist &&
    attestation.dist?.index_html_sha256 !== currentDist.index_html_sha256
  ) {
    issues.push("dist.index_html_sha256 does not match current index.html");
  }
  if (attestation.toolchain?.node_version !== process.version) {
    issues.push(
      "toolchain.node_version does not match the measurement runtime",
    );
  }
  const currentNpmVersion = npmVersion();
  if (
    typeof attestation.toolchain?.npm_version !== "string" ||
    attestation.toolchain.npm_version !== currentNpmVersion
  ) {
    issues.push("toolchain.npm_version does not match the measurement runtime");
  }
  if (!isSha256(attestation.source_inputs?.sha256)) {
    issues.push("source_inputs.sha256 must be a lowercase SHA-256");
  }
  if (!isSha256(attestation.dist?.sha256)) {
    issues.push("dist.sha256 must be a lowercase SHA-256");
  }
  return {
    status: issues.length === 0 ? "passed" : "failed",
    issues: [...new Set(issues)].sort(),
    current_source_inputs: currentSource,
    current_dist: currentDist,
  };
}

export function resolveBoundedOutputPath({
  repoRoot = defaultRepoRoot,
  candidatePath,
  mustNotExist = false,
  requireExisting = false,
  expectedType = "",
}) {
  const rawCandidate = String(candidatePath ?? "");
  if (
    process.platform === "win32" &&
    /^(?:\\\\|\/\/)[?.](?:\\|\/)/.test(rawCandidate)
  ) {
    throw new Error(`device-namespace paths are forbidden: ${candidatePath}`);
  }
  const resolvedRepoRoot = path.resolve(repoRoot);
  const outputRoot = path.resolve(resolvedRepoRoot, "output");
  const resolvedCandidate = path.resolve(resolvedRepoRoot, rawCandidate);
  const caseInsensitive = process.platform === "win32";
  const comparableCandidate = caseInsensitive
    ? resolvedCandidate.toLowerCase()
    : resolvedCandidate;
  const comparableOutputRoot = caseInsensitive
    ? outputRoot.toLowerCase()
    : outputRoot;
  const comparableOutputPrefix = `${comparableOutputRoot}${path.sep}`;
  if (
    comparableCandidate === comparableOutputRoot ||
    !comparableCandidate.startsWith(comparableOutputPrefix)
  ) {
    throw new Error(`path must remain below ${outputRoot}: ${candidatePath}`);
  }
  const relativeCandidate = path.relative(outputRoot, resolvedCandidate);
  const candidateSegments = relativeCandidate.split(path.sep);
  if (
    candidateSegments.some(
      (segment) =>
        !segment ||
        segment === "." ||
        segment === ".." ||
        (process.platform === "win32" &&
          (segment.includes(":") ||
            segment.endsWith(".") ||
            segment.endsWith(" ") ||
            /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(segment))),
    )
  ) {
    throw new Error(
      `path contains an unsafe output component: ${candidatePath}`,
    );
  }
  fs.mkdirSync(outputRoot, { recursive: true });
  const outputStat = fs.lstatSync(outputRoot);
  if (!outputStat.isDirectory() || outputStat.isSymbolicLink()) {
    throw new Error(`output root must be an ordinary directory: ${outputRoot}`);
  }
  const realOutputRoot = fs.realpathSync.native(outputRoot);
  const normalizedRealOutput = caseInsensitive
    ? realOutputRoot.toLowerCase()
    : realOutputRoot;
  if (normalizedRealOutput !== comparableOutputRoot) {
    throw new Error(`output root must not be a reparse alias: ${outputRoot}`);
  }

  let cursor = outputRoot;
  for (const segment of candidateSegments) {
    cursor = path.join(cursor, segment);
    const cursorStat = lstatIfPresent(cursor);
    if (!cursorStat) break;
    if (cursorStat.isSymbolicLink()) {
      throw new Error(`output path contains a reparse point: ${cursor}`);
    }
    const isTarget = cursor === resolvedCandidate;
    if (!isTarget && !cursorStat.isDirectory()) {
      throw new Error(`output parent is not a directory: ${cursor}`);
    }
  }

  const candidateStat = lstatIfPresent(resolvedCandidate);
  if (candidateStat) {
    if (candidateStat.isSymbolicLink()) {
      throw new Error(
        `path must not be a reparse target: ${resolvedCandidate}`,
      );
    }
    if (expectedType === "file" && !candidateStat.isFile()) {
      throw new Error(`output target must be a file: ${resolvedCandidate}`);
    }
    if (expectedType === "directory" && !candidateStat.isDirectory()) {
      throw new Error(
        `output target must be a directory: ${resolvedCandidate}`,
      );
    }
    if (candidateStat.isFile() && candidateStat.nlink !== 1) {
      throw new Error(
        `output target must not be a hardlink: ${resolvedCandidate}`,
      );
    }
    const realCandidate = fs.realpathSync.native(resolvedCandidate);
    const comparableRealCandidate = caseInsensitive
      ? realCandidate.toLowerCase()
      : realCandidate;
    if (
      comparableRealCandidate === normalizedRealOutput ||
      !comparableRealCandidate.startsWith(`${normalizedRealOutput}${path.sep}`)
    ) {
      throw new Error(`path resolves outside output/: ${candidatePath}`);
    }
  }
  if (mustNotExist && candidateStat) {
    throw new Error(`refusing existing output target: ${resolvedCandidate}`);
  }
  if (requireExisting && !candidateStat) {
    throw new Error(`required output target is missing: ${resolvedCandidate}`);
  }
  return resolvedCandidate;
}

function ensureBoundedParentDirectory(repoRoot, pathName) {
  const resolvedPath = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: pathName,
  });
  const outputRoot = path.resolve(repoRoot, "output");
  const parent = path.dirname(resolvedPath);
  const parentSegments = path.relative(outputRoot, parent).split(path.sep);
  let cursor = outputRoot;
  for (const segment of parentSegments) {
    if (!segment || segment === ".") continue;
    cursor = path.join(cursor, segment);
    if (!lstatIfPresent(cursor)) fs.mkdirSync(cursor);
    const stat = fs.lstatSync(cursor);
    if (!stat.isDirectory() || stat.isSymbolicLink()) {
      throw new Error(`output parent must be an ordinary directory: ${cursor}`);
    }
  }
  resolveBoundedOutputPath({
    repoRoot,
    candidatePath: resolvedPath,
  });
  const parentStat = fs.statSync(parent);
  return {
    path: resolvedPath,
    parent,
    parent_realpath: fs.realpathSync.native(parent),
    parent_dev: parentStat.dev,
    parent_ino: parentStat.ino,
  };
}

function assertParentIdentity(repoRoot, pathName, expected) {
  const current = ensureBoundedParentDirectory(repoRoot, pathName);
  const currentRealPath =
    process.platform === "win32"
      ? current.parent_realpath.toLowerCase()
      : current.parent_realpath;
  const expectedRealPath =
    process.platform === "win32"
      ? expected.parent_realpath.toLowerCase()
      : expected.parent_realpath;
  if (
    currentRealPath !== expectedRealPath ||
    current.parent_dev !== expected.parent_dev ||
    current.parent_ino !== expected.parent_ino
  ) {
    throw new Error(`output parent identity changed: ${current.parent}`);
  }
  return current;
}

export function readBoundedFilePublication({
  repoRoot = defaultRepoRoot,
  candidatePath,
  expectedSha256 = "",
  expectedPrefix = "",
}) {
  const resolvedPath = resolveBoundedOutputPath({
    repoRoot,
    candidatePath,
    requireExisting: true,
    expectedType: "file",
  });
  const relativePath = path
    .relative(path.resolve(repoRoot), resolvedPath)
    .replaceAll("\\", "/");
  if (expectedPrefix && !relativePath.startsWith(expectedPrefix)) {
    throw new Error(
      `output path must start with ${expectedPrefix}: ${relativePath}`,
    );
  }
  const handle = fs.openSync(resolvedPath, "r");
  let before;
  let after;
  let bytes;
  try {
    before = fs.fstatSync(handle);
    if (!before.isFile() || before.nlink !== 1) {
      throw new Error(
        `published output must be a single-link file: ${resolvedPath}`,
      );
    }
    bytes = fs.readFileSync(handle);
    after = fs.fstatSync(handle);
  } finally {
    fs.closeSync(handle);
  }
  if (
    before.dev !== after.dev ||
    before.ino !== after.ino ||
    before.size !== after.size ||
    before.mtimeMs !== after.mtimeMs
  ) {
    throw new Error(
      `published output changed while being read: ${resolvedPath}`,
    );
  }
  const resolvedAgain = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: resolvedPath,
    requireExisting: true,
    expectedType: "file",
  });
  const pathStat = fs.statSync(resolvedAgain);
  if (pathStat.dev !== after.dev || pathStat.ino !== after.ino) {
    throw new Error(`published output path identity changed: ${resolvedPath}`);
  }
  const sha256 = sha256Bytes(bytes);
  if (expectedSha256 && sha256 !== expectedSha256) {
    throw new Error(
      `published output SHA-256 mismatch: expected ${expectedSha256}, observed ${sha256}`,
    );
  }
  return {
    path: resolvedPath,
    relative_path: relativePath,
    bytes,
    sha256,
    stat: after,
  };
}

export function atomicWriteJson(
  pathName,
  value,
  { replace = false, repoRoot = null } = {},
) {
  const serialized = `${JSON.stringify(value, null, 2)}\n`;
  const serializedBytes = Buffer.from(serialized, "utf8");
  const expectedSha256 = sha256Bytes(serializedBytes);
  if (!repoRoot) {
    if (!replace && lstatIfPresent(pathName)) {
      throw new Error(`refusing existing output target: ${pathName}`);
    }
    fs.mkdirSync(path.dirname(pathName), { recursive: true });
    const tempPath = `${pathName}.tmp-${process.pid}-${crypto.randomUUID()}`;
    let handle = null;
    try {
      handle = fs.openSync(tempPath, "wx");
      fs.writeFileSync(handle, serializedBytes);
      fs.fsyncSync(handle);
      fs.closeSync(handle);
      handle = null;
      if (!replace && lstatIfPresent(pathName)) {
        throw new Error(`refusing existing output target: ${pathName}`);
      }
      fs.renameSync(tempPath, pathName);
      return {
        path: pathName,
        relative_path: null,
        bytes: fs.readFileSync(pathName),
        sha256: expectedSha256,
        stat: fs.statSync(pathName),
      };
    } catch (error) {
      if (handle != null) {
        try {
          fs.closeSync(handle);
        } catch {
          // Preserve the original atomic-write failure.
        }
      }
      try {
        fs.rmSync(tempPath, { force: true });
      } catch {
        // Preserve the original atomic-write failure.
      }
      throw error;
    }
  }

  const parentGuard = repoRoot
    ? ensureBoundedParentDirectory(repoRoot, pathName)
    : null;
  const publicationPath = parentGuard.path;
  if (!replace && lstatIfPresent(publicationPath)) {
    throw new Error(`refusing existing output target: ${publicationPath}`);
  }
  const writePath = `${publicationPath}.tmp-${process.pid}-${crypto.randomUUID()}`;
  let handle = null;
  let createdIdentity = null;
  let publicationCreated = false;
  try {
    assertParentIdentity(repoRoot, publicationPath, parentGuard);
    handle = fs.openSync(writePath, "wx");
    const tempStat = fs.fstatSync(handle);
    if (!tempStat.isFile() || tempStat.nlink !== 1) {
      throw new Error(
        `reserved output must be a single-link file: ${writePath}`,
      );
    }
    createdIdentity = { dev: tempStat.dev, ino: tempStat.ino };
    const resolvedWritePath = resolveBoundedOutputPath({
      repoRoot,
      candidatePath: writePath,
      requireExisting: true,
      expectedType: "file",
    });
    const reservedPathStat = fs.statSync(resolvedWritePath);
    if (
      reservedPathStat.dev !== tempStat.dev ||
      reservedPathStat.ino !== tempStat.ino
    ) {
      throw new Error(`reserved output path identity changed: ${writePath}`);
    }
    assertParentIdentity(repoRoot, publicationPath, parentGuard);
    fs.writeFileSync(handle, serializedBytes);
    fs.fsyncSync(handle);
    const completedStat = fs.fstatSync(handle);
    if (
      completedStat.dev !== tempStat.dev ||
      completedStat.ino !== tempStat.ino ||
      completedStat.nlink !== 1 ||
      completedStat.size !== serializedBytes.length
    ) {
      throw new Error(`reserved output changed while writing: ${writePath}`);
    }
    assertParentIdentity(repoRoot, publicationPath, parentGuard);
    fs.closeSync(handle);
    handle = null;
    readBoundedFilePublication({
      repoRoot,
      candidatePath: writePath,
      expectedSha256,
    });
    assertParentIdentity(repoRoot, publicationPath, parentGuard);
    if (replace) {
      const currentTarget = lstatIfPresent(publicationPath);
      if (currentTarget) {
        readBoundedFilePublication({
          repoRoot,
          candidatePath: publicationPath,
        });
      }
      assertParentIdentity(repoRoot, publicationPath, parentGuard);
      fs.renameSync(writePath, publicationPath);
    } else {
      fs.linkSync(writePath, publicationPath);
      publicationCreated = true;
      fs.unlinkSync(writePath);
    }
    assertParentIdentity(repoRoot, publicationPath, parentGuard);
    return readBoundedFilePublication({
      repoRoot,
      candidatePath: publicationPath,
      expectedSha256,
    });
  } catch (error) {
    if (handle != null) {
      try {
        fs.closeSync(handle);
      } catch {
        // Preserve the original atomic-write failure.
      }
    }
    try {
      const currentPath = resolveBoundedOutputPath({
        repoRoot,
        candidatePath: writePath,
        requireExisting: true,
        expectedType: "file",
      });
      const currentStat = fs.statSync(currentPath);
      if (
        createdIdentity &&
        currentStat.dev === createdIdentity.dev &&
        currentStat.ino === createdIdentity.ino
      ) {
        fs.rmSync(currentPath);
      }
    } catch {
      // Never follow an ambiguous path while preserving the original failure.
    }
    if (publicationCreated) {
      try {
        const currentPublication = resolveBoundedOutputPath({
          repoRoot,
          candidatePath: publicationPath,
          requireExisting: true,
          expectedType: "file",
        });
        const currentStat = fs.statSync(currentPublication);
        if (
          createdIdentity &&
          currentStat.dev === createdIdentity.dev &&
          currentStat.ino === createdIdentity.ino
        ) {
          fs.rmSync(currentPublication);
        }
      } catch {
        // Never follow an ambiguous path while preserving the original failure.
      }
    }
    throw error;
  }
}

function snapshotFieldIssues(expected, observed, prefix) {
  const issues = [];
  for (const field of [
    "sha256",
    "path_manifest_sha256",
    "file_count",
    "total_bytes",
    "index_html_sha256",
    "hash_framing",
  ]) {
    if (expected?.[field] !== observed?.[field]) {
      issues.push(`${prefix}.${field} does not match`);
    }
  }
  for (const field of ["paths", "files"]) {
    if (
      JSON.stringify(expected?.[field]) !== JSON.stringify(observed?.[field])
    ) {
      issues.push(`${prefix}.${field} does not match`);
    }
  }
  return issues;
}

function makeTreeWritable(directory) {
  const rootStat = lstatIfPresent(directory);
  if (!rootStat || rootStat.isSymbolicLink()) return;
  if (rootStat.isDirectory()) {
    for (const entry of fs.readdirSync(directory)) {
      makeTreeWritable(path.join(directory, entry));
    }
  }
  try {
    fs.chmodSync(directory, rootStat.isDirectory() ? 0o755 : 0o644);
  } catch {
    // Cleanup remains best-effort.
  }
}

function removeCreatedSnapshotIfStillBounded(repoRoot, snapshotRoot, identity) {
  try {
    const boundedRoot = resolveBoundedOutputPath({
      repoRoot,
      candidatePath: snapshotRoot,
      requireExisting: true,
      expectedType: "directory",
    });
    const current = fs.statSync(boundedRoot);
    if (current.dev !== identity?.dev || current.ino !== identity?.ino) {
      return;
    }
    makeTreeWritable(boundedRoot);
    fs.rmSync(boundedRoot, { recursive: true });
  } catch {
    // Never follow an ambiguous cleanup path.
  }
}

function assertSnapshotRootIdentity(repoRoot, snapshotRoot, identity) {
  const boundedRoot = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: snapshotRoot,
    requireExisting: true,
    expectedType: "directory",
  });
  const current = fs.statSync(boundedRoot);
  if (current.dev !== identity.dev || current.ino !== identity.ino) {
    throw new Error(`immutable snapshot root identity changed: ${boundedRoot}`);
  }
  return boundedRoot;
}

function sealSnapshotTreeReadOnly(snapshotRoot) {
  const directories = [];
  function visit(directory) {
    directories.push(directory);
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const entryPath = path.join(directory, entry.name);
      const entryStat = fs.lstatSync(entryPath);
      if (entryStat.isSymbolicLink()) {
        throw new Error(`snapshot tree contains a reparse point: ${entryPath}`);
      }
      if (entryStat.isDirectory()) {
        visit(entryPath);
      } else if (entryStat.isFile()) {
        if (entryStat.nlink !== 1) {
          throw new Error(`snapshot tree contains a hardlink: ${entryPath}`);
        }
        fs.chmodSync(entryPath, 0o444);
      } else {
        throw new Error(`snapshot tree contains a special file: ${entryPath}`);
      }
    }
  }
  visit(snapshotRoot);
  for (const directory of directories.reverse()) {
    fs.chmodSync(directory, 0o555);
  }
}

function assertSnapshotTreeReadOnly(snapshotRoot) {
  function visit(directory) {
    const directoryStat = fs.lstatSync(directory);
    if (!directoryStat.isDirectory() || directoryStat.isSymbolicLink()) {
      throw new Error(
        `immutable snapshot path must be an ordinary directory: ${directory}`,
      );
    }
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const entryPath = path.join(directory, entry.name);
      const entryStat = fs.lstatSync(entryPath);
      if (entryStat.isSymbolicLink()) {
        throw new Error(
          `immutable snapshot contains a reparse point: ${entryPath}`,
        );
      }
      if (entryStat.isDirectory()) {
        visit(entryPath);
      } else if (!entryStat.isFile()) {
        throw new Error(
          `immutable snapshot contains a special file: ${entryPath}`,
        );
      } else {
        if (entryStat.nlink !== 1) {
          throw new Error(
            `immutable snapshot contains a hardlink: ${entryPath}`,
          );
        }
        if ((entryStat.mode & 0o222) !== 0) {
          throw new Error(
            `immutable snapshot file remains writable: ${entryPath}`,
          );
        }
      }
    }
  }
  visit(snapshotRoot);
}

function snapshotObjectFileName(sha256) {
  if (!isSha256(sha256)) {
    throw new Error("snapshot object requires a lowercase SHA-256");
  }
  return `object-${sha256}.bin`;
}

function loadFlatSnapshotObjects({
  repoRoot,
  snapshotRoot,
  recordedSnapshot,
  requireReadOnly,
}) {
  if (
    !recordedSnapshot ||
    !Array.isArray(recordedSnapshot.files) ||
    recordedSnapshot.files.length === 0
  ) {
    throw new Error("snapshot manifest must contain a non-empty file manifest");
  }
  const entries = new Map();
  const objectNames = new Set();
  for (const file of recordedSnapshot.files) {
    if (
      typeof file?.path !== "string" ||
      !file.path.startsWith("dist/") ||
      file.path.includes("\\") ||
      !Number.isInteger(file.size) ||
      file.size < 0 ||
      !isSha256(file.sha256)
    ) {
      throw new Error("snapshot manifest contains an invalid file object");
    }
    const relativePath = file.path.slice("dist/".length);
    const segments = relativePath.split("/");
    if (
      !relativePath ||
      segments.some(
        (segment) => !segment || segment === "." || segment === "..",
      ) ||
      entries.has(relativePath)
    ) {
      throw new Error(
        `snapshot manifest contains an unsafe or duplicate path: ${file.path}`,
      );
    }
    const objectName = snapshotObjectFileName(file.sha256);
    objectNames.add(objectName);
    const publication = readBoundedFilePublication({
      repoRoot,
      candidatePath: path.join(snapshotRoot, objectName),
      expectedPrefix: "output/qa/frontend-interaction-latency/dist-snapshots/",
      expectedSha256: file.sha256,
    });
    if (publication.bytes.length !== file.size) {
      throw new Error(`snapshot object size mismatch: ${file.path}`);
    }
    if (requireReadOnly && (publication.stat.mode & 0o222) !== 0) {
      throw new Error(`immutable snapshot object is writable: ${objectName}`);
    }
    entries.set(relativePath, Buffer.from(publication.bytes));
  }
  const snapshot = snapshotDistEntries(
    entries,
    recordedSnapshot.newest_mtime_utc,
  );
  return {
    entries,
    snapshot,
    object_names: [...objectNames].sort(),
  };
}

export function createImmutableDistSnapshot({
  repoRoot = defaultRepoRoot,
  webRoot = defaultWebRoot,
  snapshotRoot,
  expectedDistSha256,
  now = new Date(),
} = {}) {
  if (!isSha256(expectedDistSha256)) {
    throw new Error("immutable snapshot requires an expected dist SHA-256");
  }
  const boundedRoot = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: snapshotRoot,
    mustNotExist: true,
  });
  const parentGuard = ensureBoundedParentDirectory(repoRoot, boundedRoot);
  const source = loadDistByteSnapshot(path.resolve(webRoot, "dist"));
  if (source.snapshot.sha256 !== expectedDistSha256) {
    throw new Error(
      `distribution changed before snapshot: expected ${expectedDistSha256}, observed ${source.snapshot.sha256}`,
    );
  }

  let rootIdentity = null;
  try {
    assertParentIdentity(repoRoot, boundedRoot, parentGuard);
    fs.mkdirSync(boundedRoot);
    const createdRoot = resolveBoundedOutputPath({
      repoRoot,
      candidatePath: boundedRoot,
      requireExisting: true,
      expectedType: "directory",
    });
    const createdRootStat = fs.statSync(createdRoot);
    rootIdentity = {
      dev: createdRootStat.dev,
      ino: createdRootStat.ino,
    };
    assertParentIdentity(repoRoot, boundedRoot, parentGuard);
    assertSnapshotRootIdentity(repoRoot, createdRoot, rootIdentity);

    const writtenObjects = new Map();
    for (const bytes of source.entries.values()) {
      assertSnapshotRootIdentity(repoRoot, createdRoot, rootIdentity);
      const objectSha256 = sha256Bytes(bytes);
      if (writtenObjects.has(objectSha256)) {
        if (!writtenObjects.get(objectSha256).equals(bytes)) {
          throw new Error(`snapshot object digest collision: ${objectSha256}`);
        }
        continue;
      }
      const destination = path.join(
        createdRoot,
        snapshotObjectFileName(objectSha256),
      );
      const writePath = path.join(
        createdRoot,
        `.pending-${crypto.randomUUID()}.bin`,
      );
      assertSnapshotRootIdentity(repoRoot, createdRoot, rootIdentity);
      const handle = fs.openSync(writePath, "wx");
      try {
        const before = fs.fstatSync(handle);
        if (!before.isFile() || before.nlink !== 1) {
          throw new Error(
            `snapshot object must be a single-link file: ${writePath}`,
          );
        }
        const resolvedWritePath = resolveBoundedOutputPath({
          repoRoot,
          candidatePath: writePath,
          requireExisting: true,
          expectedType: "file",
        });
        const destinationStat = fs.statSync(resolvedWritePath);
        if (
          destinationStat.dev !== before.dev ||
          destinationStat.ino !== before.ino
        ) {
          throw new Error(
            `snapshot object path identity changed: ${writePath}`,
          );
        }
        assertSnapshotRootIdentity(repoRoot, createdRoot, rootIdentity);
        fs.writeFileSync(handle, bytes);
        fs.fsyncSync(handle);
        const after = fs.fstatSync(handle);
        if (
          after.dev !== before.dev ||
          after.ino !== before.ino ||
          after.nlink !== 1 ||
          after.size !== bytes.length
        ) {
          throw new Error(
            `snapshot object changed while writing: ${writePath}`,
          );
        }
      } finally {
        fs.closeSync(handle);
      }
      readBoundedFilePublication({
        repoRoot,
        candidatePath: writePath,
        expectedPrefix:
          "output/qa/frontend-interaction-latency/dist-snapshots/",
        expectedSha256: objectSha256,
      });
      assertSnapshotRootIdentity(repoRoot, createdRoot, rootIdentity);
      fs.linkSync(writePath, destination);
      fs.unlinkSync(writePath);
      readBoundedFilePublication({
        repoRoot,
        candidatePath: destination,
        expectedPrefix:
          "output/qa/frontend-interaction-latency/dist-snapshots/",
        expectedSha256: objectSha256,
      });
      writtenObjects.set(objectSha256, Buffer.from(bytes));
    }

    assertParentIdentity(repoRoot, boundedRoot, parentGuard);
    assertSnapshotRootIdentity(repoRoot, createdRoot, rootIdentity);
    resolveBoundedOutputPath({
      repoRoot,
      candidatePath: createdRoot,
      requireExisting: true,
      expectedType: "directory",
    });
    const copied = loadFlatSnapshotObjects({
      repoRoot,
      snapshotRoot: createdRoot,
      recordedSnapshot: source.snapshot,
      requireReadOnly: false,
    });
    const copyIssues = snapshotFieldIssues(
      source.snapshot,
      copied.snapshot,
      "copied_dist",
    );
    if (copyIssues.length > 0) {
      throw new Error(copyIssues.join("; "));
    }
    const manifest = {
      schema_version: 2,
      snapshot_kind: IMMUTABLE_DIST_SNAPSHOT_KIND,
      object_layout: IMMUTABLE_DIST_OBJECT_LAYOUT,
      created_at: now.toISOString(),
      source_dist_sha256: source.snapshot.sha256,
      dist: copied.snapshot,
    };
    atomicWriteJson(path.join(createdRoot, "manifest.json"), manifest, {
      repoRoot,
    });
    sealSnapshotTreeReadOnly(createdRoot);
    const validated = validateImmutableDistSnapshot({
      repoRoot,
      snapshotRoot: createdRoot,
      expectedDistSha256,
    });
    return {
      ...validated,
      entries: undefined,
    };
  } catch (error) {
    if (rootIdentity) {
      removeCreatedSnapshotIfStillBounded(repoRoot, boundedRoot, rootIdentity);
    }
    throw error;
  }
}

export function validateImmutableDistSnapshot({
  repoRoot = defaultRepoRoot,
  snapshotRoot,
  expectedDistSha256,
} = {}) {
  if (!isSha256(expectedDistSha256)) {
    throw new Error("snapshot verification requires an expected dist SHA-256");
  }
  const boundedRoot = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: snapshotRoot,
    requireExisting: true,
    expectedType: "directory",
  });
  assertSnapshotTreeReadOnly(boundedRoot);

  const manifestPublication = readBoundedFilePublication({
    repoRoot,
    candidatePath: path.join(boundedRoot, "manifest.json"),
    expectedPrefix: "output/qa/frontend-interaction-latency/dist-snapshots/",
  });
  const manifest = JSON.parse(manifestPublication.bytes.toString("utf8"));
  const issues = [];
  if (manifest.schema_version !== 2) {
    issues.push("snapshot manifest schema_version must equal 2");
  }
  if (manifest.snapshot_kind !== IMMUTABLE_DIST_SNAPSHOT_KIND) {
    issues.push(
      `snapshot manifest kind must equal ${IMMUTABLE_DIST_SNAPSHOT_KIND}`,
    );
  }
  if (manifest.object_layout !== IMMUTABLE_DIST_OBJECT_LAYOUT) {
    issues.push(
      `snapshot object_layout must equal ${IMMUTABLE_DIST_OBJECT_LAYOUT}`,
    );
  }
  if (manifest.source_dist_sha256 !== expectedDistSha256) {
    issues.push("snapshot source hash does not match the build attestation");
  }
  const loaded = loadFlatSnapshotObjects({
    repoRoot,
    snapshotRoot: boundedRoot,
    recordedSnapshot: manifest.dist,
    requireReadOnly: true,
  });
  const expectedChildren = [...loaded.object_names, "manifest.json"].sort();
  const actualChildren = fs.readdirSync(boundedRoot).sort();
  if (JSON.stringify(actualChildren) !== JSON.stringify(expectedChildren)) {
    issues.push(
      `immutable snapshot root has unexpected entries: ${actualChildren.join(", ")}`,
    );
  }
  issues.push(
    ...snapshotFieldIssues(manifest.dist, loaded.snapshot, "snapshot.dist"),
  );
  if (loaded.snapshot.sha256 !== expectedDistSha256) {
    issues.push(
      "sealed distribution hash does not match the build attestation",
    );
  }
  readBoundedFilePublication({
    repoRoot,
    candidatePath: manifestPublication.path,
    expectedPrefix: "output/qa/frontend-interaction-latency/dist-snapshots/",
    expectedSha256: manifestPublication.sha256,
  });
  const resolvedAgain = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: boundedRoot,
    requireExisting: true,
    expectedType: "directory",
  });
  if (resolvedAgain !== boundedRoot) {
    issues.push("immutable snapshot path identity changed");
  }
  if (issues.length > 0) {
    throw new Error([...new Set(issues)].sort().join("; "));
  }
  return {
    status: "passed",
    snapshot_root: path
      .relative(path.resolve(repoRoot), boundedRoot)
      .replaceAll("\\", "/"),
    manifest_path: manifestPublication.relative_path,
    manifest_sha256: manifestPublication.sha256,
    source_dist_sha256: manifest.source_dist_sha256,
    snapshot: loaded.snapshot,
    entries: loaded.entries,
  };
}

function parseArgs(argv) {
  const [command, ...rest] = argv;
  const parsed = {
    command,
    outPath: "",
    attestationPath: "",
    snapshotRoot: "",
    expectedSourceSha256: "",
    expectedDistSha256: "",
    expectedSha256: "",
    maxAgeMinutes: 240,
  };
  for (let index = 0; index < rest.length; index += 1) {
    const arg = rest[index];
    const value = rest[index + 1];
    if (
      [
        "--out",
        "--attestation",
        "--snapshot",
        "--expected-source-sha",
        "--expected-dist-sha",
        "--expected-sha",
        "--max-age-minutes",
      ].includes(arg)
    ) {
      if (!value) throw new Error(`${arg} requires a value`);
      index += 1;
      if (arg === "--out") parsed.outPath = value;
      if (arg === "--attestation") parsed.attestationPath = value;
      if (arg === "--snapshot") parsed.snapshotRoot = value;
      if (arg === "--expected-source-sha") parsed.expectedSourceSha256 = value;
      if (arg === "--expected-dist-sha") parsed.expectedDistSha256 = value;
      if (arg === "--expected-sha") parsed.expectedSha256 = value;
      if (arg === "--max-age-minutes") parsed.maxAgeMinutes = Number(value);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.command === "fingerprint") {
    console.log(JSON.stringify(computeBuildInputSnapshot(defaultWebRoot)));
    return;
  }
  if (args.command === "capture") {
    if (!args.outPath) throw new Error("capture requires --out");
    const outPath = resolveBoundedOutputPath({
      candidatePath: args.outPath,
      mustNotExist: true,
    });
    const attestation = createBuildAttestation({
      expectedSourceSha256: args.expectedSourceSha256,
    });
    atomicWriteJson(outPath, attestation, { repoRoot: defaultRepoRoot });
    console.log(`Build attestation captured: ${outPath}`);
    return;
  }
  if (args.command === "verify") {
    if (!args.attestationPath) throw new Error("verify requires --attestation");
    const publication = readBoundedFilePublication({
      candidatePath: args.attestationPath,
      expectedPrefix: "output/",
    });
    const attestation = JSON.parse(publication.bytes.toString("utf8"));
    const result = validateBuildAttestation({
      attestation,
      maxAgeMinutes: args.maxAgeMinutes,
    });
    console.log(JSON.stringify(result));
    if (result.status !== "passed") process.exitCode = 1;
    return;
  }
  if (args.command === "snapshot") {
    if (!args.snapshotRoot) throw new Error("snapshot requires --snapshot");
    const result = createImmutableDistSnapshot({
      snapshotRoot: args.snapshotRoot,
      expectedDistSha256: args.expectedDistSha256,
    });
    console.log(JSON.stringify(result));
    return;
  }
  if (args.command === "verify-snapshot") {
    if (!args.snapshotRoot) {
      throw new Error("verify-snapshot requires --snapshot");
    }
    const verified = validateImmutableDistSnapshot({
      snapshotRoot: args.snapshotRoot,
      expectedDistSha256: args.expectedDistSha256,
    });
    const result = { ...verified };
    delete result.entries;
    console.log(JSON.stringify(result));
    return;
  }
  if (args.command === "inspect-output") {
    if (!args.outPath) throw new Error("inspect-output requires --out");
    const publication = readBoundedFilePublication({
      candidatePath: args.outPath,
      expectedPrefix: "output/",
      expectedSha256: args.expectedSha256,
    });
    console.log(
      JSON.stringify({
        status: "passed",
        path: publication.relative_path,
        sha256: publication.sha256,
        size: publication.stat.size,
        device: publication.stat.dev,
        inode: publication.stat.ino,
      }),
    );
    return;
  }
  if (args.command === "publish-copy") {
    if (!args.attestationPath || !args.outPath) {
      throw new Error("publish-copy requires --attestation and --out");
    }
    const source = readBoundedFilePublication({
      candidatePath: args.attestationPath,
      expectedPrefix: "output/",
    });
    const value = JSON.parse(source.bytes.toString("utf8"));
    const outPath = resolveBoundedOutputPath({
      candidatePath: args.outPath,
    });
    const publication = atomicWriteJson(outPath, value, {
      replace: true,
      repoRoot: defaultRepoRoot,
    });
    console.log(
      JSON.stringify({
        status: "passed",
        path: publication.relative_path,
        sha256: publication.sha256,
        source_sha256: source.sha256,
      }),
    );
    return;
  }
  throw new Error(
    "expected command: fingerprint, capture, verify, snapshot, verify-snapshot, inspect-output, or publish-copy",
  );
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
