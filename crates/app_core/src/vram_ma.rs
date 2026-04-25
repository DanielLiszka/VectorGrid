#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

pub(crate) use vector_ta::cuda::moving_averages::vram_ma::VramMaInputs;
pub(crate) use vector_ta::cuda::moving_averages::vram_ma::{
    supports_vram_kernel_ma, VramMaComputer,
};
