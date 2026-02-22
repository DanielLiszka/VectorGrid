import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

function argValue(name, fallback = '') {
  const idx = process.argv.findIndex((arg) => arg === name)
  if (idx < 0) return fallback
  return process.argv[idx + 1] ?? fallback
}

function fail(message) {
  console.error(`ERROR: ${message}`)
  process.exit(1)
}

function normalizePath(root, filePath) {
  return path.resolve(root, String(filePath || '').trim())
}

function fileType(filePath) {
  return path.extname(filePath).toLowerCase()
}

function parseJson(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8')
  return JSON.parse(raw)
}

function typeOfJsonValue(value) {
  if (value === null) return 'null'
  if (Array.isArray(value)) return 'array'
  return typeof value
}

function compareJsonShape(baseline, candidate, atPath, out) {
  const bt = typeOfJsonValue(baseline)
  const ct = typeOfJsonValue(candidate)
  if (bt !== ct) {
    out.push(`${atPath}: type mismatch (${bt} vs ${ct})`)
    return
  }

  if (bt === 'object') {
    const bKeys = Object.keys(baseline)
    for (const key of bKeys) {
      if (!(key in candidate)) {
        out.push(`${atPath}.${key}: missing key in candidate`)
        continue
      }
      compareJsonShape(baseline[key], candidate[key], `${atPath}.${key}`, out)
    }
    return
  }

  if (bt === 'array') {
    const bArr = baseline
    const cArr = candidate
    if (bArr.length > 0 && cArr.length === 0) {
      out.push(`${atPath}: candidate array is empty while baseline has ${bArr.length} item(s)`)
      return
    }
    const bProbe = bArr.find((v) => v !== null && v !== undefined)
    const cProbe = cArr.find((v) => v !== null && v !== undefined)
    if (bProbe !== undefined && cProbe !== undefined) {
      compareJsonShape(bProbe, cProbe, `${atPath}[]`, out)
    } else if (bProbe !== undefined && cProbe === undefined) {
      out.push(`${atPath}: candidate lacks non-null sample item to compare`)
    }
  }
}

function parseCsvLine(line) {
  const out = []
  let cur = ''
  let inQuotes = false
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i]
    if (ch === '"') {
      if (inQuotes && line[i + 1] === '"') {
        cur += '"'
        i += 1
        continue
      }
      inQuotes = !inQuotes
      continue
    }
    if (ch === ',' && !inQuotes) {
      out.push(cur)
      cur = ''
      continue
    }
    cur += ch
  }
  out.push(cur)
  return out.map((v) => v.trim())
}

function compareCsvShape(baselinePath, candidatePath, out) {
  const bRaw = fs.readFileSync(baselinePath, 'utf8').replace(/\r\n/g, '\n')
  const cRaw = fs.readFileSync(candidatePath, 'utf8').replace(/\r\n/g, '\n')
  const bLines = bRaw.split('\n').filter((line) => line.length > 0)
  const cLines = cRaw.split('\n').filter((line) => line.length > 0)
  if (bLines.length === 0 || cLines.length === 0) {
    out.push('csv: one side is empty')
    return
  }
  const bHeader = parseCsvLine(bLines[0])
  const cHeader = parseCsvLine(cLines[0])
  if (JSON.stringify(bHeader) !== JSON.stringify(cHeader)) {
    out.push(`csv: header mismatch\n  baseline=${JSON.stringify(bHeader)}\n  candidate=${JSON.stringify(cHeader)}`)
  }

  const bDataSample = bLines.slice(1, 6).map(parseCsvLine)
  const cDataSample = cLines.slice(1, 6).map(parseCsvLine)
  const expectedCols = bHeader.length
  for (const [idx, row] of bDataSample.entries()) {
    if (row.length !== expectedCols) out.push(`csv: baseline row ${idx + 2} has ${row.length} cols, expected ${expectedCols}`)
  }
  for (const [idx, row] of cDataSample.entries()) {
    if (row.length !== expectedCols) out.push(`csv: candidate row ${idx + 2} has ${row.length} cols, expected ${expectedCols}`)
  }
}

