# Horn: Roadmap to 90% Matterhorn Protocol Coverage

## Current State (as of March 2026)

Horn implements **21 check modules** with **89.5% accuracy** against the veraPDF PDF/UA-1 test corpus (296 files).

| Metric | Value |
|--------|-------|
| **Overall accuracy** | **265/296 (89.5%)** |
| **Pass accuracy** | **139/141 (98.6%)** — 2 false positives |
| **Fail detection** | **126/155 (81.3%)** |
| **Check modules** | 21 (baseline, metadata, version, structure, dict_entries, fonts, headings, tables, images, annotations, lists, xfa, security, optional_content, embedded_files, math, notes, language, content_stream, nesting, annot_struct) |

**Repository**: `/Users/tim/dev/horn`
**Test corpus**: `tests/fixtures/verapdf-corpus/PDF_UA-1/` (296 PDF files, git submodule from veraPDF)

### Known False Positives (2)

| File | Rule | Issue |
|------|------|-------|
| 7.18.1-t02-pass-d.pdf | 28-006 | Highlight annotation under Annot struct elem flagged for missing Alt/Contents — the annotation is valid because it has F flags that exempt it from this requirement. Need to detect additional annotation flag combinations. |
| 7.18.1-t03-pass-c.pdf | 28-009 | Widget annotation flagged for missing TU — the widget is hidden/non-interactive but our hidden detection doesn't catch all cases. Need to check additional widget visibility indicators. |

---

## Architecture Overview

### How to add a new check

1. Create `src/checks/{name}.rs`
2. Implement the `Check` trait:
   ```rust
   pub struct MyCheck;
   impl Check for MyCheck {
       fn id(&self) -> &'static str { "XX-name" }
       fn checkpoint(&self) -> u8 { XX }
       fn description(&self) -> &'static str { "..." }
       fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> { ... }
   }
   ```
3. Add `pub mod name;` to `src/checks/mod.rs`
4. Add `Box::new(name::MyCheck)` to `CheckRegistry::new()`
5. Run `cargo clippy` — project uses pedantic clippy

### Key types

- `HornDocument` — wraps both `pdf_oxide::PdfDocument` and `lopdf::Document`
- `doc.oxide()` — structure tree, compliance baseline
- `doc.lopdf()` — raw PDF object access, content streams, font dicts, annotations
- `doc.raw_catalog()` — document catalog dictionary
- `lopdf::Object::as_stream()` — use this for PDF stream objects (NOT `as_dict()` which only matches Dictionary, not Stream)
- Stream dict access: `stream.dict` gives you the dictionary portion of a stream object
- `CheckResult` with `rule_id`, `checkpoint`, `severity`, `outcome`
- `CheckOutcome::Pass | Fail { message, location } | NeedsReview { reason } | NotApplicable`

### Key patterns

**Structure tree walking** — used by language.rs, nesting.rs, images.rs, annot_struct.rs:
```rust
fn walk_struct_tree(doc: &lopdf::Document, dict: &lopdf::Dictionary, depth: usize) {
    if depth > 100 { return; }
    let elem_type = dict.get(b"S").ok().and_then(|o| o.as_name().ok());
    // ... check this element ...
    let Ok(kids) = dict.get(b"K") else { return };
    match kids {
        lopdf::Object::Array(arr) => { /* iterate, dereference, recurse */ }
        lopdf::Object::Reference(id) => { /* dereference and recurse */ }
        lopdf::Object::Dictionary(d) => { /* recurse */ }
        _ => {} // Integer MCIDs — leaf content
    }
}
```

**Annotation OBJR collection with parent info** — annot_struct.rs:
```rust
// collect_objr_with_info tracks parent struct elem type, Alt, and TU
// so we can validate annotation↔structure associations
struct ObjrInfo {
    parent_type: Vec<u8>,    // /S of parent struct elem
    parent_has_alt: bool,    // parent has non-empty /Alt
    parent_has_tu: bool,     // parent has non-empty /TU
}
```

**Font iteration** — used by fonts.rs:
```rust
let pages = lopdf_doc.get_pages();
for (page_num, page_id) in &pages {
    let Ok(fonts) = lopdf_doc.get_page_fonts(*page_id) else { continue };
    for (font_name, font_dict) in &fonts { /* check font */ }
}
```

### Known lopdf limitations

- `lopdf` merges incremental PDF updates — it cannot detect broken incremental updates where later revisions invalidate earlier structure. This affects 2 test files (7.1-t02-fail-a, 7.1-t03-fail-a).
- `Object::as_dict()` does NOT work on Stream objects — use `as_stream()` then access `.dict` for the dictionary portion.
- `page_iter()` returns `ObjectId` (tuple `(u32, u16)`), not page numbers.

