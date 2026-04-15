use serde::Serialize;
use vector_ta::utilities::data_loader::Candles;

#[derive(Clone, Debug, Serialize)]
pub struct DoubleMaProgressPayload {
    pub processed_pairs: usize,
    pub total_pairs: usize,
    pub phase: &'static str,
}

#[derive(Clone, Debug)]
pub enum DoubleMaCampaignVramPrefetchFeedback {
    Staged { stage_wall_ms: f64 },
    AlreadyResident { stage_wall_ms: f64 },
    Warning { note: String, stage_wall_ms: f64 },
    Fatal { note: String, stage_wall_ms: f64 },
}

pub trait ProgressSink: Send + Sync {
    fn emit_double_ma_progress(&self, payload: DoubleMaProgressPayload);

    fn with_double_ma_campaign_vram_prefetch_request(
        &self,
        _f: &mut dyn FnMut(u32, &Candles, &str, &str, &str),
    ) {
    }

    fn on_double_ma_campaign_vram_prefetch_feedback(
        &self,
        _feedback: DoubleMaCampaignVramPrefetchFeedback,
    ) {
    }
}

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
