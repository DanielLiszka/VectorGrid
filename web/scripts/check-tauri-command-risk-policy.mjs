import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const ALLOWED_RISK_CLASSES = new Set([
  'broker_action_capable',
  'credential_secret_handler',
  'data_exfiltration_or_path_capable',
  'network_data_acquisition',
  'artifact_report_integrity_critical',
  'scientific_compute',
  'read_only_diagnostic',
  'shell_window_process_control',
])

function readText(filePath) {
  return fs.readFileSync(filePath, 'utf8')
}

function uniqueSorted(items) {
  return Array.from(new Set(items)).sort()
}

function diff(left, right) {
  const r = new Set(right)
  return left.filter((x) => !r.has(x))
}

function resolveRoots() {
  const cwd = process.cwd()
  if (fs.existsSync(path.join(cwd, 'src-tauri', 'src', 'lib.rs'))) {
    return { repoRoot: cwd, webRoot: path.join(cwd, 'web') }
  }
  return { repoRoot: path.resolve(cwd, '..'), webRoot: cwd }
}

function parseHandlerCommands(libRsText) {
  const m = libRsText.match(/generate_handler!\[(?<body>[\s\S]*?)\]/m)
  if (!m?.groups?.body) {
    throw new Error('Could not find tauri::generate_handler![...] command registry.')
  }
  const body = m.groups.body
  const all = []
  const re = /\b([a-z_][a-z0-9_]*)\b/g
  for (const mm of body.matchAll(re)) {
    const name = mm[1]
    if (name === 'tauri' || name === 'generate_handler') continue
    all.push(name)
  }
  const duplicates = uniqueSorted(
    all.filter((name, index) => all.indexOf(name) !== index),
  )
  return { all, commands: uniqueSorted(all), duplicates }
}

function parseTopLevelCommandKeys(policyText) {
  const commandsIdx = policyText.search(/"commands"\s*:/)
  if (commandsIdx < 0) return []
  const firstBrace = policyText.indexOf('{', commandsIdx)
  if (firstBrace < 0) return []

  const keys = []
  let depth = 0
  let inString = false
  let escape = false
  for (let i = firstBrace; i < policyText.length; i += 1) {
    const ch = policyText[i]
    if (inString) {
      if (escape) {
        escape = false
      } else if (ch === '\\') {
        escape = true
      } else if (ch === '"') {
        inString = false
      }
      continue
    }
    if (ch === '"') {
      if (depth === 1) {
        let j = i + 1
        let key = ''
        let keyEscape = false
        for (; j < policyText.length; j += 1) {
          const keyCh = policyText[j]
          if (keyEscape) {
            key += keyCh
            keyEscape = false
          } else if (keyCh === '\\') {
            keyEscape = true
          } else if (keyCh === '"') {
            break
          } else {
            key += keyCh
          }
        }
        let k = j + 1
        while (/\s/.test(policyText[k] ?? '')) k += 1
        if (policyText[k] === ':') keys.push(key)
        i = j
      } else {
        inString = true
      }
      continue
    }
    if (ch === '{') depth += 1
    if (ch === '}') {
      depth -= 1
      if (depth === 0) break
    }
  }
  return keys
}

function checkContains(checks, patterns) {
  const joined = checks.join(' ').toLowerCase()
  return patterns.some((pattern) => pattern.test(joined))
}

function validatePolicyEntry(command, entry) {
  const errors = []
  if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
    return [`${command}: policy entry must be an object.`]
  }
  if (!ALLOWED_RISK_CLASSES.has(entry.riskClass)) {
    errors.push(`${command}: unknown riskClass ${JSON.stringify(entry.riskClass)}.`)
  }
  if (
    !Array.isArray(entry.requiresBackendChecks) ||
    entry.requiresBackendChecks.length === 0 ||
    entry.requiresBackendChecks.some((v) => typeof v !== 'string' || v.trim() === '')
  ) {
    errors.push(`${command}: requiresBackendChecks must be a non-empty string array.`)
    return errors
  }

  const checks = entry.requiresBackendChecks
  if (
    entry.riskClass === 'broker_action_capable' &&
    !checkContains(checks, [/broker/, /execution/, /risk/, /runner/, /paper/, /halt/])
  ) {
    errors.push(`${command}: broker_action_capable entry must name broker/execution preconditions.`)
  }
  if (
    entry.riskClass === 'credential_secret_handler' &&
    !checkContains(checks, [/credential/, /keyring/, /secret/, /non_secret/, /no_secret/])
  ) {
    errors.push(`${command}: credential_secret_handler entry must name credential or non-secret handling.`)
  }
  if (
    entry.riskClass === 'credential_secret_handler' &&
    !checkContains(checks, [/redact/, /non_secret/, /no_secret/])
  ) {
    errors.push(`${command}: credential_secret_handler entry must name redaction or non-secret response behavior.`)
  }
  if (
    entry.riskClass === 'network_data_acquisition' &&
    !checkContains(checks, [/provider/, /network/, /stream/, /redact/])
  ) {
    errors.push(`${command}: network_data_acquisition entry must name provider/network handling.`)
  }
  if (
    entry.riskClass === 'data_exfiltration_or_path_capable' &&
    !checkContains(checks, [/path/, /dataset/, /scope/, /picker/])
  ) {
    errors.push(`${command}: data/path entry must name path, dataset, scope, or picker handling.`)
  }
  if (
    entry.riskClass === 'artifact_report_integrity_critical' &&
    !checkContains(checks, [/artifact/, /report/, /replay/, /provenance/, /root/])
  ) {
    errors.push(`${command}: artifact/report entry must name artifact, replay, provenance, or root handling.`)
  }
  if (
    entry.riskClass === 'shell_window_process_control' &&
    !checkContains(checks, [/shell/, /window/, /command/])
  ) {
    errors.push(`${command}: shell/window entry must name shell, window, or command handling.`)
  }
  if (
    entry.riskClass === 'shell_window_process_control' &&
    /(open|reveal|path)/.test(command) &&
    !checkContains(checks, [/path/, /contain/])
  ) {
    errors.push(`${command}: shell open/reveal entry must name path containment behavior.`)
  }
  return errors
}

