const REDACTED = "[redacted]";

const SENSITIVE_KEY_PATTERN =
  "(?:alpaca[_-]?)?api[_-]?(?:key|secret)|apiKey|apiSecret|authorization|bearer|password|secret|token";

const JSON_SECRET_FIELD_RE = new RegExp(
  `(["'](?:${SENSITIVE_KEY_PATTERN})["']\\s*:\\s*["'])([^"']+)(["'])`,
  "gi",
);
const ASSIGNMENT_SECRET_FIELD_RE = new RegExp(
  `\\b((?:${SENSITIVE_KEY_PATTERN})\\s*[:=]\\s*)([^\\s,;&]+)`,
  "gi",
);
const QUERY_SECRET_FIELD_RE = new RegExp(
  `([?&](?:${SENSITIVE_KEY_PATTERN})=)([^&#\\s]+)`,
  "gi",
);
const BEARER_RE = /\bBearer\s+[A-Za-z0-9._~+/=-]{8,}/gi;

function stringifyRedactionInput(value: unknown): string {
  if (value instanceof Error) return String(value);
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value ?? "");
  }
}

function explicitSecretValues(values: Iterable<unknown> | undefined): string[] {
  if (!values) return [];
  const out: string[] = [];
  for (const value of values) {
    const text = String(value || "").trim();
    if (text.length >= 3) out.push(text);
  }
  return Array.from(new Set(out));
}

export function redactKnownSecrets(
  value: unknown,
  explicitSecrets?: Iterable<unknown>,
): string {
  let text = stringifyRedactionInput(value).trim();
  if (!text) return "";

  for (const secret of explicitSecretValues(explicitSecrets)) {
    text = text.split(secret).join(REDACTED);
  }

  return text
    .replace(BEARER_RE, `Bearer ${REDACTED}`)
    .replace(JSON_SECRET_FIELD_RE, `$1${REDACTED}$3`)
    .replace(QUERY_SECRET_FIELD_RE, `$1${REDACTED}`)
    .replace(ASSIGNMENT_SECRET_FIELD_RE, `$1${REDACTED}`);
}
