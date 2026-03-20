# CI/CD Integration

Horn is built for automated pipelines. Use any of the output formats to integrate accessibility checks into your CI/CD workflow.

## GitHub Actions

### Using the Horn Action

The simplest way to add Horn to GitHub Actions:

```yaml
steps:
  - uses: actions/checkout@v4
  - uses: focusring/horn@v1
    with:
      path: ./output/
      format: sarif
      fail-on: error
```

When `format: sarif`, the action automatically uploads results to GitHub Code Scanning. Accessibility findings appear directly in pull request reviews.

See [GitHub Action Reference](/reference/github-action) for all options.

### Manual setup

If you need more control:

```yaml
steps:
  - uses: actions/checkout@v4
  - name: Install Horn
    run: |
      curl -sL https://github.com/focusring/horn/releases/latest/download/horn-linux-x86_64 \
        -o /usr/local/bin/horn && chmod +x /usr/local/bin/horn
  - name: Validate PDFs
    run: horn validate ./docs/ --recurse --format sarif -o results.sarif
  - uses: github/codeql-action/upload-sarif@v3
    if: always()
    with:
      sarif_file: results.sarif
```

## GitLab CI

```yaml
pdf-accessibility:
  image: rust:latest
  script:
    - cargo install --git https://github.com/focusring/horn.git
    - horn validate ./docs/ --recurse --format junit -o results.xml
  artifacts:
    reports:
      junit: results.xml
```

JUnit results appear in GitLab's merge request test report widget.

## Docker

For environments where installing a binary is not possible:

```yaml
steps:
  - name: Validate PDFs
    run: |
      docker run --rm -v $(pwd):/data horn \
        validate /data/output/ --recurse --format json
```

## Gradual adoption

Use `--fail-on` to introduce accessibility checks without blocking existing pipelines:

1. **Start with `--fail-on error`** — only block on critical failures
2. **Move to `--fail-on warning`** — catch more issues as your team fixes existing ones
3. **End with `--fail-on info`** — enforce full compliance
