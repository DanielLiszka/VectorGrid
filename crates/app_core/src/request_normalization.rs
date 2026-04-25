pub(crate) fn normalize_optional_path(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub(crate) fn normalize_topk_include_all(
    top_k: Option<usize>,
    include_all: Option<bool>,
) -> (usize, bool) {
    (top_k.unwrap_or(0), include_all.unwrap_or(false))
}

pub(crate) fn normalize_heatmap_bins(
    heatmap_bins: Option<u16>,
    max_bins: usize,
) -> Result<usize, String> {
    let bins = heatmap_bins.unwrap_or(0) as usize;
    if bins > max_bins {
        return Err(format!("heatmap_bins too large (max {max_bins})"));
    }
    Ok(bins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_optional_path_trims_and_drops_empty() {
        assert_eq!(normalize_optional_path(None), None);
        assert_eq!(normalize_optional_path(Some("".to_string())), None);
        assert_eq!(normalize_optional_path(Some("   ".to_string())), None);
        assert_eq!(
            normalize_optional_path(Some("  out.csv ".to_string())),
            Some("out.csv".to_string())
        );
    }

    #[test]
    fn normalize_optional_path_preserves_interior_whitespace_and_newline_wrapped_input() {
        assert_eq!(
            normalize_optional_path(Some(" reports/weekly run/output.csv ".to_string())),
            Some("reports/weekly run/output.csv".to_string())
        );
        assert_eq!(
            normalize_optional_path(Some("\n\treports/out.csv\r\n".to_string())),
            Some("reports/out.csv".to_string())
        );
    }

    #[test]
    fn normalize_topk_include_all_defaults() {
        assert_eq!(normalize_topk_include_all(None, None), (0, false));
        assert_eq!(normalize_topk_include_all(Some(0), None), (0, false));
        assert_eq!(normalize_topk_include_all(None, Some(true)), (0, true));
        assert_eq!(normalize_topk_include_all(Some(0), Some(true)), (0, true));
        assert_eq!(normalize_topk_include_all(Some(7), Some(true)), (7, true));
    }

    #[test]
    fn normalize_heatmap_bins_applies_bounds() {
        assert_eq!(
            normalize_heatmap_bins(None, 512).expect("disabled bins default"),
            0usize
        );
        assert_eq!(
            normalize_heatmap_bins(Some(0), 512).expect("explicit disabled bins"),
            0usize
        );
        assert_eq!(
            normalize_heatmap_bins(Some(12), 512).expect("valid bins"),
            12usize
        );
        let err = normalize_heatmap_bins(Some(513), 512).expect_err("must reject overflow");
        assert!(err.contains("max 512"));
    }
}
