import { describe, expect, it } from 'vitest'
import { INSTRUMENTED_EVENT_CHANNELS } from './instrumentation'

describe('tauri instrumentation event parity', () => {
  it('subscribes to all required backend event channels', () => {
    expect(INSTRUMENTED_EVENT_CHANNELS).toEqual([
      'double_ma_progress',
      'distributed_progress',
      'live_execution_runner_event',
      'polygon_live_status',
      'polygon_live_bar',
    ])
  })
})
