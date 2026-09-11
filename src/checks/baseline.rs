use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity, Standard};
use anyhow::Result;
use pdf_oxide::compliance::{PdfUaLevel, PdfUaValidator, UaErrorCode};

/// Baseline check that delegates to `pdf_oxide`'s built-in `PdfUaValidator`.
/// Maps its error codes to Matterhorn Protocol failure condition IDs.
///
/// This check only supports PDF/UA-1, as `pdf_oxide` only implements UA-1 validation.
pub struct BaselineCheck;

impl Check for BaselineCheck {
    fn id(&self) -> &'static str {
        "baseline"
    }

    fn checkpoint(&self) -> u8 {
        0 // Meta-check spanning multiple checkpoints
    }

    fn rules(&self) -> &'static [&'static str] {
        &[
            "01-005", "01-x01", "02-001", "04-001", "05-002", "06-002", "06-003", "07-001",
            "09-001", "09-004", "09-005", "11-001", "13-004", "13-x01", "13-x02", "13-x03",
            "14-003", "15-003", "15-x01", "15-x02", "27-001", "28-002", "28-004", "28-005",
            "28-010", "28-x01", "28-x02", "28-x03", "28-x04", "28-x05", "29-001", "31-009",
            "31-027",
        ]
    }

    fn description(&self) -> &'static str {
        "pdf_oxide built-in PDF/UA-1 validation"
    }

    fn supports(&self, standard: Standard) -> bool {
        standard == Standard::Ua1 || standard == Standard::Unknown
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let validator = PdfUaValidator::new()
            .check_heading_sequence(true)
            .check_color_contrast(false); // Too unreliable for automated pass/fail

        let result = validator
            .validate(doc.oxide(), PdfUaLevel::Ua1)
            .map_err(|e| anyhow::anyhow!("pdf_oxide validation failed: {e}"))?;

        let mut results = Vec::new();

        if result.errors.is_empty() {
            results.push(CheckResult {
                rule_id: "baseline".to_string(),
                checkpoint: 0,
                description: "pdf_oxide PDF/UA-1 baseline validation".to_string(),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
            return Ok(results);
        }

        for error in &result.errors {
            // Skip checks handled more accurately by dedicated modules:
            // - MissingTitle: metadata.rs checks XMP dc:title (not just Info dict)
            // - MissingLanguage: metadata.rs + language.rs handle this
            // - NotTaggedPdf: structure.rs dereferences indirect MarkInfo/Marked
            if matches!(
                error.code,
                UaErrorCode::MissingTitle
                    | UaErrorCode::MissingLanguage
                    | UaErrorCode::NotTaggedPdf
            ) {
                continue;
            }

            let (rule_id, checkpoint, severity) = map_error_code(error.code);
            results.push(CheckResult {
                rule_id,
                checkpoint,
                description: error.message.clone(),
                severity,
                outcome: CheckOutcome::Fail {
                    message: error.message.clone(),
                    location: error.location.as_ref().map(|loc| crate::model::Location {
                        page: None,
                        element: Some(loc.clone()),
                    }),
                },
            });
        }

        Ok(results)
    }
}

/// Map a `pdf_oxide` error code to a Matterhorn Protocol failure condition ID,
/// checkpoint number, and severity.
fn map_error_code(code: UaErrorCode) -> (String, u8, Severity) {
    match code {
        // Checkpoint 06/07/11: Metadata, viewer preferences, language
        UaErrorCode::MissingLanguage => ("11-001".into(), 11, Severity::Error),
        UaErrorCode::MissingTitle => ("06-003".into(), 6, Severity::Error),
        UaErrorCode::TitleNotDisplayed => ("07-001".into(), 7, Severity::Error),
        UaErrorCode::MissingPdfuaId | UaErrorCode::InvalidPdfuaId => {
            ("06-002".into(), 6, Severity::Error)
        }

        // Checkpoint 01/09: Structure and tagging
        UaErrorCode::NotTaggedPdf => ("01-x01".into(), 1, Severity::Error),
        UaErrorCode::ContentNotTagged => ("01-005".into(), 1, Severity::Error),
        UaErrorCode::InvalidStructureNesting => ("09-004".into(), 9, Severity::Error),

        // Checkpoint 02: Role Mapping
        UaErrorCode::InvalidStructureType | UaErrorCode::MissingRoleMapping => {
            ("02-001".into(), 2, Severity::Error)
        }

        // Checkpoint 13: Images/Figures
        UaErrorCode::FigureMissingAlt => ("13-004".into(), 13, Severity::Error),
        UaErrorCode::DecorativeNotArtifact => ("13-x01".into(), 13, Severity::Warning),
        UaErrorCode::FigureCaptionNotAssociated => ("13-x02".into(), 13, Severity::Warning),

        // Checkpoint 14: Headings
        UaErrorCode::HeadingLevelSkipped => ("14-003".into(), 14, Severity::Error),

        // Checkpoint 15: Tables
        UaErrorCode::TableMissingHeaders | UaErrorCode::TableHeadersNotAssociated => {
            ("15-x01".into(), 15, Severity::Error)
        }
        UaErrorCode::TableHeaderNotTh | UaErrorCode::TableDataNotTd => {
            ("09-004".into(), 9, Severity::Error)
        }
        UaErrorCode::TableScopeMissing => ("15-003".into(), 15, Severity::Warning),
        UaErrorCode::ComplexTableNoIds => ("15-x02".into(), 15, Severity::Error),

        // Checkpoint 28: Annotations/Links
        UaErrorCode::LinkTextNotDescriptive => ("28-x02".into(), 28, Severity::Warning),
        UaErrorCode::LinkNoDestination => ("28-x01".into(), 28, Severity::Error),
        UaErrorCode::AnnotationNotTagged => ("28-002".into(), 28, Severity::Error),
        UaErrorCode::AnnotationMissingContents => ("28-004".into(), 28, Severity::Error),
        UaErrorCode::WidgetMissingRole => ("28-010".into(), 28, Severity::Error),

        // Checkpoint 31: Fonts
        UaErrorCode::FontNotEmbedded => ("31-009".into(), 31, Severity::Error),
        UaErrorCode::MissingUnicodeMapping => ("31-027".into(), 31, Severity::Error),
        UaErrorCode::MissingActualText => ("13-x03".into(), 13, Severity::Error),

        // Checkpoint 09: Lists
        UaErrorCode::ListItemsNotMarked | UaErrorCode::NestedListInvalid => {
            ("09-005".into(), 9, Severity::Error)
        }

        // Checkpoint 28: Form fields
        UaErrorCode::FormFieldMissingName => ("28-x03".into(), 28, Severity::Error),
        UaErrorCode::FormFieldMissingTooltip => ("28-005".into(), 28, Severity::Error),
        UaErrorCode::RequiredFieldNotIndicated => ("28-x04".into(), 28, Severity::Warning),
        UaErrorCode::FormNoSubmitButton => ("28-x05".into(), 28, Severity::Warning),

        // Human-judgment checkpoints — pdf_oxide heuristics, reported as warnings
        UaErrorCode::InsufficientContrast | UaErrorCode::ColorOnlyInformation => {
            ("04-001".into(), 4, Severity::Warning)
        }
        UaErrorCode::JavaScriptNoAlternative => ("29-001".into(), 29, Severity::Warning),
        UaErrorCode::MultimediaNoCaptions => ("05-002".into(), 5, Severity::Warning),
        UaErrorCode::ReadingOrderInvalid => ("09-001".into(), 9, Severity::Warning),
        UaErrorCode::BookmarksMismatch => ("27-001".into(), 27, Severity::Warning),
    }
}
