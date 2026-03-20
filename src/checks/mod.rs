pub mod annot_struct;
pub mod annotations;
pub mod baseline;
pub mod content_stream;
pub mod dict_entries;
pub mod embedded_files;
pub mod fonts;
pub mod headings;
pub mod images;
pub mod language;
pub mod lists;
pub mod math;
pub mod metadata;
pub mod nesting;
pub mod notes;
pub mod optional_content;
pub mod security;
pub mod structure;
pub mod tables;
pub mod version;
pub mod xfa;

use crate::document::HornDocument;
use crate::model::{CheckResult, Standard};
use anyhow::Result;

/// Trait for a single Matterhorn Protocol check.
pub trait Check: Send + Sync {
    /// Matterhorn failure condition ID (e.g., "06-001").
    fn id(&self) -> &'static str;

    /// Checkpoint number (1-31).
    fn checkpoint(&self) -> u8;

    /// Human-readable description of what this check validates.
    fn description(&self) -> &'static str;

    /// Whether this check is fully machine-checkable.
    fn is_machine_checkable(&self) -> bool {
        true
    }

    /// Whether this check applies to the given PDF/UA standard.
    /// Defaults to true (check applies to both UA-1 and UA-2).
    fn supports(&self, _standard: Standard) -> bool {
        true
    }

    /// Run the check against a document, returning one or more results.
    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>>;
}

/// Registry of all available checks.
pub struct CheckRegistry {
    checks: Vec<Box<dyn Check>>,
}

impl CheckRegistry {
    /// Create a registry with all built-in checks.
    pub fn new() -> Self {
        let checks: Vec<Box<dyn Check>> = vec![
            Box::new(baseline::BaselineCheck),
            Box::new(metadata::MetadataChecks),
            Box::new(version::VersionChecks),
            Box::new(structure::StructureChecks),
            Box::new(dict_entries::DictEntryChecks),
            Box::new(fonts::FontChecks),
            Box::new(headings::HeadingChecks),
            Box::new(tables::TableChecks),
            Box::new(images::ImageChecks),
            Box::new(annotations::AnnotationChecks),
            Box::new(lists::ListChecks),
            Box::new(xfa::XfaCheck),
            Box::new(security::SecurityChecks),
            Box::new(optional_content::OptionalContentChecks),
            Box::new(embedded_files::EmbeddedFileChecks),
            Box::new(math::MathChecks),
            Box::new(notes::NoteChecks),
            Box::new(language::LanguageChecks),
            Box::new(content_stream::ContentStreamChecks),
            Box::new(nesting::NestingChecks),
            Box::new(annot_struct::AnnotStructChecks),
        ];
        Self { checks }
    }

    /// Run all applicable checks against a document for the given standard.
    pub fn run_all(&self, doc: &mut HornDocument, standard: Standard) -> Vec<CheckResult> {
        let mut results = Vec::new();
        for check in &self.checks {
            if !check.supports(standard) {
                continue;
            }
            match check.run(doc) {
                Ok(check_results) => results.extend(check_results),
                Err(e) => {
                    log::warn!("Check {} failed to run: {e}", check.id());
                }
            }
        }
        results
    }

    /// Get the number of registered checks.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// Iterate over all registered checks.
    pub fn checks(&self) -> &[Box<dyn Check>] {
        &self.checks
    }
}

impl Default for CheckRegistry {
    fn default() -> Self {
        Self::new()
    }
}
