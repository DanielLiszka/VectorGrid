use std::fs;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::mem::{self, MaybeUninit};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

const MAGIC: &[u8; 4] = b"VTA1";
const VERSION: u32 = 2;
const HEADER_LEN: usize = 64;
const FNV1A64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;
const DEFAULT_MAX_SOURCE_CSV_BYTES: u64 = 512 * 1024 * 1024;
const DEFAULT_MAX_SOURCE_CSV_ROWS: u64 = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceIdentity {
    pub size: u64,
    pub mtime_utc_ms: i64,
    pub change_marker: i64,
}

#[derive(Debug, Clone)]
pub struct LoadOutcome {
    pub candles: Candles,
    pub cache_hit: bool,
    pub cache_path: Option<PathBuf>,
    pub source_identity: SourceIdentity,
    pub source_content_fingerprint: String,
}

#[derive(Debug, Clone, Copy)]
struct CsvLoadLimits {
    max_source_bytes: u64,
    max_rows: u64,
}

impl CsvLoadLimits {
    fn from_env() -> Self {
        Self {
            max_source_bytes: positive_env_u64(
                "VECTORBT_MAX_SOURCE_CSV_BYTES",
                DEFAULT_MAX_SOURCE_CSV_BYTES,
            ),
            max_rows: positive_env_u64("VECTORBT_MAX_SOURCE_CSV_ROWS", DEFAULT_MAX_SOURCE_CSV_ROWS),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceContentInspection {
    hash: u64,
    data_rows: u64,
}

fn positive_env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub fn app_data_cache_dir(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join("data_cache")
}

pub fn identity_for_source(path: &Path) -> Result<SourceIdentity, String> {
    let md = fs::metadata(path).map_err(|e| format!("failed to read CSV source metadata: {e}"))?;
    if !md.is_file() {
        return Err("CSV source path is not a regular file".to_string());
    }
    let size = md.len();

    let mtime = md.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let ms = mtime
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    Ok(SourceIdentity {
        size,
        mtime_utc_ms: ms,
        change_marker: source_change_marker(path, &md)?,
    })
}

#[cfg(windows)]
fn source_change_marker(path: &Path, _metadata: &fs::Metadata) -> Result<i64, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileBasicInfo, GetFileInformationByHandleEx, FILE_BASIC_INFO,
    };

    // LastWriteTime can be restored after a same-size rewrite. NTFS ChangeTime records
    // the file-record mutation independently, giving run-start checks a strong O(1) marker.
    let file = fs::File::open(path)
        .map_err(|error| format!("failed to open CSV source for identity check: {error}"))?;
    let mut info = std::mem::MaybeUninit::<FILE_BASIC_INFO>::uninit();
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle().cast(),
            FileBasicInfo,
            info.as_mut_ptr().cast(),
            std::mem::size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(format!(
            "failed to inspect CSV source change time: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(unsafe { info.assume_init() }.ChangeTime)
}

#[cfg(unix)]
fn source_change_marker(_path: &Path, metadata: &fs::Metadata) -> Result<i64, String> {
    use std::os::unix::fs::MetadataExt;

    // ctime changes for content or inode metadata mutations and cannot be reset with mtime.
    Ok(metadata
        .ctime()
        .wrapping_mul(1_000_000_000)
        .wrapping_add(metadata.ctime_nsec()))
}

#[cfg(not(any(windows, unix)))]
fn source_change_marker(_path: &Path, _metadata: &fs::Metadata) -> Result<i64, String> {
    Err("CSV source identity checks are unsupported on this platform".to_string())
}

fn fnv1a64_update(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV1A64_PRIME);
    }
    h
}

fn inspect_file_contents(
    file: &mut fs::File,
    mut h: u64,
) -> Result<SourceContentInspection, String> {
    let mut buf = [0u8; 64 * 1024];
    let mut physical_lines = 0u64;
    let mut saw_bytes = false;
    let mut ended_with_newline = false;
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        saw_bytes = true;
        physical_lines = physical_lines
            .checked_add(buf[..n].iter().filter(|&&byte| byte == b'\n').count() as u64)
            .ok_or_else(|| "CSV physical line count overflow".to_string())?;
        ended_with_newline = buf[n - 1] == b'\n';
        h = fnv1a64_update(h, &buf[..n]);
    }
    if saw_bytes && !ended_with_newline {
        physical_lines = physical_lines
            .checked_add(1)
            .ok_or_else(|| "CSV physical line count overflow".to_string())?;
    }
    Ok(SourceContentInspection {
        hash: h,
        data_rows: physical_lines.saturating_sub(1),
    })
}

