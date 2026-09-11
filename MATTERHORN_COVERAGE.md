# Horn: Matterhorn Protocol Coverage

## Status (2026-09-11)

Horn covers **every failure condition** of the Matterhorn Protocol 1.1 (PDF/UA-1).
The protocol's tables contain 137 index entries — its introduction still says
136, but version 1.1 added condition 13-008:

| | Conditions | Horn |
|---|---|---|
| Machine-checkable | 87 | 87 implemented as automated checks |
| Human judgment | 48 | 48 reported as manual-review items when the document contains the relevant feature |
| No test defined (23-001, 27-001) | 2 | reported as not applicable |
| **Total** | **137** | **137** appear in every report |

Test results (`cargo test`, enforced by `tests/corpus.rs`):

| Fixture set | Result |
|-------------|--------|
| veraPDF PDF/UA-1 corpus, pass files | 141/141 compliant (0 false positives) |
| veraPDF PDF/UA-1 corpus, fail files | 156/156 detected |
| PDF/UA-1 Reference Suite (PDF Association) | 10/10 compliant |
| Generated edge-case + pdfcheck fixtures | 93/93 as expected |
| veraPDF PDF/UA-2 corpus (not a target yet) | 122/138 (52/54 pass, 70/84 fail) |

`horn coverage` prints the matrix from the installed binary; the reference
documentation is in `docs/reference/checks.md`.

## Rule ids

Every finding uses the **official Matterhorn index** as its `rule_id`
(`28-010` = "A widget annotation is not nested within a `<Form>` tag"). Checks
that go beyond the protocol — PDF/UA prerequisites such as "not a tagged PDF",
irregular tables, file syntax — use extension ids of the form `NN-xNN`.

Versions before 0.3.0 used their own numbering; see the 0.3.0 changelog entry
in the release notes when migrating SARIF / JUnit consumers.

## How the hard conditions are covered

| Area | Approach |
|------|----------|
| Font programs (31-011 … 31-018, 31-023 … 31-026) | `src/fontprog/` parses embedded TrueType/OpenType (`ttf-parser`), CFF/Type1C and Type 1 programs: glyph names and CIDs, advance widths (incl. CFF `FontMatrix`, Type 1 `hsbw`), cmap subtables, built-in encodings. |
| Used glyphs (31-009, 31-011, 31-016, 31-018, 31-030, 10-001, 31-027) | `src/content/` walks every content stream (pages, nested Form XObjects, annotation appearances) tracking `Tf`/`Tr`, splits strings into codes via the font's CMap, and records which codes are rendered (Tr ≠ 3). |
| Code → glyph resolution | ISO 32000-1 9.6.6: Differences → base encoding → built-in encoding → StandardEncoding; TrueType (3,1)/(1,0)/(3,0)/post lookups; CID → GID via `CIDToGIDMap` or CFF charset. |
| ToUnicode (31-027 … 31-029) | A real CMap parser (`bfchar`, `bfrange` incl. array form); the 7.21.7 exemptions (standard encodings, AGL glyph names, Adobe CJK collections, non-symbolic TrueType). |
| Tables (15-003, 15-x04) | Grid layout with `RowSpan`/`ColSpan` to detect irregular tables (veraPDF 7.2-42/43). |
| Human-judgment conditions | `human_review.rs` scans the document for tables, lists, figures, annotations, multimedia, JavaScript, article threads, OCR text, embedded-font licence flags, … and emits `NeedsReview` with a reviewer-facing reason for each applicable condition. |

## Known interpretation choices

- **31-003** follows ISO 14289-1 7.21.3.1 / veraPDF (CIDFont Supplement ≤ CMap Supplement) rather than the inverted wording in the protocol table.
- **Irregular tables** (`15-x04`) are failures, as in the veraPDF corpus, although Matterhorn 01-006 only suggests a warning.
- **17-003** is derived from 10-001 / 31-027 for documents containing `<Formula>` elements (Matterhorn: "see 10-001").
- **28-006** mirrors 28-002 / 28-004 for annotation subtypes not defined in ISO 32000.

## Not yet done

- PDF/UA-2 (ISO 14289-2) has its own rule set; Horn only applies the UA-1 checks that also hold for UA-2. 16 UA-2 corpus files are still misjudged.
- `pdf_oxide` heuristics mapped to extension ids (`13-x01` … `28-x05`) are advisory warnings.

## Progress history

| Date | Score (UA-1 corpus) | Notes |
|------|---------------------|-------|
| Session 1 | 168/296 (56.8%) | Initial implementation |
| Session 2 | 244/296 (82.4%) | Font, annotation, language, content stream checks |
| Session 3 | 265/296 (89.5%) | Encoding validation, RoleMap cycles, OC AS, table attributes, annotation struct overhaul |
| Session 4 | 285/296 (96.3%) | Untagged images, annotation language, .notdef detection |
| 2026-09-11 | **297/297 (100%)** | Official Matterhorn ids, font-program analysis, content usage, table grid, all 137 conditions covered |

## References

- [Matterhorn Protocol 1.1](https://pdfa.org/resource/the-matterhorn-protocol/) — the definitive spec
- [veraPDF PDF/UA-1 rules](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) — machine-readable rule definitions
- [veraPDF test corpus](https://github.com/veraPDF/veraPDF-corpus) — pass/fail test files
- [ISO 32000-1 (PDF 1.7)](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf) — PDF reference for object structures
- [PDFlib Matterhorn knowledge base](https://www.pdflib.com/pdf-knowledge-base/pdfua/matterhorn-protocol/) — human-readable checkpoint explanations
