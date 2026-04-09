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
})
