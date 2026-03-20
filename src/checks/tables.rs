use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Severity};
use anyhow::Result;

/// Checkpoint 15: Table structure checks.
///
/// Validates table elements have correct TH/TD structure,
/// header cells are identified, and complex tables have proper attributes.
pub struct TableChecks;

impl Check for TableChecks {
    fn id(&self) -> &'static str {
        "15-tables"
    }

    fn checkpoint(&self) -> u8 {
        15
    }

    fn description(&self) -> &'static str {
        "Tables: TH/TD structure, header identification"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf_doc = doc.lopdf();
        let catalog = doc.raw_catalog()?;

        let Some(struct_tree) = get_struct_tree(catalog, lopdf_doc) else {
            return Ok(results);
        };

        let mut tables: Vec<TableInfo> = Vec::new();
        collect_tables(lopdf_doc, struct_tree, &mut tables, 0);

        for (i, table) in tables.iter().enumerate() {
            let table_label = format!("Table {}", i + 1);

            // 15-002: Tables must contain TR elements with TH or TD children
            if table.rows.is_empty() {
                results.push(fail(
                    "15-002",
                    15,
                    &format!("{table_label}: Table structure element has no TR (row) children"),
                ));
            }

            // 15-003: Tables must have at least one TH (header cell)
            let has_th = table.rows.iter().any(|r| r.has_th);
            if !has_th && !table.rows.is_empty() {
                results.push(fail(
                    "15-003",
                    15,
                    &format!(
                        "{table_label}: No TH (header) cells found — tables must identify headers"
                    ),
                ));
            }

            // 15-005: Complex tables (multiple header rows or irregular structure)
            // should use /Headers attribute on TD cells, OR /Scope on TH cells,
            // OR THead/TBody structure for header association.
            // All three approaches are valid per PDF/UA-1.
            if table.is_complex
                && !table.has_headers_attr
                && !table.has_scope_attr
                && !table.has_thead
            {
                results.push(CheckResult {
                    rule_id: "15-005".to_string(),
                    checkpoint: 15,
                    description: format!(
                        "{table_label}: Complex table without /Headers attributes on cells"
                    ),
                    severity: Severity::Warning,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "{table_label}: Complex table structure detected but TD cells lack /Headers attributes for programmatic association"
                        ),
                        location: None,
                    },
                });
            }

            // 15-004: TH cells should have /Scope attribute for proper header association.
            // If Scope is missing on TH cells and the table doesn't use THead or Headers,
            // data cells can't be programmatically associated with headers.
            if has_th && !table.has_scope_attr && !table.has_headers_attr && !table.has_thead {
                results.push(fail(
                    "15-004",
                    15,
                    &format!("{table_label}: TH cells lack /Scope attribute — headers cannot be associated with data cells"),
                ));
            }

            // 15-004: Scope values must be /Row, /Column, or /Both
            if table.has_scope_attr && !table.has_valid_scope {
                results.push(fail(
                    "15-004",
                    15,
                    &format!("{table_label}: TH cells have /Scope with invalid value (must be /Row, /Column, or /Both)"),
                ));
            }

            // 15-006: RowSpan/ColSpan attribute values must be valid positive integers
            // and must not exceed the actual table dimensions
            for issue in &table.attr_issues {
                results.push(fail("15-006", 15, &format!("{table_label}: {issue}")));
            }

            // Check RowSpan/ColSpan values against actual table geometry
            let total_rows = table.rows.len();
            let max_cols = table.rows.iter().map(|r| r.cell_count).max().unwrap_or(0);
            for span in &table.span_values {
                if span.is_row && usize::try_from(span.value).unwrap_or(0) > total_rows {
                    results.push(fail(
                        "15-006",
                        15,
                        &format!(
                            "{table_label}: {} cell has RowSpan={} but table only has {} rows",
                            span.cell_type, span.value, total_rows
                        ),
                    ));
                }
                if !span.is_row && usize::try_from(span.value).unwrap_or(0) > max_cols {
                    results.push(fail(
                        "15-006",
                        15,
                        &format!(
                            "{table_label}: {} cell has ColSpan={} but max column count is {}",
                            span.cell_type, span.value, max_cols
                        ),
                    ));
                }
            }

            // If everything checks out
            if has_th && !table.rows.is_empty() && table.attr_issues.is_empty() {
                results.push(pass(
                    "15-002",
                    15,
                    &format!("{table_label}: Table has valid TR/TH/TD structure"),
                ));
            }
        }

        Ok(results)
    }
}

