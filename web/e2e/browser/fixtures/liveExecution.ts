import type { FixtureFamilyModule } from './types'
import { liveReadyScenario, liveSafetyReadyScenario } from './scenarios'

export const liveStreamStopReadyScenario = {
  ...liveSafetyReadyScenario,
  name: 'live-stream-stop-ready',
  family: 'liveExecution' as const,
  description:
    'A local-paper live stream can be started and stopped through explicit live controls.',
  commandResults: {
    ...liveSafetyReadyScenario.commandResults,
    polygon_stop_live_candle_stream: {
      stopped: true,
      symbol: 'AAPL',
      delayed: true,
    },
  },
  expectedInvokes: [
    'polygon_start_live_candle_stream',
    'polygon_stop_live_candle_stream',
  ],
}

export { liveReadyScenario, liveSafetyReadyScenario }

export const liveExecutionFixtureFamily: FixtureFamilyModule = {
  family: 'liveExecution',
  description: 'Live execution, local-paper safety, credentials, and broker-review browser QA scenarios.',
  scenarios: [liveReadyScenario, liveSafetyReadyScenario, liveStreamStopReadyScenario],
}
