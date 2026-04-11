extern "C" __global__ void options_expression_score_split_f64(
    unsigned int event_start,
    unsigned int event_end,
    unsigned int candidate_slot_stride,
    unsigned int summary_stride,
    const unsigned int* tuple_candidate_offsets,
    const unsigned int* tuple_summary_offsets,
    const unsigned char* outcome_valid,
    const float* outcome_pnl,
    const float* outcome_premium,
    unsigned int tuple_count,
    int* out_valid,
    int* out_wins,
    float* out_total_pnl,
    float* out_total_premium,
    float* out_sum_return_on_premium,
    float* out_sumsq_return_on_premium
) {
    const unsigned int tuple_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (tuple_index >= tuple_count) {
        return;
    }

    const unsigned int candidate_offset = tuple_candidate_offsets[tuple_index];
    const unsigned int summary_offset = tuple_summary_offsets[tuple_index];

    int valid = 0;
    int wins = 0;
    float total_pnl = 0.0f;
    float total_premium = 0.0f;
    float sum_return_on_premium = 0.0f;
    float sumsq_return_on_premium = 0.0f;

    for (unsigned int event_index = event_start; event_index < event_end; ++event_index) {
        const unsigned int candidate_base = event_index * candidate_slot_stride;
        const unsigned int summary_index =
            (candidate_base + candidate_offset) * summary_stride + summary_offset;
        if (outcome_valid[summary_index] != 0) {
            const float pnl = outcome_pnl[summary_index];
            const float premium = outcome_premium[summary_index];
            const float return_on_premium = pnl / premium;
            total_pnl += pnl;
            total_premium += premium;
            sum_return_on_premium += return_on_premium;
            sumsq_return_on_premium += return_on_premium * return_on_premium;
            valid += 1;
            if (pnl > 0.0f) {
                wins += 1;
            }
        }
    }

    out_valid[tuple_index] = valid;
    out_wins[tuple_index] = wins;
    out_total_pnl[tuple_index] = total_pnl;
    out_total_premium[tuple_index] = total_premium;
    out_sum_return_on_premium[tuple_index] = sum_return_on_premium;
    out_sumsq_return_on_premium[tuple_index] = sumsq_return_on_premium;
}
