//! The Matterhorn Protocol 1.1 failure-condition catalogue.
//!
//! Every one of the 136 failure conditions (31 checkpoints) published by the PDF
//! Association in *Matterhorn Protocol 1.1 — PDF/UA Conformance Testing Model*
//! (2021-04, CC-BY-4.0) is listed here with its official index, PDF/UA-1 clause,
//! and whether it is machine-checkable (`How::Machine`) or requires human
//! judgment (`How::Human`).
//!
//! Horn's checks emit results whose `rule_id` is the official Matterhorn index
//! (e.g. `"28-010"`). Additional checks that go beyond the protocol use ids of
//! the form `"NN-xNN"` (see [`is_extension_rule`]).

use serde::Serialize;

/// How a failure condition is normally decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum How {
    /// Can be determined by software alone.
    Machine,
    /// Usually requires human judgment.
    Human,
    /// No specific test is defined (23-001, 27-001).
    None,
}

/// One Matterhorn failure condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Condition {
    /// Official index, e.g. `"14-003"`.
    pub id: &'static str,
    /// Checkpoint number (1..=31).
    pub checkpoint: u8,
    /// Checkpoint title, e.g. `"Headings"`.
    pub checkpoint_name: &'static str,
    /// Failure condition as worded (or condensed) from the protocol.
    pub description: &'static str,
    /// Referenced PDF/UA-1 clause, e.g. `"UA1:7.4-1"`.
    pub section: &'static str,
    /// Machine or human.
    pub how: How,
}

/// Human-readable checkpoint titles, indexed by checkpoint number (index 0 unused).
pub const CHECKPOINT_NAMES: [&str; 32] = [
    "",
    "Real content tagged",
    "Role Mapping",
    "Flickering",
    "Color and Contrast",
    "Sound",
    "Metadata",
    "Dictionary",
    "OCR Validation",
    "Appropriate Tags",
    "Character Mappings",
    "Declared Natural Language",
    "Stretchable Characters",
    "Graphics",
    "Headings",
    "Tables",
    "Lists",
    "Mathematical Expressions",
    "Page Headers and Footers",
    "Notes and References",
    "Optional Content",
    "Embedded Files",
    "Article Threads",
    "Digital Signatures",
    "Non-Interactive Forms",
    "XFA",
    "Security",
    "Navigation",
    "Annotations",
    "Actions",
    "XObjects",
    "Fonts",
];

