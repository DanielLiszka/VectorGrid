import { describe, expect, it, vi } from "vitest";
import {
  createOptionsValidationFormOptions,
  createOptionsValidationFormValues,
  optionsValidationFormSchema,
  toOptionsValidationCommandRequest,
  validateOptionsValidationFormValues,
} from "./optionsValidationForm";

describe("optionsValidationForm", () => {
  it("normalizes production defaults for TanStack Form", () => {
    expect(
      createOptionsValidationFormValues({
        artifactPath: " C:/runs/options.json ",
      }),
    ).toMatchObject({
      artifactPath: "C:/runs/options.json",
      datasetPolicy: "require_exact_replay",
      validationPlanMode: "original_embedded_splits",
      trainSessionCountInput: "5",
      minValidTradesInput: "",
      includeSealedFinalists: true,
    });
  });

  it("validates required artifact path", () => {
    const errors = validateOptionsValidationFormValues(
      createOptionsValidationFormValues(),
    );

    expect(errors?.fields.artifactPath).toBe(
      "Load or run an options expression artifact before validation.",
    );
  });

  it("requires replacement dataset selection for replacement policies", () => {
    const errors = validateOptionsValidationFormValues(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        datasetPolicy: "allow_same_root_replacement",
      }),
    );

    expect(errors?.fields.selectedDatasetId).toBe(
      "Select a Polygon.io replacement dataset before using a replacement validation policy.",
    );
  });

  it("requires valid date range fields for holdout mode", () => {
    const errors = validateOptionsValidationFormValues(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        validationPlanMode: "date_range_holdout",
        startDate: "2026-04-03",
        endDate: "2026-04-01",
      }),
    );

    expect(errors?.fields.endDate).toBe(
      "Validation start date must be before validation end date.",
    );
  });

  it("rejects impossible holdout dates instead of letting Date.parse roll them over", () => {
    const errors = validateOptionsValidationFormValues(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        validationPlanMode: "date_range_holdout",
        startDate: "2026-02-31",
        endDate: "2026-03-05",
      }),
    );

    expect(errors?.fields.startDate).toBe(
      "Validation start date must be a valid YYYY-MM-DD date.",
    );
  });

  it("requires positive session counts for walk-forward modes", () => {
    const errors = validateOptionsValidationFormValues(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        validationPlanMode: "rolling_session_walk_forward",
        trainSessionCountInput: "0",
        validationSessionCountInput: "1",
        stepSessionCountInput: "1",
      }),
    );

    expect(errors?.fields.trainSessionCountInput).toBe(
      "Train sessions must be a positive whole number.",
    );
  });

  it("builds exact-replay validation command requests with tuple selection", () => {
    const request = toOptionsValidationCommandRequest(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        activeArtifactOptionsDatasetId: "spy_options",
        selectedDatasetId: "replacement_ignored",
        tupleIndexesInput: "3, 1, 3",
        minValidTradesInput: "",
        includeHoldoutLeader: false,
      }),
    );

    expect(request).toEqual({
      artifact_path: "C:/runs/options.json",
      options_dataset_id: "spy_options",
      dataset_policy: "require_exact_replay",
      tuple_selection: {
        include_sealed_finalists: true,
        include_robust_leader: true,
        include_holdout_leader: false,
        tuple_indexes: [1, 3],
      },
      validation_plan: {
        mode: "original_embedded_splits",
      },
      min_valid_trades: null,
    });
  });

  it("rejects malformed tuple index selections", () => {
    const values = createOptionsValidationFormValues({
      artifactPath: "C:/runs/options.json",
      activeArtifactOptionsDatasetId: "spy_options",
      tupleIndexesInput: "3, 1.5, nope",
    });

    expect(
      validateOptionsValidationFormValues(values)?.fields.tupleIndexesInput,
    ).toBe("Tuple indexes must be comma-separated non-negative whole numbers.");
    expect(() => toOptionsValidationCommandRequest(values)).toThrow(
      "Tuple indexes must be comma-separated non-negative whole numbers.",
    );
  });

  it("builds date-range holdout validation command requests with UTC timestamps", () => {
    const request = toOptionsValidationCommandRequest(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        activeArtifactOptionsDatasetId: "spy_options",
        validationPlanMode: "date_range_holdout",
        startDate: "2026-04-01",
        endDate: "2026-04-03",
        minValidTradesInput: "4",
      }),
    );

    expect(request.validation_plan).toEqual({
      mode: "date_range_holdout",
      start_timestamp_ms: Date.parse("2026-04-01T00:00:00.000Z"),
      end_timestamp_ms: Date.parse("2026-04-03T00:00:00.000Z"),
    });
    expect(request.min_valid_trades).toBe(4);
  });

  it("builds session walk-forward validation command requests", () => {
    const request = toOptionsValidationCommandRequest(
      createOptionsValidationFormValues({
        artifactPath: "C:/runs/options.json",
        activeArtifactOptionsDatasetId: "spy_options",
        validationPlanMode: "anchored_walk_forward",
        trainSessionCountInput: "8",
        validationSessionCountInput: "2",
        stepSessionCountInput: "2",
      }),
    );

    expect(request.validation_plan).toEqual({
      mode: "anchored_walk_forward",
      train_session_count: 8,
      validation_session_count: 2,
      step_session_count: 2,
    });
  });

  it("exposes TanStack Form-compatible validators and submit handler", async () => {
    const onSubmit = vi.fn();
    const formOptions = createOptionsValidationFormOptions({
      defaultValues: {
        artifactPath: "C:/runs/options.json",
        activeArtifactOptionsDatasetId: "spy_options",
      },
      onSubmit,
    });

    const changeResult = formOptions.validators.onChange({
      value: formOptions.defaultValues,
    });
    await formOptions.onSubmit({ value: formOptions.defaultValues });

    expect(changeResult).toBeUndefined();
    expect(onSubmit).toHaveBeenCalledWith(
      optionsValidationFormSchema.parse(formOptions.defaultValues),
    );
  });
});
