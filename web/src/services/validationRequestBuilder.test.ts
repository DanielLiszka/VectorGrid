import { describe, expect, it } from 'vitest'
import { buildValidationRequest } from './validationRequestBuilder'

describe('validationRequestBuilder', () => {
  it('builds double-like validation request from top rows with gates and aux configs', () => {
    const req = buildValidationRequest({
      currentRunReq: {
        strategy_kind: 'DoubleMa',
        data_id: 'data-123',
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
      wfEnable: true,
      wfWindow: '20000',
      wfStep: '5000',
      perturbEnable: true,
      perturbIters: '8',
      perturbMin: '0.01',
      perturbMax: '0.05',
      mcEnable: true,
      mcSims: '500',
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
    expect(req.seed).toBe(7)
    expect(req.min_psr).toBe(0.6)
    expect(req.min_dsr).toBe(0.4)
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
    expect(req.trade_bootstrap).toEqual({ simulations: 500 })
    expect(req.cscv_pbo).toEqual({
      slices: 16,
      max_combinations: 50000,
      threshold: 0
    })
    expect(req.cross_ticker).toEqual({
      max_datasets: 2,
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
