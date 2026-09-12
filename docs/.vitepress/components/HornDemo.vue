<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import pkg from '../../package.json'

// The demo always runs the newest released engine: the version is resolved
// from the npm registry at page load and the files come from jsDelivr, so a
// new release is picked up without rebuilding the docs. The build bundled by
// docs/scripts/sync-wasm.mjs (public/wasm) is the offline fallback.
const NPM_PACKAGE = '@focusring/horn-wasm'
const CDN = `https://cdn.jsdelivr.net/npm/${NPM_PACKAGE}`

const engineVersion = ref(pkg.version)
const engineSource = ref<'npm' | 'bundled'>('bundled')

interface CheckResult {
  rule_id: string
  checkpoint: number
  description: string
  severity: 'error' | 'warning' | 'info'
  outcome:
    | { status: 'Pass' }
    | { status: 'Fail'; message: string; location?: { page?: number; element?: string } }
    | { status: 'NeedsReview'; reason: string }
    | { status: 'NotApplicable' }
}

interface FileReport {
  path: string
  standard: string
  results: CheckResult[]
  error: string | null
}

const wasmReady = ref(false)
const loading = ref(false)
const dragging = ref(false)
const reports = ref<FileReport[]>([])
const error = ref<string | null>(null)
const processingTime = ref<number | null>(null)
const liveAnnouncement = ref('')

let validateFn: (name: string, data: Uint8Array) => FileReport

interface EngineSource {
  source: 'npm' | 'bundled'
  version: string
  js: string
  wasm: string
}

async function latestNpmVersion(): Promise<string | null> {
  try {
    const res = await fetch(`https://registry.npmjs.org/${NPM_PACKAGE}/latest`, { cache: 'no-store' })
    if (!res.ok) return null
    const { version } = await res.json()
    return typeof version === 'string' && /^\d+\.\d+\.\d+/.test(version) ? version : null
  } catch {
    return null
  }
}

async function bundledVersion(base: string): Promise<string> {
  // Written by docs/scripts/sync-wasm.mjs next to the files it installed, so the
  // label always names the build that is actually served, even when the script
  // fell back to another release or to a local wasm-pack output.
  try {
    const res = await fetch(`${base}wasm/VERSION`, { cache: 'no-store' })
    const text = res.ok ? (await res.text()).trim() : ''
    return text || pkg.version
  } catch {
    return pkg.version
  }
}