/// All 137 failure conditions of Matterhorn Protocol 1.1, in protocol order.
#[rustfmt::skip]
pub static CONDITIONS: [Condition; 137] = [
    Condition { id: "01-001", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-1", how: How::Human,
        description: "Artifact is tagged as real content." },
    Condition { id: "01-002", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-1", how: How::Human,
        description: "Real content is marked as artifact." },
    Condition { id: "01-003", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-1", how: How::Machine,
        description: "Content marked as Artifact is present inside tagged content." },
    Condition { id: "01-004", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-1", how: How::Machine,
        description: "Tagged content is present inside content marked as Artifact." },
    Condition { id: "01-005", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-2", how: How::Machine,
        description: "Content is neither marked as Artifact nor tagged as real content." },
    Condition { id: "01-006", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-2", how: How::Human,
        description: "The structure type and attributes of a structure element are not semantically appropriate for the content." },
    Condition { id: "01-007", checkpoint: 1, checkpoint_name: "Real content tagged", section: "UA1:7.1-11", how: How::Machine,
        description: "Suspects entry has a value of true." },
    Condition { id: "02-001", checkpoint: 2, checkpoint_name: "Role Mapping", section: "UA1:7.1-3", how: How::Machine,
        description: "One or more non-standard tag's mapping does not terminate with a standard type." },
    Condition { id: "02-002", checkpoint: 2, checkpoint_name: "Role Mapping", section: "UA1:7.1-3", how: How::Human,
        description: "The mapping of one or more non-standard types is semantically inappropriate." },
    Condition { id: "02-003", checkpoint: 2, checkpoint_name: "Role Mapping", section: "UA1:7.1-3", how: How::Machine,
        description: "A circular mapping exists." },
    Condition { id: "02-004", checkpoint: 2, checkpoint_name: "Role Mapping", section: "UA1:7.1-4", how: How::Machine,
        description: "One or more standard types are remapped." },
    Condition { id: "03-001", checkpoint: 3, checkpoint_name: "Flickering", section: "UA1:7.1-5", how: How::Human,
        description: "One or more Actions lead to flickering." },
    Condition { id: "03-002", checkpoint: 3, checkpoint_name: "Flickering", section: "UA1:7.1-5", how: How::Human,
        description: "One or more multimedia objects contain flickering content." },
    Condition { id: "03-003", checkpoint: 3, checkpoint_name: "Flickering", section: "UA1:7.1-5", how: How::Human,
        description: "One or more JavaScript actions lead to flickering." },
    Condition { id: "04-001", checkpoint: 4, checkpoint_name: "Color and Contrast", section: "UA1:7.1-6", how: How::Human,
        description: "Information is conveyed by contrast, color, format or layout but the content is not tagged to reflect that meaning." },
    Condition { id: "05-001", checkpoint: 5, checkpoint_name: "Sound", section: "UA1:7.1-7", how: How::Human,
        description: "Media annotation present, but audio content not available in another form." },
    Condition { id: "05-002", checkpoint: 5, checkpoint_name: "Sound", section: "UA1:7.1-7", how: How::Human,
        description: "Audio annotation present, but content not available in another form." },
    Condition { id: "05-003", checkpoint: 5, checkpoint_name: "Sound", section: "UA1:7.1-7", how: How::Human,
        description: "JavaScript uses beep function but does not provide another means of notification." },
    Condition { id: "06-001", checkpoint: 6, checkpoint_name: "Metadata", section: "UA1:7.1-8", how: How::Machine,
        description: "Document does not contain an XMP metadata stream." },
    Condition { id: "06-002", checkpoint: 6, checkpoint_name: "Metadata", section: "UA1:5", how: How::Machine,
        description: "The XMP metadata stream in the Catalog dictionary does not include the PDF/UA identifier." },
    Condition { id: "06-003", checkpoint: 6, checkpoint_name: "Metadata", section: "UA1:7.1-8", how: How::Machine,
        description: "XMP metadata stream does not contain dc:title." },
    Condition { id: "06-004", checkpoint: 6, checkpoint_name: "Metadata", section: "UA1:7.1-8", how: How::Human,
        description: "dc:title does not clearly identify the document." },
    Condition { id: "07-001", checkpoint: 7, checkpoint_name: "Dictionary", section: "UA1:7.1-9", how: How::Machine,
        description: "ViewerPreferences dictionary of the Catalog dictionary does not contain a DisplayDocTitle entry." },
    Condition { id: "07-002", checkpoint: 7, checkpoint_name: "Dictionary", section: "UA1:7.1-9", how: How::Machine,
        description: "ViewerPreferences dictionary of the Catalog dictionary contains a DisplayDocTitle entry with a value of false." },
    Condition { id: "08-001", checkpoint: 8, checkpoint_name: "OCR Validation", section: "UA1:7.1-10", how: How::Human,
        description: "OCR-generated text contains significant errors." },
    Condition { id: "08-002", checkpoint: 8, checkpoint_name: "OCR Validation", section: "UA1:7.1-10", how: How::Human,
        description: "OCR-generated text is not tagged." },
    Condition { id: "09-001", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Human,
        description: "Tags are not in logical reading order." },
    Condition { id: "09-002", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Human,
        description: "Structure elements are nested in a semantically inappropriate manner. (e.g., a table inside a heading)." },
    Condition { id: "09-003", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Human,
        description: "The structure type (after role mapping) of a structure element is not semantically appropriate." },
    Condition { id: "09-004", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Machine,
        description: "A table-related structure element is used in a way that does not conform to the syntax defined in ISO 32000-1, Table 337." },
    Condition { id: "09-005", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Machine,
        description: "A list-related structure element is used in a way that does not conform to Table 336 in ISO 32000-1." },
    Condition { id: "09-006", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Machine,
        description: "A TOC-related structure element is used in a way that does not conform to Table 333 in ISO 32000-1." },
    Condition { id: "09-007", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Machine,
        description: "A Ruby-related structure element is used in a way that does not conform to Table 338 in ISO 32000-1." },
    Condition { id: "09-008", checkpoint: 9, checkpoint_name: "Appropriate Tags", section: "UA1:7.2-1", how: How::Machine,
        description: "A Warichu-related structure element is used in a way that does not conform to Table 338 in ISO 32000-1." },
    Condition { id: "10-001", checkpoint: 10, checkpoint_name: "Character Mappings", section: "UA1:7.2-2", how: How::Machine,
        description: "Character code cannot be mapped to Unicode." },
    Condition { id: "11-001", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Machine,
        description: "Natural language for text in page content cannot be determined." },
    Condition { id: "11-002", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Machine,
        description: "Natural language for text in Alt, ActualText and E attributes cannot be determined." },
    Condition { id: "11-003", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Machine,
        description: "Natural language in the Outline entries cannot be determined." },
    Condition { id: "11-004", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Machine,
        description: "Natural language in the Contents entry for annotations cannot be determined." },
    Condition { id: "11-005", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Machine,
        description: "Natural language in the TU entry for form fields cannot be determined." },
    Condition { id: "11-006", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Machine,
        description: "Natural language for document metadata cannot be determined." },
    Condition { id: "11-007", checkpoint: 11, checkpoint_name: "Declared Natural Language", section: "UA1:7.2-3", how: How::Human,
        description: "Natural language is not appropriate." },
    Condition { id: "12-001", checkpoint: 12, checkpoint_name: "Stretchable Characters", section: "UA1:7.2-4", how: How::Human,
        description: "Stretched characters are not represented appropriately." },
    Condition { id: "13-001", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-1", how: How::Human,
        description: "Graphics objects other than text objects and artifacts are not tagged with a <Figure> tag." },
    Condition { id: "13-002", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-1", how: How::Human,
        description: "A link with a meaningful background does not include alternative text describing both the link and the graphic's purpose." },
    Condition { id: "13-003", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-2", how: How::Human,
        description: "A caption is not tagged with a <Caption> tag." },
    Condition { id: "13-004", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-3", how: How::Machine,
        description: "<Figure> tag alternative or replacement text missing." },
    Condition { id: "13-005", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-4", how: How::Human,
        description: "ActualText used for a <Figure> for which alternative text is more appropriate." },
    Condition { id: "13-006", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-5", how: How::Human,
        description: "Graphics objects that possess semantic value only within a group of graphics objects is tagged on its own." },
    Condition { id: "13-007", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-6", how: How::Human,
        description: "A more accessible representation is not used." },
    Condition { id: "13-008", checkpoint: 13, checkpoint_name: "Graphics", section: "UA1:7.3-4", how: How::Human,
        description: "ActualText not present when a <Figure> is intended to be consumed primarily as text." },
    Condition { id: "14-001", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4-1", how: How::Human,
        description: "Headings are not tagged." },
    Condition { id: "14-002", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4.2-1", how: How::Machine,
        description: "Does use numbered headings, but the first heading tag is not <H1>." },
    Condition { id: "14-003", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4-1", how: How::Machine,
        description: "Numbered heading levels in descending sequence are skipped (Example: <H3> follows directly after <H1>)." },
    Condition { id: "14-004", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4.3-1", how: How::Human,
        description: "Numbered heading tags do not use Arabic numerals and are not role mapped to heading types that do." },
    Condition { id: "14-005", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4.3-1", how: How::Human,
        description: "Content representing a 7th level (or higher) heading does not use an <H7> (or higher) tag." },
    Condition { id: "14-006", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4.4-1", how: How::Machine,
        description: "A node contains more than one <H> tag." },
    Condition { id: "14-007", checkpoint: 14, checkpoint_name: "Headings", section: "UA1:7.4.4-3", how: How::Machine,
        description: "Document uses both <H> and <H#> tags." },
    Condition { id: "15-001", checkpoint: 15, checkpoint_name: "Tables", section: "UA1:7.5-1", how: How::Human,
        description: "A row has a header cell, but that header cell is not tagged as a header." },
    Condition { id: "15-002", checkpoint: 15, checkpoint_name: "Tables", section: "UA1:7.5-1", how: How::Human,
        description: "A column has a header cell, but that header cell is not tagged as a header." },
    Condition { id: "15-003", checkpoint: 15, checkpoint_name: "Tables", section: "UA1:7.5-2", how: How::Machine,
        description: "In a table not organized with Headers attributes and IDs, a <TH> cell does not contain a Scope attribute." },
    Condition { id: "15-004", checkpoint: 15, checkpoint_name: "Tables", section: "UA1:7.5-3", how: How::Human,
        description: "Content is tagged as a table for information that is not organized in rows and columns." },
    Condition { id: "15-005", checkpoint: 15, checkpoint_name: "Tables", section: "UA1:7.5-2", how: How::Human,
        description: "A given cell's header cannot be unambiguously determined." },
    Condition { id: "16-001", checkpoint: 16, checkpoint_name: "Lists", section: "UA1:7.6-1", how: How::Human,
        description: "List is an ordered list, but no value for the ListNumbering attribute is present." },
    Condition { id: "16-002", checkpoint: 16, checkpoint_name: "Lists", section: "UA1:7.6-1", how: How::Human,
        description: "List is an ordered list, but the ListNumbering value is not Decimal, UpperRoman, LowerRoman, UpperAlpha or LowerAlpha." },
    Condition { id: "16-003", checkpoint: 16, checkpoint_name: "Lists", section: "UA1:7.6-2", how: How::Human,
        description: "Content is a list but is not tagged as a list." },
    Condition { id: "17-001", checkpoint: 17, checkpoint_name: "Mathematical Expressions", section: "UA1:7.7-1", how: How::Human,
        description: "Content is a mathematical expression but is not tagged with a <Formula> tag." },
    Condition { id: "17-002", checkpoint: 17, checkpoint_name: "Mathematical Expressions", section: "UA1:7.7-1", how: How::Machine,
        description: "<Formula> tag is missing an Alt attribute." },
    Condition { id: "17-003", checkpoint: 17, checkpoint_name: "Mathematical Expressions", section: "UA1:7.7-2", how: How::Machine,
        description: "Unicode mapping requirements are not met." },
    Condition { id: "18-001", checkpoint: 18, checkpoint_name: "Page Headers and Footers", section: "UA1:7.8-1", how: How::Human,
        description: "Headers and footers are not marked as pagination artifacts." },
    Condition { id: "18-002", checkpoint: 18, checkpoint_name: "Page Headers and Footers", section: "UA1:7.8-1", how: How::Human,
        description: "Header or footer artifacts are not classified as Header or Footer subtypes." },
    Condition { id: "19-001", checkpoint: 19, checkpoint_name: "Notes and References", section: "UA1:7.9-1", how: How::Human,
        description: "Footnotes or endnotes are not tagged as <Note>." },
    Condition { id: "19-002", checkpoint: 19, checkpoint_name: "Notes and References", section: "UA1:7.9-1", how: How::Human,
        description: "References are not tagged as <Reference>." },
    Condition { id: "19-003", checkpoint: 19, checkpoint_name: "Notes and References", section: "UA1:7.9-2", how: How::Machine,
        description: "ID entry of the <Note> tag is not present." },
    Condition { id: "19-004", checkpoint: 19, checkpoint_name: "Notes and References", section: "UA1:7.9-2", how: How::Machine,
        description: "ID entry of the <Note> tag is non-unique." },
    Condition { id: "20-001", checkpoint: 20, checkpoint_name: "Optional Content", section: "UA1:7.10-1", how: How::Machine,
        description: "Name entry is missing or empty in an Optional Content Configuration Dictionary in the Configs array of OCProperties." },
    Condition { id: "20-002", checkpoint: 20, checkpoint_name: "Optional Content", section: "UA1:7.10-1", how: How::Machine,
        description: "Name entry is missing or empty in the Optional Content Configuration Dictionary that is the D entry of OCProperties." },
    Condition { id: "20-003", checkpoint: 20, checkpoint_name: "Optional Content", section: "UA1:7.10-2", how: How::Machine,
        description: "An AS entry appears in an Optional Content Configuration Dictionary." },
    Condition { id: "21-001", checkpoint: 21, checkpoint_name: "Embedded Files", section: "UA1:7.11-1", how: How::Machine,
        description: "The file specification dictionary for an embedded file does not contain F and UF entries." },
    Condition { id: "22-001", checkpoint: 22, checkpoint_name: "Article Threads", section: "UA1:7.12-1", how: How::Human,
        description: "Article threads do not reflect logical reading order." },
    Condition { id: "23-001", checkpoint: 23, checkpoint_name: "Digital Signatures", section: "UA1:7.13-1", how: How::None,
        description: "No test specific to digital signatures is required; other provisions apply (form fields)." },
    Condition { id: "24-001", checkpoint: 24, checkpoint_name: "Non-Interactive Forms", section: "UA1:7.14-1", how: How::Human,
        description: "Non-interactive forms are not tagged with the PrintFields attribute." },
    Condition { id: "25-001", checkpoint: 25, checkpoint_name: "XFA", section: "UA1:7.15-1", how: How::Machine,
        description: "File contains the dynamicRender element with value \"required\"." },
    Condition { id: "26-001", checkpoint: 26, checkpoint_name: "Security", section: "UA1:7.16-1", how: How::Machine,
        description: "The file is encrypted but does not contain a P entry in its encryption dictionary." },
    Condition { id: "26-002", checkpoint: 26, checkpoint_name: "Security", section: "UA1:7.16-1", how: How::Machine,
        description: "The file is encrypted and does contain a P entry but the 10th bit position of the P entry is false." },
    Condition { id: "27-001", checkpoint: 27, checkpoint_name: "Navigation", section: "UA1:7.17-1", how: How::None,
        description: "No tests specific to navigation are required; use appropriate semantics." },
    Condition { id: "28-001", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.1-2", how: How::Human,
        description: "An annotation is not in correct reading order." },
    Condition { id: "28-002", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.1-2", how: How::Machine,
        description: "An annotation, other than of subtype Widget, Link and PrinterMark, is not a direct child of an <Annot> structure element." },
    Condition { id: "28-003", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.1-3", how: How::Human,
        description: "An annotation is used for visual formatting but is not tagged according to its semantic function." },
    Condition { id: "28-004", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.1-4", how: How::Machine,
        description: "An annotation, other than of subtype Widget, has neither a Contents entry nor an Alt entry on the enclosing structure element." },
    Condition { id: "28-005", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.1-4", how: How::Machine,
        description: "A form field has neither a TU entry nor an Alt entry on the enclosing structure element." },
    Condition { id: "28-006", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.2-1", how: How::Machine,
        description: "An annotation with subtype undefined in ISO 32000 does not meet 7.18.1." },
    Condition { id: "28-007", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.2-2", how: How::Machine,
        description: "An annotation of subtype TrapNet exists." },
    Condition { id: "28-008", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.3-1", how: How::Machine,
        description: "A page containing an annotation does not contain a Tabs entry." },
    Condition { id: "28-009", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.3-1", how: How::Machine,
        description: "A page containing an annotation has a Tabs entry with a value other than S." },
    Condition { id: "28-010", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.4-1", how: How::Machine,
        description: "A widget annotation is not nested within a <Form> tag." },
    Condition { id: "28-011", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.5-1", how: How::Machine,
        description: "A link annotation is not nested within a <Link> tag." },
    Condition { id: "28-012", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.5-2", how: How::Machine,
        description: "A link annotation does not include an alternate description in its Contents entry." },
    Condition { id: "28-013", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.5-3", how: How::Human,
        description: "An IsMap entry is present with a value of true but the functionality is not provided in some other way." },
    Condition { id: "28-014", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.6.2-1", how: How::Machine,
        description: "CT entry is missing from the media clip data dictionary." },
    Condition { id: "28-015", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.6.2-1", how: How::Machine,
        description: "Alt entry is missing from the media clip data dictionary." },
    Condition { id: "28-016", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.7-1", how: How::Machine,
        description: "File attachment annotations do not conform to 7.11." },
    Condition { id: "28-017", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.8-1", how: How::Machine,
        description: "A PrinterMark annotation is included in the logical structure." },
    Condition { id: "28-018", checkpoint: 28, checkpoint_name: "Annotations", section: "UA1:7.18.8-2", how: How::Machine,
        description: "The appearance stream of a PrinterMark annotation is not marked as Artifact." },
    Condition { id: "29-001", checkpoint: 29, checkpoint_name: "Actions", section: "UA1:7.19-1", how: How::Human,
        description: "A script requires specific timing for individual keystrokes." },
    Condition { id: "30-001", checkpoint: 30, checkpoint_name: "XObjects", section: "UA1:7.20-1", how: How::Machine,
        description: "A reference XObject is present." },
    Condition { id: "30-002", checkpoint: 30, checkpoint_name: "XObjects", section: "UA1:7.20-2", how: How::Machine,
        description: "Form XObject contains MCIDs and is referenced more than once." },
    Condition { id: "31-001", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3-1", how: How::Machine,
        description: "A Type 0 font with a non-Identity encoding has different Registry values in the CIDFont and CMap CIDSystemInfo dictionaries." },
    Condition { id: "31-002", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.1-1", how: How::Machine,
        description: "A Type 0 font with a non-Identity encoding has different Ordering values in the CIDFont and CMap CIDSystemInfo dictionaries." },
    Condition { id: "31-003", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.1-1", how: How::Machine,
        description: "A Type 0 font with a non-Identity encoding has a CIDFont Supplement that is less than the CMap Supplement." },
    Condition { id: "31-004", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.2-1", how: How::Machine,
        description: "A Type 2 CID font contains neither a stream nor the name Identity as the value of the CIDToGIDMap entry." },
    Condition { id: "31-005", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.2-1", how: How::Machine,
        description: "A Type 2 CID font does not contain a CIDToGIDMap entry." },
    Condition { id: "31-006", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.3-1", how: How::Machine,
        description: "A CMap is neither a predefined CMap listed in ISO 32000-1 Table 118 nor embedded." },
    Condition { id: "31-007", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.3-1", how: How::Machine,
        description: "The WMode entry in a CMap dictionary is not identical to the WMode value in the CMap stream." },
    Condition { id: "31-008", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.3.3-2", how: How::Machine,
        description: "A CMap references another CMap which is not listed in ISO 32000-1:2008, 9.7.5.2, Table 118." },
    Condition { id: "31-009", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.1-1", how: How::Machine,
        description: "For a font used by text intended to be rendered the font program is not embedded." },
    Condition { id: "31-010", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.1-2", how: How::Human,
        description: "A font program is embedded that is not legally embeddable for unlimited, universal rendering." },
    Condition { id: "31-011", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.1-3", how: How::Machine,
        description: "For a font used by text the font program is embedded but does not contain glyphs for all glyphs referenced for rendering." },
    Condition { id: "31-012", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.2-1", how: How::Machine,
        description: "An embedded Type 1 font has a CharSet string, but a glyph present in the font program is not listed in it." },
    Condition { id: "31-013", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.2-2", how: How::Machine,
        description: "An embedded Type 1 font has a CharSet string, but a glyph listed in it is not present in the font program." },
    Condition { id: "31-014", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.2-3", how: How::Machine,
        description: "An embedded CID font has a CIDSet stream, but a glyph present in the font program is not listed in it." },
    Condition { id: "31-015", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.4.2-4", how: How::Machine,
        description: "An embedded CID font has a CIDSet stream, but a glyph listed in it is not present in the font program." },
    Condition { id: "31-016", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.5-1", how: How::Machine,
        description: "For one or more glyphs, the width in the font dictionary and in the embedded font program differ by more than 1/1000 unit." },
    Condition { id: "31-017", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-1", how: How::Machine,
        description: "A non-symbolic TrueType font is used for rendering, but the embedded font program has no non-symbolic cmap subtable." },
    Condition { id: "31-018", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-2", how: How::Machine,
        description: "A non-symbolic TrueType font is used for rendering, but a rendered glyph cannot be looked up via any non-symbolic cmap subtable." },
    Condition { id: "31-019", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-3", how: How::Machine,
        description: "The font dictionary for a non-symbolic TrueType font does not contain an Encoding entry." },
    Condition { id: "31-020", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-4", how: How::Machine,
        description: "The font dictionary for a non-symbolic TrueType font has an Encoding dictionary without a BaseEncoding entry." },
    Condition { id: "31-021", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-5", how: How::Machine,
        description: "The Encoding or BaseEncoding of a non-symbolic TrueType font is neither MacRomanEncoding nor WinAnsiEncoding." },
    Condition { id: "31-022", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-6", how: How::Machine,
        description: "The Differences array of a non-symbolic TrueType font contains glyph names not listed in the Adobe Glyph List." },
    Condition { id: "31-023", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-7", how: How::Machine,
        description: "A non-symbolic TrueType font has a Differences array but the embedded font program has no (3,1) Microsoft Unicode cmap." },
    Condition { id: "31-024", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-8", how: How::Machine,
        description: "The Encoding entry is present in the font dictionary for a symbolic TrueType font." },
    Condition { id: "31-025", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-9", how: How::Machine,
        description: "The embedded font program for a symbolic TrueType font contains no cmap." },
    Condition { id: "31-026", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.6-10", how: How::Machine,
        description: "The embedded font program for a symbolic TrueType font has more than one cmap subtable but none is a (3,0) Microsoft Symbol cmap." },
    Condition { id: "31-027", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.7-1", how: How::Machine,
        description: "A font dictionary has no ToUnicode entry and no other permitted mechanism (standard encoding, AGL glyph names, Adobe CJK collection, non-symbolic TrueType) provides Unicode mapping." },
    Condition { id: "31-028", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.7-2", how: How::Machine,
        description: "One or more Unicode values specified in the ToUnicode CMap are zero (0)." },
    Condition { id: "31-029", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.7-3", how: How::Machine,
        description: "One or more Unicode values specified in the ToUnicode CMap are equal to either U+FEFF or U+FFFE." },
    Condition { id: "31-030", checkpoint: 31, checkpoint_name: "Fonts", section: "UA1:7.21.8-1", how: How::Machine,
        description: "One or more characters used in text showing operators reference the .notdef glyph." },
];

