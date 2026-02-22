export type TauriRuntimeErrorCode =
  | 'TAURI_COMMAND_UNKNOWN'
  | 'TAURI_INVOKE_FAILED'
  | 'TAURI_EVENT_LISTEN_FAILED'

export class TauriRuntimeError extends Error {
  readonly code: TauriRuntimeErrorCode
  readonly operation: string
  readonly causeValue: unknown

  constructor(params: {
    code: TauriRuntimeErrorCode
    operation: string
    message: string
    causeValue?: unknown
  }) {
    super(params.message)
    this.name = 'TauriRuntimeError'
    this.code = params.code
    this.operation = params.operation
    this.causeValue = params.causeValue
  }
}

function stringifyCause(cause: unknown): string {
  if (cause instanceof Error) {
    const text = String(cause.message || '').trim()
    if (text) return text
  }
  if (typeof cause === 'string') {
    const text = cause.trim()
    if (text) return text
  }
  if (cause === null || cause === undefined) return ''
  try {
    const json = JSON.stringify(cause)
    if (json && json !== '{}' && json !== '[]') return json
  } catch {
    // Ignore serialization errors and fall through to default.
  }
  return ''
}

export function normalizeTauriRuntimeError(params: {
  code: TauriRuntimeErrorCode
  operation: string
  fallbackMessage: string
  causeValue?: unknown
}): TauriRuntimeError {
  if (params.causeValue instanceof TauriRuntimeError) return params.causeValue
  const causeMessage = stringifyCause(params.causeValue)
  const message = causeMessage
    ? `${params.fallbackMessage} Cause: ${causeMessage}`
    : params.fallbackMessage
  return new TauriRuntimeError({
    code: params.code,
    operation: params.operation,
    message,
    causeValue: params.causeValue,
  })
}
