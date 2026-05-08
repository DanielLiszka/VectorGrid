const REDACTED: &str = "[redacted]";
const SENSITIVE_KEYS: &[&str] = &[
    "apca-api-key-id",
    "apca-api-secret-key",
    "x-api-key",
    "api-key",
    "api-secret-key",
    "api_key_id",
    "api_key",
    "apikey",
    "api-secret",
    "api_secret",
    "authorization",
    "password",
    "secret",
    "secret-key",
    "token",
];

pub fn redact_sensitive_text_with_values<'a>(
    raw: &str,
    explicit_secrets: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut out = raw.trim().to_string();
    if out.is_empty() {
        return out;
    }

    for secret in explicit_secrets {
        let secret = secret.trim();
        if secret.len() >= 3 {
            out = out.replace(secret, REDACTED);
        }
    }

    out = redact_bearer_tokens(out);
    out = redact_line_prefix_assignments(out);
    out = redact_assignments(out);
    redact_url_params(out)
}

fn redact_bearer_tokens(input: String) -> String {
    let mut out = input;
    let mut start = 0usize;
    loop {
        let lower = out.to_ascii_lowercase();
        let Some(relative) = lower[start..].find("bearer ") else {
            return out;
        };
        let token_start = start + relative + "bearer ".len();
        let token_end = out[token_start..]
            .find(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .map(|idx| token_start + idx)
            .unwrap_or(out.len());
        if token_end > token_start {
            out.replace_range(token_start..token_end, REDACTED);
            start = token_start + REDACTED.len();
        } else {
            start = token_start;
        }
    }
}

fn redact_assignments(input: String) -> String {
    let mut out = input;
    for &key in SENSITIVE_KEYS {
        let mut start = 0usize;
        loop {
            let lower = out.to_ascii_lowercase();
            let Some(relative) = lower[start..].find(key) else {
                break;
            };
            let key_start = start + relative;
            let mut cursor = key_start + key.len();
            cursor = skip_ascii_ws(&out, cursor);
            if !is_assignment_separator(&out, cursor) {
                start = cursor;
                continue;
            }
            cursor += 1;
            cursor = skip_ascii_ws(&out, cursor);
            let quote = out
                .as_bytes()
                .get(cursor)
                .copied()
                .filter(|byte| matches!(*byte, b'"' | b'\''));
            let value_start = if matches!(quote, Some(b'"') | Some(b'\'')) {
                cursor + 1
            } else {
                cursor
            };
            let value_end = find_value_end(&out, value_start, quote);
            if value_end > value_start {
                out.replace_range(value_start..value_end, REDACTED);
                start = value_start + REDACTED.len();
            } else {
                start = value_start;
            }
        }
    }
    out
}

fn redact_line_prefix_assignments(input: String) -> String {
    let mut out = String::with_capacity(input.len());
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        if ch == '\n' || ch == '\r' {
            let end = idx + ch.len_utf8();
            out.push_str(&redact_line_prefix_segment(&input[start..end]));
            start = end;
        }
    }
    if start < input.len() {
        out.push_str(&redact_line_prefix_segment(&input[start..]));
    }
    out
}

fn redact_line_prefix_segment(segment: &str) -> String {
    let line_end = match (segment.find('\n'), segment.find('\r')) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => segment.len(),
    };
    let (line, ending) = segment.split_at(line_end);
    let leading_ws = line
        .find(|c: char| c != ' ' && c != '\t')
        .unwrap_or(line.len());
    let candidate = &line[leading_ws..];
    for &key in SENSITIVE_KEYS {
        let Some(prefix) = candidate.get(..key.len()) else {
            continue;
        };
        if !prefix.eq_ignore_ascii_case(key) {
            continue;
        }
        let mut cursor = leading_ws + key.len();
        cursor = skip_ascii_ws(line, cursor);
        if !is_assignment_separator(line, cursor) {
            continue;
        }
        cursor += 1;
        cursor = skip_ascii_ws(line, cursor);
        let quote = line
            .as_bytes()
            .get(cursor)
            .copied()
            .filter(|byte| matches!(*byte, b'"' | b'\''));
        let value_start = if matches!(quote, Some(b'"') | Some(b'\'')) {
            cursor + 1
        } else {
            cursor
        };
        let value_end = find_value_end(line, value_start, quote);
        if value_end <= value_start {
            continue;
        }
        let mut redacted = line.to_string();
        redacted.replace_range(value_start..value_end, REDACTED);
        redacted.push_str(ending);
        return redacted;
    }
    segment.to_string()
}

