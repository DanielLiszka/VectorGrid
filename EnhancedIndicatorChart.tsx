// src/components/charts/EnhancedIndicatorChart.tsx
import { useEffect, useRef, useState, useCallback, startTransition } from 'react';
import {
  createChart,
  ColorType,
  CrosshairMode,
  CandlestickSeries,
  LineSeries,
  HistogramSeries,
  AreaSeries,
  BarSeries,
  PriceScaleMode,
  createSeriesMarkers,
  type ISeriesMarkersPluginApi,
  type SeriesMarker,
  type IChartApi,
  type ISeriesApi,
  type UTCTimestamp,
  type Time,
  type SeriesType,
} from 'lightweight-charts';
import { talib } from '../../lib/wasm';
import type { OHLCV } from '../../lib/utils/data-loader.server';
import { IndicatorLegend } from './IndicatorLegend';
import { ChartTypeSwitcher, type ChartType } from './ChartTypeSwitcher';
import { ChartContextMenu } from './ChartContextMenu';
// import { DrawingToolbar } from './DrawingToolbar';
// import { DrawingToolsPlugin } from './plugins/DrawingTools';
import { useChartTouchGestures } from '../../hooks/useChartTouchGestures';
import { MobileChartControls } from './MobileChartControls';
import { MobileIndicatorLegend } from './MobileIndicatorLegend';
import { useIsMobile } from '../../hooks/useMediaQuery';
import { analytics } from '../../lib/goatcounter';
import '../../styles/chart-mobile.css';

const INDICATOR_COLOR_STORAGE_PREFIX = 'va.indicator.color.';

// TradingView-inspired defaults (kept intentionally small and distinct).
// These defaults are only used for single-output indicators where users can
// override/persist a preferred color.
const INDICATOR_COLOR_PALETTE = [
  '#2962FF', // blue
  '#089981', // teal/green
  '#9C27B0', // purple
  '#787B86', // gray
];

const isHexColor = (value: unknown): value is string =>
  typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value);

const hashString = (input: string): number => {
  // Simple deterministic hash (unsigned 32-bit) for stable palette selection.
  let hash = 0;
  for (let i = 0; i < input.length; i++) {
    hash = (hash * 31 + input.charCodeAt(i)) >>> 0;
  }
  return hash;
};

const defaultColorForIndicator = (indicatorId: string): string =>
  INDICATOR_COLOR_PALETTE[hashString(indicatorId) % INDICATOR_COLOR_PALETTE.length];

const readSavedIndicatorColor = (indicatorId: string): string | null => {
  try {
    if (typeof window === 'undefined') return null;
    const saved = localStorage.getItem(`${INDICATOR_COLOR_STORAGE_PREFIX}${indicatorId}`);
    return isHexColor(saved) ? saved : null;
  } catch {
    return null;
  }
};

const writeSavedIndicatorColor = (indicatorId: string, color: string) => {
  try {
    if (typeof window === 'undefined') return;
    if (!isHexColor(color)) return;
    localStorage.setItem(`${INDICATOR_COLOR_STORAGE_PREFIX}${indicatorId}`, color);
  } catch {
    // ignore storage write issues
  }
};

interface TimeframeOption {
  label: string;
  value: string;
  seconds: number;
}

interface EnhancedIndicatorChartProps {
  indicatorId: string;
  indicatorName: string;
  parameters?: Record<string, any>;
  isOscillator?: boolean;
  customData?: OHLCV[];
  height?: number;
  externalTimeframe?: string;
  onTimeframeChange?: (timeframe: string) => void;
}

const TIMEFRAME_OPTIONS: TimeframeOption[] = [
  { label: '1m', value: '1m', seconds: 60 },
  { label: '2m', value: '2m', seconds: 120 },
  { label: '3m', value: '3m', seconds: 180 },
  { label: '5m', value: '5m', seconds: 300 },
  { label: '15m', value: '15m', seconds: 900 },
  { label: '30m', value: '30m', seconds: 1800 },
  { label: '45m', value: '45m', seconds: 2700 },
  { label: '1h', value: '1h', seconds: 3600 },
  { label: '2h', value: '2h', seconds: 7200 },
  { label: '4h', value: '4h', seconds: 14400 },
];

// CSV helpers (header once, escaped values, BOM for Excel)
const escapeCsv = (v: unknown): string => {
  const s = v === null || v === undefined ? '' : String(v);
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
};

const buildCsv = (
  candles: OHLCV[],
  indicatorName: string,
  indicatorValues: any
): string => {
  const baseHeaders = ['Time', 'Open', 'High', 'Low', 'Close', 'Volume'];
  const headers: string[] = [...baseHeaders];
  const rows: string[][] = [];

  if (Array.isArray(indicatorValues)) {
    headers.push(indicatorName);
  } else if (indicatorValues && typeof indicatorValues === 'object') {
    Object.keys(indicatorValues).forEach((key) => {
      const title = key.charAt(0).toUpperCase() + key.slice(1);
      if (!headers.includes(title)) headers.push(title);
    });
  }

  for (let i = 0; i < candles.length; i++) {
    const d = candles[i];
    const row: (string | number)[] = [
      new Date(d.time * 1000).toISOString(),
      d.open,
      d.high,
      d.low,
      d.close,
      d.volume ?? 0,
    ];

    if (Array.isArray(indicatorValues)) {
      row.push(indicatorValues[i] ?? '');
    } else if (indicatorValues && typeof indicatorValues === 'object') {
      Object.keys(indicatorValues).forEach((key) => {
        const arr = (indicatorValues as any)[key] ?? [];
        row.push(arr[i] ?? '');
      });
    }
    rows.push(row.map(escapeCsv));
  }

  const csv = [headers.map(escapeCsv).join(','), ...rows.map((r) => r.join(','))].join('\n');
  return '\uFEFF' + csv; // BOM for Excel
};


// Calculate Heikin Ashi candles
function calculateHeikinAshi(data: OHLCV[]): any[] {
  if (data.length === 0) return [];
  
  const haData: any[] = [];
  let prevHaCandle: any = null;
  
  data.forEach((candle, index) => {
    const haCandle: any = {
      time: candle.time as UTCTimestamp,
    };
    
    // Calculate HA Close: (Open + High + Low + Close) / 4
    haCandle.close = (candle.open + candle.high + candle.low + candle.close) / 4;
    
    // Calculate HA Open
    if (index === 0) {
      // First candle: (Open + Close) / 2
      haCandle.open = (candle.open + candle.close) / 2;
    } else {
      // Subsequent candles: (Previous HA Open + Previous HA Close) / 2
      haCandle.open = (prevHaCandle.open + prevHaCandle.close) / 2;
    }
    
    // Calculate HA High: Max of High, HA Open, HA Close
    haCandle.high = Math.max(candle.high, haCandle.open, haCandle.close);
    
    // Calculate HA Low: Min of Low, HA Open, HA Close
    haCandle.low = Math.min(candle.low, haCandle.open, haCandle.close);
    
    haData.push(haCandle);
    prevHaCandle = haCandle;
  });
  
  return haData;
}

// Calculate ATR for dynamic Renko brick size
function calculateATR(data: OHLCV[], period: number = 14): number {
  if (data.length < period + 1) return 0;
  
  const trueRanges: number[] = [];
  
  for (let i = 1; i <= period; i++) {
    const high = data[i].high;
    const low = data[i].low;
    const prevClose = data[i - 1].close;
    const tr = Math.max(
      high - low,
      Math.abs(high - prevClose),
      Math.abs(low - prevClose)
    );
    trueRanges.push(tr);
  }
  
  return trueRanges.reduce((sum, tr) => sum + tr, 0) / period;
}

// Calculate Renko bricks
function calculateRenko(data: OHLCV[], brickSizePercent: number = 0.25): any[] {
  if (data.length === 0) return [];
  
  // Calculate brick size as percentage of recent price range
  const recentData = data.slice(-500); // Use last 500 candles
  const avgPrice = recentData.reduce((sum, d) => sum + d.close, 0) / recentData.length;
  const brickSize = avgPrice * (brickSizePercent / 100); // Convert percentage to decimal
  
  const renkoBricks: any[] = [];
  let basePrice = data[0].close;
  let currentBrickLevel = Math.floor(basePrice / brickSize) * brickSize;
  let lastBrickTime = data[0].time;
  let lastBrickDirection = 0; // 1 for up, -1 for down
  
  // Track the last brick to ensure proper open/close relationship
  let lastBrick: any = null;
  
  for (let i = 0; i < data.length; i++) {
    const candle = data[i];
    const high = candle.high;
    const low = candle.low;
    const currentTime = candle.time;
    
    // Check for upward movement
    while (high >= currentBrickLevel + brickSize) {
      const newBrick = {
        time: currentTime as UTCTimestamp,
        open: lastBrick ? lastBrick.close : currentBrickLevel,
        high: currentBrickLevel + brickSize,
        low: currentBrickLevel,
        close: currentBrickLevel + brickSize,
      };
      
      renkoBricks.push(newBrick);
      lastBrick = newBrick;
      currentBrickLevel += brickSize;
      lastBrickDirection = 1;
      lastBrickTime = currentTime;
    }
    
    // Check for downward movement
    while (low <= currentBrickLevel - brickSize) {
      const newBrick = {
        time: currentTime as UTCTimestamp,
        open: lastBrick ? lastBrick.close : currentBrickLevel,
        high: currentBrickLevel,
        low: currentBrickLevel - brickSize,
        close: currentBrickLevel - brickSize,
      };
      
      renkoBricks.push(newBrick);
      lastBrick = newBrick;
      currentBrickLevel -= brickSize;
      lastBrickDirection = -1;
      lastBrickTime = currentTime;
    }
  }
  
  // Ensure unique timestamps by adding small increments
  for (let i = 1; i < renkoBricks.length; i++) {
    if (renkoBricks[i].time <= renkoBricks[i - 1].time) {
      renkoBricks[i].time = renkoBricks[i - 1].time + 60; // Add 1 minute
    }
  }
  
  return renkoBricks;
}


