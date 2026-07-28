export interface PixelSize {
  width: number
  height: number
}

const MAXIMIZE_TOLERANCE_PX = 2

export function maximizedBoundsExceedMonitor(
  windowSize: PixelSize,
  monitorSize: PixelSize,
): boolean {
  if (!Number.isFinite(windowSize.width) || !Number.isFinite(windowSize.height)) return true
  if (!Number.isFinite(monitorSize.width) || !Number.isFinite(monitorSize.height)) return true
  if (windowSize.width <= 0 || windowSize.height <= 0) return true
  if (monitorSize.width <= 0 || monitorSize.height <= 0) return true
  return windowSize.width > monitorSize.width + MAXIMIZE_TOLERANCE_PX
    || windowSize.height > monitorSize.height + MAXIMIZE_TOLERANCE_PX
}
