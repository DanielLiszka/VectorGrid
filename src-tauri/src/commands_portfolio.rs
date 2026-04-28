#[tauri::command]
pub async fn analyze_portfolio_feasibility(
    req: ta_app_core::PortfolioFeasibilityRequestV1,
) -> Result<ta_app_core::PortfolioFeasibilityResultV1, String> {
    Ok(ta_app_core::portfolio_feasibility_from_artifacts(req))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::analyze_portfolio_feasibility;

    #[tokio::test]
    async fn analyze_portfolio_feasibility_is_read_only_and_blocks_without_replay() {
        let result = analyze_portfolio_feasibility(ta_app_core::PortfolioFeasibilityRequestV1 {
            source_artifacts: vec![json!({
                "artifact_kind": "validation",
                "artifact_hash": "hash-1",
                "candidates": [{
                    "candidate_id": "candidate-1",
                    "validation_passed": true,
                    "symbol": "AAPL"
                }]
            })],
            max_candidates: 10,
            require_approved_validation: true,
            require_replay_matrix: false,
            require_source_provenance: false,
        })
        .await
        .expect("portfolio feasibility");

        assert_eq!(
            result.status,
            ta_app_core::PortfolioEvidenceStatus::InsufficientCovarianceEvidence
        );
        assert_eq!(result.eligible_candidates.len(), 1);
        assert!(result
            .next_required_step
            .contains("co-timed return/exposure replay matrix"));
    }
}
