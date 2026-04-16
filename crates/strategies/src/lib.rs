//! Shared strategy definitions and logic used by the optimizer and app-core crates.
//!
//! Modules:
//! - [`double_ma`]: Double-MA strategy definitions, metrics, and batch execution helpers.
//! - [`composite_cpu`]: CPU-side composite strategy logic and signal-mode support.
//! - [`records`]: Shared trade-record structures and helpers.

pub mod composite_cpu;
pub mod double_ma;
pub mod records;
