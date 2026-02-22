use std::mem::{self, MaybeUninit};

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[inline(always)]
pub(crate) fn alloc_uninit_copy<T: Copy>(len: usize) -> Vec<T> {
    let mut buf: Vec<MaybeUninit<T>> = Vec::with_capacity(len);
    unsafe {
        // SAFETY: callers must ensure all elements are fully written before any read.
        buf.set_len(len);
    }
    let ptr = buf.as_mut_ptr() as *mut T;
    let cap = buf.capacity();
    mem::forget(buf);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

#[inline(always)]
pub(crate) fn alloc_f64_with_prefix_uninit_tail(len: usize, warm: usize, prefix: f64) -> Vec<f64> {
    let warm = warm.min(len);
    let mut buf: Vec<MaybeUninit<f64>> = Vec::with_capacity(len);

    #[cfg(not(debug_assertions))]
    unsafe {
        // SAFETY: the tail is intentionally left uninitialized for overwrite-heavy paths.
        buf.set_len(len);
    }

    #[cfg(debug_assertions)]
    {
        for _ in 0..len {
            buf.push(MaybeUninit::new(f64::from_bits(0x1111_1111_1111_1111)));
        }
    }

    for i in 0..warm {
        buf[i].write(prefix);
    }

    let ptr = buf.as_mut_ptr() as *mut f64;
    let cap = buf.capacity();
    mem::forget(buf);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}
