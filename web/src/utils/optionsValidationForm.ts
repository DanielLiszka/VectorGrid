import { useForm } from "@tanstack/vue-form";
import { z } from "zod";

export const optionsValidationDatasetPolicies = [
  "require_exact_replay",
  "allow_same_root_replacement",
  "allow_explicit_cross_lane_comparison",
] as const;

export const optionsValidationPlanModes = [
  "original_embedded_splits",
  "session_chronological_replay",
  "date_range_holdout",
  "anchored_walk_forward",
  "rolling_session_walk_forward",
] as const;

const UTC_DATE_INPUT_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;

export const optionsValidationFormSchema = z
  .object({
    artifactPath: z
      .string()
      .trim()
      .min(1, "Load or run an options expression artifact before validation."),
    activeArtifactOptionsDatasetId: z.string().trim(),
    selectedDatasetId: z.string().trim(),
    datasetPolicy: z.enum(optionsValidationDatasetPolicies),
    validationPlanMode: z.enum(optionsValidationPlanModes),
    startDate: z.string().trim(),
    endDate: z.string().trim(),
    trainSessionCountInput: z.string().trim(),
    validationSessionCountInput: z.string().trim(),
    stepSessionCountInput: z.string().trim(),
    tupleIndexesInput: z.string().trim(),
    includeSealedFinalists: z.boolean(),
    includeRobustLeader: z.boolean(),
    includeHoldoutLeader: z.boolean(),
    minValidTradesInput: z.string().trim(),
  })
  .superRefine((value, ctx) => {
    const replacementPolicy = value.datasetPolicy !== "require_exact_replay";
    if (replacementPolicy && !value.selectedDatasetId) {
      ctx.addIssue({
        code: "custom",
        path: ["selectedDatasetId"],
        message:
          "Select a Polygon.io replacement dataset before using a replacement validation policy.",
      });
    }

    if (value.validationPlanMode === "date_range_holdout") {
      const start = parseUtcDateInputToMs(
        value.startDate,
        "Validation start date",
      );
      const end = parseUtcDateInputToMs(value.endDate, "Validation end date");
      if (typeof start === "string") {
        ctx.addIssue({ code: "custom", path: ["startDate"], message: start });
      }
      if (typeof end === "string") {
        ctx.addIssue({ code: "custom", path: ["endDate"], message: end });
      }
      if (
        typeof start === "number" &&
        typeof end === "number" &&
        start >= end
      ) {
        ctx.addIssue({
          code: "custom",
          path: ["endDate"],
          message: "Validation start date must be before validation end date.",
        });
      }
    }

    if (
      value.validationPlanMode === "anchored_walk_forward" ||
      value.validationPlanMode === "rolling_session_walk_forward"
    ) {
      for (const [field, label] of [
        ["trainSessionCountInput", "Train sessions"],
        ["validationSessionCountInput", "Validation sessions"],
        ["stepSessionCountInput", "Step sessions"],
      ] as const) {
        const parsed = parseRequiredPositiveIntegerInput(value[field], label);
        if (typeof parsed === "string") {
          ctx.addIssue({ code: "custom", path: [field], message: parsed });
        }
      }
    }

    const minValidTrades = parseOptionalNonNegativeInteger(
      value.minValidTradesInput,
      "Min valid trades",
    );
    if (typeof minValidTrades === "string") {
      ctx.addIssue({
        code: "custom",
        path: ["minValidTradesInput"],
        message: minValidTrades,
      });
    }

    const tupleIndexes = parseIntegerList(value.tupleIndexesInput);
    if (typeof tupleIndexes === "string") {
      ctx.addIssue({
        code: "custom",
        path: ["tupleIndexesInput"],
        message: tupleIndexes,
      });
    }
  });

export type OptionsValidationFormValues = z.infer<
  typeof optionsValidationFormSchema
>;
export type OptionsValidationDatasetPolicy =
  OptionsValidationFormValues["datasetPolicy"];
export type OptionsValidationPlanMode =
  OptionsValidationFormValues["validationPlanMode"];

export interface OptionsExpressionValidationCommandRequest {
  artifact_path: string;
  options_dataset_id: string;
  dataset_policy: OptionsValidationDatasetPolicy;
  tuple_selection: {
    include_sealed_finalists: boolean;
    include_robust_leader: boolean;
    include_holdout_leader: boolean;
    tuple_indexes: number[];
  };
  validation_plan: {
    mode: OptionsValidationPlanMode;
    start_timestamp_ms?: number;
    end_timestamp_ms?: number;
    train_session_count?: number;
    validation_session_count?: number;
    step_session_count?: number;
  };
  min_valid_trades: number | null;
}

