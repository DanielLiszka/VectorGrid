#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]

pub use vector_ta::cuda::moving_averages::vram_ma::VramMaInputs;
pub use vector_ta::cuda::moving_averages::vram_ma::{supports_vram_kernel_ma, VramMaComputer};
