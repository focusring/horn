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
        // Checkpoint 06: Metadata
        UaErrorCode::MissingLanguage => ("06-001".into(), 6, Severity::Error),
        UaErrorCode::MissingTitle => ("06-004".into(), 6, Severity::Error),
        UaErrorCode::TitleNotDisplayed => ("06-003".into(), 6, Severity::Error),
        UaErrorCode::MissingPdfuaId | UaErrorCode::InvalidPdfuaId => {
            ("06-004".into(), 6, Severity::Error)
        }

        // Checkpoint 01/09: Structure and tagging
        UaErrorCode::NotTaggedPdf => ("01-003".into(), 1, Severity::Error),
        UaErrorCode::ContentNotTagged => ("01-004".into(), 1, Severity::Error),
        UaErrorCode::InvalidStructureType => ("09-004".into(), 9, Severity::Error),
        UaErrorCode::InvalidStructureNesting => ("09-006".into(), 9, Severity::Error),

        // Checkpoint 02: Role Mapping
        UaErrorCode::MissingRoleMapping => ("02-001".into(), 2, Severity::Error),

        // Checkpoint 13: Images/Figures
        UaErrorCode::FigureMissingAlt => ("13-004".into(), 13, Severity::Error),
        UaErrorCode::DecorativeNotArtifact => ("01-002".into(), 1, Severity::Warning),
        UaErrorCode::FigureCaptionNotAssociated => ("13-005".into(), 13, Severity::Warning),

        // Checkpoint 14: Headings
        UaErrorCode::HeadingLevelSkipped => ("14-006".into(), 14, Severity::Error),

        // Checkpoint 15: Tables
        UaErrorCode::TableMissingHeaders | UaErrorCode::TableHeadersNotAssociated => {
            ("15-003".into(), 15, Severity::Error)
        }
        UaErrorCode::TableHeaderNotTh | UaErrorCode::TableDataNotTd => {
            ("15-002".into(), 15, Severity::Error)
        }
        UaErrorCode::TableScopeMissing => ("15-004".into(), 15, Severity::Warning),
        UaErrorCode::ComplexTableNoIds => ("15-005".into(), 15, Severity::Error),

        // Checkpoint 28: Annotations/Links
        UaErrorCode::LinkTextNotDescriptive => ("28-003".into(), 28, Severity::Warning),
        UaErrorCode::LinkNoDestination => ("28-004".into(), 28, Severity::Error),
        UaErrorCode::AnnotationNotTagged => ("28-002".into(), 28, Severity::Error),
        UaErrorCode::AnnotationMissingContents => ("28-006".into(), 28, Severity::Error),
        UaErrorCode::WidgetMissingRole => ("28-008".into(), 28, Severity::Error),

        // Checkpoint 31: Fonts
        UaErrorCode::FontNotEmbedded => ("31-001".into(), 31, Severity::Error),
        UaErrorCode::MissingUnicodeMapping => ("31-006".into(), 31, Severity::Error),
        UaErrorCode::MissingActualText => ("31-025".into(), 31, Severity::Error),

        // Checkpoint 16: Lists
        UaErrorCode::ListItemsNotMarked => ("16-001".into(), 16, Severity::Error),
        UaErrorCode::NestedListInvalid => ("16-002".into(), 16, Severity::Error),

        // Checkpoint 25/Form fields
        UaErrorCode::FormFieldMissingName => ("28-009".into(), 28, Severity::Error),
        UaErrorCode::FormFieldMissingTooltip => ("28-010".into(), 28, Severity::Error),
        UaErrorCode::RequiredFieldNotIndicated => ("28-011".into(), 28, Severity::Warning),
        UaErrorCode::FormNoSubmitButton => ("28-012".into(), 28, Severity::Warning),

        // Other checks — mapped to closest Matterhorn condition
        UaErrorCode::InsufficientContrast | UaErrorCode::ColorOnlyInformation => {
            ("04-001".into(), 4, Severity::Warning)
        }
        UaErrorCode::JavaScriptNoAlternative => ("29-001".into(), 29, Severity::Warning),
        UaErrorCode::MultimediaNoCaptions => ("05-002".into(), 5, Severity::Warning),
        UaErrorCode::ReadingOrderInvalid => ("09-001".into(), 9, Severity::Warning),
        UaErrorCode::BookmarksMismatch => ("27-001".into(), 27, Severity::Warning),
    }
}
