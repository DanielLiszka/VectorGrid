use core::fmt;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr::NonNull;

#[derive(Clone, Copy, Debug)]
pub enum TcPrecision {
    /// FP32 inputs with FAST_TF32 compute on Tensor Cores.
    Tf32Fast = 0,
    /// FP16 inputs with FP32 accumulation on Tensor Cores.
    F16 = 1,
}

#[derive(Clone, Copy, Debug)]
pub struct AlmaSpec {
    pub period: u16,
    pub offset: f32,
    pub sigma: f32,
}

#[derive(Debug)]
pub struct TensorCoreError {
    code: i32,
    message: String,
}

impl TensorCoreError {
    fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    fn from_last_error(code: i32, fallback: &str) -> Self {
        let msg = unsafe {
            let ptr = tca_last_error();
            if ptr.is_null() {
                fallback.to_string()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        Self::new(code, msg)
    }
}

impl fmt::Display for TensorCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TensorCoreError(code={}, message={})", self.code, self.message)
    }
}

impl std::error::Error for TensorCoreError {}

#[repr(C)]
struct TcaPlanOpaque {
    _private: [u8; 0],
}

extern "C" {
    fn tca_create_plan(
        host_weights_t: *const f32,
        n_windows: c_int,
        window: c_int,
        precision: c_int,
        out_plan: *mut *mut TcaPlanOpaque,
    ) -> c_int;
    fn tca_destroy_plan(plan: *mut TcaPlanOpaque) -> c_int;
    fn tca_run_plan(
        plan: *mut TcaPlanOpaque,
        device_prices: *const f32,
        n_prices: c_int,
        chunk_rows: c_int,
        device_out: *mut f32,
        out_rows: *mut c_int,
        cuda_stream: *mut c_void,
    ) -> c_int;
    fn tca_last_error() -> *const c_char;

    fn tca_malloc_f32(count: usize, out_ptr: *mut *mut f32) -> c_int;
    fn tca_free_ptr(ptr: *mut c_void) -> c_int;
    fn tca_memcpy_h2d_f32(dst: *mut f32, src: *const f32, count: usize) -> c_int;
    fn tca_memcpy_d2h_f32(dst: *mut f32, src: *const f32, count: usize) -> c_int;
    fn tca_device_synchronize() -> c_int;
}

/// A self-contained Tensor Core ALMA plan.
///
/// Intended integration point:
/// - build one plan per unique ALMA sweep (fast family, slow family)
/// - keep prices in VRAM
/// - produce one indicator slab [rows, params] per family
/// - feed slabs into the existing exact row-first backtest kernel
///
/// The output rows correspond to windows ending at indices `valid_from..n_prices-1`.
/// `valid_from()` equals `max_period - 1` for the plan.
pub struct TensorCoreAlmaPlan {
    raw: NonNull<TcaPlanOpaque>,
    n_windows: usize,
    window: usize,
    precision: TcPrecision,
}

