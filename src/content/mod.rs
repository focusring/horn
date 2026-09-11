//! Content stream analysis shared by several checks: which character codes
//! each font is used with, and in which text rendering mode.

pub mod cmap;
pub mod usage;

pub use usage::{ContentUsage, FontUsage, FontUsageMap, collect_content_usage};
