# Checks Reference

Horn implements 21 check modules covering the Matterhorn Protocol checkpoints. Each check targets specific PDF/UA-1 (and some PDF/UA-2) failure conditions.

## Check modules

### baseline — Checkpoint 01–31

Built-in `pdf_oxide` PDF/UA-1 structure validation. Provides foundational checks that other modules build on.

### structure — Checkpoint 01, 02

Tagged PDF structure validation.

| Rule | Description |
|------|-------------|
| 01-003 | `/MarkInfo` must exist with `/Marked = true` |
| 01-004 | `/StructTreeRoot` must exist and have children |
| 02-001 | `/RoleMap` entries must resolve to standard types; circular chains detected |

### content_stream — Checkpoint 01, 10

Content stream analysis.

| Rule | Description |
|------|-------------|
| 01-001 | All text operations must be inside `BMC`/`BDC`..`EMC` marked content sequences |
| 01-005 | XObject invocations must be tagged or marked as artifacts |

### language — Checkpoint 02, 11

Natural language specification for text strings.

| Rule | Description |
|------|-------------|
| 02-001 | Outline `/Title` text must have language context (catalog `/Lang` or struct-level `/Lang`) |
| 02-002 | Annotation `/Contents` text must have language context |
| 02-003 | Widget `/TU` (tooltip) text must have language context |
| 02-004 | XMP `dc:title` must have a real language (not just `x-default`) when no catalog `/Lang` |
| 11-002 | All `/Lang` values must be valid BCP 47 tags |
| 11-005 | Elements with `/Alt` text need language context |
| 11-006 | Elements with `/ActualText` need language context |
| 11-007 | Elements with `/E` (expansion text) need language context |

### version — Checkpoint 05

PDF/UA version identification and XMP extension schema.

| Rule | Description |
|------|-------------|
| 05-001 | `/Metadata` stream must exist in catalog |
| 05-002 | `pdfuaid:part` value must match standard (1 for UA-1, 2 for UA-2); UA-2 requires PDF 2.0 |
| 05-003 | `pdfuaid:part` identifier must be present |
| 05-004 | Extension schema for `pdfuaid` must be properly defined (correct URI and prefix) |
| 05-005 | No duplicate extension schema definitions for the PDF/UA namespace |

### metadata — Checkpoint 06

Document-level metadata validation.

| Rule | Description |
|------|-------------|
| 06-001 | Document catalog must contain `/Lang` entry |
| 06-002 | XMP must contain `pdfuaid:part` identifier |
| 06-003 | `ViewerPreferences/DisplayDocTitle` must be `true` |
| 06-004 | XMP must contain `dc:title` |

### dict_entries — Checkpoint 07, 25

Dictionary-level validation and structural integrity.

| Rule | Description |
|------|-------------|
| 07-001 | `StructTreeRoot` must contain `/ParentTree`; completeness validated |
| 07-002 | `MarkInfo/Suspects` must not be `true` |
| 07-003 | Non-standard structure types must have `RoleMap` entries |
| 25-001 | Reference XObjects (`/Ref` on Form XObjects) are forbidden |

### nesting — Checkpoint 09

Structure element parent-child rules.

| Rule | Description |
|------|-------------|
| 09-001 | `TR` must be inside `Table`/`THead`/`TBody`/`TFoot` |
| 09-004 | `TH`/`TD` must be inside `TR` |
| 09-006 | Container child type rules: Table&#8594;TR/THead/TBody/TFoot/Caption; TR&#8594;TH/TD; L&#8594;LI/Caption; LI&#8594;Lbl/LBody; TOC&#8594;TOCI/TOC/Caption |
| &#8212; | Cardinality: at most one `THead`/`TFoot` per Table; `THead`/`TFoot` require `TBody` |
| &#8212; | Caption position: first or last for Table; first for TOC and List |

### images — Checkpoint 13

Figure accessibility.

| Rule | Description |
|------|-------------|
| 13-004 | `Figure` elements must have `/Alt` or `/ActualText` |
| 13-005 | `/Alt` must not be empty |

### headings — Checkpoint 14

Heading hierarchy validation.

