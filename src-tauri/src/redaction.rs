const REDACTED: &str = "[redacted]";
const SENSITIVE_KEYS: &[&str] = &[
    "apikey",
    "api_key",
    "api-secret",
    "api_secret",
    "authorization",
    "password",
    "secret",
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
    redact_assignments(out)
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
    for key in SENSITIVE_KEYS {
        let mut start = 0usize;
        loop {
            let lower = out.to_ascii_lowercase();
            let Some(relative) = lower[start..].find(key) else {
                break;
            };
            let key_start = start + relative;
            let mut cursor = key_start + key.len();
            cursor = skip_ascii_ws(&out, cursor);
            if !matches!(out.as_bytes().get(cursor), Some(b':') | Some(b'=')) {
                start = cursor;
                continue;
            }
            cursor += 1;
            cursor = skip_ascii_ws(&out, cursor);
            let quote = out.as_bytes().get(cursor).copied();
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

fn skip_ascii_ws(input: &str, mut cursor: usize) -> usize {
    while matches!(
        input.as_bytes().get(cursor),
        Some(b' ' | b'\t' | b'\r' | b'\n')
    ) {
        cursor += 1;
    }
    cursor
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
}
