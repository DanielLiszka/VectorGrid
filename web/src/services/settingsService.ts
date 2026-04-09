import { invokeCommand } from '../tauri/invoke'
import type { WindowEffect } from '../utils/windowEffects'

export async function setWindowEffect(effect: WindowEffect, dark: boolean): Promise<void> {
  await invokeCommand('set_window_effect', {
    effect: effect === 'none' ? null : effect,
    dark: !!dark,
  })
}

export async function openRunsDir(): Promise<void> {
  await invokeCommand('open_runs_dir')
}

export async function openReportsDir(): Promise<void> {
  await invokeCommand('open_reports_dir')
}