fn inspect_source_contents(
    path: &Path,
    source_identity: SourceIdentity,
) -> Result<SourceContentInspection, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("failed to read CSV source contents: {e}"))?;

    let mut h = FNV1A64_OFFSET_BASIS;
    h = fnv1a64_update(h, &source_identity.size.to_le_bytes());
    inspect_file_contents(&mut file, h)
}

fn inspect_source_with_limits(
    path: &Path,
    expected_identity: SourceIdentity,
    limits: CsvLoadLimits,
) -> Result<SourceContentInspection, String> {
    let current_identity = identity_for_source(path)?;
    if current_identity != expected_identity {
        return Err("CSV source changed while it was being loaded; retry the load".to_string());
    }
    if current_identity.size > limits.max_source_bytes {
        return Err(format!(
            "CSV source is {} bytes, exceeding the bounded load limit of {} bytes; split or resample the file, or raise VECTORBT_MAX_SOURCE_CSV_BYTES explicitly",
            current_identity.size, limits.max_source_bytes
        ));
    }
    let inspection = inspect_source_contents(path, current_identity)?;
    if inspection.data_rows > limits.max_rows {
        return Err(format!(
            "CSV source contains more than {} data rows; split or resample the file, or raise VECTORBT_MAX_SOURCE_CSV_ROWS explicitly",
            limits.max_rows
        ));
    }
    Ok(inspection)
}

fn ensure_source_identity_unchanged(
    path: &Path,
    expected_identity: SourceIdentity,
) -> Result<(), String> {
    if identity_for_source(path)? != expected_identity {
        return Err("CSV source changed while it was being loaded; retry the load".to_string());
    }
    Ok(())
}

fn content_fingerprint_parts_for_source(path: &Path) -> Result<(&'static str, u64), String> {
    let source_identity = identity_for_source(path)?;
    let inspection = inspect_source_contents(path, source_identity)?;
    Ok(("full", inspection.hash))
}

#[cfg(test)]
fn content_fingerprint_hash_for_source(path: &Path) -> Result<u64, String> {
    let (_, hash) = content_fingerprint_parts_for_source(path)?;
    Ok(hash)
}

pub fn content_fingerprint_for_source(path: &Path) -> Result<String, String> {
    let (mode, hash) = content_fingerprint_parts_for_source(path)?;
    Ok(format!("fnv1a64-{mode}-{hash:016x}"))
}

fn format_content_fingerprint(hash: u64) -> String {
    format!("fnv1a64-full-{hash:016x}")
}

fn sidecar_cache_path(csv_path: &Path) -> PathBuf {
    csv_path.with_extension("vta")
}

