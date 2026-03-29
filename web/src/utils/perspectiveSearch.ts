import Fuse from 'fuse.js'
import type { IFuseOptions } from 'fuse.js'
import type { WorkstationPerspective } from '../stores/settingsStore'

interface PerspectiveSearchRecord {
  entry: WorkstationPerspective
  index: number
  name: string
  builtInLabel: string
  scopeLabels: string[]
  searchText: string
}

const PERSPECTIVE_FUSE_OPTIONS: IFuseOptions<PerspectiveSearchRecord> = {
  includeScore: true,
  ignoreLocation: true,
  shouldSort: true,
  threshold: 0.34,
  minMatchCharLength: 2,
  keys: [
    { name: 'name', weight: 0.44 },
    { name: 'scopeLabels', weight: 0.34 },
    { name: 'builtInLabel', weight: 0.14 },
    { name: 'searchText', weight: 0.08 },
  ],
}

function normalizeText(value: unknown): string {
  return String(value ?? '').trim()
}

function normalizeSearchText(value: unknown): string {
  return normalizeText(value).toLowerCase()
}

function filterTokens(value: string): string[] {
  const query = normalizeSearchText(value)
  if (!query) return []
  return query
    .split(/\s+/g)
    .map((token) => normalizeText(token).toLowerCase())
    .filter((token) => token.length > 0)
}

function buildScopeLabels(entry: WorkstationPerspective): string[] {
  const snapshot = entry.snapshot
  return [
    normalizeSearchText(snapshot.workspace),
    normalizeSearchText(snapshot.workflow),
    normalizeSearchText(snapshot.workflowStep),
    normalizeSearchText(snapshot.controlsTab),
    normalizeSearchText(snapshot.resultsTab),
    normalizeSearchText(snapshot.aggregateTab),
    normalizeSearchText(snapshot.densityMode),
    entry.builtIn ? 'built-in' : 'custom',
  ].filter((value) => value.length > 0)
}

function shortQueryMatches(records: PerspectiveSearchRecord[], tokens: string[]): WorkstationPerspective[] {
  const scored = records
    .map((record) => {
      const fields = [
        record.name,
        ...record.scopeLabels,
        record.builtInLabel,
      ]
      let totalScore = 0
      for (const token of tokens) {
        let tokenScore = -1
        for (const field of fields) {
          if (!field) continue
          if (field === token) tokenScore = Math.max(tokenScore, 10)
          else if (field.startsWith(token)) tokenScore = Math.max(tokenScore, 7)
          else if (field.includes(token)) tokenScore = Math.max(tokenScore, 4)
        }
        if (tokenScore < 0) return null
        totalScore += tokenScore
      }
      return { record, totalScore }
    })
    .filter((row): row is { record: PerspectiveSearchRecord; totalScore: number } => row !== null)
    .sort((left, right) => {
      if (right.totalScore !== left.totalScore) return right.totalScore - left.totalScore
      return left.record.index - right.record.index
    })
  return scored.map((row) => row.record.entry)
}

export function searchPerspectives(
  entries: WorkstationPerspective[],
  query: string,
): WorkstationPerspective[] {
  const tokens = filterTokens(query)
  if (!tokens.length) return entries

  const records: PerspectiveSearchRecord[] = entries.map((entry, index) => {
    const name = normalizeSearchText(entry.name)
    const builtInLabel = entry.builtIn ? 'built-in' : 'custom'
    const scopeLabels = buildScopeLabels(entry)
    const searchText = [name, builtInLabel, ...scopeLabels].join(' ')
    return {
      entry,
      index,
      name,
      builtInLabel,
      scopeLabels,
      searchText,
    }
  })

  if (tokens.every((token) => token.length < 3)) {
    return shortQueryMatches(records, tokens)
  }

  const fuse = new Fuse(records, PERSPECTIVE_FUSE_OPTIONS)
  const normalizedQuery = normalizeSearchText(query)
  const results = fuse.search(normalizedQuery)
  if (!results.length) return shortQueryMatches(records, tokens)

  const strictTokenMatches = results.filter(({ item }) =>
    tokens.length <= 1 || tokens.every((token) => item.searchText.includes(token)),
  )
  const ranked = strictTokenMatches.length > 0 ? strictTokenMatches : results
  return ranked.map(({ item }) => item.entry)
}