#[allow(clippy::struct_excessive_bools)]
struct TableInfo {
    rows: Vec<RowInfo>,
    is_complex: bool,
    has_headers_attr: bool,
    has_scope_attr: bool,
    has_valid_scope: bool,
    has_thead: bool,
    attr_issues: Vec<String>,
    span_values: Vec<SpanInfo>,
}

struct SpanInfo {
    cell_type: String,
    value: i64,
    is_row: bool, // true = RowSpan, false = ColSpan
}

struct RowInfo {
    has_th: bool,
    has_td: bool,
    cell_count: usize,
    effective_cols: usize, // cell_count adjusted for ColSpan values
}

fn collect_tables(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    tables: &mut Vec<TableInfo>,
    depth: usize,
) {
    if depth > 100 {
        return;
    }

    if let Ok(s_type) = dict.get(b"S").and_then(|o| o.as_name()) {
        if s_type == b"Table" {
            let mut table = TableInfo {
                rows: Vec::new(),
                is_complex: false,
                has_headers_attr: false,
                has_scope_attr: false,
                has_valid_scope: false,
                has_thead: false,
                attr_issues: Vec::new(),
                span_values: Vec::new(),
            };
            analyze_table(doc, dict, &mut table, 0);

            // Detect complex tables: varying cell counts or multiple header rows
            let header_rows = table.rows.iter().filter(|r| r.has_th && !r.has_td).count();
            let cell_counts: Vec<usize> = table.rows.iter().map(|r| r.cell_count).collect();
            let varying_cells = cell_counts.windows(2).any(|w| w[0] != w[1]);
            table.is_complex = header_rows > 1 || varying_cells;

            tables.push(table);
            return; // Don't recurse into table children again
        }
    }

    walk_children(doc, dict, |child_dict| {
        collect_tables(doc, child_dict, tables, depth + 1);
    });
}

fn analyze_table(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    table: &mut TableInfo,
    depth: usize,
) {
    if depth > 50 {
        return;
    }

    if let Ok(s_type) = dict.get(b"S").and_then(|o| o.as_name()) {
        if s_type == b"TR" {
            let mut row = RowInfo {
                has_th: false,
                has_td: false,
                cell_count: 0,
                effective_cols: 0,
            };
            analyze_row(doc, dict, &mut row, table);
            table.rows.push(row);
            return;
        }

        // THead, TBody, TFoot contain TR elements
        if matches!(s_type, b"THead" | b"TBody" | b"TFoot") {
            if s_type == b"THead" {
                table.has_thead = true;
            }
            walk_children(doc, dict, |child_dict| {
                analyze_table(doc, child_dict, table, depth + 1);
            });
            return;
        }
    }

    walk_children(doc, dict, |child_dict| {
        analyze_table(doc, child_dict, table, depth + 1);
    });
}

fn analyze_row(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    row: &mut RowInfo,
    table: &mut TableInfo,
) {
    walk_children(doc, dict, |child_dict| {
        if let Ok(s_type) = child_dict.get(b"S").and_then(|o| o.as_name()) {
            match s_type {
                b"TH" => {
                    row.has_th = true;
                    row.cell_count += 1;
                    row.effective_cols += get_colspan(doc, child_dict);
                    // Check for /Scope attribute on TH cells and validate value
                    if let Ok(attrs) = child_dict.get(b"A") {
                        let (has, valid) = check_scope_attr(doc, attrs);
                        if has {
                            table.has_scope_attr = true;
                            if valid {
                                table.has_valid_scope = true;
                            }
                        }
                    }
                    // Validate RowSpan/ColSpan attributes
                    validate_cell_span_attrs(doc, child_dict, "TH", table);
                }
                b"TD" => {
                    row.has_td = true;
                    row.cell_count += 1;
                    row.effective_cols += get_colspan(doc, child_dict);

                    // Check for /Headers attribute (PDF 2.0 / PDF/UA)
                    if let Ok(attrs) = child_dict.get(b"A") {
                        if check_for_headers_attr(doc, attrs) {
                            table.has_headers_attr = true;
                        }
                    }
                    // Validate RowSpan/ColSpan attributes
                    validate_cell_span_attrs(doc, child_dict, "TD", table);
                }
                _ => {}
            }
        }
    });
}

