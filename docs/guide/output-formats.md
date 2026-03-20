# Output Formats

Horn supports four output formats. Choose the one that best fits your workflow.

## Text (default)

Human-readable terminal output with file summaries and failure details.

```bash
horn validate document.pdf
```

```
document.pdf (PDF/UA-1)
  FAIL  06-001  Document language not set
  FAIL  13-004  Figure missing Alt text (page 3, element Figure)
  PASS  14-002  Heading levels not skipped

1 file, 2 failures, 1 pass
```

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
          "rule_id": "06-001",
          "checkpoint": 6,
          "description": "Document language not set",
          "severity": "error",
          "outcome": {
            "status": "Fail",
            "message": "No /Lang entry in document catalog"
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
