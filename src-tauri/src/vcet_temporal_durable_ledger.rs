//! Cooperative-process-crash-safe, fail-closed authorization ledger for VCET temporal stages.
//!
//! The scientific contract is deliberately stricter than a retry queue: once an
//! authorization is claimed, an interrupted execution is poisoned instead of
//! being opened again. A successfully published response can be replayed only
//! for the byte-identical request identity. This is not authority against power
//! loss, filesystem rollback, backup restore, or adversarial local modification.
//!
//! The live-claim registry below is deliberately process-local. Product desktop
//! commands run in one application process because Tauri registers
//! `tauri-plugin-single-instance`; separate cooperating processes remain governed
//! only by the durable fail-closed records and cannot distinguish a live foreign
//! claimant from a crashed one.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const LEDGER_SCHEMA_VERSION: u16 = 2;
const FINAL_LEDGER_DIRECTORY: &str = "vcet-temporal-final-ledger-v2";
const PRE_FINAL_LEDGER_DIRECTORY: &str = "vcet-temporal-winner-lock-ledger-v2";
const CLAIM_FILE: &str = "claim.json";
const TERMINAL_FILE: &str = "terminal.json";
const MAX_COMPLETED_RECORD_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CLAIM_RECORD_BYTES: u64 = 64 * 1024;
const MAX_FAILED_RECORD_BYTES: u64 = 64 * 1024;
const MAX_FAILURE_TEXT_BYTES: usize = 8 * 1024;

const CLAIM_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_durable_claim.v2";
const COMPLETED_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_durable_completed.v2";
const FAILED_HASH_CONTRACT: &str = "vectorgrid.vcet.temporal_durable_failed.v2";

static LEDGER_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static LIVE_DURABLE_AUTHORIZATIONS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DurableClaimRecordV1 {
    schema_version: u16,
    authorization_sha256: String,
    request_sha256: String,
    claimed_at_unix_ms: u64,
    claimant_process_id: u32,
    claim_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum DurableTerminalRecordV2<T> {
    Completed {
        schema_version: u16,
        authorization_sha256: String,
        request_sha256: String,
        response: T,
        terminal_sha256: String,
    },
    Failed {
        schema_version: u16,
        authorization_sha256: String,
        request_sha256: String,
        error: String,
        terminal_sha256: String,
    },
}

#[derive(Serialize)]
struct ClaimHashInput<'a> {
    hash_contract: &'static str,
    schema_version: u16,
    authorization_sha256: &'a str,
    request_sha256: &'a str,
    claimed_at_unix_ms: u64,
    claimant_process_id: u32,
}

#[derive(Serialize)]
struct CompletedHashInput<'a, T> {
    hash_contract: &'static str,
    schema_version: u16,
    authorization_sha256: &'a str,
    request_sha256: &'a str,
    response: &'a T,
}

#[derive(Serialize)]
struct FailedHashInput<'a> {
    hash_contract: &'static str,
    schema_version: u16,
    authorization_sha256: &'a str,
    request_sha256: &'a str,
    error: &'a str,
}

pub(crate) enum DurableFinalAdmission<T> {
    Acquired(DurableFinalClaim),
    Replay(T),
}

/// Read-only campaign recovery result for one immutable durable authorization.
///
/// `Blocked` is reserved for ledger state that was positively observed and can
/// never be retried under the same authorization. Filesystem availability
/// failures are returned separately so callers cannot accidentally turn a
/// sharing violation or permission outage into a permanent scientific state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DurableProbeBlockedReason {
    Failed,
    Interrupted,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DurableCompletionProbe<T> {
    Missing,
    Completed { request_sha256: String, response: T },
    Blocked(DurableProbeBlockedReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DurableProbeUnavailableKind {
    LiveClaim,
    PermissionDenied,
    SharingViolation,
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DurableProbeUnavailable {
    kind: DurableProbeUnavailableKind,
}

impl DurableProbeUnavailable {
    #[cfg(test)]
    pub(crate) fn kind(self) -> DurableProbeUnavailableKind {
        self.kind
    }
}

impl std::fmt::Display for DurableProbeUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self.kind {
            DurableProbeUnavailableKind::LiveClaim => "same-process execution still active",
            DurableProbeUnavailableKind::PermissionDenied => "permission denied",
            DurableProbeUnavailableKind::SharingViolation => "sharing violation",
            DurableProbeUnavailableKind::Io => "I/O error",
        };
        write!(
            formatter,
            "VCET durable state is temporarily unavailable ({reason}); retry the operation"
        )
    }
}

#[derive(Debug)]
enum DurableProbeReadError {
    Blocked(DurableProbeBlockedReason),
    Unavailable(DurableProbeUnavailable),
}

pub(crate) struct DurableFinalClaim {
    authorization_dir: PathBuf,
    authorization_sha256: String,
    request_sha256: String,
    live_registration: LiveDurableAuthorizationRegistration,
}

#[derive(Debug)]
struct LiveDurableAuthorizationRegistration {
    authorization_identity: PathBuf,
    active: AtomicBool,
}

impl LiveDurableAuthorizationRegistration {
    fn acquire(authorization_identity: PathBuf) -> Result<Self, DurableProbeUnavailable> {
        let mut live = live_durable_authorizations()
            .lock()
            .unwrap_or_else(|poisoned| {
                // Retain the fail-closed set after an unrelated panic while holding
                // the registry. Its operations below do not themselves panic.
                poisoned.into_inner()
            });
        if !live.insert(authorization_identity.clone()) {
            return Err(live_claim_unavailable());
        }
        Ok(Self {
            authorization_identity,
            active: AtomicBool::new(true),
        })
    }

    fn release(&self) {
        if !self.active.swap(false, Ordering::AcqRel) {
            return;
        }
        live_durable_authorizations()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.authorization_identity);
    }
}

impl Drop for LiveDurableAuthorizationRegistration {
    fn drop(&mut self) {
        self.release();
    }
}

fn live_durable_authorizations() -> &'static Mutex<HashSet<PathBuf>> {
    LIVE_DURABLE_AUTHORIZATIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn lock_live_durable_authorizations() -> MutexGuard<'static, HashSet<PathBuf>> {
    live_durable_authorizations()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn live_claim_unavailable() -> DurableProbeUnavailable {
    DurableProbeUnavailable {
        kind: DurableProbeUnavailableKind::LiveClaim,
    }
}

fn canonical_live_authorization_identity(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    authorization_sha256: &str,
) -> Result<PathBuf, DurableProbeUnavailable> {
    let canonical_app_root = fs::canonicalize(app_local_data_dir).map_err(|error| {
        match durable_probe_unavailable(&error) {
            DurableProbeReadError::Unavailable(error) => error,
            DurableProbeReadError::Blocked(_) => DurableProbeUnavailable {
                kind: DurableProbeUnavailableKind::Io,
            },
        }
    })?;
    Ok(canonical_app_root
        .join(ledger_directory)
        .join(sha256_hex(authorization_sha256)))
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hash_serializable(value: &impl Serialize, label: &str) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("failed to serialize {label} for SHA-256: {error}"))?;
    Ok(sha256_prefixed(&bytes))
}

fn normalize_sha256(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("sha256:").unwrap_or(trimmed);
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{field} must contain exactly 64 hexadecimal SHA-256 digits"
        ));
    }
    Ok(format!("sha256:{}", hex.to_ascii_lowercase()))
}

fn sha256_hex(value: &str) -> &str {
    value.strip_prefix("sha256:").unwrap_or(value)
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

#[cfg(test)]
std::thread_local! {
    static PLAIN_READ_OPEN_COUNT: std::cell::Cell<usize> = std::cell::Cell::new(0);
}

#[cfg(test)]
fn record_plain_read_open() {
    PLAIN_READ_OPEN_COUNT.with(|count| count.set(count.get().saturating_add(1)));
}

#[cfg(test)]
fn reset_plain_read_open_count() {
    PLAIN_READ_OPEN_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
fn plain_read_open_count() -> usize {
    PLAIN_READ_OPEN_COUNT.with(std::cell::Cell::get)
}

#[cfg(not(test))]
fn record_plain_read_open() {}

#[cfg(windows)]
fn open_plain_read_handle(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    let result = OpenOptions::new()
        .read(true)
        // Durable records are immutable. Denying new write/delete handles while
        // this read is active prevents a pathname replacement from changing the
        // bytes or discriminator being validated.
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path);
    if result.is_ok() {
        record_plain_read_open();
    }
    result
}

#[cfg(not(windows))]
fn open_plain_read_handle(path: &Path) -> io::Result<File> {
    let result = OpenOptions::new().read(true).open(path);
    if result.is_ok() {
        record_plain_read_open();
    }
    result
}

#[derive(Debug)]
enum BoundedHandleReadError {
    Invalid,
    Io(io::Error),
}

fn read_bounded_from_open_file(
    file: File,
    observed_len: u64,
    max_bytes: u64,
) -> Result<Vec<u8>, BoundedHandleReadError> {
    if observed_len > max_bytes {
        return Err(BoundedHandleReadError::Invalid);
    }
    let mut bytes = Vec::with_capacity(observed_len.min(max_bytes) as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(BoundedHandleReadError::Io)?;
    if bytes.len() as u64 > max_bytes {
        return Err(BoundedHandleReadError::Invalid);
    }
    Ok(bytes)
}

#[derive(Debug)]
enum TerminalHandleReadError {
    Invalid,
    OversizedWithoutCompletedDiscriminator,
    Io(io::Error),
}

fn read_terminal_bytes_from_open_file(
    mut file: File,
    observed_len: u64,
) -> Result<Vec<u8>, TerminalHandleReadError> {
    const COMPLETED_PREFIX: &[u8] = b"{\n  \"state\": \"completed\",";
    if observed_len > MAX_COMPLETED_RECORD_BYTES {
        return Err(TerminalHandleReadError::Invalid);
    }
    if observed_len <= MAX_FAILED_RECORD_BYTES {
        return read_bounded_from_open_file(file, observed_len, MAX_FAILED_RECORD_BYTES).map_err(
            |error| match error {
                BoundedHandleReadError::Invalid => TerminalHandleReadError::Invalid,
                BoundedHandleReadError::Io(error) => TerminalHandleReadError::Io(error),
            },
        );
    }

    // Length, discriminator, and body all come from this one handle. `read_exact`
    // avoids treating a legal short read as a mismatched completed prefix.
    let mut bytes = Vec::with_capacity(observed_len as usize);
    let mut prefix = [0u8; COMPLETED_PREFIX.len()];
    match file.read_exact(&mut prefix) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => {
            return Err(TerminalHandleReadError::Invalid)
        }
        Err(error) => return Err(TerminalHandleReadError::Io(error)),
    }
    if prefix.as_slice() != COMPLETED_PREFIX {
        return Err(TerminalHandleReadError::OversizedWithoutCompletedDiscriminator);
    }
    bytes.extend_from_slice(&prefix);
    file.take(
        MAX_COMPLETED_RECORD_BYTES
            .saturating_add(1)
            .saturating_sub(prefix.len() as u64),
    )
    .read_to_end(&mut bytes)
    .map_err(TerminalHandleReadError::Io)?;
    if bytes.len() as u64 > MAX_COMPLETED_RECORD_BYTES {
        return Err(TerminalHandleReadError::Invalid);
    }
    Ok(bytes)
}

fn ensure_metadata_is_plain(
    path: &Path,
    metadata: &fs::Metadata,
    label: &str,
    expect_directory: bool,
) -> Result<(), String> {
    let kind_matches = if expect_directory {
        metadata.is_dir()
    } else {
        metadata.is_file()
    };
    if metadata.file_type().is_symlink() || metadata_is_reparse_point(metadata) || !kind_matches {
        let expected = if expect_directory {
            "directory"
        } else {
            "file"
        };
        return Err(format!(
            "{label} must be an ordinary non-reparse {expected}: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_existing_directory_ancestors_are_plain(path: &Path, label: &str) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!(
            "{label} must use an absolute path: {}",
            path.display()
        ));
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => ensure_metadata_is_plain(ancestor, &metadata, label, true)?,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(format!(
                    "failed to inspect {label} ancestor {}: {error}",
                    ancestor.display()
                ));
            }
        }
    }
    Ok(())
}

