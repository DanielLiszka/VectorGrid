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

__device__ inline void options_expression_accumulate_lane(
    unsigned int event_index,
    unsigned int candidate_slot_stride,
    unsigned int summary_stride,
    unsigned int candidate_offset,
    unsigned int summary_offset,
    const unsigned char* outcome_valid,
    const float* outcome_pnl,
    const float* outcome_premium,
    int* valid,
    int* wins,
    float* total_pnl,
    float* total_premium,
    float* sum_return_on_premium,
    float* sumsq_return_on_premium
) {
    const unsigned int candidate_base = event_index * candidate_slot_stride;
    const unsigned int summary_index =
        (candidate_base + candidate_offset) * summary_stride + summary_offset;
    if (outcome_valid[summary_index] != 0) {
        const float pnl = outcome_pnl[summary_index];
        const float premium = outcome_premium[summary_index];
        const float return_on_premium = pnl / premium;
        *total_pnl += pnl;
        *total_premium += premium;
        *sum_return_on_premium += return_on_premium;
        *sumsq_return_on_premium += return_on_premium * return_on_premium;
        *valid += 1;
        if (pnl > 0.0f) {
            *wins += 1;
        }
    }
}

__device__ inline double options_expression_lane_utility(
    unsigned int split_event_count,
    int valid,
    float total_pnl,
    float total_premium
) {
    if (split_event_count == 0 || valid <= 0 || total_premium <= 0.0f) {
        return 0.0;
    }
    const double coverage =
        static_cast<double>(valid) / static_cast<double>(split_event_count);
    const double expectancy_per_premium =
        static_cast<double>(total_pnl) / static_cast<double>(total_premium);
    return coverage * expectancy_per_premium;
}


