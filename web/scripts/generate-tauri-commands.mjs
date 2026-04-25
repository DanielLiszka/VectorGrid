import fs from 'node:fs'
import path from 'node:path'
import process from 'node:process'

const CHECK_MODE = process.argv.includes('--check')

function parseHandlerCommands(libRsText) {
  const m = libRsText.match(/generate_handler!\[(?<body>[\s\S]*?)\]/m)
  if (!m?.groups?.body) return []

  const out = []
  const seen = new Set()
  const parts = m.groups.body
    .split(',')
    .map((part) => part.replace(/\/\/.*$/g, '').trim())
    .filter(Boolean)

  for (const name of parts) {
    if (!/^[a-z_][a-z0-9_]*$/.test(name)) continue
    if (seen.has(name)) continue
    seen.add(name)
    out.push(name)
  }
  return out
}

function renderCommandsTs(commands) {
  const lines = commands.map((cmd) => `  '${cmd}',`)
  return [
    'export const TAURI_COMMANDS = [',
    ...lines,
    '] as const',
    '',
    'const TAURI_COMMAND_SET = new Set<string>(TAURI_COMMANDS)',
    '',
    'export type TauriCommand = (typeof TAURI_COMMANDS)[number]',
    '',
    'export function isTauriCommand(value: string): value is TauriCommand {',
    '  return TAURI_COMMAND_SET.has(value)',
    '}',
    '',
  ].join('\n')
}

function main() {
  const webRoot = process.cwd()
  const repoRoot = path.resolve(webRoot, '..')
  const libRsPath = path.join(repoRoot, 'src-tauri', 'src', 'lib.rs')
  const outPath = path.join(webRoot, 'src', 'tauri', 'commands.ts')

  const libRsText = fs.readFileSync(libRsPath, 'utf8')
  const commands = parseHandlerCommands(libRsText)
  if (commands.length === 0) {
    throw new Error('No commands parsed from src-tauri/src/lib.rs generate_handler! block.')
  }

  const rendered = renderCommandsTs(commands)

  if (CHECK_MODE) {
    const existing = fs.readFileSync(outPath, 'utf8')
    if (existing !== rendered) {
      console.error('ERROR: src/tauri/commands.ts is stale. Run: npm run gen:commands')
      process.exit(1)
    }
    console.log(`OK: src/tauri/commands.ts is up to date (${commands.length} commands).`)
    return
  }

  fs.writeFileSync(outPath, rendered, 'utf8')
  console.log(`Wrote src/tauri/commands.ts (${commands.length} commands).`)
}

main()