async function loadEngine(src: EngineSource) {
  // Fetch the JS glue code as text and load it as a blob URL module.
  // Files in /public cannot be imported directly by Vite, and the CDN copy
  // must not go through the dev server's transform pipeline either.
  const res = await fetch(src.js)
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${src.js}`)
  const blob = new Blob([await res.text()], { type: 'text/javascript' })
  const blobUrl = URL.createObjectURL(blob)
  try {
    const mod = await import(/* @vite-ignore */ blobUrl)
    await mod.default({ module_or_path: src.wasm })
    return mod.validate as typeof validateFn
  } finally {
    URL.revokeObjectURL(blobUrl)
  }
}

onMounted(async () => {
  const base = import.meta.env.BASE_URL || '/'
  const candidates: EngineSource[] = []

  const latest = await latestNpmVersion()
  if (latest) {
    candidates.push({
      source: 'npm',
      version: latest,
      js: `${CDN}@${latest}/horn_wasm.js`,
      wasm: `${CDN}@${latest}/horn_wasm_bg.wasm`,
    })
  }
  candidates.push({
    source: 'bundled',
    version: await bundledVersion(base),
    js: `${base}wasm/horn_wasm.js`,
    wasm: `${base}wasm/horn_wasm_bg.wasm`,
  })

  const failures: string[] = []
  for (const candidate of candidates) {
    try {
      validateFn = await loadEngine(candidate)
      engineVersion.value = candidate.version
      engineSource.value = candidate.source
      wasmReady.value = true
      return
    } catch (e) {
      failures.push(`${candidate.source} ${candidate.version}: ${e}`)
    }
  }
  error.value = `Failed to load the Horn WASM engine (${failures.join('; ')})`
})

function handleFiles(files: FileList | File[]) {
  if (!validateFn || files.length === 0) return

  loading.value = true
  error.value = null
  reports.value = []

  requestAnimationFrame(() => {
    try {
      const start = performance.now()

      const promises = Array.from(files).map(
        (file) =>
          new Promise<FileReport>((resolve, reject) => {
            const reader = new FileReader()
            reader.onload = () => {
              try {
                const bytes = new Uint8Array(reader.result as ArrayBuffer)
                const report = validateFn(file.name, bytes)
                resolve(report)
              } catch (e) {
                resolve({
                  path: file.name,
                  standard: 'unknown',
                  results: [],
                  error: String(e),
                })
              }
            }
            reader.onerror = () => reject(reader.error)
            reader.readAsArrayBuffer(file)
          }),
      )

      Promise.all(promises).then((results) => {
        processingTime.value = Math.round(performance.now() - start)
        reports.value = results
        loading.value = false

        const errors = results.reduce((sum, r) => sum + countBySeverity(r.results, 'error'), 0)
        const warnings = results.reduce((sum, r) => sum + countBySeverity(r.results, 'warning'), 0)
        const parts = [`Validated ${results.length} file${results.length !== 1 ? 's' : ''} in ${processingTime.value}ms.`]
        if (errors > 0) parts.push(`${errors} error${errors !== 1 ? 's' : ''}`)
        if (warnings > 0) parts.push(`${warnings} warning${warnings !== 1 ? 's' : ''}`)
        const review = results.reduce((sum, r) => sum + countNeedsReview(r.results), 0)
        if (errors === 0 && warnings === 0) parts.push('No automated checks failed')
        if (review > 0) parts.push(`${review} condition${review !== 1 ? 's' : ''} need${review === 1 ? 's' : ''} manual review`)

        liveAnnouncement.value = ''
        requestAnimationFrame(() => {
          liveAnnouncement.value = parts.join('. ')
        })
      })
    } catch (e) {
      error.value = String(e)
      loading.value = false
    }
  })
}

function onFileInput(event: Event) {
  const input = event.target as HTMLInputElement
  if (input.files) handleFiles(input.files)
}

function onDrop(event: DragEvent) {
  dragging.value = false
  const files = Array.from(event.dataTransfer?.files ?? []).filter((f) =>
    f.name.toLowerCase().endsWith('.pdf'),
  )
  if (files.length) handleFiles(files)
}

function severityIcon(severity: string) {
  switch (severity) {
    case 'error':
      return '\u274C'
    case 'warning':
      return '\u26A0\uFE0F'
    case 'info':
      return '\u2139\uFE0F'
    default:
      return ''
  }
}

function standardLabel(standard: string) {
  switch (standard) {
    case 'ua1':
      return 'PDF/UA-1'
    case 'ua2':
      return 'PDF/UA-2'
    default:
      return standard || 'Unknown standard'
  }
}

function clearResults() {
  reports.value = []
  processingTime.value = null
  error.value = null
}

function countBySeverity(results: CheckResult[], severity: string) {
  return results.filter(
    (r) => r.severity === severity && r.outcome.status === 'Fail',
  ).length
}

function countNeedsReview(results: CheckResult[]) {
  return results.filter((r) => r.outcome.status === 'NeedsReview').length
}

function failures(results: CheckResult[]) {
  return results.filter((r) => r.outcome.status === 'Fail')
}

function reviewItems(results: CheckResult[]) {
  return results.filter((r) => r.outcome.status === 'NeedsReview')
}

const totalErrors = computed(() =>
  reports.value.reduce((sum, r) => sum + countBySeverity(r.results, 'error'), 0),
)
const totalWarnings = computed(() =>
  reports.value.reduce((sum, r) => sum + countBySeverity(r.results, 'warning'), 0),
)
const totalReview = computed(() =>
  reports.value.reduce((sum, r) => sum + countNeedsReview(r.results), 0),
)
</script>

<template>
  <div class="horn-demo">
    <div role="alert" class="visually-hidden">{{ liveAnnouncement }}</div>

    <div v-if="!wasmReady && !error" class="loading-wasm">
      Loading Horn WASM module...
    </div>

    <div v-if="error && !wasmReady" class="error-box" role="alert">
      {{ error }}
    </div>

    <template v-if="wasmReady">
      <div
        class="drop-zone"
        :class="{ dragging, disabled: loading }"
        @dragover.prevent="dragging = true"
        @dragleave="dragging = false"
        @drop.prevent="onDrop"
      >
        <div class="drop-content">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
          <p class="drop-text">Drag & drop PDF files here</p>
          <p class="drop-or">or</p>
          <label class="file-btn">
            Choose files
            <input
              type="file"
              accept=".pdf"
              multiple
              :disabled="loading"
              aria-describedby="drop-note"
              @change="onFileInput"
            />
          </label>
          <p id="drop-note" class="drop-note">Files are validated locally in your browser. Nothing is uploaded.</p>
          <p class="drop-version">
            Horn v{{ engineVersion }} · WebAssembly build · PDF/UA-1 (Matterhorn Protocol 1.1) and PDF/UA-2
            <template v-if="engineSource === 'npm'"> · latest release from npm</template>
            <template v-else> · build bundled with this site</template>
          </p>
        </div>
      </div>

      <div v-if="loading" class="status-bar">
        Validating...
      </div>

      <div v-if="reports.length > 0" class="results">
        <div class="results-summary">
          <span v-if="processingTime !== null" class="time">
            Validated {{ reports.length }} file{{ reports.length !== 1 ? 's' : '' }} in {{ processingTime }}ms
          </span>
          <span class="counts">
            <span class="count-error" v-if="totalErrors > 0">{{ totalErrors }} error{{ totalErrors !== 1 ? 's' : '' }}</span>
            <span class="count-warning" v-if="totalWarnings > 0">{{ totalWarnings }} warning{{ totalWarnings !== 1 ? 's' : '' }}</span>
            <span class="count-pass" v-if="totalErrors === 0 && totalWarnings === 0">No automated checks failed</span>
            <span class="count-review" v-if="totalReview > 0">
              {{ totalReview }} need{{ totalReview === 1 ? 's' : '' }} manual review
            </span>
          </span>
          <button class="clear-btn" @click="clearResults" type="button">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
            Clear
          </button>
        </div>

        <details
          v-for="report in reports"
          :key="report.path"
          class="file-report"
          open
        >
          <summary>
            <span class="file-name">{{ report.path }}</span>
            <span class="file-standard">{{ standardLabel(report.standard) }}</span>
            <span
              class="file-status"
              :class="report.error || countBySeverity(report.results, 'error') > 0 ? 'status-fail' : 'status-pass'"
            >
              {{ report.error ? 'Error' : countBySeverity(report.results, 'error') > 0 ? 'Non-compliant' : 'Compliant' }}
            </span>
          </summary>

          <div v-if="report.error" class="error-box">
            {{ report.error }}
          </div>

          <table v-if="failures(report.results).length > 0" class="results-table">
            <thead>
              <tr>
                <th>Status</th>
                <th>Rule</th>
                <th>Issue</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(result, i) in failures(report.results)"
                :key="i"
                :class="`severity-${result.severity}`"
              >
                <td class="col-status" data-label="Status"><span aria-hidden="true">{{ severityIcon(result.severity) }}</span><span class="visually-hidden">{{ result.severity }}</span></td>
                <td class="col-rule" data-label="Rule"><code>{{ result.rule_id }}</code></td>
                <td class="col-issue" data-label="Issue">
                  <template v-if="result.outcome.status === 'Fail'">
                    {{ result.outcome.message }}
                    <span v-if="result.outcome.location?.page" class="location">
                      (page {{ result.outcome.location.page }})
                    </span>
                  </template>
                  <template v-else>{{ result.description }}</template>
                </td>
              </tr>
            </tbody>
          </table>

          <p v-if="failures(report.results).length === 0 && !report.error" class="all-pass">
            No automated checks failed.
          </p>

          <details v-if="reviewItems(report.results).length > 0" class="review-list">
            <summary>
              {{ reviewItems(report.results).length }} condition{{ reviewItems(report.results).length !== 1 ? 's' : '' }}
              need manual review — the Matterhorn Protocol leaves these to a human
            </summary>
            <ul>
              <li v-for="(item, i) in reviewItems(report.results)" :key="i">
                <code>{{ item.rule_id }}</code>
                <span class="review-desc">{{ item.description }}</span>
                <span v-if="item.outcome.status === 'NeedsReview'" class="review-reason">{{ item.outcome.reason }}</span>
              </li>
            </ul>
          </details>
        </details>
      </div>
    </template>
  </div>
</template>

<style scoped>
.horn-demo {
  max-width: 888px;
  margin: 1.5rem auto 0;
}

.visually-hidden {
  clip: rect(0 0 0 0);
  clip-path: inset(50%);
  height: 1px;
  overflow: hidden;
  position: absolute;
  white-space: nowrap;
  width: 1px;
}

.loading-wasm {
  text-align: center;
  padding: 2rem;
  color: var(--vp-c-text-2);
}

.drop-zone {
  border: 2px dashed var(--vp-c-divider);
  border-radius: 12px;
  padding: 2.5rem 1.5rem;
  text-align: center;
  transition: border-color 0.2s, background 0.2s;
  cursor: pointer;
}

.drop-zone:hover,
.drop-zone.dragging {
  border-color: var(--vp-c-brand-1);
  background: var(--vp-c-bg-soft);
}

.drop-zone.disabled {
  opacity: 0.6;
  pointer-events: none;
}

.drop-content {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.drop-content svg {
  color: var(--vp-c-text-2);
  margin-bottom: 0.75rem;
}

.drop-text {
  font-size: 1.1rem;
  font-weight: 500;
  margin: 0;
}

.drop-or {
  color: var(--vp-c-text-2);
  margin: 0.5rem 0;
  font-size: 0.875rem;
}

.drop-note {
  color: var(--vp-c-text-2);
  font-size: 0.8rem;
  margin-top: 0.75rem;
  margin-bottom: 0;
}

.file-btn {
  display: inline-block;
  padding: 0.5rem 1.25rem;
  background: var(--vp-c-brand-3);
  color: var(--vp-c-white);
  border-radius: 8px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}

.file-btn:hover {
  background: var(--vp-c-brand-2);
}

.file-btn:focus-within {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: 2px;
}

.file-btn input {
  display: none;
}

.status-bar {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  background: var(--vp-c-bg-soft);
  border-radius: 8px;
  text-align: center;
  color: var(--vp-c-text-2);
}

.results {
  margin-top: 1.5rem;
}

.results-summary {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.75rem 1rem;
  background: var(--vp-c-bg-soft);
  border-radius: 8px;
  margin-bottom: 1rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.time {
  color: var(--vp-c-text-2);
  font-size: 0.875rem;
}

.counts {
  display: flex;
  gap: 0.75rem;
  font-weight: 600;
  font-size: 0.875rem;
}

.count-error {
  color: var(--vp-c-danger-1);
}

.count-warning {
  color: var(--vp-c-warning-3);
}

.count-pass {
  color: var(--vp-c-green-3);
}

.count-review {
  color: var(--vp-c-text-2);
}

.drop-version {
  color: var(--vp-c-text-3);
  font-size: 0.75rem;
  margin: 0.35rem 0 0;
}

.review-list {
  border-top: 1px solid var(--vp-c-divider);
  font-size: 0.85rem;
}

.review-list summary {
  padding: 0.6rem 1rem;
  cursor: pointer;
  color: var(--vp-c-text-2);
}

.review-list ul {
  list-style: none;
  margin: 0;
  padding: 0 1rem 0.75rem;
}

.review-list li {
  display: grid;
  grid-template-columns: 4.5rem 1fr;
  gap: 0.15rem 0.75rem;
  padding: 0.4rem 0;
  border-top: 1px solid var(--vp-c-divider);
}

.review-list li code {
  font-size: 0.8rem;
  background: var(--vp-c-bg-soft);
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
  align-self: start;
}

.review-desc {
  font-weight: 500;
}

.review-reason {
  grid-column: 2;
  color: var(--vp-c-text-2);
}

.file-report {
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  margin-bottom: 0.75rem;
  overflow: hidden;
}

.file-report summary {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  cursor: pointer;
  font-weight: 500;
  background: var(--vp-c-bg-soft);
  flex-wrap: wrap;
}

.file-report summary:hover {
  background: var(--vp-c-bg-elv);
}

.file-name {
  font-family: var(--vp-font-family-mono);
  font-size: 0.9rem;
}

.file-standard {
  color: var(--vp-c-text-2);
  font-size: 0.8rem;
  font-weight: 400;
}

.file-status {
  margin-left: auto;
  font-size: 0.8rem;
  font-weight: 600;
  padding: 0.15rem 0.5rem;
  border-radius: 4px;
}

.status-pass {
  background: color-mix(in srgb, var(--vp-c-green-3) 15%, transparent);
  color: var(--vp-c-green-3);
}

.status-fail {
  background: color-mix(in srgb, var(--vp-c-danger-1) 15%, transparent);
  color: var(--vp-c-danger-1);
}

.error-box {
  padding: 0.75rem 1rem;
  color: var(--vp-c-danger-1);
  background: color-mix(in srgb, var(--vp-c-danger-1) 8%, transparent);
  font-size: 0.875rem;
}

.results-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.875rem;
}

.results-table th {
  text-align: left;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--vp-c-divider);
  color: var(--vp-c-text-2);
  font-weight: 600;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.results-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--vp-c-divider);
  vertical-align: top;
}

.results-table tr:last-child td {
  border-bottom: none;
}

.col-status {
  width: 2.5rem;
  text-align: center;
}

.col-rule {
  white-space: nowrap;
}

.col-rule code {
  font-size: 0.8rem;
  background: var(--vp-c-bg-soft);
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
}

.col-detail {
  color: var(--vp-c-text-2);
}

.location {
  color: var(--vp-c-text-2);
  font-size: 0.8rem;
}

.all-pass {
  padding: 1rem;
  text-align: center;
  color: var(--vp-c-green-3);
  font-weight: 500;
}

.clear-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.35rem 0.75rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-2);
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
  transition: border-color 0.2s, color 0.2s;
}

.clear-btn:hover {
  border-color: var(--vp-c-text-3);
  color: var(--vp-c-text-1);
}

@media (max-width: 640px) {
  .drop-zone {
    padding: 1.5rem 1rem;
  }

  .drop-text {
    font-size: 1rem;
  }

  .results-summary {
    flex-direction: column;
    align-items: flex-start;
  }

  .file-report summary {
    gap: 0.5rem;
  }

  .file-name {
    font-size: 0.8rem;
    word-break: break-all;
  }

  .file-status {
    margin-left: 0;
  }

  .results-table thead {
    display: none;
  }

  .results-table,
  .results-table tbody,
  .results-table tr,
  .results-table td {
    display: block;
  }

  .results-table tr {
    padding: 0.75rem;
    border-bottom: 1px solid var(--vp-c-divider);
  }

  .results-table tr:last-child {
    border-bottom: none;
  }

  .results-table td {
    padding: 0.15rem 0;
    border-bottom: none;
    text-align: left;
  }

  .results-table td::before {
    content: attr(data-label);
    display: block;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--vp-c-text-2);
    margin-bottom: 0.1rem;
  }

  .col-status {
    width: auto;
    text-align: left;
  }

  .col-rule {
    white-space: normal;
  }
}
</style>
