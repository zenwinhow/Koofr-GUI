import { existsSync, rmSync } from 'node:fs'
import { dirname, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')

const targets = {
  frontend: ['dist', 'node_modules/.tmp', 'node_modules/.vite'],
  rust: ['src-tauri/target', 'src-tauri/gen'],
}

const modes = {
  frontend: targets.frontend,
  rust: targets.rust,
  build: [...targets.frontend, ...targets.rust],
  all: [...targets.frontend, ...targets.rust, 'node_modules'],
}

const mode = process.argv[2] ?? 'build'
const selectedTargets = modes[mode]
const dryRun = process.argv.includes('--dry-run')

if (!selectedTargets) {
  console.error(`Unknown clean mode: ${mode}`)
  console.error(`Available modes: ${Object.keys(modes).join(', ')}`)
  process.exitCode = 1
} else {
  let removed = 0

  for (const target of [...new Set(selectedTargets)]) {
    const absoluteTarget = resolve(projectRoot, target)
    const pathFromRoot = relative(projectRoot, absoluteTarget)

    if (pathFromRoot.startsWith('..') || pathFromRoot === '') {
      throw new Error(`Refusing to remove a path outside the project: ${target}`)
    }

    if (!existsSync(absoluteTarget)) {
      console.log(`skip    ${target}`)
      continue
    }

    if (dryRun) {
      console.log(`would remove ${target}`)
      continue
    }

    rmSync(absoluteTarget, {
      recursive: true,
      force: true,
      maxRetries: 3,
      retryDelay: 200,
    })
    removed += 1
    console.log(`removed ${target}`)
  }

  if (dryRun) {
    console.log('Dry run complete (nothing removed).')
  } else {
    console.log(`Clean complete (${removed} path${removed === 1 ? '' : 's'} removed).`)
  }
}
