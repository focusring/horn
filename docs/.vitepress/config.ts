import { defineConfig } from 'vitepress'

export default defineConfig({
  base: '/horn/',
  title: 'Horn',
  description: 'Open-source PDF/UA accessibility checker based on the Matterhorn Protocol',
  head: [
    ['link', { rel: 'icon', href: '/horn/favicon.ico', type: 'image/x-icon' }],
    ['meta', { name: 'theme-color', content: '#f97316' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Reference', link: '/reference/cli' },
      { text: 'Demo', link: '/demo' },
      { text: 'Downloads', link: 'https://github.com/focusring/horn/releases/latest' },
      {
        text: 'v0.1.0',
        items: [
          { text: 'Changelog', link: 'https://github.com/focusring/horn/releases' },
          { text: 'GitHub', link: 'https://github.com/focusring/horn' },
        ],
      },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is Horn?', link: '/guide/what-is-horn' },
            { text: 'Getting Started', link: '/guide/getting-started' },
          ],
        },
        {
          text: 'Usage',
          items: [
            { text: 'Validating PDFs', link: '/guide/validating-pdfs' },
            { text: 'Output Formats', link: '/guide/output-formats' },
            { text: 'CI/CD Integration', link: '/guide/ci-cd' },
            { text: 'WebAssembly', link: '/guide/wasm' },
            { text: 'Desktop App', link: '/guide/desktop-app' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'Reference',
          items: [
            { text: 'CLI', link: '/reference/cli' },
            { text: 'Checks', link: '/reference/checks' },
            { text: 'GitHub Action', link: '/reference/github-action' },
            { text: 'Exit Codes', link: '/reference/exit-codes' },
          ],
        },
      ],
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/focusring/horn' },
    ],
    footer: {
      message: 'Released under the MIT / Apache 2.0 License.',
      copyright: 'Copyright © 2025-present Horn Contributors',
    },
    search: {
      provider: 'local',
    },
  },
})
