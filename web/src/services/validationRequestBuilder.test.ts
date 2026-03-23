import { describe, expect, it } from 'vitest'
import { buildValidationCapabilitySummary, buildValidationRequest } from './validationRequestBuilder'

describe('validationRequestBuilder', () => {
  it('builds double-like validation request from top rows with gates and aux configs', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['alma'],
        strategy: { fee_bps: 1.5 },
        halftrend_filter: { enabled: true }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'alma',
            fast_len: 10,
            slow_len: 40,
            fast_ma_params: { alpha: 2.0 },
            slow_ma_params: { offset: 0.85 }
          }
        }
      ],
      topRowsFiltered: [],
      candidates: '32',
      seed: '7',
      minPsr: '0.6',
      minDsr: '0.4',
      spaEnable: true,
      spaSlices: '12',
      spaBootstrapSamples: '750',
      spaBlockLength: '5',
      reoptWfEnable: true,
      reoptWfWindow: '12000',
      reoptWfStep: '3000',
      reoptWfTopN: '5',
      wfEnable: true,
      wfWindow: '20000',
      wfStep: '5000',
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: true,
      mcSims: '500',
      mcBlockLen: '9',
      cscvEnable: true,
      cscvSlices: '17',
      crossEnable: true,
      crossMax: '2',
      crossDatasets: [
        { csv_path: 'C:/data/a.csv', label: 'A' },
        { csv_path: 'C:/data/b.csv', label: 'B' },
        { csv_path: 'C:/data/c.csv', label: 'C' }
      ]
    })

    expect(req.data_id).toBe('data-123')
    expect(req.backend).toEqual({ backend: 'GpuKernel', device_id: 0 })
    expect(req.seed).toBe(7)
    expect(req.min_psr).toBe(0.6)
    expect(req.min_dsr).toBe(0.4)
    expect(req.spa).toEqual({
      slices: 12,
      bootstrap_samples: 750,
      block_length: 5,
      benchmark: 'buy_hold',
    })
    expect(req.reopt_walk_forward).toEqual({
      window_bars: 12000,
      step_bars: 3000,
      top_n_candidates: 5,
    })
    expect(req.candidates).toHaveLength(1)
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'sma',
      slow_ma_type: 'alma',
      fast_len: 10,
      slow_len: 40
    })
    expect(req.walk_forward).toEqual({ window_bars: 20000, step_bars: 5000 })
    expect(req.perturb).toEqual({
      iterations: 8,
      pct_min: 0.01,
      pct_max: 0.05,
      flip_bool_prob: 0.02
    })
    expect(req.trade_bootstrap).toEqual({ simulations: 500, stationary_mean_block_len: 9 })
    expect(req.cscv_pbo).toEqual({
      slices: 16,
      max_combinations: 50000,
      threshold: 0
    })
    expect(req.cross_ticker).toEqual({
      max_datasets: 2,
      mode: 'preserved_finalists',
      datasets: [
        { csv_path: 'C:/data/a.csv', label: 'A' },
        { csv_path: 'C:/data/b.csv', label: 'B' }
      ]
    })
  })

  it('builds composite request and enforces halftrend presence', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'CompositeMo',
        data_id: 'data-456',
        ma_source: 'hlc3',
        strategy: { eps_rel: 0.01 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'soft_vote',
            strategy: { fee_bps: 2 }
          }
        },
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 7,
            slow_len: 25
          }
        }
      ],
      candidates: '10'
    })

    expect(req.candidates).toHaveLength(1)
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'ema',
      slow_ma_type: 'sma',
      signal_mode: 'soft_vote'
    })
  })

  it('dedupes normalized validation candidates before applying the max candidate limit', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-dedupe',
        ma_source: 'close',
        fast_ma_types: ['alma'],
        slow_ma_types: ['alma'],
        strategy: { fee_bps: 1.0 },
        halftrend_filter: { enabled: true }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'alma',
            slow_ma_type: 'alma',
            fast_len: 9,
            slow_len: 30,
            fast_ma_params: { sigma: 6, offset: 0.85 },
            slow_ma_params: { offset: 0.85, sigma: 6 }
          }
        },
        {
          params: {
            fast_ma_type: 'alma',
            slow_ma_type: 'alma',
            fast_len: 9,
            slow_len: 30,
            fast_ma_params: { offset: 0.85, sigma: 6 },
            slow_ma_params: { sigma: 6, offset: 0.85 }
          }
        },
        {
          params: {
            fast_ma_type: 'alma',
            slow_ma_type: 'alma',
            fast_len: 11,
            slow_len: 40,
            fast_ma_params: { offset: 0.85, sigma: 6 },
            slow_ma_params: { offset: 0.85, sigma: 6 }
          }
        }
      ],
      candidates: '2'
    })

    expect(req.candidates).toHaveLength(2)
    expect(req.candidates[0]).toMatchObject({ fast_len: 9, slow_len: 30 })
    expect(req.candidates[1]).toMatchObject({ fast_len: 11, slow_len: 40 })
  })

  it('dedupes cross datasets by normalized path', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-cross-dedupe',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['sma'],
        strategy: { fee_bps: 1.0 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'sma',
            fast_len: 10,
            slow_len: 40
          }
        }
      ],
      candidates: '4',
      crossEnable: true,
      crossMax: '3',
      crossDatasets: [
        { csv_path: 'C:/data/AAPL.csv', label: 'AAPL first' },
        { csv_path: ' c:\\DATA\\aapl.csv ', label: 'AAPL duplicate' },
        { csv_path: 'D:/data/MSFT.csv', label: 'MSFT' }
      ]
    })

    expect(req.cross_ticker).toEqual({
      max_datasets: 3,
      mode: 'preserved_finalists',
      datasets: [
        { csv_path: 'C:/data/AAPL.csv', label: 'AAPL first' },
        { csv_path: 'D:/data/MSFT.csv', label: 'MSFT' }
      ]
    })
  })

  it('treats Marrt as a double-like validation strategy', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Marrt',
        data_id: 'data-ribbon',
        ma_source: 'close',
        fast_ma_types: ['alma'],
        slow_ma_types: ['vwma'],
        strategy: { fee_bps: 1.0 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'alma',
            slow_ma_type: 'vwma',
            fast_len: 8,
            slow_len: 34,
            fast_ma_params: { offset: 0.85 }
          }
        }
      ],
      candidates: '8'
    })

    expect(req.validation_context.strategy_kind).toBe('Marrt')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'alma',
      slow_ma_type: 'vwma',
      fast_len: 8,
      slow_len: 34
    })
  })

  it('treats Set as a double-like validation strategy', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Set',
        data_id: 'data-set',
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['sma'],
        strategy: { slope_efficiency_trend_enable: true, set_er_period: 14 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'sma',
            fast_len: 8,
            slow_len: 34,
          }
        }
      ],
      candidates: '8'
    })

    expect(req.validation_context.strategy_kind).toBe('Set')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'sma',
      slow_ma_type: 'sma',
      fast_len: 8,
      slow_len: 34,
      strategy: { slope_efficiency_trend_enable: true, set_er_period: 14 }
    })
  })

  it('treats Pit as a double-like validation strategy', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Pit',
        data_id: 'data-pit',
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['sma'],
        strategy: { pressure_impulse_trend_enable: true, pit_efi_period: 13 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'sma',
            fast_len: 8,
            slow_len: 34,
          }
        }
      ],
      candidates: '8'
    })

    expect(req.validation_context.strategy_kind).toBe('Pit')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'sma',
      slow_ma_type: 'sma',
      fast_len: 8,
      slow_len: 34,
      strategy: { pressure_impulse_trend_enable: true, pit_efi_period: 13 }
    })
  })

  it('treats Mts as a double-like validation strategy', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Mts',
        data_id: 'data-mts',
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['sma'],
        strategy: { momentum_triple_stack_enable: true, mts_roc_period: 10 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'sma',
            fast_len: 8,
            slow_len: 34,
          }
        }
      ],
      candidates: '8'
    })

    expect(req.validation_context.strategy_kind).toBe('Mts')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'sma',
      slow_ma_type: 'sma',
      fast_len: 8,
      slow_len: 34,
      strategy: { momentum_triple_stack_enable: true, mts_roc_period: 10 }
    })
  })

  it('treats Orc as a double-like validation strategy', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Orc',
        data_id: 'data-orc',
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['sma'],
        strategy: { oscillator_reversion_confluence_enable: true, orc_rsi_period: 14 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'sma',
            fast_len: 8,
            slow_len: 34,
          }
        }
      ],
      candidates: '8'
    })

    expect(req.validation_context.strategy_kind).toBe('Orc')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'sma',
      slow_ma_type: 'sma',
      fast_len: 8,
      slow_len: 34,
      strategy: { oscillator_reversion_confluence_enable: true, orc_rsi_period: 14 }
    })
  })

  it('treats Vfct as a double-like validation strategy', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Vfct',
        data_id: 'data-vfct',
        ma_source: 'close',
        fast_ma_types: ['vwma'],
        slow_ma_types: ['vwma'],
        strategy: { volume_flow_confirm_enable: true, volume_flow_mfi_period: 14 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'vwma',
            slow_ma_type: 'vwma',
            fast_len: 8,
            slow_len: 34,
          }
        }
      ],
      candidates: '8'
    })

    expect(req.validation_context.strategy_kind).toBe('Vfct')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'vwma',
      slow_ma_type: 'vwma',
      fast_len: 8,
      slow_len: 34,
      strategy: { volume_flow_confirm_enable: true, volume_flow_mfi_period: 14 }
    })
  })

  it('builds ROCB validation request using run-level ROCB config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Rocb',
        data_id: 'rocb-data',
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['ema'],
        strategy: { fee_bps: 1.0 },
        rocb: { channel_period: 20, atr_period: 14, atr_stop_mult: 2.0, max_trade_bars: 80 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'ema',
            fast_len: 10,
            slow_len: 40,
            rocb: { channel_period: 22, atr_period: 16, atr_stop_mult: 2.2, max_trade_bars: 90 }
          }
        }
      ],
      candidates: '5',
      seed: '6'
    })

    expect(req.data_id).toBe('rocb-data')
    expect(req.seed).toBe(6)
    expect(req.validation_context.strategy_kind).toBe('Rocb')
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'sma',
      slow_ma_type: 'ema',
      fast_len: 10,
      slow_len: 40,
      rocb: { channel_period: 22, atr_period: 16, atr_stop_mult: 2.2, max_trade_bars: 90 }
    })
  })

  it('builds ASR validation request using run-level ASR config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Asr',
        data_id: 'asr-data',
        ma_source: 'close',
        fast_ma_types: ['alma'],
        slow_ma_types: ['alma'],
        strategy: { fee_bps: 1.0 },
        asr: { angle_deg_min: 12, z_entry: 1.8 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'alma',
            slow_ma_type: 'alma',
            fast_len: 20,
            slow_len: 80
          }
        }
      ],
      candidates: '5',
      seed: '3'
    })

    expect(req.data_id).toBe('asr-data')
    expect(req.seed).toBe(3)
    expect(req.candidates).toHaveLength(1)
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'alma',
      slow_ma_type: 'alma',
      fast_len: 20,
      slow_len: 80,
      asr: { angle_deg_min: 12, z_entry: 1.8 }
    })
  })

  it('builds VEMR validation request using run-level VEMR config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Vemr',
        data_id: 'vemr-data',
        ma_source: 'close',
        fast_ma_types: ['ema'],
        slow_ma_types: ['ema'],
        strategy: { fee_bps: 1.0 },
        vemr: { z_entry: 2.1, atr_stop_mult: 1.8 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'ema',
            fast_len: 14,
            slow_len: 55
          }
        }
      ],
      candidates: '5',
      seed: '9'
    })

    expect(req.data_id).toBe('vemr-data')
    expect(req.seed).toBe(9)
    expect(req.candidates).toHaveLength(1)
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'ema',
      slow_ma_type: 'ema',
      fast_len: 14,
      slow_len: 55,
      vemr: { z_entry: 2.1, atr_stop_mult: 1.8 }
    })
  })

  it('builds MKCS validation request using run-level MKCS config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Mkcs',
        data_id: 'mkcs-data',
        ma_source: 'close',
        backend: { backend: 'GpuKernel', device_id: 0 },
        fast_ma_types: ['ema'],
        slow_ma_types: ['sma'],
        strategy: { fee_bps: 1.25 },
        mkcs: { compression_threshold: 0.01, reexpansion_threshold: 0.03 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 12,
            slow_len: 48,
            mkcs: { stddev_period: 20, breakout_lookback: 30 }
          }
        }
      ],
      candidates: '5',
      seed: '5'
    })

    expect(req.data_id).toBe('mkcs-data')
    expect(req.seed).toBe(5)
    expect(req.validation_context.strategy_kind).toBe('Mkcs')
    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    expect(req.candidates).toHaveLength(1)
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'ema',
      slow_ma_type: 'sma',
      fast_len: 12,
      slow_len: 48,
      mkcs: { stddev_period: 20, breakout_lookback: 30 }
    })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CPU-preferred composite route (MKCS)')
    expect(summary.warningSummary).toContain('Auto composite validation currently prefers CPU for MKCS')
  })

  it('builds KQSR validation request using run-level KQSR config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Kqsr',
        data_id: 'kqsr-data',
        ma_source: 'close',
        fast_ma_types: ['ema'],
        slow_ma_types: ['ema'],
        strategy: { fee_bps: 0.5 },
        kqsr: { oversold: 18, overbought: 82 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'ema',
            fast_len: 12,
            slow_len: 36,
            kqsr: { prb_period: 24, prb_threshold: 0.15 }
          }
        }
      ]
    })

    expect(req.validation_context.strategy_kind).toBe('Kqsr')
    expect(req.candidates[0]).toMatchObject({
      fast_len: 12,
      slow_len: 36,
      kqsr: { prb_period: 24, prb_threshold: 0.15 }
    })
  })

  it('builds CARM validation request using run-level CARM config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Carm',
        data_id: 'carm-data',
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['ema'],
        strategy: { fee_bps: 0.5 },
        carm: { otto_threshold: 0.1 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'ema',
            fast_len: 10,
            slow_len: 42,
            carm: { rocr_period: 12, otto_period: 24 }
          }
        }
      ]
    })

    expect(req.validation_context.strategy_kind).toBe('Carm')
    expect(req.candidates[0]).toMatchObject({
      fast_len: 10,
      slow_len: 42,
      carm: { rocr_period: 12, otto_period: 24 }
    })
  })

  it('builds KVDF validation request using run-level KVDF config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Kvdf',
        data_id: 'kvdf-data',
        ma_source: 'close',
        fast_ma_types: ['ema'],
        slow_ma_types: ['sma'],
        strategy: { fee_bps: 0.5 },
        kvdf: { keltner_mult: 1.8, dti_threshold: 22 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 15,
            slow_len: 55,
            kvdf: { fvg_lookback: 18, devstop_period: 16 }
          }
        }
      ]
    })

    expect(req.validation_context.strategy_kind).toBe('Kvdf')
    expect(req.candidates[0]).toMatchObject({
      fast_len: 15,
      slow_len: 55,
      kvdf: { fvg_lookback: 18, devstop_period: 16 }
    })
  })

  it('builds CDMC validation request using run-level CDMC config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Cdmc',
        data_id: 'cdmc-data',
        ma_source: 'close',
        fast_ma_types: ['ema'],
        slow_ma_types: ['ema'],
        strategy: { fee_bps: 0.5 },
        cdmc: { mama_fast_limit: 0.45, mama_slow_limit: 0.08 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'ema',
            fast_len: 11,
            slow_len: 44,
            cdmc: { dpo_period: 22, decycler_period: 34 }
          }
        }
      ]
    })

    expect(req.validation_context.strategy_kind).toBe('Cdmc')
    expect(req.candidates[0]).toMatchObject({
      fast_len: 11,
      slow_len: 44,
      cdmc: { dpo_period: 22, decycler_period: 34 }
    })
  })

  it('builds VCC validation request using run-level VCC config', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Vcc',
        data_id: 'vcc-data',
        ma_source: 'close',
        fast_ma_types: ['ema'],
        slow_ma_types: ['ema'],
        strategy: { fee_bps: 1.0 },
        vcc: { squeeze_period: 20, trend_period: 6 }
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'ema',
            fast_len: 10,
            slow_len: 40
          }
        }
      ],
      candidates: '5',
      seed: '4'
    })

    expect(req.data_id).toBe('vcc-data')
    expect(req.seed).toBe(4)
    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    expect(req.candidates).toHaveLength(1)
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'ema',
      slow_ma_type: 'ema',
      fast_len: 10,
      slow_len: 40,
      vcc: { squeeze_period: 20, trend_period: 6 }
    })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CPU-preferred composite route (VCC)')
    expect(summary.warningSummary).toContain('Auto composite validation currently prefers CPU for VCC')
  })

  it('summarizes double-ma gpu preserved-finalists capability expectations', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-gpu',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        fast_ma_types: ['alma'],
        slow_ma_types: ['alma'],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'alma',
            slow_ma_type: 'alma',
            fast_len: 9,
            slow_len: 30,
          }
        }
      ],
      wfEnable: true,
      wfWindow: '12000',
      wfStep: '3000',
      reoptWfEnable: true,
      reoptWfWindow: '12000',
      reoptWfStep: '3000',
      reoptWfTopN: '4',
      spaEnable: true,
      spaSlices: '12',
      spaBootstrapSamples: '300',
      spaBlockLength: '4',
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: true,
      mcSims: '500',
      mcBlockLen: '9',
      cscvEnable: true,
      cscvSlices: '16',
      crossEnable: true,
      crossMax: '2',
      crossMode: 'preserved_finalists',
      crossDatasets: [
        { csv_path: 'C:/data/SPY.csv', label: 'SPY' },
        { csv_path: 'C:/data/QQQ.csv', label: 'QQQ' }
      ]
    })

    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('GpuKernel requested')
    expect(summary.crossSummary).toContain('2 datasets')
    expect(summary.crossSummary).toContain('finalists only')
    expect(summary.moduleSummary).toContain('GPU main route')
    expect(summary.moduleSummary).toContain('auxiliary CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=9)')
    expect(summary.categorySummary).toContain('GPU-native: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa')
    expect(summary.categorySummary).toContain('GPU main + CPU tail: perturb + trade_bootstrap')
    expect(summary.warningSummary).toContain('Preserved-finalists cross reruns keep CSCV/PBO + hard gates')
  })

  it('warns when gpuonly is requested for fixed-candidate double-ma validation', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-gpuonly',
        backend: { backend: 'GpuOnly', device_id: 0 },
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['sma'],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'sma',
            fast_len: 8,
            slow_len: 34,
          }
        }
      ],
      candidates: '4'
    })

    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CpuOnly fallback')
    expect(summary.moduleSummary).toContain('Expected CPU fallback route')
    expect(summary.categorySummary).toContain('CPU-only: base reruns + hard gates')
    expect(summary.warningSummary).toContain('GpuOnly is not supported for fixed-candidate Double-MA validation')
  })

  it('supports iid-only bootstrap mode for validation requests and summaries', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-iid',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        fast_ma_types: ['sma'],
        slow_ma_types: ['alma'],
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'sma',
            slow_ma_type: 'alma',
            fast_len: 10,
            slow_len: 40,
          }
        }
      ],
      mcEnable: true,
      mcSims: '400',
      mcMode: 'iid_only',
      mcBlockLen: '9',
      candidates: '8'
    })

    expect(req.trade_bootstrap).toEqual({ simulations: 400, stationary_mean_block_len: 0 })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.moduleSummary).toContain('trade bootstrap iid-only')
  })

  it('surfaces CompositeMo cross-ticker fallback and lightweight-rerun expectations explicitly', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'CompositeMo',
        data_id: 'data-composite-cross',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'soft_vote',
          }
        }
      ],
      wfEnable: true,
      wfWindow: '12000',
      wfStep: '3000',
      cscvEnable: true,
      cscvSlices: '16',
      crossEnable: true,
      crossMax: '2',
      crossMode: 'lightweight',
      crossDatasets: [
        { csv_path: 'C:/data/SPY.csv', label: 'SPY' },
        { csv_path: 'C:/data/QQQ.csv', label: 'QQQ' }
      ],
      candidates: '8'
    })

    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CpuOnly fallback for composite validation')
    expect(summary.moduleSummary).toContain('Expected CPU fallback route (base composite)')
    expect(summary.categorySummary).toContain('CPU-only: base reruns + hard gates + walk_forward + cscv_pbo')
    expect(summary.crossSummary).toContain('lightweight reruns')
    expect(summary.skippedSummary).toContain('Cross reruns: composite main route does not support inline cross_ticker; lightweight composite reruns omit walk_forward + reopt_walk_forward + SPA + perturb + trade bootstrap + CSCV + hard gates')
    expect(summary.warningSummary).toContain('CompositeMo GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only')
    expect(summary.warningSummary).toContain('Composite fixed-candidate main-route validation does not support inline cross_ticker; composite cross reruns are lightweight and omit walk_forward')
  })

  it('surfaces plain CompositeCpu GPU route restrictions explicitly in previews', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'CompositeCpu',
        data_id: 'data-composite-cpu',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        strategy: { fee_bps: 1.0 },
        halftrend: { amplitude: 2, channel_deviation: 2.0, atr_period: 100 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 9,
            slow_len: 30,
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'hard_gate',
            strategy: { fee_bps: 1.0 },
          }
        }
      ],
      candidates: '8'
    })

    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected GPU composite route (base composite)')
    expect(summary.warningSummary).toContain('Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists')
  })

  it('defaults Carm composite validation to Auto so CPU-preferred policy applies after GPU optimization', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Carm',
        data_id: 'data-carm-preview',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            carm: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'hard_gate',
          }
        }
      ],
      candidates: '8'
    })

    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CPU-preferred composite route (CARM)')
    expect(summary.moduleSummary).toContain('Expected CPU route (CARM)')
    expect(summary.categorySummary).toContain('CPU-only: base reruns + hard gates')
    expect(summary.warningSummary).toContain('Auto composite validation currently prefers CPU for CARM')
  })

  it('defaults VCET validation to Auto and carries vcet params into candidates', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Vcet',
        data_id: 'data-vcet',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        vcet: {
          vidya_short_period: 6,
          vidya_long_period: 24,
          ce_period: 22,
          ce_mult: 3.0,
        },
        strategy: { fee_bps: 1.0 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'vidya',
            slow_ma_type: 'vidya',
            fast_len: 8,
            slow_len: 34,
            vcet: {
              vidya_short_period: 8,
              vidya_long_period: 34,
              ce_period: 34,
              ce_mult: 2.8,
            },
            strategy: { fee_bps: 1.0 },
          }
        }
      ],
      candidates: '8'
    })

    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    expect(req.candidates[0]).toMatchObject({
      fast_ma_type: 'vidya',
      slow_ma_type: 'vidya',
      fast_len: 8,
      slow_len: 34,
      vcet: {
        vidya_short_period: 8,
        vidya_long_period: 34,
        ce_period: 34,
        ce_mult: 2.8,
      },
    })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected GPU composite route (VCET)')
    expect(summary.warningSummary).toContain('VCET')
  })

  it('treats Carm Auto composite validation as CPU-preferred in previews', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Carm',
        data_id: 'data-carm-auto-preview',
        backend: { backend: 'Auto', device_id: 0 },
        ma_source: 'close',
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            carm: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'hard_gate',
          }
        }
      ],
      candidates: '8'
    })

    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CPU-preferred composite route (CARM)')
    expect(summary.moduleSummary).toContain('Expected CPU route (CARM)')
    expect(summary.categorySummary).toContain('CPU-only: base reruns + hard gates')
    expect(summary.warningSummary).toContain('Auto composite validation currently prefers CPU for CARM')
  })

  it('treats Cdmc Auto composite validation as GPU-routed in previews', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Cdmc',
        data_id: 'data-cdmc-auto-preview',
        backend: { backend: 'Auto', device_id: 0 },
        ma_source: 'close',
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            cdmc: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'hard_gate',
          }
        }
      ],
      candidates: '8'
    })

    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected GPU composite route (CDMC)')
    expect(summary.moduleSummary).toContain('Expected GPU composite route (CDMC)')
    expect(summary.categorySummary).toContain('GPU main + CPU tail: base reruns + hard gates')
    expect(summary.warningSummary).toContain('CDMC GPU validation currently uses CPU-reference signal prep with GPU metric evaluation')
  })

  it('defaults Cdmc composite validation to Auto so the measured GPU-preferred policy remains active', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'Cdmc',
        data_id: 'data-cdmc-preview',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 10,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            cdmc: {},
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'hard_gate',
          }
        }
      ],
      candidates: '8'
    })

    expect(req.backend).toEqual({ backend: 'Auto', device_id: 0 })
    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected GPU composite route (CDMC)')
    expect(summary.moduleSummary).toContain('Expected GPU composite route (CDMC)')
    expect(summary.categorySummary).toContain('GPU main + CPU tail: base reruns + hard gates')
    expect(summary.warningSummary).toContain('CDMC GPU validation currently uses CPU-reference signal prep with GPU metric evaluation')
  })

  it('treats CompositeMo GPU fixed-candidate validation as CpuOnly fallback in previews', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'CompositeMo',
        data_id: 'data-composite-mo',
        backend: { backend: 'GpuKernel', device_id: 0 },
        ma_source: 'close',
        strategy: { fee_bps: 1.0 },
        halftrend: { amplitude: 2 },
      },
      currentRunRes: { ok: true },
      topRows: [
        {
          params: {
            fast_ma_type: 'ema',
            slow_ma_type: 'sma',
            fast_len: 9,
            slow_len: 30,
            halftrend: { amplitude: 2 },
            macz: { fast_length: 12 },
            ott: { period: 2 },
            signal_mode: 'soft_vote',
          }
        }
      ],
      candidates: '8'
    })

    const summary = buildValidationCapabilitySummary(req)
    expect(summary.routeSummary).toContain('expected CpuOnly fallback for composite validation')
    expect(summary.warningSummary).toContain('CompositeMo GPU fixed-candidate validation currently supports plain CompositeCpu HardGate candidates only')
  })

  it('throws for missing run context or rows', () => {
    expect(() =>
      buildValidationRequest({
        currentRunReq: null,
        currentRunRes: { ok: true },
        topRows: []
      })
    ).toThrow('Run an optimization first.')

    expect(() =>
      buildValidationRequest({
        currentRunReq: { strategy_kind: 'DoubleMa', data_id: '' },
        currentRunRes: { ok: true },
        topRows: []
      })
    ).toThrow('Missing data id (run an optimization first).')

    expect(() =>
      buildValidationRequest({
        currentRunReq: { strategy_kind: 'DoubleMa', data_id: 'x' },
        currentRunRes: { ok: true },
        topRows: [],
        topRowsFiltered: []
      })
    ).toThrow('No Top rows available. Go to the Top tab and run an optimization.')
  })
})