| Rule | Description |
|------|-------------|
| 14-002 | First heading should be H1 |
| 14-003 | Generic `H` headings must use nesting to convey hierarchy (no sibling `H` elements) |
| 14-006 | No skipped heading levels (e.g., H1 followed by H3) |
| 14-007 | Must not mix numbered (H1&#8211;H6) and generic (`H`) headings |

### tables — Checkpoint 15

Table structure and header association.

| Rule | Description |
|------|-------------|
| 15-002 | Tables must contain `TR` with `TH` or `TD` children |
| 15-003 | Tables must have at least one `TH` header cell |
| 15-004 | `TH` cells must have `/Scope` (`/Row`, `/Column`, or `/Both`); invalid values flagged |
| 15-005 | Complex tables need `/Headers`, `/Scope`, or `THead`/`TBody` for header association |
| 15-006 | `RowSpan`/`ColSpan` must be valid positive integers within table dimensions |

### lists — Checkpoint 16

List structure validation.

| Rule | Description |
|------|-------------|
| 16-001 | `L` must contain `LI` children |
| 16-002 | `LI` must contain `Lbl` or `LBody` |
| 16-003 | `LBody` structure validation |

### math — Checkpoint 17

| Rule | Description |
|------|-------------|
| 17-001 | `Formula` elements must have alternative text (`/Alt`) |

### notes — Checkpoint 19

| Rule | Description |
|------|-------------|
| 19-001 | `Note` elements must have `/ID` attribute |
| 19-002 | `NoteRef` links validation |

### optional_content — Checkpoint 20

| Rule | Description |
|------|-------------|
| 20-001 | Optional content groups must have `/Name` |
| 20-002 | Default OCG configuration must be valid |
| 20-003 | `/AS` entry (auto-state) is forbidden |

### embedded_files — Checkpoint 21

| Rule | Description |
|------|-------------|
| 21-001 | Embedded files must have `/AF` relationship |
| 21-002 | File specification must have `/Desc` |

### xfa — Checkpoint 25

| Rule | Description |
|------|-------------|
| 25-001 | Document must not contain XFA form data |

### security — Checkpoint 26

| Rule | Description |
|------|-------------|
| 26-001 | Encryption must not block assistive technology access |
| 26-002 | Security handler must allow content extraction for accessibility |

### annotations — Checkpoint 28

Annotation accessibility (page-level checks).

| Rule | Description |
|------|-------------|
| 28-001 | Pages with annotations must have `/Tabs = /S` (structure order) |
| 28-004 | Link annotations must have `/A` (action) or `/Dest` (destination) |
| 28-006 | Annotations should have `/Contents` for accessible text |
| 28-009 | Form fields must have `/T` (field name) or `/TU` (tooltip) |

### annot_struct — Checkpoint 28

Annotation-to-structure-tree cross-validation.

| Rule | Description |
|------|-------------|
| 28-002 | All annotations (except Popup/PrinterMark) must have `OBJR` in structure tree |
| 28-003 | Parent struct element type must match annotation subtype (Link&#8594;`/Link`, Widget&#8594;`/Form`) |
| 28-005 | Screen annotations must have `/CT` (content type) on media clip |
| 28-006 | Annotations under `/Annot` struct elements need `/Contents` or `/Alt`; zero-size (invisible) annotations exempted |
| 28-007 | `TrapNet` annotations forbidden; `PrinterMark` validation |
| 28-008 | `FileAttachment` must have `/FS` with `/F` and `/UF` |
| 28-009 | Form fields need `/TU` (tooltip) or `/Alt` on parent; zero-size (invisible) widgets exempted |

### fonts — Checkpoint 31

Font embedding, encoding, and Unicode mapping.

| Rule | Description |
|------|-------------|
| 31-001 | All fonts must be embedded (`FontFile`/`FontFile2`/`FontFile3`) |
| 31-002 | `CIDFontType2` must have `/CIDToGIDMap` (`/Identity` or stream) |
| 31-003 | `CIDFont` must have valid `/CIDSystemInfo`; CMap encoding validated; Supplement consistency checked |
| 31-004 | `/CIDSet` must be a valid stream when present |
| 31-005 | `/Encoding` must be a valid predefined name or dictionary; non-symbolic TrueType must have encoding |
| 31-006 | Font must have `/ToUnicode` CMap or standard encoding |
| 31-007 | `ToUnicode` CMap must not map to U+0000 (null), U+FFFE, or U+FEFF (noncharacters) |

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
