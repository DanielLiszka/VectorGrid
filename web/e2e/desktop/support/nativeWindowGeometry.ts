import { execFile } from 'node:child_process'
import path from 'node:path'
import { promisify } from 'node:util'
import { desktopAppBinaryPath } from './appPaths.js'

const execFileAsync = promisify(execFile)
const TARGET_PATH_PLACEHOLDER = '__VECTORGRID_TARGET_PATH__'

const POWERSHELL_GEOMETRY_SCRIPT = `
$ErrorActionPreference = 'Stop'
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VectorGridNativeWindowGeometry {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [DllImport("user32.dll", SetLastError = true)]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
}
"@

$targetPath = [System.IO.Path]::GetFullPath('${TARGET_PATH_PLACEHOLDER}')
$process = Get-Process | Where-Object {
  $_.MainWindowHandle -ne 0 -and $_.Path -eq $targetPath
} | Sort-Object StartTime -Descending | Select-Object -First 1

if (-not $process) {
  throw "No desktop window is currently running for $targetPath"
}

$rect = New-Object VectorGridNativeWindowGeometry+RECT
if (-not [VectorGridNativeWindowGeometry]::GetWindowRect([IntPtr]$process.MainWindowHandle, [ref]$rect)) {
  throw "GetWindowRect failed for handle $($process.MainWindowHandle)"
}

[pscustomobject]@{
  processId = $process.Id
  handle = ('0x{0:X}' -f $process.MainWindowHandle)
  left = $rect.Left
  top = $rect.Top
  right = $rect.Right
  bottom = $rect.Bottom
  width = $rect.Right - $rect.Left
  height = $rect.Bottom - $rect.Top
} | ConvertTo-Json -Compress
`

export interface NativeWindowRect {
  processId: number
  handle: string
  left: number
  top: number
  right: number
  bottom: number
  width: number
  height: number
}

export async function getNativeAppWindowRect(): Promise<NativeWindowRect> {
  const targetPath = path.resolve(desktopAppBinaryPath).replace(/'/g, "''")
  const script = POWERSHELL_GEOMETRY_SCRIPT.replace(TARGET_PATH_PLACEHOLDER, targetPath)
  const { stdout } = await execFileAsync(
    'powershell.exe',
    ['-NoProfile', '-Command', script],
    {
      encoding: 'utf8',
      maxBuffer: 1024 * 1024,
      windowsHide: true,
    },
  )

  return JSON.parse(stdout) as NativeWindowRect
}