fn durable_probe_unavailable(error: &io::Error) -> DurableProbeReadError {
    #[cfg(windows)]
    if matches!(error.raw_os_error(), Some(32 | 33)) {
        return DurableProbeReadError::Unavailable(DurableProbeUnavailable {
            kind: DurableProbeUnavailableKind::SharingViolation,
        });
    }
    let kind = if error.kind() == ErrorKind::PermissionDenied {
        DurableProbeUnavailableKind::PermissionDenied
    } else {
        DurableProbeUnavailableKind::Io
    };
    DurableProbeReadError::Unavailable(DurableProbeUnavailable { kind })
}

fn probe_existing_directory_ancestors_are_plain(path: &Path) -> Result<(), DurableProbeReadError> {
    if !path.is_absolute() {
        return Err(DurableProbeReadError::Blocked(
            DurableProbeBlockedReason::Invalid,
        ));
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink()
                    || metadata_is_reparse_point(&metadata)
                    || !metadata.is_dir()
                {
                    return Err(DurableProbeReadError::Blocked(
                        DurableProbeBlockedReason::Invalid,
                    ));
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => return Err(durable_probe_unavailable(&error)),
        }
    }
    Ok(())
}

fn probe_optional_plain_metadata(
    path: &Path,
    expect_directory: bool,
) -> Result<Option<fs::Metadata>, DurableProbeReadError> {
    let parent = path.parent().ok_or(DurableProbeReadError::Blocked(
        DurableProbeBlockedReason::Invalid,
    ))?;
    probe_existing_directory_ancestors_are_plain(parent)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let kind_matches = if expect_directory {
                metadata.is_dir()
            } else {
                metadata.is_file()
            };
            if metadata.file_type().is_symlink()
                || metadata_is_reparse_point(&metadata)
                || !kind_matches
            {
                return Err(DurableProbeReadError::Blocked(
                    DurableProbeBlockedReason::Invalid,
                ));
            }
            Ok(Some(metadata))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(durable_probe_unavailable(&error)),
    }
}

#[cfg(unix)]
fn open_handle_still_matches_path(path: &Path, handle_metadata: &fs::Metadata) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let path_metadata = fs::symlink_metadata(path)?;
    Ok(!path_metadata.file_type().is_symlink()
        && path_metadata.is_file()
        && path_metadata.dev() == handle_metadata.dev()
        && path_metadata.ino() == handle_metadata.ino())
}

#[cfg(not(unix))]
fn open_handle_still_matches_path(
    _path: &Path,
    _handle_metadata: &fs::Metadata,
) -> io::Result<bool> {
    // Windows denies write/delete sharing on the opened handle and validates
    // FILE_ATTRIBUTE_REPARSE_POINT from that handle's metadata.
    Ok(true)
}

fn probe_open_plain_read_file(path: &Path) -> Result<Option<(File, u64)>, DurableProbeReadError> {
    // Preserve the semantic distinction between never-observed state and a
    // record that vanished while it was being opened.
    let Some(_) = probe_optional_plain_metadata(path, false)? else {
        return Ok(None);
    };
    let parent = path.parent().ok_or(DurableProbeReadError::Blocked(
        DurableProbeBlockedReason::Invalid,
    ))?;
    let file = open_plain_read_handle(path).map_err(|error| durable_probe_unavailable(&error))?;
    let metadata = file
        .metadata()
        .map_err(|error| durable_probe_unavailable(&error))?;
    if metadata.file_type().is_symlink()
        || metadata_is_reparse_point(&metadata)
        || !metadata.is_file()
    {
        return Err(DurableProbeReadError::Blocked(
            DurableProbeBlockedReason::Invalid,
        ));
    }
    probe_existing_directory_ancestors_are_plain(parent)?;
    match open_handle_still_matches_path(path, &metadata) {
        Ok(true) => {}
        Ok(false) => {
            return Err(DurableProbeReadError::Unavailable(
                DurableProbeUnavailable {
                    kind: DurableProbeUnavailableKind::Io,
                },
            ))
        }
        Err(error) => return Err(durable_probe_unavailable(&error)),
    }
    Ok(Some((file, metadata.len())))
}

fn probe_read_bounded(
    path: &Path,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, DurableProbeReadError> {
    let Some((file, observed_len)) = probe_open_plain_read_file(path)? else {
        return Ok(None);
    };
    match read_bounded_from_open_file(file, observed_len, max_bytes) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(BoundedHandleReadError::Invalid) => Err(DurableProbeReadError::Blocked(
            DurableProbeBlockedReason::Invalid,
        )),
        Err(BoundedHandleReadError::Io(error)) => Err(durable_probe_unavailable(&error)),
    }
}

fn ensure_plain_directory(path: &Path, label: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} has no parent directory: {}", path.display()))?;
    ensure_existing_directory_ancestors_are_plain(parent, label)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure_metadata_is_plain(path, &metadata, label, true)?;
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            match fs::create_dir(path) {
                Ok(()) => {}
                Err(create_error) if create_error.kind() == ErrorKind::AlreadyExists => {}
                Err(create_error) => {
                    return Err(format!(
                        "failed to create {label} {}: {create_error}",
                        path.display()
                    ));
                }
            }
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                format!("failed to inspect {label} {}: {error}", path.display())
            })?;
            ensure_metadata_is_plain(path, &metadata, label, true)?;
        }
        Err(error) => {
            return Err(format!(
                "failed to inspect {label} {}: {error}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn ensure_ledger_root(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    stage_label: &str,
) -> Result<PathBuf, String> {
    ensure_plain_directory(app_local_data_dir, "application data directory")?;
    let root = app_local_data_dir.join(ledger_directory);
    ensure_plain_directory(&root, &format!("VCET temporal {stage_label} ledger root"))?;
    Ok(root)
}

fn authorization_dir(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    stage_label: &str,
    authorization_sha256: &str,
) -> Result<PathBuf, String> {
    let root = ensure_ledger_root(app_local_data_dir, ledger_directory, stage_label)?;
    let dir = root.join(sha256_hex(authorization_sha256));
    ensure_plain_directory(
        &dir,
        &format!("VCET temporal {stage_label} authorization directory"),
    )?;
    Ok(dir)
}

fn optional_existing_authorization_dir(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    stage_label: &str,
    authorization_sha256: &str,
) -> Result<Option<PathBuf>, String> {
    let root = app_local_data_dir.join(ledger_directory);
    ensure_existing_directory_ancestors_are_plain(
        app_local_data_dir,
        "application data directory",
    )?;
    let app_metadata = fs::symlink_metadata(app_local_data_dir).map_err(|error| {
        format!(
            "failed to inspect application data directory {}: {error}",
            app_local_data_dir.display()
        )
    })?;
    ensure_metadata_is_plain(
        app_local_data_dir,
        &app_metadata,
        "application data directory",
        true,
    )?;

    ensure_existing_directory_ancestors_are_plain(&root, "VCET temporal durable ledger root")?;
    let root_metadata = match fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to inspect VCET temporal durable ledger root {}: {error}",
                root.display()
            ))
        }
    };
    ensure_metadata_is_plain(
        &root,
        &root_metadata,
        "VCET temporal durable ledger root",
        true,
    )?;

    let dir = root.join(sha256_hex(authorization_sha256));
    let metadata = match fs::symlink_metadata(&dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to inspect VCET durable {stage_label} authorization directory {}: {error}",
                dir.display()
            ))
        }
    };
    ensure_metadata_is_plain(
        &dir,
        &metadata,
        &format!("VCET temporal {stage_label} authorization directory"),
        true,
    )?;
    Ok(Some(dir))
}

fn existing_authorization_dir(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    stage_label: &str,
    authorization_sha256: &str,
) -> Result<PathBuf, String> {
    optional_existing_authorization_dir(
        app_local_data_dir,
        ledger_directory,
        stage_label,
        authorization_sha256,
    )?
    .ok_or_else(|| {
        format!(
            "VCET durable {stage_label} authorization has no existing ledger directory for {authorization_sha256}"
        )
    })
}

