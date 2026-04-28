import { createActor } from 'xstate'
import { describe, expect, it } from 'vitest'
import {
  liveExecutionLifecycleMachine,
  liveExecutionLifecycleStateName,
  type LiveExecutionLifecycleEvent,
  type LiveExecutionTarget,
} from './liveExecutionLifecycleMachine'

interface TransitionCase {
  name: string
  target: LiveExecutionTarget
  events: LiveExecutionLifecycleEvent[]
  expectedState: string
  expectedRecovery: boolean
}

function run(events: LiveExecutionLifecycleEvent[]) {
  const actor = createActor(liveExecutionLifecycleMachine)
  actor.start()
  for (const event of events) actor.send(event)
  return actor.getSnapshot()
}

describe('liveExecutionLifecycleMachine', () => {
  const cases: TransitionCase[] = [
    {
      name: 'arms live stream before starting runner',
      target: 'dry_run',
      events: [
        { type: 'START_LIVE', symbol: ' spy ', target: 'dry_run' },
        { type: 'LIVE_SUBSCRIBED' },
        { type: 'START_RUNNER' },
        { type: 'RUNNER_STARTED' },
      ],
      expectedState: 'running',
      expectedRecovery: false,
    },
    {
      name: 'returns to stream state when the runner is stopped',
      target: 'alpaca_paper',
      events: [
        { type: 'START_LIVE', symbol: 'MSFT', target: 'alpaca_paper', delayed: false },
        { type: 'LIVE_SUBSCRIBED' },
        { type: 'START_RUNNER' },
        { type: 'RUNNER_STARTED' },
        { type: 'RUNNER_STOPPED' },
      ],
      expectedState: 'runnerStopped',
      expectedRecovery: false,
    },
    {
      name: 'requires broker review after an armed feed interruption',
      target: 'alpaca_live',
      events: [
        { type: 'START_LIVE', symbol: 'AAPL', target: 'alpaca_live' },
        { type: 'LIVE_SUBSCRIBED' },
        { type: 'START_RUNNER' },
        { type: 'RUNNER_STARTED' },
        { type: 'LIVE_INTERRUPTED', reason: 'remote close' },
      ],
      expectedState: 'recoveryRequired',
      expectedRecovery: true,
    },
    {
      name: 'dry-run interruptions disarm without broker recovery',
      target: 'dry_run',
      events: [
        { type: 'START_LIVE', symbol: 'QQQ', target: 'dry_run' },
        { type: 'LIVE_SUBSCRIBED' },
        { type: 'LIVE_INTERRUPTED', reason: 'network timeout' },
      ],
      expectedState: 'idle',
      expectedRecovery: false,
    },
    {
      name: 'broker review releases recovery lock',
      target: 'local_paper',
      events: [
        { type: 'START_LIVE', symbol: 'TSLA', target: 'local_paper' },
        { type: 'LIVE_SUBSCRIBED' },
        { type: 'LIVE_INTERRUPTED', reason: 'stream read failed' },
        { type: 'BROKER_REVIEW_CONFIRMED' },
      ],
      expectedState: 'idle',
      expectedRecovery: false,
    },
    {
      name: 'emergency halt preserves armed recovery requirement',
      target: 'alpaca_paper',
      events: [
        { type: 'START_LIVE', symbol: 'NVDA', target: 'alpaca_paper' },
        { type: 'LIVE_SUBSCRIBED' },
        { type: 'EMERGENCY_HALT', reason: 'operator halt' },
      ],
      expectedState: 'halted',
      expectedRecovery: true,
    },
  ]

  it.each(cases)('$name', ({ events, expectedState, expectedRecovery }) => {
    const snapshot = run(events)
    expect(liveExecutionLifecycleStateName(snapshot.value)).toBe(expectedState)
    expect(snapshot.context.recoveryReviewRequired).toBe(expectedRecovery)
  })

  it('normalizes symbol and delayed context on start', () => {
    const snapshot = run([
      { type: 'START_LIVE', symbol: ' brk.b ', target: 'alpaca_paper', delayed: false },
    ])

    expect(snapshot.context.symbol).toBe('BRK.B')
    expect(snapshot.context.delayed).toBe(false)
    expect(snapshot.context.target).toBe('alpaca_paper')
  })
})