function hasZipSignature(buffer) {
  if (buffer.length < 4) return false
  return buffer[0] === 0x50 && buffer[1] === 0x4b && (buffer[2] === 0x03 || buffer[2] === 0x05 || buffer[2] === 0x07) && (buffer[3] === 0x04 || buffer[3] === 0x06 || buffer[3] === 0x08)
}

function compareXlsxShape(baselinePath, candidatePath, out) {
  const bBuf = fs.readFileSync(baselinePath)
  const cBuf = fs.readFileSync(candidatePath)
  if (!hasZipSignature(bBuf)) out.push('xlsx: baseline is not a valid zip container')
  if (!hasZipSignature(cBuf)) out.push('xlsx: candidate is not a valid zip container')

  const bText = bBuf.toString('latin1')
  const cText = cBuf.toString('latin1')
  for (const marker of ['[Content_Types].xml', 'xl/workbook.xml', 'xl/worksheets/sheet1.xml']) {
    if (!bText.includes(marker)) out.push(`xlsx: baseline missing marker ${marker}`)
    if (!cText.includes(marker)) out.push(`xlsx: candidate missing marker ${marker}`)
  }

  if (bBuf.length > 0) {
    const ratio = cBuf.length / bBuf.length
    if (ratio < 0.2 || ratio > 5) {
      out.push(`xlsx: suspicious size ratio candidate/baseline = ${ratio.toFixed(3)}`)
    }
  }
}

function comparePair(rootDir, pair) {
  const name = String(pair.name || '').trim() || 'unnamed'
  const baselinePath = normalizePath(rootDir, pair.baseline)
  const candidatePath = normalizePath(rootDir, pair.candidate)
  if (!fs.existsSync(baselinePath)) return [`${name}: missing baseline file ${baselinePath}`]
  if (!fs.existsSync(candidatePath)) return [`${name}: missing candidate file ${candidatePath}`]

  const bType = fileType(baselinePath)
  const cType = fileType(candidatePath)
  if (bType !== cType) {
    return [`${name}: extension mismatch (${bType} vs ${cType})`]
  }

  const local = []
  if (bType === '.json') {
    const bJson = parseJson(baselinePath)
    const cJson = parseJson(candidatePath)
    compareJsonShape(bJson, cJson, '$', local)
  } else if (bType === '.csv') {
    compareCsvShape(baselinePath, candidatePath, local)
  } else if (bType === '.xlsx') {
    compareXlsxShape(baselinePath, candidatePath, local)
  } else {
    const bSize = fs.statSync(baselinePath).size
    const cSize = fs.statSync(candidatePath).size
    if (bSize === 0 || cSize === 0) local.push(`${name}: one side is empty`)
  }

  return local.map((msg) => `${name}: ${msg}`)
}

function main() {
  const rootDir = process.cwd()
  const pairsArg = argValue('--pairs', 'scripts/output_parity_pairs.json')
  const pairsPath = normalizePath(rootDir, pairsArg)
  if (!fs.existsSync(pairsPath)) fail(`Pairs file not found: ${pairsPath}`)

  const config = JSON.parse(fs.readFileSync(pairsPath, 'utf8'))
  const pairsBaseDir = normalizePath(path.dirname(pairsPath), String(config.base_dir || '.'))
  const pairs = Array.isArray(config.pairs) ? config.pairs : []
  if (pairs.length === 0) fail(`No parity pairs found in ${pairsPath}`)

  const failures = []
  for (const pair of pairs) {
    failures.push(...comparePair(pairsBaseDir, pair))
  }

  if (failures.length > 0) {
    console.error('Output parity check failed:')
    for (const item of failures) console.error(`  - ${item}`)
    process.exit(1)
  }

  console.log(`Output parity check passed for ${pairs.length} pair(s).`)
}

main()