fn open_plain_read_file(path: &Path, label: &str) -> Result<Option<(File, u64)>, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} path has no parent: {}", path.display()))?;
    ensure_existing_directory_ancestors_are_plain(parent, label)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => ensure_metadata_is_plain(path, &metadata, label, false)?,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to inspect {label} {}: {error}",
                path.display()
            ))
        }
    }
    let file = open_plain_read_handle(path)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    ensure_metadata_is_plain(path, &metadata, label, false)?;
    ensure_existing_directory_ancestors_are_plain(parent, label)?;
    if !open_handle_still_matches_path(path, &metadata)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?
    {
        return Err(format!(
            "{label} changed while it was being opened: {}",
            path.display()
        ));
    }
    Ok(Some((file, metadata.len())))
}

fn read_bounded_optional(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<Option<Vec<u8>>, String> {
    let Some((file, observed_len)) = open_plain_read_file(path, label)? else {
        return Ok(None);
    };
    match read_bounded_from_open_file(file, observed_len, max_bytes) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(BoundedHandleReadError::Invalid) => Err(format!(
            "{label} exceeds the {} byte safety limit",
            max_bytes
        )),
        Err(BoundedHandleReadError::Io(error)) => Err(format!(
            "failed to read {label} {}: {error}",
            path.display()
        )),
    }
}

fn read_bounded(path: &Path, label: &str, max_bytes: u64) -> Result<Vec<u8>, String> {
    read_bounded_optional(path, label, max_bytes)?.ok_or_else(|| {
        format!(
            "failed to inspect {label} {}: file does not exist",
            path.display()
        )
    })
}

