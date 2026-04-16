import fs from 'node:fs'
import path from 'node:path'

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function collectBenchmarks(report) {
  if (Array.isArray(report.cases)) {
    return report.cases.map((row) => ({
      name: row.name,
      mean: row.meanMs,
    }))
  }
  const rows = []
  for (const file of report.files ?? []) {
    for (const group of file.groups ?? []) {
      for (const bench of group.benchmarks ?? []) {
        rows.push(bench)
      }
    }
  }
  return rows
}

function formatMs(value) {
  if (!Number.isFinite(value)) return 'n/a'
  return `${value.toFixed(6)} ms`
}

function formatPct(value) {
  if (!Number.isFinite(value)) return 'n/a'
  return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`
}

const [, , baselineArg, candidateArg] = process.argv
const baselinePath = path.resolve(process.cwd(), baselineArg ?? '../output/benchmarks/request-builder-bench-baseline.json')
const candidatePath = path.resolve(process.cwd(), candidateArg ?? '../output/benchmarks/request-builder-bench.json')

if (!fs.existsSync(baselinePath)) {
  throw new Error(`Missing baseline benchmark file: ${baselinePath}`)
}
if (!fs.existsSync(candidatePath)) {
  throw new Error(`Missing candidate benchmark file: ${candidatePath}`)
}

const baseline = collectBenchmarks(readJson(baselinePath))
const candidate = collectBenchmarks(readJson(candidatePath))
const baselineByName = new Map(baseline.map((bench) => [bench.name, bench]))

const lines = []
lines.push(`Baseline: ${baselinePath}`)
lines.push(`Candidate: ${candidatePath}`)
lines.push('')

for (const bench of candidate) {
  const prev = baselineByName.get(bench.name)
  const currentMean = Number(bench.mean)
  const previousMean = Number(prev?.mean)
  const deltaPct =
    Number.isFinite(currentMean) && Number.isFinite(previousMean) && previousMean > 0
      ? ((currentMean - previousMean) / previousMean) * 100
      : Number.NaN
  lines.push(
    `${bench.name}: candidate=${formatMs(currentMean)} baseline=${formatMs(previousMean)} delta=${formatPct(deltaPct)}`,
  )
}

console.log(lines.join('\n'))
