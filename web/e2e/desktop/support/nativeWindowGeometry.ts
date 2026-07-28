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

  [StructLayout(LayoutKind.Sequential)]
  public struct MONITORINFO {
    public int cbSize;
    public RECT rcMonitor;
    public RECT rcWork;
    public int dwFlags;
  }

  [DllImport("user32.dll", SetLastError = true)]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool IsZoomed(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern IntPtr MonitorFromWindow(IntPtr hWnd, uint dwFlags);

  [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool GetMonitorInfo(IntPtr hMonitor, ref MONITORINFO monitorInfo);
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

$monitorHandle = [VectorGridNativeWindowGeometry]::MonitorFromWindow(
  [IntPtr]$process.MainWindowHandle,
  2
)
if ($monitorHandle -eq [IntPtr]::Zero) {
  throw "MonitorFromWindow failed for handle $($process.MainWindowHandle)"
}

$monitorInfo = New-Object VectorGridNativeWindowGeometry+MONITORINFO
$monitorInfo.cbSize = [Runtime.InteropServices.Marshal]::SizeOf(
  [type][VectorGridNativeWindowGeometry+MONITORINFO]
)
if (-not [VectorGridNativeWindowGeometry]::GetMonitorInfo($monitorHandle, [ref]$monitorInfo)) {
  throw "GetMonitorInfo failed for handle $($process.MainWindowHandle)"
}

[pscustomobject]@{
  window = [pscustomobject]@{
    processId = $process.Id
    handle = ('0x{0:X}' -f $process.MainWindowHandle)
    left = $rect.Left
    top = $rect.Top
    right = $rect.Right
    bottom = $rect.Bottom
    width = $rect.Right - $rect.Left
    height = $rect.Bottom - $rect.Top
    maximized = [VectorGridNativeWindowGeometry]::IsZoomed([IntPtr]$process.MainWindowHandle)
  }
  monitor = [pscustomobject]@{
    left = $monitorInfo.rcMonitor.Left
    top = $monitorInfo.rcMonitor.Top
    right = $monitorInfo.rcMonitor.Right
    bottom = $monitorInfo.rcMonitor.Bottom
    width = $monitorInfo.rcMonitor.Right - $monitorInfo.rcMonitor.Left
    height = $monitorInfo.rcMonitor.Bottom - $monitorInfo.rcMonitor.Top
  }
  workArea = [pscustomobject]@{
    left = $monitorInfo.rcWork.Left
    top = $monitorInfo.rcWork.Top
    right = $monitorInfo.rcWork.Right
    bottom = $monitorInfo.rcWork.Bottom
    width = $monitorInfo.rcWork.Right - $monitorInfo.rcWork.Left
    height = $monitorInfo.rcWork.Bottom - $monitorInfo.rcWork.Top
  }
} | ConvertTo-Json -Compress
`

export interface NativeRect {
  left: number
  top: number
  right: number
  bottom: number
  width: number
  height: number
}

export interface NativeWindowRect extends NativeRect {
  processId: number
  handle: string
  maximized: boolean
}

export interface NativeWindowGeometry {
  window: NativeWindowRect
  monitor: NativeRect
  workArea: NativeRect
}

export function nativeRectWithinBounds(
  rect: NativeRect,
  bounds: NativeRect,
  tolerance = 0,
): boolean {
  return rect.left >= bounds.left - tolerance
    && rect.top >= bounds.top - tolerance
    && rect.right <= bounds.right + tolerance
    && rect.bottom <= bounds.bottom + tolerance
}

export async function getNativeAppWindowGeometry(): Promise<NativeWindowGeometry> {
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

  return JSON.parse(stdout) as NativeWindowGeometry
}

export async function getNativeAppWindowRect(): Promise<NativeWindowRect> {
  return (await getNativeAppWindowGeometry()).window
}
