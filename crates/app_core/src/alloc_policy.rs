use std::mem::{self, MaybeUninit};

#[cfg_attr(not(debug_assertions), allow(dead_code))]
const DEBUG_F64_POISON_BITS: u64 = 0x1111_1111_1111_1111;
#[cfg_attr(not(debug_assertions), allow(dead_code))]
const DEBUG_F64_MATRIX_POISON_BITS: u64 = 0x3333_3333_3333_3333;
#[cfg_attr(not(debug_assertions), allow(dead_code))]
const DEBUG_F32_POISON_BITS: u32 = 0x1111_1111;
#[cfg_attr(not(debug_assertions), allow(dead_code))]
const DEBUG_I32_POISON_BITS: i32 = 0x1111_1111u32 as i32;
#[cfg_attr(not(debug_assertions), allow(dead_code))]
const DEBUG_F32_MATRIX_POISON_BITS: u32 = 0x3333_3333;

#[inline(always)]
fn assume_init_vec<T>(mut buf: Vec<MaybeUninit<T>>) -> Vec<T> {
    let ptr = buf.as_mut_ptr() as *mut T;
    let cap = buf.capacity();
    let len = buf.len();
    mem::forget(buf);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

#[cfg_attr(any(debug_assertions, test), allow(dead_code))]
#[inline(always)]
fn reserve_for_uninit_resize<T>(buf: &mut Vec<T>, len: usize) {
    if len > buf.len() {
        buf.reserve(len - buf.len());
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[inline(always)]
pub(crate) fn alloc_uninit_copy<T: Copy>(len: usize) -> Vec<T> {
    let mut buf: Vec<MaybeUninit<T>> = Vec::with_capacity(len);
    unsafe {
        // SAFETY: callers must ensure all elements are fully written before any read.
        buf.set_len(len);
    }
    assume_init_vec(buf)
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
            buf.push(MaybeUninit::new(f64::from_bits(DEBUG_F64_POISON_BITS)));
        }
    }

    for i in 0..warm {
        buf[i].write(prefix);
    }

    assume_init_vec(buf)
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline(always)]
pub(crate) fn alloc_f32_with_prefix_uninit_tail(len: usize, warm: usize, prefix: f32) -> Vec<f32> {
    let warm = warm.min(len);
    let mut buf: Vec<MaybeUninit<f32>> = Vec::with_capacity(len);

    #[cfg(not(debug_assertions))]
    unsafe {
        // SAFETY: the tail is intentionally left uninitialized for overwrite-heavy paths.
        buf.set_len(len);
    }

    #[cfg(debug_assertions)]
    {
        for _ in 0..len {
            buf.push(MaybeUninit::new(f32::from_bits(DEBUG_F32_POISON_BITS)));
        }
    }

    for i in 0..warm {
        buf[i].write(prefix);
    }

    assume_init_vec(buf)
}

#[inline(always)]
pub(crate) fn alloc_uninit_f64(len: usize) -> Vec<f64> {
    #[cfg(not(debug_assertions))]
    {
        let mut buf: Vec<MaybeUninit<f64>> = Vec::with_capacity(len);
        unsafe {
            // SAFETY: callers must fully initialize each element before any read.
            buf.set_len(len);
        }
        assume_init_vec(buf)
    }

    #[cfg(debug_assertions)]
    {
        vec![f64::from_bits(DEBUG_F64_POISON_BITS); len]
    }
}

#[inline(always)]
pub(crate) fn alloc_uninit_f32(len: usize) -> Vec<f32> {
    #[cfg(not(debug_assertions))]
    {
        let mut buf: Vec<MaybeUninit<f32>> = Vec::with_capacity(len);
        unsafe {
            // SAFETY: callers must fully initialize each element before any read.
            buf.set_len(len);
        }
        assume_init_vec(buf)
    }

    #[cfg(debug_assertions)]
    {
        vec![f32::from_bits(DEBUG_F32_POISON_BITS); len]
    }
}

#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
#[inline(always)]
pub(crate) fn resize_uninit_f32(buf: &mut Vec<f32>, len: usize) {
    #[cfg(not(debug_assertions))]
    {
        reserve_for_uninit_resize(buf, len);
        unsafe {
            // SAFETY: callers must fully initialize each element before any read.
            buf.set_len(len);
        }
    }

    #[cfg(debug_assertions)]
    {
        buf.resize(len, f32::from_bits(DEBUG_F32_POISON_BITS));
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline(always)]
pub(crate) fn resize_uninit_f64(buf: &mut Vec<f64>, len: usize) {
    #[cfg(not(debug_assertions))]
    {
        reserve_for_uninit_resize(buf, len);
        unsafe {
            // SAFETY: callers must fully initialize each element before any read.
            buf.set_len(len);
        }
    }

    #[cfg(debug_assertions)]
    {
        buf.resize(len, f64::from_bits(DEBUG_F64_POISON_BITS));
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline(always)]
pub(crate) fn resize_uninit_i32(buf: &mut Vec<i32>, len: usize) {
    #[cfg(not(debug_assertions))]
    {
        reserve_for_uninit_resize(buf, len);
        unsafe {
            // SAFETY: callers must fully initialize each element before any read.
            buf.set_len(len);
        }
    }

    #[cfg(debug_assertions)]
    {
        buf.resize(len, DEBUG_I32_POISON_BITS);
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline]
pub(crate) fn make_uninit_matrix_f64(rows: usize, cols: usize) -> Vec<MaybeUninit<f64>> {
    let total = rows
        .checked_mul(cols)
        .expect("rows * cols overflowed usize");

    let mut v: Vec<MaybeUninit<f64>> = Vec::new();
    v.try_reserve_exact(total)
        .expect("OOM in make_uninit_matrix_f64");

    #[cfg(not(debug_assertions))]
    unsafe {
        // SAFETY: callers must initialize each cell before reading it.
        v.set_len(total);
    }

    #[cfg(debug_assertions)]
    {
        for _ in 0..total {
            v.push(MaybeUninit::new(f64::from_bits(
                DEBUG_F64_MATRIX_POISON_BITS,
            )));
        }
    }

    v
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline]
pub(crate) fn make_uninit_matrix_f32(rows: usize, cols: usize) -> Vec<MaybeUninit<f32>> {
    let total = rows
        .checked_mul(cols)
        .expect("rows * cols overflowed usize");

    let mut v: Vec<MaybeUninit<f32>> = Vec::new();
    v.try_reserve_exact(total)
        .expect("OOM in make_uninit_matrix_f32");

    #[cfg(not(debug_assertions))]
    unsafe {
        // SAFETY: callers must initialize each cell before reading it.
        v.set_len(total);
    }

    #[cfg(debug_assertions)]
    {
        for _ in 0..total {
            v.push(MaybeUninit::new(f32::from_bits(
                DEBUG_F32_MATRIX_POISON_BITS,
            )));
        }
    }

    v
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline]
pub(crate) fn init_matrix_prefixes_f64(
    buf: &mut [MaybeUninit<f64>],
    cols: usize,
    warm_prefixes: &[usize],
    prefix: f64,
) {
    assert!(
        cols != 0 && buf.len() % cols == 0,
        "`buf` length must be a multiple of `cols`"
    );
    let rows = buf.len() / cols;
    assert_eq!(
        rows,
        warm_prefixes.len(),
        "`warm_prefixes` length must equal number of rows"
    );

    #[cfg(debug_assertions)]
    {
        for cell in buf.iter_mut() {
            cell.write(f64::from_bits(DEBUG_F64_MATRIX_POISON_BITS));
        }
    }

    for (row, &warm) in buf.chunks_exact_mut(cols).zip(warm_prefixes.iter()) {
        assert!(warm <= cols, "warm prefix exceeds row width");
        for cell in &mut row[..warm] {
            cell.write(prefix);
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[inline]
pub(crate) fn init_matrix_prefixes_f32(
    buf: &mut [MaybeUninit<f32>],
    cols: usize,
    warm_prefixes: &[usize],
    prefix: f32,
) {
    assert!(
        cols != 0 && buf.len() % cols == 0,
        "`buf` length must be a multiple of `cols`"
    );
    let rows = buf.len() / cols;
    assert_eq!(
        rows,
        warm_prefixes.len(),
        "`warm_prefixes` length must equal number of rows"
    );

    #[cfg(debug_assertions)]
    {
        for cell in buf.iter_mut() {
            cell.write(f32::from_bits(DEBUG_F32_MATRIX_POISON_BITS));
        }
    }

    for (row, &warm) in buf.chunks_exact_mut(cols).zip(warm_prefixes.iter()) {
        assert!(warm <= cols, "warm prefix exceeds row width");
        for cell in &mut row[..warm] {
            cell.write(prefix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        alloc_f32_with_prefix_uninit_tail, alloc_f64_with_prefix_uninit_tail, alloc_uninit_f32,
        alloc_uninit_f64, init_matrix_prefixes_f32, init_matrix_prefixes_f64,
        make_uninit_matrix_f32, make_uninit_matrix_f64, reserve_for_uninit_resize,
        resize_uninit_f32, resize_uninit_f64, resize_uninit_i32,
    };

    #[test]
    fn alloc_f64_prefix_initializes_requested_prefix_only() {
        let mut values = alloc_f64_with_prefix_uninit_tail(6, 2, f64::NAN);
        values[2..].fill(5.0);

        assert!(values[0].is_nan());
        assert!(values[1].is_nan());
        assert_eq!(&values[2..], &[5.0, 5.0, 5.0, 5.0]);
    }

    #[test]
    fn alloc_f32_prefix_initializes_requested_prefix_only() {
        let mut values = alloc_f32_with_prefix_uninit_tail(5, 3, f32::NAN);
        values[3..].fill(7.0);

        assert!(values[0].is_nan());
        assert!(values[1].is_nan());
        assert!(values[2].is_nan());
        assert_eq!(&values[3..], &[7.0, 7.0]);
    }

    #[test]
    fn alloc_uninit_scalars_can_be_fully_overwritten() {
        let mut values_f64 = alloc_uninit_f64(4);
        values_f64.copy_from_slice(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(values_f64, vec![1.0, 2.0, 3.0, 4.0]);

        let mut values_f32 = alloc_uninit_f32(3);
        values_f32.copy_from_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(values_f32, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn resize_uninit_buffers_can_be_reused_after_overwrite() {
        let mut values_f32 = vec![1.0, 2.0];
        resize_uninit_f32(&mut values_f32, 4);
        values_f32.copy_from_slice(&[3.0, 4.0, 5.0, 6.0]);
        assert_eq!(values_f32, vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn reserve_for_uninit_resize_grows_from_len_not_capacity() {
        let mut values = Vec::with_capacity(4);
        values.extend_from_slice(&[1.0f32, 2.0]);

        reserve_for_uninit_resize(&mut values, 6);

        assert!(values.capacity() >= 6);
    }

    #[test]
    fn resize_uninit_i32_buffers_can_be_reused_after_overwrite() {
        let mut values = vec![1, 2];
        resize_uninit_i32(&mut values, 5);
        values.copy_from_slice(&[3, 4, 5, 6, 7]);
        assert_eq!(values, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn resize_uninit_f64_buffers_can_be_reused_after_overwrite() {
        let mut values = vec![1.0, 2.0];
        resize_uninit_f64(&mut values, 5);
        values.copy_from_slice(&[3.0, 4.0, 5.0, 6.0, 7.0]);
        assert_eq!(values, vec![3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn matrix_prefix_initializers_only_touch_requested_prefixes() {
        let mut f64_buf = make_uninit_matrix_f64(2, 4);
        init_matrix_prefixes_f64(&mut f64_buf, 4, &[1, 3], f64::NAN);
        for cell in &mut f64_buf[1..4] {
            cell.write(9.0);
        }
        f64_buf[4 + 3].write(8.0);
        let f64_values: Vec<f64> = f64_buf
            .into_iter()
            .map(|cell| unsafe { cell.assume_init() })
            .collect();
        assert!(f64_values[0].is_nan());
        assert_eq!(&f64_values[1..4], &[9.0, 9.0, 9.0]);
        assert!(f64_values[4].is_nan());
        assert!(f64_values[5].is_nan());
        assert!(f64_values[6].is_nan());
        assert_eq!(f64_values[7], 8.0);

        let mut f32_buf = make_uninit_matrix_f32(1, 3);
        init_matrix_prefixes_f32(&mut f32_buf, 3, &[2], f32::NAN);
        f32_buf[2].write(4.0);
        let f32_values: Vec<f32> = f32_buf
            .into_iter()
            .map(|cell| unsafe { cell.assume_init() })
            .collect();
        assert!(f32_values[0].is_nan());
        assert!(f32_values[1].is_nan());
        assert_eq!(f32_values[2], 4.0);
    }
}