extern "C" __global__ void options_expression_score_search_holdout_stress_f64(
    unsigned int search_end,
    unsigned int holdout_end,
    unsigned int baseline_candidate_slot_stride,
    unsigned int baseline_summary_stride,
    const unsigned int* baseline_tuple_candidate_offsets,
    const unsigned int* baseline_tuple_summary_offsets,
    const unsigned char* baseline_outcome_valid,
    const float* baseline_outcome_pnl,
    const float* baseline_outcome_premium,
    unsigned int conservative_candidate_slot_stride,
    unsigned int conservative_summary_stride,
    const unsigned int* conservative_tuple_candidate_offsets,
    const unsigned int* conservative_tuple_summary_offsets,
    const unsigned char* conservative_outcome_valid,
    const float* conservative_outcome_pnl,
    const float* conservative_outcome_premium,
    unsigned int harsher_candidate_slot_stride,
    unsigned int harsher_summary_stride,
    const unsigned int* harsher_tuple_candidate_offsets,
    const unsigned int* harsher_tuple_summary_offsets,
    const unsigned char* harsher_outcome_valid,
    const float* harsher_outcome_pnl,
    const float* harsher_outcome_premium,
    unsigned int tuple_count,
    int* out_search_valid,
    int* out_search_wins,
    float* out_search_total_pnl,
    float* out_search_total_premium,
    float* out_search_sum_return_on_premium,
    float* out_search_sumsq_return_on_premium,
    int* out_holdout_valid,
    int* out_holdout_wins,
    float* out_holdout_total_pnl,
    float* out_holdout_total_premium,
    float* out_holdout_sum_return_on_premium,
    float* out_holdout_sumsq_return_on_premium,
    int* out_conservative_valid,
    int* out_conservative_wins,
    float* out_conservative_total_pnl,
    float* out_conservative_total_premium,
    float* out_conservative_sum_return_on_premium,
    float* out_conservative_sumsq_return_on_premium,
    int* out_harsher_valid,
    int* out_harsher_wins,
    float* out_harsher_total_pnl,
    float* out_harsher_total_premium,
    float* out_harsher_sum_return_on_premium,
    float* out_harsher_sumsq_return_on_premium,
    double* out_robust_score
) {
    const unsigned int tuple_index = blockIdx.x * blockDim.x + threadIdx.x;
    if (tuple_index >= tuple_count) {
        return;
    }

    const unsigned int baseline_candidate_offset = baseline_tuple_candidate_offsets[tuple_index];
    const unsigned int baseline_summary_offset = baseline_tuple_summary_offsets[tuple_index];
    const unsigned int conservative_candidate_offset =
        conservative_tuple_candidate_offsets[tuple_index];
    const unsigned int conservative_summary_offset =
        conservative_tuple_summary_offsets[tuple_index];
    const unsigned int harsher_candidate_offset = harsher_tuple_candidate_offsets[tuple_index];
    const unsigned int harsher_summary_offset = harsher_tuple_summary_offsets[tuple_index];

    int search_valid = 0;
    int search_wins = 0;
    float search_total_pnl = 0.0f;
    float search_total_premium = 0.0f;
    float search_sum_return_on_premium = 0.0f;
    float search_sumsq_return_on_premium = 0.0f;

    int holdout_valid = 0;
    int holdout_wins = 0;
    float holdout_total_pnl = 0.0f;
    float holdout_total_premium = 0.0f;
    float holdout_sum_return_on_premium = 0.0f;
    float holdout_sumsq_return_on_premium = 0.0f;

    int conservative_valid = 0;
    int conservative_wins = 0;
    float conservative_total_pnl = 0.0f;
    float conservative_total_premium = 0.0f;
    float conservative_sum_return_on_premium = 0.0f;
    float conservative_sumsq_return_on_premium = 0.0f;

    int harsher_valid = 0;
    int harsher_wins = 0;
    float harsher_total_pnl = 0.0f;
    float harsher_total_premium = 0.0f;
    float harsher_sum_return_on_premium = 0.0f;
    float harsher_sumsq_return_on_premium = 0.0f;

    for (unsigned int event_index = 0; event_index < holdout_end; ++event_index) {
        if (event_index < search_end) {
            options_expression_accumulate_lane(
                event_index,
                baseline_candidate_slot_stride,
                baseline_summary_stride,
                baseline_candidate_offset,
                baseline_summary_offset,
                baseline_outcome_valid,
                baseline_outcome_pnl,
                baseline_outcome_premium,
                &search_valid,
                &search_wins,
                &search_total_pnl,
                &search_total_premium,
                &search_sum_return_on_premium,
                &search_sumsq_return_on_premium
            );
        } else {
            options_expression_accumulate_lane(
                event_index,
                baseline_candidate_slot_stride,
                baseline_summary_stride,
                baseline_candidate_offset,
                baseline_summary_offset,
                baseline_outcome_valid,
                baseline_outcome_pnl,
                baseline_outcome_premium,
                &holdout_valid,
                &holdout_wins,
                &holdout_total_pnl,
                &holdout_total_premium,
                &holdout_sum_return_on_premium,
                &holdout_sumsq_return_on_premium
            );
            options_expression_accumulate_lane(
                event_index,
                conservative_candidate_slot_stride,
                conservative_summary_stride,
                conservative_candidate_offset,
                conservative_summary_offset,
                conservative_outcome_valid,
                conservative_outcome_pnl,
                conservative_outcome_premium,
                &conservative_valid,
                &conservative_wins,
                &conservative_total_pnl,
                &conservative_total_premium,
                &conservative_sum_return_on_premium,
                &conservative_sumsq_return_on_premium
            );
            options_expression_accumulate_lane(
                event_index,
                harsher_candidate_slot_stride,
                harsher_summary_stride,
                harsher_candidate_offset,
                harsher_summary_offset,
                harsher_outcome_valid,
                harsher_outcome_pnl,
                harsher_outcome_premium,
                &harsher_valid,
                &harsher_wins,
                &harsher_total_pnl,
                &harsher_total_premium,
                &harsher_sum_return_on_premium,
                &harsher_sumsq_return_on_premium
            );
        }
    }

    out_search_valid[tuple_index] = search_valid;
    out_search_wins[tuple_index] = search_wins;
    out_search_total_pnl[tuple_index] = search_total_pnl;
    out_search_total_premium[tuple_index] = search_total_premium;
    out_search_sum_return_on_premium[tuple_index] = search_sum_return_on_premium;
    out_search_sumsq_return_on_premium[tuple_index] = search_sumsq_return_on_premium;

    out_holdout_valid[tuple_index] = holdout_valid;
    out_holdout_wins[tuple_index] = holdout_wins;
    out_holdout_total_pnl[tuple_index] = holdout_total_pnl;
    out_holdout_total_premium[tuple_index] = holdout_total_premium;
    out_holdout_sum_return_on_premium[tuple_index] = holdout_sum_return_on_premium;
    out_holdout_sumsq_return_on_premium[tuple_index] = holdout_sumsq_return_on_premium;

    out_conservative_valid[tuple_index] = conservative_valid;
    out_conservative_wins[tuple_index] = conservative_wins;
    out_conservative_total_pnl[tuple_index] = conservative_total_pnl;
    out_conservative_total_premium[tuple_index] = conservative_total_premium;
    out_conservative_sum_return_on_premium[tuple_index] = conservative_sum_return_on_premium;
    out_conservative_sumsq_return_on_premium[tuple_index] = conservative_sumsq_return_on_premium;

    out_harsher_valid[tuple_index] = harsher_valid;
    out_harsher_wins[tuple_index] = harsher_wins;
    out_harsher_total_pnl[tuple_index] = harsher_total_pnl;
    out_harsher_total_premium[tuple_index] = harsher_total_premium;
    out_harsher_sum_return_on_premium[tuple_index] = harsher_sum_return_on_premium;
    out_harsher_sumsq_return_on_premium[tuple_index] = harsher_sumsq_return_on_premium;

    const unsigned int holdout_count =
        holdout_end > search_end ? (holdout_end - search_end) : 0;
    const double search_score = options_expression_lane_utility(
        search_end,
        search_valid,
        search_total_pnl,
        search_total_premium
    );
    double holdout_score = search_score;
    double conservative_score = search_score;
    double harsher_score = search_score;
    if (holdout_count > 0) {
        holdout_score = options_expression_lane_utility(
            holdout_count,
            holdout_valid,
            holdout_total_pnl,
            holdout_total_premium
        );
        conservative_score = options_expression_lane_utility(
            holdout_count,
            conservative_valid,
            conservative_total_pnl,
            conservative_total_premium
        );
        harsher_score = options_expression_lane_utility(
            holdout_count,
            harsher_valid,
            harsher_total_pnl,
            harsher_total_premium
        );
    }
    out_robust_score[tuple_index] =
        0.45 * search_score +
        0.25 * holdout_score +
        0.15 * conservative_score +
        0.15 * harsher_score;
}

