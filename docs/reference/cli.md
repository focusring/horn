# CLI Reference

## Commands

### `horn validate`

Validate PDF files against PDF/UA-1.

```
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

### `horn list-checks`

Print all registered checks with their ID, checkpoint number, and description.

```
horn list-checks
```

### `horn completions`

Generate shell completion scripts.

```
horn completions <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | All files are compliant |
| `1` | One or more files have findings at or above `--fail-on` severity |
| `2` | CLI or runtime error (invalid arguments, file not found, etc.) |