fn write_create_new(path: &Path, bytes: &[u8], max_bytes: u64, label: &str) -> Result<(), String> {
    if bytes.len() as u64 > max_bytes {
        return Err(format!(
            "{label} requires {} bytes, exceeding the {max_bytes} byte publish limit",
            bytes.len()
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("failed to create {label} {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("failed to write {label} {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to sync {label} {}: {error}", path.display()))
}

#[cfg(windows)]
fn sync_parent_namespace_best_effort(path: &Path) {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    if let Ok(directory) = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
    {
        let _ = directory.sync_all();
    }
}

#[cfg(not(windows))]
fn sync_parent_namespace_best_effort(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

fn immutable_publish(path: &Path, bytes: &[u8], max_bytes: u64, label: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} path has no parent directory"))?;
    ensure_existing_directory_ancestors_are_plain(parent, label)?;
    let sequence = LEDGER_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let stage = parent.join(format!(
        ".{label}.{}.{}.{}.tmp",
        std::process::id(),
        unix_ms(),
        sequence
    ));
    write_create_new(&stage, bytes, max_bytes, &format!("staged {label}"))?;
    let publish = fs::hard_link(&stage, path).map_err(|error| {
        format!(
            "failed to publish immutable {label} {}: {error}",
            path.display()
        )
    });
    if publish.is_ok() {
        sync_parent_namespace_best_effort(parent);
    }
    let _ = fs::remove_file(&stage);
    sync_parent_namespace_best_effort(parent);
    publish
}

fn build_claim_record(
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<DurableClaimRecordV1, String> {
    let claimed_at_unix_ms = unix_ms();
    let claimant_process_id = std::process::id();
    let claim_sha256 = hash_serializable(
        &ClaimHashInput {
            hash_contract: CLAIM_HASH_CONTRACT,
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256,
            request_sha256,
            claimed_at_unix_ms,
            claimant_process_id,
        },
        "VCET durable final claim",
    )?;
    Ok(DurableClaimRecordV1 {
        schema_version: LEDGER_SCHEMA_VERSION,
        authorization_sha256: authorization_sha256.to_string(),
        request_sha256: request_sha256.to_string(),
        claimed_at_unix_ms,
        claimant_process_id,
        claim_sha256,
    })
}

fn validate_claim(
    record: &DurableClaimRecordV1,
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<(), String> {
    let expected_hash = hash_serializable(
        &ClaimHashInput {
            hash_contract: CLAIM_HASH_CONTRACT,
            schema_version: record.schema_version,
            authorization_sha256: &record.authorization_sha256,
            request_sha256: &record.request_sha256,
            claimed_at_unix_ms: record.claimed_at_unix_ms,
            claimant_process_id: record.claimant_process_id,
        },
        "VCET durable final claim",
    )?;
    if record.schema_version != LEDGER_SCHEMA_VERSION
        || record.authorization_sha256 != authorization_sha256
        || record.request_sha256 != request_sha256
        || record.claim_sha256 != expected_hash
    {
        return Err("VCET durable final claim identity or integrity mismatch".to_string());
    }
    Ok(())
}

fn completed_hash<T: Serialize>(
    authorization_sha256: &str,
    request_sha256: &str,
    response: &T,
) -> Result<String, String> {
    hash_serializable(
        &CompletedHashInput {
            hash_contract: COMPLETED_HASH_CONTRACT,
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256,
            request_sha256,
            response,
        },
        "VCET durable final completed response",
    )
}

fn failed_hash(
    authorization_sha256: &str,
    request_sha256: &str,
    error: &str,
) -> Result<String, String> {
    hash_serializable(
        &FailedHashInput {
            hash_contract: FAILED_HASH_CONTRACT,
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256,
            request_sha256,
            error,
        },
        "VCET durable failed terminal",
    )
}

fn plain_file_len(path: &Path, label: &str) -> Result<Option<u64>, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} path has no parent: {}", path.display()))?;
    ensure_existing_directory_ancestors_are_plain(parent, label)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure_metadata_is_plain(path, &metadata, label, false)?;
            Ok(Some(metadata.len()))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "failed to inspect {label} {}: {error}",
            path.display()
        )),
    }
}

fn plain_file_exists(path: &Path, label: &str) -> Result<bool, String> {
    plain_file_len(path, label).map(|length| length.is_some())
}

fn read_terminal_bytes(path: &Path, label: &str) -> Result<Option<Vec<u8>>, String> {
    let Some((file, observed_len)) = open_plain_read_file(path, label)? else {
        return Ok(None);
    };
    match read_terminal_bytes_from_open_file(file, observed_len) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(TerminalHandleReadError::Invalid) => Err(format!(
            "{label} is invalid or exceeds the {} byte completed-terminal safety limit",
            MAX_COMPLETED_RECORD_BYTES
        )),
        Err(TerminalHandleReadError::OversizedWithoutCompletedDiscriminator) => Err(format!(
            "{label} exceeds the {} byte failed-terminal limit without the canonical completed discriminator",
            MAX_FAILED_RECORD_BYTES
        )),
        Err(TerminalHandleReadError::Io(error)) => Err(format!(
            "failed to read {label} {}: {error}",
            path.display()
        )),
    }
}

fn read_claim(
    dir: &Path,
    stage_label: &str,
    authorization_sha256: &str,
) -> Result<DurableClaimRecordV1, String> {
    let claim_path = dir.join(CLAIM_FILE);
    let claim_bytes = read_bounded(
        &claim_path,
        &format!("VCET durable {stage_label} claim record"),
        MAX_CLAIM_RECORD_BYTES,
    )?;
    let claim: DurableClaimRecordV1 = serde_json::from_slice(&claim_bytes)
        .map_err(|error| format!("invalid VCET durable {stage_label} claim JSON: {error}"))?;
    let request_sha256 = normalize_sha256(&claim.request_sha256, "claim.request_sha256")?;
    validate_claim(&claim, authorization_sha256, &request_sha256)?;
    Ok(claim)
}

fn read_terminal<T: DeserializeOwned + Serialize>(
    dir: &Path,
    stage_label: &str,
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<T, String> {
    let terminal_path = dir.join(TERMINAL_FILE);
    let terminal_label = format!("VCET durable {stage_label} terminal record");
    let Some(bytes) = read_terminal_bytes(&terminal_path, &terminal_label)? else {
        return Err(format!(
            "VCET {stage_label} authorization already has an uncompleted durable claim; the prior execution is still active or crashed, so the stage remains fail-closed"
        ));
    };
    let record: DurableTerminalRecordV2<T> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid VCET durable {stage_label} terminal JSON: {error}"))?;
    match record {
        DurableTerminalRecordV2::Completed {
            schema_version,
            authorization_sha256: record_authorization,
            request_sha256: record_request,
            response,
            terminal_sha256,
        } => {
            let expected_hash = completed_hash(&record_authorization, &record_request, &response)?;
            if schema_version != LEDGER_SCHEMA_VERSION
                || record_authorization != authorization_sha256
                || record_request != request_sha256
                || terminal_sha256 != expected_hash
            {
                return Err(format!(
                    "VCET durable {stage_label} completed terminal identity or integrity mismatch"
                ));
            }
            Ok(response)
        }
        DurableTerminalRecordV2::Failed {
            schema_version,
            authorization_sha256: record_authorization,
            request_sha256: record_request,
            error,
            terminal_sha256,
        } => {
            if bytes.len() as u64 > MAX_FAILED_RECORD_BYTES {
                return Err(format!(
                    "VCET durable {stage_label} failed terminal exceeds the {} byte failed-terminal limit",
                    MAX_FAILED_RECORD_BYTES
                ));
            }
            let expected_hash = failed_hash(&record_authorization, &record_request, &error)?;
            if schema_version != LEDGER_SCHEMA_VERSION
                || record_authorization != authorization_sha256
                || record_request != request_sha256
                || terminal_sha256 != expected_hash
            {
                return Err(format!(
                    "VCET durable {stage_label} failed terminal identity or integrity mismatch"
                ));
            }
            Err(format!(
                "VCET {stage_label} authorization is durably failed and cannot be reopened: {error}"
            ))
        }
    }
}

fn accept_existing_failed_terminal(
    dir: &Path,
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<(), String> {
    let terminal_path = dir.join(TERMINAL_FILE);
    let Some((file, observed_len)) =
        open_plain_read_file(&terminal_path, "VCET durable terminal record")?
    else {
        return Err("VCET durable terminal was not published".to_string());
    };
    if observed_len > MAX_FAILED_RECORD_BYTES {
        return Err(format!(
            "existing VCET durable terminal cannot be accepted as failed because it exceeds the {} byte failed-terminal limit",
            MAX_FAILED_RECORD_BYTES
        ));
    }
    let bytes = match read_bounded_from_open_file(file, observed_len, MAX_FAILED_RECORD_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedHandleReadError::Invalid) => {
            return Err(format!(
                "existing VCET durable terminal cannot be accepted as failed because it exceeds the {} byte failed-terminal limit",
                MAX_FAILED_RECORD_BYTES
            ))
        }
        Err(BoundedHandleReadError::Io(error)) => {
            return Err(format!(
                "failed to read VCET durable terminal record {}: {error}",
                terminal_path.display()
            ))
        }
    };
    let record: DurableTerminalRecordV2<serde_json::Value> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid VCET durable terminal JSON: {error}"))?;
    match record {
        DurableTerminalRecordV2::Completed { .. } => Err(
            "VCET durable authorization already contains a completed terminal; failure publication was rejected"
                .to_string(),
        ),
        DurableTerminalRecordV2::Failed {
            schema_version,
            authorization_sha256: record_authorization,
            request_sha256: record_request,
            error,
            terminal_sha256,
        } => {
            let expected_hash = failed_hash(&record_authorization, &record_request, &error)?;
            if schema_version != LEDGER_SCHEMA_VERSION
                || record_authorization != authorization_sha256
                || record_request != request_sha256
                || terminal_sha256 != expected_hash
            {
                return Err(
                    "VCET durable failed terminal identity or integrity mismatch".to_string(),
                );
            }
            Ok(())
        }
    }
}

fn existing_admission<T: DeserializeOwned + Serialize>(
    dir: &Path,
    stage_label: &str,
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<DurableFinalAdmission<T>, String> {
    let claim = read_claim(dir, stage_label, authorization_sha256)?;
    validate_claim(&claim, authorization_sha256, request_sha256)?;
    read_terminal(dir, stage_label, authorization_sha256, request_sha256)
        .map(DurableFinalAdmission::Replay)
}

fn admit_durable_authorization<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    stage_label: &str,
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<DurableFinalAdmission<T>, String> {
    let authorization_sha256 = normalize_sha256(authorization_sha256, "authorization_sha256")?;
    let request_sha256 = normalize_sha256(request_sha256, "request_sha256")?;
    ensure_plain_directory(app_local_data_dir, "application data directory")?;
    let authorization_identity = canonical_live_authorization_identity(
        app_local_data_dir,
        ledger_directory,
        &authorization_sha256,
    )
    .map_err(|error| error.to_string())?;
    // Reserve before creating the ledger/authorization directories. A probe
    // can therefore classify every filesystem-publication window as live,
    // including the interval before claim.json exists.
    let live_registration = LiveDurableAuthorizationRegistration::acquire(authorization_identity)
        .map_err(|error| error.to_string())?;
    let dir = authorization_dir(
        app_local_data_dir,
        ledger_directory,
        stage_label,
        &authorization_sha256,
    )?;
    let claim_path = dir.join(CLAIM_FILE);
    if plain_file_exists(
        &claim_path,
        &format!("VCET durable {stage_label} claim record"),
    )? {
        return existing_admission(&dir, stage_label, &authorization_sha256, &request_sha256);
    }
    if plain_file_exists(
        &dir.join(TERMINAL_FILE),
        &format!("VCET durable {stage_label} terminal record"),
    )? {
        return Err(format!(
            "VCET durable {stage_label} ledger has a terminal state without its immutable claim"
        ));
    }

    let claim_record = build_claim_record(&authorization_sha256, &request_sha256)?;
    let claim_bytes = serde_json::to_vec_pretty(&claim_record)
        .map_err(|error| format!("failed to encode VCET durable claim: {error}"))?;
    match immutable_publish(&claim_path, &claim_bytes, MAX_CLAIM_RECORD_BYTES, "claim") {
        Ok(()) => Ok(DurableFinalAdmission::Acquired(DurableFinalClaim {
            authorization_dir: dir,
            authorization_sha256,
            request_sha256,
            live_registration,
        })),
        Err(publish_error) => match plain_file_exists(
            &claim_path,
            &format!("VCET durable {stage_label} claim record"),
        ) {
            Ok(true) => {
                existing_admission(&dir, stage_label, &authorization_sha256, &request_sha256)
            }
            Ok(false) => Err(publish_error),
            Err(inspect_error) => Err(format!("{publish_error}; {inspect_error}")),
        },
    }
}

pub(crate) fn admit_durable_final_authorization<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    authorization_sha256: &str,
    request_sha256: &str,
) -> Result<DurableFinalAdmission<T>, String> {
    admit_durable_authorization(
        app_local_data_dir,
        FINAL_LEDGER_DIRECTORY,
        "final-test",
        authorization_sha256,
        request_sha256,
    )
}

pub(crate) fn admit_durable_pre_final_winner_lock<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    universe_sha256: &str,
    request_sha256: &str,
) -> Result<DurableFinalAdmission<T>, String> {
    admit_durable_authorization(
        app_local_data_dir,
        PRE_FINAL_LEDGER_DIRECTORY,
        "pre-final winner-lock",
        universe_sha256,
        request_sha256,
    )
}

fn probe_durable_completion_for_authorization_inner<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    authorization_sha256: &str,
) -> Result<DurableCompletionProbe<T>, DurableProbeReadError> {
    let authorization_sha256 = match normalize_sha256(authorization_sha256, "authorization_sha256")
    {
        Ok(value) => value,
        Err(_) => {
            return Ok(DurableCompletionProbe::Blocked(
                DurableProbeBlockedReason::Invalid,
            ))
        }
    };
    let Some(_) = probe_optional_plain_metadata(app_local_data_dir, true)? else {
        return Ok(DurableCompletionProbe::Missing);
    };
    let authorization_identity = canonical_live_authorization_identity(
        app_local_data_dir,
        ledger_directory,
        &authorization_sha256,
    )
    .map_err(DurableProbeReadError::Unavailable)?;
    let root = app_local_data_dir.join(ledger_directory);
    if probe_optional_plain_metadata(&root, true)?.is_none() {
        let live = lock_live_durable_authorizations();
        if live.contains(&authorization_identity) {
            return Err(DurableProbeReadError::Unavailable(live_claim_unavailable()));
        }
        if probe_optional_plain_metadata(&root, true)?.is_none() {
            return Ok(DurableCompletionProbe::Missing);
        }
    }
    let dir = root.join(sha256_hex(&authorization_sha256));
    if probe_optional_plain_metadata(&dir, true)?.is_none() {
        let live = lock_live_durable_authorizations();
        if live.contains(&authorization_identity) {
            return Err(DurableProbeReadError::Unavailable(live_claim_unavailable()));
        }
        if probe_optional_plain_metadata(&dir, true)?.is_none() {
            return Ok(DurableCompletionProbe::Missing);
        }
    }

    let claim_path = dir.join(CLAIM_FILE);
    let claim_bytes = match probe_read_bounded(&claim_path, MAX_CLAIM_RECORD_BYTES)? {
        Some(bytes) => bytes,
        None => {
            let live = lock_live_durable_authorizations();
            if live.contains(&authorization_identity) {
                return Err(DurableProbeReadError::Unavailable(live_claim_unavailable()));
            }

            // Admission reserves the live registry before publishing the
            // immutable claim. If a full claim/terminal publication completed
            // between the first read and the registry lookup, observe it on a
            // second bounded read rather than reporting stale `Missing` state.
            match probe_read_bounded(&claim_path, MAX_CLAIM_RECORD_BYTES)? {
                Some(bytes) => bytes,
                None => {
                    return if probe_optional_plain_metadata(&dir.join(TERMINAL_FILE), false)?
                        .is_some()
                    {
                        Ok(DurableCompletionProbe::Blocked(
                            DurableProbeBlockedReason::Invalid,
                        ))
                    } else {
                        Ok(DurableCompletionProbe::Missing)
                    }
                }
            }
        }
    };
    let claim: DurableClaimRecordV1 = match serde_json::from_slice(&claim_bytes) {
        Ok(value) => value,
        Err(_) => {
            return Ok(DurableCompletionProbe::Blocked(
                DurableProbeBlockedReason::Invalid,
            ))
        }
    };
    let request_sha256 = match normalize_sha256(&claim.request_sha256, "claim.request_sha256") {
        Ok(value) => value,
        Err(_) => {
            return Ok(DurableCompletionProbe::Blocked(
                DurableProbeBlockedReason::Invalid,
            ))
        }
    };
    if validate_claim(&claim, &authorization_sha256, &request_sha256).is_err() {
        return Ok(DurableCompletionProbe::Blocked(
            DurableProbeBlockedReason::Invalid,
        ));
    }

    let terminal_path = dir.join(TERMINAL_FILE);
    let (terminal_file, terminal_len) = match probe_open_plain_read_file(&terminal_path)? {
        Some(opened) => opened,
        None => {
            let live = lock_live_durable_authorizations();
            if live.contains(&authorization_identity) {
                return Err(DurableProbeReadError::Unavailable(live_claim_unavailable()));
            }

            // A claimant publishes and syncs the terminal before releasing
            // its registry lease. If it completed between the first open
            // and the registry lookup, this second open observes the
            // terminal instead of permanently misclassifying the claim as
            // interrupted.
            match probe_open_plain_read_file(&terminal_path)? {
                Some(opened) => opened,
                None => {
                    return Ok(DurableCompletionProbe::Blocked(
                        DurableProbeBlockedReason::Interrupted,
                    ))
                }
            }
        }
    };
    let terminal_bytes = match read_terminal_bytes_from_open_file(terminal_file, terminal_len) {
        Ok(bytes) => bytes,
        Err(
            TerminalHandleReadError::Invalid
            | TerminalHandleReadError::OversizedWithoutCompletedDiscriminator,
        ) => {
            return Ok(DurableCompletionProbe::Blocked(
                DurableProbeBlockedReason::Invalid,
            ))
        }
        Err(TerminalHandleReadError::Io(error)) => return Err(durable_probe_unavailable(&error)),
    };
    let record: DurableTerminalRecordV2<T> = match serde_json::from_slice(&terminal_bytes) {
        Ok(value) => value,
        Err(_) => {
            return Ok(DurableCompletionProbe::Blocked(
                DurableProbeBlockedReason::Invalid,
            ))
        }
    };
    match record {
        DurableTerminalRecordV2::Completed {
            schema_version,
            authorization_sha256: record_authorization,
            request_sha256: record_request,
            response,
            terminal_sha256,
        } => {
            let expected_hash =
                match completed_hash(&record_authorization, &record_request, &response) {
                    Ok(value) => value,
                    Err(_) => {
                        return Ok(DurableCompletionProbe::Blocked(
                            DurableProbeBlockedReason::Invalid,
                        ))
                    }
                };
            if schema_version != LEDGER_SCHEMA_VERSION
                || record_authorization != authorization_sha256
                || record_request != request_sha256
                || terminal_sha256 != expected_hash
            {
                return Ok(DurableCompletionProbe::Blocked(
                    DurableProbeBlockedReason::Invalid,
                ));
            }
            Ok(DurableCompletionProbe::Completed {
                request_sha256,
                response,
            })
        }
        DurableTerminalRecordV2::Failed {
            schema_version,
            authorization_sha256: record_authorization,
            request_sha256: record_request,
            error,
            terminal_sha256,
        } => {
            if terminal_bytes.len() as u64 > MAX_FAILED_RECORD_BYTES {
                return Ok(DurableCompletionProbe::Blocked(
                    DurableProbeBlockedReason::Invalid,
                ));
            }
            let expected_hash = match failed_hash(&record_authorization, &record_request, &error) {
                Ok(value) => value,
                Err(_) => {
                    return Ok(DurableCompletionProbe::Blocked(
                        DurableProbeBlockedReason::Invalid,
                    ))
                }
            };
            let reason = if schema_version == LEDGER_SCHEMA_VERSION
                && record_authorization == authorization_sha256
                && record_request == request_sha256
                && terminal_sha256 == expected_hash
            {
                DurableProbeBlockedReason::Failed
            } else {
                DurableProbeBlockedReason::Invalid
            };
            Ok(DurableCompletionProbe::Blocked(reason))
        }
    }
}

