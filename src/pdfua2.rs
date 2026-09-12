//! PDF/UA-2 (ISO 14289-2:2024) rule catalogue.
//!
//! PDF/UA-2 is checked against the requirements of ISO 14289-2 directly: the
//! Matterhorn Protocol 2.0 for PDF/UA-2 has not been published yet, so there
//! is no official failure-condition index to cite. Horn uses interim ids of
//! the form `ua2:<clause>-<test>` — the ISO 14289-2 clause number followed by
//! a test number — which mirror the rule ids of the veraPDF PDF/UA-2
//! validation profile (the interpretation PAC 2024 follows as well). Once
//! Matterhorn 2.0 is released these ids will be re-mapped to its indices.
//!
//! Requirements that PDF/UA-2 shares with PDF/UA-1 (fonts, tagging, tables,
//! optional content, …) are reported under their Matterhorn 1.1 index by the
//! regular checks; only rules that exist solely in PDF/UA-2 are listed here.
//! Every rule is machine-checkable.

use serde::Serialize;

/// One PDF/UA-2 rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Rule {
    /// Interim id, e.g. `"ua2:8.2.5.12-1"`.
    pub id: &'static str,
    /// ISO 14289-2 clause, e.g. `"8.2.5.12"`.
    pub clause: &'static str,
    /// Closest Matterhorn 1.1 checkpoint; used to group the rule in reports.
    pub checkpoint: u8,
    /// The failure condition.
    pub description: &'static str,
}

