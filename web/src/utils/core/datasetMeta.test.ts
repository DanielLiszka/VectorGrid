import { describe, expect, it } from 'vitest'
import {
  buildDatasetMetaForArtifact,
  DATASET_META_SCHEMA_VERSION,
  RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
  RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
} from './datasetMeta'

describe('datasetMeta', () => {
  it('builds artifact dataset provenance with resample identity and source fingerprint', () => {
    const meta = buildDatasetMetaForArtifact('C:/data/aapl.csv', {
      currentDataMeta: {
        len: 20,
        min_ts: 1700000000,
        max_ts: 1700001000,
        timeframe: '15m',
      },
      currentBaseDataMeta: {
        timeframe: '1m',
        source_size_bytes: 2048,
        source_mtime_utc_ms: 1700002000000,
        source_content_fingerprint: 'fp-aapl-1m',
      },
      currentBaseDatasetCtx: {
        base_timeframe: '1m',
        source: 'polygon',
        market: 'stocks',
        symbol: 'AAPL',
        adjusted: true,
      },
      currentDatasetCtx: {
        label: 'AAPL',
        timeframe: '15m',
        base_timeframe: '1m',
        derived_timeframe: '15m',
        source: 'polygon',
        market: 'stocks',
        symbol: 'AAPL',
        adjusted: true,
      },
    })

    expect(meta).toEqual({
      schema_version: DATASET_META_SCHEMA_VERSION,
      label: 'AAPL',
      csv_path: 'C:/data/aapl.csv',
      timeframe: '15m',
      base_timeframe: '1m',
      derived_timeframe: '15m',
      resample_applied: true,
      resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
      resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
      source: 'polygon',
      market: 'stocks',
      symbol: 'AAPL',
      adjusted: true,
      len: 20,
      min_ts: 1700000000,
      max_ts: 1700001000,
      source_size_bytes: 2048,
      source_mtime_utc_ms: 1700002000000,
      source_content_fingerprint: 'fp-aapl-1m',
    })
  })

  it('keeps base dataset provenance explicit when no derived timeframe is active', () => {
    const meta = buildDatasetMetaForArtifact('C:/data/msft.csv', {
      currentDataMeta: {
        len: 100,
        min_ts: 1700000000,
        max_ts: 1700005000,
        timeframe: '1m',
        source_size_bytes: 1024,
        source_mtime_utc_ms: 1700003000000,
      },
      currentBaseDataMeta: {
        timeframe: '1m',
      },
      currentBaseDatasetCtx: {
        source: 'csv',
      },
      currentDatasetCtx: {
        label: 'msft.csv',
        timeframe: '1m',
        source: 'csv',
      },
    })

    expect(meta).toEqual({
      schema_version: DATASET_META_SCHEMA_VERSION,
      label: 'msft.csv',
      csv_path: 'C:/data/msft.csv',
      timeframe: '1m',
      base_timeframe: '1m',
      derived_timeframe: '1m',
      resample_applied: false,
      resample_rule: RESAMPLE_RULE_SESSION_RELATIVE_OHLCV,
      resample_anchor: RESAMPLE_ANCHOR_DAY_FIRST_TIMESTAMP,
      source: 'csv',
      market: null,
      symbol: null,
      adjusted: null,
      len: 100,
      min_ts: 1700000000,
      max_ts: 1700005000,
      source_size_bytes: 1024,
      source_mtime_utc_ms: 1700003000000,
      source_content_fingerprint: null,
    })
  })

  it('preserves explicit resample identity from the current dataset context', () => {
    const meta = buildDatasetMetaForArtifact('C:/data/spy.csv', {
      currentDataMeta: {
        len: 50,
        min_ts: 1700000000,
        max_ts: 1700009000,
        timeframe: '30m',
      },
      currentBaseDataMeta: {
        timeframe: '5m',
      },
      currentBaseDatasetCtx: {
        source: 'polygon',
        market: 'stocks',
        symbol: 'SPY',
      },
      currentDatasetCtx: {
        label: 'SPY',
        timeframe: '30m',
        base_timeframe: '5m',
        derived_timeframe: '30m',
        resample_rule: 'explicit_rule',
        resample_anchor: 'explicit_anchor',
        source: 'polygon',
        market: 'stocks',
        symbol: 'SPY',
      },
    })

    expect(meta).toMatchObject({
      csv_path: 'C:/data/spy.csv',
      timeframe: '30m',
      base_timeframe: '5m',
      derived_timeframe: '30m',
      resample_applied: true,
      resample_rule: 'explicit_rule',
      resample_anchor: 'explicit_anchor',
      source: 'polygon',
      market: 'stocks',
      symbol: 'SPY',
    })
  })

  it('accepts frontend camelCase metadata fields when building artifact dataset provenance', () => {
    const meta = buildDatasetMetaForArtifact('C:/data/qqq.csv', {
      currentDataMeta: {
        len: 60,
        minTs: 1700000100,
        maxTs: 1700009100,
        timeframe: '1d',
        sourceSizeBytes: 4096,
        sourceMtimeUtcMs: 1700009200000,
        sourceContentFingerprint: 'fp-qqq-1d',
      },
      currentBaseDataMeta: {
        timeframe: '1d',
      },
      currentBaseDatasetCtx: {
        source: 'csv',
        symbol: 'QQQ',
      },
      currentDatasetCtx: {
        label: 'QQQ',
        timeframe: '1d',
        source: 'csv',
        symbol: 'QQQ',
      },
    })

    expect(meta).toMatchObject({
      csv_path: 'C:/data/qqq.csv',
      len: 60,
      min_ts: 1700000100,
      max_ts: 1700009100,
      source_size_bytes: 4096,
      source_mtime_utc_ms: 1700009200000,
      source_content_fingerprint: 'fp-qqq-1d',
      symbol: 'QQQ',
    })
  })
})