fn probe_durable_completion_for_authorization<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    authorization_sha256: &str,
) -> Result<DurableCompletionProbe<T>, DurableProbeUnavailable> {
    match probe_durable_completion_for_authorization_inner(
        app_local_data_dir,
        ledger_directory,
        authorization_sha256,
    ) {
        Ok(value) => Ok(value),
        Err(DurableProbeReadError::Blocked(reason)) => Ok(DurableCompletionProbe::Blocked(reason)),
        Err(DurableProbeReadError::Unavailable(error)) => Err(error),
    }
}

pub(crate) fn probe_durable_final_completion_for_authorization<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    authorization_sha256: &str,
) -> Result<DurableCompletionProbe<T>, DurableProbeUnavailable> {
    probe_durable_completion_for_authorization(
        app_local_data_dir,
        FINAL_LEDGER_DIRECTORY,
        authorization_sha256,
    )
}

pub(crate) fn probe_durable_pre_final_winner_lock<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    universe_sha256: &str,
) -> Result<DurableCompletionProbe<T>, DurableProbeUnavailable> {
    probe_durable_completion_for_authorization(
        app_local_data_dir,
        PRE_FINAL_LEDGER_DIRECTORY,
        universe_sha256,
    )
}

fn try_load_durable_completion_for_authorization<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    ledger_directory: &str,
    stage_label: &str,
    authorization_sha256: &str,
) -> Result<Option<(String, T)>, String> {
    let authorization_sha256 = normalize_sha256(authorization_sha256, "authorization_sha256")?;
    let Some(dir) = optional_existing_authorization_dir(
        app_local_data_dir,
        ledger_directory,
        stage_label,
        &authorization_sha256,
    )?
    else {
        return Ok(None);
    };
    let claim = read_claim(&dir, stage_label, &authorization_sha256)?;
    let response = read_terminal(
        &dir,
        stage_label,
        &authorization_sha256,
        &claim.request_sha256,
    )?;
    Ok(Some((claim.request_sha256, response)))
}

pub(crate) fn try_load_durable_final_completion_for_authorization<
    T: DeserializeOwned + Serialize,
>(
    app_local_data_dir: &Path,
    authorization_sha256: &str,
) -> Result<Option<(String, T)>, String> {
    try_load_durable_completion_for_authorization(
        app_local_data_dir,
        FINAL_LEDGER_DIRECTORY,
        "final-test",
        authorization_sha256,
    )
}

pub(crate) fn load_durable_final_completion_for_authorization<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    authorization_sha256: &str,
) -> Result<(String, T), String> {
    try_load_durable_final_completion_for_authorization(app_local_data_dir, authorization_sha256)?
        .ok_or_else(|| {
            format!(
                "VCET durable final-test authorization has no existing ledger directory for {authorization_sha256}"
            )
        })
}

pub(crate) fn load_durable_pre_final_winner_lock<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    universe_sha256: &str,
) -> Result<T, String> {
    let universe_sha256 = normalize_sha256(universe_sha256, "universe_sha256")?;
    let dir = existing_authorization_dir(
        app_local_data_dir,
        PRE_FINAL_LEDGER_DIRECTORY,
        "pre-final winner-lock",
        &universe_sha256,
    )?;
    let claim = read_claim(&dir, "pre-final winner-lock", &universe_sha256)?;
    read_terminal(
        &dir,
        "pre-final winner-lock",
        &universe_sha256,
        &claim.request_sha256,
    )
}

#[cfg(test)]
pub(crate) fn try_load_durable_pre_final_winner_lock<T: DeserializeOwned + Serialize>(
    app_local_data_dir: &Path,
    universe_sha256: &str,
) -> Result<Option<T>, String> {
    let universe_sha256 = normalize_sha256(universe_sha256, "universe_sha256")?;
    try_load_durable_completion_for_authorization(
        app_local_data_dir,
        PRE_FINAL_LEDGER_DIRECTORY,
        "pre-final winner-lock",
        &universe_sha256,
    )
    .map(|loaded| loaded.map(|(_request_sha256, response)| response))
}

impl DurableFinalClaim {
    pub(crate) fn complete<T: Serialize>(&self, response: &T) -> Result<(), String> {
        self.complete_with_max_bytes(response, MAX_COMPLETED_RECORD_BYTES)
    }

    fn complete_with_max_bytes<T: Serialize>(
        &self,
        response: &T,
        max_bytes: u64,
    ) -> Result<(), String> {
        let terminal_sha256 =
            completed_hash(&self.authorization_sha256, &self.request_sha256, response)?;
        let record = DurableTerminalRecordV2::Completed {
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256: self.authorization_sha256.clone(),
            request_sha256: self.request_sha256.clone(),
            response,
            terminal_sha256,
        };
        let bytes = serde_json::to_vec_pretty(&record)
            .map_err(|error| format!("failed to encode VCET durable completion: {error}"))?;
        if bytes.len() as u64 > max_bytes {
            let error = format!(
                "VCET durable completion requires {} bytes, exceeding the {max_bytes} byte publish limit",
                bytes.len()
            );
            self.fail(&error).map_err(|failure_error| {
                format!("{error}; failed to durably publish the oversize failure: {failure_error}")
            })?;
            return Err(error);
        }
        let publish = immutable_publish(
            &self.authorization_dir.join(TERMINAL_FILE),
            &bytes,
            max_bytes,
            "completed",
        );
        if publish.is_ok() {
            self.live_registration.release();
        }
        publish
    }

    pub(crate) fn fail(&self, error: &str) -> Result<(), String> {
        let mut error = error.trim().to_string();
        if error.len() > MAX_FAILURE_TEXT_BYTES {
            let mut boundary = MAX_FAILURE_TEXT_BYTES;
            while !error.is_char_boundary(boundary) {
                boundary -= 1;
            }
            error.truncate(boundary);
        }
        if error.is_empty() {
            error = "VCET final-test execution failed without an error message".to_string();
        }
        let terminal_sha256 =
            failed_hash(&self.authorization_sha256, &self.request_sha256, &error)?;
        let record: DurableTerminalRecordV2<serde_json::Value> = DurableTerminalRecordV2::Failed {
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256: self.authorization_sha256.clone(),
            request_sha256: self.request_sha256.clone(),
            error,
            terminal_sha256,
        };
        let bytes = serde_json::to_vec_pretty(&record).map_err(|encode_error| {
            format!("failed to encode VCET durable failure: {encode_error}")
        })?;
        let publish = immutable_publish(
            &self.authorization_dir.join(TERMINAL_FILE),
            &bytes,
            MAX_FAILED_RECORD_BYTES,
            "failed",
        );
        let result = match publish {
            Ok(()) => Ok(()),
            Err(publish_error) => accept_existing_failed_terminal(
                &self.authorization_dir,
                &self.authorization_sha256,
                &self.request_sha256,
            )
            .map_err(|terminal_error| format!("{publish_error}; {terminal_error}")),
        };
        if result.is_ok() {
            self.live_registration.release();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct TestResponse {
        artifact_sha256: String,
        dataset_count: usize,
    }

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vectorgrid_vcet_durable_{label}_{}_{}",
            std::process::id(),
            unix_ms()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp app data");
        dir
    }

    #[cfg(windows)]
    fn open_replaceable_test_file(path: &Path) -> File {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ_WRITE_DELETE: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0004;
        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ_WRITE_DELETE)
            .open(path)
            .expect("open replaceable test file")
    }

