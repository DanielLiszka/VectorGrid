// Suggested insertion point inside the validation / scenario-evaluation lane once you have
// batched entry bars and stop/target prices. This keeps RT-core work out of the main MA-prep
// path unless explicitly requested.

use crate::rtcore_exit_scan::{maybe_evaluate_exit_bars_rtcore, ExitHit};

fn resolve_exit_hits_with_rtcore(
    lows_f32: &[f32],
    highs_f32: &[f32],
    entry_bars: &[i32],
    stop_prices: &[f32],
    target_prices: &[f32],
) -> crate::rtcore_exit_scan::Result<Option<Vec<ExitHit>>> {
    maybe_evaluate_exit_bars_rtcore(
        lows_f32,
        highs_f32,
        entry_bars,
        stop_prices,
        target_prices,
    )
}

// Typical downstream policy:
// - if both stop_hit_bar and target_hit_bar are Some(same_bar), defer to your existing intrabar rule
// - if both are Some(different bars), choose the smaller bar index
// - if one is Some and the other None, take the one that exists
// - if both are None, the trade survives to your fallback horizon / end-of-window
