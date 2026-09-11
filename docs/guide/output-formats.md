# Output Formats

Horn supports four output formats. Choose the one that best fits your workflow.

## Text (default)

Human-readable terminal output with file summaries and failure details.

```bash
horn validate document.pdf
```

```
document.pdf
============
  Standard: PDF/UA-1
  [FAIL] 11-001: Document catalog missing /Lang entry
  [FAIL] 13-004: Figure element has no /Alt or /ActualText [Figure]

  Summary: 41 passed, 2 failed, 24 needs review

---
Total: 0/1 files compliant
```

Rule ids are the official Matterhorn Protocol 1.1 indices. Add `--review` to
also list the human-judgment conditions that apply to the document (for
example, "tables are present: verify every column header cell is tagged as
`<TH>`"), so a reviewer has the complete Matterhorn checklist in one place.

Best for: local development and manual review.

## JSON

Structured JSON output with full report details.

```bash
horn validate document.pdf --format json
```

```json
{
  "files": [
    {
      "path": "document.pdf",
      "standard": "pdf-ua-1",
      "results": [
        {
          "rule_id": "11-001",
          "checkpoint": 11,
          "description": "Document catalog missing /Lang entry",
          "severity": "error",
          "outcome": {
            "status": "Fail",
            "message": "Document catalog missing /Lang entry"
          }
        },
        {
          "rule_id": "15-001",
          "checkpoint": 15,
          "description": "A row has a header cell, but that header cell is not tagged as a header.",
          "severity": "info",
          "outcome": {
            "status": "NeedsReview",
            "reason": "Tables are present: verify every row header cell is tagged as <TH>"
          }
        }
      ],
      "error": null
    }
  ]
}
```

Best for: programmatic processing, custom dashboards, integration with other tools.

## SARIF

[SARIF v2.1.0](https://sarifweb.azurewebsites.net/) output for GitHub Code Scanning.

```bash
horn validate document.pdf --format sarif -o results.sarif
```

When used with the Horn GitHub Action, SARIF results are automatically uploaded to GitHub Code Scanning, showing accessibility findings directly in pull requests.

Best for: GitHub repositories, security-style workflows.

## JUnit XML

JUnit XML format for CI dashboards.

```bash
horn validate document.pdf --format junit -o results.xml
```

Compatible with Jenkins, GitLab CI, Azure DevOps, and other CI systems that support JUnit test reports.

Best for: CI/CD pipelines with existing JUnit report infrastructure.
