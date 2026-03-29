import { describe, expect, it } from 'vitest'
import type { WorkstationPerspective } from '../stores/settingsStore'
import { searchPerspectives } from './perspectiveSearch'

const PERSPECTIVES: WorkstationPerspective[] = [
  {
    id: 'builtin_optimize',
    name: 'Optimize',
    builtIn: true,
    snapshot: {
      workspace: 'workflow',
      workflow: 'optimize',
      workflowStep: 'optimize',
      controlsTab: 'opt',
      resultsTab: 'overview',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'standard',
    },
  },
  {
    id: 'builtin_validate',
    name: 'Validate',
    builtIn: true,
    snapshot: {
      workspace: 'workflow',
      workflow: 'validate',
      workflowStep: 'validate',
      controlsTab: 'opt',
      resultsTab: 'validate',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'standard',
    },
  },
  {
    id: 'builtin_run_library',
    name: 'Run Library',
    builtIn: true,
    snapshot: {
      workspace: 'library',
      workflow: 'history',
      workflowStep: 'data',
      controlsTab: 'data',
      resultsTab: 'history',
      aggregateTab: 'meta',
      splitFraction: 0.34,
      densityMode: 'dense',
    },
  },
  {
    id: 'custom_review_focus',
    name: 'Review Focus',
    builtIn: false,
    snapshot: {
      workspace: 'workflow',
      workflow: 'analyze',
      workflowStep: 'analyze',
      controlsTab: 'opt',
      resultsTab: 'top',
      aggregateTab: 'meta',
      splitFraction: 0.41,
      densityMode: 'dense',
    },
  },
]

describe('searchPerspectives', () => {
  it('returns all perspectives for an empty query', () => {
    expect(searchPerspectives(PERSPECTIVES, '')).toEqual(PERSPECTIVES)
  })

  it('supports typo-tolerant fuzzy matches for longer queries', () => {
    const results = searchPerspectives(PERSPECTIVES, 'librari')
    expect(results[0]?.id).toBe('builtin_run_library')
  })

  it('uses deterministic prefix ranking for short queries', () => {
    const results = searchPerspectives(PERSPECTIVES, 're')
    expect(results[0]?.id).toBe('custom_review_focus')
  })

  it('matches metadata terms like density and workflow scope', () => {
    const denseResults = searchPerspectives(PERSPECTIVES, 'dense')
    expect(denseResults.map((entry) => entry.id)).toEqual([
      'builtin_run_library',
      'custom_review_focus',
    ])

    const workflowResults = searchPerspectives(PERSPECTIVES, 'history')
    expect(workflowResults[0]?.id).toBe('builtin_run_library')
  })
})