/// All PDF/UA-2-only rules Horn checks, in clause order.
pub static RULES: [Rule; 37] = [
    // 5 — Conformance requirements / version identification
    Rule {
        id: "ua2:5-3",
        clause: "5",
        checkpoint: 6,
        description: "The 'part' property of the PDF/UA identification schema does not use the 'pdfuaid' namespace prefix.",
    },
    Rule {
        id: "ua2:5-4",
        clause: "5",
        checkpoint: 6,
        description: "The 'rev' property of the PDF/UA identification schema does not use the 'pdfuaid' namespace prefix.",
    },
    Rule {
        id: "ua2:5-5",
        clause: "5",
        checkpoint: 6,
        description: "pdfuaid:rev is missing or its value is not the four-digit year 2024.",
    },
    // 8.2.4 — Structure types and role mapping (PDF 2.0 namespaces)
    Rule {
        id: "ua2:8.2.4-2",
        clause: "8.2.4",
        checkpoint: 2,
        description: "A circular role mapping exists in a namespace RoleMapNS.",
    },
    Rule {
        id: "ua2:8.2.4-3",
        clause: "8.2.4",
        checkpoint: 2,
        description: "A structure type is role mapped to another type within the same namespace.",
    },
    // 8.2.5.2 — Document and DocumentFragment
    Rule {
        id: "ua2:8.2.5.2-1",
        clause: "8.2.5.2",
        checkpoint: 9,
        description: "The structure tree root does not contain a single Document element as its only child.",
    },
    Rule {
        id: "ua2:8.2.5.2-2",
        clause: "8.2.5.2",
        checkpoint: 9,
        description: "The Document element is not in the PDF 2.0 namespace (http://iso.org/pdf2/ssn).",
    },
    // 8.2.5.8 — TOC and TOCI
    Rule {
        id: "ua2:8.2.5.8-1",
        clause: "8.2.5.8",
        checkpoint: 9,
        description: "A TOCI element has no Ref entry identifying the content it refers to.",
    },
    // 8.2.5.12 — Headings
    Rule {
        id: "ua2:8.2.5.12-1",
        clause: "8.2.5.12",
        checkpoint: 14,
        description: "The generic H structure type is used; PDF/UA-2 requires numbered headings (Hn).",
    },
    // 8.2.5.14 — Note and FENote
    Rule {
        id: "ua2:8.2.5.14-1",
        clause: "8.2.5.14",
        checkpoint: 19,
        description: "The Note structure type is used; PDF 2.0 deprecates Note in favour of FENote.",
    },
    Rule {
        id: "ua2:8.2.5.14-4",
        clause: "8.2.5.14",
        checkpoint: 19,
        description: "A FENote element has a NoteType attribute other than Footnote, Endnote or None.",
    },
    // 8.2.5.25 — Lists
    Rule {
        id: "ua2:8.2.5.25-1",
        clause: "8.2.5.25",
        checkpoint: 16,
        description: "A list whose items contain Lbl elements has no ListNumbering attribute, or ListNumbering is None.",
    },
    Rule {
        id: "ua2:8.2.5.25-2",
        clause: "8.2.5.25",
        checkpoint: 16,
        description: "An LI element contains content that is not enclosed in a Lbl or LBody element.",
    },
    // 8.2.5.20 — Link and Reference
    Rule {
        id: "ua2:8.2.5.20-2",
        clause: "8.2.5.20",
        checkpoint: 28,
        description: "Link annotations enclosed in the same Link or Reference element target different locations.",
    },
    // 8.2.5.29 — Formula and MathML
    Rule {
        id: "ua2:8.2.5.29-1",
        clause: "8.2.5.29",
        checkpoint: 17,
        description: "A MathML structure element is not a child of a Formula element (or of another MathML element).",
    },
    // 8.4.3 — Unicode private use area
    Rule {
        id: "ua2:8.4.3-2",
        clause: "8.4.3",
        checkpoint: 10,
        description: "An ActualText entry contains Unicode private-use-area code points.",
    },
    Rule {
        id: "ua2:8.4.3-3",
        clause: "8.4.3",
        checkpoint: 10,
        description: "An Alt entry contains Unicode private-use-area code points.",
    },
    // 8.8 — Intra-document destinations
    Rule {
        id: "ua2:8.8-1",
        clause: "8.8",
        checkpoint: 27,
        description: "An intra-document destination (outline item, link, GoTo action) is not a structure destination.",
    },
    Rule {
        id: "ua2:8.8-2",
        clause: "8.8",
        checkpoint: 29,
        description: "A GoTo action has no structure destination (SD entry).",
    },
    // 8.9.2.2 — Annotations as artifacts
    Rule {
        id: "ua2:8.9.2.2-1",
        clause: "8.9.2.2",
        checkpoint: 28,
        description: "An annotation with the Invisible flag is included in the logical structure and is not an artifact.",
    },
    Rule {
        id: "ua2:8.9.2.2-2",
        clause: "8.9.2.2",
        checkpoint: 28,
        description: "An annotation with the NoView flag (and without ToggleNoView) is included in the logical structure and is not an artifact.",
    },
    // 8.9.2.4 — Annotation types
    Rule {
        id: "ua2:8.9.2.4.7-1",
        clause: "8.9.2.4.7",
        checkpoint: 28,
        description: "A rubber stamp annotation has neither a Name nor a Contents entry.",
    },
    Rule {
        id: "ua2:8.9.2.4.8-1",
        clause: "8.9.2.4.8",
        checkpoint: 28,
        description: "An Ink annotation has no Contents entry.",
    },
    Rule {
        id: "ua2:8.9.2.4.9-1",
        clause: "8.9.2.4.9",
        checkpoint: 28,
        description: "A Popup annotation is included in the logical structure.",
    },
    Rule {
        id: "ua2:8.9.2.4.10-1",
        clause: "8.9.2.4.10",
        checkpoint: 28,
        description: "The file specification of a file attachment annotation has no AFRelationship entry.",
    },
    Rule {
        id: "ua2:8.9.2.4.11-1",
        clause: "8.9.2.4.11",
        checkpoint: 28,
        description: "A Sound annotation is present (deprecated in PDF 2.0, not permitted in PDF/UA-2).",
    },
    Rule {
        id: "ua2:8.9.2.4.11-2",
        clause: "8.9.2.4.11",
        checkpoint: 28,
        description: "A Movie annotation is present (deprecated in PDF 2.0, not permitted in PDF/UA-2).",
    },
    Rule {
        id: "ua2:8.9.2.4.12-1",
        clause: "8.9.2.4.12",
        checkpoint: 28,
        description: "A Screen annotation has no Contents entry.",
    },
    Rule {
        id: "ua2:8.9.2.4.13-1",
        clause: "8.9.2.4.13",
        checkpoint: 28,
        description: "A zero-size Widget annotation is included in the logical structure and is not an artifact.",
    },
    Rule {
        id: "ua2:8.9.2.4.19-1",
        clause: "8.9.2.4.19",
        checkpoint: 28,
        description: "A 3D annotation has no Contents entry.",
    },
    Rule {
        id: "ua2:8.9.2.4.19-2",
        clause: "8.9.2.4.19",
        checkpoint: 28,
        description: "A RichMedia annotation has no Contents entry.",
    },
    // 8.9.4.2 — Contents and Alt
    Rule {
        id: "ua2:8.9.4.2-1",
        clause: "8.9.4.2",
        checkpoint: 28,
        description: "An annotation's Contents entry differs from the Alt entry of its enclosing structure element.",
    },
    // 8.10 — Forms
    Rule {
        id: "ua2:8.10.1-2",
        clause: "8.10.1",
        checkpoint: 28,
        description: "A Form structure element contains more than one widget annotation.",
    },
    Rule {
        id: "ua2:8.10.2.3-1",
        clause: "8.10.2.3",
        checkpoint: 28,
        description: "A form field widget has neither a Lbl element in its Form structure element nor a Contents entry.",
    },
    Rule {
        id: "ua2:8.10.2.3-2",
        clause: "8.10.2.3",
        checkpoint: 28,
        description: "A form field widget with additional actions (AA) has no Contents entry.",
    },
    // 8.11 — Metadata
    Rule {
        id: "ua2:8.11.1-2",
        clause: "8.11.1",
        checkpoint: 6,
        description: "The Metadata stream in the catalog lacks /Type /Metadata and /Subtype /XML.",
    },
    // 8.14 — Embedded files
    Rule {
        id: "ua2:8.14.1-1",
        clause: "8.14.1",
        checkpoint: 21,
        description: "A file specification in the EmbeddedFiles name tree has no Desc entry.",
    },
];

/// Whether a rule id is an interim PDF/UA-2 id (`ua2:…`).
pub fn is_ua2_rule(id: &str) -> bool {
    id.starts_with("ua2:")
}

/// Look up a PDF/UA-2 rule by id.
pub fn rule(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

/// The Matterhorn checkpoint a PDF/UA-2 rule is grouped under.
pub fn checkpoint(id: &str) -> Option<u8> {
    rule(id).map(|r| r.checkpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_well_formed() {
        let mut ids: Vec<&str> = RULES.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate PDF/UA-2 rule ids");
        for r in &RULES {
            assert!(is_ua2_rule(r.id), "{} must start with ua2:", r.id);
            assert!(
                r.id[4..].starts_with(r.clause),
                "{} does not start with its clause {}",
                r.id,
                r.clause
            );
            assert!((1..=31).contains(&r.checkpoint), "{} checkpoint", r.id);
        }
    }

    #[test]
    fn lookup() {
        assert_eq!(checkpoint("ua2:8.2.5.12-1"), Some(14));
        assert!(rule("ua2:nope").is_none());
        assert!(!is_ua2_rule("28-010"));
    }
}
