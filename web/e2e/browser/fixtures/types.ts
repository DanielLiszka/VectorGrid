export const fixtureFamilyIds = [
  'empty',
  'dataset',
  'optimization',
  'validation',
  'artifactLibrary',
  'aggregate',
  'options',
  'mlSignal',
  'campaign',
  'liveExecution',
  'settings',
] as const

export type FixtureFamilyId = (typeof fixtureFamilyIds)[number]

export interface MockTauriScenario {
  name: string
  family: FixtureFamilyId
  description: string
  urlSearch?: string
  localStorage?: Record<string, string>
  sessionStorage?: Record<string, string>
  commandResults: Record<string, unknown>
  commandDelays?: Record<string, number>
  commandEvents?: Record<string, MockTauriScheduledEvent[]>
  expectedInvokes?: string[]
  forbiddenInvokes?: string[]
  tags?: string[]
}

export interface MockTauriScheduledEvent {
  event: string
  delayMs?: number
  payload: unknown
}

export interface FixtureFamilyModule {
  family: FixtureFamilyId
  description: string
  scenarios: MockTauriScenario[]
  deferred?: {
    reason: string
  }
}
