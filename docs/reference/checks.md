# Checks Reference

Horn implements 21 check modules covering the Matterhorn Protocol checkpoints. Each check targets specific PDF/UA-1 failure conditions.

## Check modules

| Module | Checkpoint | What it validates |
|--------|-----------|-------------------|
| baseline | 01–31 | pdf_oxide built-in PDF/UA-1 structure validation |
| metadata | 06 | Document language, title display, PDF/UA identifier, XMP metadata |
| version | 05 | `/Metadata` stream presence and validity |
| structure | 01, 02 | Tagged PDF (`MarkInfo`), `StructTreeRoot`, role mapping, circular refs |
| dict_entries | 07 | `ParentTree`, `Suspects` flag, structure type validation |
| fonts | 31 | Font embedding, `ToUnicode` CMaps, `CIDSet`, encoding |
| headings | 14 | H1–H6 hierarchy, no skipped levels, no `H`/`Hn` mixing |
| tables | 15 | `TH`/`TD` structure, `Scope` attribute, `RowSpan`/`ColSpan` |
| images | 13 | `Figure` alt text (`Alt` or `ActualText`) |
| annotations | 28 | Tab order, link destinations, widget `TU`/`T` fields, `Contents` |
| annot_struct | 28 | Annotation–structure association, `OBJR` references, visibility |
| lists | 16 | `L`/`LI`/`LBody` nesting validation |
| xfa | 25 | XFA form detection (forbidden in PDF/UA) |
| security | 26 | Encryption accessibility flags |
| optional_content | 20 | OCG names, default config, `AS` entry (forbidden) |
| embedded_files | 21 | `AF` relationship, file description |
| math | 17 | Formula alt text |
| notes | 19 | Note ID, NoteRef links |
| language | 11 | BCP 47 validation, `Lang` on all text elements |
| content_stream | 01, 10 | Untagged content detection |
| nesting | 09 | Structure element nesting rules |

## Severities

Each finding has a severity level:

| Severity | Meaning |
|----------|---------|
| **error** | The PDF violates a PDF/UA-1 requirement |
| **warning** | Potential issue that may affect accessibility |
| **info** | Informational finding or best-practice suggestion |

## Check outcomes

Each check produces one of four outcomes:

| Outcome | Meaning |
|---------|---------|
| **Pass** | The document satisfies this check |
| **Fail** | A violation was found (includes a message and optional location) |
| **NeedsReview** | The check cannot determine compliance automatically — manual review required |
| **NotApplicable** | The check does not apply to this document |

## Listing checks

Run `horn list-checks` to see all checks registered in your version of Horn:

```bash
horn list-checks
```
