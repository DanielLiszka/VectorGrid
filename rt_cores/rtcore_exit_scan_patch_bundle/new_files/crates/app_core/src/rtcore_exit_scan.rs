use std::ffi::{c_char, c_int, c_void};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::rc::Rc;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExitHit {
    pub stop_hit_bar: Option<i32>,
    pub target_hit_bar: Option<i32>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct RtBarrierBuildInput {
    lows: *const f32,
    highs: *const f32,
    count: i32,
    z_min: f32,
    z_max: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct RtBarrierQuery {
    entry_bar: i32,
    barrier_price: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct RtBarrierResult {
    hit: i32,
    hit_bar: i32,
    primitive_index: i32,
}

#[cfg(rtcore_barrier_compiled)]
mod ffi {
    use super::*;

    #[link(name = "rtcore_barrier")]
    extern "C" {
        pub fn rtbarrier_is_available(device_ordinal: c_int) -> c_int;
        pub fn rtbarrier_create(
            device_ordinal: c_int,
            out_scene: *mut *mut c_void,
            error_buf: *mut c_char,
            error_buf_len: c_int,
        ) -> c_int;
        pub fn rtbarrier_destroy(scene: *mut c_void) -> c_int;
        pub fn rtbarrier_build(
            scene: *mut c_void,
            input: *const RtBarrierBuildInput,
            error_buf: *mut c_char,
            error_buf_len: c_int,
        ) -> c_int;
        pub fn rtbarrier_query(
            scene: *mut c_void,
            queries: *const RtBarrierQuery,
            query_count: c_int,
            out_results: *mut RtBarrierResult,
            cuda_stream: u64,
            error_buf: *mut c_char,
            error_buf_len: c_int,
        ) -> c_int;
    }
}

#[cfg(not(rtcore_barrier_compiled))]
mod ffi {
    use super::*;

    pub unsafe fn rtbarrier_is_available(_device_ordinal: c_int) -> c_int {
        0
    }

    pub unsafe fn rtbarrier_create(
        _device_ordinal: c_int,
        _out_scene: *mut *mut c_void,
        _error_buf: *mut c_char,
        _error_buf_len: c_int,
    ) -> c_int {
        0
    }

    pub unsafe fn rtbarrier_destroy(_scene: *mut c_void) -> c_int {
        1
    }

    pub unsafe fn rtbarrier_build(
        _scene: *mut c_void,
        _input: *const RtBarrierBuildInput,
        _error_buf: *mut c_char,
        _error_buf_len: c_int,
    ) -> c_int {
        0
    }

    pub unsafe fn rtbarrier_query(
        _scene: *mut c_void,
        _queries: *const RtBarrierQuery,
        _query_count: c_int,
        _out_results: *mut RtBarrierResult,
        _cuda_stream: u64,
        _error_buf: *mut c_char,
        _error_buf_len: c_int,
    ) -> c_int {
        0
    }
}

fn rtcore_runtime_requested() -> bool {
    matches!(
        std::env::var("VECTORBT_RTCORE_BARRIER").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

fn take_error(prefix: &str, buf: &[i8]) -> Error {
    let nul = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let bytes: Vec<u8> = buf[..nul].iter().map(|b| *b as u8).collect();
    let msg = String::from_utf8_lossy(&bytes).trim().to_owned();
    let full = if msg.is_empty() {
        prefix.to_owned()
    } else {
        format!("{prefix}: {msg}")
    };
    full.into()
}

pub struct RtCoreBarrierScene {
    raw: NonNull<c_void>,
    bar_count: usize,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl RtCoreBarrierScene {
    pub fn compiled() -> bool {
        cfg!(rtcore_barrier_compiled)
    }

    pub fn runtime_requested() -> bool {
        rtcore_runtime_requested()
    }

    pub fn available(device_ordinal: i32) -> bool {
        if !Self::compiled() {
            return false;
        }
        unsafe { ffi::rtbarrier_is_available(device_ordinal as c_int) == 1 }
    }

    pub fn new(device_ordinal: i32) -> Result<Self> {
        if !Self::compiled() {
            return Err(
                "RT-core exit scan was not compiled in. Set OPTIX_HOME and rebuild to enable it.".into(),
            );
        }

        let mut scene_ptr: *mut c_void = std::ptr::null_mut();
        let mut error_buf = [0i8; 1024];
        let ok = unsafe {
            ffi::rtbarrier_create(
                device_ordinal as c_int,
                &mut scene_ptr,
                error_buf.as_mut_ptr(),
                error_buf.len() as c_int,
            )
        };
        if ok != 1 {
            return Err(take_error("rtbarrier_create failed", &error_buf));
        }

        let raw = NonNull::new(scene_ptr).ok_or_else(|| -> Error {
            "rtbarrier_create succeeded but returned a null scene pointer".into()
        })?;

        Ok(Self {
            raw,
            bar_count: 0,
            _not_send_sync: PhantomData,
        })
    }

    pub fn build_from_low_high(&mut self, lows: &[f32], highs: &[f32]) -> Result<()> {
        if lows.len() != highs.len() {
            return Err("lows.len() must equal highs.len()".into());
        }
        if lows.is_empty() {
            return Err("cannot build an RT-core barrier scene from empty inputs".into());
        }
        if lows.len() > i32::MAX as usize {
            return Err("too many bars for rtcore barrier scene".into());
        }

        let input = RtBarrierBuildInput {
            lows: lows.as_ptr(),
            highs: highs.as_ptr(),
            count: lows.len() as i32,
            z_min: 0.0,
            z_max: 1.0,
        };
        let mut error_buf = [0i8; 1024];
        let ok = unsafe {
            ffi::rtbarrier_build(
                self.raw.as_ptr(),
                &input,
                error_buf.as_mut_ptr(),
                error_buf.len() as c_int,
            )
        };
        if ok != 1 {
            return Err(take_error("rtbarrier_build failed", &error_buf));
        }
        self.bar_count = lows.len();
        Ok(())
    }

    pub fn first_hit_batch(
        &self,
        entry_bars: &[i32],
        barrier_prices: &[f32],
    ) -> Result<Vec<Option<i32>>> {
        self.first_hit_batch_on_stream(entry_bars, barrier_prices, 0)
    }

    pub fn first_hit_batch_on_stream(
        &self,
        entry_bars: &[i32],
        barrier_prices: &[f32],
        cuda_stream: u64,
    ) -> Result<Vec<Option<i32>>> {
        if self.bar_count == 0 {
            return Err("barrier scene has not been built yet".into());
        }
        if entry_bars.len() != barrier_prices.len() {
            return Err("entry_bars.len() must equal barrier_prices.len()".into());
        }
        if entry_bars.is_empty() {
            return Ok(Vec::new());
        }
        if entry_bars.len() > i32::MAX as usize {
            return Err("too many queries for rtcore barrier launch".into());
        }

        let queries: Vec<RtBarrierQuery> = entry_bars
            .iter()
            .copied()
            .zip(barrier_prices.iter().copied())
            .map(|(entry_bar, barrier_price)| RtBarrierQuery {
                entry_bar,
                barrier_price,
            })
            .collect();

        let mut results = vec![RtBarrierResult::default(); queries.len()];
        let mut error_buf = [0i8; 1024];
        let ok = unsafe {
            ffi::rtbarrier_query(
                self.raw.as_ptr(),
                queries.as_ptr(),
                queries.len() as c_int,
                results.as_mut_ptr(),
                cuda_stream,
                error_buf.as_mut_ptr(),
                error_buf.len() as c_int,
            )
        };
        if ok != 1 {
            return Err(take_error("rtbarrier_query failed", &error_buf));
        }

        Ok(results
            .into_iter()
            .map(|r| if r.hit == 1 && r.hit_bar >= 0 { Some(r.hit_bar) } else { None })
            .collect())
    }

    pub fn evaluate_stop_target_hits(
        &self,
        entry_bars: &[i32],
        stop_prices: &[f32],
        target_prices: &[f32],
    ) -> Result<Vec<ExitHit>> {
        if entry_bars.len() != stop_prices.len() || entry_bars.len() != target_prices.len() {
            return Err("entry_bars, stop_prices, and target_prices must have identical lengths".into());
        }
        let stop_hit_bars = self.first_hit_batch(entry_bars, stop_prices)?;
        let target_hit_bars = self.first_hit_batch(entry_bars, target_prices)?;
        Ok(stop_hit_bars
            .into_iter()
            .zip(target_hit_bars.into_iter())
            .map(|(stop_hit_bar, target_hit_bar)| ExitHit {
                stop_hit_bar,
                target_hit_bar,
            })
            .collect())
    }
}

impl Drop for RtCoreBarrierScene {
    fn drop(&mut self) {
        unsafe {
            let _ = ffi::rtbarrier_destroy(self.raw.as_ptr());
        }
    }
}

pub fn evaluate_exit_bars_rtcore(
    lows: &[f32],
    highs: &[f32],
    entry_bars: &[i32],
    stop_prices: &[f32],
    target_prices: &[f32],
) -> Result<Vec<ExitHit>> {
    let mut scene = RtCoreBarrierScene::new(0)?;
    scene.build_from_low_high(lows, highs)?;
    scene.evaluate_stop_target_hits(entry_bars, stop_prices, target_prices)
}

pub fn maybe_evaluate_exit_bars_rtcore(
    lows: &[f32],
    highs: &[f32],
    entry_bars: &[i32],
    stop_prices: &[f32],
    target_prices: &[f32],
) -> Result<Option<Vec<ExitHit>>> {
    if !rtcore_runtime_requested() {
        return Ok(None);
    }
    let hits = evaluate_exit_bars_rtcore(lows, highs, entry_bars, stop_prices, target_prices)?;
    Ok(Some(hits))
}
