# VEMR stateful lifecycle example

This is a concrete example of turning `Vemr` from a mostly static indicator-alignment rule into a small trade lifecycle while keeping it optimizer-friendly.

## Why VEMR is a good candidate

Per your strategy catalog, `Vemr` is a standalone composite-family strategy that already owns its own execution path and uses a simple ingredient set: z-score, Bollinger bands, and ATR stop. That makes it a clean place to introduce more realistic behavior without needing a giant universal engine.

The implementation in `vemr_stateful_example.rs` keeps the same ingredients but changes the behavior:

- **Arm** on a real overshoot (`z <= -z_arm` or `z >= z_arm`)
- **Trigger** only after price and z-score reclaim from the extreme
- **Exit** on mean reversion, ATR stop, or max-hold
- **Reset** with cooldown

That is a strategy lifecycle, not just a stack of indicator booleans.

## Suggested repo insertion points

Primary owner from your catalog:

- `crates/app_core/src/vemr.rs`

Likely related surfaces to wire through, depending on your repo layout:

- `vemr_gpu_kernel.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Minimal Rust integration pattern

The cleanest low-risk rollout is to keep your existing VEMR as `Classic` and add a second mode.

```rust
// crates/app_core/src/vemr.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VemrSignalMode {
    Classic,
    ArmReclaim,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VemrRequest {
    // ...existing fields...
    pub signal_mode: Option<VemrSignalMode>,
    pub z_arm: Option<f64>,
    pub setup_expiry_bars: Option<usize>,
    pub cooldown_bars: Option<usize>,
    pub max_hold_bars: Option<usize>,
}
```

Then branch once at the strategy boundary, not inside a giant mixed-family loop:

```rust
let signal_mode = req.signal_mode.unwrap_or(VemrSignalMode::Classic);

let desired_positions = match signal_mode {
    VemrSignalMode::Classic => {
        build_vemr_desired_positions(
            // your current path
        )?
    }
    VemrSignalMode::ArmReclaim => {
        let cfg = VemrLifecycleConfig {
            signal_mode,
            side_mode: if req.long_only {
                VemrSideMode::LongOnly
            } else {
                VemrSideMode::Both
            },
            z_arm: req.z_arm.unwrap_or(req.z_entry * 1.25),
            z_entry: req.z_entry,
            z_exit: req.z_exit,
            atr_stop_mult: req.atr_stop_mult,
            require_band_touch: req.require_band_touch,
            confirm_bars: req.confirm_bars.max(1),
            setup_expiry_bars: req.setup_expiry_bars.unwrap_or(4),
            cooldown_bars: req.cooldown_bars.unwrap_or(2),
            max_hold_bars: req.max_hold_bars.unwrap_or(0),
            allow_flip: req.allow_flip,
            trade_on_next_bar: req.trade_on_next_bar,
        };

        build_vemr_stateful_desired_positions(
            cfg,
            close,
            &zscore,
            &bb_lower,
            &bb_mid,
            &bb_upper,
            &atr,
        )?
        .desired_positions
    }
};
```

## Front-end request shaping

Expose only a few new knobs. The point is not to explode the search space.

```ts
// web/src/services/optimizeRequestBuilder.ts

if (strategyKind === 'Vemr') {
  return {
    // ...existing fields...
    signalMode: form.signalMode ?? 'Classic',
    zArm: form.zArm ?? null,
    setupExpiryBars: form.setupExpiryBars ?? 4,
    cooldownBars: form.cooldownBars ?? 2,
    maxHoldBars: form.maxHoldBars ?? 0,
  };
}
```

Suggested defaults/caps:

- `signalMode`: `Classic | ArmReclaim`
- `zArm`: default `zEntry * 1.25`
- `setupExpiryBars`: `2..8`
- `cooldownBars`: `0..8`
- `maxHoldBars`: `0, 4, 8, 12, 16`

That stays grid-searchable.

## Why this stays GPU-friendly

Do **not** make one kernel handle every strategy shape.

For VEMR specifically, the stateful mode can still be cheap because each combo only needs a few extra thread-local fields:

- `u8 state`
- `u16 setup_age`
- `u16 trigger_streak`
- `u16 cooldown_left`
- `u16 bars_in_trade`
- `f32 stop_price`

The per-bar logic remains fixed and homogeneous inside the VEMR kernel. The important rule is to batch `Classic` and `ArmReclaim` separately rather than mixing them in the same hot inner loop.

## Suggested optimizer search space

A bounded search space is enough for this mode:

- `z_entry`: existing range
- `z_exit`: existing range
- `z_arm_mult`: `1.10, 1.20, 1.30`
- `confirm_bars`: `1, 2, 3`
- `setup_expiry_bars`: `2, 4, 6`
- `cooldown_bars`: `0, 2, 4`
- `max_hold_bars`: `0, 8, 16`
- `require_band_touch`: existing boolean
- `allow_flip`: existing boolean

Notice that almost all of this is counters and thresholds, not extra indicators.

## Validation/tests worth adding

Add at least these CPU tests before mirroring the mode into the GPU kernel:

1. **arm then reclaim**: oversold event does not enter until reclaim happens
2. **expiry**: setup cancels if no reclaim in time
3. **cooldown**: no immediate re-entry after exit
4. **trade_on_next_bar**: shifted desired-position semantics still match your backtester
5. **allow_flip**: long exits to armed-short when opposite extreme appears
6. **edge parity**: threshold-adjacent values around `z_arm`, `z_entry`, `z_exit`

If you later add GPU support for this mode, run parity on raw desired positions first, then on full metrics.

## Practical rollout order

1. Add `ArmReclaim` to the CPU reference path in `crates/app_core/src/vemr.rs`
2. Keep `Classic` as the default so nothing breaks
3. Wire new fields through request shaping
4. Add unit tests for the state machine
5. Only then decide whether to mirror `ArmReclaim` into `vemr_gpu_kernel.rs`

That gives you a real strategy upgrade without forcing an all-at-once refactor.
