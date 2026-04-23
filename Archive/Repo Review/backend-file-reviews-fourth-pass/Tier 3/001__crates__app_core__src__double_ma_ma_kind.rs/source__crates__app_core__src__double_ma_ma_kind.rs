pub(crate) fn lookup_ma_info(
    ma_type: &str,
) -> Option<&'static vector_ta::indicators::moving_averages::registry::MovingAverageInfo> {
    vector_ta::indicators::moving_averages::registry::list_moving_averages()
        .iter()
        .find(|m| m.id.eq_ignore_ascii_case(ma_type))
}

pub(crate) fn validate_ma_kind(ma_type: &str) -> Result<(), String> {
    let ma = ma_type.trim();
    let info = lookup_ma_info(ma).ok_or_else(|| format!("Unknown moving average type: '{ma}'"))?;
    if !info.period_based {
        return Err(format!(
            "MA '{ma}' is not period-based and cannot be used in the double-MA optimizer."
        ));
    }
    if !info.single_output {
        return Err(format!(
            "MA '{ma}' has multiple outputs and cannot be used in the double-MA optimizer."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{lookup_ma_info, validate_ma_kind};

    #[test]
    fn validate_ma_kind_accepts_a_supported_period_based_ma() {
        let ma = vector_ta::indicators::moving_averages::registry::list_moving_averages()
            .iter()
            .find(|info| info.period_based && info.single_output)
            .expect("expected at least one period-based single-output MA");
        assert!(lookup_ma_info(ma.id).is_some());
        assert!(validate_ma_kind(ma.id).is_ok());
    }

    #[test]
    fn validate_ma_kind_rejects_unknown_ma() {
        let err = validate_ma_kind("__unknown_ma__").expect_err("unknown MA must be rejected");
        assert!(err.contains("Unknown moving average type"));
    }
}
