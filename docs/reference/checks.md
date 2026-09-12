# Checks Reference

Horn covers **every failure condition of the Matterhorn Protocol 1.1** for
PDF/UA-1 (ISO 14289-1). The protocol's tables list 137 index entries (its
introduction still says 136; version 1.1 added 13-008):

| | Conditions | Coverage |
|---|---|---|
| Machine-checkable (`M`) | 87 | **87/87** implemented as automated checks |
| Human judgment (`H`) | 48 | **48/48** reported as *manual review* items when applicable |
| No test defined | 2 | 23-001 and 27-001 are reported as not applicable |
| **Total** | **137** | **137/137** appear in every report |

Every result's `rule_id` is the **official Matterhorn index** (e.g. `28-010` — a
Widget annotation not nested in a `<Form>` tag), so findings can be compared
directly with PAC, veraPDF and the protocol itself. Checks that go beyond the
protocol use *extension ids* of the form `NN-xNN` (listed at the end).

Run `horn coverage` (or `horn coverage --json`) to print this matrix from the
binary you have installed.

PDF/UA-2 (ISO 14289-2:2024) documents — `pdfuaid:part` 2 — get the same checks
(adapted where PDF 2.0 changes the rules) plus the
[PDF/UA-2 rules](#pdf-ua-2-iso-14289-2-rules) for the requirements that only
exist in PDF/UA-2.

## Outcomes and severities

| Outcome | Meaning |
|---------|---------|
| **Pass** | The document satisfies this condition |
| **Fail** | A violation was found (includes a message and, where known, a page/element) |
| **NeedsReview** | The condition requires human judgment; the document contains the relevant feature (tables, figures, JavaScript, …). Listed in text output with `--review`. |
| **NotApplicable** | The condition does not apply to this document |

Severities: **error** (PDF/UA-1 requirement violated), **warning** (potential issue or heuristic), **info** (informational). `--fail-on` selects which severities affect the exit code; `NeedsReview` never does.

## Matterhorn Protocol coverage

Legend — *How*: `M` machine-checkable, `H` human judgment. *Module*: the check module in `src/checks/` that emits the id.


### Checkpoint 01: Real content tagged

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 01-001 | H | Artifact is tagged as real content. | `human_review` |
| 01-002 | H | Real content is marked as artifact. | `human_review` |
| 01-003 | M | Content marked as Artifact is present inside tagged content. | `content_stream` |
| 01-004 | M | Tagged content is present inside content marked as Artifact. | `content_stream` |
| 01-005 | M | Content is neither marked as Artifact nor tagged as real content. | `baseline`, `content_stream` |
| 01-006 | H | The structure type and attributes of a structure element are not semantically appropriate for the content. | `human_review` |
| 01-007 | M | Suspects entry has a value of true. | `dict_entries` |

### Checkpoint 02: Role Mapping

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 02-001 | M | One or more non-standard tag's mapping does not terminate with a standard type. | `baseline`, `dict_entries`, `structure` |
| 02-002 | H | The mapping of one or more non-standard types is semantically inappropriate. | `human_review` |
| 02-003 | M | A circular mapping exists. | `structure` |
| 02-004 | M | One or more standard types are remapped. | `structure` |

### Checkpoint 03: Flickering

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 03-001 | H | One or more Actions lead to flickering. | `human_review` |
| 03-002 | H | One or more multimedia objects contain flickering content. | `human_review` |
| 03-003 | H | One or more JavaScript actions lead to flickering. | `human_review` |

### Checkpoint 04: Color and Contrast

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 04-001 | H | Information is conveyed by contrast, color, format or layout but the content is not tagged to reflect that meaning. | `human_review` |

### Checkpoint 05: Sound

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 05-001 | H | Media annotation present, but audio content not available in another form. | `human_review` |
| 05-002 | H | Audio annotation present, but content not available in another form. | `human_review` |
| 05-003 | H | JavaScript uses beep function but does not provide another means of notification. | `human_review` |

### Checkpoint 06: Metadata

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 06-001 | M | Document does not contain an XMP metadata stream. | `version` |
| 06-002 | M | The XMP metadata stream in the Catalog dictionary does not include the PDF/UA identifier. | `baseline`, `metadata`, `version` |
| 06-003 | M | XMP metadata stream does not contain dc:title. | `baseline`, `metadata` |
| 06-004 | H | dc:title does not clearly identify the document. | `human_review` |

### Checkpoint 07: Dictionary

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 07-001 | M | ViewerPreferences dictionary of the Catalog dictionary does not contain a DisplayDocTitle entry. | `baseline`, `metadata` |
| 07-002 | M | ViewerPreferences dictionary of the Catalog dictionary contains a DisplayDocTitle entry with a value of false. | `metadata` |

### Checkpoint 08: OCR Validation

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 08-001 | H | OCR-generated text contains significant errors. | `human_review` |
| 08-002 | H | OCR-generated text is not tagged. | `human_review` |

### Checkpoint 09: Appropriate Tags

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 09-001 | H | Tags are not in logical reading order. | `human_review` |
| 09-002 | H | Structure elements are nested in a semantically inappropriate manner. (e.g., a table inside a heading). | `human_review` |
| 09-003 | H | The structure type (after role mapping) of a structure element is not semantically appropriate. | `human_review` |
| 09-004 | M | A table-related structure element is used in a way that does not conform to the syntax defined in ISO 32000-1, Table 337. | `baseline`, `nesting`, `tables` |
| 09-005 | M | A list-related structure element is used in a way that does not conform to Table 336 in ISO 32000-1. | `baseline`, `lists`, `nesting` |
| 09-006 | M | A TOC-related structure element is used in a way that does not conform to Table 333 in ISO 32000-1. | `nesting` |
| 09-007 | M | A Ruby-related structure element is used in a way that does not conform to Table 338 in ISO 32000-1. | `nesting` |
| 09-008 | M | A Warichu-related structure element is used in a way that does not conform to Table 338 in ISO 32000-1. | `nesting` |

### Checkpoint 10: Character Mappings

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 10-001 | M | Character code cannot be mapped to Unicode. | `font_program` |

### Checkpoint 11: Declared Natural Language

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 11-001 | M | Natural language for text in page content cannot be determined. | `baseline`, `language`, `metadata` |
| 11-002 | M | Natural language for text in Alt, ActualText and E attributes cannot be determined. | `language` |
| 11-003 | M | Natural language in the Outline entries cannot be determined. | `language` |
| 11-004 | M | Natural language in the Contents entry for annotations cannot be determined. | `language` |
| 11-005 | M | Natural language in the TU entry for form fields cannot be determined. | `language` |
| 11-006 | M | Natural language for document metadata cannot be determined. | `language` |
| 11-007 | H | Natural language is not appropriate. | `human_review` |

### Checkpoint 12: Stretchable Characters

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 12-001 | H | Stretched characters are not represented appropriately. | `human_review` |

### Checkpoint 13: Graphics

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 13-001 | H | Graphics objects other than text objects and artifacts are not tagged with a `<Figure>` tag. | `human_review` |
| 13-002 | H | A link with a meaningful background does not include alternative text describing both the link and the graphic's purpose. | `human_review` |
| 13-003 | H | A caption is not tagged with a `<Caption>` tag. | `human_review` |
| 13-004 | M | `<Figure>` tag alternative or replacement text missing. | `baseline`, `images` |
| 13-005 | H | ActualText used for a `<Figure>` for which alternative text is more appropriate. | `human_review` |
| 13-006 | H | Graphics objects that possess semantic value only within a group of graphics objects is tagged on its own. | `human_review` |
| 13-007 | H | A more accessible representation is not used. | `human_review` |
| 13-008 | H | ActualText not present when a `<Figure>` is intended to be consumed primarily as text. | `human_review` |

### Checkpoint 14: Headings

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 14-001 | H | Headings are not tagged. | `human_review` |
| 14-002 | M | Does use numbered headings, but the first heading tag is not `<H1>`. | `headings` |
| 14-003 | M | Numbered heading levels in descending sequence are skipped (Example: `<H3>` follows directly after `<H1>`). | `baseline`, `headings` |
| 14-004 | H | Numbered heading tags do not use Arabic numerals and are not role mapped to heading types that do. | `human_review` |
| 14-005 | H | Content representing a 7th level (or higher) heading does not use an `<H7>` (or higher) tag. | `human_review` |
| 14-006 | M | A node contains more than one `<H>` tag. | `headings` |
| 14-007 | M | Document uses both `<H>` and `<H#>` tags. | `headings` |

### Checkpoint 15: Tables

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 15-001 | H | A row has a header cell, but that header cell is not tagged as a header. | `human_review` |
| 15-002 | H | A column has a header cell, but that header cell is not tagged as a header. | `human_review` |
| 15-003 | M | In a table not organized with Headers attributes and IDs, a `<TH>` cell does not contain a Scope attribute. | `baseline`, `tables` |
| 15-004 | H | Content is tagged as a table for information that is not organized in rows and columns. | `human_review` |
| 15-005 | H | A given cell's header cannot be unambiguously determined. | `human_review` |

### Checkpoint 16: Lists

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 16-001 | H | List is an ordered list, but no value for the ListNumbering attribute is present. | `human_review` |
| 16-002 | H | List is an ordered list, but the ListNumbering value is not Decimal, UpperRoman, LowerRoman, UpperAlpha or LowerAlpha. | `human_review` |
| 16-003 | H | Content is a list but is not tagged as a list. | `human_review` |

### Checkpoint 17: Mathematical Expressions

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 17-001 | H | Content is a mathematical expression but is not tagged with a `<Formula>` tag. | `human_review` |
| 17-002 | M | `<Formula>` tag is missing an Alt attribute. | `math` |
| 17-003 | M | Unicode mapping requirements are not met. | `font_program` |

### Checkpoint 18: Page Headers and Footers

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 18-001 | H | Headers and footers are not marked as pagination artifacts. | `human_review` |
| 18-002 | H | Header or footer artifacts are not classified as Header or Footer subtypes. | `human_review` |

### Checkpoint 19: Notes and References

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 19-001 | H | Footnotes or endnotes are not tagged as `<Note>`. | `human_review` |
| 19-002 | H | References are not tagged as `<Reference>`. | `human_review` |
| 19-003 | M | ID entry of the `<Note>` tag is not present. | `notes` |
| 19-004 | M | ID entry of the `<Note>` tag is non-unique. | `notes` |

### Checkpoint 20: Optional Content

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 20-001 | M | Name entry is missing or empty in an Optional Content Configuration Dictionary in the Configs array of OCProperties. | `optional_content` |
| 20-002 | M | Name entry is missing or empty in the Optional Content Configuration Dictionary that is the D entry of OCProperties. | `optional_content` |
| 20-003 | M | An AS entry appears in an Optional Content Configuration Dictionary. | `optional_content` |

### Checkpoint 21: Embedded Files

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 21-001 | M | The file specification dictionary for an embedded file does not contain F and UF entries. | `embedded_files` |

### Checkpoint 22: Article Threads

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 22-001 | H | Article threads do not reflect logical reading order. | `human_review` |

### Checkpoint 23: Digital Signatures

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 23-001 | – | No test specific to digital signatures is required; other provisions apply (form fields). | `human_review` (not applicable) |

### Checkpoint 24: Non-Interactive Forms

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 24-001 | H | Non-interactive forms are not tagged with the PrintFields attribute. | `human_review` |

### Checkpoint 25: XFA

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 25-001 | M | File contains the dynamicRender element with value "required". | `xfa` |

### Checkpoint 26: Security

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 26-001 | M | The file is encrypted but does not contain a P entry in its encryption dictionary. | `security` |
| 26-002 | M | The file is encrypted and does contain a P entry but the 10th bit position of the P entry is false. | `security` |

### Checkpoint 27: Navigation

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 27-001 | – | No tests specific to navigation are required; use appropriate semantics. | `human_review` (not applicable) |

### Checkpoint 28: Annotations

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 28-001 | H | An annotation is not in correct reading order. | `human_review` |
| 28-002 | M | An annotation, other than of subtype Widget, Link and PrinterMark, is not a direct child of an `<Annot>` structure element. | `annot_struct`, `baseline` |
| 28-003 | H | An annotation is used for visual formatting but is not tagged according to its semantic function. | `human_review` |
| 28-004 | M | An annotation, other than of subtype Widget, has neither a Contents entry nor an Alt entry on the enclosing structure element. | `annot_struct`, `baseline` |
| 28-005 | M | A form field has neither a TU entry nor an Alt entry on the enclosing structure element. | `annot_struct`, `baseline` |
| 28-006 | M | An annotation with subtype undefined in ISO 32000 does not meet 7.18.1. | `annot_struct` |
| 28-007 | M | An annotation of subtype TrapNet exists. | `annot_struct` |
| 28-008 | M | A page containing an annotation does not contain a Tabs entry. | `annotations` |
| 28-009 | M | A page containing an annotation has a Tabs entry with a value other than S. | `annotations` |
| 28-010 | M | A widget annotation is not nested within a `<Form>` tag. | `annot_struct`, `baseline` |
| 28-011 | M | A link annotation is not nested within a `<Link>` tag. | `annot_struct` |
| 28-012 | M | A link annotation does not include an alternate description in its Contents entry. | `annot_struct` |
| 28-013 | H | An IsMap entry is present with a value of true but the functionality is not provided in some other way. | `human_review` |
| 28-014 | M | CT entry is missing from the media clip data dictionary. | `annot_struct` |
| 28-015 | M | Alt entry is missing from the media clip data dictionary. | `annot_struct` |
| 28-016 | M | File attachment annotations do not conform to 7.11. | `annot_struct` |
| 28-017 | M | A PrinterMark annotation is included in the logical structure. | `annot_struct` |
| 28-018 | M | The appearance stream of a PrinterMark annotation is not marked as Artifact. | `annot_struct` |

### Checkpoint 29: Actions

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 29-001 | H | A script requires specific timing for individual keystrokes. | `human_review` |

### Checkpoint 30: XObjects

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 30-001 | M | A reference XObject is present. | `dict_entries` |
| 30-002 | M | Form XObject contains MCIDs and is referenced more than once. | `xobjects` |

### Checkpoint 31: Fonts

| Rule | How | Failure condition | Module |
|------|-----|-------------------|--------|
| 31-001 | M | A Type 0 font with a non-Identity encoding has different Registry values in the CIDFont and CMap CIDSystemInfo dictionaries. | `fonts` |
| 31-002 | M | A Type 0 font with a non-Identity encoding has different Ordering values in the CIDFont and CMap CIDSystemInfo dictionaries. | `fonts` |
| 31-003 | M | A Type 0 font with a non-Identity encoding has a CIDFont Supplement that is less than the CMap Supplement. | `fonts` |
| 31-004 | M | A Type 2 CID font contains neither a stream nor the name Identity as the value of the CIDToGIDMap entry. | `fonts` |
| 31-005 | M | A Type 2 CID font does not contain a CIDToGIDMap entry. | `fonts` |
| 31-006 | M | A CMap is neither a predefined CMap listed in ISO 32000-1 Table 118 nor embedded. | `fonts` |
| 31-007 | M | The WMode entry in a CMap dictionary is not identical to the WMode value in the CMap stream. | `fonts` |
| 31-008 | M | A CMap references another CMap which is not listed in ISO 32000-1:2008, 9.7.5.2, Table 118. | `fonts` |
| 31-009 | M | For a font used by text intended to be rendered the font program is not embedded. | `baseline`, `fonts` |
| 31-010 | H | A font program is embedded that is not legally embeddable for unlimited, universal rendering. | `human_review` |
| 31-011 | M | For a font used by text the font program is embedded but does not contain glyphs for all glyphs referenced for rendering. | `font_program` |
| 31-012 | M | An embedded Type 1 font has a CharSet string, but a glyph present in the font program is not listed in it. | `font_program` |
| 31-013 | M | An embedded Type 1 font has a CharSet string, but a glyph listed in it is not present in the font program. | `font_program` |
| 31-014 | M | An embedded CID font has a CIDSet stream, but a glyph present in the font program is not listed in it. | `font_program` |
| 31-015 | M | An embedded CID font has a CIDSet stream, but a glyph listed in it is not present in the font program. | `font_program` |
| 31-016 | M | For one or more glyphs, the width in the font dictionary and in the embedded font program differ by more than 1/1000 unit. | `font_program` |
| 31-017 | M | A non-symbolic TrueType font is used for rendering, but the embedded font program has no non-symbolic cmap subtable. | `font_program` |
| 31-018 | M | A non-symbolic TrueType font is used for rendering, but a rendered glyph cannot be looked up via any non-symbolic cmap subtable. | `font_program` |
| 31-019 | M | The font dictionary for a non-symbolic TrueType font does not contain an Encoding entry. | `fonts` |
| 31-020 | M | The font dictionary for a non-symbolic TrueType font has an Encoding dictionary without a BaseEncoding entry. | `fonts` |
| 31-021 | M | The Encoding or BaseEncoding of a non-symbolic TrueType font is neither MacRomanEncoding nor WinAnsiEncoding. | `fonts` |
| 31-022 | M | The Differences array of a non-symbolic TrueType font contains glyph names not listed in the Adobe Glyph List. | `fonts` |
| 31-023 | M | A non-symbolic TrueType font has a Differences array but the embedded font program has no (3,1) Microsoft Unicode cmap. | `font_program` |
| 31-024 | M | The Encoding entry is present in the font dictionary for a symbolic TrueType font. | `font_program` |
| 31-025 | M | The embedded font program for a symbolic TrueType font contains no cmap. | `font_program` |
| 31-026 | M | The embedded font program for a symbolic TrueType font has more than one cmap subtable but none is a (3,0) Microsoft Symbol cmap. | `font_program` |
| 31-027 | M | A font dictionary has no ToUnicode entry and no other permitted mechanism (standard encoding, AGL glyph names, Adobe CJK collection, non-symbolic TrueType) provides Unicode mapping. | `baseline`, `font_program` |
| 31-028 | M | One or more Unicode values specified in the ToUnicode CMap are zero (0). | `font_program` |
| 31-029 | M | One or more Unicode values specified in the ToUnicode CMap are equal to either U+FEFF or U+FFFE. | `font_program` |
| 31-030 | M | One or more characters used in text showing operators reference the .notdef glyph. | `font_program` |

## Horn extension rules

These ids do not correspond to a published Matterhorn condition. They catch PDF/UA prerequisites (a file that is not a tagged PDF cannot be PDF/UA), strong indicators of broken structure, and a few heuristics inherited from `pdf_oxide`.

| Rule | Description | Module |
|------|-------------|--------|
| 00-x01 | File header is not `%PDF-1.0` … `%PDF-1.7` or `%PDF-2.0` | `file_syntax` |
| 00-x02 | Non-whitespace bytes follow the final `%%EOF` | `file_syntax` |
| 01-x01 | `/MarkInfo` missing or `/Marked` not true (document is not a tagged PDF) | `baseline`, `structure` |
| 01-x02 | `/StructTreeRoot` missing or empty | `structure` |
| 01-x03 | `/ParentTree` missing or incomplete (a page `/StructParents` has no entry) | `dict_entries` |
| 13-x01 | Decorative graphic not marked as artifact (pdf_oxide heuristic) | `baseline` |
| 13-x02 | Figure caption not associated with its figure (pdf_oxide heuristic) | `baseline` |
| 13-x03 | Figure text lacks ActualText (pdf_oxide heuristic) | `baseline` |
| 15-x01 | Table has no `TH` header cells at all | `baseline`, `tables` |
| 15-x02 | Complex table without `/Headers`, `/Scope` or `THead` for header association | `baseline`, `tables` |
| 15-x03 | `RowSpan`/`ColSpan` values invalid or exceeding the table dimensions | `tables` |
| 15-x04 | Table rows do not span the same number of columns after `RowSpan`/`ColSpan` (irregular table, veraPDF 7.2-42/43) | `tables` |
| 20-x01 | Optional content group without a `/Name` | `optional_content` |
| 26-x01 | Encryption `/P` bit 5 (copy/extract) not set | `security` |
| 28-x01 | Link annotation has neither `/A` (action) nor `/Dest` | `annotations`, `baseline` |
| 28-x02 | Link text is not descriptive (pdf_oxide heuristic) | `baseline` |
| 28-x03 | Form field has no `/T` field name (pdf_oxide) | `baseline` |
| 28-x04 | Required form field not indicated (pdf_oxide) | `baseline` |
| 28-x05 | Form has no submit button (pdf_oxide) | `baseline` |
| 31-x01 | `/CIDSystemInfo` missing or incomplete (Registry/Ordering/Supplement) | `fonts` |
| 31-x02 | `/CIDSet` is not a valid stream | `fonts` |
| 31-x04 | `/Differences` array maps a code to `.notdef` | `fonts` |

## PDF/UA-2 (ISO 14289-2) rules

PDF/UA-2 is checked against ISO 14289-2:2024 directly: the Matterhorn Protocol 2.0 for PDF/UA-2 has not been published yet, so there is no official failure-condition index to cite. These rules use **interim ids of the form `ua2:<clause>-<test>`** (the ISO 14289-2 clause and a test number), which mirror the veraPDF PDF/UA-2 validation profile that PAC 2024 also follows; they will be re-mapped once Matterhorn 2.0 is released. They run only for documents whose XMP declares `pdfuaid:part` 2 (see `horn coverage` for the same list from your installed binary).

Requirements that PDF/UA-2 shares with PDF/UA-1 are reported under their Matterhorn 1.1 index by the regular checks. For PDF/UA-2 documents a few of those checks follow PDF 2.0 semantics:

- role mapping is resolved through PDF 2.0 namespaces (`/NS`, `/RoleMapNS`); types in the PDF 2.0 and MathML namespaces are never role mapped, and the `Artifact` structure type is accepted (02-001);
- an annotation enclosed in an `Artifact` structure element is an artifact and is exempt from the parent-type and accessible-text rules (28-002, 28-004, 28-010, 28-011, 28-012);
- a link annotation may be enclosed in a `Reference` element (28-011) and needs no `/Contents` when its `Link`/`Reference` element carries content or `/Alt` (28-012);
- pages with annotations may use `/Tabs /A` or `/W` as well as `/S` (28-009).

Horn validates **138/138** files of the veraPDF PDF/UA-2 corpus as expected (54/54 pass files compliant, 84/84 fail files detected).

| Rule | Clause | Failure condition | Module |
|------|--------|-------------------|--------|
| `ua2:5-3` | 5 | The 'part' property of the PDF/UA identification schema does not use the 'pdfuaid' namespace prefix. | `ua2/identification` |
| `ua2:5-4` | 5 | The 'rev' property of the PDF/UA identification schema does not use the 'pdfuaid' namespace prefix. | `ua2/identification` |
| `ua2:5-5` | 5 | pdfuaid:rev is missing or its value is not the four-digit year 2024. | `ua2/identification` |
| `ua2:8.2.4-2` | 8.2.4 | A circular role mapping exists in a namespace RoleMapNS. | `ua2/structure` |
| `ua2:8.2.4-3` | 8.2.4 | A structure type is role mapped to another type within the same namespace. | `ua2/structure` |
| `ua2:8.2.5.2-1` | 8.2.5.2 | The structure tree root does not contain a single Document element as its only child. | `ua2/structure` |
| `ua2:8.2.5.2-2` | 8.2.5.2 | The Document element is not in the PDF 2.0 namespace (http://iso.org/pdf2/ssn). | `ua2/structure` |
| `ua2:8.2.5.8-1` | 8.2.5.8 | A TOCI element has no Ref entry identifying the content it refers to. | `ua2/structure` |
| `ua2:8.2.5.12-1` | 8.2.5.12 | The generic H structure type is used; PDF/UA-2 requires numbered headings (Hn). | `ua2/structure` |
| `ua2:8.2.5.14-1` | 8.2.5.14 | The Note structure type is used; PDF 2.0 deprecates Note in favour of FENote. | `ua2/structure` |
| `ua2:8.2.5.14-4` | 8.2.5.14 | A FENote element has a NoteType attribute other than Footnote, Endnote or None. | `ua2/structure` |
| `ua2:8.2.5.25-1` | 8.2.5.25 | A list whose items contain Lbl elements has no ListNumbering attribute, or ListNumbering is None. | `ua2/structure` |
| `ua2:8.2.5.25-2` | 8.2.5.25 | An LI element contains content that is not enclosed in a Lbl or LBody element. | `ua2/structure` |
| `ua2:8.2.5.20-2` | 8.2.5.20 | Link annotations enclosed in the same Link or Reference element target different locations. | `ua2/annotations` |
| `ua2:8.2.5.29-1` | 8.2.5.29 | A MathML structure element is not a child of a Formula element (or of another MathML element). | `ua2/structure` |
| `ua2:8.4.3-2` | 8.4.3 | An ActualText entry contains Unicode private-use-area code points. | `ua2/structure` |
| `ua2:8.4.3-3` | 8.4.3 | An Alt entry contains Unicode private-use-area code points. | `ua2/structure` |
| `ua2:8.8-1` | 8.8 | An intra-document destination (outline item, link, GoTo action) is not a structure destination. | `ua2/destinations` |
| `ua2:8.8-2` | 8.8 | A GoTo action has no structure destination (SD entry). | `ua2/destinations` |
| `ua2:8.9.2.2-1` | 8.9.2.2 | An annotation with the Invisible flag is included in the logical structure and is not an artifact. | `ua2/annotations` |
| `ua2:8.9.2.2-2` | 8.9.2.2 | An annotation with the NoView flag (and without ToggleNoView) is included in the logical structure and is not an artifact. | `ua2/annotations` |
| `ua2:8.9.2.4.7-1` | 8.9.2.4.7 | A rubber stamp annotation has neither a Name nor a Contents entry. | `ua2/annotations` |
| `ua2:8.9.2.4.8-1` | 8.9.2.4.8 | An Ink annotation has no Contents entry. | `ua2/annotations` |
| `ua2:8.9.2.4.9-1` | 8.9.2.4.9 | A Popup annotation is included in the logical structure. | `ua2/annotations` |
| `ua2:8.9.2.4.10-1` | 8.9.2.4.10 | The file specification of a file attachment annotation has no AFRelationship entry. | `ua2/annotations` |
| `ua2:8.9.2.4.11-1` | 8.9.2.4.11 | A Sound annotation is present (deprecated in PDF 2.0, not permitted in PDF/UA-2). | `ua2/annotations` |
| `ua2:8.9.2.4.11-2` | 8.9.2.4.11 | A Movie annotation is present (deprecated in PDF 2.0, not permitted in PDF/UA-2). | `ua2/annotations` |
| `ua2:8.9.2.4.12-1` | 8.9.2.4.12 | A Screen annotation has no Contents entry. | `ua2/annotations` |
| `ua2:8.9.2.4.13-1` | 8.9.2.4.13 | A zero-size Widget annotation is included in the logical structure and is not an artifact. | `ua2/annotations` |
| `ua2:8.9.2.4.19-1` | 8.9.2.4.19 | A 3D annotation has no Contents entry. | `ua2/annotations` |
| `ua2:8.9.2.4.19-2` | 8.9.2.4.19 | A RichMedia annotation has no Contents entry. | `ua2/annotations` |
| `ua2:8.9.4.2-1` | 8.9.4.2 | An annotation's Contents entry differs from the Alt entry of its enclosing structure element. | `ua2/annotations` |
| `ua2:8.10.1-2` | 8.10.1 | A Form structure element contains more than one widget annotation. | `ua2/annotations` |
| `ua2:8.10.2.3-1` | 8.10.2.3 | A form field widget has neither a Lbl element in its Form structure element nor a Contents entry. | `ua2/annotations` |
| `ua2:8.10.2.3-2` | 8.10.2.3 | A form field widget with additional actions (AA) has no Contents entry. | `ua2/annotations` |
| `ua2:8.11.1-2` | 8.11.1 | The Metadata stream in the catalog lacks /Type /Metadata and /Subtype /XML. | `ua2/identification` |
| `ua2:8.14.1-1` | 8.14.1 | A file specification in the EmbeddedFiles name tree has no Desc entry. | `ua2/identification` |

## Notes on specific conditions

- **31-003 (Supplement)** — the protocol text says the CIDFont Supplement must not be *less* than the CMap Supplement; ISO 14289-1 7.21.3.1 and the veraPDF corpus require it to be *less than or equal*. Horn follows ISO 14289-1: a CIDFont Supplement greater than the CMap Supplement fails.
- **15-x04 (irregular tables)** — Matterhorn 01-006 NOTE 3 only suggests a warning for irregular tables, but the veraPDF PDF/UA-1 corpus (7.2-42, 7.2-43) treats them as failures, and header/data association cannot be determined for them. Horn reports them as failures.
- **17-003** — the Unicode requirements for `<Formula>` content are the general 10-001 / 31-027 requirements; Horn reports 17-003 when a document with `<Formula>` elements has a font that fails them.
- **28-006** — annotations with a subtype not defined in ISO 32000 are validated like any other annotation (28-002, 28-004); a failure is additionally reported under 28-006.
- **31-009 / 31-011 / 31-016 / 31-018** apply to glyphs *used for rendering*: text shown only in rendering mode 3 (invisible, e.g. OCR layers) is exempt, as required by the protocol.
- **PDF/UA-2 rules** follow the veraPDF PDF/UA-2 profile where ISO 14289-2 leaves room for interpretation: a `TOCI` without `/Ref` (`ua2:8.2.5.8-1`) and the deprecated `Note` type (`ua2:8.2.5.14-1`) are reported as failures, and structure destinations are required for every outline item, link and GoTo action (`ua2:8.8-1`, `ua2:8.8-2`).
- **Manual review items** are only emitted when the document contains the feature the condition is about (e.g. 15-001/15-002 only for documents with tables, 03-003 only when JavaScript is present, 31-010 always for embedded fonts, with a warning when the OS/2 `fsType` declares restricted embedding).

## Check modules

Run `horn list-checks` to see the registered modules of your installed version.

| Module | Checkpoint(s) | Scope |
|--------|---------------|-------|
| `file_syntax` | — | PDF header and end-of-file marker (extension) (2 rules) |
| `baseline` | various | `pdf_oxide` built-in PDF/UA-1 validator, mapped to Matterhorn ids (33 rules) |
| `structure` | 01, 02 | Tagged PDF prerequisites, role map termination, cycles and remapping (5 rules) |
| `dict_entries` | 01, 02, 30 | ParentTree, Suspects, unmapped types, reference XObjects (4 rules) |
| `content_stream` | 01 | Marked content: untagged content, artifact nesting (3 rules) |
| `version` | 06 | PDF/UA identification schema in XMP (2 rules) |
| `metadata` | 06, 07, 11 | XMP identifier and title, DisplayDocTitle, catalog language (5 rules) |
| `language` | 11 | BCP 47 tags and language context (6 rules) |
| `nesting` | 09 | Table, list, TOC, Ruby and Warichu nesting rules (5 rules) |
| `headings` | 14 | Heading hierarchy (4 rules) |
| `tables` | 09, 15 | Table structure, Scope, spans and grid regularity (6 rules) |
| `lists` | 09 | List structure (1 rules) |
| `images` | 13 | Figure alternative text (1 rules) |
| `math` | 17 | Formula alternative text (1 rules) |
| `notes` | 19 | Note IDs (2 rules) |
| `optional_content` | 20 | Optional content configurations (4 rules) |
| `embedded_files` | 21 | Embedded file specifications (1 rules) |
| `xfa` | 25 | Dynamic XFA (1 rules) |
| `security` | 26 | Encryption permissions (3 rules) |
| `annotations` | 28 | Tab order, link destinations (3 rules) |
| `annot_struct` | 28 | Annotation ↔ structure tree association, media clips, PrinterMark (13 rules) |
| `xobjects` | 30 | Structured Form XObjects painted once (1 rules) |
| `fonts` | 31 | Font embedding, composite-font CMaps, CIDToGIDMap, simple-font encodings (16 rules) |
| `font_program` | 10, 17, 31 | Embedded font programs: glyph coverage, CharSet/CIDSet, widths, TrueType cmaps, ToUnicode (18 rules) |
| `ua2/identification` | 06, 21 | PDF/UA-2: identification schema (part/rev), Metadata stream, embedded-file descriptions (5 rules) |
| `ua2/structure` | 02, 09, 10, 14, 16, 17, 19 | PDF/UA-2: PDF 2.0 namespaces, Document root, headings, lists, notes, MathML, private-use text (13 rules) |
| `ua2/annotations` | 28 | PDF/UA-2: annotation artifacts, annotation types, Contents/Alt, form widgets (17 rules) |
| `ua2/destinations` | 27, 29 | PDF/UA-2: structure destinations for outlines, links and GoTo actions (2 rules) |
| `human_review` | all | Manual-review items for the 48 human-judgment conditions (48 rules) |