fn hash_key(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn fallback_cache_path(app_cache_dir: &Path, csv_path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(csv_path).unwrap_or_else(|_| csv_path.to_path_buf());
    let key = canonical.to_string_lossy();
    let stem = csv_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("candles");
    let safe_stem = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();

    let hash = hash_key(&key);
    app_cache_dir.join(format!("{safe_stem}__{hash:016x}.vta"))
}

fn flags_to_mask(fields: CandleFieldFlags) -> u32 {
    (fields.open as u32)
        | ((fields.high as u32) << 1)
        | ((fields.low as u32) << 2)
        | ((fields.close as u32) << 3)
        | ((fields.volume as u32) << 4)
}

fn mask_to_flags(mask: u32) -> CandleFieldFlags {
    CandleFieldFlags {
        open: (mask & (1 << 0)) != 0,
        high: (mask & (1 << 1)) != 0,
        low: (mask & (1 << 2)) != 0,
        close: (mask & (1 << 3)) != 0,
        volume: (mask & (1 << 4)) != 0,
    }
}

fn checked_cache_payload_bytes(len: usize) -> Option<usize> {
    len.checked_mul(8)
}

fn checked_expected_cache_size(len_u64: u64) -> Option<u64> {
    let len = usize::try_from(len_u64).ok()?;
    let payload = u64::try_from(checked_cache_payload_bytes(len)?).ok()?;
    let total_payload = payload.checked_mul(6)?;
    (HEADER_LEN as u64).checked_add(total_payload)
}

fn write_header(
    out: &mut [u8; HEADER_LEN],
    fields_mask: u32,
    len: u64,
    csv_size: u64,
    csv_mtime_ms: i64,
    min_ts: i64,
    max_ts: i64,
    source_content_hash: u64,
) {
    out.fill(0);
    out[0..4].copy_from_slice(MAGIC);
    out[4..8].copy_from_slice(&VERSION.to_le_bytes());
    out[8..12].copy_from_slice(&fields_mask.to_le_bytes());
    out[16..24].copy_from_slice(&len.to_le_bytes());
    out[24..32].copy_from_slice(&csv_size.to_le_bytes());
    out[32..40].copy_from_slice(&csv_mtime_ms.to_le_bytes());
    out[40..48].copy_from_slice(&min_ts.to_le_bytes());
    out[48..56].copy_from_slice(&max_ts.to_le_bytes());
    out[56..64].copy_from_slice(&source_content_hash.to_le_bytes());
}

fn read_header(input: &[u8; HEADER_LEN]) -> Option<(u32, u64, u64, i64, u64)> {
    if &input[0..4] != MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(input[4..8].try_into().ok()?);
    if version != VERSION {
        return None;
    }
    let fields_mask = u32::from_le_bytes(input[8..12].try_into().ok()?);
    let len = u64::from_le_bytes(input[16..24].try_into().ok()?);
    let csv_size = u64::from_le_bytes(input[24..32].try_into().ok()?);
    let csv_mtime_ms = i64::from_le_bytes(input[32..40].try_into().ok()?);
    let source_content_hash = u64::from_le_bytes(input[56..64].try_into().ok()?);
    Some((
        fields_mask,
        len,
        csv_size,
        csv_mtime_ms,
        source_content_hash,
    ))
}

fn write_le_slice_i64(w: &mut dyn Write, v: &[i64]) -> Result<(), String> {
    #[cfg(target_endian = "little")]
    {
        let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) };
        w.write_all(bytes).map_err(|e| e.to_string())
    }
    #[cfg(target_endian = "big")]
    {
        for &x in v {
            w.write_all(&x.to_le_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn write_le_slice_f64(w: &mut dyn Write, v: &[f64]) -> Result<(), String> {
    #[cfg(target_endian = "little")]
    {
        let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) };
        w.write_all(bytes).map_err(|e| e.to_string())
    }
    #[cfg(target_endian = "big")]
    {
        for &x in v {
            w.write_all(&x.to_bits().to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn read_le_vec_i64(r: &mut dyn Read, len: usize) -> Result<Vec<i64>, String> {
    let byte_len = checked_cache_payload_bytes(len)
        .ok_or_else(|| "cache payload byte length overflow".to_string())?;
    let mut out_uninit: Vec<MaybeUninit<i64>> = Vec::with_capacity(len);
    unsafe {
        out_uninit.set_len(len);
    }

    let buf =
        unsafe { std::slice::from_raw_parts_mut(out_uninit.as_mut_ptr() as *mut u8, byte_len) };
    r.read_exact(buf).map_err(|e| e.to_string())?;
    let ptr = out_uninit.as_mut_ptr() as *mut i64;
    let cap = out_uninit.capacity();
    mem::forget(out_uninit);

    #[cfg(target_endian = "little")]
    let out = unsafe { Vec::from_raw_parts(ptr, len, cap) };
    #[cfg(target_endian = "big")]
    let mut out = unsafe { Vec::from_raw_parts(ptr, len, cap) };

    #[cfg(target_endian = "big")]
    {
        for x in &mut out {
            *x = i64::from_le(*x);
        }
    }
    Ok(out)
}

fn read_le_vec_f64(r: &mut dyn Read, len: usize) -> Result<Vec<f64>, String> {
    let byte_len = checked_cache_payload_bytes(len)
        .ok_or_else(|| "cache payload byte length overflow".to_string())?;
    let mut out_uninit: Vec<MaybeUninit<f64>> = Vec::with_capacity(len);
    unsafe {
        out_uninit.set_len(len);
    }

    let buf =
        unsafe { std::slice::from_raw_parts_mut(out_uninit.as_mut_ptr() as *mut u8, byte_len) };
    r.read_exact(buf).map_err(|e| e.to_string())?;
    let ptr = out_uninit.as_mut_ptr() as *mut f64;
    let cap = out_uninit.capacity();
    mem::forget(out_uninit);

    #[cfg(target_endian = "little")]
    let out = unsafe { Vec::from_raw_parts(ptr, len, cap) };
    #[cfg(target_endian = "big")]
    let mut out = unsafe { Vec::from_raw_parts(ptr, len, cap) };

    #[cfg(target_endian = "big")]
    {
        for x in &mut out {
            let bits = x.to_bits();
            *x = f64::from_bits(u64::from_le(bits));
        }
    }
    Ok(out)
}

pub fn load_or_build(csv_path: &Path, app_cache_dir: &Path) -> Result<LoadOutcome, String> {
    let src_id = identity_for_source(csv_path)?;
    load_or_build_with_identity(csv_path, app_cache_dir, src_id)
}

pub fn load_or_build_with_identity(
    csv_path: &Path,
    app_cache_dir: &Path,
    src_id: SourceIdentity,
) -> Result<LoadOutcome, String> {
    load_or_build_with_identity_and_limits(
        csv_path,
        app_cache_dir,
        src_id,
        CsvLoadLimits::from_env(),
    )
}

fn load_or_build_with_identity_and_limits(
    csv_path: &Path,
    app_cache_dir: &Path,
    src_id: SourceIdentity,
    limits: CsvLoadLimits,
) -> Result<LoadOutcome, String> {
    let sidecar = sidecar_cache_path(csv_path);
    let fallback = fallback_cache_path(app_cache_dir, csv_path);
    let source_inspection = inspect_source_with_limits(csv_path, src_id, limits)?;

    if let Ok(Some(candles)) =
        try_load_cache(&sidecar, src_id, source_inspection.hash, limits.max_rows)
    {
        ensure_source_identity_unchanged(csv_path, src_id)?;
        return Ok(LoadOutcome {
            candles,
            cache_hit: true,
            cache_path: Some(sidecar),
            source_identity: src_id,
            source_content_fingerprint: format_content_fingerprint(source_inspection.hash),
        });
    }

    if sidecar != fallback {
        if let Ok(Some(candles)) =
            try_load_cache(&fallback, src_id, source_inspection.hash, limits.max_rows)
        {
            ensure_source_identity_unchanged(csv_path, src_id)?;
            return Ok(LoadOutcome {
                candles,
                cache_hit: true,
                cache_path: Some(fallback),
                source_identity: src_id,
                source_content_fingerprint: format_content_fingerprint(source_inspection.hash),
            });
        }
    }

    let candles = ta_app_core::read_candles_from_csv_compat(csv_path.to_string_lossy().as_ref())?;
    if u64::try_from(candles.timestamp.len()).unwrap_or(u64::MAX) > limits.max_rows {
        return Err(format!(
            "CSV source contains more than {} parsed data rows; split or resample the file, or raise VECTORBT_MAX_SOURCE_CSV_ROWS explicitly",
            limits.max_rows
        ));
    }

    let mut wrote_path = None;
    ensure_source_identity_unchanged(csv_path, src_id)?;
    if let Ok(()) = write_cache_with_source_hash(&sidecar, &candles, src_id, source_inspection.hash)
    {
        wrote_path = Some(sidecar);
    } else if sidecar != fallback {
        if let Ok(()) =
            write_cache_with_source_hash(&fallback, &candles, src_id, source_inspection.hash)
        {
            wrote_path = Some(fallback);
        }
    }

    Ok(LoadOutcome {
        candles,
        cache_hit: false,
        cache_path: wrote_path,
        source_identity: src_id,
        source_content_fingerprint: format_content_fingerprint(source_inspection.hash),
    })
}

fn try_load_cache(
    cache_path: &Path,
    src_id: SourceIdentity,
    current_source_content_hash: u64,
    max_rows: u64,
) -> Result<Option<Candles>, String> {
    let mut f = match fs::File::open(cache_path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.to_string()),
    };
    let md = f.metadata().map_err(|e| e.to_string())?;
    if md.len() < HEADER_LEN as u64 {
        return Ok(None);
    }
    let mut header = [0u8; HEADER_LEN];
    f.read_exact(&mut header).map_err(|e| e.to_string())?;

    let Some((fields_mask, len_u64, csv_size, csv_mtime_ms, source_content_hash)) =
        read_header(&header)
    else {
        return Ok(None);
    };

    if csv_size != src_id.size || csv_mtime_ms != src_id.mtime_utc_ms {
        return Ok(None);
    }

    if source_content_hash != current_source_content_hash {
        return Ok(None);
    }
    if len_u64 > max_rows {
        return Ok(None);
    }

    let Some(expected_size) = checked_expected_cache_size(len_u64) else {
        return Ok(None);
    };
    let Some(len) = usize::try_from(len_u64).ok() else {
        return Ok(None);
    };
    if md.len() != expected_size {
        return Ok(None);
    }

    let mut r = std::io::BufReader::new(f);

    let timestamp = read_le_vec_i64(&mut r, len)?;
    let open = read_le_vec_f64(&mut r, len)?;
    let high = read_le_vec_f64(&mut r, len)?;
    let low = read_le_vec_f64(&mut r, len)?;
    let close = read_le_vec_f64(&mut r, len)?;
    let volume = read_le_vec_f64(&mut r, len)?;

    let fields = mask_to_flags(fields_mask);
    Ok(Some(Candles::new_with_fields(
        timestamp, open, high, low, close, volume, fields,
    )))
}

#[cfg(test)]
fn write_cache(
    cache_path: &Path,
    csv_path: &Path,
    candles: &Candles,
    src_id: SourceIdentity,
) -> Result<(), String> {
    let source_content_hash = content_fingerprint_hash_for_source(csv_path)?;
    write_cache_with_source_hash(cache_path, candles, src_id, source_content_hash)
}

fn write_cache_with_source_hash(
    cache_path: &Path,
    candles: &Candles,
    src_id: SourceIdentity,
    source_content_hash: u64,
) -> Result<(), String> {
    let len = candles.timestamp.len();
    if candles.open.len() != len
        || candles.high.len() != len
        || candles.low.len() != len
        || candles.close.len() != len
        || candles.volume.len() != len
    {
        return Err("candles vectors have inconsistent lengths".to_string());
    }

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut min_ts = 0i64;
    let mut max_ts = 0i64;
    for &t in &candles.timestamp {
        if min_ts == 0 || t < min_ts {
            min_ts = t;
        }
        if t > max_ts {
            max_ts = t;
        }
    }

    let fields_mask = flags_to_mask(candles.fields);
    let mut header = [0u8; HEADER_LEN];
    write_header(
        &mut header,
        fields_mask,
        len as u64,
        src_id.size,
        src_id.mtime_utc_ms,
        min_ts,
        max_ts,
        source_content_hash,
    );

    let (tmp, f) = create_unique_temp_cache_file(cache_path)?;
    let mut w = std::io::BufWriter::new(f);

    w.write_all(&header).map_err(|e| e.to_string())?;
    write_le_slice_i64(&mut w, &candles.timestamp)?;
    write_le_slice_f64(&mut w, &candles.open)?;
    write_le_slice_f64(&mut w, &candles.high)?;
    write_le_slice_f64(&mut w, &candles.low)?;
    write_le_slice_f64(&mut w, &candles.close)?;
    write_le_slice_f64(&mut w, &candles.volume)?;

    w.flush().map_err(|e| e.to_string())?;
    drop(w);

    match fs::rename(&tmp, cache_path) {
        Ok(()) => Ok(()),
        Err(rename_err)
            if matches!(
                rename_err.kind(),
                std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            match fs::remove_file(cache_path) {
                Ok(()) => {
                    fs::rename(&tmp, cache_path).map_err(|e| e.to_string())?;
                    Ok(())
                }
                Err(remove_err) if remove_err.kind() == std::io::ErrorKind::NotFound => {
                    fs::rename(&tmp, cache_path).map_err(|e| e.to_string())?;
                    Ok(())
                }
                Err(remove_err) => {
                    let _ = fs::remove_file(&tmp);
                    Err(remove_err.to_string())
                }
            }
        }
        Err(rename_err) => {
            let _ = fs::remove_file(&tmp);
            Err(rename_err.to_string())
        }
    }
}

fn create_unique_temp_cache_file(cache_path: &Path) -> Result<(PathBuf, fs::File), String> {
    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    let parent = cache_path
        .parent()
        .ok_or_else(|| "cache path has no parent directory".to_string())?;
    let file_name = cache_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cache.vta");
    let pid = std::process::id();
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    for _ in 0..32 {
        let nonce = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!("{file_name}.{pid}.{seed:016x}.{nonce:016x}.tmp"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.to_string()),
        }
    }

    Err("failed to create unique temp cache file".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        content_fingerprint_for_source, fallback_cache_path, identity_for_source, load_or_build,
        load_or_build_with_identity, load_or_build_with_identity_and_limits, write_cache,
        write_header, CsvLoadLimits, SourceIdentity, HEADER_LEN,
    };
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let pid = std::process::id();
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{prefix}_{pid}_{uniq}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn write_candles_csv(path: &Path) {
        fs::write(
            path,
            concat!(
                "timestamp,open,high,low,close,volume\n",
                "1700000000000,100,101,99,100.5,1000\n",
                "1700000060000,100.5,101.5,100,101,1100\n",
                "1700000120000,101,102,100.5,101.5,1200\n"
            ),
        )
        .expect("write candles csv");
    }

    fn write_short_candles_csv(path: &Path) {
        fs::write(
            path,
            concat!(
                "timestamp,open,high,low,close,volume\n",
                "1700000000000,10,10.5,9.5,10.25,100\n",
                "1700000060000,11,11.5,10.5,11.25,200\n"
            ),
        )
        .expect("write short candles csv");
    }

    fn write_candles_csv_same_size_variant(path: &Path) {
        fs::write(
            path,
            concat!(
                "timestamp,open,high,low,close,volume\n",
                "1700000000000,110,111,98,110.5,1001\n",
                "1700000060000,110.5,111.5,109,111,1101\n",
                "1700000120000,111,112,110.5,111.5,1201\n"
            ),
        )
        .expect("write same-size candles csv variant");
    }

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        let pid = std::process::id();
        std::env::temp_dir().join(format!("vta_fp_test_{pid}_{name}.bin"))
    }

    #[test]
    fn content_fingerprint_is_stable_for_same_content() {
        let p = temp_file_path("stable");
        let mut f = fs::File::create(&p).expect("create");
        f.write_all(b"abc123\nxyz").expect("write");
        drop(f);

        let a = content_fingerprint_for_source(&p).expect("fp a");
        let b = content_fingerprint_for_source(&p).expect("fp b");
        assert_eq!(a, b);
        assert!(a.starts_with("fnv1a64-full-"));

        let _ = fs::remove_file(&p);
    }

    #[test]
    fn content_fingerprint_changes_when_content_changes() {
        let p = temp_file_path("change");
        let mut f = fs::File::create(&p).expect("create");
        f.write_all(b"left").expect("write left");
        drop(f);
        let a = content_fingerprint_for_source(&p).expect("fp a");

        let mut f2 = fs::File::create(&p).expect("recreate");
        f2.write_all(b"right").expect("write right");
        drop(f2);
        let b = content_fingerprint_for_source(&p).expect("fp b");

        assert_ne!(a, b);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn source_identity_equality_uses_size_mtime_and_change_marker() {
        let a = SourceIdentity {
            size: 123,
            mtime_utc_ms: 456,
            change_marker: 789,
        };
        let b = SourceIdentity {
            size: 123,
            mtime_utc_ms: 456,
            change_marker: 789,
        };
        let c = SourceIdentity {
            size: 124,
            mtime_utc_ms: 456,
            change_marker: 789,
        };
        let d = SourceIdentity {
            size: 123,
            mtime_utc_ms: 456,
            change_marker: 790,
        };

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn load_or_build_with_identity_matches_default_loader_behavior() {
        let temp = make_temp_dir("data_cache_loader");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);

        let src_id = identity_for_source(&csv).expect("source identity");
        let first = load_or_build_with_identity(&csv, &cache_dir, src_id).expect("first load");
        let second = load_or_build(&csv, &cache_dir).expect("second load");

        assert!(!first.cache_hit);
        assert_eq!(first.source_identity, src_id);
        assert_eq!(second.source_identity, src_id);
        assert_eq!(
            first.source_content_fingerprint,
            content_fingerprint_for_source(&csv).expect("source fingerprint")
        );
        assert_eq!(
            second.source_content_fingerprint,
            first.source_content_fingerprint
        );
        assert!(second.cache_hit);
        assert_eq!(first.candles.timestamp, second.candles.timestamp);
        assert_eq!(first.candles.close, second.candles.close);
        assert_eq!(first.cache_path, second.cache_path);
    }

    #[test]
    fn bounded_loader_rejects_source_bytes_before_parse_or_cache_write() {
        let temp = make_temp_dir("data_cache_source_byte_limit");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");
        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let src_id = identity_for_source(&csv).expect("source identity");

        let err = load_or_build_with_identity_and_limits(
            &csv,
            &cache_dir,
            src_id,
            CsvLoadLimits {
                max_source_bytes: src_id.size.saturating_sub(1),
                max_rows: u64::MAX,
            },
        )
        .expect_err("oversized CSV should fail before parse");

        assert!(err.contains("bounded load limit"));
        assert!(err.contains("VECTORBT_MAX_SOURCE_CSV_BYTES"));
        assert!(!csv.with_extension("vta").exists());
        assert!(
            fs::read_dir(&cache_dir)
                .expect("read cache dir")
                .next()
                .is_none(),
            "bounded rejection must not publish a fallback cache"
        );
    }

    #[test]
    fn bounded_loader_rejects_excess_rows_before_parse_or_cache_write() {
        let temp = make_temp_dir("data_cache_source_row_limit");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");
        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let src_id = identity_for_source(&csv).expect("source identity");

        let err = load_or_build_with_identity_and_limits(
            &csv,
            &cache_dir,
            src_id,
            CsvLoadLimits {
                max_source_bytes: u64::MAX,
                max_rows: 2,
            },
        )
        .expect_err("row-heavy CSV should fail before parse");

        assert!(err.contains("more than 2 data rows"));
        assert!(err.contains("VECTORBT_MAX_SOURCE_CSV_ROWS"));
        assert!(!csv.with_extension("vta").exists());
        assert!(
            fs::read_dir(&cache_dir)
                .expect("read cache dir")
                .next()
                .is_none(),
            "bounded rejection must not publish a fallback cache"
        );
    }

    #[test]
    #[ignore = "manual source-integrity timing probe; set VECTORBT_SOURCE_TIMING_CSV"]
    fn source_identity_timing_probe() {
        let path = std::env::var("VECTORBT_SOURCE_TIMING_CSV")
            .map(PathBuf::from)
            .expect("set VECTORBT_SOURCE_TIMING_CSV to an existing CSV");
        let iterations = 100u32;

        let identity_started = std::time::Instant::now();
        for _ in 0..iterations {
            identity_for_source(&path).expect("source identity");
        }
        let identity_elapsed = identity_started.elapsed();

        let fingerprint_started = std::time::Instant::now();
        content_fingerprint_for_source(&path).expect("source fingerprint");
        let fingerprint_elapsed = fingerprint_started.elapsed();

        eprintln!(
            "source_identity_timing path={} bytes={} iterations={} identity_total_ms={:.3} identity_mean_ms={:.6} full_fingerprint_ms={:.3}",
            path.display(),
            fs::metadata(&path).expect("source metadata").len(),
            iterations,
            identity_elapsed.as_secs_f64() * 1000.0,
            identity_elapsed.as_secs_f64() * 1000.0 / f64::from(iterations),
            fingerprint_elapsed.as_secs_f64() * 1000.0,
        );
    }

    #[test]
    fn cache_reparse_is_forced_when_content_changes_with_same_size_and_mtime() {
        let temp = make_temp_dir("data_cache_same_identity_content_drift");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);

        let initial_identity = identity_for_source(&csv).expect("initial source identity");
        let initial_fingerprint =
            content_fingerprint_for_source(&csv).expect("initial content fingerprint");
        let initial_modified = fs::metadata(&csv)
            .expect("csv metadata")
            .modified()
            .expect("csv modified time");

        let first =
            load_or_build_with_identity(&csv, &cache_dir, initial_identity).expect("initial load");
        assert!(!first.cache_hit);

        write_candles_csv_same_size_variant(&csv);
        let reopened = fs::File::options()
            .write(true)
            .open(&csv)
            .expect("reopen variant csv");
        reopened
            .set_times(std::fs::FileTimes::new().set_modified(initial_modified))
            .expect("restore original modified time");
        drop(reopened);

        let drifted_identity = identity_for_source(&csv).expect("drifted source identity");
        let drifted_fingerprint =
            content_fingerprint_for_source(&csv).expect("drifted content fingerprint");
        assert_eq!(drifted_identity.size, initial_identity.size);
        assert_eq!(drifted_identity.mtime_utc_ms, initial_identity.mtime_utc_ms);
        assert_ne!(
            drifted_identity.change_marker,
            initial_identity.change_marker
        );
        assert_ne!(drifted_fingerprint, initial_fingerprint);

        let second =
            load_or_build_with_identity(&csv, &cache_dir, drifted_identity).expect("reload");
        assert!(!second.cache_hit);
        assert_ne!(first.candles.close, second.candles.close);
    }

    #[test]
    fn loader_uses_fallback_cache_when_sidecar_is_missing() {
        let temp = make_temp_dir("data_cache_fallback");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let src_id = identity_for_source(&csv).expect("source identity");
        let candles =
            ta_app_core::read_candles_from_csv_compat(csv.to_string_lossy().as_ref()).expect("csv");
        let fallback = fallback_cache_path(&cache_dir, &csv);
        write_cache(&fallback, &csv, &candles, src_id).expect("write fallback cache");

        let load = load_or_build_with_identity(&csv, &cache_dir, src_id).expect("fallback load");
        assert!(load.cache_hit);
        assert_eq!(load.cache_path, Some(fallback));
        assert_eq!(load.candles.timestamp, candles.timestamp);
    }

    #[test]
    fn loader_prefers_sidecar_cache_over_fallback_when_both_exist() {
        let temp = make_temp_dir("data_cache_sidecar_precedence");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("candles.csv");
        let fallback_csv = temp.join("fallback.csv");
        write_candles_csv(&csv);
        write_short_candles_csv(&fallback_csv);
        let src_id = identity_for_source(&csv).expect("source identity");

        let sidecar_candles =
            ta_app_core::read_candles_from_csv_compat(csv.to_string_lossy().as_ref()).expect("csv");
        let fallback_candles =
            ta_app_core::read_candles_from_csv_compat(fallback_csv.to_string_lossy().as_ref())
                .expect("fallback csv");

        let sidecar = csv.with_extension("vta");
        let fallback = fallback_cache_path(&cache_dir, &csv);
        write_cache(&sidecar, &csv, &sidecar_candles, src_id).expect("write sidecar cache");
        write_cache(&fallback, &csv, &fallback_candles, src_id).expect("write fallback cache");

        let load = load_or_build_with_identity(&csv, &cache_dir, src_id).expect("sidecar load");
        assert!(load.cache_hit);
        assert_eq!(load.cache_path, Some(sidecar));
        assert_eq!(load.candles.timestamp, sidecar_candles.timestamp);
        assert_eq!(load.candles.close, sidecar_candles.close);
        assert_ne!(load.candles.timestamp, fallback_candles.timestamp);
    }

    #[test]
    fn corrupt_cache_is_ignored_and_source_is_reparsed() {
        let temp = make_temp_dir("data_cache_corrupt");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let sidecar = csv.with_extension("vta");
        fs::write(&sidecar, b"bad-cache").expect("write corrupt sidecar");

        let load = load_or_build(&csv, &cache_dir).expect("load after corrupt cache");
        assert!(!load.cache_hit);
        assert_eq!(load.candles.timestamp.len(), 3);
        assert!(load.cache_path.is_some());
    }

    #[test]
    fn malformed_header_with_oversized_len_is_ignored_and_source_is_reparsed() {
        let temp = make_temp_dir("data_cache_oversized_header");
        let cache_dir = temp.join("cache");
        fs::create_dir_all(&cache_dir).expect("create cache dir");

        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let src_id = identity_for_source(&csv).expect("source identity");
        let sidecar = csv.with_extension("vta");

        let mut header = [0u8; HEADER_LEN];
        write_header(
            &mut header,
            0b1_1111,
            u64::MAX,
            src_id.size,
            src_id.mtime_utc_ms,
            0,
            0,
            0,
        );
        fs::write(&sidecar, header).expect("write malformed cache header");

        let load = load_or_build_with_identity(&csv, &cache_dir, src_id)
            .expect("load after malformed oversized header");
        assert!(!load.cache_hit);
        assert_eq!(load.candles.timestamp.len(), 3);
        assert!(load.cache_path.is_some());
    }

    #[test]
    fn write_cache_ignores_legacy_fixed_tmp_path_collision() {
        let temp = make_temp_dir("data_cache_tmp_collision");
        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let src_id = identity_for_source(&csv).expect("source identity");
        let candles =
            ta_app_core::read_candles_from_csv_compat(csv.to_string_lossy().as_ref()).expect("csv");
        let cache_path = csv.with_extension("vta");
        let legacy_tmp = cache_path.with_extension("vta.tmp");
        fs::write(&legacy_tmp, b"sentinel").expect("write legacy tmp collision file");

        write_cache(&cache_path, &csv, &candles, src_id)
            .expect("write cache with fixed tmp collision");

        assert_eq!(fs::read(&legacy_tmp).expect("read legacy tmp"), b"sentinel");
        assert!(cache_path.exists());
    }

    #[test]
    fn write_cache_supports_same_target_concurrent_writers() {
        let temp = make_temp_dir("data_cache_concurrent_same_target");
        let csv = temp.join("candles.csv");
        write_candles_csv(&csv);
        let src_id = identity_for_source(&csv).expect("source identity");
        let cache_path = csv.with_extension("vta");
        let barrier = Arc::new(Barrier::new(2));

        let mut handles = Vec::new();
        for _ in 0..2 {
            let csv = csv.clone();
            let cache_path = cache_path.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                let candles =
                    ta_app_core::read_candles_from_csv_compat(csv.to_string_lossy().as_ref())
                        .expect("read csv");
                barrier.wait();
                write_cache(&cache_path, &csv, &candles, src_id)
            }));
        }

        for handle in handles {
            handle
                .join()
                .expect("join concurrent cache writer")
                .expect("concurrent cache write");
        }

        let load = load_or_build_with_identity(&csv, &temp.join("cache"), src_id)
            .expect("load after concurrent writes");
        assert!(load.cache_hit);
        assert_eq!(load.candles.timestamp.len(), 3);
        assert!(cache_path.exists());
    }
}
