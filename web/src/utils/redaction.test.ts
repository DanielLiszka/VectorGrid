import { describe, expect, it } from "vitest";
import { redactKnownSecrets } from "./redaction";

describe("redactKnownSecrets", () => {
  it("redacts explicit credential values from display text", () => {
    const text = redactKnownSecrets("broker rejected key_1 / secret_1", [
      "key_1",
      "secret_1",
    ]);

    expect(text).toBe("broker rejected [redacted] / [redacted]");
    expect(text).not.toContain("secret_1");
  });

  it("redacts common secret fields and authorization tokens", () => {
    const text = redactKnownSecrets(
      '{"api_key":"poly_secret","apiSecret":"alpaca_secret","Authorization":"Bearer abcdefghijkl"}?apiKey=query_secret',
    );

    expect(text).toContain('"api_key":"[redacted]"');
    expect(text).toContain('"apiSecret":"[redacted]"');
    expect(text).toContain('"Authorization":"[redacted]"');
    expect(text).toContain("?apiKey=[redacted]");
    expect(text).not.toContain("poly_secret");
    expect(text).not.toContain("alpaca_secret");
    expect(text).not.toContain("query_secret");
  });

  it("does not redact ordinary API-key guidance without a value", () => {
    expect(redactKnownSecrets("API key is not set. Enter your key.")).toBe(
      "API key is not set. Enter your key.",
    );
  });

  it("redacts unquoted authorization headers as one value", () => {
    const text = redactKnownSecrets("Authorization: Bearer abcdefghijkl");

    expect(text).toBe("Authorization: [redacted]");
    expect(text).not.toContain("abcdefghijkl");
  });
});
