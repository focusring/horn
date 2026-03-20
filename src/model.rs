use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

/// Which PDF/UA standard a document conforms to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Standard {
    /// PDF/UA-1 (ISO 14289-1) — pdfuaid:part = 1
    Ua1,
    /// PDF/UA-2 (ISO 14289-2) — pdfuaid:part = 2
    Ua2,
    /// Could not determine the standard from XMP metadata.
    Unknown,
}

impl fmt::Display for Standard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ua1 => write!(f, "PDF/UA-1"),
            Self::Ua2 => write!(f, "PDF/UA-2"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Severity level for a check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Location within a PDF where an issue was found.
#[derive(Debug, Clone, Serialize)]
pub struct Location {
    /// Page number (1-based), if applicable.
    pub page: Option<u32>,
    /// Structure element path, if applicable.
    pub element: Option<String>,
}

/// The outcome of running a single check against a document.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status")]
pub enum CheckOutcome {
    Pass,
    Fail {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<Location>,
    },
    NeedsReview {
        reason: String,
    },
    NotApplicable,
}

/// A single finding from a check run.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Matterhorn failure condition ID (e.g., "06-001").
    pub rule_id: String,
    /// Which checkpoint this belongs to.
    pub checkpoint: u8,
    /// Human-readable description of the check.
    pub description: String,
    /// Severity of this finding.
    pub severity: Severity,
    /// The outcome.
    pub outcome: CheckOutcome,
}

impl CheckResult {
    pub fn is_failure(&self) -> bool {
        matches!(self.outcome, CheckOutcome::Fail { .. })
    }
}

/// Aggregated validation report for a single PDF file.
#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub path: PathBuf,
    pub standard: Standard,
    pub results: Vec<CheckResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FileReport {
    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, CheckOutcome::Pass))
            .count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| r.is_failure()).count()
    }

    pub fn needs_review(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, CheckOutcome::NeedsReview { .. }))
            .count()
    }

    pub fn is_compliant(&self) -> bool {
        self.error.is_none() && self.failed() == 0
    }
}

/// Report covering all validated files.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub files: Vec<FileReport>,
}

impl ValidationReport {
    pub fn is_compliant(&self) -> bool {
        self.files.iter().all(FileReport::is_compliant)
    }

    /// Check compliance at a given minimum severity threshold.
    /// Only failures at or above the threshold cause non-compliance.
    pub fn is_compliant_at(&self, min_severity: Severity) -> bool {
        self.files.iter().all(|f| {
            f.error.is_none()
                && !f
                    .results
                    .iter()
                    .any(|r| r.is_failure() && r.severity >= min_severity)
        })
    }
}
