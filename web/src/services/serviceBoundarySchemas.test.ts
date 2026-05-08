import { describe, expect, it } from "vitest";
import {
  parseBoundaryArray,
  parseBoundaryObject,
  requireBoundaryArray,
  requireBoundaryObject,
} from "./serviceBoundarySchemas";

describe("serviceBoundarySchemas", () => {
  it("keeps legacy tolerant parsers available for optional nested payloads", () => {
    expect(parseBoundaryObject("not-object")).toEqual({});
    expect(parseBoundaryArray({ not: "array" })).toEqual([]);
  });

  it("throws from strict parsers used at command boundaries", () => {
    expect(() => requireBoundaryObject("not-object", "test response")).toThrow(
      "test response must be an object",
    );
    expect(() => requireBoundaryArray({ not: "array" }, "test list")).toThrow(
      "test list must be an array",
    );
  });
});
