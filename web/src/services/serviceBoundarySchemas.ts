import { z } from "zod";

export const boundaryObjectSchema = z.object({}).passthrough();
export const boundaryArraySchema = z.array(z.unknown());

export type BoundaryObject = z.infer<typeof boundaryObjectSchema>;

export function isBoundaryObject(value: unknown): value is BoundaryObject {
  return boundaryObjectSchema.safeParse(value).success;
}

export function parseBoundaryObject(value: unknown): BoundaryObject {
  const parsed = boundaryObjectSchema.safeParse(value);
  return parsed.success ? parsed.data : {};
}

export function requireBoundaryObject(
  value: unknown,
  context = "IPC response",
): BoundaryObject {
  const parsed = boundaryObjectSchema.safeParse(value);
  if (!parsed.success) {
    throw new Error(`${context} must be an object.`);
  }
  return parsed.data;
}

export function parseOptionalBoundaryObject(
  value: unknown,
): BoundaryObject | null {
  const parsed = boundaryObjectSchema.safeParse(value);
  return parsed.success ? parsed.data : null;
}

export function parseBoundaryArray(value: unknown): unknown[] {
  const parsed = boundaryArraySchema.safeParse(value);
  return parsed.success ? parsed.data : [];
}

export function requireBoundaryArray(
  value: unknown,
  context = "IPC response",
): unknown[] {
  const parsed = boundaryArraySchema.safeParse(value);
  if (!parsed.success) {
    throw new Error(`${context} must be an array.`);
  }
  return parsed.data;
}
