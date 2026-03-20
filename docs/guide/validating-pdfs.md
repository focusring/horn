# Validating PDFs

## Basic usage

```bash
horn validate <FILES...> [OPTIONS]
```

Horn accepts one or more file paths or directories as arguments.

### Single file

```bash
horn validate report.pdf
```

### Multiple files

```bash
horn validate report.pdf invoice.pdf brochure.pdf
```

### Glob patterns

```bash
horn validate *.pdf
horn validate docs/**/*.pdf
```

### Directories

Use `--recurse` to scan directories recursively for PDF files:

```bash
horn validate ./output/ --recurse
```

## Output format

Choose the output format with `--format`:

```bash
horn validate document.pdf --format json
horn validate document.pdf --format sarif
horn validate document.pdf --format junit
horn validate document.pdf --format text   # default
```

See [Output Formats](/guide/output-formats) for details on each format.

## Writing to a file

Use `--output` to write results to a file instead of stdout:

```bash
horn validate ./pdfs/ --recurse --format sarif -o results.sarif
```

## Controlling failure severity

By default, Horn exits with code `1` if any **error**-level findings are found. Use `--fail-on` to change the threshold:

```bash
# Fail on warnings or errors (ignore info)
horn validate document.pdf --fail-on warning

# Fail on any finding including info
horn validate document.pdf --fail-on info

# Only fail on errors (default)
horn validate document.pdf --fail-on error
```

This is useful in CI/CD pipelines where you may want to gradually tighten accessibility requirements.

## Performance

Horn processes PDFs in parallel using [Rayon](https://github.com/rayon-rs/rayon). On a modern machine, expect throughput of **500+ PDFs per second** in release mode.

For best performance:

- Use release builds (`cargo build --release`)
- Pass directories with `--recurse` instead of individual files — this lets Horn's thread pool work efficiently
- Use `--format json` or `--format sarif` when piping to other tools — text output is slower for large batches
