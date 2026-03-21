# GitHub Action

Horn provides a drop-in GitHub Action for validating PDFs in your CI pipeline.

## Usage

```yaml
- uses: focusring/horn@v1
  with:
    path: ./output/
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `path` | yes | — | PDF file or directory to validate |
| `recurse` | no | `true` | Recursively scan directories |
| `format` | no | `sarif` | Output format: `text`, `json`, `sarif`, `junit` |
| `fail-on` | no | `error` | Minimum severity to fail: `error`, `warning`, `info` |
| `version` | no | `latest` | Horn version to install |

## Outputs

| Output | Description |
|--------|-------------|
| `report` | Path to the generated report file |
| `compliant` | `true` if all files are compliant, `false` otherwise |

## SARIF integration

When `format: sarif` (the default), the action automatically uploads results to GitHub Code Scanning using the `github/codeql-action/upload-sarif` action. Accessibility findings appear as annotations on pull requests and in the repository's Security tab.

## Examples

### Basic usage

```yaml
name: PDF Accessibility
on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: focusring/horn@v1
        with:
          path: ./docs/
```

### With JUnit output

```yaml
- uses: focusring/horn@v1
  with:
    path: ./output/
    format: junit
    fail-on: warning
```

### Pinned version

```yaml
- uses: focusring/horn@v1
  with:
    path: ./output/
    version: '0.2.0'
```
