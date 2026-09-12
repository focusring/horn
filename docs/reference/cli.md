# CLI Reference

## Commands

### `horn validate`

Validate PDF files against PDF/UA-1 or PDF/UA-2 (auto-detected from `pdfuaid:part`).

```bash
horn validate <FILES...> [OPTIONS]
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `<FILES...>` | One or more PDF files or directories to validate |

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `-f, --format <FORMAT>` | `text` | Output format: `text`, `json`, `sarif`, `junit` |
| `-o, --output <PATH>` | stdout | Write output to a file |
| `-r, --recurse` | off | Recursively scan directories for PDFs |
| `--fail-on <SEVERITY>` | `error` | Minimum severity to trigger a non-zero exit: `error`, `warning`, `info` |
| `--review` | off | Text output: also list the Matterhorn conditions that need manual review for this document |

Findings use the official Matterhorn Protocol 1.1 failure-condition index as
their rule id (for example `28-010`); Horn-specific extension rules use ids of
the form `NN-xNN`. See the [checks reference](./checks.md).

### `horn coverage`

Print the Matterhorn Protocol coverage matrix: every failure condition with its
checkpoint, whether it is machine-checkable or requires human judgment, and how
Horn covers it.

```bash
horn coverage          # table
horn coverage --json   # machine-readable
```

### `horn list-checks`

Print all registered check modules with their ID, checkpoint number,
description and the number of rules they emit.

```bash
horn list-checks
```

### `horn completions`

Generate shell completion scripts.

```bash
horn completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | All files are compliant |
| `1` | One or more files have findings at or above `--fail-on` severity |
| `2` | CLI or runtime error (invalid arguments, file not found, etc.) |
