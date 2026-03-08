# ALMA Unification Matrix (2026-02-28)

Source: `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel`

| Case | GpuKernel ms | GpuHybridKernel ms | GpuOnly ms | CpuOnly ms | GpuKernel vs CpuOnly | GpuKernel vs GpuOnly |
|---|---:|---:|---:|---:|---:|---:|
| large_58300pairs | 108.82 | 109.65 | 3,525.89 | 3,389.68 | 31.15x | 32.40x |
| large_batched_1gb_58300pairs | 207.91 | - | - | - | - | - |
| medium_14150pairs | 101.08 | - | 1,074.75 | 868.63 | 8.59x | 10.63x |
| small_1590pairs | 100.10 | 99.94 | 322.84 | 124.84 | 1.25x | 3.23x |
| xlarge_370750pairs | 387.91 | 385.26 | - | - | - | - |
| xlarge_batched_1gb_370750pairs | 1,356.50 | 1,350.33 | - | - | - | - |
| xxlarge_1491500pairs | 1,318.01 | 1,321.85 | - | - | - | - |
| xxlarge_batched_1gb_1491500pairs | 6,003.25 | 5,984.60 | - | - | - | - |

Notes:
- Missing cells mean that backend/case combination is not part of that benchmark group.
- Use this table for rollout comparisons; keep interpreting with system load context.