impl TensorCoreAlmaPlan {
    pub fn from_alma_specs(specs: &[AlmaSpec], precision: TcPrecision) -> Result<Self, TensorCoreError> {
        if specs.is_empty() {
            return Err(TensorCoreError::new(-1, "empty ALMA spec list"));
        }
        let window = specs.iter().map(|s| s.period as usize).max().unwrap_or(0);
        if window == 0 {
            return Err(TensorCoreError::new(-2, "max ALMA period is zero"));
        }

        let mut weights_t = vec![0.0f32; window * specs.len()];
        for (col, spec) in specs.iter().enumerate() {
            let period = spec.period as usize;
            if period == 0 {
                return Err(TensorCoreError::new(-3, format!("period=0 at column {}", col)));
            }
            if !(spec.sigma.is_finite() && spec.sigma > 0.0) {
                return Err(TensorCoreError::new(-4, format!("invalid sigma at column {}", col)));
            }
            if !spec.offset.is_finite() {
                return Err(TensorCoreError::new(-5, format!("invalid offset at column {}", col)));
            }

            let mean = spec.offset * ((period - 1) as f32);
            let stdev = (period as f32) / spec.sigma;
            let mut tmp = vec![0.0f32; period];
            let mut sum = 0.0f32;
            for i in 0..period {
                let x = i as f32;
                let w = (-(x - mean).powi(2) / (2.0 * stdev * stdev)).exp();
                tmp[i] = w;
                sum += w;
            }
            if sum == 0.0 || !sum.is_finite() {
                return Err(TensorCoreError::new(-6, format!("degenerate ALMA weights at column {}", col)));
            }
            let base = window - period;
            for i in 0..period {
                let row = base + i;
                weights_t[row * specs.len() + col] = tmp[i] / sum;
            }
        }

        let mut raw = std::ptr::null_mut();
        let rc = unsafe {
            tca_create_plan(
                weights_t.as_ptr(),
                specs.len() as c_int,
                window as c_int,
                precision as c_int,
                &mut raw,
            )
        };
        if rc != 0 {
            return Err(TensorCoreError::from_last_error(rc, "tca_create_plan failed"));
        }
        let raw = NonNull::new(raw).ok_or_else(|| TensorCoreError::new(-7, "tca_create_plan returned null plan"))?;
        Ok(Self { raw, n_windows: specs.len(), window, precision })
    }

    pub fn from_sma_periods(periods: &[u16], precision: TcPrecision) -> Result<Self, TensorCoreError> {
        let specs: Vec<AlmaSpec> = periods
            .iter()
            .map(|&p| AlmaSpec { period: p, offset: 0.5, sigma: 1_000_000.0 })
            .collect();
        Self::from_uniform_windows(&specs, precision)
    }

    fn from_uniform_windows(specs: &[AlmaSpec], precision: TcPrecision) -> Result<Self, TensorCoreError> {
        if specs.is_empty() {
            return Err(TensorCoreError::new(-8, "empty uniform-window spec list"));
        }
        let window = specs.iter().map(|s| s.period as usize).max().unwrap_or(0);
        let mut weights_t = vec![0.0f32; window * specs.len()];
        for (col, spec) in specs.iter().enumerate() {
            let period = spec.period as usize;
            if period == 0 {
                return Err(TensorCoreError::new(-9, format!("period=0 at column {}", col)));
            }
            let base = window - period;
            let val = 1.0f32 / period as f32;
            for i in 0..period {
                let row = base + i;
                weights_t[row * specs.len() + col] = val;
            }
        }

        let mut raw = std::ptr::null_mut();
        let rc = unsafe {
            tca_create_plan(
                weights_t.as_ptr(),
                specs.len() as c_int,
                window as c_int,
                precision as c_int,
                &mut raw,
            )
        };
        if rc != 0 {
            return Err(TensorCoreError::from_last_error(rc, "tca_create_plan failed"));
        }
        let raw = NonNull::new(raw).ok_or_else(|| TensorCoreError::new(-10, "tca_create_plan returned null plan"))?;
        Ok(Self { raw, n_windows: specs.len(), window, precision })
    }

    pub fn n_windows(&self) -> usize { self.n_windows }
    pub fn max_period(&self) -> usize { self.window }
    pub fn valid_from(&self) -> usize { self.window.saturating_sub(1) }
    pub fn precision(&self) -> TcPrecision { self.precision }

    pub fn output_rows(&self, n_prices: usize) -> Result<usize, TensorCoreError> {
        if n_prices < self.window {
            return Err(TensorCoreError::new(
                -11,
                format!("prices length {} is smaller than required window {}", n_prices, self.window),
            ));
        }
        Ok(n_prices - self.window + 1)
    }

