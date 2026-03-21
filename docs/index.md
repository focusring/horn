---
layout: home

hero:
  image:
    src: /logo.svg
    alt: Horn logo
  name: Horn
  text: PDF accessibility testing at scale
  tagline: "Open-source PDF/UA checker based on the Matterhorn Protocol — v0.1.0"
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Try Live Demo
      link: /demo
    - theme: alt
      text: View on GitHub
      link: https://github.com/focusring/horn

features:
  - icon: "\u26A1"
    title: Blazing Fast
    details: Process 500+ PDFs per second with parallel processing. Built in Rust for maximum throughput — validate entire document libraries in seconds, not hours.
  - icon: "\uD83D\uDD0D"
    title: Matterhorn Protocol
    details: 21 check modules covering the machine-checkable failure conditions defined in the Matterhorn Protocol 1.1 for PDF/UA-1 (ISO 14289-1) compliance.
  - icon: "\uD83D\uDE80"
    title: CI/CD Native
    details: First-class support for SARIF (GitHub Code Scanning), JUnit XML, and JSON output. Ship accessible PDFs with every build.
  - icon: "\uD83D\uDCE6"
    title: Use Anywhere
    details: CLI, desktop app, browser (WebAssembly), npm package, GitHub Action, or Docker. One tool, every platform.
  - icon: "\uD83D\uDEE0\uFE0F"
    title: GitHub Action
    details: Drop-in GitHub Action with automatic SARIF upload. Add PDF accessibility checks to your workflow in two lines of YAML.
  - icon: "\uD83E\uDDE9"
    title: Extensible
    details: Add custom checks with the Check trait. Horn's modular architecture makes it easy to tailor validation to your requirements.
---

<style>
.benchmark {
  max-width: 688px;
  margin: 4rem auto 0;
  padding: 2rem;
  border-radius: 12px;
  background: var(--vp-c-bg-soft);
  text-align: center;
}

.benchmark h2 {
  font-size: 1.25rem;
  font-weight: 600;
  margin-bottom: 1rem;
}

.benchmark-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1.5rem;
}

.benchmark-stat .number {
  font-size: 2rem;
  font-weight: 700;
  color: var(--vp-c-brand-1);
}

.benchmark-stat .label {
  font-size: 0.875rem;
  color: var(--vp-c-text-2);
  margin-top: 0.25rem;
}
</style>

<div class="benchmark">
  <h2>Performance</h2>
  <div class="benchmark-grid">
    <div class="benchmark-stat">
      <div class="number">500+</div>
      <div class="label">PDFs / second</div>
    </div>
    <div class="benchmark-stat">
      <div class="number">21</div>
      <div class="label">check modules</div>
    </div>
    <div class="benchmark-stat">
      <div class="number">0</div>
      <div class="label">runtime dependencies</div>
    </div>
  </div>
</div>
