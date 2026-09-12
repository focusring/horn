// Fetch the @focusring/horn-wasm build that matches this docs version into
// public/wasm, so the live demo always runs the engine of the documented release.
//
//   pnpm sync-wasm                      # download focusring-horn-wasm-<version>.tgz from GitHub Releases
//   HORN_WASM_DIR=../horn-wasm/pkg ...  # use a local `wasm-pack build --target web` output instead
//
// The download is skipped when public/wasm/VERSION already matches. If the
// release for this exact version does not exist yet (e.g. a docs preview built
// before the tag is pushed) the script falls back to the latest release and says so.

import { createRequire } from 'node:module'
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const require = createRequire(import.meta.url)
const { version } = require('../package.json')
const docsDir = dirname(dirname(fileURLToPath(import.meta.url)))
const outDir = join(docsDir, 'public', 'wasm')
const versionFile = join(outDir, 'VERSION')
const FILES = ['horn_wasm.js', 'horn_wasm_bg.wasm', 'horn_wasm.d.ts']
const REPO = 'focusring/horn'

function install(fromDir, label) {
  mkdirSync(outDir, { recursive: true })
  for (const f of FILES) copyFileSync(join(fromDir, f), join(outDir, f))
  writeFileSync(versionFile, `${label}\n`)
  console.log(`[sync-wasm] installed horn-wasm ${label} into public/wasm`)
}

if (process.env.HORN_WASM_DIR) {
  install(process.env.HORN_WASM_DIR, `local (${process.env.HORN_WASM_DIR})`)
  process.exit(0)
}

const upToDate =
  existsSync(versionFile) &&
  readFileSync(versionFile, 'utf8').trim() === version &&
  FILES.every((f) => existsSync(join(outDir, f)))
if (upToDate) {
  console.log(`[sync-wasm] public/wasm already has horn-wasm ${version}`)
  process.exit(0)
}

async function download(tag) {
  const v = tag.replace(/^v/, '')
  const url = `https://github.com/${REPO}/releases/download/${tag}/focusring-horn-wasm-${v}.tgz`
  const res = await fetch(url, { redirect: 'follow' })
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${url}`)
  const work = mkdtempSync(join(tmpdir(), 'horn-wasm-'))
  const tgz = join(work, 'pkg.tgz')
  writeFileSync(tgz, Buffer.from(await res.arrayBuffer()))
  execFileSync('tar', ['-xzf', tgz, '-C', work])
  install(join(work, 'package'), v)
  rmSync(work, { recursive: true, force: true })
}

try {
  await download(`v${version}`)
} catch (e) {
  console.warn(`[sync-wasm] no release asset for v${version} (${e.message})`)
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { accept: 'application/vnd.github+json' },
    })
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`)
    const { tag_name } = await res.json()
    console.warn(`[sync-wasm] falling back to the latest release, ${tag_name}`)
    await download(tag_name)
  } catch (e2) {
    if (FILES.every((f) => existsSync(join(outDir, f)))) {
      console.warn(`[sync-wasm] keeping the existing public/wasm files (${e2.message})`)
    } else {
      console.error(`[sync-wasm] could not obtain a horn-wasm build: ${e2.message}`)
      process.exit(1)
    }
  }
}
