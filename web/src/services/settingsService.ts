import { invokeCommand } from '../tauri/invoke'

export type WindowEffect = 'none' | 'mica' | 'acrylic' | 'vibrancy'

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
