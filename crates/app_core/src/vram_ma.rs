#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

//! Internal CUDA bridge over `vector_ta` VRAM moving-average helpers.

pub(crate) use vector_ta::cuda::moving_averages::vram_ma::VramMaInputs;
pub(crate) use vector_ta::cuda::moving_averages::vram_ma::{
    supports_vram_kernel_ma, VramMaComputer,
};