export interface OptionsValidationFormFactoryOptions {
  defaultValues?: Partial<OptionsValidationFormValues>;
  onSubmit?: (values: OptionsValidationFormValues) => void | Promise<void>;
}

export const defaultOptionsValidationFormValues: OptionsValidationFormValues = {
  artifactPath: "",
  activeArtifactOptionsDatasetId: "",
  selectedDatasetId: "",
  datasetPolicy: "require_exact_replay",
  validationPlanMode: "original_embedded_splits",
  startDate: "",
  endDate: "",
  trainSessionCountInput: "5",
  validationSessionCountInput: "1",
  stepSessionCountInput: "1",
  tupleIndexesInput: "",
  includeSealedFinalists: true,
  includeRobustLeader: true,
  includeHoldoutLeader: true,
  minValidTradesInput: "",
};

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function pickSupported<T extends string>(
  value: unknown,
  allowed: readonly T[],
  fallback: T,
): T {
  const normalized = normalizeText(value);
  return (allowed.includes(normalized as T) ? normalized : fallback) as T;
}

function parseRequiredPositiveIntegerInput(
  value: unknown,
  label: string,
): number | string {
  const raw = normalizeText(value);
  const parsed = Number(raw);
  if (!raw || !Number.isInteger(parsed) || parsed <= 0) {
    return `${label} must be a positive whole number.`;
  }
  return parsed;
}

function parseOptionalNonNegativeInteger(
  value: unknown,
  label: string,
): number | null | string {
  const raw = normalizeText(value);
  if (!raw) return null;
  const parsed = Number(raw);
  if (!Number.isInteger(parsed) || parsed < 0) {
    return `${label} must be a non-negative whole number.`;
  }
  return parsed;
}

function parseUtcDateInputToMs(value: unknown, label: string): number | string {
  const raw = normalizeText(value);
  if (!raw) return `${label} is required.`;
  const match = UTC_DATE_INPUT_PATTERN.exec(raw);
  if (!match) {
    return `${label} must be a valid YYYY-MM-DD date.`;
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const parsed = Date.UTC(year, month - 1, day);
  const date = new Date(parsed);
  if (
    !Number.isFinite(parsed) ||
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() + 1 !== month ||
    date.getUTCDate() !== day
  ) {
    return `${label} must be a valid YYYY-MM-DD date.`;
  }
  return parsed;
}

function parseIntegerList(value: string): number[] | string {
  const tokens = normalizeText(value)
    .split(",")
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0);
  const out: number[] = [];
  for (const entry of tokens) {
    const parsed = Number(entry);
    if (!Number.isInteger(parsed) || parsed < 0) {
      return "Tuple indexes must be comma-separated non-negative whole numbers.";
    }
    out.push(parsed);
  }
  return Array.from(new Set(out)).sort((a, b) => a - b);
}

export function createOptionsValidationFormValues(
  overrides: Partial<OptionsValidationFormValues> = {},
): OptionsValidationFormValues {
  return {
    artifactPath: normalizeText(
      overrides.artifactPath ?? defaultOptionsValidationFormValues.artifactPath,
    ),
    activeArtifactOptionsDatasetId: normalizeText(
      overrides.activeArtifactOptionsDatasetId ??
        defaultOptionsValidationFormValues.activeArtifactOptionsDatasetId,
    ),
    selectedDatasetId: normalizeText(
      overrides.selectedDatasetId ??
        defaultOptionsValidationFormValues.selectedDatasetId,
    ),
    datasetPolicy: pickSupported(
      overrides.datasetPolicy,
      optionsValidationDatasetPolicies,
      defaultOptionsValidationFormValues.datasetPolicy,
    ),
    validationPlanMode: pickSupported(
      overrides.validationPlanMode,
      optionsValidationPlanModes,
      defaultOptionsValidationFormValues.validationPlanMode,
    ),
    startDate: normalizeText(
      overrides.startDate ?? defaultOptionsValidationFormValues.startDate,
    ),
    endDate: normalizeText(
      overrides.endDate ?? defaultOptionsValidationFormValues.endDate,
    ),
    trainSessionCountInput: normalizeText(
      overrides.trainSessionCountInput ??
        defaultOptionsValidationFormValues.trainSessionCountInput,
    ),
    validationSessionCountInput: normalizeText(
      overrides.validationSessionCountInput ??
        defaultOptionsValidationFormValues.validationSessionCountInput,
    ),
    stepSessionCountInput: normalizeText(
      overrides.stepSessionCountInput ??
        defaultOptionsValidationFormValues.stepSessionCountInput,
    ),
    tupleIndexesInput: normalizeText(
      overrides.tupleIndexesInput ??
        defaultOptionsValidationFormValues.tupleIndexesInput,
    ),
    includeSealedFinalists:
      overrides.includeSealedFinalists ??
      defaultOptionsValidationFormValues.includeSealedFinalists,
    includeRobustLeader:
      overrides.includeRobustLeader ??
      defaultOptionsValidationFormValues.includeRobustLeader,
    includeHoldoutLeader:
      overrides.includeHoldoutLeader ??
      defaultOptionsValidationFormValues.includeHoldoutLeader,
    minValidTradesInput: normalizeText(
      overrides.minValidTradesInput ??
        defaultOptionsValidationFormValues.minValidTradesInput,
    ),
  };
}

