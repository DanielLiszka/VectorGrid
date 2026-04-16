use serde::Serialize;
use vector_ta::utilities::data_loader::Candles;

/// Thin progress payload forwarded to an optional [`ProgressSink`].
///
/// This wire shape is public and IPC-adjacent. Callers own ordering, monotonicity,
/// and the `processed_pairs <= total_pairs` discipline.
#[derive(Clone, Debug, Serialize)]
pub struct DoubleMaProgressPayload {
    pub processed_pairs: usize,
    pub total_pairs: usize,
    pub phase: &'static str,
}

/// Optional feedback emitted by the VRAM prefetch capability hook.
///
/// This is a cold-path coordination surface rather than the primary progress stream.
#[derive(Clone, Debug)]
pub enum DoubleMaCampaignVramPrefetchFeedback {
    Staged { stage_wall_ms: f64 },
    AlreadyResident { stage_wall_ms: f64 },
    Warning { note: String, stage_wall_ms: f64 },
    Fatal { note: String, stage_wall_ms: f64 },
}

/// Consumer for double-MA progress events and optional VRAM prefetch coordination.
///
/// `emit_double_ma_progress()` is the hot path. Prefetch request/feedback hooks are
/// synchronous optional capabilities and are not required for basic progress reporting.
pub trait ProgressSink: Send + Sync {
    fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload);

    /// Exposes a synchronous capability hook for campaign VRAM prefetch setup.
    ///
    /// Implementations may choose to do nothing. The callback is invoked directly and is
    /// not isolated from blocking or panics.
    fn with_double_ma_campaign_vram_prefetch_request(
        &self,
        _f: &mut dyn FnMut(u32, &Candles, &str, &str, &str),
    ) {
    }

    /// Receives cold-path feedback about campaign VRAM prefetch staging work.
    fn on_double_ma_campaign_vram_prefetch_feedback(
        &self,
        _feedback: DoubleMaCampaignVramPrefetchFeedback,
    ) {
    }
}

/// Forwards a progress payload to `sink` when one is present.
///
/// `None` is a silent no-op. This helper forwards `processed_pairs`, `total_pairs`,
/// and `phase` unchanged and does not validate ordering, monotonicity, or panic behavior.
pub fn emit_double_ma_progress(
    sink: Option<&dyn ProgressSink>,
    processed_pairs: usize,
    total_pairs: usize,
    phase: &'static str,
) {
    let Some(sink) = sink else { return };
    sink.emit_double_ma_progress(DoubleMaProgressPayload {
        processed_pairs,
        total_pairs,
        phase,
    });
}

#[cfg(test)]
mod tests {
    use super::{emit_double_ma_progress, DoubleMaProgressPayload, ProgressSink};
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        payloads: Mutex<Vec<(usize, usize, &'static str)>>,
    }

    impl ProgressSink for RecordingSink {
        fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload) {
            self.payloads.lock().expect("payload lock").push((
                payload.processed_pairs,
                payload.total_pairs,
                payload.phase,
            ));
        }
    }

    #[test]
    fn emit_double_ma_progress_is_noop_without_sink() {
        emit_double_ma_progress(None, 3, 10, "score");
    }

    #[test]
    fn emit_double_ma_progress_forwards_payload_unchanged() {
        let sink = RecordingSink::default();

        emit_double_ma_progress(Some(&sink), 7, 19, "prefetch");

        let payloads = sink.payloads.lock().expect("payload lock");
        assert_eq!(payloads.as_slice(), &[(7, 19, "prefetch")]);
    }
}