---

## What's Implemented (check modules)

| Module | Checkpoint(s) | Conditions covered |
|--------|--------------|-------------------|
| baseline.rs | Various | Delegates to pdf_oxide PdfUaValidator (~30 conditions) |
| metadata.rs | 06 | Lang, pdfuaid:part, DisplayDocTitle, dc:title |
| version.rs | 05 | /Metadata stream, pdfuaid:part value, extension schema |
| structure.rs | 01, 02 | MarkInfo/Marked, StructTreeRoot, RoleMap, **circular RoleMap detection** |
| dict_entries.rs | 07 | ParentTree presence + completeness, Suspects, struct type validation |
| fonts.rs | 31 | Font embedding (31-001), CIDToGIDMap (31-002), CIDSystemInfo (31-003), CMap encoding validation, WMode consistency, **Encoding/BaseEncoding validation (31-005)**, ToUnicode presence (31-006), ToUnicode null mapping (31-007), CIDSet stream validation (31-004) |
| headings.rs | 14 | H1 first, level skipping, H/Hn mixing |
| tables.rs | 15 | TH/TD structure, header cells, Scope, **Scope value validation**, complex table Headers, **RowSpan/ColSpan validation** |
| images.rs | 13 | Figure Alt/ActualText |
| annotations.rs | 28 | Tab order (/Tabs=/S), link destinations, Contents, widget TU/T |
| annot_struct.rs | 28 | **Comprehensive annotation↔structure validation**: OBJR cross-ref, TrapNet forbidden, PrinterMark not in struct tree, **OBJR parent type validation** (Link→Link, Widget→Form, other→Annot), **Annot struct elem Alt/Contents**, **Widget TU/Alt with visibility checks**, **Screen media clip CT/Alt**, **FileAttachment FileSpec F/UF validation** |
| lists.rs | 16 | L/LI/LBody validation |
| xfa.rs | 25 | XFA form data forbidden |
| security.rs | 26 | Encryption accessibility permissions |
| optional_content.rs | 20 | OCG names, default config, **AS (auto-state) forbidden** |
| embedded_files.rs | 21 | AF relationship, file description |
| math.rs | 17 | Formula alt text |
| notes.rs | 19 | Note ID, NoteRef links |
| language.rs | 11 | BCP 47 validation (all /Lang tags), Lang on Alt/ActualText/E elements |
| content_stream.rs | 01, 10 | Untagged content detection, marked content analysis |
| nesting.rs | 09 | Structure element nesting validation |

---

## Remaining 29 Missed Fail Files (what's needed for 90%+)

### Group 1: Annotations (1 file) — Medium difficulty

| File | Issue | What's needed |
|------|-------|---------------|
| 7.18.1-t03-fail-d | Radio button TU | /TU is on individual radio buttons but not on the parent field dict — need to detect radio group field hierarchy |

### Group 2: Text/Tables (9 files) — Medium difficulty

| File | Issue | What's needed |
|------|-------|---------------|
| 7.2-t02-fail-a | Outlines Lang | Document Outlines (/Outlines) must have /Lang when doc-level Lang is missing |
| 7.2-t24-fail-a | Contents Lang | /Contents entry on annotation needs Lang context |
| 7.2-t25-fail-a | TU Lang | /TU (tooltip) on form field needs Lang context |
| 7.2-t27-fail-a | Caption nesting | Caption struct elem must be last child of its parent |
| 7.2-t33-fail-a | dc:title Lang | XMP dc:title needs associated xml:lang attribute |
| 7.2-t42-fail-a | ColSpan mismatch | ColSpan value doesn't match actual table grid width |
| 7.2-t43-fail-a/b/c | Table grid | RowSpan/ColSpan values create inconsistent table grid (needs full grid layout algorithm) |

### Group 3: Fonts (8 files) — High difficulty

| File | Issue | What's needed |
|------|-------|---------------|
| 7.21.3.1-t01-fail-c | Supplement mismatch | CIDSystemInfo Supplement comparison |
| 7.21.4.2-t01-fail-a/b | Font embedding | CIDFontType2 specific embedding requirements |
| 7.21.4.2-t02-fail-a | CIDSet content | CIDSet stream content doesn't cover all used CIDs |
| 7.21.5-t01-fail-a | Width mismatch | Font /Widths vs embedded font program metrics |
| 7.21.6-t02-fail-c/d | Encoding edge cases | c: missing /Encoding on non-symbolic TrueType; d: empty /Differences array |
| 7.21.7-t02-fail-b/c | ToUnicode CMap | Incomplete or malformed bfchar/bfrange entries |
| 7.21.8-t01-fail-a | .notdef glyph | Content stream uses character codes mapping to .notdef |

