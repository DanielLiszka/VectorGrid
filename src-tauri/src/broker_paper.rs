use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

const PAPER_BROKER_VERSION: u32 = 1;
const DEFAULT_STARTING_CASH: f64 = 100_000.0;
static PAPER_BROKER_STATE_LOCK: Mutex<()> = Mutex::new(());
static PAPER_BROKER_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaperOrderSide {
    Buy,
    Sell,
}

impl PaperOrderSide {
    pub fn as_str(self) -> &'static str {
        match self {
            PaperOrderSide::Buy => "buy",
            PaperOrderSide::Sell => "sell",
        }
    }

    fn signed_qty(self, qty: f64) -> f64 {
        match self {
            PaperOrderSide::Buy => qty,
            PaperOrderSide::Sell => -qty,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaperBracketLeg {
    Parent,
    StopLoss,
    TakeProfit,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaperBracketTieBreak {
    StopLossFirst,
}

impl PaperBracketTieBreak {
    pub fn as_str(self) -> &'static str {
        match self {
            PaperBracketTieBreak::StopLossFirst => "stop_loss_first",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerAccount {
    pub account_id: String,
    pub currency: String,
    pub starting_cash: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub realized_pnl: f64,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerPosition {
    pub symbol: String,
    pub qty: f64,
    pub avg_entry_price: f64,
    pub market_price: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub last_fill_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerOrder {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bracket_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bracket_leg: Option<PaperBracketLeg>,
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    pub filled_qty: f64,
    pub avg_fill_price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tie_break: Option<PaperBracketTieBreak>,
    #[serde(default)]
    pub fee_bps: f64,
    #[serde(default)]
    pub spread_bps: f64,
    #[serde(default)]
    pub slippage_bps: f64,
    pub submitted_at_unix_ms: i64,
    pub filled_at_unix_ms: i64,
    #[serde(default)]
    pub filled_recorded_at_unix_ms: i64,
    #[serde(default)]
    pub updated_at_unix_ms: i64,
    #[serde(default)]
    pub recorded_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled_at_unix_ms: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerFill {
    pub id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    #[serde(default)]
    pub spread: f64,
    pub slippage: f64,
    #[serde(default)]
    pub realized_pnl_delta: f64,
    pub at_unix_ms: i64,
    #[serde(default)]
    pub recorded_at_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaperBrokerBracketCheckpoint {
    pub last_evaluated_bar_unix_ms: i64,
    pub last_processed_at_unix_ms: i64,
    /// Exact OHLC fingerprint for the durable anchor bar. An empty value is
    /// accepted only for legacy state that has no open protective orders.
    #[serde(default)]
    pub last_evaluated_bar_fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerStateDoc {
    pub version: u32,
    pub account: PaperBrokerAccount,
    #[serde(default)]
    pub positions: BTreeMap<String, PaperBrokerPosition>,
    #[serde(default)]
    pub orders: Vec<PaperBrokerOrder>,
    #[serde(default)]
    pub fills: Vec<PaperBrokerFill>,
    #[serde(default)]
    pub bracket_checkpoints: BTreeMap<String, PaperBrokerBracketCheckpoint>,
    pub next_order_seq: u64,
    pub next_fill_seq: u64,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerMarketOrder {
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub price: f64,
    pub at_unix_ms: i64,
    pub fee_bps: f64,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub client_order_id: Option<String>,
    pub bracket: Option<PaperBrokerBracketSpec>,
    pub daily_risk_limits: Option<PaperBrokerDailyRiskLimits>,
}

#[derive(Clone, Copy, Debug)]
pub struct PaperBrokerDailyRiskLimits {
    pub max_daily_loss: Option<f64>,
    pub max_daily_orders: Option<u32>,
    pub at_unix_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaperBrokerDailyRiskSnapshot {
    pub day_bucket_utc: i64,
    pub order_count: u32,
    pub realized_pnl: f64,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketSpec {
    pub stop_price: f64,
    pub take_profit_limit_price: f64,
    pub time_in_force: String,
    pub tie_break: PaperBracketTieBreak,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerExecution {
    pub order: PaperBrokerOrder,
    pub fill: PaperBrokerFill,
    pub child_orders: Vec<PaperBrokerOrder>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaperBrokerAtomicTargetStatus {
    Executed,
    AlreadyAtTarget,
    PositionMismatch,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerAtomicTargetOrder {
    pub symbol: String,
    pub expected_position_qty: f64,
    pub target_position_qty: f64,
    pub price: f64,
    pub event_at_unix_ms: i64,
    pub processing_at_unix_ms: i64,
    pub fee_bps: f64,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub client_order_id: Option<String>,
    pub bracket: Option<PaperBrokerBracketSpec>,
    pub daily_risk_limits: Option<PaperBrokerDailyRiskLimits>,
    pub cancel_owned_brackets_for_risk_reduction: bool,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerAtomicTargetResult {
    pub status: PaperBrokerAtomicTargetStatus,
    pub expected_position_qty: f64,
    pub observed_position_qty: f64,
    pub target_position_qty: f64,
    pub execution: Option<PaperBrokerExecution>,
    pub cancelled_orders: Vec<PaperBrokerOrder>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketAdvanceBar {
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub at_unix_ms: i64,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketAdvanceResult {
    pub open_order_count_before: usize,
    pub filled_orders: Vec<PaperBrokerOrder>,
    pub cancelled_orders: Vec<PaperBrokerOrder>,
    pub tie_break_events: Vec<String>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketReconcileRequest {
    pub symbol: String,
    /// Complete, strictly ordered bars beginning with the persisted checkpoint
    /// bar. The checkpoint bar is an anchor and is not evaluated twice.
    pub bars: Vec<PaperBrokerBracketAdvanceBar>,
    pub processing_at_unix_ms: i64,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketReconcileResult {
    pub supplied_bar_count: usize,
    pub processed_bar_count: usize,
    pub previous_checkpoint_unix_ms: Option<i64>,
    pub last_evaluated_bar_unix_ms: i64,
    pub open_order_count_before: usize,
    pub filled_orders: Vec<PaperBrokerOrder>,
    pub cancelled_orders: Vec<PaperBrokerOrder>,
    pub tie_break_events: Vec<String>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerCancelOpenOrdersResult {
    pub open_order_count_before: usize,
    pub cancelled_orders: Vec<PaperBrokerOrder>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

pub fn paper_broker_state_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir
        .join("execution")
        .join("paper_broker")
        .join("state.json")
}

fn paper_broker_state_backup_path(path: &Path) -> PathBuf {
    let backup_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|name| format!("{name}.bak"))
        .unwrap_or_else(|| "state.json.bak".to_string());
    path.with_file_name(backup_name)
}

struct PaperBrokerStateGuard {
    _process_guard: MutexGuard<'static, ()>,
    #[cfg(windows)]
    _cross_process_guard: PaperBrokerCrossProcessGuard,
}

#[cfg(windows)]
struct PaperBrokerCrossProcessGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl PaperBrokerCrossProcessGuard {
    fn acquire() -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{
            CloseHandle, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
        };
        use windows_sys::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};

        const LOCK_TIMEOUT_MS: u32 = 30_000;
        let name = std::ffi::OsStr::new("Local\\VectorGridPaperBrokerStateV1")
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "failed to create cross-process paper broker mutex: {}",
                std::io::Error::last_os_error()
            ));
        }
        match unsafe { WaitForSingleObject(handle, LOCK_TIMEOUT_MS) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(Self(handle)),
            WAIT_TIMEOUT => {
                unsafe {
                    CloseHandle(handle);
                }
                Err("timed out waiting for the cross-process paper broker state lock".to_string())
            }
            WAIT_FAILED => {
                let error = std::io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                Err(format!(
                    "failed while waiting for the cross-process paper broker state lock: {error}"
                ))
            }
            other => {
                unsafe {
                    CloseHandle(handle);
                }
                Err(format!(
                    "unexpected cross-process paper broker mutex wait result {other}"
                ))
            }
        }
    }
}

#[cfg(windows)]
impl Drop for PaperBrokerCrossProcessGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::ReleaseMutex;

        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

fn paper_broker_state_lock() -> Result<PaperBrokerStateGuard, String> {
    let process_guard = PAPER_BROKER_STATE_LOCK
        .lock()
        .map_err(|_| "paper broker state lock is poisoned".to_string())?;
    #[cfg(windows)]
    let cross_process_guard = PaperBrokerCrossProcessGuard::acquire()?;
    Ok(PaperBrokerStateGuard {
        _process_guard: process_guard,
        #[cfg(windows)]
        _cross_process_guard: cross_process_guard,
    })
}

fn paper_broker_temp_path(path: &Path) -> Result<PathBuf, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("paper broker state path has no parent: {}", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("state.json");
    let seq = PAPER_BROKER_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(parent.join(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        nonce,
        seq
    )))
}

#[cfg(windows)]
fn replace_existing_file_with_temp(path: &Path, temp_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    fn wide_null(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let replaced = wide_null(path);
    let replacement = wide_null(temp_path);
    let ok = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_existing_file_with_temp(path: &Path, temp_path: &Path) -> Result<(), String> {
    fs::rename(temp_path, path).map_err(|e| e.to_string())
}

fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create paper broker state directory: {e}"))?;
    }
    let temp_path = paper_broker_temp_path(path)?;
    let write_temp_result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    })();
    if let Err(error) = write_temp_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    let replace_result = if path.exists() {
        replace_existing_file_with_temp(path, &temp_path)
    } else {
        fs::rename(&temp_path, path).map_err(|e| e.to_string())
    };
    if let Err(error) = replace_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    Ok(())
}

fn encode_state_doc(doc: &PaperBrokerStateDoc) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(doc).map_err(|e| format!("failed to encode paper broker state: {e}"))
}

fn decode_state_doc(path: &Path, bytes: &[u8]) -> Result<PaperBrokerStateDoc, String> {
    let doc: PaperBrokerStateDoc = serde_json::from_slice(bytes).map_err(|e| {
        format!(
            "failed to decode paper broker state {}: {e}",
            path.to_string_lossy()
        )
    })?;
    if doc.version != PAPER_BROKER_VERSION {
        return Err(format!(
            "unsupported paper broker state version {} in {}",
            doc.version,
            path.to_string_lossy()
        ));
    }
    validate_state_doc(&doc).map_err(|error| {
        format!(
            "invalid paper broker state {}: {error}",
            path.to_string_lossy()
        )
    })?;
    Ok(doc)
}

fn read_state_doc(path: &Path, now_unix_ms: i64) -> Result<PaperBrokerStateDoc, String> {
    let bytes = fs::read(path).map_err(|e| {
        format!(
            "failed to read paper broker state {}: {e}",
            path.to_string_lossy()
        )
    })?;
    let mut doc = decode_state_doc(path, &bytes)?;
    doc.recompute_account(now_unix_ms.max(doc.account.updated_at_unix_ms));
    Ok(doc)
}

fn load_or_default_unlocked(
    app_local_data_dir: &Path,
    now_unix_ms: i64,
) -> Result<PaperBrokerStateDoc, String> {
    let path = paper_broker_state_path(app_local_data_dir);
    let primary_exists = path.try_exists().map_err(|error| {
        format!(
            "failed to inspect paper broker state {}: {error}",
            path.to_string_lossy()
        )
    })?;
    if !primary_exists {
        let backup_path = paper_broker_state_backup_path(&path);
        let backup_exists = backup_path.try_exists().map_err(|error| {
            format!(
                "failed to inspect paper broker backup {}: {error}",
                backup_path.to_string_lossy()
            )
        })?;
        if !backup_exists {
            return Ok(PaperBrokerStateDoc::new(now_unix_ms));
        }
        let doc = read_state_doc(&backup_path, now_unix_ms).map_err(|error| {
            format!(
                "paper broker primary state is missing and backup recovery failed: {error}. \
                 State was not reset or modified."
            )
        })?;
        let bytes = encode_state_doc(&doc)?;
        write_bytes_atomically(&path, &bytes).map_err(|restore_error| {
            format!(
                "recovered missing paper broker state from backup, but failed to restore {}: \
                 {restore_error}",
                path.to_string_lossy()
            )
        })?;
        return Ok(doc);
    }

    match read_state_doc(&path, now_unix_ms) {
        Ok(doc) => Ok(doc),
        Err(primary_error) => {
            let backup_path = paper_broker_state_backup_path(&path);
            let backup_exists = backup_path.try_exists().map_err(|error| {
                format!(
                    "{primary_error}; failed to inspect paper broker backup {}: {error}",
                    backup_path.to_string_lossy()
                )
            })?;
            if !backup_exists {
                return Err(primary_error);
            }
            let doc = read_state_doc(&backup_path, now_unix_ms).map_err(|backup_error| {
                format!(
                    "{primary_error}; backup recovery failed: {backup_error}. Paper broker state \
                     was not reset or modified. Preserve both files for review and use the \
                     explicit paper-broker reset only after reconciling any prior simulated \
                     orders and positions."
                )
            })?;
            let bytes = encode_state_doc(&doc)?;
            write_bytes_atomically(&path, &bytes).map_err(|restore_error| {
                format!(
                    "recovered paper broker state from backup, but failed to restore {}: {restore_error}",
                    path.to_string_lossy()
                )
            })?;
            Ok(doc)
        }
    }
}

fn save_unlocked(app_local_data_dir: &Path, doc: &PaperBrokerStateDoc) -> Result<(), String> {
    validate_state_doc(doc)
        .map_err(|error| format!("refusing to persist invalid paper broker state: {error}"))?;
    let path = paper_broker_state_path(app_local_data_dir);
    let path_exists = path.try_exists().map_err(|error| {
        format!(
            "failed to inspect existing paper broker state {}: {error}",
            path.to_string_lossy()
        )
    })?;
    let backup_path = paper_broker_state_backup_path(&path);
    if path_exists {
        let current = fs::read(&path)
            .map_err(|e| format!("failed to read existing paper broker state for backup: {e}"))?;
        write_bytes_atomically(&backup_path, &current).map_err(|e| {
            format!(
                "failed to update paper broker state backup {}: {e}",
                backup_path.to_string_lossy()
            )
        })?;
    }
    let bytes = encode_state_doc(doc)?;
    write_bytes_atomically(&path, &bytes)
        .map_err(|e| format!("failed to write paper broker state: {e}"))?;
    // Keep the steady-state recovery copy byte-for-byte current. The previous
    // primary remains in the backup during replacement, then is refreshed only
    // after the new primary commit succeeds. A refresh failure must not report
    // the already-committed order as failed (which could provoke a duplicate
    // retry), but it is emitted prominently for operator diagnosis.
    if let Err(error) = write_bytes_atomically(&backup_path, &bytes) {
        log::error!(
            "paper broker primary state committed, but current backup refresh failed at {}: {}",
            backup_path.to_string_lossy(),
            error
        );
    }
    Ok(())
}

pub fn load_or_default(
    app_local_data_dir: &Path,
    now_unix_ms: i64,
) -> Result<PaperBrokerStateDoc, String> {
    let _guard = paper_broker_state_lock()?;
    load_or_default_unlocked(app_local_data_dir, now_unix_ms)
}

pub fn save(app_local_data_dir: &Path, doc: &PaperBrokerStateDoc) -> Result<(), String> {
    let _guard = paper_broker_state_lock()?;
    save_unlocked(app_local_data_dir, doc)
}

pub fn reset(app_local_data_dir: &Path, now_unix_ms: i64) -> Result<PaperBrokerStateDoc, String> {
    let _guard = paper_broker_state_lock()?;
    let doc = PaperBrokerStateDoc::new(now_unix_ms);
    save_unlocked(app_local_data_dir, &doc)?;
    Ok(doc)
}

const MILLIS_PER_UTC_DAY: i64 = 86_400_000;

fn utc_day_bucket(at_unix_ms: i64) -> i64 {
    at_unix_ms.div_euclid(MILLIS_PER_UTC_DAY)
}

fn durable_recorded_at(recorded_at_unix_ms: i64, fallback_unix_ms: i64) -> i64 {
    if recorded_at_unix_ms > 0 {
        recorded_at_unix_ms
    } else {
        fallback_unix_ms
    }
}

fn daily_risk_snapshot_from_doc(
    doc: &PaperBrokerStateDoc,
    at_unix_ms: i64,
) -> PaperBrokerDailyRiskSnapshot {
    let day_bucket_utc = utc_day_bucket(at_unix_ms);
    let order_count = doc
        .orders
        .iter()
        .filter(|order| {
            order.parent_order_id.is_none()
                && utc_day_bucket(durable_recorded_at(
                    order.recorded_at_unix_ms,
                    order.submitted_at_unix_ms,
                )) == day_bucket_utc
        })
        .count()
        .min(u32::MAX as usize) as u32;

    // Replay the durable fill ledger so legacy version-1 fills remain usable even
    // though they predate the explicit realized_pnl_delta audit field.
    let mut replay_positions = BTreeMap::new();
    let mut realized_pnl = 0.0;
    for fill in &doc.fills {
        let realized = apply_position_fill(
            &mut replay_positions,
            &fill.symbol,
            fill.side.signed_qty(fill.qty),
            fill.price,
            fill.at_unix_ms,
        ) - fill.fee;
        if utc_day_bucket(durable_recorded_at(
            fill.recorded_at_unix_ms,
            fill.at_unix_ms,
        )) == day_bucket_utc
        {
            realized_pnl += realized;
        }
    }

    PaperBrokerDailyRiskSnapshot {
        day_bucket_utc,
        order_count,
        realized_pnl,
    }
}

pub fn daily_risk_snapshot(
    app_local_data_dir: &Path,
    at_unix_ms: i64,
) -> Result<PaperBrokerDailyRiskSnapshot, String> {
    validate_processing_time(at_unix_ms)?;
    let _guard = paper_broker_state_lock()?;
    let doc = load_or_default_unlocked(app_local_data_dir, at_unix_ms)?;
    Ok(daily_risk_snapshot_from_doc(&doc, at_unix_ms))
}

fn enforce_daily_risk_limits(
    doc: &PaperBrokerStateDoc,
    limits: PaperBrokerDailyRiskLimits,
) -> Result<(), String> {
    let snapshot = daily_risk_snapshot_from_doc(doc, limits.at_unix_ms);
    if let Some(max_daily_orders) = limits.max_daily_orders.filter(|value| *value > 0) {
        if snapshot.order_count >= max_daily_orders {
            return Err(format!(
                "local paper daily order cap reached: {} order(s) today, cap={max_daily_orders}",
                snapshot.order_count
            ));
        }
    }
    if let Some(max_daily_loss) = limits
        .max_daily_loss
        .filter(|value| value.is_finite() && *value > 0.0)
    {
        if snapshot.realized_pnl <= -max_daily_loss {
            return Err(format!(
                "local paper daily realized loss cap reached: pnl={:.2}, cap=-{max_daily_loss:.2}",
                snapshot.realized_pnl
            ));
        }
    }
    Ok(())
}

fn adverse_fill_price(
    reference_price: f64,
    side: PaperOrderSide,
    spread_bps: f64,
    slippage_bps: f64,
) -> (f64, f64, f64) {
    let spread = reference_price * (spread_bps.max(0.0) * 0.5 / 10_000.0);
    let slippage = reference_price * (slippage_bps.max(0.0) / 10_000.0);
    let total_impact = spread + slippage;
    let fill_price = match side {
        PaperOrderSide::Buy => reference_price + total_impact,
        PaperOrderSide::Sell => (reference_price - total_impact).max(0.01),
    };
    (fill_price, spread, slippage)
}

pub fn submit_market_order(
    app_local_data_dir: &Path,
    mut order: PaperBrokerMarketOrder,
) -> Result<PaperBrokerExecution, String> {
    order.symbol = canonical_symbol(&order.symbol);
    validate_market_order(&order)?;
    if let Some(bracket) = order.bracket.as_ref() {
        validate_bracket_spec(&order, bracket)?;
    }
    let has_explicit_processing_time = order.daily_risk_limits.is_some();
    let requested_processing_at_unix_ms = order
        .daily_risk_limits
        .map(|limits| limits.at_unix_ms)
        .unwrap_or(order.at_unix_ms);
    validate_processing_time(requested_processing_at_unix_ms)?;
    if order.at_unix_ms > requested_processing_at_unix_ms {
        return Err("paper broker event time cannot be after processing time".to_string());
    }
    let _guard = paper_broker_state_lock()?;
    let mut doc = load_or_default_unlocked(app_local_data_dir, requested_processing_at_unix_ms)?;
    let processing_at_unix_ms = if has_explicit_processing_time {
        validate_processing_time_against_state(&doc, requested_processing_at_unix_ms)?;
        requested_processing_at_unix_ms
    } else {
        // Legacy/direct callers have no independent processing timestamp. Keep
        // their event time for market semantics, but serialize the durable
        // recorded time monotonically in lock-acquisition order.
        requested_processing_at_unix_ms.max(doc.account.updated_at_unix_ms)
    };
    let execution = submit_market_order_unlocked(&mut doc, order, processing_at_unix_ms, true)?;
    save_unlocked(app_local_data_dir, &doc)?;
    Ok(execution)
}

fn submit_market_order_unlocked(
    doc: &mut PaperBrokerStateDoc,
    mut order: PaperBrokerMarketOrder,
    processing_at_unix_ms: i64,
    enforce_risk_limits: bool,
) -> Result<PaperBrokerExecution, String> {
    order.symbol = canonical_symbol(&order.symbol);
    normalize_position_symbol(doc, &order.symbol)?;
    let prior_position = position_for_symbol(doc, &order.symbol)?;
    if let Some(position) = prior_position.as_ref() {
        if order.at_unix_ms < position.last_fill_unix_ms {
            return Err(format!(
                "paper broker order event time {} precedes the durable last fill {} for {}; \
                 refusing to apply stale market state",
                order.at_unix_ms, position.last_fill_unix_ms, order.symbol
            ));
        }
    }
    if order.bracket.is_some() {
        let prior_qty = prior_position
            .as_ref()
            .map(|position| position.qty)
            .unwrap_or(0.0);
        let signed_order_qty = order.side.signed_qty(order.qty);
        if prior_qty.abs() > f64::EPSILON && prior_qty.signum() != signed_order_qty.signum() {
            return Err(
                "paper brackets may only protect an entry from flat or a same-direction \
                 position increase; flatten a reversal before attaching new protection"
                    .to_string(),
            );
        }
    }
    if let Some(checkpoint) = bracket_checkpoint_for_symbol(doc, &order.symbol) {
        if order.bracket.is_some() && order.at_unix_ms < checkpoint.last_evaluated_bar_unix_ms {
            return Err(format!(
                "paper bracket entry event {} is older than the durable bracket checkpoint {} \
                 for {}; refusing to create protection whose intervening bars cannot be \
                 reconciled",
                order.at_unix_ms,
                checkpoint.last_evaluated_bar_unix_ms,
                canonical_symbol(&order.symbol)
            ));
        }
    }
    if enforce_risk_limits {
        if let Some(limits) = order.daily_risk_limits {
            enforce_daily_risk_limits(
                doc,
                PaperBrokerDailyRiskLimits {
                    at_unix_ms: processing_at_unix_ms,
                    ..limits
                },
            )?;
        }
    }

    let order_id = next_order_id(doc);
    let fill_id = next_fill_id(doc);

    let (fill_price, spread, slippage) = adverse_fill_price(
        order.price,
        order.side,
        order.spread_bps,
        order.slippage_bps,
    );
    let fee = order.qty * fill_price * (order.fee_bps.max(0.0) / 10_000.0);
    let signed_qty = order.side.signed_qty(order.qty);
    doc.account.cash -= signed_qty * fill_price;
    doc.account.cash -= fee;

    let realized_delta = apply_position_fill(
        &mut doc.positions,
        &order.symbol,
        signed_qty,
        fill_price,
        order.at_unix_ms,
    );
    let realized_pnl_delta = realized_delta - fee;
    doc.account.realized_pnl += realized_pnl_delta;

    let bracket_group_id = order
        .bracket
        .as_ref()
        .map(|_| format!("paper_bracket_{order_id}"));
    let stored_order = PaperBrokerOrder {
        id: order_id.clone(),
        client_order_id: order.client_order_id,
        parent_order_id: None,
        bracket_group_id: bracket_group_id.clone(),
        bracket_leg: order.bracket.as_ref().map(|_| PaperBracketLeg::Parent),
        symbol: order.symbol.clone(),
        side: order.side,
        qty: order.qty,
        order_type: "market".to_string(),
        time_in_force: "day".to_string(),
        status: "filled".to_string(),
        filled_qty: order.qty,
        avg_fill_price: fill_price,
        limit_price: None,
        stop_price: None,
        tie_break: order.bracket.as_ref().map(|bracket| bracket.tie_break),
        fee_bps: order.fee_bps.max(0.0),
        spread_bps: order.spread_bps.max(0.0),
        slippage_bps: order.slippage_bps.max(0.0),
        submitted_at_unix_ms: order.at_unix_ms,
        filled_at_unix_ms: order.at_unix_ms,
        filled_recorded_at_unix_ms: processing_at_unix_ms,
        updated_at_unix_ms: order.at_unix_ms,
        recorded_at_unix_ms: processing_at_unix_ms,
        canceled_at_unix_ms: None,
    };
    let fill = PaperBrokerFill {
        id: fill_id,
        order_id,
        symbol: order.symbol.clone(),
        side: order.side,
        qty: order.qty,
        price: fill_price,
        fee,
        spread,
        slippage,
        realized_pnl_delta,
        at_unix_ms: order.at_unix_ms,
        recorded_at_unix_ms: processing_at_unix_ms,
    };
    doc.orders.push(stored_order.clone());
    doc.fills.push(fill.clone());
    let child_orders = if let (Some(bracket), Some(group_id)) =
        (order.bracket.as_ref(), bracket_group_id.as_ref())
    {
        create_bracket_child_orders(doc, &stored_order, group_id, bracket)?
    } else {
        Vec::new()
    };
    doc.recompute_account(processing_at_unix_ms);

    Ok(PaperBrokerExecution {
        order: stored_order,
        fill,
        child_orders,
        account: doc.account.clone(),
        position: position_for_symbol(doc, &order.symbol)?,
    })
}

/// Atomically compare the durable position with `expected_position_qty`, derive
/// the order delta, optionally cancel owned protective children for a
/// risk-reducing target, and persist the resulting execution under one broker
/// lock. Mismatch and already-at-target outcomes do not consume sequences or
/// mutate broker state.
pub fn execute_target_if_position_matches(
    app_local_data_dir: &Path,
    request: PaperBrokerAtomicTargetOrder,
) -> Result<PaperBrokerAtomicTargetResult, String> {
    validate_atomic_target_request(&request)?;
    let symbol = canonical_symbol(&request.symbol);
    let _guard = paper_broker_state_lock()?;
    let mut doc = load_or_default_unlocked(app_local_data_dir, request.processing_at_unix_ms)?;
    validate_processing_time_against_state(&doc, request.processing_at_unix_ms)?;
    let observed_position = position_for_symbol(&doc, &symbol)?;
    let observed_position_qty = observed_position
        .as_ref()
        .map(|position| position.qty)
        .unwrap_or(0.0);

    if !quantities_match(observed_position_qty, request.expected_position_qty) {
        return Ok(PaperBrokerAtomicTargetResult {
            status: PaperBrokerAtomicTargetStatus::PositionMismatch,
            expected_position_qty: request.expected_position_qty,
            observed_position_qty,
            target_position_qty: request.target_position_qty,
            execution: None,
            cancelled_orders: Vec::new(),
            account: doc.account.clone(),
            position: observed_position,
        });
    }
    if quantities_match(observed_position_qty, request.target_position_qty) {
        return Ok(PaperBrokerAtomicTargetResult {
            status: PaperBrokerAtomicTargetStatus::AlreadyAtTarget,
            expected_position_qty: request.expected_position_qty,
            observed_position_qty,
            target_position_qty: request.target_position_qty,
            execution: None,
            cancelled_orders: Vec::new(),
            account: doc.account.clone(),
            position: observed_position,
        });
    }

    let delta = request.target_position_qty - observed_position_qty;
    let side = if delta > 0.0 {
        PaperOrderSide::Buy
    } else {
        PaperOrderSide::Sell
    };
    let daily_risk_limits = request
        .daily_risk_limits
        .map(|limits| PaperBrokerDailyRiskLimits {
            at_unix_ms: request.processing_at_unix_ms,
            ..limits
        });
    let order = PaperBrokerMarketOrder {
        symbol: symbol.clone(),
        side,
        qty: delta.abs(),
        price: request.price,
        at_unix_ms: request.event_at_unix_ms,
        fee_bps: request.fee_bps,
        spread_bps: request.spread_bps,
        slippage_bps: request.slippage_bps,
        client_order_id: request.client_order_id,
        bracket: request.bracket,
        daily_risk_limits,
    };
    validate_market_order(&order)?;
    if let Some(bracket) = order.bracket.as_ref() {
        validate_bracket_spec(&order, bracket)?;
    }
    if request.cancel_owned_brackets_for_risk_reduction
        && !is_strictly_risk_reducing_target(observed_position_qty, request.target_position_qty)
    {
        return Err(
            "owned paper brackets may only be atomically cancelled for a strictly \
             exposure-reducing target"
                .to_string(),
        );
    }
    if request.cancel_owned_brackets_for_risk_reduction && order.bracket.is_some() {
        return Err(
            "a risk-reducing paper target cannot attach a new bracket while atomically \
             cancelling its existing protective children"
                .to_string(),
        );
    }
    if let Some(limits) = daily_risk_limits {
        // Fail before touching protective children when the new order would be
        // rejected by a durable daily risk gate.
        enforce_daily_risk_limits(&doc, limits)?;
    }

    normalize_position_symbol(&mut doc, &symbol)?;
    let mut cancelled_orders = Vec::new();
    if request.cancel_owned_brackets_for_risk_reduction {
        let indices = doc
            .orders
            .iter()
            .enumerate()
            .filter_map(|(idx, existing)| {
                (is_open_bracket_child(existing) && symbols_match(&existing.symbol, &symbol))
                    .then_some(idx)
            })
            .collect::<Vec<_>>();
        if let Some(order) = indices
            .iter()
            .filter_map(|idx| doc.orders.get(*idx))
            .find(|order| order.submitted_at_unix_ms > request.event_at_unix_ms)
        {
            return Err(format!(
                "paper broker risk-reduction event time {} precedes protective order {} \
                 submission {}; refusing to backdate cancellation",
                request.event_at_unix_ms, order.id, order.submitted_at_unix_ms
            ));
        }
        for idx in indices {
            cancelled_orders.push(cancel_open_order_at_index(
                &mut doc,
                idx,
                request.event_at_unix_ms,
            )?);
        }
    }

    let execution =
        submit_market_order_unlocked(&mut doc, order, request.processing_at_unix_ms, false)?;
    save_unlocked(app_local_data_dir, &doc)?;
    Ok(PaperBrokerAtomicTargetResult {
        status: PaperBrokerAtomicTargetStatus::Executed,
        expected_position_qty: request.expected_position_qty,
        observed_position_qty,
        target_position_qty: request.target_position_qty,
        execution: Some(execution),
        cancelled_orders,
        account: doc.account.clone(),
        position: position_for_symbol(&doc, &symbol)?,
    })
}

pub fn advance_brackets_for_bar(
    app_local_data_dir: &Path,
    bar: PaperBrokerBracketAdvanceBar,
) -> Result<PaperBrokerBracketAdvanceResult, String> {
    // A single latest bar cannot prove that no intervening stop/limit touch was
    // skipped. Route this compatibility API through the durable reconciliation
    // contract: it may establish the initial anchor (or repeat that anchor),
    // but advancing beyond it requires callers to provide the complete ordered
    // history through `reconcile_brackets_for_bars`.
    let result = reconcile_brackets_for_bars(
        app_local_data_dir,
        PaperBrokerBracketReconcileRequest {
            symbol: bar.symbol.clone(),
            processing_at_unix_ms: bar.at_unix_ms,
            bars: vec![bar],
        },
    )?;
    Ok(PaperBrokerBracketAdvanceResult {
        open_order_count_before: result.open_order_count_before,
        filled_orders: result.filled_orders,
        cancelled_orders: result.cancelled_orders,
        tie_break_events: result.tie_break_events,
        account: result.account,
        position: result.position,
    })
}

fn advance_brackets_for_bar_unlocked(
    doc: &mut PaperBrokerStateDoc,
    bar: &PaperBrokerBracketAdvanceBar,
    processing_at_unix_ms: i64,
) -> Result<PaperBrokerBracketAdvanceResult, String> {
    let symbol = canonical_symbol(&bar.symbol);
    normalize_position_symbol(doc, &symbol)?;
    let open_order_count_before = doc
        .orders
        .iter()
        .filter(|order| is_open_order(order) && symbols_match(&order.symbol, &symbol))
        .count();

    #[derive(Default)]
    struct OpenBracketGroup {
        stop_idx: Option<usize>,
        take_idx: Option<usize>,
    }

    let mut groups: BTreeMap<String, OpenBracketGroup> = BTreeMap::new();
    for (idx, order) in doc.orders.iter().enumerate() {
        if !is_open_order(order)
            || !symbols_match(&order.symbol, &symbol)
            || order.submitted_at_unix_ms >= bar.at_unix_ms
        {
            continue;
        }
        let Some(group_id) = order.bracket_group_id.clone() else {
            continue;
        };
        let group = groups.entry(group_id.clone()).or_default();
        match order.bracket_leg {
            Some(PaperBracketLeg::StopLoss) => {
                if group.stop_idx.replace(idx).is_some() {
                    return Err(format!(
                        "paper bracket group {group_id} has multiple open stop-loss children; \
                         refusing ambiguous OCO evaluation"
                    ));
                }
            }
            Some(PaperBracketLeg::TakeProfit) => {
                if group.take_idx.replace(idx).is_some() {
                    return Err(format!(
                        "paper bracket group {group_id} has multiple open take-profit children; \
                         refusing ambiguous OCO evaluation"
                    ));
                }
            }
            _ => {}
        }
    }

    let mut filled_orders = Vec::new();
    let mut cancelled_orders = Vec::new();
    let mut tie_break_events = Vec::new();
    for (group_id, group) in groups {
        let (Some(stop_idx), Some(take_idx)) = (group.stop_idx, group.take_idx) else {
            return Err(format!(
                "paper bracket group {group_id} has an incomplete open OCO pair; cancel or \
                 reconcile the protective orders before advancing"
            ));
        };
        validate_open_bracket_pair(doc, &group_id, stop_idx, take_idx)?;
        let stop_hit = group
            .stop_idx
            .and_then(|idx| doc.orders.get(idx))
            .is_some_and(|order| bracket_child_hit(order, bar));
        let take_hit = group
            .take_idx
            .and_then(|idx| doc.orders.get(idx))
            .is_some_and(|order| bracket_child_hit(order, bar));
        let fill_idx = if stop_hit {
            if take_hit {
                tie_break_events.push(format!(
                    "{group_id}: both stop-loss and take-profit touched; tie_break=stop_loss_first"
                ));
            }
            group.stop_idx
        } else if take_hit {
            group.take_idx
        } else {
            None
        };
        let Some(fill_idx) = fill_idx else {
            continue;
        };
        validate_bracket_child_reduces_position(doc, fill_idx)?;
        let fill_price = bracket_child_fill_reference_price(&doc.orders[fill_idx], bar)
            .ok_or_else(|| {
                format!(
                    "open bracket order {} is missing trigger price",
                    doc.orders[fill_idx].id
                )
            })?;
        let filled = fill_open_order_at_price(
            doc,
            fill_idx,
            fill_price,
            bar.at_unix_ms,
            processing_at_unix_ms,
        )?;
        filled_orders.push(filled);
        let sibling_idx = if Some(fill_idx) == group.stop_idx {
            group.take_idx
        } else {
            group.stop_idx
        };
        if let Some(sibling_idx) = sibling_idx {
            if doc.orders.get(sibling_idx).is_some_and(is_open_order) {
                let cancelled = cancel_open_order_at_index(doc, sibling_idx, bar.at_unix_ms)?;
                cancelled_orders.push(cancelled);
            }
        }
    }

    if let Some(position) = doc.positions.get_mut(&symbol) {
        update_position_marks(position, bar.close);
    }
    doc.recompute_account(processing_at_unix_ms);
    Ok(PaperBrokerBracketAdvanceResult {
        open_order_count_before,
        filled_orders,
        cancelled_orders,
        tie_break_events,
        account: doc.account.clone(),
        position: position_for_symbol(doc, &symbol)?,
    })
}

/// Reconcile a complete, ordered bar history against durable bracket state.
///
/// Once a checkpoint exists, the first supplied bar must exactly equal it. The
/// anchor is skipped and every subsequent bar is evaluated once. On first use
/// with open bracket children, history must begin no later than the earliest
/// child submission. A latest-bar-only call therefore fails closed after
/// downtime instead of silently skipping unseen stop or limit touches.
pub fn reconcile_brackets_for_bars(
    app_local_data_dir: &Path,
    request: PaperBrokerBracketReconcileRequest,
) -> Result<PaperBrokerBracketReconcileResult, String> {
    validate_bracket_reconcile_request(&request)?;
    let symbol = canonical_symbol(&request.symbol);
    let supplied_bar_count = request.bars.len();
    let _guard = paper_broker_state_lock()?;
    let mut doc = load_or_default_unlocked(app_local_data_dir, request.processing_at_unix_ms)?;
    validate_processing_time_against_state(&doc, request.processing_at_unix_ms)?;
    let previous_checkpoint = bracket_checkpoint_for_symbol(&doc, &symbol).cloned();
    let open_order_count_before = doc
        .orders
        .iter()
        .filter(|order| is_open_order(order) && symbols_match(&order.symbol, &symbol))
        .count();

    let start_idx = if let Some(checkpoint) = previous_checkpoint.as_ref() {
        let supplied_anchor = request.bars[0].at_unix_ms;
        if supplied_anchor != checkpoint.last_evaluated_bar_unix_ms {
            return Err(format!(
                "paper bracket reconciliation for {symbol} requires checkpoint anchor {}, but \
                 the first supplied bar is {supplied_anchor}; load the complete ordered bar \
                 history beginning at the durable checkpoint before advancing",
                checkpoint.last_evaluated_bar_unix_ms
            ));
        }
        if request.processing_at_unix_ms < checkpoint.last_processed_at_unix_ms {
            return Err(format!(
                "paper bracket reconciliation processing time {} precedes durable processing \
                 checkpoint {} for {symbol}",
                request.processing_at_unix_ms, checkpoint.last_processed_at_unix_ms
            ));
        }
        let supplied_fingerprint = bracket_bar_fingerprint(&request.bars[0]);
        if checkpoint.last_evaluated_bar_fingerprint.is_empty() {
            if open_order_count_before > 0 {
                return Err(format!(
                    "paper bracket checkpoint {} for {symbol} predates anchor fingerprinting \
                     while protective orders remain open; refusing unauditable advancement",
                    checkpoint.last_evaluated_bar_unix_ms
                ));
            }
        } else if supplied_fingerprint != checkpoint.last_evaluated_bar_fingerprint {
            return Err(format!(
                "paper bracket checkpoint anchor {} for {symbol} has different OHLC content; \
                 refusing to advance across revised or mismatched history",
                checkpoint.last_evaluated_bar_unix_ms
            ));
        }
        1
    } else {
        let earliest_open_child_submission = doc
            .orders
            .iter()
            .filter(|order| is_open_bracket_child(order) && symbols_match(&order.symbol, &symbol))
            .map(|order| order.submitted_at_unix_ms)
            .min();
        if let Some(earliest) = earliest_open_child_submission {
            if request.bars[0].at_unix_ms > earliest {
                return Err(format!(
                    "paper bracket reconciliation for {symbol} has no durable checkpoint and \
                     open protection submitted at {earliest}, but history begins at {}; replay \
                     complete ordered bars from at or before {earliest}",
                    request.bars[0].at_unix_ms
                ));
            }
        }
        0
    };

    let mut filled_orders = Vec::new();
    let mut cancelled_orders = Vec::new();
    let mut tie_break_events = Vec::new();
    for bar in request.bars.iter().skip(start_idx) {
        let result =
            advance_brackets_for_bar_unlocked(&mut doc, bar, request.processing_at_unix_ms)?;
        filled_orders.extend(result.filled_orders);
        cancelled_orders.extend(result.cancelled_orders);
        tie_break_events.extend(result.tie_break_events);
    }
    let last_evaluated_bar_unix_ms = request
        .bars
        .last()
        .expect("reconciliation validation requires at least one bar")
        .at_unix_ms;
    let last_evaluated_bar_fingerprint = bracket_bar_fingerprint(
        request
            .bars
            .last()
            .expect("reconciliation validation requires at least one bar"),
    );
    doc.bracket_checkpoints.insert(
        symbol.clone(),
        PaperBrokerBracketCheckpoint {
            last_evaluated_bar_unix_ms,
            last_processed_at_unix_ms: request.processing_at_unix_ms,
            last_evaluated_bar_fingerprint,
        },
    );
    doc.recompute_account(request.processing_at_unix_ms);
    save_unlocked(app_local_data_dir, &doc)?;

    Ok(PaperBrokerBracketReconcileResult {
        supplied_bar_count,
        processed_bar_count: supplied_bar_count.saturating_sub(start_idx),
        previous_checkpoint_unix_ms: previous_checkpoint
            .map(|checkpoint| checkpoint.last_evaluated_bar_unix_ms),
        last_evaluated_bar_unix_ms,
        open_order_count_before,
        filled_orders,
        cancelled_orders,
        tie_break_events,
        account: doc.account.clone(),
        position: position_for_symbol(&doc, &symbol)?,
    })
}

pub fn cancel_open_orders_for_symbol(
    app_local_data_dir: &Path,
    symbol: &str,
    now_unix_ms: i64,
) -> Result<PaperBrokerCancelOpenOrdersResult, String> {
    let symbol = canonical_symbol(symbol);
    if symbol.is_empty() {
        return Err("symbol is required".to_string());
    }
    validate_processing_time(now_unix_ms)?;
    let _guard = paper_broker_state_lock()?;
    let mut doc = load_or_default_unlocked(app_local_data_dir, now_unix_ms)?;
    validate_processing_time_against_state(&doc, now_unix_ms)?;
    let indices = doc
        .orders
        .iter()
        .enumerate()
        .filter_map(|(idx, order)| {
            (is_open_order(order) && symbols_match(&order.symbol, &symbol)).then_some(idx)
        })
        .collect::<Vec<_>>();
    if let Some(order) = indices
        .iter()
        .filter_map(|idx| doc.orders.get(*idx))
        .find(|order| order.submitted_at_unix_ms > now_unix_ms)
    {
        return Err(format!(
            "paper broker cancellation time {now_unix_ms} precedes order {} submission {}; \
             refusing to backdate cancellation",
            order.id, order.submitted_at_unix_ms
        ));
    }
    let open_order_count_before = indices.len();
    let mut cancelled_orders = Vec::with_capacity(indices.len());
    for idx in indices {
        cancelled_orders.push(cancel_open_order_at_index(&mut doc, idx, now_unix_ms)?);
    }
    doc.recompute_account(now_unix_ms);
    save_unlocked(app_local_data_dir, &doc)?;
    Ok(PaperBrokerCancelOpenOrdersResult {
        open_order_count_before,
        cancelled_orders,
        account: doc.account.clone(),
        position: position_for_symbol(&doc, &symbol)?,
    })
}

impl PaperBrokerStateDoc {
    fn new(now_unix_ms: i64) -> Self {
        Self {
            version: PAPER_BROKER_VERSION,
            account: PaperBrokerAccount {
                account_id: "local-paper".to_string(),
                currency: "USD".to_string(),
                starting_cash: DEFAULT_STARTING_CASH,
                cash: DEFAULT_STARTING_CASH,
                buying_power: DEFAULT_STARTING_CASH,
                portfolio_value: DEFAULT_STARTING_CASH,
                realized_pnl: 0.0,
                updated_at_unix_ms: now_unix_ms,
            },
            positions: BTreeMap::new(),
            orders: Vec::new(),
            fills: Vec::new(),
            bracket_checkpoints: BTreeMap::new(),
            next_order_seq: 1,
            next_fill_seq: 1,
        }
    }

    pub fn recompute_account(&mut self, now_unix_ms: i64) {
        let position_value = self
            .positions
            .values_mut()
            .map(|position| {
                update_position_marks(position, position.market_price);
                position.market_value
            })
            .sum::<f64>();
        self.account.portfolio_value = self.account.cash + position_value;
        self.account.buying_power = self.account.cash.max(0.0);
        self.account.updated_at_unix_ms = now_unix_ms;
    }
}

fn validate_state_doc(doc: &PaperBrokerStateDoc) -> Result<(), String> {
    if doc.version != PAPER_BROKER_VERSION {
        return Err(format!(
            "unsupported paper broker state version {}",
            doc.version
        ));
    }
    if doc.account.account_id.trim().is_empty() || doc.account.currency.trim().is_empty() {
        return Err("account identity and currency must not be empty".to_string());
    }
    for (name, value) in [
        ("starting_cash", doc.account.starting_cash),
        ("cash", doc.account.cash),
        ("buying_power", doc.account.buying_power),
        ("portfolio_value", doc.account.portfolio_value),
        ("realized_pnl", doc.account.realized_pnl),
    ] {
        if !value.is_finite() {
            return Err(format!("account {name} must be finite"));
        }
    }
    if doc.account.starting_cash <= 0.0 || doc.account.buying_power < 0.0 {
        return Err("account starting cash must be > 0 and buying power must be >= 0".to_string());
    }
    if doc.account.updated_at_unix_ms < 0 {
        return Err("account update time must be >= 0".to_string());
    }

    let mut canonical_positions = BTreeSet::new();
    for (key, position) in &doc.positions {
        let canonical = canonical_symbol(&position.symbol);
        if canonical.is_empty() || !symbols_match(key, &position.symbol) {
            return Err(format!(
                "position key {key:?} does not match a non-empty position symbol {:?}",
                position.symbol
            ));
        }
        if !canonical_positions.insert(canonical.clone()) {
            return Err(format!(
                "multiple position records map to canonical symbol {canonical}"
            ));
        }
        for (name, value) in [
            ("qty", position.qty),
            ("avg_entry_price", position.avg_entry_price),
            ("market_price", position.market_price),
            ("market_value", position.market_value),
            ("unrealized_pnl", position.unrealized_pnl),
        ] {
            if !value.is_finite() {
                return Err(format!("position {canonical} {name} must be finite"));
            }
        }
        if position.qty.abs() <= f64::EPSILON
            || position.avg_entry_price <= 0.0
            || position.market_price <= 0.0
            || position.last_fill_unix_ms <= 0
        {
            return Err(format!(
                "position {canonical} has invalid quantity, price, or fill time"
            ));
        }
    }

    let mut order_ids = BTreeSet::new();
    let mut max_order_seq = 0_u64;
    for order in &doc.orders {
        if order.id.trim().is_empty() || !order_ids.insert(order.id.clone()) {
            return Err(format!(
                "paper order id {:?} is empty or duplicated",
                order.id
            ));
        }
        if canonical_symbol(&order.symbol).is_empty() {
            return Err(format!("paper order {} has an empty symbol", order.id));
        }
        if !order.qty.is_finite()
            || order.qty <= 0.0
            || !order.filled_qty.is_finite()
            || order.filled_qty < 0.0
            || order.filled_qty > order.qty + 1e-9
            || !order.avg_fill_price.is_finite()
            || order.avg_fill_price < 0.0
        {
            return Err(format!(
                "paper order {} has invalid fill quantities",
                order.id
            ));
        }
        for (name, value) in [
            ("fee_bps", order.fee_bps),
            ("spread_bps", order.spread_bps),
            ("slippage_bps", order.slippage_bps),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("paper order {} {name} is invalid", order.id));
            }
        }
        for (name, value) in [
            ("limit_price", order.limit_price),
            ("stop_price", order.stop_price),
        ] {
            if value.is_some_and(|price| !price.is_finite() || price <= 0.0) {
                return Err(format!("paper order {} {name} is invalid", order.id));
            }
        }
        if order.submitted_at_unix_ms <= 0
            || order.filled_at_unix_ms < 0
            || order.filled_recorded_at_unix_ms < 0
            || order.updated_at_unix_ms < 0
            || order.recorded_at_unix_ms < 0
            || order.canceled_at_unix_ms.is_some_and(|value| value <= 0)
        {
            return Err(format!("paper order {} has invalid timestamps", order.id));
        }
        if (order.recorded_at_unix_ms > 0 && order.recorded_at_unix_ms < order.submitted_at_unix_ms)
            || (order.filled_recorded_at_unix_ms > 0
                && order.filled_recorded_at_unix_ms < order.filled_at_unix_ms)
            || order
                .canceled_at_unix_ms
                .is_some_and(|value| value < order.submitted_at_unix_ms)
        {
            return Err(format!(
                "paper order {} records processing before its market event",
                order.id
            ));
        }
        if let Some(seq) = parse_paper_sequence(&order.id, "paper_ord_") {
            max_order_seq = max_order_seq.max(seq);
        }
    }
    if doc.next_order_seq == 0 || doc.next_order_seq <= max_order_seq {
        return Err(format!(
            "next_order_seq {} does not exceed durable maximum {max_order_seq}",
            doc.next_order_seq
        ));
    }

    let mut fill_ids = BTreeSet::new();
    let mut max_fill_seq = 0_u64;
    for fill in &doc.fills {
        if fill.id.trim().is_empty() || !fill_ids.insert(fill.id.clone()) {
            return Err(format!(
                "paper fill id {:?} is empty or duplicated",
                fill.id
            ));
        }
        if !order_ids.contains(&fill.order_id) {
            return Err(format!(
                "paper fill {} references missing order {}",
                fill.id, fill.order_id
            ));
        }
        if canonical_symbol(&fill.symbol).is_empty()
            || !fill.qty.is_finite()
            || fill.qty <= 0.0
            || !fill.price.is_finite()
            || fill.price <= 0.0
            || !fill.fee.is_finite()
            || fill.fee < 0.0
            || !fill.spread.is_finite()
            || fill.spread < 0.0
            || !fill.slippage.is_finite()
            || fill.slippage < 0.0
            || !fill.realized_pnl_delta.is_finite()
            || fill.at_unix_ms <= 0
            || fill.recorded_at_unix_ms < 0
            || (fill.recorded_at_unix_ms > 0 && fill.recorded_at_unix_ms < fill.at_unix_ms)
        {
            return Err(format!("paper fill {} has invalid values", fill.id));
        }
        if let Some(seq) = parse_paper_sequence(&fill.id, "paper_fill_") {
            max_fill_seq = max_fill_seq.max(seq);
        }
    }
    if doc.next_fill_seq == 0 || doc.next_fill_seq <= max_fill_seq {
        return Err(format!(
            "next_fill_seq {} does not exceed durable maximum {max_fill_seq}",
            doc.next_fill_seq
        ));
    }

    let mut replay_positions = BTreeMap::new();
    let mut replay_cash = doc.account.starting_cash;
    let mut replay_realized_pnl = 0.0;
    for fill in &doc.fills {
        let signed_qty = fill.side.signed_qty(fill.qty);
        replay_cash -= signed_qty * fill.price;
        replay_cash -= fill.fee;
        replay_realized_pnl += apply_position_fill(
            &mut replay_positions,
            &fill.symbol,
            signed_qty,
            fill.price,
            fill.at_unix_ms,
        ) - fill.fee;
    }
    if !financial_values_match(replay_cash, doc.account.cash)
        || !financial_values_match(replay_realized_pnl, doc.account.realized_pnl)
    {
        return Err(format!(
            "account ledger mismatch: durable cash/realized_pnl={:.12}/{:.12}, \
             replayed={replay_cash:.12}/{replay_realized_pnl:.12}",
            doc.account.cash, doc.account.realized_pnl
        ));
    }
    if replay_positions.len() != canonical_positions.len() {
        return Err(format!(
            "position ledger mismatch: {} durable position(s), {} replayed position(s)",
            canonical_positions.len(),
            replay_positions.len()
        ));
    }
    for (symbol, replayed) in replay_positions {
        let durable = doc
            .positions
            .values()
            .find(|position| symbols_match(&position.symbol, &symbol))
            .ok_or_else(|| format!("replayed position {symbol} is missing from durable state"))?;
        if !quantities_match(replayed.qty, durable.qty)
            || !financial_values_match(replayed.avg_entry_price, durable.avg_entry_price)
        {
            return Err(format!(
                "position ledger mismatch for {symbol}: durable qty/average={:.12}/{:.12}, \
                 replayed={:.12}/{:.12}",
                durable.qty, durable.avg_entry_price, replayed.qty, replayed.avg_entry_price
            ));
        }
    }

    let mut canonical_checkpoints = BTreeSet::new();
    for (symbol, checkpoint) in &doc.bracket_checkpoints {
        let canonical = canonical_symbol(symbol);
        if canonical.is_empty() || !canonical_checkpoints.insert(canonical.clone()) {
            return Err(format!(
                "paper bracket checkpoint symbol {symbol:?} is empty or duplicated canonically"
            ));
        }
        if checkpoint.last_evaluated_bar_unix_ms <= 0
            || checkpoint.last_processed_at_unix_ms <= 0
            || checkpoint.last_evaluated_bar_unix_ms > checkpoint.last_processed_at_unix_ms
        {
            return Err(format!(
                "paper bracket checkpoint for {canonical} has invalid timestamps"
            ));
        }
        if !checkpoint.last_evaluated_bar_fingerprint.is_empty()
            && !valid_bracket_bar_fingerprint(&checkpoint.last_evaluated_bar_fingerprint)
        {
            return Err(format!(
                "paper bracket checkpoint for {canonical} has an invalid anchor fingerprint"
            ));
        }
    }
    Ok(())
}

fn parse_paper_sequence(id: &str, prefix: &str) -> Option<u64> {
    id.strip_prefix(prefix)?.parse().ok()
}

fn financial_values_match(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1e-10 * scale
}

fn validate_market_order(order: &PaperBrokerMarketOrder) -> Result<(), String> {
    if order.symbol.trim().is_empty() {
        return Err("symbol is required".to_string());
    }
    if !order.qty.is_finite() || order.qty <= 0.0 {
        return Err("qty must be finite and > 0".to_string());
    }
    if !order.price.is_finite() || order.price <= 0.0 {
        return Err("paper broker fill price must be finite and > 0".to_string());
    }
    if order.at_unix_ms <= 0 {
        return Err("paper broker event time must be > 0".to_string());
    }
    for (name, value) in [
        ("fee_bps", order.fee_bps),
        ("spread_bps", order.spread_bps),
        ("slippage_bps", order.slippage_bps),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("paper broker {name} must be finite and >= 0"));
        }
    }
    if let Some(limits) = order.daily_risk_limits {
        if limits
            .max_daily_loss
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err("paper broker max_daily_loss must be finite and > 0".to_string());
        }
        if matches!(limits.max_daily_orders, Some(0)) {
            return Err("paper broker max_daily_orders must be > 0".to_string());
        }
    }
    Ok(())
}

fn validate_processing_time(processing_at_unix_ms: i64) -> Result<(), String> {
    if processing_at_unix_ms <= 0 {
        return Err("paper broker processing time must be > 0".to_string());
    }
    Ok(())
}

fn validate_processing_time_against_state(
    doc: &PaperBrokerStateDoc,
    processing_at_unix_ms: i64,
) -> Result<(), String> {
    if processing_at_unix_ms < doc.account.updated_at_unix_ms {
        return Err(format!(
            "paper broker processing time {processing_at_unix_ms} precedes durable state time {}; \
             refusing out-of-order mutation",
            doc.account.updated_at_unix_ms
        ));
    }
    Ok(())
}

fn validate_atomic_target_request(request: &PaperBrokerAtomicTargetOrder) -> Result<(), String> {
    if canonical_symbol(&request.symbol).is_empty() {
        return Err("symbol is required".to_string());
    }
    for (name, value) in [
        ("expected_position_qty", request.expected_position_qty),
        ("target_position_qty", request.target_position_qty),
    ] {
        if !value.is_finite() {
            return Err(format!("paper broker {name} must be finite"));
        }
    }
    if !request.price.is_finite() || request.price <= 0.0 {
        return Err("paper broker fill price must be finite and > 0".to_string());
    }
    if request.event_at_unix_ms <= 0 {
        return Err("paper broker event time must be > 0".to_string());
    }
    validate_processing_time(request.processing_at_unix_ms)?;
    if request.event_at_unix_ms > request.processing_at_unix_ms {
        return Err("paper broker event time cannot be after processing time".to_string());
    }
    for (name, value) in [
        ("fee_bps", request.fee_bps),
        ("spread_bps", request.spread_bps),
        ("slippage_bps", request.slippage_bps),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("paper broker {name} must be finite and >= 0"));
        }
    }
    if let Some(limits) = request.daily_risk_limits {
        if limits
            .max_daily_loss
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err("paper broker max_daily_loss must be finite and > 0".to_string());
        }
        if matches!(limits.max_daily_orders, Some(0)) {
            return Err("paper broker max_daily_orders must be > 0".to_string());
        }
    }
    Ok(())
}

fn validate_bracket_spec(
    order: &PaperBrokerMarketOrder,
    bracket: &PaperBrokerBracketSpec,
) -> Result<(), String> {
    if !bracket.stop_price.is_finite() || bracket.stop_price <= 0.0 {
        return Err("paper bracket stop price must be finite and > 0".to_string());
    }
    if !bracket.take_profit_limit_price.is_finite() || bracket.take_profit_limit_price <= 0.0 {
        return Err("paper bracket take-profit limit price must be finite and > 0".to_string());
    }
    let tif = bracket.time_in_force.trim().to_ascii_lowercase();
    if tif != "day" && tif != "gtc" {
        return Err("paper bracket time_in_force must be day or gtc".to_string());
    }
    match order.side {
        PaperOrderSide::Buy => {
            if !(bracket.stop_price < order.price && order.price < bracket.take_profit_limit_price)
            {
                return Err(
                    "paper buy bracket requires stop below entry and take-profit above entry"
                        .to_string(),
                );
            }
        }
        PaperOrderSide::Sell => {
            if !(bracket.take_profit_limit_price < order.price && order.price < bracket.stop_price)
            {
                return Err(
                    "paper sell bracket requires take-profit below entry and stop above entry"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn validate_advance_bar(bar: &PaperBrokerBracketAdvanceBar) -> Result<(), String> {
    if canonical_symbol(&bar.symbol).is_empty() {
        return Err("symbol is required".to_string());
    }
    for (name, value) in [
        ("open", bar.open),
        ("high", bar.high),
        ("low", bar.low),
        ("close", bar.close),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("paper bracket bar {name} must be finite and > 0"));
        }
    }
    if bar.high < bar.low {
        return Err("paper bracket bar high must be >= low".to_string());
    }
    if bar.open < bar.low || bar.open > bar.high || bar.close < bar.low || bar.close > bar.high {
        return Err(
            "paper bracket bar open and close must lie within the low/high range".to_string(),
        );
    }
    if bar.at_unix_ms <= 0 {
        return Err("paper bracket bar time must be > 0".to_string());
    }
    Ok(())
}

fn validate_bracket_reconcile_request(
    request: &PaperBrokerBracketReconcileRequest,
) -> Result<(), String> {
    let symbol = canonical_symbol(&request.symbol);
    if symbol.is_empty() {
        return Err("symbol is required".to_string());
    }
    validate_processing_time(request.processing_at_unix_ms)?;
    if request.bars.is_empty() {
        return Err(
            "paper bracket reconciliation requires at least one checkpoint/history bar".to_string(),
        );
    }
    let mut previous_at_unix_ms = None;
    for bar in &request.bars {
        validate_advance_bar(bar)?;
        if !symbols_match(&bar.symbol, &symbol) {
            return Err(format!(
                "paper bracket reconciliation bar symbol {} does not match request symbol \
                 {symbol}",
                canonical_symbol(&bar.symbol)
            ));
        }
        if bar.at_unix_ms > request.processing_at_unix_ms {
            return Err(format!(
                "paper bracket bar event time {} cannot be after processing time {}",
                bar.at_unix_ms, request.processing_at_unix_ms
            ));
        }
        if previous_at_unix_ms.is_some_and(|previous| bar.at_unix_ms <= previous) {
            return Err(
                "paper bracket reconciliation bars must be strictly increasing without \
                 duplicate timestamps"
                    .to_string(),
            );
        }
        previous_at_unix_ms = Some(bar.at_unix_ms);
    }
    Ok(())
}

fn create_bracket_child_orders(
    doc: &mut PaperBrokerStateDoc,
    parent: &PaperBrokerOrder,
    group_id: &str,
    bracket: &PaperBrokerBracketSpec,
) -> Result<Vec<PaperBrokerOrder>, String> {
    let child_side = match parent.side {
        PaperOrderSide::Buy => PaperOrderSide::Sell,
        PaperOrderSide::Sell => PaperOrderSide::Buy,
    };
    let stop_id = next_order_id(doc);
    let take_id = next_order_id(doc);
    let common_tif = bracket.time_in_force.trim().to_ascii_lowercase();
    let stop = PaperBrokerOrder {
        id: stop_id,
        client_order_id: None,
        parent_order_id: Some(parent.id.clone()),
        bracket_group_id: Some(group_id.to_string()),
        bracket_leg: Some(PaperBracketLeg::StopLoss),
        symbol: parent.symbol.clone(),
        side: child_side,
        qty: parent.qty,
        order_type: "stop".to_string(),
        time_in_force: common_tif.clone(),
        status: "new".to_string(),
        filled_qty: 0.0,
        avg_fill_price: 0.0,
        limit_price: None,
        stop_price: Some(bracket.stop_price),
        tie_break: Some(bracket.tie_break),
        fee_bps: parent.fee_bps,
        spread_bps: parent.spread_bps,
        slippage_bps: parent.slippage_bps,
        submitted_at_unix_ms: parent.filled_at_unix_ms,
        filled_at_unix_ms: 0,
        filled_recorded_at_unix_ms: 0,
        updated_at_unix_ms: parent.filled_at_unix_ms,
        recorded_at_unix_ms: parent.recorded_at_unix_ms,
        canceled_at_unix_ms: None,
    };
    let take = PaperBrokerOrder {
        id: take_id,
        client_order_id: None,
        parent_order_id: Some(parent.id.clone()),
        bracket_group_id: Some(group_id.to_string()),
        bracket_leg: Some(PaperBracketLeg::TakeProfit),
        symbol: parent.symbol.clone(),
        side: child_side,
        qty: parent.qty,
        order_type: "limit".to_string(),
        time_in_force: common_tif,
        status: "new".to_string(),
        filled_qty: 0.0,
        avg_fill_price: 0.0,
        limit_price: Some(bracket.take_profit_limit_price),
        stop_price: None,
        tie_break: Some(bracket.tie_break),
        fee_bps: parent.fee_bps,
        spread_bps: parent.spread_bps,
        slippage_bps: parent.slippage_bps,
        submitted_at_unix_ms: parent.filled_at_unix_ms,
        filled_at_unix_ms: 0,
        filled_recorded_at_unix_ms: 0,
        updated_at_unix_ms: parent.filled_at_unix_ms,
        recorded_at_unix_ms: parent.recorded_at_unix_ms,
        canceled_at_unix_ms: None,
    };
    doc.orders.push(stop.clone());
    doc.orders.push(take.clone());
    Ok(vec![stop, take])
}

fn next_order_id(doc: &mut PaperBrokerStateDoc) -> String {
    let order_id = format!("paper_ord_{:012}", doc.next_order_seq);
    doc.next_order_seq = doc.next_order_seq.saturating_add(1);
    order_id
}

fn next_fill_id(doc: &mut PaperBrokerStateDoc) -> String {
    let fill_id = format!("paper_fill_{:012}", doc.next_fill_seq);
    doc.next_fill_seq = doc.next_fill_seq.saturating_add(1);
    fill_id
}

fn canonical_symbol(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn symbols_match(left: &str, right: &str) -> bool {
    canonical_symbol(left) == canonical_symbol(right)
}

fn quantities_match(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}

fn is_strictly_risk_reducing_target(current_qty: f64, target_qty: f64) -> bool {
    if quantities_match(current_qty, 0.0)
        || target_qty.abs() >= current_qty.abs()
        || (!quantities_match(target_qty, 0.0) && current_qty.signum() != target_qty.signum())
    {
        return false;
    }
    true
}

fn position_for_symbol(
    doc: &PaperBrokerStateDoc,
    symbol: &str,
) -> Result<Option<PaperBrokerPosition>, String> {
    let mut matches = doc
        .positions
        .values()
        .filter(|position| symbols_match(&position.symbol, symbol));
    let position = matches.next().cloned();
    if matches.next().is_some() {
        return Err(format!(
            "paper broker has multiple position records matching canonical symbol {}; \
             reconcile state before target execution",
            canonical_symbol(symbol)
        ));
    }
    Ok(position)
}

fn normalize_position_symbol(doc: &mut PaperBrokerStateDoc, canonical: &str) -> Result<(), String> {
    let matching_keys = doc
        .positions
        .iter()
        .filter_map(|(key, position)| {
            symbols_match(&position.symbol, canonical).then_some(key.clone())
        })
        .collect::<Vec<_>>();
    if matching_keys.len() > 1 {
        return Err(format!(
            "paper broker has multiple position records matching canonical symbol {canonical}; \
             reconcile state before target execution"
        ));
    }
    let Some(key) = matching_keys.into_iter().next() else {
        return Ok(());
    };
    if key == canonical {
        return Ok(());
    }
    let mut position = doc
        .positions
        .remove(&key)
        .ok_or_else(|| "paper position disappeared during symbol normalization".to_string())?;
    position.symbol = canonical.to_string();
    doc.positions.insert(canonical.to_string(), position);
    Ok(())
}

fn bracket_checkpoint_for_symbol<'a>(
    doc: &'a PaperBrokerStateDoc,
    symbol: &str,
) -> Option<&'a PaperBrokerBracketCheckpoint> {
    doc.bracket_checkpoints
        .iter()
        .find_map(|(stored_symbol, checkpoint)| {
            symbols_match(stored_symbol, symbol).then_some(checkpoint)
        })
}

fn bracket_bar_fingerprint(bar: &PaperBrokerBracketAdvanceBar) -> String {
    format!(
        "{:016x}:{:016x}:{:016x}:{:016x}",
        bar.open.to_bits(),
        bar.high.to_bits(),
        bar.low.to_bits(),
        bar.close.to_bits()
    )
}

fn valid_bracket_bar_fingerprint(value: &str) -> bool {
    let parts = value.split(':').collect::<Vec<_>>();
    parts.len() == 4
        && parts
            .iter()
            .all(|part| part.len() == 16 && u64::from_str_radix(part, 16).is_ok())
}

fn is_open_order(order: &PaperBrokerOrder) -> bool {
    matches!(
        order.status.trim().to_ascii_lowercase().as_str(),
        "new"
            | "accepted"
            | "pending_new"
            | "accepted_for_bidding"
            | "partially_filled"
            | "pending_replace"
            | "pending_cancel"
            | "held"
            | "stopped"
            | "suspended"
            | "calculated"
    )
}

fn is_open_bracket_child(order: &PaperBrokerOrder) -> bool {
    is_open_order(order)
        && order.parent_order_id.is_some()
        && matches!(
            order.bracket_leg,
            Some(PaperBracketLeg::StopLoss | PaperBracketLeg::TakeProfit)
        )
}

fn validate_open_bracket_pair(
    doc: &PaperBrokerStateDoc,
    group_id: &str,
    stop_idx: usize,
    take_idx: usize,
) -> Result<(), String> {
    let stop = doc
        .orders
        .get(stop_idx)
        .ok_or_else(|| "paper stop order index out of range".to_string())?;
    let take = doc
        .orders
        .get(take_idx)
        .ok_or_else(|| "paper take-profit order index out of range".to_string())?;
    if stop.parent_order_id.is_none()
        || stop.parent_order_id != take.parent_order_id
        || !symbols_match(&stop.symbol, &take.symbol)
        || stop.side != take.side
        || !quantities_match(stop.qty, take.qty)
    {
        return Err(format!(
            "paper bracket group {group_id} has inconsistent OCO child ownership, symbol, side, \
            or quantity"
        ));
    }
    let parent_id = stop
        .parent_order_id
        .as_deref()
        .expect("checked bracket parent id");
    let parent = doc
        .orders
        .iter()
        .find(|order| order.id == parent_id)
        .ok_or_else(|| {
            format!("paper bracket group {group_id} references missing parent {parent_id}")
        })?;
    if parent.bracket_group_id.as_deref() != Some(group_id)
        || parent.bracket_leg != Some(PaperBracketLeg::Parent)
        || !symbols_match(&parent.symbol, &stop.symbol)
    {
        return Err(format!(
            "paper bracket group {group_id} does not match its durable parent order"
        ));
    }
    Ok(())
}

fn validate_bracket_child_reduces_position(
    doc: &PaperBrokerStateDoc,
    order_idx: usize,
) -> Result<(), String> {
    let order = doc
        .orders
        .get(order_idx)
        .ok_or_else(|| "paper bracket order index out of range".to_string())?;
    let current_qty = position_for_symbol(doc, &order.symbol)?
        .map(|position| position.qty)
        .ok_or_else(|| {
            format!(
                "protective paper order {} has no matching open position; refusing to create \
                 opposite exposure",
                order.id
            )
        })?;
    let signed_child_qty = order.side.signed_qty(order.qty);
    if current_qty.signum() == signed_child_qty.signum() || order.qty > current_qty.abs() + 1e-9 {
        return Err(format!(
            "protective paper order {} qty {} would not strictly reduce current position {}; \
             cancel or resize stale protection before advancing",
            order.id, order.qty, current_qty
        ));
    }
    Ok(())
}

fn bracket_child_hit(order: &PaperBrokerOrder, bar: &PaperBrokerBracketAdvanceBar) -> bool {
    match (order.bracket_leg, order.side) {
        (Some(PaperBracketLeg::StopLoss), PaperOrderSide::Sell) => {
            order.stop_price.is_some_and(|price| bar.low <= price)
        }
        (Some(PaperBracketLeg::StopLoss), PaperOrderSide::Buy) => {
            order.stop_price.is_some_and(|price| bar.high >= price)
        }
        (Some(PaperBracketLeg::TakeProfit), PaperOrderSide::Sell) => {
            order.limit_price.is_some_and(|price| bar.high >= price)
        }
        (Some(PaperBracketLeg::TakeProfit), PaperOrderSide::Buy) => {
            order.limit_price.is_some_and(|price| bar.low <= price)
        }
        _ => false,
    }
}

fn bracket_child_fill_reference_price(
    order: &PaperBrokerOrder,
    bar: &PaperBrokerBracketAdvanceBar,
) -> Option<f64> {
    match (order.bracket_leg, order.side) {
        (Some(PaperBracketLeg::StopLoss), PaperOrderSide::Sell) => {
            order.stop_price.map(|stop| bar.open.min(stop))
        }
        (Some(PaperBracketLeg::StopLoss), PaperOrderSide::Buy) => {
            order.stop_price.map(|stop| bar.open.max(stop))
        }
        (Some(PaperBracketLeg::TakeProfit), _) => order.limit_price,
        _ => None,
    }
}

fn fill_open_order_at_price(
    doc: &mut PaperBrokerStateDoc,
    order_idx: usize,
    price: f64,
    at_unix_ms: i64,
    processing_at_unix_ms: i64,
) -> Result<PaperBrokerOrder, String> {
    let (order_id, symbol, side, qty, bracket_leg, limit_price, fee_bps, spread_bps, slippage_bps) = {
        let order = doc
            .orders
            .get(order_idx)
            .ok_or_else(|| "paper order index out of range".to_string())?;
        (
            order.id.clone(),
            order.symbol.clone(),
            order.side,
            order.qty,
            order.bracket_leg,
            order.limit_price,
            order.fee_bps,
            order.spread_bps,
            order.slippage_bps,
        )
    };
    let symbol = canonical_symbol(&symbol);
    normalize_position_symbol(doc, &symbol)?;
    let (unclamped_fill_price, _, _) = adverse_fill_price(price, side, spread_bps, slippage_bps);
    // A triggered stop becomes a market order and can gap beyond its stop.
    // A take-profit remains a limit order, so modeled adverse costs must not
    // fabricate a fill through its limit price.
    let fill_price = match (bracket_leg, side, limit_price) {
        (Some(PaperBracketLeg::TakeProfit), PaperOrderSide::Sell, Some(limit)) => {
            unclamped_fill_price.max(limit)
        }
        (Some(PaperBracketLeg::TakeProfit), PaperOrderSide::Buy, Some(limit)) => {
            unclamped_fill_price.min(limit)
        }
        _ => unclamped_fill_price,
    };
    let actual_adverse_impact = match side {
        PaperOrderSide::Buy => (fill_price - price).max(0.0),
        PaperOrderSide::Sell => (price - fill_price).max(0.0),
    };
    let requested_spread = price * (spread_bps.max(0.0) * 0.5 / 10_000.0);
    let spread = requested_spread.min(actual_adverse_impact);
    let slippage = (actual_adverse_impact - spread).max(0.0);
    let fee = qty * fill_price * (fee_bps.max(0.0) / 10_000.0);
    let signed_qty = side.signed_qty(qty);
    doc.account.cash -= signed_qty * fill_price;
    doc.account.cash -= fee;
    let realized_delta = apply_position_fill(
        &mut doc.positions,
        &symbol,
        signed_qty,
        fill_price,
        at_unix_ms,
    );
    let realized_pnl_delta = realized_delta - fee;
    doc.account.realized_pnl += realized_pnl_delta;
    let fill = PaperBrokerFill {
        id: next_fill_id(doc),
        order_id,
        symbol: symbol.clone(),
        side,
        qty,
        price: fill_price,
        fee,
        spread,
        slippage,
        realized_pnl_delta,
        at_unix_ms,
        recorded_at_unix_ms: processing_at_unix_ms,
    };
    doc.fills.push(fill);
    let order = doc
        .orders
        .get_mut(order_idx)
        .ok_or_else(|| "paper order index out of range".to_string())?;
    order.status = "filled".to_string();
    order.filled_qty = qty;
    order.avg_fill_price = fill_price;
    order.filled_at_unix_ms = at_unix_ms;
    order.filled_recorded_at_unix_ms = processing_at_unix_ms;
    order.updated_at_unix_ms = at_unix_ms;
    Ok(order.clone())
}

fn cancel_open_order_at_index(
    doc: &mut PaperBrokerStateDoc,
    order_idx: usize,
    at_unix_ms: i64,
) -> Result<PaperBrokerOrder, String> {
    let order = doc
        .orders
        .get_mut(order_idx)
        .ok_or_else(|| "paper order index out of range".to_string())?;
    order.status = "canceled".to_string();
    order.updated_at_unix_ms = at_unix_ms;
    order.canceled_at_unix_ms = Some(at_unix_ms);
    Ok(order.clone())
}

fn apply_position_fill(
    positions: &mut BTreeMap<String, PaperBrokerPosition>,
    symbol: &str,
    signed_fill_qty: f64,
    fill_price: f64,
    at_unix_ms: i64,
) -> f64 {
    let symbol = canonical_symbol(symbol);
    let prior = positions.get(&symbol).cloned();
    let Some(prior_position) = prior else {
        let mut position = PaperBrokerPosition {
            symbol: symbol.clone(),
            qty: signed_fill_qty,
            avg_entry_price: fill_price,
            market_price: fill_price,
            market_value: 0.0,
            unrealized_pnl: 0.0,
            last_fill_unix_ms: at_unix_ms,
        };
        update_position_marks(&mut position, fill_price);
        positions.insert(symbol, position);
        return 0.0;
    };

    let prior_qty = prior_position.qty;
    let new_qty = prior_qty + signed_fill_qty;
    let mut realized = 0.0;
    let same_direction = prior_qty.signum() == signed_fill_qty.signum();
    let mut avg_entry_price = prior_position.avg_entry_price;

    if prior_qty.abs() <= f64::EPSILON || same_direction {
        let total_abs = prior_qty.abs() + signed_fill_qty.abs();
        if total_abs > f64::EPSILON {
            avg_entry_price = ((prior_position.avg_entry_price * prior_qty.abs())
                + (fill_price * signed_fill_qty.abs()))
                / total_abs;
        }
    } else {
        let closed_qty = prior_qty.abs().min(signed_fill_qty.abs());
        realized = if prior_qty > 0.0 {
            (fill_price - prior_position.avg_entry_price) * closed_qty
        } else {
            (prior_position.avg_entry_price - fill_price) * closed_qty
        };
        if new_qty.signum() != prior_qty.signum() && new_qty.abs() > f64::EPSILON {
            avg_entry_price = fill_price;
        }
    }

    if new_qty.abs() <= f64::EPSILON {
        positions.remove(&symbol);
    } else {
        let mut position = PaperBrokerPosition {
            symbol: symbol.clone(),
            qty: new_qty,
            avg_entry_price,
            market_price: fill_price,
            market_value: 0.0,
            unrealized_pnl: 0.0,
            last_fill_unix_ms: at_unix_ms,
        };
        update_position_marks(&mut position, fill_price);
        positions.insert(symbol, position);
    }
    realized
}

fn update_position_marks(position: &mut PaperBrokerPosition, market_price: f64) {
    position.market_price = market_price;
    position.market_value = position.qty * market_price;
    position.unrealized_pnl = if position.qty >= 0.0 {
        (market_price - position.avg_entry_price) * position.qty.abs()
    } else {
        (position.avg_entry_price - market_price) * position.qty.abs()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_local_data_dir(test_name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join("vectorgrid_paper_broker_tests")
            .join(format!("{test_name}_{unique}"))
    }

    fn reconcile_bracket_from_entry(
        dir: &Path,
        bar: PaperBrokerBracketAdvanceBar,
    ) -> PaperBrokerBracketReconcileResult {
        reconcile_brackets_for_bars(
            dir,
            PaperBrokerBracketReconcileRequest {
                symbol: bar.symbol.clone(),
                processing_at_unix_ms: bar.at_unix_ms,
                bars: vec![
                    PaperBrokerBracketAdvanceBar {
                        symbol: bar.symbol.clone(),
                        open: 100.0,
                        high: 100.0,
                        low: 100.0,
                        close: 100.0,
                        at_unix_ms: 1_000,
                    },
                    bar,
                ],
            },
        )
        .expect("reconcile complete bracket history")
    }

    #[test]
    fn market_order_persists_filled_order_and_position() {
        let dir = temp_app_local_data_dir("market_order");
        let exec = submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("paper fill");

        assert_eq!(exec.order.id, "paper_ord_000000000001");
        assert_eq!(exec.order.status, "filled");
        assert_eq!(exec.fill.price, 100.0);
        assert_eq!(exec.position.as_ref().map(|p| p.qty), Some(2.0));

        let doc = load_or_default(&dir, 2_000).expect("reload");
        assert_eq!(doc.orders.len(), 1);
        assert_eq!(doc.fills.len(), 1);
        assert_eq!(doc.positions.get("AAPL").map(|p| p.qty), Some(2.0));
        assert_eq!(doc.account.cash, 99_800.0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn market_orders_canonicalize_symbols_without_splitting_positions() {
        let dir = temp_app_local_data_dir("canonical_market_symbols");
        for symbol in [" aapl ", "AAPL"] {
            submit_market_order(
                &dir,
                PaperBrokerMarketOrder {
                    symbol: symbol.to_string(),
                    side: PaperOrderSide::Buy,
                    qty: 1.0,
                    price: 100.0,
                    at_unix_ms: 1_000,
                    fee_bps: 0.0,
                    spread_bps: 0.0,
                    slippage_bps: 0.0,
                    client_order_id: None,
                    bracket: None,
                    daily_risk_limits: None,
                },
            )
            .expect("canonicalized paper fill");
        }

        let doc = load_or_default(&dir, 2_000).expect("reload");
        assert_eq!(doc.positions.len(), 1);
        assert_eq!(
            doc.positions.get("AAPL").map(|position| position.qty),
            Some(2.0)
        );
        assert!(doc.orders.iter().all(|order| order.symbol == "AAPL"));
        assert!(doc.fills.iter().all(|fill| fill.symbol == "AAPL"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn market_order_applies_and_persists_fee_half_spread_and_slippage() {
        let dir = temp_app_local_data_dir("market_order_costs");
        let exec = submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 2.0,
                spread_bps: 10.0,
                slippage_bps: 5.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("cost-aware paper fill");

        assert!((exec.fill.spread - 0.05).abs() < 1e-12);
        assert!((exec.fill.slippage - 0.05).abs() < 1e-12);
        assert!((exec.fill.price - 100.10).abs() < 1e-12);
        assert!((exec.fill.fee - 0.02002).abs() < 1e-12);
        assert!((exec.fill.realized_pnl_delta + 0.02002).abs() < 1e-12);
        assert_eq!(exec.order.fee_bps, 2.0);
        assert_eq!(exec.order.spread_bps, 10.0);
        assert_eq!(exec.order.slippage_bps, 5.0);

        let doc = load_or_default(&dir, 2_000).expect("reload");
        assert!((doc.fills[0].price - 100.10).abs() < 1e-12);
        assert!((doc.account.realized_pnl + 0.02002).abs() < 1e-12);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn closing_position_records_realized_pnl() {
        let dir = temp_app_local_data_dir("realized_pnl");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("buy");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Sell,
                qty: 1.5,
                price: 110.0,
                at_unix_ms: 2_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("sell");

        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert_eq!(doc.positions.get("AAPL").map(|p| p.qty), Some(0.5));
        assert_eq!(doc.account.realized_pnl, 15.0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reset_reinitializes_account_state() {
        let dir = temp_app_local_data_dir("reset");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("fill");

        let doc = reset(&dir, 2_000).expect("reset");
        assert!(doc.positions.is_empty());
        assert!(doc.orders.is_empty());
        assert_eq!(doc.account.cash, DEFAULT_STARTING_CASH);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrent_market_orders_allocate_unique_sequences() {
        let dir = temp_app_local_data_dir("concurrent_orders");
        let workers = 12usize;
        let mut handles = Vec::with_capacity(workers);
        for idx in 0..workers {
            let dir = dir.clone();
            handles.push(std::thread::spawn(move || {
                submit_market_order(
                    &dir,
                    PaperBrokerMarketOrder {
                        symbol: "AAPL".to_string(),
                        side: PaperOrderSide::Buy,
                        qty: 1.0,
                        price: 100.0 + idx as f64,
                        at_unix_ms: 1_000,
                        fee_bps: 0.0,
                        spread_bps: 0.0,
                        slippage_bps: 0.0,
                        client_order_id: Some(format!("client-{idx}")),
                        bracket: None,
                        daily_risk_limits: None,
                    },
                )
                .expect("paper fill")
                .order
                .id
            }));
        }

        let mut ids = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker should not panic"))
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), workers);

        let doc = load_or_default(&dir, 10_000).expect("reload");
        assert_eq!(doc.orders.len(), workers);
        assert_eq!(doc.fills.len(), workers);
        assert_eq!(doc.next_order_seq, workers as u64 + 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrent_atomic_targets_execute_only_once_from_the_same_expected_position() {
        let dir = temp_app_local_data_dir("concurrent_atomic_targets");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut handles = Vec::new();
        for client in ["first", "second"] {
            let dir = dir.clone();
            let barrier = barrier.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                execute_target_if_position_matches(
                    &dir,
                    PaperBrokerAtomicTargetOrder {
                        symbol: " aApL ".to_string(),
                        expected_position_qty: 0.0,
                        target_position_qty: 1.0,
                        price: 100.0,
                        event_at_unix_ms: 1_000,
                        processing_at_unix_ms: 2_000,
                        fee_bps: 0.0,
                        spread_bps: 0.0,
                        slippage_bps: 0.0,
                        client_order_id: Some(client.to_string()),
                        bracket: None,
                        daily_risk_limits: None,
                        cancel_owned_brackets_for_risk_reduction: false,
                    },
                )
                .expect("atomic target")
            }));
        }

        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker should not panic"))
            .collect::<Vec<_>>();
        assert_eq!(
            results
                .iter()
                .filter(|result| result.status == PaperBrokerAtomicTargetStatus::Executed)
                .count(),
            1
        );
        assert_eq!(
            results
                .iter()
                .filter(|result| result.status == PaperBrokerAtomicTargetStatus::PositionMismatch)
                .count(),
            1
        );
        let mismatch = results
            .iter()
            .find(|result| result.status == PaperBrokerAtomicTargetStatus::PositionMismatch)
            .expect("one stale request");
        assert_eq!(mismatch.observed_position_qty, 1.0);
        assert!(mismatch.execution.is_none());

        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert_eq!(doc.orders.len(), 1);
        assert_eq!(doc.fills.len(), 1);
        assert_eq!(doc.next_order_seq, 2);
        assert_eq!(doc.next_fill_seq, 2);
        assert_eq!(
            doc.positions.get("AAPL").map(|position| position.qty),
            Some(1.0)
        );

        let state_path = paper_broker_state_path(&dir);
        let before_noops = fs::read(&state_path).expect("read state before no-op CAS requests");
        let already = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 1.0,
                target_position_qty: 1.0,
                price: 101.0,
                event_at_unix_ms: 3_000,
                processing_at_unix_ms: 4_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: true,
            },
        )
        .expect("already-at-target result");
        assert_eq!(
            already.status,
            PaperBrokerAtomicTargetStatus::AlreadyAtTarget
        );
        let stale = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 2.0,
                price: 101.0,
                event_at_unix_ms: 3_000,
                processing_at_unix_ms: 4_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("position-mismatch result");
        assert_eq!(
            stale.status,
            PaperBrokerAtomicTargetStatus::PositionMismatch
        );
        assert_eq!(
            fs::read(&state_path).expect("read state after no-op CAS requests"),
            before_noops
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(windows)]
    #[test]
    fn cross_process_atomic_target_worker() {
        let Some(dir) = std::env::var_os("VECTORGRID_PAPER_BROKER_CROSS_PROCESS_DIR") else {
            return;
        };
        let start_path = std::env::var_os("VECTORGRID_PAPER_BROKER_CROSS_PROCESS_START")
            .map(PathBuf::from)
            .expect("cross-process start path");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !start_path.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for cross-process test start"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let result = execute_target_if_position_matches(
            Path::new(&dir),
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 1.0,
                price: 100.0,
                event_at_unix_ms: 1_000,
                processing_at_unix_ms: 2_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: std::env::var("VECTORGRID_PAPER_BROKER_CROSS_PROCESS_CLIENT").ok(),
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("cross-process atomic target");
        assert!(matches!(
            result.status,
            PaperBrokerAtomicTargetStatus::Executed
                | PaperBrokerAtomicTargetStatus::PositionMismatch
        ));
    }

    #[cfg(windows)]
    #[test]
    fn cross_process_atomic_targets_are_serialized_by_named_mutex() {
        let dir = temp_app_local_data_dir("cross_process_atomic_targets");
        let start_path = dir.join("start.marker");
        fs::create_dir_all(&dir).expect("create cross-process fixture");
        let current_exe = std::env::current_exe().expect("current test executable");
        let mut children = ["first", "second"].map(|client| {
            std::process::Command::new(&current_exe)
                .arg("--exact")
                .arg("broker_paper::tests::cross_process_atomic_target_worker")
                .arg("--nocapture")
                .env("VECTORGRID_PAPER_BROKER_CROSS_PROCESS_DIR", &dir)
                .env("VECTORGRID_PAPER_BROKER_CROSS_PROCESS_START", &start_path)
                .env("VECTORGRID_PAPER_BROKER_CROSS_PROCESS_CLIENT", client)
                .spawn()
                .expect("spawn cross-process paper broker worker")
        });
        fs::write(&start_path, b"go").expect("release cross-process workers");
        for child in &mut children {
            assert!(
                child
                    .wait()
                    .expect("wait for paper broker worker")
                    .success(),
                "cross-process paper broker worker failed"
            );
        }

        let doc = load_or_default(&dir, 3_000).expect("load serialized cross-process state");
        assert_eq!(doc.orders.len(), 1);
        assert_eq!(doc.fills.len(), 1);
        assert_eq!(
            doc.positions.get("AAPL").map(|position| position.qty),
            Some(1.0)
        );
        assert_eq!(doc.next_order_seq, 2);
        assert_eq!(doc.next_fill_seq, 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_target_cas_does_not_accept_large_absolute_position_mismatch() {
        let dir = temp_app_local_data_dir("atomic_target_absolute_cas");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1_000_000_000.0,
                price: 1.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("large position");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before stale CAS");

        let result = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 999_999_999.5,
                target_position_qty: 0.0,
                price: 1.0,
                event_at_unix_ms: 2_000,
                processing_at_unix_ms: 2_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("stale CAS result");
        assert_eq!(
            result.status,
            PaperBrokerAtomicTargetStatus::PositionMismatch
        );
        assert_eq!(result.observed_position_qty, 1_000_000_000.0);
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_target_rejects_processing_time_older_than_durable_state() {
        let dir = temp_app_local_data_dir("atomic_target_stale_processing");
        execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 1.0,
                price: 100.0,
                event_at_unix_ms: 1_000,
                processing_at_unix_ms: 2_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("initial target");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before stale processing");
        let error = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 1.0,
                target_position_qty: 2.0,
                price: 101.0,
                event_at_unix_ms: 1_500,
                processing_at_unix_ms: 1_500,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect_err("stale processing time must fail");
        assert!(error.contains("precedes durable state time"));
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_target_rejects_market_event_older_than_position_fill() {
        let dir = temp_app_local_data_dir("atomic_target_stale_event");
        execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 1.0,
                price: 100.0,
                event_at_unix_ms: 2_000,
                processing_at_unix_ms: 3_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("initial target");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before stale event");
        let error = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 1.0,
                target_position_qty: 2.0,
                price: 101.0,
                event_at_unix_ms: 1_500,
                processing_at_unix_ms: 4_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect_err("stale market event must fail");
        assert!(error.contains("precedes the durable last fill"));
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_target_rejects_single_order_reversal_with_oversized_protection() {
        let dir = temp_app_local_data_dir("atomic_target_bracket_reversal");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("long position");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before reversal");
        let error = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 1.0,
                target_position_qty: -2.0,
                price: 100.0,
                event_at_unix_ms: 2_000,
                processing_at_unix_ms: 2_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 105.0,
                    take_profit_limit_price: 90.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect_err("reversal protection must require staged flattening");
        assert!(error.contains("flatten a reversal"));
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_target_records_event_and_processing_days_separately() {
        let dir = temp_app_local_data_dir("atomic_target_processing_day");
        let event_at_unix_ms = MILLIS_PER_UTC_DAY * 20_010 + 1_000;
        let processing_at_unix_ms = event_at_unix_ms + MILLIS_PER_UTC_DAY;
        let result = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 1.0,
                price: 100.0,
                event_at_unix_ms,
                processing_at_unix_ms,
                fee_bps: 10.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("atomic target");
        let execution = result.execution.expect("execution");
        assert_eq!(execution.order.submitted_at_unix_ms, event_at_unix_ms);
        assert_eq!(execution.order.filled_at_unix_ms, event_at_unix_ms);
        assert_eq!(
            execution.order.filled_recorded_at_unix_ms,
            processing_at_unix_ms
        );
        assert_eq!(execution.order.recorded_at_unix_ms, processing_at_unix_ms);
        assert_eq!(execution.fill.at_unix_ms, event_at_unix_ms);
        assert_eq!(execution.fill.recorded_at_unix_ms, processing_at_unix_ms);

        let event_day = daily_risk_snapshot(&dir, event_at_unix_ms).expect("event-day snapshot");
        assert_eq!(event_day.order_count, 0);
        assert_eq!(event_day.realized_pnl, 0.0);
        let processing_day =
            daily_risk_snapshot(&dir, processing_at_unix_ms).expect("processing-day snapshot");
        assert_eq!(processing_day.order_count, 1);
        assert!((processing_day.realized_pnl + 0.1).abs() < 1e-12);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_risk_reducing_target_cancels_owned_brackets_and_exits_under_one_lock() {
        let dir = temp_app_local_data_dir("atomic_cancel_and_exit");
        let entry = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 2.0,
                price: 100.0,
                event_at_unix_ms: 1_000,
                processing_at_unix_ms: 1_100,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("entry");
        assert_eq!(
            entry
                .execution
                .as_ref()
                .map(|execution| execution.child_orders.len()),
            Some(2)
        );

        let exit = execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "aapl".to_string(),
                expected_position_qty: 2.0,
                target_position_qty: 0.0,
                price: 99.0,
                event_at_unix_ms: 2_000,
                processing_at_unix_ms: 2_100,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: true,
            },
        )
        .expect("atomic risk-reducing exit");
        assert_eq!(exit.status, PaperBrokerAtomicTargetStatus::Executed);
        assert_eq!(exit.cancelled_orders.len(), 2);
        assert!(exit.position.is_none());

        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert!(!doc.positions.contains_key("AAPL"));
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_bracket_child(order))
                .count(),
            0
        );
        assert_eq!(doc.fills.len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn concurrent_manual_exit_and_bracket_trigger_produce_exactly_one_exit() {
        let dir = temp_app_local_data_dir("concurrent_exit_vs_bracket");
        execute_target_if_position_matches(
            &dir,
            PaperBrokerAtomicTargetOrder {
                symbol: "AAPL".to_string(),
                expected_position_qty: 0.0,
                target_position_qty: 1.0,
                price: 100.0,
                event_at_unix_ms: 1_000,
                processing_at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
                cancel_owned_brackets_for_risk_reduction: false,
            },
        )
        .expect("protected entry");

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let exit_dir = dir.clone();
        let exit_barrier = barrier.clone();
        let exit_worker = std::thread::spawn(move || {
            exit_barrier.wait();
            execute_target_if_position_matches(
                &exit_dir,
                PaperBrokerAtomicTargetOrder {
                    symbol: "AAPL".to_string(),
                    expected_position_qty: 1.0,
                    target_position_qty: 0.0,
                    price: 94.0,
                    event_at_unix_ms: 2_000,
                    processing_at_unix_ms: 3_000,
                    fee_bps: 0.0,
                    spread_bps: 0.0,
                    slippage_bps: 0.0,
                    client_order_id: None,
                    bracket: None,
                    daily_risk_limits: None,
                    cancel_owned_brackets_for_risk_reduction: true,
                },
            )
            .expect("manual exit outcome")
        });
        let bracket_dir = dir.clone();
        let bracket_worker = std::thread::spawn(move || {
            barrier.wait();
            reconcile_brackets_for_bars(
                &bracket_dir,
                PaperBrokerBracketReconcileRequest {
                    symbol: "AAPL".to_string(),
                    bars: vec![
                        PaperBrokerBracketAdvanceBar {
                            symbol: "AAPL".to_string(),
                            open: 100.0,
                            high: 100.0,
                            low: 100.0,
                            close: 100.0,
                            at_unix_ms: 1_000,
                        },
                        PaperBrokerBracketAdvanceBar {
                            symbol: "AAPL".to_string(),
                            open: 94.0,
                            high: 96.0,
                            low: 93.0,
                            close: 94.0,
                            at_unix_ms: 2_000,
                        },
                    ],
                    processing_at_unix_ms: 3_000,
                },
            )
            .expect("bracket reconciliation outcome")
        });

        let exit = exit_worker.join().expect("manual exit worker");
        let bracket = bracket_worker.join().expect("bracket worker");
        let manual_executed = exit.status == PaperBrokerAtomicTargetStatus::Executed;
        let bracket_executed = bracket.filled_orders.len() == 1;
        assert_ne!(
            manual_executed, bracket_executed,
            "exactly one competing exit path must execute"
        );

        let doc = load_or_default(&dir, 4_000).expect("final durable state");
        assert!(doc.positions.is_empty());
        assert_eq!(doc.fills.len(), 2, "entry plus exactly one exit fill");
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_bracket_child(order))
                .count(),
            0
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn daily_order_cap_is_checked_atomically_against_durable_parent_orders() {
        let dir = temp_app_local_data_dir("daily_order_cap");
        let day = MILLIS_PER_UTC_DAY * 20_000;
        let build_order = |at_unix_ms| PaperBrokerMarketOrder {
            symbol: "AAPL".to_string(),
            side: PaperOrderSide::Buy,
            qty: 1.0,
            price: 100.0,
            at_unix_ms,
            fee_bps: 0.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            client_order_id: None,
            bracket: None,
            daily_risk_limits: Some(PaperBrokerDailyRiskLimits {
                max_daily_loss: Some(1_000.0),
                max_daily_orders: Some(1),
                at_unix_ms,
            }),
        };

        submit_market_order(&dir, build_order(day + 1_000)).expect("first daily order");
        let error = submit_market_order(&dir, build_order(day + 2_000))
            .expect_err("second daily order must be blocked");
        assert!(error.contains("daily order cap reached"));
        assert_eq!(
            daily_risk_snapshot(&dir, day + 3_000)
                .expect("daily snapshot")
                .order_count,
            1
        );
        submit_market_order(&dir, build_order(day + MILLIS_PER_UTC_DAY + 1_000))
            .expect("next UTC day should reset the count");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn daily_realized_loss_cap_replays_the_durable_fill_ledger() {
        let dir = temp_app_local_data_dir("daily_loss_cap");
        let day = MILLIS_PER_UTC_DAY * 20_001;
        let submit = |side, price, at_unix_ms, max_daily_loss| {
            submit_market_order(
                &dir,
                PaperBrokerMarketOrder {
                    symbol: "AAPL".to_string(),
                    side,
                    qty: 1.0,
                    price,
                    at_unix_ms,
                    fee_bps: 0.0,
                    spread_bps: 0.0,
                    slippage_bps: 0.0,
                    client_order_id: None,
                    bracket: None,
                    daily_risk_limits: Some(PaperBrokerDailyRiskLimits {
                        max_daily_loss: Some(max_daily_loss),
                        max_daily_orders: Some(20),
                        at_unix_ms,
                    }),
                },
            )
        };

        submit(PaperOrderSide::Buy, 100.0, day + 1_000, 1_000.0).expect("entry");
        submit(PaperOrderSide::Sell, 90.0, day + 2_000, 1_000.0).expect("loss exit");
        let snapshot = daily_risk_snapshot(&dir, day + 3_000).expect("daily snapshot");
        assert!((snapshot.realized_pnl + 10.0).abs() < 1e-12);

        let error = submit(PaperOrderSide::Buy, 90.0, day + 4_000, 5.0)
            .expect_err("daily loss cap must block a new entry");
        assert!(error.contains("daily realized loss cap reached"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_recovers_from_corrupt_state_using_backup() {
        let dir = temp_app_local_data_dir("corrupt_recovery");
        let mut doc = PaperBrokerStateDoc::new(1_000);
        doc.next_order_seq = 42;
        save(&dir, &doc).expect("initial save");
        doc.next_order_seq = 43;
        save(&dir, &doc).expect("second save should create backup");

        let path = paper_broker_state_path(&dir);
        let backup_path = paper_broker_state_backup_path(&path);
        assert!(backup_path.is_file(), "backup should exist");
        assert_eq!(
            fs::read(&path).expect("current primary"),
            fs::read(&backup_path).expect("current backup"),
            "successful commits keep the recovery copy current"
        );
        fs::write(&path, b"{\"version\":").expect("corrupt primary state");

        let recovered = load_or_default(&dir, 3_000).expect("recover from backup");
        assert_eq!(recovered.next_order_seq, 43);
        let restored_bytes = fs::read(&path).expect("read restored primary");
        decode_state_doc(&path, &restored_bytes).expect("restored primary should decode");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_recovers_from_semantically_corrupt_sequence_state_using_backup() {
        let dir = temp_app_local_data_dir("semantic_corrupt_recovery");
        let order = |at_unix_ms| PaperBrokerMarketOrder {
            symbol: "AAPL".to_string(),
            side: PaperOrderSide::Buy,
            qty: 1.0,
            price: 100.0,
            at_unix_ms,
            fee_bps: 0.0,
            spread_bps: 0.0,
            slippage_bps: 0.0,
            client_order_id: None,
            bracket: None,
            daily_risk_limits: None,
        };
        submit_market_order(&dir, order(1_000)).expect("first order");
        submit_market_order(&dir, order(2_000)).expect("second order creates backup");

        let path = paper_broker_state_path(&dir);
        let mut corrupt: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("read primary")).expect("decode JSON");
        corrupt["next_order_seq"] = serde_json::json!(1);
        fs::write(
            &path,
            serde_json::to_vec_pretty(&corrupt).expect("encode corrupt state"),
        )
        .expect("write semantic corruption");

        let recovered = load_or_default(&dir, 3_000).expect("recover valid backup");
        assert_eq!(recovered.orders.len(), 2);
        assert_eq!(recovered.next_order_seq, 3);
        let restored =
            decode_state_doc(&path, &fs::read(&path).expect("restored primary")).expect("valid");
        assert_eq!(restored.next_order_seq, 3);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_recovers_backup_instead_of_resetting_when_primary_is_missing() {
        let dir = temp_app_local_data_dir("missing_primary_recovery");
        let mut doc = PaperBrokerStateDoc::new(1_000);
        doc.next_order_seq = 42;
        save(&dir, &doc).expect("initial save");
        doc.next_order_seq = 43;
        save(&dir, &doc).expect("second save creates backup");

        let path = paper_broker_state_path(&dir);
        let backup_path = paper_broker_state_backup_path(&path);
        assert!(backup_path.is_file());
        fs::remove_file(&path).expect("simulate missing primary");

        let recovered = load_or_default(&dir, 3_000).expect("recover missing primary");
        assert_eq!(recovered.next_order_seq, 43);
        assert!(path.is_file(), "primary should be restored atomically");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_fails_closed_when_primary_and_backup_are_both_corrupt() {
        let dir = temp_app_local_data_dir("dual_corrupt_recovery");
        let mut doc = PaperBrokerStateDoc::new(1_000);
        doc.next_order_seq = 42;
        save(&dir, &doc).expect("initial save");
        doc.next_order_seq = 43;
        save(&dir, &doc).expect("second save should create backup");

        let path = paper_broker_state_path(&dir);
        let backup_path = paper_broker_state_backup_path(&path);
        let corrupt_primary = b"{\"version\":";
        let corrupt_backup = b"{\"account\":";
        fs::write(&path, corrupt_primary).expect("corrupt primary state");
        fs::write(&backup_path, corrupt_backup).expect("corrupt backup state");

        let error =
            load_or_default(&dir, 3_000).expect_err("dual corruption must not create defaults");
        assert!(error.contains(&path.to_string_lossy().to_string()));
        assert!(error.contains(&backup_path.to_string_lossy().to_string()));
        assert!(error.contains("backup recovery failed"));
        assert!(error.contains("was not reset or modified"));
        assert!(error.contains("explicit paper-broker reset"));
        assert_eq!(
            fs::read(&path).expect("read unchanged primary"),
            corrupt_primary
        );
        assert_eq!(
            fs::read(&backup_path).expect("read unchanged backup"),
            corrupt_backup
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn market_order_with_bracket_creates_linked_child_orders() {
        let dir = temp_app_local_data_dir("bracket_children");
        let exec = submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("paper bracket fill");

        assert_eq!(exec.child_orders.len(), 2);
        assert_eq!(
            exec.child_orders[0].parent_order_id.as_deref(),
            Some(exec.order.id.as_str())
        );
        assert_eq!(exec.child_orders[0].status, "new");
        assert_eq!(exec.child_orders[1].status, "new");

        let doc = load_or_default(&dir, 2_000).expect("reload");
        assert_eq!(doc.orders.len(), 3);
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_order(order))
                .count(),
            2
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn single_latest_bar_api_fails_closed_when_initial_history_would_be_skipped() {
        let dir = temp_app_local_data_dir("single_latest_bar_fail_closed");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("bracket entry");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before unsafe latest-bar call");

        let error = advance_brackets_for_bar(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 94.0,
                high: 96.0,
                low: 93.0,
                close: 94.0,
                at_unix_ms: 3_000,
            },
        )
        .expect_err("latest-only call must not skip unseen bars");
        assert!(error.contains("no durable checkpoint"));
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_stop_fill_cancels_take_profit_sibling() {
        let dir = temp_app_local_data_dir("bracket_stop");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("paper bracket fill");

        let advanced = reconcile_bracket_from_entry(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 99.0,
                high: 101.0,
                low: 94.0,
                close: 96.0,
                at_unix_ms: 2_000,
            },
        );

        assert_eq!(advanced.filled_orders.len(), 1);
        assert_eq!(
            advanced.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::StopLoss)
        );
        assert_eq!(advanced.cancelled_orders.len(), 1);
        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert!(!doc.positions.contains_key("AAPL"));
        assert_eq!(doc.account.realized_pnl, -10.0);
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_order(order))
                .count(),
            0
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_reconciliation_fails_closed_then_replays_unseen_bars_after_restart() {
        let dir = temp_app_local_data_dir("bracket_restart_replay");
        let event_day = MILLIS_PER_UTC_DAY * 20_011;
        let entry_at = event_day + 1_000;
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: entry_at,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("bracket entry");

        let state_path = paper_broker_state_path(&dir);
        let before_missing_initial_history =
            fs::read(&state_path).expect("read state before failed first reconciliation");
        let first_error = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![PaperBrokerBracketAdvanceBar {
                    symbol: "AAPL".to_string(),
                    open: 100.0,
                    high: 101.0,
                    low: 99.0,
                    close: 100.0,
                    at_unix_ms: entry_at + 1_000,
                }],
                processing_at_unix_ms: entry_at + 2_000,
            },
        )
        .expect_err("first reconciliation cannot begin after bracket submission");
        assert!(first_error.contains("no durable checkpoint"));
        assert!(first_error.contains("replay complete ordered bars"));
        assert_eq!(
            fs::read(&state_path).expect("read unchanged state"),
            before_missing_initial_history
        );

        let first = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "aapl".to_string(),
                bars: vec![
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 100.0,
                        high: 100.0,
                        low: 100.0,
                        close: 100.0,
                        at_unix_ms: entry_at,
                    },
                    PaperBrokerBracketAdvanceBar {
                        symbol: "aapl".to_string(),
                        open: 100.0,
                        high: 102.0,
                        low: 98.0,
                        close: 101.0,
                        at_unix_ms: entry_at + 1_000,
                    },
                ],
                processing_at_unix_ms: entry_at + 2_000,
            },
        )
        .expect("initial complete reconciliation");
        assert_eq!(first.processed_bar_count, 2);
        assert!(first.filled_orders.is_empty());
        assert_eq!(first.last_evaluated_bar_unix_ms, entry_at + 1_000);

        // A fresh load models process restart: the checkpoint must survive, and
        // offering only the latest bar must not skip the unseen interval.
        let restarted = load_or_default(&dir, entry_at + 3_000).expect("restart load");
        assert_eq!(
            restarted
                .bracket_checkpoints
                .get("AAPL")
                .map(|checkpoint| checkpoint.last_evaluated_bar_unix_ms),
            Some(entry_at + 1_000)
        );
        let before_missing_anchor =
            fs::read(&state_path).expect("read state before missing-anchor attempt");
        let restart_error = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![PaperBrokerBracketAdvanceBar {
                    symbol: "AAPL".to_string(),
                    open: 94.0,
                    high: 96.0,
                    low: 93.0,
                    close: 94.0,
                    at_unix_ms: entry_at + 3_000,
                }],
                processing_at_unix_ms: event_day + MILLIS_PER_UTC_DAY + 5_000,
            },
        )
        .expect_err("restart cannot skip the durable anchor");
        assert!(restart_error.contains("requires checkpoint anchor"));
        assert!(restart_error.contains("complete ordered bar history"));
        assert_eq!(
            fs::read(&state_path).expect("read unchanged state"),
            before_missing_anchor
        );

        let processing_at_unix_ms = event_day + MILLIS_PER_UTC_DAY + 5_000;
        let replayed = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 100.0,
                        high: 102.0,
                        low: 98.0,
                        close: 101.0,
                        at_unix_ms: entry_at + 1_000,
                    },
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 94.0,
                        high: 96.0,
                        low: 93.0,
                        close: 94.0,
                        at_unix_ms: entry_at + 2_000,
                    },
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 94.0,
                        high: 95.0,
                        low: 92.0,
                        close: 93.0,
                        at_unix_ms: entry_at + 3_000,
                    },
                ],
                processing_at_unix_ms,
            },
        )
        .expect("complete restart replay");
        assert_eq!(replayed.processed_bar_count, 2);
        assert_eq!(replayed.filled_orders.len(), 1);
        assert_eq!(
            replayed.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::StopLoss)
        );
        assert_eq!(
            replayed.filled_orders[0].filled_at_unix_ms,
            entry_at + 2_000
        );
        assert_eq!(
            replayed.filled_orders[0].filled_recorded_at_unix_ms,
            processing_at_unix_ms
        );
        assert_eq!(replayed.cancelled_orders.len(), 1);
        assert!(replayed.position.is_none());

        let final_doc = load_or_default(&dir, processing_at_unix_ms).expect("final restart load");
        assert_eq!(
            final_doc
                .bracket_checkpoints
                .get("AAPL")
                .map(|checkpoint| checkpoint.last_evaluated_bar_unix_ms),
            Some(entry_at + 3_000)
        );
        let exit_fill = final_doc.fills.last().expect("replayed exit fill");
        assert_eq!(exit_fill.at_unix_ms, entry_at + 2_000);
        assert_eq!(exit_fill.recorded_at_unix_ms, processing_at_unix_ms);
        assert_eq!(
            daily_risk_snapshot(&dir, entry_at + 2_000)
                .expect("market-event-day snapshot")
                .realized_pnl,
            0.0
        );
        assert_eq!(
            daily_risk_snapshot(&dir, processing_at_unix_ms)
                .expect("processing-day snapshot")
                .realized_pnl,
            -6.0
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_reconciliation_rejects_revised_checkpoint_anchor_content() {
        let dir = temp_app_local_data_dir("bracket_anchor_content");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("bracket entry");
        reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 100.0,
                        high: 100.0,
                        low: 100.0,
                        close: 100.0,
                        at_unix_ms: 1_000,
                    },
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 100.0,
                        high: 102.0,
                        low: 98.0,
                        close: 101.0,
                        at_unix_ms: 2_000,
                    },
                ],
                processing_at_unix_ms: 2_000,
            },
        )
        .expect("initial reconciliation");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before revised anchor");
        let error = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 100.0,
                        high: 103.0,
                        low: 98.0,
                        close: 101.0,
                        at_unix_ms: 2_000,
                    },
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 101.0,
                        high: 102.0,
                        low: 100.0,
                        close: 101.0,
                        at_unix_ms: 3_000,
                    },
                ],
                processing_at_unix_ms: 3_000,
            },
        )
        .expect_err("revised anchor must fail closed");
        assert!(error.contains("different OHLC content"));
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_reconciliation_rejects_malformed_ohlc_without_mutation() {
        let dir = temp_app_local_data_dir("malformed_ohlc");
        let before = PaperBrokerStateDoc::new(1_000);
        save(&dir, &before).expect("seed state");
        let state_path = paper_broker_state_path(&dir);
        let bytes_before = fs::read(&state_path).expect("state before invalid bar");
        let error = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![PaperBrokerBracketAdvanceBar {
                    symbol: "AAPL".to_string(),
                    open: 105.0,
                    high: 104.0,
                    low: 99.0,
                    close: 100.0,
                    at_unix_ms: 2_000,
                }],
                processing_at_unix_ms: 2_000,
            },
        )
        .expect_err("open above high must fail");
        assert!(error.contains("open and close must lie within"));
        assert_eq!(
            fs::read(&state_path).expect("unchanged state"),
            bytes_before
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stale_protective_child_cannot_reverse_a_partially_reduced_position() {
        let dir = temp_app_local_data_dir("stale_bracket_no_reversal");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("bracket entry");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Sell,
                qty: 1.0,
                price: 99.0,
                at_unix_ms: 1_500,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: None,
                daily_risk_limits: None,
            },
        )
        .expect("manual partial reduction");
        let state_path = paper_broker_state_path(&dir);
        let before = fs::read(&state_path).expect("state before stale protection");
        let error = reconcile_brackets_for_bars(
            &dir,
            PaperBrokerBracketReconcileRequest {
                symbol: "AAPL".to_string(),
                bars: vec![
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 100.0,
                        high: 100.0,
                        low: 100.0,
                        close: 100.0,
                        at_unix_ms: 1_000,
                    },
                    PaperBrokerBracketAdvanceBar {
                        symbol: "AAPL".to_string(),
                        open: 94.0,
                        high: 96.0,
                        low: 93.0,
                        close: 94.0,
                        at_unix_ms: 2_000,
                    },
                ],
                processing_at_unix_ms: 2_000,
            },
        )
        .expect_err("stale bracket must not reverse the position");
        assert!(error.contains("would not strictly reduce current position"));
        assert_eq!(fs::read(&state_path).expect("unchanged state"), before);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_same_bar_stop_and_take_uses_stop_loss_first() {
        let dir = temp_app_local_data_dir("bracket_tie_break");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("paper bracket fill");

        let advanced = reconcile_bracket_from_entry(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 100.0,
                high: 111.0,
                low: 94.0,
                close: 101.0,
                at_unix_ms: 2_000,
            },
        );

        assert_eq!(
            advanced.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::StopLoss)
        );
        assert_eq!(advanced.tie_break_events.len(), 1);
        assert!(advanced.tie_break_events[0].contains("stop_loss_first"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_stop_gap_uses_gap_open_before_adverse_costs() {
        let dir = temp_app_local_data_dir("bracket_stop_gap");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 20.0,
                slippage_bps: 10.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("paper bracket fill");

        let advanced = reconcile_bracket_from_entry(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 90.0,
                high: 94.0,
                low: 89.0,
                close: 92.0,
                at_unix_ms: 2_000,
            },
        );

        assert_eq!(advanced.filled_orders.len(), 1);
        assert_eq!(
            advanced.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::StopLoss)
        );
        assert!((advanced.filled_orders[0].avg_fill_price - 89.82).abs() < 1e-9);
        let doc = load_or_default(&dir, 3_000).expect("reload");
        let exit_fill = doc.fills.last().expect("exit fill");
        assert!((exit_fill.spread - 0.09).abs() < 1e-9);
        assert!((exit_fill.slippage - 0.09).abs() < 1e-9);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_take_profit_costs_never_cross_limit_price() {
        let dir = temp_app_local_data_dir("bracket_take_limit");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 20.0,
                slippage_bps: 10.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("paper bracket fill");

        let advanced = reconcile_bracket_from_entry(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 109.0,
                high: 111.0,
                low: 108.0,
                close: 110.0,
                at_unix_ms: 2_000,
            },
        );

        assert_eq!(advanced.filled_orders.len(), 1);
        assert_eq!(
            advanced.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::TakeProfit)
        );
        assert_eq!(advanced.filled_orders[0].avg_fill_price, 110.0);
        let doc = load_or_default(&dir, 3_000).expect("reload");
        let exit_fill = doc.fills.last().expect("exit fill");
        assert_eq!(exit_fill.spread, 0.0);
        assert_eq!(exit_fill.slippage, 0.0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cancel_open_orders_for_symbol_cancels_child_orders_only() {
        let dir = temp_app_local_data_dir("cancel_children");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                spread_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
                daily_risk_limits: None,
            },
        )
        .expect("paper bracket fill");

        let cancelled = cancel_open_orders_for_symbol(&dir, "AAPL", 2_000).expect("cancel");
        assert_eq!(cancelled.open_order_count_before, 2);
        assert_eq!(cancelled.cancelled_orders.len(), 2);
        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_order(order))
                .count(),
            0
        );
        assert_eq!(doc.positions.get("AAPL").map(|p| p.qty), Some(1.0));
        let _ = fs::remove_dir_all(dir);
    }
}
