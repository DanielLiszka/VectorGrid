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

function parseCsvRecords(text) {
  const records = []
  let record = []
  let field = ''
  let inQuotes = false

  for (let idx = 0; idx < text.length; idx += 1) {
    const ch = text[idx]
    if (ch === '"') {
      if (inQuotes && text[idx + 1] === '"') {
        field += '"'
        idx += 1
        continue
      }
      inQuotes = !inQuotes
      continue
    }
    if (ch === ',' && !inQuotes) {
      record.push(field)
      field = ''
      continue
    }
    if ((ch === '\n' || ch === '\r') && !inQuotes) {
      if (ch === '\r' && text[idx + 1] === '\n') idx += 1
      record.push(field)
      if (record.some((value) => value.length > 0)) records.push(record)
      record = []
      field = ''
      continue
    }
    field += ch
  }

  if (inQuotes) throw new Error('unterminated quoted CSV field')
  if (field.length > 0 || record.length > 0) {
    record.push(field)
    if (record.some((value) => value.length > 0)) records.push(record)
  }
  return records
}

function compareCsvShape(baselinePath, candidatePath, out) {
  const bRecords = parseCsvRecords(fs.readFileSync(baselinePath, 'utf8'))
  const cRecords = parseCsvRecords(fs.readFileSync(candidatePath, 'utf8'))
  if (bRecords.length === 0 || cRecords.length === 0) {
    out.push('csv: one side is empty')
    return
  }
  const bHeader = bRecords[0]
  const cHeader = cRecords[0]
  if (JSON.stringify(bHeader) !== JSON.stringify(cHeader)) {
    out.push(`csv: header mismatch\n  baseline=${JSON.stringify(bHeader)}\n  candidate=${JSON.stringify(cHeader)}`)
  }

  const expectedCols = bHeader.length
  for (const [idx, row] of bRecords.slice(1).entries()) {
    if (row.length !== expectedCols) out.push(`csv: baseline row ${idx + 2} has ${row.length} cols, expected ${expectedCols}`)
  }
  for (const [idx, row] of cRecords.slice(1).entries()) {
    if (row.length !== expectedCols) out.push(`csv: candidate row ${idx + 2} has ${row.length} cols, expected ${expectedCols}`)
  }
}

function compareCsvContract(candidatePath, pair) {
  const out = []
  let records
  try {
    records = parseCsvRecords(fs.readFileSync(candidatePath, 'utf8'))
  } catch (error) {
    return [`csv: ${error.message}`]
  }
  if (records.length === 0) return ['csv: output is empty']

  const expectedHeader = Array.isArray(pair.expected_header)
    ? pair.expected_header.map((value) => String(value))
    : []
  if (expectedHeader.length === 0) return ['csv: expected_header contract is empty']
  if (JSON.stringify(records[0]) !== JSON.stringify(expectedHeader)) {
    out.push(
      `csv: header contract mismatch\n  expected=${JSON.stringify(expectedHeader)}\n  candidate=${JSON.stringify(records[0])}`,
    )
  }

  records.slice(1).forEach((row, idx) => {
    if (row.length !== expectedHeader.length) {
      out.push(`csv: candidate row ${idx + 2} has ${row.length} cols, expected ${expectedHeader.length}`)
    }
  })
  const minimumDataRows = Number(pair.minimum_data_rows ?? 0)
  if (!Number.isInteger(minimumDataRows) || minimumDataRows < 0) {
    out.push('csv: minimum_data_rows must be a non-negative integer')
  } else if (records.length - 1 < minimumDataRows) {
    out.push(`csv: expected at least ${minimumDataRows} data row(s), found ${records.length - 1}`)
  }
  return out
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
  const candidatePath = normalizePath(rootDir, pair.candidate)
  if (!fs.existsSync(candidatePath)) return [`${name}: missing candidate file ${candidatePath}`]

  const kind = String(pair.kind || 'baseline').trim()
  if (kind === 'csv_contract') {
    return compareCsvContract(candidatePath, pair).map((msg) => `${name}: ${msg}`)
  }
  if (kind !== 'baseline') return [`${name}: unsupported parity kind ${JSON.stringify(kind)}`]

  const baselinePath = normalizePath(rootDir, pair.baseline)
  if (!fs.existsSync(baselinePath)) return [`${name}: missing baseline file ${baselinePath}`]

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