export function validateOptionsValidationFormValues(
  values: OptionsValidationFormValues,
):
  | {
      fields: Partial<Record<keyof OptionsValidationFormValues, string>>;
    }
  | undefined {
  const parsed = optionsValidationFormSchema.safeParse(values);
  if (parsed.success) return undefined;
  const fields: Partial<Record<keyof OptionsValidationFormValues, string>> = {};
  for (const issue of parsed.error.issues) {
    const key = issue.path[0];
    if (typeof key !== "string") continue;
    const field = key as keyof OptionsValidationFormValues;
    if (!fields[field]) fields[field] = issue.message;
  }
  return { fields };
}

export function optionsValidationErrorMessages(
  values: OptionsValidationFormValues,
): string[] {
  return Object.values(
    validateOptionsValidationFormValues(values)?.fields ?? {},
  )
    .map((entry) => normalizeText(entry))
    .filter((entry) => entry.length > 0);
}

export function toOptionsValidationCommandRequest(
  values: OptionsValidationFormValues,
): OptionsExpressionValidationCommandRequest {
  const parsed = optionsValidationFormSchema.parse(values);
  const replacementPolicy = parsed.datasetPolicy !== "require_exact_replay";
  const request: OptionsExpressionValidationCommandRequest = {
    artifact_path: parsed.artifactPath,
    options_dataset_id: replacementPolicy
      ? parsed.selectedDatasetId
      : parsed.activeArtifactOptionsDatasetId || parsed.selectedDatasetId,
    dataset_policy: parsed.datasetPolicy,
    tuple_selection: {
      include_sealed_finalists: parsed.includeSealedFinalists,
      include_robust_leader: parsed.includeRobustLeader,
      include_holdout_leader: parsed.includeHoldoutLeader,
      tuple_indexes: parseIntegerList(parsed.tupleIndexesInput) as number[],
    },
    validation_plan: {
      mode: parsed.validationPlanMode,
    },
    min_valid_trades: parseOptionalNonNegativeInteger(
      parsed.minValidTradesInput,
      "Min valid trades",
    ) as number | null,
  };

  if (parsed.validationPlanMode === "date_range_holdout") {
    request.validation_plan.start_timestamp_ms = parseUtcDateInputToMs(
      parsed.startDate,
      "Validation start date",
    ) as number;
    request.validation_plan.end_timestamp_ms = parseUtcDateInputToMs(
      parsed.endDate,
      "Validation end date",
    ) as number;
  }

  if (
    parsed.validationPlanMode === "anchored_walk_forward" ||
    parsed.validationPlanMode === "rolling_session_walk_forward"
  ) {
    request.validation_plan.train_session_count =
      parseRequiredPositiveIntegerInput(
        parsed.trainSessionCountInput,
        "Train sessions",
      ) as number;
    request.validation_plan.validation_session_count =
      parseRequiredPositiveIntegerInput(
        parsed.validationSessionCountInput,
        "Validation sessions",
      ) as number;
    request.validation_plan.step_session_count =
      parseRequiredPositiveIntegerInput(
        parsed.stepSessionCountInput,
        "Step sessions",
      ) as number;
  }

  return request;
}

export function createOptionsValidationFormOptions(
  options: OptionsValidationFormFactoryOptions = {},
) {
  return {
    defaultValues: createOptionsValidationFormValues(options.defaultValues),
    validators: {
      onChange: ({ value }: { value: OptionsValidationFormValues }) =>
        validateOptionsValidationFormValues(value),
      onSubmit: ({ value }: { value: OptionsValidationFormValues }) =>
        validateOptionsValidationFormValues(value),
    },
    onSubmit: async ({ value }: { value: OptionsValidationFormValues }) => {
      const parsed = optionsValidationFormSchema.parse(value);
      await options.onSubmit?.(parsed);
    },
  };
}

export function useOptionsValidationForm(
  options: OptionsValidationFormFactoryOptions = {},
) {
  return useForm(createOptionsValidationFormOptions(options));
}
