import { invokeCommand } from '../tauri/invoke'

function normalizePath(path: unknown): string {
  return String(path ?? '').trim()
}

export type OpenOrRevealPathResult = {
  path: string
  openedWith: 'open_path' | 'reveal_path'
}

export async function openPathWithRevealFallback(path: unknown): Promise<OpenOrRevealPathResult> {
  const nextPath = normalizePath(path)
  if (!nextPath) {
    throw new Error('Path is required.')
  }
  try {
    await invokeCommand('open_path', { path: nextPath })
    return { path: nextPath, openedWith: 'open_path' }
  } catch {
    await invokeCommand('reveal_path', { path: nextPath })
    return { path: nextPath, openedWith: 'reveal_path' }
  }
}
