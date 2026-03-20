# PDF/UA-1 Reference Suite 1.1

**Source:** <https://pdfa.org/resource/pdfua-reference-suite/>

These PDF documents are gold-standard PDF/UA-1 (ISO 14289-1) conformance files.
The 9 PDFUA-Ref files are the official reference suite from the PDF Association's
PDF/UA Competence Center, following the Tagged PDF Best Practice Guide: Syntax.
The Matterhorn Protocol PDF is itself a reference-quality PDF/UA file.

All files must pass horn validation with zero errors.

## Files

| File | Type | Pages | Lang |
|------|------|-------|------|
| PDFUA-Ref-2-01_Magazine-danish.pdf | Magazine | 127 | da |
| PDFUA-Ref-2-02_Invoice.pdf | Invoice | 1 | en |
| PDFUA-Ref-2-03_AcademicAbstract.pdf | Academic abstract | 3 | en |
| PDFUA-Ref-2-04_Presentation.pdf | Presentation | 13 | en |
| PDFUA-Ref-2-05_BookChapter-german.pdf | Book chapter | 8 | de |
| PDFUA-Ref-2-06_Brochure.pdf | Brochure | 17 | en |
| PDFUA-Ref-2-08_BookChapter.pdf | Book chapter | 23 | en |
| PDFUA-Ref-2-09_Scanned.pdf | Scanned (OCR) | 104 | en |
| PDFUA-Ref-2-10_Form.pdf | Interactive form | 1 | en |
| Matterhorn-Protocol-1-1.pdf | Matterhorn Protocol spec | 10 | en |

## Notable PDF features exercised

- Number tree `/Kids` hierarchies in ParentTree (large documents)
- Form field `/Parent` inheritance for `/T` and `/TU` attributes
- `xmlns:pdfuaid` namespace declarations (without `pdfaExtension:schemas`)
- `THead`/`TBody` table structure with `/Scope` attributes on TH cells
- Multiple languages (Danish, German, English)
- Scanned/OCR content with proper tagging
- Complex layouts (magazines, brochures, presentations)

## License

These files are provided by the PDF Association under Creative Commons
Attribution 4.0 International (CC BY 4.0).
