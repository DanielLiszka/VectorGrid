export type AnalysisExecutionInvalidator = () => void;

let registeredInvalidator: AnalysisExecutionInvalidator | null = null;

export function registerAnalysisExecutionInvalidator(
  invalidator: AnalysisExecutionInvalidator,
): () => void {
  registeredInvalidator = invalidator;
  return () => {
    if (registeredInvalidator === invalidator) {
      registeredInvalidator = null;
    }
  };
}

export function invalidateAnalysisExecutionForDatasetChange(): void {
  registeredInvalidator?.();
}
