import { resolveRuntimePinia } from "../../stores/runtimePinia";
import { useWorkflowStore } from "../../stores/workflowStore";
import { workflowStepForMlSignalStage } from "./workflowModel";

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

export function isMlSignalStrategyKind(value: unknown): boolean {
  return normalizeText(value) === "MlSignal";
}

function isMlSignalArtifactContext(
  req: Record<string, unknown> | null | undefined,
  mlSignalStage?: unknown,
): boolean {
  return (
    isMlSignalStrategyKind(req?.strategy_kind ?? req?.strategyKind) ||
    normalizeText(mlSignalStage).length > 0
  );
}

function hasMlSignalDatasetExportShape(
  req: Record<string, unknown> | null | undefined,
): boolean {
  return (
    normalizeText(req?.ma_source ?? req?.maSource).length > 0 ||
    Number.isFinite(Number(req?.horizon_bars ?? req?.horizonBars))
  );
}

function hasMlSignalValidationShape(
  validationReq: Record<string, unknown> | null | undefined,
): boolean {
  return (
    !!validationReq &&
    Array.isArray(validationReq.candidates) &&
    validationReq.candidates.length > 0
  );
}

function hasMlSignalSweepShape(
  validationReq: Record<string, unknown> | null | undefined,
): boolean {
  if (!validationReq || Array.isArray(validationReq.candidates)) return false;
  return (
    validationReq.t_long != null ||
    validationReq.t_short != null ||
    validationReq.delta != null ||
    validationReq.max_hold_bars != null
  );
}

function inferMlSignalWorkflowStep(
  req: Record<string, unknown> | null | undefined,
  validationReq?: Record<string, unknown> | null,
  mlSignalStage?: unknown,
): "data" | "optimize" | "validate" | "analyze" | "finalize" {
  const normalizedStage = normalizeText(mlSignalStage);
  if (normalizedStage) return workflowStepForMlSignalStage(normalizedStage);
  if (hasMlSignalDatasetExportShape(req)) return "data";
  if (hasMlSignalValidationShape(validationReq)) return "validate";
  if (hasMlSignalSweepShape(validationReq)) return "analyze";
  return "optimize";
}

function inferConventionalWorkflowStep(
  validationReq?: Record<string, unknown> | null,
): "optimize" | "validate" {
  if (!validationReq) return "optimize";
  if (Array.isArray(validationReq.candidates)) {
    return validationReq.candidates.length > 0 ? "validate" : "optimize";
  }
  return Object.keys(validationReq).length > 0 ? "validate" : "optimize";
}

export interface ApplyShellStateOptions {
  conventionalTarget?: "mode-only" | "workflow-step";
}

export function applyShellStateForRunRequest(
  req: Record<string, unknown> | null | undefined,
  validationReq?: Record<string, unknown> | null,
  mlSignalStage?: unknown,
  options: ApplyShellStateOptions = {},
): void {
  const pinia = resolveRuntimePinia();
  if (!pinia) return;
  const workflowStore = useWorkflowStore(pinia);
  if (isMlSignalArtifactContext(req, mlSignalStage)) {
    workflowStore.selectMode("ml-signal");
    workflowStore.selectWorkflowStep(
      inferMlSignalWorkflowStep(req, validationReq, mlSignalStage),
    );
    return;
  }

  workflowStore.selectMode("conventional");
  if (options.conventionalTarget !== "mode-only") {
    workflowStore.selectWorkflowStep(
      inferConventionalWorkflowStep(validationReq),
    );
  }
}