    #[cfg(not(windows))]
    fn open_replaceable_test_file(path: &Path) -> File {
        File::open(path).expect("open replaceable test file")
    }

    #[test]
    fn completed_claim_replays_only_the_identical_request() {
        let root = temp_dir("complete_replay");
        let authorization = digest('a');
        let request = digest('b');
        let response = TestResponse {
            artifact_sha256: digest('c'),
            dataset_count: 7,
        };
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("first admission")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("first admission cannot replay"),
        };
        claim.complete(&response).expect("publish completion");

        let replay = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("identical replay")
        {
            DurableFinalAdmission::Replay(value) => value,
            DurableFinalAdmission::Acquired(_) => panic!("completed admission must replay"),
        };
        assert_eq!(replay, response);

        let mismatch =
            admit_durable_final_authorization::<TestResponse>(&root, &authorization, &digest('d'))
                .err()
                .expect("different request must fail");
        assert!(mismatch.contains("identity"));
    }

    #[test]
    fn completed_terminal_has_priority_over_a_same_process_registry_lease() {
        let root = temp_dir("completed_with_live_registry_lease");
        let authorization = digest('a');
        let request = digest('b');
        let response = TestResponse {
            artifact_sha256: digest('c'),
            dataset_count: 11,
        };
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("fresh claim cannot replay"),
        };
        claim.complete(&response).expect("publish terminal");

        let artificial_live_identity =
            canonical_live_authorization_identity(&root, FINAL_LEDGER_DIRECTORY, &authorization)
                .expect("live identity");
        let artificial_live_lease =
            LiveDurableAuthorizationRegistration::acquire(artificial_live_identity)
                .expect("artificial live lease");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &root,
                &authorization,
            )
            .expect("completed probe"),
            DurableCompletionProbe::Completed {
                request_sha256,
                response: observed,
            } if request_sha256 == request && observed == response
        ));
        drop(artificial_live_lease);
    }

    #[test]
    fn interrupted_claim_and_failed_claim_remain_fail_closed() {
        let interrupted_root = temp_dir("interrupted");
        let authorization = digest('1');
        let request = digest('2');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &interrupted_root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        let interrupted = admit_durable_final_authorization::<TestResponse>(
            &interrupted_root,
            &authorization,
            &request,
        )
        .err()
        .expect("live claim must reject a concurrent admission");
        assert!(interrupted.contains("temporarily unavailable"));
        assert!(interrupted.contains("same-process execution still active"));
        assert!(!interrupted.contains("crashed"));

        drop(claim);
        let interrupted = admit_durable_final_authorization::<TestResponse>(
            &interrupted_root,
            &authorization,
            &request,
        )
        .err()
        .expect("dropped uncompleted claim must remain interrupted");
        assert!(interrupted.contains("still active or crashed"));

        let failed_root = temp_dir("failed");
        let failed_claim = match admit_durable_final_authorization::<TestResponse>(
            &failed_root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        failed_claim
            .fail("scoring failed")
            .expect("publish failure");
        let failed = admit_durable_final_authorization::<TestResponse>(
            &failed_root,
            &authorization,
            &request,
        )
        .err()
        .expect("failed claim must not reopen");
        assert!(failed.contains("durably failed"));
        assert!(failed.contains("scoring failed"));
    }

    #[test]
    fn concurrent_admission_has_exactly_one_winner() {
        let root = Arc::new(temp_dir("concurrent"));
        let authorization = digest('3');
        let request = digest('4');
        let barrier = Arc::new(Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let root = Arc::clone(&root);
                let authorization = authorization.clone();
                let request = request.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    admit_durable_final_authorization::<TestResponse>(
                        &root,
                        &authorization,
                        &request,
                    )
                })
            })
            .collect::<Vec<_>>();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("thread"))
            .collect::<Vec<_>>();
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Ok(DurableFinalAdmission::Acquired(_))))
                .count(),
            1
        );
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let retryable = results
            .iter()
            .find_map(|result| result.as_ref().err())
            .expect("one concurrent admission must be retryable");
        assert!(retryable.contains("temporarily unavailable"), "{retryable}");
        assert!(
            retryable.contains("same-process execution still active"),
            "{retryable}"
        );
    }

    #[test]
    fn pre_final_admission_and_probe_share_the_canonical_live_registry() {
        let root = temp_dir("pre_final_live_registry");
        let alias = root.join(".");
        let universe = digest('d');
        let request = digest('e');
        let claim =
            match admit_durable_pre_final_winner_lock::<TestResponse>(&root, &universe, &request)
                .expect("pre-final claim")
            {
                DurableFinalAdmission::Acquired(claim) => claim,
                DurableFinalAdmission::Replay(_) => panic!("fresh claim cannot replay"),
            };

        let probe = probe_durable_pre_final_winner_lock::<TestResponse>(&alias, &universe)
            .expect_err("aliased probe must observe the live claim");
        assert_eq!(probe.kind(), DurableProbeUnavailableKind::LiveClaim);

        let admission =
            admit_durable_pre_final_winner_lock::<TestResponse>(&alias, &universe, &request)
                .err()
                .expect("aliased admission must observe the live claim");
        assert!(admission.contains("temporarily unavailable"), "{admission}");

        drop(claim);
        assert!(matches!(
            probe_durable_pre_final_winner_lock::<TestResponse>(&root, &universe)
                .expect("dropped pre-final claim probe"),
            DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Interrupted)
        ));
    }

    #[test]
    fn probe_reports_prepublication_registry_reservation_as_retryable_not_missing() {
        let root = temp_dir("prepublication_live_registry");
        let authorization = digest('7');
        let authorization_identity =
            canonical_live_authorization_identity(&root, FINAL_LEDGER_DIRECTORY, &authorization)
                .expect("authorization identity");
        let reserved = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let owner = {
            let reserved = Arc::clone(&reserved);
            let release = Arc::clone(&release);
            std::thread::spawn(move || {
                let live = LiveDurableAuthorizationRegistration::acquire(authorization_identity)
                    .expect("prepublication live reservation");
                reserved.wait();
                release.wait();
                drop(live);
            })
        };
        reserved.wait();
        assert!(
            !root.join(FINAL_LEDGER_DIRECTORY).exists(),
            "the canary must probe before any ledger directory is published"
        );

        let error =
            probe_durable_final_completion_for_authorization::<TestResponse>(&root, &authorization)
                .expect_err("reserved prepublication claim must remain retryable");
        assert_eq!(error.kind(), DurableProbeUnavailableKind::LiveClaim);

        release.wait();
        owner.join().expect("prepublication owner thread");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &root,
                &authorization,
            )
            .expect("unreserved empty authorization probe"),
            DurableCompletionProbe::Missing
        ));
    }

    #[test]
    fn tampered_completion_is_rejected() {
        let root = temp_dir("tamper");
        let authorization = digest('5');
        let request = digest('6');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        claim
            .complete(&TestResponse {
                artifact_sha256: digest('7'),
                dataset_count: 2,
            })
            .expect("complete");
        let completed = claim.authorization_dir.join(TERMINAL_FILE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&completed).expect("read completed"))
                .expect("completed JSON");
        value["response"]["dataset_count"] = serde_json::json!(999);
        fs::write(
            &completed,
            serde_json::to_vec_pretty(&value).expect("encode tamper"),
        )
        .expect("tamper completed");

        let error =
            admit_durable_final_authorization::<TestResponse>(&root, &authorization, &request)
                .err()
                .expect("tampered replay must fail");
        assert!(error.contains("integrity mismatch"));
    }

    #[test]
    fn pre_final_winner_lock_is_read_only_and_replays_the_sealed_response() {
        let root = temp_dir("pre_final_replay");
        let universe = digest('8');
        let request = digest('9');
        let response = TestResponse {
            artifact_sha256: digest('a'),
            dataset_count: 42,
        };
        let missing = load_durable_pre_final_winner_lock::<TestResponse>(&root, &universe)
            .expect_err("read-only lookup must not create a claim");
        assert!(
            missing.contains("failed to inspect")
                || missing.contains("no existing ledger directory")
        );
        assert!(
            try_load_durable_pre_final_winner_lock::<TestResponse>(&root, &universe)
                .expect("optional read-only lookup")
                .is_none()
        );

        let claim =
            match admit_durable_pre_final_winner_lock::<TestResponse>(&root, &universe, &request)
                .expect("pre-final claim")
            {
                DurableFinalAdmission::Acquired(claim) => claim,
                DurableFinalAdmission::Replay(_) => panic!("first admission cannot replay"),
            };
        claim.complete(&response).expect("complete winner lock");
        let replay = load_durable_pre_final_winner_lock::<TestResponse>(&root, &universe)
            .expect("load durable winner lock");
        assert_eq!(replay, response);
        assert_eq!(
            try_load_durable_pre_final_winner_lock::<TestResponse>(&root, &universe)
                .expect("optional durable winner-lock replay"),
            Some(response)
        );
    }

    #[test]
    fn final_completion_receipt_lookup_never_opens_a_missing_authorization() {
        let root = temp_dir("final_receipt_missing");
        let error =
            load_durable_final_completion_for_authorization::<TestResponse>(&root, &digest('b'))
                .expect_err("missing final receipt must fail closed");
        assert!(
            error.contains("failed to inspect") || error.contains("no existing ledger directory")
        );
        let final_root = root.join(FINAL_LEDGER_DIRECTORY);
        assert!(
            !final_root.exists(),
            "read-only receipt lookup must not create the ledger root"
        );
    }

    #[test]
    fn final_completion_can_be_loaded_by_authorization_and_derives_request_from_claim() {
        let root = temp_dir("final_receipt_by_authorization");
        let authorization = digest('d');
        let request = digest('e');
        let response = TestResponse {
            artifact_sha256: digest('f'),
            dataset_count: 11,
        };
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        claim.complete(&response).expect("complete");

        let (loaded_request, loaded_response) =
            load_durable_final_completion_for_authorization::<TestResponse>(&root, &authorization)
                .expect("load by authorization");
        assert_eq!(loaded_request, request);
        assert_eq!(loaded_response, response);
    }

    #[test]
    fn optional_final_completion_lookup_distinguishes_missing_completed_and_poisoned_claims() {
        let missing_root = temp_dir("optional_final_receipt_missing");
        assert!(
            try_load_durable_final_completion_for_authorization::<TestResponse>(
                &missing_root,
                &digest('1')
            )
            .expect("missing read-only lookup")
            .is_none()
        );
        assert!(!missing_root.join(FINAL_LEDGER_DIRECTORY).exists());

        let completed_root = temp_dir("optional_final_receipt_completed");
        let authorization = digest('2');
        let request = digest('3');
        let response = TestResponse {
            artifact_sha256: digest('4'),
            dataset_count: 3,
        };
        let completed_claim = match admit_durable_final_authorization::<TestResponse>(
            &completed_root,
            &authorization,
            &request,
        )
        .expect("completed claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected completed replay"),
        };
        completed_claim
            .complete(&response)
            .expect("publish completed terminal");
        assert_eq!(
            try_load_durable_final_completion_for_authorization::<TestResponse>(
                &completed_root,
                &authorization,
            )
            .expect("completed read-only lookup"),
            Some((request.clone(), response))
        );

        let interrupted_root = temp_dir("optional_final_receipt_interrupted");
        let _interrupted_claim = match admit_durable_final_authorization::<TestResponse>(
            &interrupted_root,
            &authorization,
            &request,
        )
        .expect("interrupted claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected interrupted replay"),
        };
        assert!(
            try_load_durable_final_completion_for_authorization::<TestResponse>(
                &interrupted_root,
                &authorization,
            )
            .expect_err("interrupted lookup must remain fail-closed")
            .contains("still active or crashed")
        );

        let failed_root = temp_dir("optional_final_receipt_failed");
        let failed_claim = match admit_durable_final_authorization::<TestResponse>(
            &failed_root,
            &authorization,
            &request,
        )
        .expect("failed claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected failed replay"),
        };
        failed_claim
            .fail("terminal failure")
            .expect("publish failure");
        assert!(
            try_load_durable_final_completion_for_authorization::<TestResponse>(
                &failed_root,
                &authorization,
            )
            .expect_err("failed lookup must remain fail-closed")
            .contains("durably failed")
        );
    }

    #[test]
    fn typed_probe_distinguishes_missing_completed_failed_interrupted_and_invalid_state() {
        let authorization = digest('a');
        let request = digest('b');

        let missing_root = temp_dir("typed_probe_missing");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &missing_root,
                &authorization,
            )
            .expect("missing probe"),
            DurableCompletionProbe::Missing
        ));

        let empty_root = temp_dir("typed_probe_empty_authorization");
        fs::create_dir_all(
            empty_root
                .join(FINAL_LEDGER_DIRECTORY)
                .join(sha256_hex(&authorization)),
        )
        .expect("create empty authorization directory");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &empty_root,
                &authorization,
            )
            .expect("empty authorization probe"),
            DurableCompletionProbe::Missing
        ));

        let interrupted_root = temp_dir("typed_probe_interrupted");
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &interrupted_root,
            &authorization,
            &request,
        )
        .expect("interrupted claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        let live_probe = probe_durable_final_completion_for_authorization::<TestResponse>(
            &interrupted_root,
            &authorization,
        )
        .expect_err("live probe must remain retryable");
        assert_eq!(live_probe.kind(), DurableProbeUnavailableKind::LiveClaim);
        assert!(!live_probe.to_string().contains("interrupted"));

        drop(claim);
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &interrupted_root,
                &authorization,
            )
            .expect("dropped claim probe"),
            DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Interrupted)
        ));

        let failed_root = temp_dir("typed_probe_failed");
        let failed_claim = match admit_durable_final_authorization::<TestResponse>(
            &failed_root,
            &authorization,
            &request,
        )
        .expect("failed claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        let secret = r#"failure at C:\Users\alice\secret\terminal.json"#;
        failed_claim.fail(secret).expect("publish failed terminal");
        let failed_probe = probe_durable_final_completion_for_authorization::<TestResponse>(
            &failed_root,
            &authorization,
        )
        .expect("failed probe");
        assert!(matches!(
            &failed_probe,
            DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Failed)
        ));
        assert!(!format!("{failed_probe:?}").contains(secret));

        let completed_root = temp_dir("typed_probe_completed");
        let response = TestResponse {
            artifact_sha256: digest('c'),
            dataset_count: 9,
        };
        let completed_claim = match admit_durable_final_authorization::<TestResponse>(
            &completed_root,
            &authorization,
            &request,
        )
        .expect("completed claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        completed_claim
            .complete(&response)
            .expect("complete terminal");
        assert_eq!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &completed_root,
                &authorization,
            )
            .expect("completed probe"),
            DurableCompletionProbe::Completed {
                request_sha256: request.clone(),
                response: response.clone(),
            }
        );

        let invalid_root = temp_dir("typed_probe_terminal_without_claim");
        let invalid_dir = invalid_root
            .join(FINAL_LEDGER_DIRECTORY)
            .join(sha256_hex(&authorization));
        fs::create_dir_all(&invalid_dir).expect("create invalid authorization directory");
        fs::write(invalid_dir.join(TERMINAL_FILE), b"{}").expect("write orphan terminal");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &invalid_root,
                &authorization,
            )
            .expect("invalid probe"),
            DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Invalid)
        ));
    }

    #[test]
    fn typed_probe_rejects_malformed_completed_terminal_as_invalid() {
        let root = temp_dir("typed_probe_malformed_completed");
        let authorization = digest('d');
        let request = digest('e');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        claim
            .complete(&TestResponse {
                artifact_sha256: digest('f'),
                dataset_count: 2,
            })
            .expect("complete");
        let terminal = claim.authorization_dir.join(TERMINAL_FILE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&terminal).expect("read terminal"))
                .expect("terminal JSON");
        value["response"]["dataset_count"] = serde_json::json!(999);
        fs::write(
            terminal,
            serde_json::to_vec_pretty(&value).expect("encode tamper"),
        )
        .expect("tamper terminal");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &root,
                &authorization,
            )
            .expect("tampered probe"),
            DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Invalid)
        ));
    }

    #[test]
    fn transient_probe_error_classification_and_messages_are_retryable_and_path_safe() {
        let permission_detail = r#"C:\Users\alice\secret\terminal.json"#;
        let permission = io::Error::new(ErrorKind::PermissionDenied, permission_detail);
        let DurableProbeReadError::Unavailable(permission) = durable_probe_unavailable(&permission)
        else {
            panic!("permission errors must remain transient")
        };
        assert_eq!(
            permission.kind(),
            DurableProbeUnavailableKind::PermissionDenied
        );
        let public = permission.to_string();
        assert!(public.contains("temporarily unavailable"));
        assert!(public.contains("retry"));
        assert!(!public.contains(permission_detail));

        let generic_detail = "/private/research/terminal.json";
        let generic = io::Error::new(ErrorKind::Other, generic_detail);
        let DurableProbeReadError::Unavailable(generic) = durable_probe_unavailable(&generic)
        else {
            panic!("generic I/O errors must remain transient")
        };
        assert_eq!(generic.kind(), DurableProbeUnavailableKind::Io);
        assert!(!generic.to_string().contains(generic_detail));

        #[cfg(windows)]
        {
            let sharing = io::Error::from_raw_os_error(32);
            let DurableProbeReadError::Unavailable(sharing) = durable_probe_unavailable(&sharing)
            else {
                panic!("sharing violations must remain transient")
            };
            assert_eq!(
                sharing.kind(),
                DurableProbeUnavailableKind::SharingViolation
            );
        }
    }

    #[test]
    fn oversize_completion_publishes_a_failed_terminal() {
        let root = temp_dir("oversize_completion");
        let authorization = digest('0');
        let request = digest('1');
        let claim =
            match admit_durable_final_authorization::<String>(&root, &authorization, &request)
                .expect("claim")
            {
                DurableFinalAdmission::Acquired(claim) => claim,
                DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
            };

        let error = claim
            .complete_with_max_bytes(&"x".repeat(512), 128)
            .expect_err("oversize completion must fail");
        assert!(error.contains("exceeding the 128 byte publish limit"));
        claim
            .fail(&error)
            .expect("outer failure recorder must accept the existing valid failed terminal");
        let terminal = claim.authorization_dir.join(TERMINAL_FILE);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&terminal).expect("read terminal"))
                .expect("terminal JSON");
        assert_eq!(value["state"], "failed");
        assert!(!claim.authorization_dir.join("completed.json").exists());
        assert!(!claim.authorization_dir.join("failed.json").exists());

        let replay_error =
            admit_durable_final_authorization::<String>(&root, &authorization, &request)
                .err()
                .expect("oversize claim must remain durably failed");
        assert!(replay_error.contains("durably failed"));
        assert!(replay_error.contains("exceeding the 128 byte publish limit"));
    }

    #[test]
    fn concurrent_complete_and_fail_publish_exactly_one_terminal() {
        let root = temp_dir("terminal_race");
        let authorization = digest('2');
        let request = digest('3');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => Arc::new(claim),
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        let barrier = Arc::new(Barrier::new(2));
        let response = TestResponse {
            artifact_sha256: digest('4'),
            dataset_count: 19,
        };

        let complete_handle = {
            let claim = Arc::clone(&claim);
            let barrier = Arc::clone(&barrier);
            let response = response.clone();
            std::thread::spawn(move || {
                barrier.wait();
                claim.complete(&response)
            })
        };
        let fail_handle = {
            let claim = Arc::clone(&claim);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                claim.fail("simulated execution failure")
            })
        };
        let results = [
            complete_handle.join().expect("complete thread"),
            fail_handle.join().expect("fail thread"),
        ];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);

        let entries = fs::read_dir(&claim.authorization_dir)
            .expect("read authorization directory")
            .map(|entry| entry.expect("directory entry").file_name())
            .collect::<Vec<_>>();
        assert!(entries.iter().any(|name| name == CLAIM_FILE));
        assert!(entries.iter().any(|name| name == TERMINAL_FILE));
        assert_eq!(
            entries.iter().filter(|name| *name == TERMINAL_FILE).count(),
            1
        );

        let value: serde_json::Value = serde_json::from_slice(
            &fs::read(claim.authorization_dir.join(TERMINAL_FILE)).expect("read terminal"),
        )
        .expect("terminal JSON");
        assert!(value["state"] == "completed" || value["state"] == "failed");
    }

    #[test]
    fn large_completion_probe_opens_claim_and_terminal_exactly_once() {
        let root = temp_dir("large_completion_single_open");
        let authorization = digest('5');
        let request = digest('6');
        let response = "x".repeat(MAX_FAILED_RECORD_BYTES as usize + 1024);
        let claim =
            match admit_durable_final_authorization::<String>(&root, &authorization, &request)
                .expect("claim")
            {
                DurableFinalAdmission::Acquired(claim) => claim,
                DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
            };
        claim.complete(&response).expect("publish large completion");
        assert!(
            fs::metadata(claim.authorization_dir.join(TERMINAL_FILE))
                .expect("large terminal metadata")
                .len()
                > MAX_FAILED_RECORD_BYTES
        );

        reset_plain_read_open_count();
        assert_eq!(
            probe_durable_final_completion_for_authorization::<String>(&root, &authorization,)
                .expect("probe large completion"),
            DurableCompletionProbe::Completed {
                request_sha256: request.clone(),
                response: response.clone(),
            }
        );
        assert_eq!(
            plain_read_open_count(),
            2,
            "one claim handle and one terminal handle must serve the complete probe"
        );

        reset_plain_read_open_count();
        let replay =
            match admit_durable_final_authorization::<String>(&root, &authorization, &request)
                .expect("replay large completion")
            {
                DurableFinalAdmission::Replay(response) => response,
                DurableFinalAdmission::Acquired(_) => panic!("completed authorization must replay"),
            };
        assert_eq!(replay, response);
        assert_eq!(
            plain_read_open_count(),
            2,
            "one claim handle and one terminal handle must serve the complete replay"
        );
    }

    #[test]
    fn terminal_discriminator_and_body_are_read_from_the_same_held_handle() {
        let root = temp_dir("terminal_held_handle_identity");
        let path = root.join(TERMINAL_FILE);
        let authorization = digest('7');
        let request = digest('8');
        let response = "original".repeat(MAX_FAILED_RECORD_BYTES as usize / 4);
        let terminal_sha256 =
            completed_hash(&authorization, &request, &response).expect("completion hash");
        let record = DurableTerminalRecordV2::Completed {
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256: authorization,
            request_sha256: request,
            response,
            terminal_sha256,
        };
        let original = serde_json::to_vec_pretty(&record).expect("encode original terminal");
        assert!(original.len() as u64 > MAX_FAILED_RECORD_BYTES);
        fs::write(&path, &original).expect("write original terminal");

        let file = open_replaceable_test_file(&path);
        let observed_len = file.metadata().expect("held metadata").len();
        let moved = root.join("terminal-original.json");
        fs::rename(&path, &moved).expect("move pathname after opening original");
        fs::write(&path, b"replacement-path-bytes").expect("publish replacement pathname");

        let observed = read_terminal_bytes_from_open_file(file, observed_len)
            .expect("read terminal through held handle");
        assert_eq!(observed, original);
        assert_eq!(
            fs::read(&path).expect("read replacement pathname"),
            b"replacement-path-bytes"
        );
    }

    #[test]
    fn bounded_handle_reads_reject_growth_and_terminal_length_overflow() {
        let root = temp_dir("held_handle_overflow");
        let path = root.join("bounded.json");
        fs::write(&path, b"1234").expect("write bounded fixture");
        let file = open_replaceable_test_file(&path);
        let observed_len = file.metadata().expect("bounded metadata").len();
        fs::write(&path, b"12345678").expect("grow bounded fixture after open");
        assert!(matches!(
            read_bounded_from_open_file(file, observed_len, 4),
            Err(BoundedHandleReadError::Invalid)
        ));

        let terminal_file = open_replaceable_test_file(&path);
        assert!(matches!(
            read_terminal_bytes_from_open_file(
                terminal_file,
                MAX_COMPLETED_RECORD_BYTES.saturating_add(1),
            ),
            Err(TerminalHandleReadError::Invalid)
        ));
    }

    #[test]
    fn structurally_hashed_oversized_failed_terminal_is_invalid() {
        let root = temp_dir("oversized_hashed_failed_terminal");
        let authorization = digest('a');
        let request = digest('b');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        let error = "x".repeat(MAX_FAILED_RECORD_BYTES as usize);
        let terminal_sha256 =
            failed_hash(&authorization, &request, &error).expect("hash oversized failed record");
        let record: DurableTerminalRecordV2<TestResponse> = DurableTerminalRecordV2::Failed {
            schema_version: LEDGER_SCHEMA_VERSION,
            authorization_sha256: authorization.clone(),
            request_sha256: request,
            error,
            terminal_sha256,
        };
        let bytes = serde_json::to_vec_pretty(&record).expect("encode oversized failed record");
        assert!(bytes.len() as u64 > MAX_FAILED_RECORD_BYTES);
        fs::write(claim.authorization_dir.join(TERMINAL_FILE), bytes)
            .expect("write oversized failed record");
        assert!(matches!(
            probe_durable_final_completion_for_authorization::<TestResponse>(
                &root,
                &authorization,
            )
            .expect("probe oversized failed record"),
            DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Invalid)
        ));
    }

    #[test]
    fn oversized_failed_terminal_is_rejected_before_the_completion_read_budget() {
        let root = temp_dir("oversized_failed_terminal");
        let authorization = digest('8');
        let request = digest('9');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        let malformed = format!(
            "{{\n  \"state\": \"failed\",\n  \"padding\": \"{}\"\n}}",
            "x".repeat(MAX_FAILED_RECORD_BYTES as usize)
        );
        fs::write(claim.authorization_dir.join(TERMINAL_FILE), malformed)
            .expect("write malformed oversized failed terminal");
        // The manual fixture write stands in for a prior process. Release the
        // same-process live lease so admission reaches the malformed durable
        // terminal instead of correctly reporting the still-live claimant.
        drop(claim);

        let error =
            admit_durable_final_authorization::<TestResponse>(&root, &authorization, &request)
                .err()
                .expect("oversized failed terminal must be rejected");
        assert!(error.contains("failed-terminal limit"));
        assert!(error.contains("canonical completed discriminator"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_final_record_reparse_points_are_not_followed() {
        use std::os::windows::fs::symlink_file;

        for target_name in [CLAIM_FILE, TERMINAL_FILE] {
            let root = temp_dir(&format!("windows_record_reparse_{target_name}"));
            let authorization = digest('c');
            let request = digest('d');
            let claim = match admit_durable_final_authorization::<TestResponse>(
                &root,
                &authorization,
                &request,
            )
            .expect("claim")
            {
                DurableFinalAdmission::Acquired(claim) => claim,
                DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
            };
            claim
                .complete(&TestResponse {
                    artifact_sha256: digest('e'),
                    dataset_count: 17,
                })
                .expect("complete fixture");
            let path = claim.authorization_dir.join(target_name);
            let outside = root.join(format!("outside-{target_name}"));
            fs::rename(&path, &outside).expect("move record outside authorization directory");
            let outside_before = fs::read(&outside).expect("read outside record before probe");
            if let Err(error) = symlink_file(&outside, &path) {
                eprintln!(
                    "skipping final-record reparse test because symlink creation is unavailable: {error}"
                );
                return;
            }

            if let Ok(handle) = open_plain_read_handle(&path) {
                assert!(metadata_is_reparse_point(
                    &handle.metadata().expect("reparse handle metadata")
                ));
            }
            assert!(matches!(
                probe_durable_final_completion_for_authorization::<TestResponse>(
                    &root,
                    &authorization,
                )
                .expect("probe reparse record"),
                DurableCompletionProbe::Blocked(DurableProbeBlockedReason::Invalid)
            ));
            assert_eq!(
                fs::read(&outside).expect("read outside record after probe"),
                outside_before,
                "the outside reparse target must not be read through or modified"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_terminal_sharing_violation_remains_retryable() {
        use std::os::windows::fs::OpenOptionsExt;

        let root = temp_dir("windows_terminal_sharing_violation");
        let authorization = digest('f');
        let request = digest('0');
        let claim = match admit_durable_final_authorization::<TestResponse>(
            &root,
            &authorization,
            &request,
        )
        .expect("claim")
        {
            DurableFinalAdmission::Acquired(claim) => claim,
            DurableFinalAdmission::Replay(_) => panic!("unexpected replay"),
        };
        claim
            .complete(&TestResponse {
                artifact_sha256: digest('1'),
                dataset_count: 23,
            })
            .expect("complete fixture");
        let terminal = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(claim.authorization_dir.join(TERMINAL_FILE))
            .expect("hold terminal without sharing");
        let error =
            probe_durable_final_completion_for_authorization::<TestResponse>(&root, &authorization)
                .expect_err("sharing violation must remain retryable");
        assert_eq!(error.kind(), DurableProbeUnavailableKind::SharingViolation);
        let root_display = root.to_string_lossy();
        assert!(!error.to_string().contains(root_display.as_ref() as &str));
        drop(terminal);
    }

    #[cfg(windows)]
    #[test]
    fn windows_reparse_ancestor_is_rejected() {
        use std::os::windows::fs::symlink_dir;

        let root = temp_dir("windows_reparse");
        let real = root.join("real");
        fs::create_dir(&real).expect("create real directory");
        let alias = root.join("alias");
        if let Err(error) = symlink_dir(&real, &alias) {
            eprintln!("skipping reparse test because symlink creation is unavailable: {error}");
            return;
        }
        let error =
            admit_durable_final_authorization::<TestResponse>(&alias, &digest('6'), &digest('7'))
                .err()
                .expect("reparse application-data path must be rejected");
        assert!(error.contains("non-reparse"));
    }
}