function summarizeHighRisk(command, entry) {
  if (entry.riskClass === 'read_only_diagnostic' || entry.riskClass === 'scientific_compute') {
    return null
  }
  return {
    command,
    riskClass: entry.riskClass,
    requiresBackendChecks: entry.requiresBackendChecks,
    notes: entry.notes ?? '',
  }
}

function writeReport(repoRoot, report) {
  const outDir = path.join(repoRoot, 'output', 'qa')
  fs.mkdirSync(outDir, { recursive: true })
  fs.writeFileSync(
    path.join(outDir, 'tauri-command-risk-policy.json'),
    `${JSON.stringify(report, null, 2)}\n`,
  )
}

function main() {
  const { repoRoot } = resolveRoots()
  const libRsPath = path.join(repoRoot, 'src-tauri', 'src', 'lib.rs')
  const policyPath = path.join(repoRoot, 'src-tauri', 'command-risk-policy.json')

  const { commands: backendCommands, duplicates: backendDuplicates } =
    parseHandlerCommands(readText(libRsPath))
  const policyText = readText(policyPath)
  const policy = JSON.parse(policyText)
  const rawPolicyCommandKeys = parseTopLevelCommandKeys(policyText)
  const policyDuplicateKeys = uniqueSorted(
    rawPolicyCommandKeys.filter((name, index) => rawPolicyCommandKeys.indexOf(name) !== index),
  )

  const policyCommandsObject = policy.commands
  const policyCommands =
    policyCommandsObject && typeof policyCommandsObject === 'object' && !Array.isArray(policyCommandsObject)
      ? Object.keys(policyCommandsObject).sort()
      : []

  const errors = []
  if (policy.schemaVersion !== 1) errors.push('schemaVersion must be 1.')
  if (policyCommands.length === 0) errors.push('commands must be a non-empty object.')
  for (const duplicate of backendDuplicates) {
    errors.push(`Backend command registry contains duplicate command: ${duplicate}.`)
  }
  for (const duplicate of policyDuplicateKeys) {
    errors.push(`Policy file contains duplicate command key: ${duplicate}.`)
  }

  const missing = diff(backendCommands, policyCommands)
  const unknown = diff(policyCommands, backendCommands)
  for (const command of missing) errors.push(`Missing risk policy for command: ${command}.`)
  for (const command of unknown) errors.push(`Unknown command in risk policy: ${command}.`)
  for (const command of policyCommands) {
    errors.push(...validatePolicyEntry(command, policyCommandsObject[command]))
  }

  const riskClassCounts = {}
  const highRiskCommands = []
  for (const command of policyCommands) {
    const entry = policyCommandsObject[command]
    if (!entry?.riskClass) continue
    riskClassCounts[entry.riskClass] = (riskClassCounts[entry.riskClass] ?? 0) + 1
    const highRisk = summarizeHighRisk(command, entry)
    if (highRisk) highRiskCommands.push(highRisk)
  }

  const report = {
    schemaVersion: 1,
    generatedAtUtc: new Date().toISOString(),
    status: errors.length === 0 ? 'passed' : 'failed',
    parsedCommandCount: backendCommands.length,
    policyCommandCount: policyCommands.length,
    riskClassCounts,
    missing,
    unknown,
    duplicateBackendCommands: backendDuplicates,
    duplicatePolicyCommands: policyDuplicateKeys,
    highRiskCommands,
    errors,
  }
  writeReport(repoRoot, report)

  console.log(
    `Tauri command risk policy: ${backendCommands.length} registered commands, ${policyCommands.length} policy rows.`,
  )
  for (const [riskClass, count] of Object.entries(riskClassCounts).sort()) {
    console.log(`  ${riskClass}: ${count}`)
  }

  if (errors.length > 0) {
    console.error('ERROR: Tauri command risk policy check failed:')
    for (const error of errors) console.error(`  - ${error}`)
    process.exit(1)
  }
}

main()
