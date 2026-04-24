use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExecutionPolicyLimits {
    pub max_order_notional: Option<f64>,
    pub max_position_notional: Option<f64>,
    pub allow_short: Option<bool>,
    pub require_bracket: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExecutionPolicyInput {
    pub qty: f64,
    pub current_position_qty: f64,
    pub target_position_qty: f64,
    pub delta_qty: f64,
    pub last_close: f64,
    pub opening_new_position: bool,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub limits: ExecutionPolicyLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionPolicyDecision {
    pub allowed: bool,
    pub code: String,
    pub message: String,
    pub warnings: Vec<String>,
}

impl ExecutionPolicyDecision {
    pub fn allow(warnings: Vec<String>) -> Self {
        Self {
            allowed: true,
            code: "allowed".to_string(),
            message: "Execution policy allowed the order.".to_string(),
            warnings,
        }
    }

    pub fn block(code: &str, message: impl Into<String>) -> Self {
        Self {
            allowed: false,
            code: code.to_string(),
            message: message.into(),
            warnings: Vec::new(),
        }
    }
}

pub fn evaluate_execution_policy(input: ExecutionPolicyInput) -> ExecutionPolicyDecision {
    if !input.qty.is_finite() || input.qty <= 0.0 {
        return ExecutionPolicyDecision::block("invalid_qty", "Quantity must be finite and > 0.");
    }
    for (name, value) in [
        ("current_position_qty", input.current_position_qty),
        ("target_position_qty", input.target_position_qty),
        ("delta_qty", input.delta_qty),
        ("last_close", input.last_close),
    ] {
        if !value.is_finite() {
            return ExecutionPolicyDecision::block(
                "invalid_numeric_input",
                format!("{name} must be finite."),
            );
        }
    }
    if input.delta_qty.abs() > f64::EPSILON && input.last_close <= 0.0 {
        return ExecutionPolicyDecision::block(
            "invalid_market_price",
            "Last close must be positive before submitting a market order.",
        );
    }
    if input
        .limits
        .allow_short
        .is_some_and(|allow_short| !allow_short)
        && input.target_position_qty < -f64::EPSILON
    {
        return ExecutionPolicyDecision::block(
            "short_exposure_blocked",
            "Execution policy blocks target short exposure.",
        );
    }

    let order_notional = input.delta_qty.abs() * input.last_close;
    if let Some(max_order_notional) = sanitize_positive_limit(input.limits.max_order_notional) {
        if order_notional > max_order_notional + f64::EPSILON {
            return ExecutionPolicyDecision::block(
                "max_order_notional_exceeded",
                format!(
                    "Order notional {order_notional:.2} exceeds policy cap {max_order_notional:.2}."
                ),
            );
        }
    }

    let position_notional = input.target_position_qty.abs() * input.last_close;
    if let Some(max_position_notional) = sanitize_positive_limit(input.limits.max_position_notional)
    {
        if position_notional > max_position_notional + f64::EPSILON {
            return ExecutionPolicyDecision::block(
                "max_position_notional_exceeded",
                format!(
                    "Target position notional {position_notional:.2} exceeds policy cap {max_position_notional:.2}."
                ),
            );
        }
    }

    if input
        .limits
        .require_bracket
        .is_some_and(|require_bracket| require_bracket)
        && input.opening_new_position
        && (input.stop_loss_pct.is_none() || input.take_profit_pct.is_none())
    {
        return ExecutionPolicyDecision::block(
            "bracket_required",
            "Execution policy requires both stop-loss and take-profit brackets for new positions.",
        );
    }

    ExecutionPolicyDecision::allow(Vec::new())
}

fn sanitize_positive_limit(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> ExecutionPolicyInput {
        ExecutionPolicyInput {
            qty: 2.0,
            current_position_qty: 0.0,
            target_position_qty: 2.0,
            delta_qty: 2.0,
            last_close: 100.0,
            opening_new_position: true,
            stop_loss_pct: Some(0.05),
            take_profit_pct: Some(0.10),
            limits: ExecutionPolicyLimits::default(),
        }
    }

    #[test]
    fn allows_when_no_policy_caps_are_configured() {
        assert!(evaluate_execution_policy(base_input()).allowed);
    }

    #[test]
    fn blocks_invalid_market_price_when_an_order_is_needed() {
        let mut input = base_input();
        input.last_close = 0.0;
        let decision = evaluate_execution_policy(input);
        assert!(!decision.allowed);
        assert_eq!(decision.code, "invalid_market_price");
    }

    #[test]
    fn blocks_explicit_notional_caps_and_short_policy() {
        let mut input = base_input();
        input.limits.max_order_notional = Some(100.0);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "max_order_notional_exceeded"
        );

        input = base_input();
        input.limits.max_position_notional = Some(100.0);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "max_position_notional_exceeded"
        );

        input = base_input();
        input.target_position_qty = -2.0;
        input.delta_qty = -2.0;
        input.limits.allow_short = Some(false);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "short_exposure_blocked"
        );
    }

    #[test]
    fn blocks_required_bracket_only_for_new_positions() {
        let mut input = base_input();
        input.take_profit_pct = None;
        input.limits.require_bracket = Some(true);
        assert_eq!(evaluate_execution_policy(input).code, "bracket_required");

        input.opening_new_position = false;
        assert!(evaluate_execution_policy(input).allowed);
    }
}
