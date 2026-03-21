---
layout: page
title: Live Demo
---

<script setup>
import HornDemo from './.vitepress/components/HornDemo.vue'
</script>

<div class="demo-page">

# Live Demo

Validate PDF files against PDF/UA-1 directly in your browser. Horn runs as WebAssembly — your files never leave your device.

<p class="demo-note">This browser demo runs via WebAssembly which is significantly slower than the native CLI. For production use and accurate performance benchmarks, use <code>horn validate</code> on the command line — it processes ~660 PDFs/second with parallel execution.</p>

<HornDemo />

</div>

<style>
.demo-page {
  max-width: 988px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
}

.demo-page h1 {
  font-size: 1.75rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.demo-page > p {
  color: var(--vp-c-text-2);
  font-size: 1rem;
  line-height: 1.6;
  margin-bottom: 0;
}

.demo-page > .demo-note {
  font-size: 0.9rem;
  margin-top: 0.75rem;
}

@media (max-width: 640px) {
  .demo-page {
    padding: 1.5rem 1rem;
  }

  .demo-page h1 {
    font-size: 1.5rem;
  }
}
</style>
