import { browser } from '@wdio/globals'

type NativeInvokeEnvelope<T> = {
  ok: boolean
  value?: T
  error?: string
}

function errorText(value: unknown): string {
  if (value instanceof Error) return value.message
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

export async function invokeNative<T = unknown>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  const normalizedCommand = String(command || '').trim()
  if (!/^[a-z][a-z0-9_]{1,127}$/.test(normalizedCommand)) {
    throw new Error(`Refusing an invalid native command name: ${JSON.stringify(command)}`)
  }
  const request = JSON.stringify({
    command: normalizedCommand,
    args,
  })
  const serialized = await browser.executeAsync(
    (
      serializedRequest: string,
      done: (value: string) => void,
    ) => {
      const parsed = JSON.parse(serializedRequest) as {
        command: string
        args: Record<string, unknown>
      }
      const root = globalThis as unknown as {
        __TAURI_INTERNALS__?: {
          invoke?: (command: string, args: Record<string, unknown>) => Promise<unknown>
        }
      }
      const invoke = root.__TAURI_INTERNALS__?.invoke
      if (typeof invoke !== 'function') {
        done(JSON.stringify({
          ok: false,
          error: 'Tauri native invoke bridge is unavailable.',
        }))
        return
      }
      Promise.resolve(invoke(parsed.command, parsed.args))
        .then((value) => {
          done(JSON.stringify({ ok: true, value }))
        })
        .catch((error: unknown) => {
          let detail = ''
          if (typeof error === 'string') detail = error
          else {
            try {
              detail = JSON.stringify(error)
            } catch {
              detail = String(error)
            }
          }
          done(JSON.stringify({ ok: false, error: detail || 'Native command failed.' }))
        })
    },
    request,
  ) as unknown as string

  let envelope: NativeInvokeEnvelope<T>
  try {
    envelope = JSON.parse(serialized) as NativeInvokeEnvelope<T>
  } catch (error) {
    throw new Error(
      `Native command ${normalizedCommand} returned a malformed harness envelope: ${errorText(error)}`,
    )
  }
  if (!envelope.ok) {
    throw new Error(
      `Native command ${normalizedCommand} failed: ${String(envelope.error || 'unknown error')}`,
    )
  }
  return envelope.value as T
}

export async function captureNativeFailure(
  command: string,
  args: Record<string, unknown> = {},
): Promise<string> {
  try {
    await invokeNative(command, args)
  } catch (error) {
    return errorText(error)
  }
  throw new Error(`Expected native command ${command} to fail, but it succeeded.`)
}