### Group 4: Other (11 files) — Mixed difficulty

| File | Issue | What's needed |
|------|-------|---------------|
| 5-t02-fail-a | pdfuaid:part=2 | File claims UA-2 but should be validated as UA-1 — need explicit standard override |
| 5-t04-fail-a | Extension schema | Specific XMP extension schema validation edge case |
| 5-t05-fail-a | Extension schema | Specific XMP extension schema validation edge case |
| 7.1-t02-fail-a | Incremental update | **lopdf limitation** — can't detect broken incremental updates |
| 7.1-t03-fail-a | Incremental update | **lopdf limitation** |
| 7.20-t01-fail-a | XObject tagging | Form XObject not properly tagged in content stream |
| 7.4.4-t01-fail-a | Unnumbered heading | Generic H elements — unclear failure condition (pass files also use only H) |
| 7.5-t01-fail-a | Invalid Scope value | TH cells have `/Scope /` (empty value) — need Scope value validation against actual struct |
| 7.5-t02-fail-a | Invalid Headers ref | /Headers references non-existent structure element ID |

---

## Path to 90% (3 more catches needed)

To reach 267/296 (90.2%), we need 2 more catches (currently 265 with 2 FPs = net 263; fixing FPs gives 265 with 0 FPs, needing 2 more for 267).

### Priority fixes:
1. **Fix the 2 false positives** — investigate exact F flag values in pass-c/pass-d to find the hidden/exempt condition
2. **Table Scope value validation** (2 files: 7.5-t01-fail-a, 7.5-t02-fail-a) — validate `/Scope` is `/Row`, `/Column`, or `/Both`, not empty
3. **Outlines Lang check** (1 file: 7.2-t02-fail-a) — check Outlines for /Lang when doc-level Lang is missing
4. **Caption nesting** (1 file: 7.2-t27-fail-a) — Caption must be last child
5. **Extension schema edge cases** (2 files: 5-t04, 5-t05) — specific XMP validation

---

## Running Tests

```bash
cd /Users/tim/dev/horn
cargo build --release

# Full corpus test
./target/release/horn validate tests/fixtures/verapdf-corpus/PDF_UA-1/ -r --format json -o /tmp/results.json

# Test accuracy
python3 -c "
import json, os
with open('/tmp/results.json') as f: data = json.load(f)
pc = pt = fc = ft = 0
for fr in data['files']:
    fn = os.path.basename(fr['path'])
    has_f = any(r['outcome']['status']=='Fail' for r in fr['results'])
    if 'fail' in fn: ft += 1; fc += has_f
    elif 'pass' in fn: pt += 1; pc += (not has_f)
c = pc + fc; t = pt + ft
print(f'Overall: {c}/{t} ({100*c/t:.1f}%) | Pass: {pc}/{pt} | Fail: {fc}/{ft}')
"

# Test specific section
./target/release/horn validate tests/fixtures/verapdf-corpus/PDF_UA-1/7.21\ Fonts/ -r --format json

# Single file with verbose output
./target/release/horn validate path/to/file.pdf --format json
```

### Test file naming convention

Files follow: `{section}-t{test_id}-{pass|fail}-{variant}.pdf`
- `pass` files must produce 0 Fail results
- `fail` files must produce ≥1 Fail result
- Sections map to ISO 14289-1 clauses

---

## Progress History

| Date | Score | Notes |
|------|-------|-------|
| Session 1 | 168/296 (56.8%) | Initial implementation |
| Session 2 | 244/296 (82.4%) | Font, annotation, language, content stream checks |
| Session 3 | 265/296 (89.5%) | Encoding validation, RoleMap cycles, OC AS, table attributes, annotation struct overhaul |

---

## References

- [Matterhorn Protocol 1.1](https://pdfa.org/resource/the-matterhorn-protocol/) — the definitive spec
- [veraPDF PDF/UA-1 rules](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) — machine-readable rule definitions
- [veraPDF test corpus](https://github.com/veraPDF/veraPDF-corpus) — pass/fail test files
- [ISO 32000-1 (PDF 1.7)](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf) — PDF reference for object structures
- [PDFlib Matterhorn knowledge base](https://www.pdflib.com/pdf-knowledge-base/pdfua/matterhorn-protocol/) — human-readable checkpoint explanations