export function EnhancedIndicatorChart({
  indicatorId,
  indicatorName,
  parameters = {},
  isOscillator = false,
  customData,
  height = 600,
  externalTimeframe,
  onTimeframeChange,
}: EnhancedIndicatorChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const mainSeriesRef = useRef<ISeriesApi<SeriesType> | null>(null);
  const indicatorSeriesRef = useRef<ISeriesApi<SeriesType> | null>(null);
  const volumeSeriesRef = useRef<ISeriesApi<SeriesType> | null>(null);
  const indicatorSeriesArrayRef = useRef<ISeriesApi<SeriesType>[]>([]);
  const markersApiRef = useRef<ISeriesMarkersPluginApi<Time> | null>(null);
  // const drawingToolsRef = useRef<DrawingToolsPlugin | null>(null);
  
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Non-fatal indicator calculation issues (e.g., missing WASM binding)
  // should not hide the base price chart, so we track them separately.
  const [indicatorError, setIndicatorError] = useState<string | null>(null);
  const [isDark, setIsDark] = useState(false);
  const [chartData, setChartData] = useState<OHLCV[]>([]);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [internalTimeframe, setInternalTimeframe] = useState('4h');
  const selectedTimeframe = externalTimeframe || internalTimeframe;
  const [crosshairMode, setCrosshairMode] = useState<'normal' | 'magnet' | 'magnetOHLC'>('magnet');
  const [tooltipData, setTooltipData] = useState<any>(null);
  const [tooltipPosition, setTooltipPosition] = useState<{ x: number; y: number } | null>(null);
  const [indicatorValues, setIndicatorValues] = useState<any[]>([]);
  const [indicatorOutputMode, setIndicatorOutputMode] = useState<'unknown' | 'single' | 'multi'>('unknown');
  const [indicatorColor, setIndicatorColor] = useState(() => defaultColorForIndicator(indicatorId));
  const [currentParameters, setCurrentParameters] = useState(parameters);
  const [isLegendVisible, setIsLegendVisible] = useState(true);
  const [currentIndicatorValue, setCurrentIndicatorValue] = useState<number | null>(null);
  const [currentMultiIndicatorValues, setCurrentMultiIndicatorValues] = useState<Record<string, number> | null>(null);
  const [isIndicatorVisible, setIsIndicatorVisible] = useState(true);
  const [chartType, setChartType] = useState<ChartType>(
    indicatorId === 'heikin_ashi_candles' ? 'heikinashi' : 'candlestick'
  );
  // Nonce to force chart re-creation when needed (e.g., after exiting fullscreen)
  const [chartResetNonce, setChartResetNonce] = useState(0);

  // Persist legend visibility across indicator pages/sessions
  const LEGEND_VISIBILITY_KEY = 'va.indicator.legend.visible';
  useEffect(() => {
    try {
      const saved = localStorage.getItem(LEGEND_VISIBILITY_KEY);
      if (saved !== null) setIsLegendVisible(saved === 'true');
    } catch (_) {
      // ignore SSR or storage access issues
    }
  }, []);
  useEffect(() => {
    try {
      localStorage.setItem(LEGEND_VISIBILITY_KEY, String(isLegendVisible));
    } catch (_) {
      // ignore storage write issues
    }
  }, [isLegendVisible]);

  // Reset parameters when the indicator or its default params change
  useEffect(() => {
    setCurrentParameters(parameters);
  }, [indicatorId, parameters]);

  // Reset output-mode and default color when switching indicators.
  useEffect(() => {
    setIndicatorOutputMode('unknown');
    setIndicatorColor(defaultColorForIndicator(indicatorId));
  }, [indicatorId]);

  // Persist single-output indicator colors per-indicator across pages/sessions.
  // Multi-output indicators intentionally keep their original multi-line scheme.
  useEffect(() => {
    if (indicatorOutputMode !== 'single') return;
    const saved = readSavedIndicatorColor(indicatorId);
    setIndicatorColor(saved ?? defaultColorForIndicator(indicatorId));
  }, [indicatorId, indicatorOutputMode]);
  useEffect(() => {
    if (indicatorOutputMode !== 'single') return;
    writeSavedIndicatorColor(indicatorId, indicatorColor);
  }, [indicatorId, indicatorColor, indicatorOutputMode]);

  // Restore the legacy "main series" legend color for multi-output indicators.
  useEffect(() => {
    if (indicatorOutputMode !== 'multi') return;
    setIndicatorColor('#3b82f6');
  }, [indicatorId, indicatorOutputMode]);

  // Reset chart type when switching indicators so Heikin Ashi
  // loads as a Heikin Ashi base chart by default while others
  // continue to use standard candlesticks.
  useEffect(() => {
    setChartType(indicatorId === 'heikin_ashi_candles' ? 'heikinashi' as ChartType : 'candlestick');
  }, [indicatorId]);
  
  // Debug chart type changes
  useEffect(() => {
    console.log('Chart type state changed to:', chartType);
  }, [chartType]);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; isVisible: boolean }>({
    x: 0,
    y: 0,
    isVisible: false,
  });
  const [isCrosshairVisible, setIsCrosshairVisible] = useState(true);
  const [isLogScale, setIsLogScale] = useState(false);
  const [rightClickData, setRightClickData] = useState<{ x: number; y: number; time?: any; price?: number } | null>(null);

  const resolveCrosshairMode = useCallback(
    (mode: 'normal' | 'magnet' | 'magnetOHLC') => {
      switch (mode) {
        case 'normal':
          return CrosshairMode.Normal;
        case 'magnet':
          return CrosshairMode.Magnet;
        case 'magnetOHLC':
          return CrosshairMode.MagnetOHLC;
      }
    },
    []
  );
  
  // Detect if mobile
  const isMobile = useIsMobile();

  // Detect theme
  useEffect(() => {
    const checkTheme = () => {
      setIsDark(document.documentElement.classList.contains('dark'));
    };
    
    checkTheme();
    const observer = new MutationObserver(checkTheme);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    });
    
    return () => observer.disconnect();
  }, []);

  // Load data based on selected timeframe (abortable)
  useEffect(() => {
    let aborted = false;
    const ac = new AbortController();
    const loadData = async () => {
      try {
        setIsLoading(true);
        setError(null);

        if (customData) {
          if (!aborted) setChartData(customData);
          return;
        }

        const base = import.meta.env.BASE_URL || '/';
        const filename = `ohlcv-${selectedTimeframe}.json`;
        const jsonPath = base.endsWith('/') ? `${base}data/${filename}` : `${base}/data/${filename}`;

        const response = await fetch(jsonPath, { signal: ac.signal });
        if (response.ok) {
          const candles = await response.json();
          if (!aborted) setChartData(candles);
        } else {
          console.warn(`Timeframe data not found for ${selectedTimeframe}, using 4h data`);
          const fallbackPath = base.endsWith('/') ? `${base}data/ohlcv-4h.json` : `${base}/data/ohlcv-4h.json`;
          const fallbackResponse = await fetch(fallbackPath, { signal: ac.signal });
          if (fallbackResponse.ok) {
            const candles = await fallbackResponse.json();
            if (!aborted) setChartData(candles);
          } else {
            throw new Error(`Failed to load data: ${response.status}`);
          }
        }
      } catch (err: any) {
        if (err?.name !== 'AbortError') {
          console.error('Failed to load data', err);
          if (!aborted) setError(err instanceof Error ? err.message : 'Failed to load chart data');
        }
      } finally {
        if (!aborted) setIsLoading(false);
      }
    };

    loadData();
    return () => { aborted = true; ac.abort(); };
  }, [customData, selectedTimeframe]);

  // Handle fullscreen
  const toggleFullscreen = useCallback(() => {
    if (!document.fullscreenElement) {
      containerRef.current?.requestFullscreen();
      setIsFullscreen(true);
    } else {
      document.exitFullscreen();
      setIsFullscreen(false);
    }
  }, []);

  // Handle fullscreen changes with explicit chart resize to avoid overflow
  useEffect(() => {
    const handleFullscreenChange = () => {
      const nowFullscreen = !!document.fullscreenElement;
      setIsFullscreen(nowFullscreen);

      const chart = chartRef.current;
      const container = chartContainerRef.current as HTMLElement | null;
      if (!chart || !container) return;

      // Ensure the container never exceeds its parent bounds after toggling
      container.style.maxWidth = '100%';
      container.style.width = '100%';
      container.style.overflow = 'hidden';

      // Defer resize to allow DOM to settle
      const adjustLayout = () => {
        try {
          const targetWidth = Math.floor(container.clientWidth);
          const targetHeight = Math.floor(container.clientHeight || (nowFullscreen ? window.innerHeight : (typeof height === 'number' ? height : 600)));
          if (targetWidth > 0 && targetHeight > 0) {
            chart.resize(targetWidth, targetHeight);
            chart.timeScale().fitContent();
          }
          // Reset any horizontal page scroll that may have occurred
          if (!nowFullscreen) {
            try {
              // Force container reflow to discard stale dimensions
              const prevDisplay = container.style.display;
              container.style.display = 'none';
              // Force reflow
              // eslint-disable-next-line @typescript-eslint/no-unused-expressions
              (container as any).offsetHeight;
              container.style.display = prevDisplay || '';

              // Constrain descendant chart element explicitly
              const chartEl = container.querySelector('.tv-lightweight-charts') as HTMLElement | null;
              if (chartEl) {
                chartEl.style.width = '100%';
                chartEl.style.maxWidth = '100%';
                chartEl.style.left = '0';
                chartEl.style.right = 'auto';
                chartEl.style.position = 'relative';
                chartEl.style.boxSizing = 'border-box';
              }

              if (document.scrollingElement && (document.scrollingElement.scrollLeft || 0) !== 0) {
                document.scrollingElement.scrollLeft = 0;
              }
              if (window.scrollX !== 0) {
                window.scrollTo({ left: 0, top: window.scrollY });
              }
              document.documentElement.style.overflowX = 'hidden';
              document.body.style.overflowX = 'hidden';
            } catch (_) {
              // no-op
            }
          }
        } catch (e) {
          console.debug('Fullscreen resize adjustment failed:', e);
        }
        // As a robust fallback, fully recreate the chart on exit
        if (!nowFullscreen) {
          setChartResetNonce((n) => n + 1);
        }
      };

      setTimeout(adjustLayout, 50);
      // Secondary pass in case layout settles slower
      if (!nowFullscreen) setTimeout(adjustLayout, 150);
    };

    document.addEventListener('fullscreenchange', handleFullscreenChange);
    // Vendor-prefixed fallbacks
    document.addEventListener('webkitfullscreenchange', handleFullscreenChange as any);
    document.addEventListener('MSFullscreenChange', handleFullscreenChange as any);
    return () => {
      document.removeEventListener('fullscreenchange', handleFullscreenChange);
      document.removeEventListener('webkitfullscreenchange', handleFullscreenChange as any);
      document.removeEventListener('MSFullscreenChange', handleFullscreenChange as any);
    };
  }, [height]);

  // Extra safety: resize chart when its container resizes
  useEffect(() => {
    if (!chartContainerRef.current || !chartRef.current) return;
    const el = chartContainerRef.current;
    const obs = new ResizeObserver((entries) => {
      const cr = entries[0]?.contentRect;
      if (!cr || !chartRef.current) return;
      const w = Math.floor(cr.width);
      const h = Math.floor(cr.height);
      if (w > 0 && h > 0) {
        chartRef.current.resize(w, h);
      }
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, [chartData.length]);

  // Handle keyboard shortcuts
  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      if (!chartRef.current) return;
      
      // Prevent defaults for our shortcuts
      if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End', '+', '-', '=', '_'].includes(e.key)) {
        e.preventDefault();
      }
      
      const timeScale = chartRef.current.timeScale();
      const visibleRange = timeScale.getVisibleRange();
      
      if (!visibleRange) return;
      
      switch (e.key) {
        case 'ArrowUp':
        case '+':
        case '=':
          // Zoom in
          const rangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
          const zoomInRange = {
            from: (Number(visibleRange.from) + rangeWidth * 0.1) as Time,
            to: (Number(visibleRange.to) - rangeWidth * 0.1) as Time,
          };
          timeScale.setVisibleRange(zoomInRange);
          analytics.trackChartInteraction('zoom', 'keyboard-in');
          break;
          
        case 'ArrowDown':
        case '-':
        case '_':
          // Zoom out
          const zoomOutRangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
          const zoomOutRange = {
            from: (Number(visibleRange.from) - zoomOutRangeWidth * 0.1) as Time,
            to: (Number(visibleRange.to) + zoomOutRangeWidth * 0.1) as Time,
          };
          timeScale.setVisibleRange(zoomOutRange);
          analytics.trackChartInteraction('zoom', 'keyboard-out');
          break;
          
        case 'ArrowLeft':
          // Pan left
          const panLeftRangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
          const panLeftRange = {
            from: (Number(visibleRange.from) - panLeftRangeWidth * 0.1) as Time,
            to: (Number(visibleRange.to) - panLeftRangeWidth * 0.1) as Time,
          };
          timeScale.setVisibleRange(panLeftRange);
          analytics.trackChartInteraction('pan', 'keyboard-left');
          break;
          
        case 'ArrowRight':
          // Pan right
          const panRightRangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
          const panRightRange = {
            from: (Number(visibleRange.from) + panRightRangeWidth * 0.1) as Time,
            to: (Number(visibleRange.to) + panRightRangeWidth * 0.1) as Time,
          };
          timeScale.setVisibleRange(panRightRange);
          analytics.trackChartInteraction('pan', 'keyboard-right');
          break;
          
        case 'Home':
          // Go to beginning
          timeScale.scrollToPosition(-1000, false);
          analytics.trackChartInteraction('pan', 'keyboard-home');
          break;
          
        case 'End':
          // Go to end
          timeScale.scrollToPosition(0, false);
          analytics.trackChartInteraction('pan', 'keyboard-end');
          break;
          
        case 'f':
          // Toggle fullscreen
          if (!e.ctrlKey && !e.metaKey) {
            toggleFullscreen();
            analytics.trackChartInteraction('zoom', 'fullscreen-toggle');
          }
          break;
      }
    };
    
    window.addEventListener('keydown', handleKeyPress);
    return () => window.removeEventListener('keydown', handleKeyPress);
  }, [toggleFullscreen]);

  // Handle timeframe change
  const handleTimeframeChange = useCallback((timeframe: string) => {
    if (onTimeframeChange) {
      onTimeframeChange(timeframe);
    } else {
      setInternalTimeframe(timeframe);
    }
    analytics.trackChartInteraction('timeframe-change', timeframe);
    // Data will be re-aggregated automatically via the useEffect hook
  }, [onTimeframeChange]);

  // Screenshot functionality
  const takeScreenshot = useCallback(() => {
    if (!chartRef.current) return;
    
    const canvas = chartRef.current.takeScreenshot(true, true);
    canvas.toBlob((blob) => {
      if (!blob) return;
      
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${indicatorName}-${new Date().toISOString()}.png`;
      a.click();
      URL.revokeObjectURL(url);
    });
  }, [indicatorName]);

  // Format time delta
  const formatTimeDelta = (start: UTCTimestamp, end: UTCTimestamp): string => {
    const deltaSeconds = Math.abs(end - start);
    const days = Math.floor(deltaSeconds / 86400);
    const hours = Math.floor((deltaSeconds % 86400) / 3600);
    const minutes = Math.floor((deltaSeconds % 3600) / 60);
    
    const parts = [];
    if (days > 0) parts.push(`${days}d`);
    if (hours > 0) parts.push(`${hours}h`);
    if (minutes > 0) parts.push(`${minutes}m`);
    
    return parts.length > 0 ? parts.join(' ') : '< 1m';
  };

  // Handle parameter changes
  const handleParameterChange = useCallback((newParams: Record<string, any>) => {
    // Use startTransition so recalculation doesn't block interactions
    startTransition(() => {
      setCurrentParameters(prev => ({ ...(prev || {}), ...(newParams || {}) }));
    });
  }, []);

  // Handle color change from floating panel
  const handleColorChange = useCallback((newColor: string) => {
    if (indicatorOutputMode !== 'single') return;
    setIndicatorColor(newColor);
  }, [indicatorOutputMode]);
  
  // Handle context menu
  const handleContextMenu = useCallback((e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    
    // Disable context menu for mobile users
    if (isMobile) return;
    
    if (!chartContainerRef.current || !chartRef.current) return;
    
    // Get mouse position relative to chart for data lookup
    const rect = chartContainerRef.current.getBoundingClientRect();
    const chartX = e.clientX - rect.left;
    const chartY = e.clientY - rect.top;
    
    // Try to get price and time at click position
    const timeScale = chartRef.current.timeScale();
    const coordinate = timeScale.coordinateToLogical(chartX);
    
    let clickData: any = { x: chartX, y: chartY };
    
    if (coordinate !== null && mainSeriesRef.current) {
      try {
        const data = mainSeriesRef.current.dataByIndex(Math.round(coordinate));
        if (data) {
          clickData.time = data.time;
          clickData.price = 'close' in data ? data.close : 'value' in data ? data.value : null;
        }
      } catch (e) {
        console.debug('Could not get price data at click position');
      }
    }
    
    setRightClickData(clickData);
    
    // For fixed positioned elements rendered via portal, we need the actual cursor position
    // If the event is coming from a canvas element or has been modified, use pageX/pageY
    const mouseX = e.pageX || e.clientX;
    const mouseY = e.pageY || e.clientY;
    
    setContextMenu({
      x: mouseX,
      y: mouseY,
      isVisible: true,
    });
  }, [isMobile]);
  
  // Handle context menu actions
  const handleContextMenuAction = useCallback((action: string) => {
    if (!chartRef.current) return;
    
    // Track the action
    analytics.trackChartInteraction('context-menu' as any, action);
    
    switch (action) {
      case 'reset':
        chartRef.current.timeScale().resetTimeScale();
        chartRef.current.priceScale('right').applyOptions({ autoScale: true });
        analytics.trackChartInteraction('reset', 'context-menu');
        break;
        
      case 'fit':
        chartRef.current.timeScale().fitContent();
        break;
        
      case 'toggle-crosshair': {
        const nextVisible = !isCrosshairVisible;
        setIsCrosshairVisible(nextVisible);
        const mode = !nextVisible
          ? CrosshairMode.Hidden
          : resolveCrosshairMode(crosshairMode);
        chartRef.current.applyOptions({ crosshair: { mode } });
        break;
      }
        
      case 'toggle-indicator':
        setIsIndicatorVisible(!isIndicatorVisible);
        break;
        
      case 'toggle-scale':
        setIsLogScale(!isLogScale);
        chartRef.current.priceScale('right').applyOptions({
          mode: !isLogScale ? PriceScaleMode.Logarithmic : PriceScaleMode.Normal,
        });
        break;
        
      case 'copy-price':
        if (rightClickData?.price) {
          navigator.clipboard.writeText(rightClickData.price.toString());
        }
        break;
        
      case 'screenshot':
        takeScreenshot();
        break;
        
      case 'export':
        if (chartData.length > 0) {
          const csv = buildCsv(chartData, indicatorName, indicatorValues);
          const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = `${indicatorId}-${new Date().toISOString().split('T')[0]}.csv`;
          a.click();
          URL.revokeObjectURL(url);
        }
        break;
    }
  }, [isCrosshairVisible, isLogScale, isIndicatorVisible, rightClickData, takeScreenshot, chartData, indicatorValues, indicatorName, indicatorId, crosshairMode, resolveCrosshairMode]);

  const applySeriesOrder = useCallback(() => {
    try {
      mainSeriesRef.current?.setSeriesOrder(0);
    } catch (_) {
      // ignore
    }
    try {
      volumeSeriesRef.current?.setSeriesOrder(1);
    } catch (_) {
      // ignore
    }
    indicatorSeriesArrayRef.current.forEach((series, idx) => {
      try {
        series.setSeriesOrder(10 + idx);
      } catch (_) {
        // ignore series that do not support ordering
      }
    });
  }, []);

  const ensureRsiPriceLines = useCallback(() => {
    if (indicatorId !== 'rsi') return;
    const series: any = indicatorSeriesRef.current;
    if (!series?.createPriceLine) return;

    try {
      const existingLines = typeof series.priceLines === 'function' ? series.priceLines() : [];
      if (Array.isArray(existingLines)) {
        existingLines.forEach((line: any) => {
          try {
            series.removePriceLine?.(line);
          } catch (_) {
            // ignore
          }
        });
      }
    } catch (_) {
      // ignore
    }

    try {
      series.createPriceLine({
        price: 30,
        color: 'rgba(34,197,94,0.4)',
        lineStyle: 2,
        lineWidth: 1,
        axisLabelVisible: true,
        title: 'Oversold',
      });
      series.createPriceLine({
        price: 70,
        color: 'rgba(239,68,68,0.4)',
        lineStyle: 2,
        lineWidth: 1,
        axisLabelVisible: true,
        title: 'Overbought',
      });
    } catch (e) {
      console.warn('Could not create RSI price lines:', e);
    }
  }, [indicatorId]);

  const updateSeriesMarkers = useCallback(
    (indicatorData: any) => {
      if (!mainSeriesRef.current || !markersApiRef.current) return;

      // Only RSI and HalfTrend currently render markers.
      // RSI uses overbought/oversold levels; HalfTrend uses buy/sell flips.
      if (indicatorId === 'rsi' && Array.isArray(indicatorData)) {
        const markers: any[] = [];
        const maxMarkers = 200;
        for (let i = 1; i < indicatorData.length && markers.length < maxMarkers; i++) {
          const value = indicatorData[i];
          const prev = indicatorData[i - 1];
          const candle = chartData[i];
          if (!candle || typeof value !== 'number' || typeof prev !== 'number') continue;

          if (prev < 70 && value >= 70) {
            markers.push({
              time: candle.time as UTCTimestamp,
              position: 'aboveBar',
              color: 'rgb(239,68,68)',
              shape: 'arrowDown',
              text: 'RSI 70',
            });
          } else if (prev > 30 && value <= 30) {
            markers.push({
              time: candle.time as UTCTimestamp,
              position: 'belowBar',
              color: 'rgb(34,197,94)',
              shape: 'arrowUp',
              text: 'RSI 30',
            });
          }
        }

        markersApiRef.current.setMarkers(markers);
        return;
      }

      if (
        indicatorId === 'halftrend' &&
        indicatorData &&
        typeof indicatorData === 'object' &&
        Array.isArray((indicatorData as any).buy_signal) &&
        Array.isArray((indicatorData as any).sell_signal)
      ) {
        const buy = (indicatorData as any).buy_signal as (number | null)[];
        const sell = (indicatorData as any).sell_signal as (number | null)[];

        const markers: any[] = [];
        const maxMarkers = 300;

        const len = Math.min(chartData.length, buy.length, sell.length);
        for (let i = 0; i < len && markers.length < maxMarkers; i++) {
          const candle = chartData[i];
          if (!candle) continue;

          const buyVal = buy[i];
          const sellVal = sell[i];

          if (typeof buyVal === 'number') {
            markers.push({
              time: candle.time as UTCTimestamp,
              position: 'belowBar',
              color: 'rgb(34,197,94)',
              shape: 'arrowUp',
              text: 'Buy',
            });
          }

          if (typeof sellVal === 'number') {
            markers.push({
              time: candle.time as UTCTimestamp,
              position: 'aboveBar',
              color: 'rgb(239,68,68)',
              shape: 'arrowDown',
              text: 'Sell',
            });
          }
        }

        markersApiRef.current.setMarkers(markers);
        return;
      }

      // Clear markers for all other indicators
      markersApiRef.current.setMarkers([]);
    },
    [chartData, indicatorId]
  );

  const syncLegendFromSeries = useCallback(() => {
    // Single indicator value
    if (indicatorSeriesRef.current) {
      try {
        const last = indicatorSeriesRef.current.lastValueData?.(true) as any;
        const value =
          typeof last?.price === 'number'
            ? last.price
            : typeof (last as any)?.value === 'number'
              ? (last as any).value
              : null;
        setCurrentIndicatorValue(typeof value === 'number' ? value : null);
      } catch (_) {
        // ignore
      }
    }

    // Multi-indicator values
    const latestMulti: Record<string, number> = {};
    indicatorSeriesArrayRef.current.forEach((series) => {
      try {
        const opts = series.options?.();
        const title = (opts as any)?.title;
        const last = series.lastValueData?.(true) as any;
        const value =
          typeof last?.price === 'number'
            ? last.price
            : typeof (last as any)?.value === 'number'
              ? (last as any).value
              : null;
        if (typeof title === 'string' && typeof value === 'number') {
          latestMulti[title] = value;
        }
      } catch (_) {
        // ignore
      }
    });
    setCurrentMultiIndicatorValues(Object.keys(latestMulti).length ? latestMulti : null);
  }, []);

  // Update indicator data when parameters change
  useEffect(() => {
    // For Heikin Ashi Candles, we intentionally avoid drawing a
    // separate overlay indicator; the main price series already
    // renders Heikin Ashi candles when selected as the chart type.
    if (indicatorId === 'heikin_ashi_candles') return;

    if (!chartRef.current || !chartData.length || !mainSeriesRef.current) return;

    // Skip if chart is not ready
    if (!chartRef.current.timeScale) return;

    // Debounce rapid parameter changes
    const debounceTimer = setTimeout(async () => {
      // Clear any previous indicator‑only error when recomputing
      setIndicatorError(null);

      // Store current viewport so we can restore it after recalculation
      let logicalRange: any = null;
      let priceRange: { from: number; to: number } | null = null;
      let autoScaleWasOn = true;

      try {
        if (chartRef.current) {
          logicalRange = chartRef.current.timeScale().getVisibleLogicalRange();
          const ps = chartRef.current.priceScale('right');
          priceRange = ps.getVisibleRange();
          const opts = ps.options?.();
          autoScaleWasOn = opts?.autoScale !== false;
          ps.setAutoScale(false);
        }
      } catch (e) {
        console.debug('Could not get current position:', e);
      }

      // Remove old indicator series
      const chart = chartRef.current;

      if (indicatorSeriesArrayRef.current.length > 0) {
        indicatorSeriesArrayRef.current.forEach(series => {
          try {
            if (series && chart) {
              chart.removeSeries(series);
            }
          } catch (e) {
            console.debug('Series already removed or invalid:', e);
          }
        });
        indicatorSeriesArrayRef.current = [];
      }

      // Recalculate and add new indicator with currentParameters
      let indicatorData: any = null;
      try {
        indicatorData = await talib.calculate(indicatorId, chartData, currentParameters);
      } catch (e) {
        console.error(`Error recalculating indicator "${indicatorId}" with updated parameters:`, e);
        const message =
          e instanceof Error
            ? e.message
            : typeof e === 'string'
              ? e
              : 'Failed to recalculate indicator values.';
        // If WASM doesn't have this indicator wired up, show a clear,
        // non‑fatal message while keeping the base chart visible.
        if (
          message.includes('WASM function not found for indicator') ||
          message.includes('Unknown indicator')
        ) {
          setIndicatorError(
            'This indicator is not yet available in the WebAssembly engine used by the docs, so only the price chart is shown.'
          );
        } else {
          setIndicatorError(message);
        }
        indicatorData = null;
      }

      if (indicatorData) {
        setIndicatorValues(indicatorData);
        setIndicatorOutputMode(Array.isArray(indicatorData) ? 'single' : 'multi');

        if (Array.isArray(indicatorData) && chart) {
          const indicatorSeries = isOscillator
            ? chart.addSeries(LineSeries, {
                color: indicatorColor,
                lineWidth: 2,
                visible: isIndicatorVisible,
              }, 2)
            : chart.addSeries(LineSeries, {
                color: indicatorColor,
                lineWidth: 2,
                visible: isIndicatorVisible,
              }, 0);

          indicatorSeriesRef.current = indicatorSeries;
          indicatorSeriesArrayRef.current.push(indicatorSeries);

          const maxLength = Math.min(indicatorData.length, chartData.length);
          const lineData = indicatorData
            .slice(0, maxLength)
            .map((value, index) => {
              if (!chartData[index]) return null;
              return {
                time: chartData[index].time as UTCTimestamp,
                value: value ?? undefined,
              };
            })
            .filter((d): d is { time: UTCTimestamp; value: number } => !!d && d.value !== undefined) as {
              time: UTCTimestamp;
              value: number;
            }[];

          indicatorSeries.setData(lineData);
          ensureRsiPriceLines();
        } else if (typeof indicatorData === 'object' && chart) {
          const emdColors = ['#ff6b6b', '#4ecdc4', '#45b7d1'];
          const defaultColors = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6'];
          const colors = indicatorId === 'emd' ? emdColors : defaultColors;
          let colorIndex = 0;
          indicatorSeriesArrayRef.current = [];

          Object.entries(indicatorData).forEach(([key, values]) => {
            // Match initial render behavior: for HalfTrend, only plot the
            // main line and ATR bands, not trend flags or sparse signals.
            if (
              indicatorId === 'halftrend' &&
              (key === 'trend' || key === 'buy_signal' || key === 'sell_signal')
            ) {
              return;
            }

            // For SuperTrend, only plot the main trend line and keep the
            // binary `changed` flags for internal use (markers/exports).
            if (indicatorId === 'supertrend' && key === 'changed') {
              return;
            }

            if (!Array.isArray(values)) return;

            let title = key.charAt(0).toUpperCase() + key.slice(1);
            if (indicatorId === 'emd') {
              const emdLabels: Record<string, string> = {
                upper: 'EMD Upper Band',
                middle: 'EMD Middle Band',
                lower: 'EMD Lower Band',
              };
              title = emdLabels[key] || title;
            }

            let paneIndex = 0;
            if (isOscillator) {
              if (indicatorId === 'bandpass') {
                paneIndex = key === 'bp' ? 2 : 3;
              } else if (indicatorId === 'correlation_cycle') {
                paneIndex = key === 'angle' ? 3 : 2;
              } else {
                paneIndex = 2;
              }
            }

            // WTO: draw histogram in its own pane below the main WTO waves
            if (indicatorId === 'wto' && key === 'histogram') {
              const histPaneIndex = isOscillator ? 3 : paneIndex;
              const series = chart.addSeries(HistogramSeries, {
                color: '#9ca3af',
                priceFormat: { type: 'price', precision: 2, minMove: 0.01 },
                title: 'Histogram',
                visible: isIndicatorVisible,
              }, histPaneIndex);

              const maxLength = Math.min(values.length, chartData.length);
              const histData = values
                .slice(0, maxLength)
                .map((raw, index) => {
                  const t = chartData[index]?.time as UTCTimestamp | undefined;
                  if (t === undefined || raw === undefined || raw === null) return null;
                  const v = typeof raw === 'number' ? raw : Number(raw);
                  if (!Number.isFinite(v)) return null;

                  const color =
                    v >= 0 ? 'rgba(34,197,94,0.65)' : 'rgba(239,68,68,0.65)';

                  return { time: t, value: v, color };
                })
                .filter(
                  (d): d is { time: UTCTimestamp; value: number; color: string } =>
                    !!d
                );

              series.setData(histData);
              indicatorSeriesArrayRef.current.push(series);
              return;
            }

            // Special plotting for TTM Squeeze:
            // - momentum as a line
            // - squeeze as a color-coded histogram at the zero line
            if (indicatorId === 'ttm_squeeze' && key === 'squeeze') {
              const series = chart.addSeries(HistogramSeries, {
                color: '#6b7280',
                priceFormat: { type: 'price', precision: 0, minMove: 1 },
                title: 'Squeeze',
                visible: isIndicatorVisible,
              }, paneIndex);

              const maxLength = Math.min(values.length, chartData.length);
              const histData = values
                .slice(0, maxLength)
                .map((raw, index) => {
                  const t = chartData[index]?.time as UTCTimestamp | undefined;
                  if (t === undefined || raw === undefined || raw === null) return null;
                  const v = typeof raw === 'number' ? raw : Number(raw);
                  if (!Number.isFinite(v)) return null;

                  let color = 'rgba(148,163,184,0.0)'; // transparent when no squeeze
                  if (v >= 3) {
                    color = 'rgba(239,68,68,0.8)'; // High squeeze – red
                  } else if (v >= 2) {
                    color = 'rgba(234,179,8,0.85)'; // Mid squeeze – amber
                  } else if (v >= 1) {
                    color = 'rgba(34,197,94,0.85)'; // Low squeeze – green
                  }

                  const value = v > 0 ? 1 : 0;
                  return { time: t, value, color };
                })
                .filter((d): d is { time: UTCTimestamp; value: number; color: string } => !!d);

              series.setData(histData);
              indicatorSeriesArrayRef.current.push(series);
              return;
            }

            const series = chart.addSeries(LineSeries, {
              color: colors[colorIndex % colors.length],
              lineWidth: key === 'middle' || key === 'signal' ? 2 : 1,
              title,
              visible: isIndicatorVisible,
            }, paneIndex);

            const maxLength = Math.min(values.length, chartData.length);
            const lineData = values
              .slice(0, maxLength)
              .map((value, index) => {
                const t = chartData[index]?.time as UTCTimestamp | undefined;
                if (t === undefined || value === undefined || value === null) return null;
                return { time: t, value: value as number };
              })
              .filter((d): d is { time: UTCTimestamp; value: number } => !!d);

            series.setData(lineData);

            if (colorIndex === 0 && indicatorId !== 'emd') {
              indicatorSeriesRef.current = series;
            }

            indicatorSeriesArrayRef.current.push(series);
            colorIndex++;
          });
        }
      }

      updateSeriesMarkers(indicatorData);
      syncLegendFromSeries();
      applySeriesOrder();

      // Restore viewport and pane proportions so the oscillator
      // pane does not visually collapse when parameters change.
      setTimeout(() => {
        const chartInstance = chartRef.current;
        if (!chartInstance) return;

        try {
          const ps = chartInstance.priceScale('right');
          if (priceRange) {
            ps.setVisibleRange(priceRange);
          }
          ps.setAutoScale(autoScaleWasOn);
        } catch (e) {
          console.debug('Could not restore price scale:', e);
        }

        if (logicalRange) {
          try {
            chartInstance.timeScale().setVisibleLogicalRange(logicalRange);
          } catch (e) {
            console.debug('Could not restore logical range:', e);
          }
        }

        // Re-apply pane stretch factors after indicator redraw so
        // the dedicated oscillator pane keeps a stable height.
        try {
          const panes = chartInstance.panes?.();
          if (!panes || !panes.length) return;

          if (isOscillator) {
            if (indicatorId === 'bandpass' && panes.length >= 4) {
              // Main 55, Volume 15, Band-pass 18, Normalized/trigger 12
              panes[0].setStretchFactor(55);
              panes[1].setStretchFactor(15);
              panes[2].setStretchFactor(18);
              panes[3].setStretchFactor(12);
            } else if (indicatorId === 'correlation_cycle' && panes.length >= 4) {
              // Main 55, Volume 15, Correlation Cycle osc 15, Angle osc 15
              panes[0].setStretchFactor(55);
              panes[1].setStretchFactor(15);
              panes[2].setStretchFactor(15);
              panes[3].setStretchFactor(15);
            } else if (indicatorId === 'wto' && panes.length >= 4) {
              // Main 55, Volume 15, WTO waves 20, Histogram 10
              panes[0].setStretchFactor(55);
              panes[1].setStretchFactor(15);
              panes[2].setStretchFactor(20);
              panes[3].setStretchFactor(10);
            } else if (panes.length >= 3) {
              // Main 60, Volume 15, Oscillator 25 (relative factors)
              panes[0].setStretchFactor(60);
              panes[1].setStretchFactor(15);
              panes[2].setStretchFactor(25);
            }
          } else if (chartData[0]?.volume !== undefined && panes.length >= 2) {
            // Main 75, Volume 25 for non‑oscillator indicators
            panes[0].setStretchFactor(75);
            panes[1].setStretchFactor(25);
          }
        } catch (e) {
          console.warn('Could not update pane heights after parameter change:', e);
        }
      }, 10);
    }, 100);

    return () => clearTimeout(debounceTimer);
  }, [currentParameters, indicatorId, chartData, isOscillator, syncLegendFromSeries, applySeriesOrder, ensureRsiPriceLines, updateSeriesMarkers]);

  // Handle indicator color and visibility updates separately
  useEffect(() => {
    if (!indicatorSeriesArrayRef.current.length) return;
    const multiple = indicatorSeriesArrayRef.current.length > 1;
    indicatorSeriesArrayRef.current.forEach(series => {
      try {
        series.applyOptions(multiple ? { visible: isIndicatorVisible } : { visible: isIndicatorVisible, color: indicatorColor });
      } catch (e) {
        console.debug('Could not update series options:', e);
      }
    });
  }, [indicatorColor, isIndicatorVisible]);

  // Create and update chart
  useEffect(() => {
    console.log('Chart useEffect triggered - chartType:', chartType, 'data length:', chartData.length);
    if (!chartContainerRef.current || chartData.length === 0) return;

    const initChart = async () => {
      let chart: IChartApi | null = null;
      
      // Store current visible range and logical range before creating/updating chart
      let previousVisibleRange: any = null;
      let previousLogicalRange: any = null;
      let previousScrollPosition: number | null = null;
      let previousPriceRange: { from: number; to: number } | null = null;
      let previousAutoScale = true;
      
      if (chartRef.current) {
        try {
          previousVisibleRange = chartRef.current.timeScale().getVisibleRange();
          previousLogicalRange = chartRef.current.timeScale().getVisibleLogicalRange();
          previousScrollPosition = chartRef.current.timeScale().scrollPosition();
          const ps = chartRef.current.priceScale('right');
          previousPriceRange = ps.getVisibleRange();
          const opts = ps.options?.();
          previousAutoScale = opts?.autoScale !== false;
        } catch (e) {
          // Chart might not be initialized yet
          console.debug('Could not get previous range:', e);
        }
      }
      
      try {
      // Create chart with initial theme (will be updated by separate useEffect)
      const el = chartContainerRef.current as HTMLDivElement; // non-null by guard above
      chart = createChart(el, {
      autoSize: true,
      layout: {
        background: { type: ColorType.Solid, color: '#ffffff' },
        textColor: '#374151',
        fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        panes: {
          separatorColor: '#e5e7eb',
          separatorHoverColor: '#d1d5db',
          enableResize: true,
        },
      },
      grid: {
        vertLines: { color: '#f3f4f6' },
        horzLines: { color: '#f3f4f6' },
      },
      crosshair: { 
        mode: isCrosshairVisible ? resolveCrosshairMode(crosshairMode) : CrosshairMode.Hidden,
        vertLine: {
          width: 1,
          color: '#9ca3af',
          style: 2,
        },
        horzLine: {
          width: 1,
          color: '#9ca3af',
          style: 2,
        },
      },
      localization: {
        tickmarksPriceFormatter: (values: number[]) => {
          if (!Array.isArray(values) || values.length === 0) return [];
          const maxAbs = Math.max(...values.map((v) => Math.abs(v)));
          const decimals = maxAbs < 1 ? 5 : maxAbs < 100 ? 3 : 2;
          return values.map((v) => v.toFixed(decimals));
        },
      },
      rightPriceScale: { 
        borderColor: '#e5e7eb',
        scaleMargins: {
          top: isOscillator ? 0.3 : 0.1,
          bottom: isOscillator ? 0.1 : 0.2,
        },
        // Optimize for mobile
        minimumWidth: isMobile ? 40 : undefined,
      },
      timeScale: {
        borderColor: '#e5e7eb',
        timeVisible: !isMobile, // Hide time on mobile to save space
        secondsVisible: false,
        barSpacing: isMobile ? 3 : 6, // Tighter bars on mobile
        rightOffsetPixels: isMobile ? 24 : 48,
      },
      handleScroll: { 
        mouseWheel: true,
        pressedMouseMove: true,
        horzTouchDrag: true,
        vertTouchDrag: !isMobile, // Allow vertical scroll on mobile for page navigation
      },
      handleScale: { 
        mouseWheel: true, 
        pinch: true,
        axisPressedMouseMove: true,
      },
    });

    chartRef.current = chart;
    // Apply crosshair visibility/mode from state
    chart.applyOptions({
      crosshair: {
        mode: isCrosshairVisible ? resolveCrosshairMode(crosshairMode) : CrosshairMode.Hidden,
      },
    });

    // Add main series based on chart type
    console.log('Creating chart with type:', chartType);
    let mainSeries: ISeriesApi<SeriesType>;
    
    switch (chartType) {
      case 'line':
        mainSeries = chart.addSeries(LineSeries, {
          color: '#3b82f6',
          lineWidth: 2,
        });
        const lineData = chartData.map(d => ({
          time: d.time as UTCTimestamp,
          value: d.close,
        }));
        mainSeries.setData(lineData);
        break;
        
      case 'area':
        mainSeries = chart.addSeries(AreaSeries, {
          lineColor: '#3b82f6',
          topColor: 'rgba(59, 130, 246, 0.4)',
          bottomColor: 'rgba(59, 130, 246, 0.1)',
          lineWidth: 2,
        });
        const areaData = chartData.map(d => ({
          time: d.time as UTCTimestamp,
          value: d.close,
        }));
        mainSeries.setData(areaData);
        break;
        
      case 'bar':
        mainSeries = chart.addSeries(BarSeries, {
          upColor: '#10b981',
          downColor: '#ef4444',
        });
        const barData = chartData.map(d => ({
          time: d.time as UTCTimestamp,
          open: d.open,
          high: d.high,
          low: d.low,
          close: d.close,
        }));
        mainSeries.setData(barData);
        break;
        
      case 'heikinashi':
        // Calculate Heikin Ashi candles
        const haData = calculateHeikinAshi(chartData);
        console.log('Heikin Ashi data:', haData.length, 'candles from', chartData.length, 'original');
        mainSeries = chart.addSeries(CandlestickSeries, {
          upColor: '#10b981',
          downColor: '#ef4444',
          borderUpColor: '#10b981',
          borderDownColor: '#ef4444',
          wickUpColor: '#10b981',
          wickDownColor: '#ef4444',
        });
        mainSeries.setData(haData);
        break;
        
      case 'renko':
        // Calculate Renko bricks with 0.25% brick size
        const renkoData = calculateRenko(chartData, 0.25); // 0.25% of average price
        console.log('Renko data:', renkoData.length, 'bricks from', chartData.length, 'original');
        
        mainSeries = chart.addSeries(CandlestickSeries, {
          upColor: '#10b981',
          downColor: '#ef4444',
          borderUpColor: '#10b981',
          borderDownColor: '#ef4444',
          wickUpColor: '#10b981',
          wickDownColor: '#ef4444',
          borderVisible: false,
          wickVisible: false,
        });
        
        // Set Renko data
        mainSeries.setData(renkoData);
        
        // Add a note about Renko brick size
        console.log(`Renko chart created with ${renkoData.length} bricks from ${chartData.length} candles`);
        break;
        
      case 'candlestick':
      default:
        mainSeries = chart.addSeries(CandlestickSeries, {
          upColor: '#10b981',
          downColor: '#ef4444',
          borderUpColor: '#10b981',
          borderDownColor: '#ef4444',
          wickUpColor: '#10b981',
          wickDownColor: '#ef4444',
        });
        const candleData = chartData.map(d => ({
          time: d.time as UTCTimestamp,
          open: d.open,
          high: d.high,
          low: d.low,
          close: d.close,
        }));
        mainSeries.setData(candleData);
        break;
    }

    mainSeriesRef.current = mainSeries;
    try {
      // Attach the series markers plugin with a high z-order so
      // arrows render above candles and other series.
      markersApiRef.current = createSeriesMarkers<Time>(mainSeries, [], {
        autoScale: true,
        zOrder: 'top',
      });
    } catch (e) {
      console.warn('Could not initialise series markers plugin:', e);
    }
    
    // Drawing tools initialization commented out for future use
    // if (chart) {
    //   drawingToolsRef.current = new DrawingToolsPlugin(chart);
    //   // Set the series reference so drawing tools can use priceToCoordinate
    //   drawingToolsRef.current.setSeries(mainSeries);
    // }

    // Add volume series in pane 1 if we have volume data
    if (chartData[0]?.volume !== undefined) {
      const volumeSeries = chart.addSeries(HistogramSeries, {
        priceFormat: {
          type: 'volume',
        },
        // Don't specify priceScaleId when using panes - each pane has its own scale
      }, 1); // Add to pane 1

      const volumeData = chartData.map(d => ({
        time: d.time as UTCTimestamp,
        value: d.volume,
        color: d.close >= d.open ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)', // Green for up, red for down
      }));
      
      volumeSeries.setData(volumeData);
      volumeSeriesRef.current = volumeSeries; // Store reference to volume series
      
      // Apply scale margins through the series' price scale
      try {
        volumeSeries.priceScale().applyOptions({
          scaleMargins: {
            top: 0.1,
            bottom: 0,
          },
        });
      } catch (e) {
        console.warn('Could not apply volume scale margins:', e);
      }
    }

    // Calculate and add indicator (except for Heikin Ashi Candles,
    // where we treat the main chart as the indicator visualization).
    let indicatorData: any = null;

	    if (indicatorId !== 'heikin_ashi_candles') {
	      // Reset any previous indicator‑only error before attempting a fresh calculation
	      setIndicatorError(null);
	      try {
	        indicatorData = await talib.calculate(indicatorId, chartData, currentParameters);
      } catch (e) {
        console.error(`Error calculating indicator "${indicatorId}" during initial chart setup:`, e);
        const message =
          e instanceof Error
            ? e.message
            : typeof e === 'string'
              ? e
              : 'Failed to calculate indicator values.';
        if (
          message.includes('WASM function not found for indicator') ||
          message.includes('Unknown indicator')
        ) {
          setIndicatorError(
            'This indicator is not yet available in the WebAssembly engine used by the docs, so only the price chart is shown.'
          );
        } else {
          setIndicatorError(message);
        }
        // Keep the base price/volume chart visible even if indicator calculation fails.
        indicatorData = null;
      }
	    }

	    if (indicatorData) {
	      setIndicatorOutputMode(Array.isArray(indicatorData) ? 'single' : 'multi');
	    }

    if (indicatorId === 'heikin_ashi_candles') {
      // For Heikin Ashi Candles, we avoid drawing a separate
      // overlay indicator and instead use the Heikin Ashi
      // candles as the primary price chart. We still expose
      // the transformed OHLC values for CSV export.
      const haData = calculateHeikinAshi(chartData);
      const exportValues = {
        open: haData.map(c => c.open),
        high: haData.map(c => c.high),
        low: haData.map(c => c.low),
        close: haData.map(c => c.close),
      };
      setIndicatorValues(exportValues as any);
    } else if (indicatorData) {
      // Store indicator values for CSV export
      setIndicatorValues(indicatorData);
      
      // Handle single-line indicators
      if (Array.isArray(indicatorData)) {
        const indicatorSeries = isOscillator
          ? chart.addSeries(LineSeries, {
              color: indicatorColor,
              lineWidth: 2,
              // Don't specify priceScaleId when using panes
            }, 2) // Add oscillators to pane 2
          : chart.addSeries(LineSeries, {
              color: indicatorColor,
              lineWidth: 2,
              // Use default right scale for overlays in main pane
            }, 0); // Add overlays to main pane

        indicatorSeriesRef.current = indicatorSeries;
        indicatorSeriesArrayRef.current = [indicatorSeries]; // Reset and track this series

        // Ensure indicator data doesn't exceed chart data length
        const maxLength = Math.min(indicatorData.length, chartData.length);
        const lineData = indicatorData
          .slice(0, maxLength)
          .map((value, index) => {
            if (!chartData[index]) return null;
            return {
              time: chartData[index].time as UTCTimestamp,
              value: value ?? undefined,
            };
          })
          .filter((d): d is { time: UTCTimestamp; value: number } => !!d && d.value !== undefined) as {
            time: UTCTimestamp;
            value: number;
          }[];
        
        indicatorSeries.setData(lineData);
        ensureRsiPriceLines();
      } 
      // Handle multi-line indicators (like Bollinger Bands, MACD, EMD)
      else if (typeof indicatorData === 'object') {
        // Special colors for EMD to distinguish bands
        const emdColors = ['#ff6b6b', '#4ecdc4', '#45b7d1'];
        const defaultColors = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6'];
        const colors = indicatorId === 'emd' ? emdColors : defaultColors;
        
        let colorIndex = 0;
        indicatorSeriesArrayRef.current = []; // Reset array for multi-line indicators
        
        Object.entries(indicatorData).forEach(([key, values]) => {
          // For HalfTrend, we only plot the main line and ATR bands;
          // trend flags and sparse buy/sell markers remain available
          // for exports but are not rendered as separate lines.
          if (
            indicatorId === 'halftrend' &&
            (key === 'trend' || key === 'buy_signal' || key === 'sell_signal')
          ) {
            return;
          }

          // For SuperTrend, hide the binary change flags line; we only
          // render the main trend overlay while keeping `changed` for
          // potential markers/exports.
          if (indicatorId === 'supertrend' && key === 'changed') {
            return;
          }

          if (Array.isArray(values) && chart) {
            // Determine the title based on indicator type
            let title = key.charAt(0).toUpperCase() + key.slice(1);
            if (indicatorId === 'emd') {
              // Proper EMD band labels
              const emdLabels: Record<string, string> = {
                'upper': 'EMD Upper Band',
                'middle': 'EMD Middle Band',
                'lower': 'EMD Lower Band'
              };
              title = emdLabels[key] || title;
            } else if (indicatorId === 'vi') {
              // Proper VI labels
              const viLabels: Record<string, string> = {
                'plus_vi': 'VI+',
                'minus_vi': 'VI-'
              };
              title = viLabels[key] || title;
              console.log(`VI creating series for ${key} with title ${title}, values length: ${values.length}`);
            } else if (indicatorId === 'damiani_volatmeter') {
              // Friendlier labels for Damiani outputs
              const damianiLabels: Record<string, string> = {
                vol: 'Volatility (vol)',
                anti: 'Anti threshold (anti)',
              };
              title = damianiLabels[key] || title;
            } else if (indicatorId === 'bandpass') {
              // Friendlier labels for band-pass components
              const bandpassLabels: Record<string, string> = {
                bp: 'Band-pass',
                bp_normalized: 'Normalized cycle',
                signal: 'Cycle signal',
                trigger: 'Trigger line',
              };
              title = bandpassLabels[key] || title;
            }

            // Decide which pane to use
            let paneIndex = 0;
            if (isOscillator) {
              if (indicatorId === 'bandpass') {
                // Raw band-pass in primary oscillator pane, others in a pane below
                paneIndex = key === 'bp' ? 2 : 3;
              } else if (indicatorId === 'correlation_cycle') {
                // Correlation Cycle: real/imag/state in first osc pane, angle in a pane below
                paneIndex = key === 'angle' ? 3 : 2;
              } else {
                paneIndex = 2;
              }
            } else {
              paneIndex = 0;
            }

            // WTO: draw histogram in its own pane below the main WTO waves
            if (indicatorId === 'wto' && key === 'histogram') {
              const histPaneIndex = isOscillator ? 3 : paneIndex;
              const series = chart.addSeries(HistogramSeries, {
                color: '#9ca3af',
                priceFormat: { type: 'price', precision: 2, minMove: 0.01 },
                title: 'Histogram',
              }, histPaneIndex as number);

              const maxLength = Math.min(values.length, chartData.length);
              const histData = values
                .slice(0, maxLength)
                .map((raw, index) => {
                  const t = chartData[index]?.time as UTCTimestamp | undefined;
                  if (t === undefined || raw === undefined || raw === null) return null;
                  const v = typeof raw === 'number' ? raw : Number(raw);
                  if (!Number.isFinite(v)) return null;

                  const color =
                    v >= 0 ? 'rgba(34,197,94,0.65)' : 'rgba(239,68,68,0.65)';

                  return { time: t, value: v, color };
                })
                .filter(
                  (d): d is { time: UTCTimestamp; value: number; color: string } =>
                    !!d
                );

              series.setData(histData);
              indicatorSeriesArrayRef.current.push(series);
              return;
            }

            // Special plotting for TTM Squeeze: momentum line + squeeze histogram
            if (indicatorId === 'ttm_squeeze' && key === 'squeeze') {
              const series = chart.addSeries(HistogramSeries, {
                color: '#6b7280',
                priceFormat: { type: 'price', precision: 0, minMove: 1 },
                title: 'Squeeze',
              }, paneIndex as number);

              const maxLength = Math.min(values.length, chartData.length);
              const histData = values
                .slice(0, maxLength)
                .map((raw, index) => {
                  const t = chartData[index]?.time as UTCTimestamp | undefined;
                  if (t === undefined || raw === undefined || raw === null) return null;
                  const v = typeof raw === 'number' ? raw : Number(raw);
                  if (!Number.isFinite(v)) return null;

                  let color = 'rgba(148,163,184,0.0)';
                  if (v >= 3) {
                    color = 'rgba(239,68,68,0.8)'; // High
                  } else if (v >= 2) {
                    color = 'rgba(234,179,8,0.85)'; // Mid
                  } else if (v >= 1) {
                    color = 'rgba(34,197,94,0.85)'; // Low
                  }
                  const value = v > 0 ? 1 : 0;
                  return { time: t, value, color };
                })
                .filter((d): d is { time: UTCTimestamp; value: number; color: string } => !!d);

              series.setData(histData);
              indicatorSeriesArrayRef.current.push(series);
              return;
            }

            const series = chart.addSeries(LineSeries, {
              color: colors[colorIndex % colors.length],
              lineWidth: key === 'middle' || key === 'signal' || key === 'residue' ? 2 : 1,
              title,
              // Price scale handled per pane
           }, paneIndex as number);

            // Ensure values array doesn't exceed chart data length
            const maxLength = Math.min(values.length, chartData.length);
            const lineData = values
              .slice(0, maxLength)
              .map((value, index) => {
                const t = chartData[index]?.time as UTCTimestamp | undefined;
                if (t === undefined || value === undefined || value === null) return null;
                return { time: t, value: value as number };
              })
              .filter((d): d is { time: UTCTimestamp; value: number } => !!d);
            
            series.setData(lineData);
            
            // Store the first series as the main indicator series
            // For EMD, don't set a main series so all bands show with their labels
            if (colorIndex === 0 && indicatorId !== 'emd') {
              indicatorSeriesRef.current = series;
            }
            
            // Track all indicator series
            indicatorSeriesArrayRef.current.push(series);
            
            colorIndex++;
          }
        });
      }
    }

    if (indicatorId !== 'heikin_ashi_candles') {
      updateSeriesMarkers(indicatorData);
      applySeriesOrder();
    }

    // Setup enhanced features
    setupEnhancedFeatures(chart);

    // Add context menu handler
    chartContainerRef.current?.addEventListener('contextmenu', handleContextMenu);

    // Restore previous range if available, otherwise fit content
    const restoreViewState = () => {
      if (!chart) return;
      try {
        const ps = chart.priceScale('right');
        if (previousPriceRange) ps.setVisibleRange(previousPriceRange);
        ps.setAutoScale(previousAutoScale);
      } catch (e) {
        console.debug('Could not restore price scale during init:', e);
      }

      if (previousVisibleRange && previousLogicalRange) {
        try {
          chart.timeScale().setVisibleRange(previousVisibleRange);
          
          if (previousScrollPosition !== null) {
            chart.timeScale().scrollToPosition(previousScrollPosition, false);
          }
        } catch (e) {
          try {
            chart.timeScale().setVisibleLogicalRange(previousLogicalRange);
          } catch (e2) {
            chart.timeScale().fitContent();
          }
        }
      } else {
        chart.timeScale().fitContent();
      }
    };

    // Small delay to ensure data is loaded
    setTimeout(restoreViewState, 10);
    
    // Set pane proportions if we have multiple panes
    // Wait a bit to ensure panes are created
    setTimeout(() => {
      if (isOscillator && chartRef.current) {
        try {
          const panes = chartRef.current.panes();
          if (!panes) return;

          if (indicatorId === 'bandpass' && panes.length >= 4) {
            // Main 55, Volume 15, Band-pass 18, Normalized/trigger 12
            panes[0].setStretchFactor(55);
            panes[1].setStretchFactor(15);
            panes[2].setStretchFactor(18);
            panes[3].setStretchFactor(12);
          } else if (indicatorId === 'correlation_cycle' && panes.length >= 4) {
            // Main 55, Volume 15, Correlation Cycle osc 15, Angle osc 15
            panes[0].setStretchFactor(55);
            panes[1].setStretchFactor(15);
            panes[2].setStretchFactor(15);
            panes[3].setStretchFactor(15);
          } else if (indicatorId === 'wto' && panes.length >= 4) {
            // Main 55, Volume 15, WTO waves 20, Histogram 10
            panes[0].setStretchFactor(55);
            panes[1].setStretchFactor(15);
            panes[2].setStretchFactor(20);
            panes[3].setStretchFactor(10);
          } else if (panes.length >= 3) {
            // Main 60, Volume 15, Oscillator 25 (relative factors)
            panes[0].setStretchFactor(60);
            panes[1].setStretchFactor(15);
            panes[2].setStretchFactor(25);
          }
        } catch (e) {
          console.warn('Could not set pane heights:', e);
        }
      } else if (chartData[0]?.volume !== undefined && chartRef.current) {
        // Main 75, Volume 25
        try {
          const panes = chartRef.current.panes();
          if (panes && panes.length >= 2) {
            panes[0].setStretchFactor(75);
            panes[1].setStretchFactor(25);
          }
        } catch (e) {
          console.warn('Could not set pane heights:', e);
        }
      }
    }, 50); // Small delay to ensure panes are created

    } catch (error) {
      console.error('Error creating chart:', error);
      setError('Failed to create chart');
    }

    // Cleanup
    return () => {
      if (chartContainerRef.current) {
        chartContainerRef.current.removeEventListener('contextmenu', handleContextMenu);
      }
      // if (drawingToolsRef.current) {
      //   drawingToolsRef.current.destroy();
      //   drawingToolsRef.current = null;
      // }
      if (chart && chartRef.current === chart) {
        try {
          console.log('Removing chart for cleanup');
          chart.remove();
          chartRef.current = null; // Important: nullify the ref
          markersApiRef.current = null;
        } catch (e) {
          console.error('Error removing chart:', e);
        }
      }
    };
    }; // End of async initChart function
    
    // Call the async function
    initChart();
    
    // Cleanup function to remove old chart before creating new one
    return () => {
      console.log('Cleanup: removing existing chart');
      if (chartRef.current) {
        try {
          chartRef.current.remove();
          chartRef.current = null;
          markersApiRef.current = null;
        } catch (e) {
          console.error('Error in cleanup:', e);
        }
      }
    };
  }, [chartData, indicatorId, isOscillator, chartType, handleContextMenu, isMobile, chartResetNonce, applySeriesOrder, ensureRsiPriceLines, resolveCrosshairMode, updateSeriesMarkers]); // Removed indicatorColor and isDark to prevent chart recreation

  // Handle theme updates without recreating the chart
  useEffect(() => {
    if (!chartRef.current) return;

    try {
      // Update chart theme using applyOptions
      chartRef.current.applyOptions({
        layout: {
          background: { type: ColorType.Solid, color: isDark ? '#111827' : '#ffffff' },
          textColor: isDark ? '#e5e7eb' : '#374151',
          fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
          panes: {
            separatorColor: isDark ? '#374151' : '#e5e7eb',
            separatorHoverColor: isDark ? '#4b5563' : '#d1d5db',
            enableResize: true,
          },
        },
        grid: {
          vertLines: { color: isDark ? '#1f2937' : '#f3f4f6' },
          horzLines: { color: isDark ? '#1f2937' : '#f3f4f6' },
        },
        crosshair: {
          vertLine: {
            width: 1,
            color: isDark ? '#4b5563' : '#9ca3af',
            style: 2,
          },
          horzLine: {
            width: 1,
            color: isDark ? '#4b5563' : '#9ca3af',
            style: 2,
          },
        },
      });

      // Update price scale theme
      chartRef.current.priceScale('right').applyOptions({
        borderColor: isDark ? '#374151' : '#e5e7eb',
      });

      // Update time scale theme
      chartRef.current.timeScale().applyOptions({
        borderColor: isDark ? '#374151' : '#e5e7eb',
      });

      // Update all series colors if they exist
      if (mainSeriesRef.current) {
        const seriesType = mainSeriesRef.current.seriesType();
        
        if (seriesType === 'Candlestick' || seriesType === 'Bar') {
          mainSeriesRef.current.applyOptions({
            upColor: '#10b981',
            downColor: '#ef4444',
            borderUpColor: '#10b981',
            borderDownColor: '#ef4444',
            wickUpColor: '#10b981',
            wickDownColor: '#ef4444',
          });
        }
      }

      // Update volume series colors if it exists
      if (volumeSeriesRef.current && chartData.length > 0) {
        const volumeData = chartData.map(d => ({
          time: d.time as UTCTimestamp,
          value: d.volume,
          color: d.close >= d.open ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)',
        }));
        volumeSeriesRef.current.setData(volumeData);
      }
    } catch (error) {
      console.error('Error updating chart theme:', error);
    }
  }, [isDark, chartData]);

  // Handle touch gestures (double tap, long press, etc.)
  const { isPinching } = useChartTouchGestures(chartContainerRef, chartRef, {
    onDoubleTap: () => {
      if (chartRef.current) {
        // Reset zoom on double tap
        chartRef.current.timeScale().resetTimeScale();
        chartRef.current.priceScale('right').applyOptions({ autoScale: true });
        chartRef.current.timeScale().fitContent();
      }
    },
    onLongPress: (event) => {
      // Show context menu on long press
      if (!chartContainerRef.current) return;
      
      const rect = chartContainerRef.current.getBoundingClientRect();
      const x = event.x;
      const y = event.y;
      
      // Try to get price and time at press position
      if (chartRef.current) {
        const timeScale = chartRef.current.timeScale();
        const coordinate = timeScale.coordinateToLogical(x - rect.left);
        
        let clickData: any = { x: x - rect.left, y: y - rect.top };
        
        if (coordinate !== null && mainSeriesRef.current) {
          try {
            const data = mainSeriesRef.current.dataByIndex(Math.round(coordinate));
            if (data) {
              clickData.time = data.time;
              clickData.price = 'close' in data ? data.close : 'value' in data ? data.value : null;
            }
          } catch (e) {
            console.debug('Could not get price data at press position');
          }
        }
        
        setRightClickData(clickData);
        setContextMenu({
          x: event.x,
          y: event.y,
          isVisible: true,
        });
      }
    },
    longPressDelay: 500,
    doubleTapDelay: 300,
  });

  // Setup enhanced features
  const setupEnhancedFeatures = (chart: IChartApi) => {
    try {
      // Subscribe to crosshair move for tooltip
      const crosshairHandler = (param: any) => {
        if (!param.point || !param.time) {
          syncLegendFromSeries();
          setTooltipData(null);
          setTooltipPosition(null);
          return;
        }

        const tooltipData: any = {
          time: param.time,
        };

        // Get candlestick data
        if (param.seriesData && mainSeriesRef.current) {
          const candleData = param.seriesData.get(mainSeriesRef.current);
          if (candleData) {
            tooltipData.candle = candleData;
          }
        }

        // Get indicator data
        if (param.seriesData && indicatorSeriesRef.current) {
          const indicatorData = param.seriesData.get(indicatorSeriesRef.current);
          if (indicatorData) {
            tooltipData.indicator = indicatorData;
            // Update current indicator value for legend
            setCurrentIndicatorValue(indicatorData.value);
          }
        }

        // Get all series data for multi-line indicators
        if (param.seriesData) {
          const allData: Record<string, any> = {};
          const multiValues: Record<string, number> = {};
          param.seriesData.forEach((data: any, series: any) => {
            if (series !== mainSeriesRef.current && series !== indicatorSeriesRef.current) {
              const seriesOptions = series.options();
              if (seriesOptions.title) {
                allData[seriesOptions.title] = data;
                // Track values for the legend
                if (data && typeof data.value === 'number') {
                  multiValues[seriesOptions.title] = data.value;
                }
              }
            }
          });
          if (Object.keys(allData).length > 0) {
            tooltipData.multiIndicator = allData;
            // Update multi-indicator values for legend
            setCurrentMultiIndicatorValues(Object.keys(multiValues).length > 0 ? multiValues : null);
          }
        }

        setTooltipData(tooltipData);
        setTooltipPosition({ x: param.point.x, y: param.point.y });
      };

      chart.subscribeCrosshairMove(crosshairHandler);
      syncLegendFromSeries();
    } catch (error) {
      console.error('Error setting up enhanced features:', error);
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-96 bg-white dark:bg-gray-900">
        <div className="text-gray-600 dark:text-gray-400">Loading chart data...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-96 bg-white dark:bg-gray-900">
        <div className="text-red-600 dark:text-red-400">Error: {error}</div>
      </div>
    );
  }

  return (
    <div 
      className={`relative`} 
      ref={containerRef}
      itemScope 
      itemType="https://schema.org/FinancialProduct"
      itemProp="hasPart"
    >
      {/* Chart Toolbar */}
      <div className="absolute top-0 left-0 right-0 z-10 bg-white/90 dark:bg-gray-900/90 backdrop-blur-sm border-b border-gray-200 dark:border-gray-700">
        {/* Mobile Toolbar */}
        {isMobile ? (
          <div className="flex items-center justify-between px-2 py-1">
            {/* Compact timeframe selector */}
            <select
              value={selectedTimeframe}
              onChange={(e) => handleTimeframeChange(e.target.value)}
              className="px-1.5 py-1 text-xs bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-1 focus:ring-blue-500 dark:focus:ring-blue-400 text-gray-700 dark:text-gray-300 min-w-[55px]"
            >
              {TIMEFRAME_OPTIONS.map((tf) => (
                <option key={tf.value} value={tf.value}>
                  {tf.label}
                </option>
              ))}
            </select>
            
            {/* Essential controls only */}
            <div className="flex items-center gap-1">
              <button
                onClick={takeScreenshot}
                className="p-1.5 text-gray-700 dark:text-gray-300"
                title="Screenshot"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 13a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </button>
              <button
                onClick={() => setIsLegendVisible(v => !v)}
                className={`p-1.5 rounded ${isLegendVisible ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300' : 'text-gray-700 dark:text-gray-300'}`}
                title={isLegendVisible ? 'Hide legend' : 'Show legend'}
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h10M4 18h7" />
                </svg>
              </button>
              <button
                onClick={toggleFullscreen}
                className="p-1.5 text-gray-700 dark:text-gray-300"
                title={isFullscreen ? 'Exit' : 'Fullscreen'}
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  {isFullscreen ? (
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 9V4.5M9 9H4.5M9 9L3.75 3.75M9 15v4.5M9 15H4.5M9 15l-5.25 5.25M15 9h4.5M15 9V4.5M15 9l5.25-5.25M15 15h4.5M15 15v4.5m0-4.5l5.25 5.25" />
                  ) : (
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
                  )}
                </svg>
              </button>
            </div>
          </div>
        ) : (
          /* Desktop Toolbar */
          <div className="flex items-center justify-between px-4 py-2">
          {/* Left side - Timeframe selector */}
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Timeframe:</span>
              <select
                value={selectedTimeframe}
                onChange={(e) => handleTimeframeChange(e.target.value)}
                className="px-2 py-1.5 text-sm bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors cursor-pointer min-w-[65px]"
              >
                {TIMEFRAME_OPTIONS.map((tf) => (
                  <option key={tf.value} value={tf.value}>
                    {tf.label}
                  </option>
                ))}
              </select>
            </div>
            
            {/* Chart Type Switcher */}
            <ChartTypeSwitcher
              currentType={chartType}
              onChange={(type) => {
                console.log('ChartTypeSwitcher onChange called with:', type);
                setChartType(type);
              }}
            />
            
            {/* Drawing Tools - commented out for future use */}
            {/* <div className="ml-4">
              <DrawingToolbar
                drawingTools={drawingToolsRef.current}
              />
            </div> */}
          </div>


          {/* Center - Zoom controls */}
          <div className="flex items-center gap-1">
            <button
              onClick={() => {
                if (chartRef.current) {
                  const timeScale = chartRef.current.timeScale();
                  const visibleRange = timeScale.getVisibleRange();
                  if (visibleRange) {
                    const rangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
                    timeScale.setVisibleRange({
                      from: (Number(visibleRange.from) + rangeWidth * 0.1) as Time,
                      to: (Number(visibleRange.to) - rangeWidth * 0.1) as Time,
                    });
                  }
                }
              }}
              className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 rounded"
              title="Zoom In"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v6m3-3H7" />
              </svg>
            </button>
            <button
              onClick={() => {
                if (chartRef.current) {
                  const timeScale = chartRef.current.timeScale();
                  const visibleRange = timeScale.getVisibleRange();
                  if (visibleRange) {
                    const rangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
                    timeScale.setVisibleRange({
                      from: (Number(visibleRange.from) - rangeWidth * 0.1) as Time,
                      to: (Number(visibleRange.to) + rangeWidth * 0.1) as Time,
                    });
                  }
                }
              }}
              className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 rounded"
              title="Zoom Out"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" />
              </svg>
            </button>
            <button
              onClick={() => {
                if (chartRef.current) {
                  chartRef.current.timeScale().fitContent();
                }
              }}
              className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 rounded"
              title="Fit Content"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
              </svg>
            </button>
          </div>

          {/* Right side - Chart controls */}
          <div className="flex items-center gap-2">
            {/* Crosshair mode selector */}
            <select
              value={crosshairMode}
              onChange={(e) => {
                const next = e.target.value as 'normal' | 'magnet' | 'magnetOHLC';
                setCrosshairMode(next);
                if (chartRef.current) {
                  chartRef.current.applyOptions({
                    crosshair: {
                      mode: !isCrosshairVisible
                        ? CrosshairMode.Hidden
                        : resolveCrosshairMode(next),
                    },
                  });
                }
              }}
              className="px-2 py-1 text-xs bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded border-0 focus:ring-2 focus:ring-blue-500"
            >
              <option value="normal">Normal Crosshair</option>
              <option value="magnet">Magnet to Close</option>
              <option value="magnetOHLC">Magnet to OHLC</option>
            </select>

            {/* Export button */}
            <button
              onClick={() => {
                if (chartData.length > 0) {
                  const csv = buildCsv(chartData, indicatorName, indicatorValues);
                  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
                  const url = URL.createObjectURL(blob);
                  const a = document.createElement('a');
                  a.href = url;
                  a.download = `${indicatorId}-${new Date().toISOString().split('T')[0]}.csv`;
                  a.click();
                  URL.revokeObjectURL(url);
                }
              }}
              className="p-2 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded"
              title="Export to CSV"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            </button>

            {/* Screenshot button */}
            <button
              onClick={takeScreenshot}
              className="p-2 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded"
              title="Take Screenshot"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 13a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
            </button>

            {/* Legend toggle */}
            <button
              onClick={() => setIsLegendVisible(v => !v)}
              className={`p-2 rounded ${isLegendVisible ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'}`}
              title={isLegendVisible ? 'Hide Legend' : 'Show Legend'}
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h10M4 18h7" />
              </svg>
            </button>

            {/* Fullscreen button */}
            <button
              onClick={toggleFullscreen}
              className="p-2 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded"
              title={isFullscreen ? 'Exit Fullscreen' : 'Enter Fullscreen'}
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                {isFullscreen ? (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 9V4.5M9 9H4.5M9 9L3.75 3.75M9 15v4.5M9 15H4.5M9 15l-5.25 5.25M15 9h4.5M15 9V4.5M15 9l5.25-5.25M15 15h4.5M15 15v4.5m0-4.5l5.25 5.25" />
                ) : (
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
                )}
              </svg>
            </button>
          </div>
        </div>
        )}
      </div>

      {/* Chart Container */}
      <div 
        className={`w-full bg-white dark:bg-gray-900 ${
          isFullscreen ? 'h-full' : ''
        }`}
        style={{ 
          height: isFullscreen 
            ? (isMobile ? 'calc(100% - 32px)' : 'calc(100% - 48px)') 
            : `${height}px`, 
          marginTop: isMobile ? '32px' : '48px',
          paddingBottom: isMobile ? '48px' : '0', // Space for mobile legend
          position: 'relative',
          width: '100%',
          maxWidth: '100%',
          overflow: 'hidden',
          boxSizing: 'border-box'
        }}
        ref={chartContainerRef}
        data-chart-container
      >
        {/* Desktop: Fixed Indicator Legend overlay (toggled) */}
        {!isMobile && isLegendVisible && (
          <div
            className="absolute top-14 left-2 z-20 bg-white/95 dark:bg-gray-900/95 backdrop-blur-sm rounded-lg shadow-lg hover:shadow-xl transition-shadow"
          >
            <IndicatorLegend
              indicatorName={indicatorName}
              indicatorId={indicatorId}
              currentValue={currentIndicatorValue}
              multiValues={currentMultiIndicatorValues}
              parameters={currentParameters}
              color={indicatorColor}
              onParameterChange={handleParameterChange}
              onColorChange={handleColorChange}
              allowColorChange={indicatorOutputMode === 'single'}
              onVisibilityToggle={() => setIsIndicatorVisible(!isIndicatorVisible)}
              isVisible={isIndicatorVisible}
            />
          </div>
        )}
        
        {/* Mobile: Fixed bottom indicator bar with bottom sheet (toggled) */}
        {isMobile && isLegendVisible && (
          <MobileIndicatorLegend
            indicatorName={indicatorName}
            indicatorId={indicatorId}
            currentValue={currentIndicatorValue}
            multiValues={currentMultiIndicatorValues}
            parameters={currentParameters}
            color={indicatorColor}
            onParameterChange={handleParameterChange}
            onColorChange={handleColorChange}
            allowColorChange={indicatorOutputMode === 'single'}
            onVisibilityToggle={() => setIsIndicatorVisible(!isIndicatorVisible)}
            isVisible={isIndicatorVisible}
          />
        )}
      </div>

      {indicatorError && (
        <div className="mt-2 text-xs text-amber-600 dark:text-amber-400">
          {indicatorError}
        </div>
      )}

      {/* Removed indicator info badge to prevent unwanted popup on desktop */}

      {/* Touch Gesture Indicators */}
      {isPinching && (
        <div className="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 pointer-events-none">
          <div className="bg-blue-500/20 backdrop-blur-sm rounded-full p-4 animate-pulse">
            <svg className="w-8 h-8 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v6m3-3H7" />
            </svg>
          </div>
        </div>
      )}

      {/* Mobile Touch Hint - moved higher to avoid controls */}
      {isMobile && (
        <div className="absolute top-20 right-4 text-xs text-gray-500 dark:text-gray-400 pointer-events-none animate-fade-in-out">
          <div className="bg-white/80 dark:bg-gray-900/80 backdrop-blur-sm px-2 py-1 rounded">
            Pinch to zoom • Double tap to reset
          </div>
        </div>
      )}


      {/* Tooltip */}
      {tooltipData && tooltipPosition && (
        <div
          className="absolute pointer-events-none bg-white/95 dark:bg-gray-900/95 backdrop-blur-sm px-3 py-2 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-20"
          style={{
            left: tooltipPosition.x + 15,
            top: tooltipPosition.y - 10,
            transform: tooltipPosition.x > window.innerWidth - 200 ? 'translateX(-100%)' : 'none',
          }}
        >
          <div className="text-xs space-y-1">
            {/* Date/Time */}
            <div className="font-semibold text-gray-900 dark:text-gray-100 border-b border-gray-200 dark:border-gray-700 pb-1 mb-1">
              {new Date(tooltipData.time * 1000).toLocaleString()}
            </div>
            
            {/* Candlestick data */}
            {tooltipData.candle && (
              <div className="space-y-0.5">
                <div className="flex justify-between gap-4">
                  <span className="text-gray-600 dark:text-gray-400">Open:</span>
                  <span className={`font-mono ${
                    tooltipData.candle.close >= tooltipData.candle.open ? 'text-green-700 dark:text-green-500' : 'text-red-700 dark:text-red-400'
                  }`}>
                    {tooltipData.candle.open?.toFixed(2)}
                  </span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-gray-600 dark:text-gray-400">High:</span>
                  <span className={`font-mono ${
                    tooltipData.candle.close >= tooltipData.candle.open ? 'text-green-700 dark:text-green-500' : 'text-red-700 dark:text-red-400'
                  }`}>{tooltipData.candle.high?.toFixed(2)}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-gray-600 dark:text-gray-400">Low:</span>
                  <span className={`font-mono ${
                    tooltipData.candle.close >= tooltipData.candle.open ? 'text-green-700 dark:text-green-500' : 'text-red-700 dark:text-red-400'
                  }`}>{tooltipData.candle.low?.toFixed(2)}</span>
                </div>
                <div className="flex justify-between gap-4">
                  <span className="text-gray-600 dark:text-gray-400">Close:</span>
                  <span className={`font-mono ${
                    tooltipData.candle.close >= tooltipData.candle.open ? 'text-green-700 dark:text-green-500' : 'text-red-700 dark:text-red-400'
                  }`}>
                    {tooltipData.candle.close?.toFixed(2)}
                  </span>
                </div>
              </div>
            )}
            
            {/* Single indicator data */}
            {tooltipData.indicator && (
              <div className="pt-1 border-t border-gray-200 dark:border-gray-700">
                <div className="flex justify-between gap-4">
                  <span className="text-gray-600 dark:text-gray-400">{indicatorName}:</span>
                  <span className="font-mono text-blue-600 dark:text-blue-400">
                    {typeof tooltipData.indicator.value === 'number' 
                      ? tooltipData.indicator.value.toFixed(2) 
                      : 'N/A'}
                  </span>
                </div>
              </div>
            )}
            
            {/* Multi-line indicator data */}
            {tooltipData.multiIndicator && (
              <div className="pt-1 border-t border-gray-200 dark:border-gray-700 space-y-0.5">
                {Object.entries(tooltipData.multiIndicator).map(([key, data]: [string, any]) => (
                  <div key={key} className="flex justify-between gap-4">
                    <span className="text-gray-600 dark:text-gray-400">{key}:</span>
                    <span className="font-mono text-blue-600 dark:text-blue-400">
                      {typeof data.value === 'number' ? data.value.toFixed(2) : 'N/A'}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Context Menu */}
      <ChartContextMenu
        x={contextMenu.x}
        y={contextMenu.y}
        isVisible={contextMenu.isVisible}
        onClose={() => setContextMenu({ ...contextMenu, isVisible: false })}
        onAction={handleContextMenuAction}
        currentChartType={chartType}
        isIndicatorVisible={isIndicatorVisible}
        isCrosshairVisible={isCrosshairVisible}
        isLogScale={isLogScale}
      />

      {/* Mobile Controls */}
      <MobileChartControls
        isFullscreen={isFullscreen}
        onToggleFullscreen={toggleFullscreen}
        onZoomIn={() => {
          if (chartRef.current) {
            const timeScale = chartRef.current.timeScale();
            const visibleRange = timeScale.getVisibleRange();
            if (visibleRange) {
              const rangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
              timeScale.setVisibleRange({
                from: (Number(visibleRange.from) + rangeWidth * 0.1) as Time,
                to: (Number(visibleRange.to) - rangeWidth * 0.1) as Time,
              });
            }
          }
        }}
        onZoomOut={() => {
          if (chartRef.current) {
            const timeScale = chartRef.current.timeScale();
            const visibleRange = timeScale.getVisibleRange();
            if (visibleRange) {
              const rangeWidth = Number(visibleRange.to) - Number(visibleRange.from);
              timeScale.setVisibleRange({
                from: (Number(visibleRange.from) - rangeWidth * 0.1) as Time,
                to: (Number(visibleRange.to) + rangeWidth * 0.1) as Time,
              });
            }
          }
        }}
        onResetView={() => {
          if (chartRef.current) {
            chartRef.current.timeScale().resetTimeScale();
            chartRef.current.priceScale('right').applyOptions({ autoScale: true });
            chartRef.current.timeScale().fitContent();
          }
        }}
        chartType={chartType}
        onChartTypeChange={setChartType}
        isIndicatorVisible={isIndicatorVisible}
        onToggleIndicator={() => setIsIndicatorVisible(!isIndicatorVisible)}
      />
    </div>
  );
}
