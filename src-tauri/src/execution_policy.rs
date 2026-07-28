use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExecutionPolicyLimits {
    pub max_order_notional: Option<f64>,
    pub max_position_notional: Option<f64>,
    pub max_daily_loss: Option<f64>,
    pub max_daily_orders: Option<u32>,
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
    pub daily_pnl: Option<f64>,
    pub daily_order_count: Option<u32>,
    pub enforce_daily_limits: bool,
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
    if input.daily_pnl.is_some_and(|value| !value.is_finite()) {
        return ExecutionPolicyDecision::block(
            "invalid_daily_pnl",
            "Daily P&L must be finite when supplied.",
        );
    }

    let mut warnings = Vec::new();
    let crosses_position_sign = input.current_position_qty.abs() > f64::EPSILON
        && input.target_position_qty.abs() > f64::EPSILON
        && input.current_position_qty.signum() != input.target_position_qty.signum();
    let risk_increasing_order = crosses_position_sign
        || input.target_position_qty.abs() > input.current_position_qty.abs() + f64::EPSILON;
    if input.delta_qty.abs() > f64::EPSILON && risk_increasing_order {
        if input.enforce_daily_limits {
            if let Some(max_daily_orders) = input.limits.max_daily_orders.filter(|v| *v > 0) {
                let Some(daily_order_count) = input.daily_order_count else {
                    return ExecutionPolicyDecision::block(
                        "daily_order_count_unavailable",
                        "Daily order count is unavailable; execution fails closed while a daily order cap is configured.",
                    );
                };
                if daily_order_count >= max_daily_orders {
                    return ExecutionPolicyDecision::block(
                        "max_daily_orders_reached",
                        format!(
                            "Daily order count {daily_order_count} has reached policy cap {max_daily_orders}."
                        ),
                    );
                }
            }
            if let Some(max_daily_loss) = sanitize_positive_limit(input.limits.max_daily_loss) {
                let Some(daily_pnl) = input.daily_pnl else {
                    return ExecutionPolicyDecision::block(
                        "daily_pnl_unavailable",
                        "Daily P&L is unavailable; execution fails closed while a daily loss cap is configured.",
                    );
                };
                if daily_pnl <= -max_daily_loss {
                    return ExecutionPolicyDecision::block(
                        "max_daily_loss_reached",
                        format!(
                            "Daily P&L {daily_pnl:.2} has reached policy loss cap -{max_daily_loss:.2}."
                        ),
                    );
                }
            }
        } else if input.limits.max_daily_orders.is_some() || input.limits.max_daily_loss.is_some() {
            warnings.push(
                "Daily loss/order limits are evaluated only against durable broker state at submission."
                    .to_string(),
            );
        }
    }
    let increases_or_opens_short = input.target_position_qty < -f64::EPSILON
        && (input.current_position_qty >= -f64::EPSILON
            || input.target_position_qty < input.current_position_qty - f64::EPSILON);
    if input
        .limits
        .allow_short
        .is_some_and(|allow_short| !allow_short)
        && increases_or_opens_short
    {
        return ExecutionPolicyDecision::block(
            "short_exposure_blocked",
            "Execution policy blocks new or increased short exposure.",
        );
    }

    if risk_increasing_order {
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
        if let Some(max_position_notional) =
            sanitize_positive_limit(input.limits.max_position_notional)
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

    ExecutionPolicyDecision::allow(warnings)
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
            daily_pnl: Some(0.0),
            daily_order_count: Some(0),
            enforce_daily_limits: true,
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
    fn no_short_policy_allows_partial_and_full_covers() {
        let mut input = base_input();
        input.current_position_qty = -10.0;
        input.target_position_qty = -4.0;
        input.delta_qty = 6.0;
        input.qty = 6.0;
        input.opening_new_position = false;
        input.limits.allow_short = Some(false);
        assert!(evaluate_execution_policy(input).allowed);

        input.target_position_qty = 0.0;
        input.delta_qty = 10.0;
        input.qty = 10.0;
        assert!(evaluate_execution_policy(input).allowed);
    }

    #[test]
    fn no_short_policy_blocks_increased_or_new_short_exposure() {
        let mut input = base_input();
        input.current_position_qty = -4.0;
        input.target_position_qty = -6.0;
        input.delta_qty = -2.0;
        input.limits.allow_short = Some(false);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "short_exposure_blocked"
        );

        input = base_input();
        input.current_position_qty = 4.0;
        input.target_position_qty = -1.0;
        input.delta_qty = -5.0;
        input.qty = 5.0;
        input.opening_new_position = false;
        input.limits.allow_short = Some(false);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "short_exposure_blocked"
        );
    }

    #[test]
    fn sign_flip_is_risk_increasing_even_when_target_absolute_exposure_is_smaller() {
        let mut input = base_input();
        input.current_position_qty = 5.0;
        input.target_position_qty = -4.0;
        input.delta_qty = -9.0;
        input.qty = 9.0;
        input.opening_new_position = false;
        input.limits.allow_short = Some(true);
        input.limits.max_order_notional = Some(500.0);

        let decision = evaluate_execution_policy(input);
        assert!(!decision.allowed);
        assert_eq!(decision.code, "max_order_notional_exceeded");
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

    #[test]
    fn blocks_daily_order_and_loss_caps_and_fails_closed_without_state() {
        let mut input = base_input();
        input.limits.max_daily_orders = Some(3);
        input.daily_order_count = Some(3);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "max_daily_orders_reached"
        );

        input = base_input();
        input.limits.max_daily_loss = Some(250.0);
        input.daily_pnl = Some(-250.0);
        assert_eq!(
            evaluate_execution_policy(input).code,
            "max_daily_loss_reached"
        );

        input = base_input();
        input.limits.max_daily_loss = Some(250.0);
        input.daily_pnl = None;
        assert_eq!(
            evaluate_execution_policy(input).code,
            "daily_pnl_unavailable"
        );
    }

    #[test]
    fn preview_without_broker_state_warns_instead_of_blocking_daily_limits() {
        let mut input = base_input();
        input.limits.max_daily_orders = Some(3);
        input.limits.max_daily_loss = Some(250.0);
        input.daily_order_count = None;
        input.daily_pnl = None;
        input.enforce_daily_limits = false;
        let decision = evaluate_execution_policy(input);
        assert!(decision.allowed);
        assert_eq!(decision.warnings.len(), 1);
    }

    #[test]
    fn daily_caps_do_not_block_risk_reducing_orders() {
        let mut input = base_input();
        input.current_position_qty = 2.0;
        input.target_position_qty = 0.0;
        input.delta_qty = -2.0;
        input.opening_new_position = false;
        input.daily_pnl = None;
        input.daily_order_count = None;
        input.limits.max_daily_loss = Some(250.0);
        input.limits.max_daily_orders = Some(3);

        let decision = evaluate_execution_policy(input);

        assert!(decision.allowed);
        assert!(decision.warnings.is_empty());
    }

    #[test]
    fn notional_caps_do_not_block_risk_reducing_orders() {
        let mut input = base_input();
        input.current_position_qty = 50.0;
        input.target_position_qty = 0.0;
        input.delta_qty = -50.0;
        input.opening_new_position = false;
        input.limits.max_order_notional = Some(1_000.0);
        input.limits.max_position_notional = Some(2_500.0);

        let decision = evaluate_execution_policy(input);

        assert!(decision.allowed);
    }
}