    /// Run against an existing device-resident price vector.
    ///
    /// # Safety
    /// - `device_prices` must point to at least `n_prices` device `f32` values.
    /// - `device_out` must point to at least `output_rows(n_prices) * n_windows()` device `f32` values.
    /// - `cuda_stream` must be either null or a valid `cudaStream_t` for the active device.
    pub unsafe fn run_on_device_raw(
        &self,
        device_prices: *const f32,
        n_prices: usize,
        device_out: *mut f32,
        chunk_rows: usize,
        cuda_stream: *mut c_void,
    ) -> Result<usize, TensorCoreError> {
        let mut out_rows = 0i32;
        let rc = tca_run_plan(
            self.raw.as_ptr(),
            device_prices,
            n_prices as c_int,
            chunk_rows as c_int,
            device_out,
            &mut out_rows,
            cuda_stream,
        );
        if rc != 0 {
            return Err(TensorCoreError::from_last_error(rc, "tca_run_plan failed"));
        }
        Ok(out_rows as usize)
    }
}

impl Drop for TensorCoreAlmaPlan {
    fn drop(&mut self) {
        unsafe {
            let _ = tca_destroy_plan(self.raw.as_ptr());
        }
    }
}

/// Minimal self-contained CUDA device buffer for the included benchmark binary.
pub struct CudaF32Buffer {
    ptr: NonNull<f32>,
    len: usize,
}

impl CudaF32Buffer {
    pub fn zeros(len: usize) -> Result<Self, TensorCoreError> {
        let mut ptr = std::ptr::null_mut();
        let rc = unsafe { tca_malloc_f32(len, &mut ptr) };
        if rc != 0 {
            return Err(TensorCoreError::from_last_error(rc, "tca_malloc_f32 failed"));
        }
        let ptr = NonNull::new(ptr).ok_or_else(|| TensorCoreError::new(-12, "tca_malloc_f32 returned null"))?;
        Ok(Self { ptr, len })
    }

    pub fn from_slice(src: &[f32]) -> Result<Self, TensorCoreError> {
        let buf = Self::zeros(src.len())?;
        buf.copy_from(src)?;
        Ok(buf)
    }

    pub fn len(&self) -> usize { self.len }
    pub fn as_device_ptr(&self) -> *const f32 { self.ptr.as_ptr() as *const f32 }
    pub fn as_mut_device_ptr(&mut self) -> *mut f32 { self.ptr.as_ptr() }

    pub fn copy_from(&self, src: &[f32]) -> Result<(), TensorCoreError> {
        if src.len() != self.len {
            return Err(TensorCoreError::new(-13, "host/device length mismatch in copy_from"));
        }
        let rc = unsafe { tca_memcpy_h2d_f32(self.ptr.as_ptr(), src.as_ptr(), src.len()) };
        if rc != 0 {
            return Err(TensorCoreError::from_last_error(rc, "tca_memcpy_h2d_f32 failed"));
        }
        Ok(())
    }

    pub fn copy_to(&self, dst: &mut [f32]) -> Result<(), TensorCoreError> {
        if dst.len() != self.len {
            return Err(TensorCoreError::new(-14, "host/device length mismatch in copy_to"));
        }
        let rc = unsafe { tca_memcpy_d2h_f32(dst.as_mut_ptr(), self.ptr.as_ptr(), dst.len()) };
        if rc != 0 {
            return Err(TensorCoreError::from_last_error(rc, "tca_memcpy_d2h_f32 failed"));
        }
        Ok(())
    }
}

impl Drop for CudaF32Buffer {
    fn drop(&mut self) {
        unsafe {
            let _ = tca_free_ptr(self.ptr.as_ptr() as *mut c_void);
        }
    }
}

pub fn device_synchronize() -> Result<(), TensorCoreError> {
    let rc = unsafe { tca_device_synchronize() };
    if rc != 0 {
        return Err(TensorCoreError::from_last_error(rc, "tca_device_synchronize failed"));
    }
    Ok(())
}