/// Get the `ColSpan` value from a cell's attribute dictionary. Defaults to 1.
fn get_colspan(doc: &lopdf::Document, cell_dict: &lopdf::Dictionary) -> usize {
    let Ok(attrs) = cell_dict.get(b"A") else {
        return 1;
    };

    let from_dict = |d: &lopdf::Dictionary| -> Option<usize> {
        d.get(b"ColSpan")
            .ok()?
            .as_i64()
            .ok()
            .map(|n| usize::try_from(n.max(1)).unwrap_or(1))
    };

    match attrs {
        lopdf::Object::Dictionary(dict) => from_dict(dict).unwrap_or(1),
        lopdf::Object::Array(arr) => {
            for item in arr {
                let d = if let Ok(d) = item.as_dict() {
                    Some(d)
                } else if let Ok(ref_id) = item.as_reference() {
                    doc.get_object(ref_id).ok().and_then(|o| o.as_dict().ok())
                } else {
                    None
                };
                if let Some(d) = d {
                    if let Some(cs) = from_dict(d) {
                        return cs;
                    }
                }
            }
            1
        }
        lopdf::Object::Reference(ref_id) => doc
            .get_object(*ref_id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(from_dict)
            .unwrap_or(1),
        _ => 1,
    }
}

/// Validate `RowSpan` and `ColSpan` attribute values on a TH or TD cell.
///
/// Per PDF spec, `RowSpan` and `ColSpan` must be positive integers (>= 1).
/// A value of 0, negative, or non-integer is invalid.
/// Also records span values for geometry validation.
fn validate_cell_span_attrs(
    doc: &lopdf::Document,
    cell_dict: &lopdf::Dictionary,
    cell_type: &str,
    table: &mut TableInfo,
) {
    let Ok(attrs) = cell_dict.get(b"A") else {
        return;
    };

    let ct = cell_type.to_string();
    let check_span =
        |attr_dict: &lopdf::Dictionary, issues: &mut Vec<String>, spans: &mut Vec<SpanInfo>| {
            for (span_key, is_row) in &[(b"RowSpan" as &[u8], true), (b"ColSpan" as &[u8], false)] {
                if let Ok(val) = attr_dict.get(span_key) {
                    let span_name = String::from_utf8_lossy(span_key);
                    match val.as_i64() {
                        Ok(n) if n < 1 => {
                            issues.push(format!(
                                "{ct} cell has invalid {span_name} value {n} (must be >= 1)"
                            ));
                        }
                        Ok(n) => {
                            spans.push(SpanInfo {
                                cell_type: ct.clone(),
                                value: n,
                                is_row: *is_row,
                            });
                        }
                        Err(_) => {
                            issues.push(format!("{ct} cell has non-integer {span_name} value"));
                        }
                    }
                }
            }
        };

    match attrs {
        lopdf::Object::Dictionary(dict) => {
            check_span(dict, &mut table.attr_issues, &mut table.span_values);
        }
        lopdf::Object::Array(arr) => {
            for item in arr {
                if let Ok(d) = item.as_dict() {
                    check_span(d, &mut table.attr_issues, &mut table.span_values);
                } else if let Ok(ref_id) = item.as_reference() {
                    if let Ok(obj) = doc.get_object(ref_id) {
                        if let Ok(d) = obj.as_dict() {
                            check_span(d, &mut table.attr_issues, &mut table.span_values);
                        }
                    }
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    check_span(d, &mut table.attr_issues, &mut table.span_values);
                }
            }
        }
        _ => {}
    }
}

/// Check if an attribute dictionary has a /Scope entry.
/// Returns (`has_scope`, `is_valid_scope`).
fn check_scope_attr(doc: &lopdf::Document, attrs: &lopdf::Object) -> (bool, bool) {
    let check_dict = |d: &lopdf::Dictionary| -> (bool, bool) {
        match d.get(b"Scope") {
            Ok(scope_obj) => {
                let valid = scope_obj
                    .as_name()
                    .ok()
                    .is_some_and(|n| matches!(n, b"Row" | b"Column" | b"Both"));
                (true, valid)
            }
            Err(_) => (false, false),
        }
    };

    match attrs {
        lopdf::Object::Dictionary(dict) => check_dict(dict),
        lopdf::Object::Array(arr) => {
            let mut has = false;
            let mut valid = false;
            for item in arr {
                let d = if let Ok(d) = item.as_dict() {
                    Some(d)
                } else if let Ok(ref_id) = item.as_reference() {
                    doc.get_object(ref_id).ok().and_then(|o| o.as_dict().ok())
                } else {
                    None
                };
                if let Some(d) = d {
                    let (h, v) = check_dict(d);
                    if h {
                        has = true;
                    }
                    if v {
                        valid = true;
                    }
                }
            }
            (has, valid)
        }
        lopdf::Object::Reference(ref_id) => doc
            .get_object(*ref_id)
            .ok()
            .map_or((false, false), |o| check_scope_attr(doc, o)),
        _ => (false, false),
    }
}

fn check_for_headers_attr(doc: &lopdf::Document, attrs: &lopdf::Object) -> bool {
    // Attributes can be a dictionary or array of dictionaries
    match attrs {
        lopdf::Object::Dictionary(dict) => dict.has(b"Headers"),
        lopdf::Object::Array(arr) => arr.iter().any(|item| {
            if let Ok(d) = item.as_dict() {
                d.has(b"Headers")
            } else if let Ok(ref_id) = item.as_reference() {
                doc.get_object(ref_id)
                    .ok()
                    .and_then(|o| o.as_dict().ok())
                    .is_some_and(|d| d.has(b"Headers"))
            } else {
                false
            }
        }),
        lopdf::Object::Reference(ref_id) => doc
            .get_object(*ref_id)
            .ok()
            .is_some_and(|o| check_for_headers_attr(doc, o)),
        _ => false,
    }
}

fn walk_children(
    doc: &lopdf::Document,
    dict: &lopdf::Dictionary,
    mut callback: impl FnMut(&lopdf::Dictionary),
) {
    let Ok(kids) = dict.get(b"K") else { return };

    match kids {
        lopdf::Object::Array(arr) => {
            for kid in arr {
                if let Ok(kid_ref) = kid.as_reference() {
                    if let Ok(kid_obj) = doc.get_object(kid_ref) {
                        if let Ok(kid_dict) = kid_obj.as_dict() {
                            callback(kid_dict);
                        }
                    }
                } else if let Ok(kid_dict) = kid.as_dict() {
                    callback(kid_dict);
                }
            }
        }
        lopdf::Object::Reference(ref_id) => {
            if let Ok(obj) = doc.get_object(*ref_id) {
                if let Ok(d) = obj.as_dict() {
                    callback(d);
                }
            }
        }
        lopdf::Object::Dictionary(d) => {
            callback(d);
        }
        _ => {}
    }
}

fn get_struct_tree<'a>(
    catalog: &'a lopdf::Dictionary,
    doc: &'a lopdf::Document,
) -> Option<&'a lopdf::Dictionary> {
    let ref_id = catalog.get(b"StructTreeRoot").ok()?.as_reference().ok()?;
    doc.get_object(ref_id).ok()?.as_dict().ok()
}

fn pass(rule_id: &str, checkpoint: u8, description: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint,
        description: description.to_string(),
        severity: Severity::Info,
        outcome: CheckOutcome::Pass,
    }
}

fn fail(rule_id: &str, checkpoint: u8, message: &str) -> CheckResult {
    CheckResult {
        rule_id: rule_id.to_string(),
        checkpoint,
        description: message.to_string(),
        severity: Severity::Error,
        outcome: CheckOutcome::Fail {
            message: message.to_string(),
            location: None,
        },
    }
}
