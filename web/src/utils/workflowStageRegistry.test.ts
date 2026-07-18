import { describe, expect, it } from 'vitest'
import {
  workflowStageDefinitionForMode,
  workflowStageDefinitionsForMode,
} from './workflowStageRegistry'

describe('workflowStageRegistry', () => {
  it('projects conventional stage metadata from the shared workflow vocabulary', () => {
    const stages = workflowStageDefinitionsForMode('conventional')

    expect(stages.map((stage) => stage.id)).toEqual([
      'data',
      'optimize',
      'validate',
      'analyze',
      'finalize',
    ])
    expect(stages.map((stage) => stage.label)).toEqual([
      'Data Source',
      'Strategy Setup',
      'Validation',
      'Summary',
      'Final Review',
    ])
    expect(stages.find((stage) => stage.id === 'validate')?.description).toContain(
      'robustness checks',
    )
  })

  it('keeps ML Signal labels mode-specific without changing persisted step IDs', () => {
    const stages = workflowStageDefinitionsForMode('ml-signal')

    expect(stages.map((stage) => stage.id)).toEqual([
      'data',
      'optimize',
      'validate',
      'analyze',
      'finalize',
    ])
    expect(stages.map((stage) => stage.label)).toEqual([
      'Data Setup',
      'Score Cache',
      'Policy Validation',
      'Policy Sweep',
      'Artifacts',
    ])
  })

  it('projects Options labels as a first-class research intent', () => {
    const stages = workflowStageDefinitionsForMode('options')

    expect(stages.map((stage) => stage.id)).toEqual([
      'data',
      'optimize',
      'validate',
      'analyze',
      'finalize',
    ])
    expect(stages.map((stage) => stage.label)).toEqual([
      'Underlying Data',
      'Options Data',
      'Expression Run',
      'Validation',
      'Final Review',
    ])
  })

  it('repairs unknown stage and mode requests to the first safe stage', () => {
    expect(workflowStageDefinitionForMode('wat', 'wat')).toMatchObject({
      id: 'data',
      label: 'Data Source',
    })
  })
})