fn skip_ascii_ws(input: &str, mut cursor: usize) -> usize {
    while input
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n'))
    {
        cursor += 1;
    }
    cursor
}

fn is_assignment_separator(input: &str, cursor: usize) -> bool {
    input
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| matches!(*byte, b':' | b'='))
}

fn find_value_end(input: &str, start: usize, quote: Option<u8>) -> usize {
    let bytes = input.as_bytes();
    let mut cursor = start;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(q) = quote {
            if byte == q {
                break;
            }
        } else if matches!(
            byte,
            b' ' | b'\t' | b'\r' | b'\n' | b',' | b';' | b'&' | b'}' | b']'
        ) {
            break;
        }
        cursor += 1;
    }
    cursor
}

fn redact_url_params(input: String) -> String {
    let mut out = input;
    for &key in SENSITIVE_KEYS {
        let marker = format!("{key}=");
        let mut start = 0usize;
        loop {
            let lower = out.to_ascii_lowercase();
            let Some(relative) = lower[start..].find(&marker) else {
                break;
            };
            let value_start = start + relative + marker.len();
            let value_end = find_value_end(&out, value_start, None);
            if value_end > value_start {
                out.replace_range(value_start..value_end, REDACTED);
                start = value_start + REDACTED.len();
            } else {
                start = value_start;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::redact_sensitive_text_with_values;

    #[test]
    fn redacts_explicit_values_assignments_and_bearer_tokens() {
        let text = redact_sensitive_text_with_values(
            r#"provider apiKey=poly_secret_123 Authorization: Bearer abcdefghijkl api_secret="alpaca_secret""#,
            ["poly_secret_123"],
        );
        assert!(text.contains("[redacted]"));
        assert!(!text.contains("poly_secret_123"));
        assert!(!text.contains("abcdefghijkl"));
        assert!(!text.contains("alpaca_secret"));
    }

    #[test]
    fn leaves_guidance_without_secret_values_readable() {
        let text = redact_sensitive_text_with_values("Polygon API key is not set.", []);
        assert_eq!(text, "Polygon API key is not set.");
    }

    #[test]
    fn redacts_camel_case_api_key_without_explicit_secret() {
        let text = redact_sensitive_text_with_values(
            "https://api.polygon.io/v2/aggs?apiKey=poly_secret_123&limit=1",
            [],
        );
        assert!(text.contains("[redacted]"));
        assert!(!text.contains("poly_secret_123"));
        assert!(text.contains("limit=1"));
    }

    #[test]
    fn redacts_header_style_provider_key_names() {
        let text = redact_sensitive_text_with_values(
            "x-api-key: poly_secret\nAPCA-API-KEY-ID=alpaca_key\nAPCA-API-SECRET-KEY: \"alpaca_secret\"",
            [],
        );
        assert!(text.contains("[redacted]"));
        assert!(!text.contains("poly_secret"));
        assert!(!text.contains("alpaca_key"));
        assert!(!text.contains("alpaca_secret"));
    }

    #[test]
    fn redacts_sensitive_key_at_line_prefix() {
        let text = super::redact_line_prefix_segment("x-api-key: poly_secret\n");
        assert_eq!(text, "x-api-key: [redacted]\n");
    }
}
