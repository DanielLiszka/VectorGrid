use crate::validation_shared::{
    ValidationArtifactContext, ValidationPriceAdjustmentState, ValidationPromotionEligibilityState,
    ValidationPromotionEligibilityV1, ValidationRealismEvidenceStatus, ValidationRealismEvidenceV1,
};

fn evidence_is_declared(status: ValidationRealismEvidenceStatus) -> bool {
    !matches!(status, ValidationRealismEvidenceStatus::Unknown)
}

pub fn build_validation_realism_evidence(
    context: Option<&ValidationArtifactContext>,
) -> ValidationRealismEvidenceV1 {
    let realism = context.and_then(|entry| entry.data_realism.as_ref());
    let price_adjustment = realism
        .map(|entry| entry.price_adjustment)
        .unwrap_or_default();
    let corporate_actions = realism
        .map(|entry| entry.corporate_actions)
        .unwrap_or_default();
    let survivorship_bias = realism
        .map(|entry| entry.survivorship_bias)
        .unwrap_or_default();
    let liquidity_capacity = realism
        .map(|entry| entry.liquidity_capacity)
        .unwrap_or_default();
    let market_impact = realism.map(|entry| entry.market_impact).unwrap_or_default();
    let borrow_availability = realism
        .map(|entry| entry.borrow_availability)
        .unwrap_or_default();

    let mut missing_or_unknown = Vec::new();
    if matches!(price_adjustment, ValidationPriceAdjustmentState::Unknown) {
        missing_or_unknown.push("price_adjustment".to_string());
    }
    for (name, status) in [
        ("corporate_actions", corporate_actions),
        ("survivorship_bias", survivorship_bias),
        ("liquidity_capacity", liquidity_capacity),
        ("market_impact", market_impact),
        ("borrow_availability", borrow_availability),
    ] {
        if !evidence_is_declared(status) {
            missing_or_unknown.push(name.to_string());
        }
    }

    ValidationRealismEvidenceV1 {
        price_adjustment,
        corporate_actions,
        survivorship_bias,
        liquidity_capacity,
        market_impact,
        borrow_availability,
        complete: missing_or_unknown.is_empty(),
        missing_or_unknown,
    }
}

pub fn build_validation_promotion_eligibility(
    validation_passed: bool,
    validation_approved: bool,
    realism: &ValidationRealismEvidenceV1,
) -> ValidationPromotionEligibilityV1 {
    let mut gates_passed = Vec::new();
    let mut gates_failed = Vec::new();

    if validation_passed && validation_approved {
        gates_passed.push("validation_run_approved".to_string());
    } else {
        gates_failed.push("validation_run_approved".to_string());
    }

    let price_adjustment_declared = !matches!(
        realism.price_adjustment,
        ValidationPriceAdjustmentState::Unknown
    );
    for (gate, passed) in [
        (
            "realism_price_adjustment_declared",
            price_adjustment_declared,
        ),
        (
            "realism_corporate_actions_declared",
            evidence_is_declared(realism.corporate_actions),
        ),
        (
            "realism_survivorship_bias_declared",
            evidence_is_declared(realism.survivorship_bias),
        ),
        (
            "realism_liquidity_capacity_declared",
            evidence_is_declared(realism.liquidity_capacity),
        ),
        (
            "realism_market_impact_declared",
            evidence_is_declared(realism.market_impact),
        ),
        (
            "realism_borrow_availability_declared",
            evidence_is_declared(realism.borrow_availability),
        ),
    ] {
        if passed {
            gates_passed.push(gate.to_string());
        } else {
            gates_failed.push(gate.to_string());
        }
    }

    let (eligible, state, reason) = if !validation_passed || !validation_approved {
        (
            false,
            ValidationPromotionEligibilityState::Blocked,
            "validation did not produce an approved statistical recommendation".to_string(),
        )
    } else if !realism.complete {
        (
            false,
            ValidationPromotionEligibilityState::ResearchOnly,
            format!(
                "promotion blocked until realism evidence is declared for: {}",
                realism.missing_or_unknown.join(", ")
            ),
        )
    } else {
        (
            true,
            ValidationPromotionEligibilityState::EligibleForReview,
            "validation and realism gates are complete; human promotion review is still required"
                .to_string(),
        )
    };

    ValidationPromotionEligibilityV1 {
        eligible,
        state,
        gates_passed,
        gates_failed,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation_shared::{ValidationRealismContext, ValidationRealismEvidenceStatus::*};

    #[test]
    fn missing_realism_context_remains_explicitly_unknown_and_blocks_promotion() {
        let evidence = build_validation_realism_evidence(None);
        let promotion = build_validation_promotion_eligibility(true, true, &evidence);

        assert!(!evidence.complete);
        assert_eq!(
            evidence.missing_or_unknown,
            [
                "price_adjustment",
                "corporate_actions",
                "survivorship_bias",
                "liquidity_capacity",
                "market_impact",
                "borrow_availability",
            ]
        );
        assert!(!promotion.eligible);
        assert_eq!(
            promotion.state,
            ValidationPromotionEligibilityState::ResearchOnly
        );
        assert!(promotion
            .gates_failed
            .contains(&"realism_market_impact_declared".to_string()));
    }

    #[test]
    fn explicit_not_applicable_is_evidence_and_does_not_invent_numeric_assumptions() {
        let context = ValidationArtifactContext {
            data_realism: Some(ValidationRealismContext {
                price_adjustment: ValidationPriceAdjustmentState::NotApplicable,
                corporate_actions: NotApplicable,
                survivorship_bias: Documented,
                liquidity_capacity: Documented,
                market_impact: Documented,
                borrow_availability: NotApplicable,
            }),
            ..ValidationArtifactContext::default()
        };

        let evidence = build_validation_realism_evidence(Some(&context));
        let promotion = build_validation_promotion_eligibility(true, true, &evidence);

        assert!(evidence.complete);
        assert!(evidence.missing_or_unknown.is_empty());
        assert!(promotion.eligible);
        assert_eq!(
            promotion.state,
            ValidationPromotionEligibilityState::EligibleForReview
        );
        assert!(promotion.reason.contains("human promotion review"));
    }

    #[test]
    fn complete_realism_does_not_override_failed_validation() {
        let context = ValidationArtifactContext {
            data_realism: Some(ValidationRealismContext {
                price_adjustment: ValidationPriceAdjustmentState::Adjusted,
                corporate_actions: Documented,
                survivorship_bias: Documented,
                liquidity_capacity: Documented,
                market_impact: Documented,
                borrow_availability: Documented,
            }),
            ..ValidationArtifactContext::default()
        };

        let evidence = build_validation_realism_evidence(Some(&context));
        let promotion = build_validation_promotion_eligibility(false, false, &evidence);

        assert!(evidence.complete);
        assert!(!promotion.eligible);
        assert_eq!(
            promotion.state,
            ValidationPromotionEligibilityState::Blocked
        );
    }
}