/// Look up a condition by its official index.
pub fn condition(id: &str) -> Option<&'static Condition> {
    CONDITIONS.iter().find(|c| c.id == id)
}

/// True for ids of Horn-specific checks that go beyond the Matterhorn Protocol
/// (`"NN-xNN"`), which never correspond to a published failure condition.
pub fn is_extension_rule(id: &str) -> bool {
    id.len() == 6 && id.as_bytes()[3] == b'x'
}

/// Number of machine-checkable conditions in the protocol.
pub fn machine_count() -> usize {
    CONDITIONS.iter().filter(|c| c.how == How::Machine).count()
}

/// Number of conditions that require human judgment.
pub fn human_count() -> usize {
    CONDITIONS.iter().filter(|c| c.how == How::Human).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_matches_protocol_totals() {
        assert_eq!(CONDITIONS.len(), 137);
        assert_eq!(machine_count(), 87);
        assert_eq!(human_count(), 48);
        assert_eq!(CONDITIONS.iter().filter(|c| c.how == How::None).count(), 2);
    }

    #[test]
    fn ids_are_unique_and_ordered() {
        for w in CONDITIONS.windows(2) {
            assert!(w[0].id < w[1].id, "{} must precede {}", w[0].id, w[1].id);
        }
    }

    #[test]
    fn extension_rule_ids() {
        assert!(is_extension_rule("28-x01"));
        assert!(!is_extension_rule("28-001"));
        assert!(!is_extension_rule("baseline"));
    }
}
