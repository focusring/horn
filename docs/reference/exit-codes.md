# Exit Codes

Horn uses exit codes to communicate validation results to scripts and CI/CD pipelines.

| Code | Meaning |
|------|---------|
| `0` | All files are compliant — no findings at or above the `--fail-on` threshold |
| `1` | One or more files have findings at or above the `--fail-on` severity |
| `2` | Runtime or CLI error (invalid arguments, unreadable files, etc.) |

## Usage in scripts

```bash
horn validate document.pdf --fail-on error
if [ $? -eq 0 ]; then
  echo "PDF is accessible"
elif [ $? -eq 1 ]; then
  echo "Accessibility issues found"
else
  echo "Validation error"
fi
```

## CI/CD behavior

Most CI systems treat any non-zero exit as a failure. Use `--fail-on` to control when Horn triggers a pipeline failure:

```bash
# Strict: fail on any finding
horn validate ./pdfs/ --recurse --fail-on info

# Moderate: fail on warnings and errors
horn validate ./pdfs/ --recurse --fail-on warning

# Lenient: only fail on errors (default)
horn validate ./pdfs/ --recurse --fail-on error
```
