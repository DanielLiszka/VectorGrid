use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vector_ta::utilities::data_loader::{CandleFieldFlags, Candles};

const MAGIC: &[u8; 4] = b"VTA1";
const VERSION: u32 = 1;
const HEADER_LEN: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct SourceIdentity {
    pub size: u64,
    pub mtime_utc_ms: i64,
}

#[derive(Debug, Clone)]
pub struct LoadOutcome {
    pub candles: Candles,
    pub cache_hit: bool,
    pub cache_path: Option<PathBuf>,
}

pub fn app_data_cache_dir(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir.join("data_cache")
}

pub fn identity_for_source(path: &Path) -> Result<SourceIdentity, String> {
    let md = fs::metadata(path).map_err(|e| e.to_string())?;
    let size = md.len();

    let mtime = md.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let ms = mtime
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    Ok(SourceIdentity {
        size,
        mtime_utc_ms: ms,
    })
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

fn expected_cache_size(len: usize) -> u64 {
    HEADER_LEN as u64 + (len as u64) * 48u64
}

fn write_header(
    out: &mut [u8; HEADER_LEN],
    fields_mask: u32,
    len: u64,
    csv_size: u64,
    csv_mtime_ms: i64,
    min_ts: i64,
    max_ts: i64,
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
}

fn read_header(input: &[u8; HEADER_LEN]) -> Option<(u32, u64, u64, i64)> {
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
    Some((fields_mask, len, csv_size, csv_mtime_ms))
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
    let mut out = vec![0i64; len];
    let buf = unsafe { std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, len * 8) };
    r.read_exact(buf).map_err(|e| e.to_string())?;

    #[cfg(target_endian = "big")]
    {
        for x in &mut out {
            *x = i64::from_le(*x);
        }
    }
    Ok(out)
}

fn read_le_vec_f64(r: &mut dyn Read, len: usize) -> Result<Vec<f64>, String> {
    let mut out = vec![0f64; len];
    let buf = unsafe { std::slice::from_raw_parts_mut(out.as_mut_ptr() as *mut u8, len * 8) };
    r.read_exact(buf).map_err(|e| e.to_string())?;

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
    let sidecar = sidecar_cache_path(csv_path);
    let fallback = fallback_cache_path(app_cache_dir, csv_path);

    if let Ok(Some(candles)) = try_load_cache(&sidecar, src_id) {
        return Ok(LoadOutcome {
            candles,
            cache_hit: true,
            cache_path: Some(sidecar),
        });
    }

    if sidecar != fallback {
        if let Ok(Some(candles)) = try_load_cache(&fallback, src_id) {
            return Ok(LoadOutcome {
                candles,
                cache_hit: true,
                cache_path: Some(fallback),
            });
        }
    }

    // Cache miss: parse CSV, then write cache for future runs.
    let candles =
        vector_ta::utilities::data_loader::read_candles_from_csv(&csv_path.to_string_lossy())
            .map_err(|e| e.to_string())?;

    let mut wrote_path = None;
    if let Ok(()) = write_cache(&sidecar, &candles, src_id) {
        wrote_path = Some(sidecar);
    } else if sidecar != fallback {
        if let Ok(()) = write_cache(&fallback, &candles, src_id) {
            wrote_path = Some(fallback);
        }
    }

    Ok(LoadOutcome {
        candles,
        cache_hit: false,
        cache_path: wrote_path,
    })
}

fn try_load_cache(cache_path: &Path, src_id: SourceIdentity) -> Result<Option<Candles>, String> {
    if !cache_path.exists() {
        return Ok(None);
    }

    let md = fs::metadata(cache_path).map_err(|e| e.to_string())?;
    if md.len() < HEADER_LEN as u64 {
        return Ok(None);
    }

    let mut f = fs::File::open(cache_path).map_err(|e| e.to_string())?;
    let mut header = [0u8; HEADER_LEN];
    f.read_exact(&mut header).map_err(|e| e.to_string())?;

    let Some((fields_mask, len_u64, csv_size, csv_mtime_ms)) = read_header(&header) else {
        return Ok(None);
    };

    if csv_size != src_id.size || csv_mtime_ms != src_id.mtime_utc_ms {
        return Ok(None);
    }

    let len = usize::try_from(len_u64).map_err(|_| "cache len overflows usize".to_string())?;
    if md.len() != expected_cache_size(len) {
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

fn write_cache(cache_path: &Path, candles: &Candles, src_id: SourceIdentity) -> Result<(), String> {
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
    );

    let tmp = cache_path.with_extension("vta.tmp");
    let f = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(f);

    w.write_all(&header).map_err(|e| e.to_string())?;
    write_le_slice_i64(&mut w, &candles.timestamp)?;
    write_le_slice_f64(&mut w, &candles.open)?;
    write_le_slice_f64(&mut w, &candles.high)?;
    write_le_slice_f64(&mut w, &candles.low)?;
    write_le_slice_f64(&mut w, &candles.close)?;
    write_le_slice_f64(&mut w, &candles.volume)?;

    w.flush().map_err(|e| e.to_string())?;

    if cache_path.exists() {
        let _ = fs::remove_file(cache_path);
    }
    fs::rename(tmp, cache_path).map_err(|e| e.to_string())?;
    Ok(())
}
