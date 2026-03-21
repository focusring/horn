use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckOutcome, CheckResult, Location, Severity, Standard};
use anyhow::Result;

/// Checkpoint 31: Font checks.
///
/// Validates font embedding, `ToUnicode` `CMaps`, and glyph mapping.
pub struct FontChecks;

impl Check for FontChecks {
    fn id(&self) -> &'static str {
        "31-fonts"
    }

    fn checkpoint(&self) -> u8 {
        31
    }

    fn description(&self) -> &'static str {
        "Fonts: embedding, ToUnicode CMaps, glyph mapping"
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let standard = doc.standard();
        let lopdf_doc = doc.lopdf();
        let pages = lopdf_doc.get_pages();

        for (page_num, page_id) in &pages {
            let Ok(fonts) = lopdf_doc.get_page_fonts(*page_id) else {
                continue;
            };

            for (font_name, font_dict) in &fonts {
                let font_label = String::from_utf8_lossy(font_name);
                let location = Some(Location {
                    page: Some(*page_num),
                    element: Some(format!("Font /{font_label}")),
                });

                check_font_embedding(
                    lopdf_doc,
                    font_dict,
                    &font_label,
                    location.as_ref(),
                    &mut results,
                );
                check_tounicode(
                    lopdf_doc,
                    font_dict,
                    &font_label,
                    location.as_ref(),
                    &mut results,
                );
                check_tounicode_content(
                    lopdf_doc,
                    font_dict,
                    &font_label,
                    location.as_ref(),
                    &mut results,
                );
                check_encoding_differences(
                    lopdf_doc,
                    font_dict,
                    &font_label,
                    location.as_ref(),
                    &mut results,
                );
            }
        }

        // Deduplicate: same font can appear on multiple pages
        dedup_results(&mut results);

        // PDF 2.0 / PDF/UA-2 deprecated the CIDSet requirement (31-004).
        // Remove those results for UA-2 documents.
        if standard == Standard::Ua2 {
            results.retain(|r| r.rule_id != "31-004");
        }

        Ok(results)
    }
}

/// 31-001: All fonts must be embedded.
fn check_font_embedding(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    // Type1 base 14 fonts are exempt from embedding in some contexts,
    // but PDF/UA requires all fonts to be embedded.
    let subtype = font_dict
        .get_deref(b"Subtype", doc)
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec);

    // For Type0 (composite) fonts, check the descendant font and CMap encoding
    if subtype.as_deref() == Some(b"Type0") {
        if let Ok(descendants) = font_dict
            .get_deref(b"DescendantFonts", doc)
            .and_then(|o| o.as_array())
        {
            for desc in descendants {
                let desc_dict = if let Ok(desc_ref) = desc.as_reference() {
                    doc.get_object(desc_ref).ok().and_then(|o| o.as_dict().ok())
                } else {
                    desc.as_dict().ok()
                };
                if let Some(dd) = desc_dict {
                    check_font_descriptor_embedding(doc, dd, font_label, location, results);
                    check_cidfont_requirements(doc, dd, font_label, location, results);
                }
            }
        }
        check_type0_cmap_encoding(doc, font_dict, font_label, location, results);
        return;
    }

    check_font_descriptor_embedding(doc, font_dict, font_label, location, results);
}

fn check_font_descriptor_embedding(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    if let Ok(obj) = font_dict.get_deref(b"FontDescriptor", doc) {
        if let Ok(descriptor) = obj.as_dict() {
            let has_font_file = descriptor.get(b"FontFile").is_ok()
                || descriptor.get(b"FontFile2").is_ok()
                || descriptor.get(b"FontFile3").is_ok();

            if has_font_file {
                results.push(CheckResult {
                    rule_id: "31-001".to_string(),
                    checkpoint: 31,
                    description: format!("Font /{font_label} is embedded"),
                    severity: Severity::Info,
                    outcome: CheckOutcome::Pass,
                });
            } else {
                results.push(CheckResult {
                    rule_id: "31-001".to_string(),
                    checkpoint: 31,
                    description: format!("Font /{font_label} is not embedded"),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Font /{font_label} is not embedded — all fonts must be embedded for PDF/UA"
                        ),
                        location: location.cloned(),
                    },
                });
            }
        }
    } else {
        // Some Type3 fonts don't have FontDescriptor — that's a problem for PDF/UA
        let subtype = font_dict
            .get_deref(b"Subtype", doc)
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .unwrap_or_default();

        if subtype != "Type3" {
            results.push(CheckResult {
                rule_id: "31-001".to_string(),
                checkpoint: 31,
                description: format!("Font /{font_label} has no FontDescriptor"),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Font /{font_label} ({subtype}) missing FontDescriptor — cannot verify embedding"
                    ),
                    location: location.cloned(),
                },
            });
        }
    }
}

/// 31-007: Validate `ToUnicode` `CMap` content for invalid mappings.
///
/// A `ToUnicode` `CMap` that maps character codes to U+0000 (NULL) is invalid —
/// it means glyphs have no Unicode representation.
fn check_tounicode_content(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    let Ok(tu_obj) = font_dict.get(b"ToUnicode") else {
        return;
    };

    let Ok(tu_ref) = tu_obj.as_reference() else {
        return;
    };
    let Ok(tu_resolved) = doc.get_object(tu_ref) else {
        return;
    };
    let Ok(tu_stream) = tu_resolved.as_stream() else {
        return;
    };
    let Ok(stream_data) = tu_stream.decompressed_content() else {
        return;
    };

    let content = String::from_utf8_lossy(&stream_data);

    // Check for mappings to invalid Unicode values in bfchar sections
    // Format: <XX> <YYYY>
    let mut null_mappings = 0;
    let mut nonchar_mappings = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('<') {
            continue;
        }
        // Extract target value: second <XXXX> on the line
        if let Some(target_start) = trimmed.find("> <") {
            let after = &trimmed[target_start + 3..];
            if let Some(target_end) = after.find('>') {
                let target = after[..target_end].trim();
                // U+0000 — null, no Unicode representation
                if target.eq_ignore_ascii_case("0000") {
                    null_mappings += 1;
                }
                // U+FFFE — guaranteed noncharacter (byte-order mark reversed)
                // U+FEFF — BOM / zero-width no-break space (invalid as text mapping target)
                // Note: U+FFFF is also a noncharacter but commonly used as a placeholder
                // in valid CID font ToUnicode CMaps, so we don't flag it.
                if target.len() == 4 {
                    let upper = target.to_ascii_uppercase();
                    if matches!(upper.as_str(), "FFFE" | "FEFF") {
                        nonchar_mappings += 1;
                    }
                }
            }
        }
    }

    if null_mappings > 0 {
        results.push(CheckResult {
            rule_id: "31-007".to_string(),
            checkpoint: 31,
            description: format!(
                "Font /{font_label}: ToUnicode CMap has {null_mappings} mapping(s) to U+0000"
            ),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "Font /{font_label}: ToUnicode CMap maps {null_mappings} character code(s) to U+0000 (null) — glyphs have no Unicode representation"
                ),
                location: location.cloned(),
            },
        });
    }

    if nonchar_mappings > 0 {
        results.push(CheckResult {
            rule_id: "31-007".to_string(),
            checkpoint: 31,
            description: format!(
                "Font /{font_label}: ToUnicode CMap has {nonchar_mappings} mapping(s) to Unicode noncharacters"
            ),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "Font /{font_label}: ToUnicode CMap maps {nonchar_mappings} character code(s) to Unicode noncharacters (U+FFFE, U+FEFF, U+FFFF, or U+FDD0-U+FDEF)"
                ),
                location: location.cloned(),
            },
        });
    }
}

/// 31-005: Font encoding validation.
///
/// Checks:
/// - /Encoding name must be a valid predefined name (`WinAnsiEncoding`, `MacRomanEncoding`,
///   `MacExpertEncoding`) or a dictionary — not arbitrary names like /Custom or /Identity
/// - When /Encoding is a dictionary, /`BaseEncoding` must be a valid standard encoding
/// - /Differences array must not contain .notdef glyph names
#[allow(clippy::too_many_lines)]
fn check_encoding_differences(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    // Skip composite (Type0) fonts — they use CMap encoding, not Differences
    let subtype = font_dict
        .get_deref(b"Subtype", doc)
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec);

    if subtype.as_deref() == Some(b"Type0") {
        return;
    }

    let Ok(enc_obj) = font_dict.get_deref(b"Encoding", doc) else {
        // TrueType fonts should have an /Encoding entry for proper character mapping.
        // Without it, glyphs can't be reliably mapped to Unicode.
        if subtype.as_deref() == Some(b"TrueType") {
            // Check if it's a symbolic font (Flags bit 3 set in FontDescriptor)
            let is_symbolic = font_dict
                .get_deref(b"FontDescriptor", doc)
                .ok()
                .and_then(|o| o.as_dict().ok())
                .and_then(|d| d.get(b"Flags").ok())
                .and_then(|o| o.as_i64().ok())
                .is_some_and(|flags| flags & 0x04 != 0); // bit 3 = symbolic
            if !is_symbolic {
                results.push(CheckResult {
                    rule_id: "31-005".to_string(),
                    checkpoint: 31,
                    description: format!(
                        "Font /{font_label}: TrueType font missing /Encoding"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Font /{font_label}: Non-symbolic TrueType font must have /Encoding for proper character mapping"
                        ),
                        location: location.cloned(),
                    },
                });
            }
        }
        return;
    };

    // Case 1: Encoding is a name — must be a valid predefined encoding
    if let Ok(name) = enc_obj.as_name() {
        if !is_valid_encoding_name(name) {
            let name_str = String::from_utf8_lossy(name);
            results.push(CheckResult {
                rule_id: "31-005".to_string(),
                checkpoint: 31,
                description: format!(
                    "Font /{font_label}: /Encoding /{name_str} is not a valid predefined encoding"
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Font /{font_label}: /Encoding must be WinAnsiEncoding, MacRomanEncoding, MacExpertEncoding, or a dictionary — got /{name_str}"
                    ),
                    location: location.cloned(),
                },
            });
        }
        return;
    }

    // Case 2: Encoding is a dictionary
    let Ok(enc_dict) = enc_obj.as_dict() else {
        return;
    };

    let has_base = enc_dict.get(b"BaseEncoding").is_ok();
    let has_diff = enc_dict.get(b"Differences").is_ok();

    // Check /BaseEncoding if present
    if let Ok(base_enc) = enc_dict.get(b"BaseEncoding") {
        if let Ok(name) = base_enc.as_name() {
            if !is_valid_encoding_name(name) {
                let name_str = String::from_utf8_lossy(name);
                results.push(CheckResult {
                    rule_id: "31-005".to_string(),
                    checkpoint: 31,
                    description: format!(
                        "Font /{font_label}: /BaseEncoding /{name_str} is not a valid encoding"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Font /{font_label}: /BaseEncoding must be WinAnsiEncoding, MacRomanEncoding, or MacExpertEncoding — got /{name_str}"
                        ),
                        location: location.cloned(),
                    },
                });
            }
        }
    }

    // Empty encoding dictionary — no BaseEncoding and no Differences is invalid
    if !has_base && !has_diff {
        results.push(CheckResult {
            rule_id: "31-005".to_string(),
            checkpoint: 31,
            description: format!(
                "Font /{font_label}: Encoding dictionary has neither /BaseEncoding nor /Differences"
            ),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "Font /{font_label}: Encoding dictionary must have /BaseEncoding or /Differences — empty encoding is invalid"
                ),
                location: location.cloned(),
            },
        });
        return;
    }

    // Check /Differences for .notdef entries
    let Ok(diff_obj) = enc_dict.get(b"Differences") else {
        return;
    };
    let Ok(diff_arr) = diff_obj.as_array() else {
        return;
    };

    let mut notdef_count = 0;
    for item in diff_arr {
        if let Ok(name) = item.as_name() {
            if name == b".notdef" {
                notdef_count += 1;
            }
        }
    }

    if notdef_count > 0 {
        results.push(CheckResult {
            rule_id: "31-005".to_string(),
            checkpoint: 31,
            description: format!(
                "Font /{font_label}: Encoding /Differences has {notdef_count} .notdef reference(s)"
            ),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "Font /{font_label}: Encoding /Differences array contains {notdef_count} .notdef glyph name(s) — character codes cannot be mapped to glyphs"
                ),
                location: location.cloned(),
            },
        });
    }
}

/// Check if a font encoding name is a valid predefined encoding.
fn is_valid_encoding_name(name: &[u8]) -> bool {
    matches!(
        name,
        b"WinAnsiEncoding" | b"MacRomanEncoding" | b"MacExpertEncoding"
    )
}

/// 31-006: Fonts must have a `ToUnicode` `CMap` or a recognized encoding
/// so that text content can be mapped to Unicode.
#[allow(clippy::too_many_lines)]
fn check_tounicode(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    let has_tounicode = font_dict.get(b"ToUnicode").is_ok();

    if has_tounicode {
        results.push(CheckResult {
            rule_id: "31-006".to_string(),
            checkpoint: 31,
            description: format!("Font /{font_label} has ToUnicode CMap"),
            severity: Severity::Info,
            outcome: CheckOutcome::Pass,
        });
        return;
    }

    // Check for known encodings that provide implicit Unicode mapping
    let has_known_encoding = font_dict
        .get_deref(b"Encoding", doc)
        .ok()
        .is_some_and(|enc| {
            // Named encodings that have well-defined Unicode mappings
            if let Ok(name) = enc.as_name() {
                matches!(
                    name,
                    b"WinAnsiEncoding" | b"MacRomanEncoding" | b"MacExpertEncoding"
                )
            } else {
                // Dictionary encoding with /BaseEncoding
                enc.as_dict()
                    .ok()
                    .and_then(|d| d.get(b"BaseEncoding").ok())
                    .and_then(|o| o.as_name().ok())
                    .is_some_and(|n| {
                        matches!(
                            n,
                            b"WinAnsiEncoding" | b"MacRomanEncoding" | b"MacExpertEncoding"
                        )
                    })
            }
        });

    // Type0 (composite) fonts: check if they have an alternative Unicode mapping.
    // Identity-H/Identity-V CMaps combined with CIDToGIDMap provide implicit
    // Unicode mapping via glyph indices, so explicit ToUnicode is not required.
    let is_composite = font_dict
        .get_deref(b"Subtype", doc)
        .ok()
        .and_then(|o| o.as_name().ok())
        .is_some_and(|n| n == b"Type0");

    if is_composite {
        // Per PDF/UA-1 clause 7.21.7, composite fonts using well-known Adobe CID
        // character collections (Japan1, GB1, CNS1, Korea1) do NOT need an explicit
        // ToUnicode CMap — these collections have published CID-to-Unicode mappings.
        // See veraPDF test 7.21.7-t01-pass-a.pdf which validates this.
        let has_known_cid_collection = font_dict
            .get_deref(b"DescendantFonts", doc)
            .ok()
            .and_then(|o| o.as_array().ok())
            .and_then(|arr| arr.first())
            .and_then(|desc| {
                desc.as_reference()
                    .ok()
                    .and_then(|r| doc.get_object(r).ok())
                    .and_then(|o| o.as_dict().ok())
                    .or_else(|| desc.as_dict().ok())
            })
            .and_then(|dd| {
                let csi_obj = dd.get(b"CIDSystemInfo").ok()?;

                if let Ok(r) = csi_obj.as_reference() {
                    doc.get_object(r).ok()?.as_dict().ok()
                } else {
                    csi_obj.as_dict().ok()
                }
            })
            .is_some_and(|csi| {
                let registry = csi
                    .get(b"Registry")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .unwrap_or(b"");
                let ordering = csi
                    .get(b"Ordering")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .unwrap_or(b"");
                registry == b"Adobe" && matches!(ordering, b"Japan1" | b"GB1" | b"CNS1" | b"Korea1")
            });

        if has_known_cid_collection {
            results.push(CheckResult {
                rule_id: "31-006".to_string(),
                checkpoint: 31,
                description: format!(
                    "Font /{font_label} uses Adobe CID collection with known Unicode mapping"
                ),
                severity: Severity::Info,
                outcome: CheckOutcome::Pass,
            });
        } else {
            results.push(CheckResult {
                rule_id: "31-006".to_string(),
                checkpoint: 31,
                description: format!("Font /{font_label} (composite) missing ToUnicode CMap"),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Composite font /{font_label} must have a ToUnicode CMap for Unicode mapping"
                    ),
                    location: location.cloned(),
                },
            });
        }
    } else if !has_known_encoding {
        results.push(CheckResult {
            rule_id: "31-006".to_string(),
            checkpoint: 31,
            description: format!(
                "Font /{font_label} missing ToUnicode and has no standard encoding"
            ),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "Font /{font_label} has neither ToUnicode CMap nor a standard encoding — text cannot be reliably mapped to Unicode"
                ),
                location: location.cloned(),
            },
        });
    } else {
        results.push(CheckResult {
            rule_id: "31-006".to_string(),
            checkpoint: 31,
            description: format!("Font /{font_label} uses standard encoding (implicit Unicode)"),
            severity: Severity::Info,
            outcome: CheckOutcome::Pass,
        });
    }
}

/// 31-002: `CIDFontType2` fonts must have a /`CIDToGIDMap` entry.
/// 31-003: `CIDFont` encoding (`CMap`) must be a recognized name or valid stream.
#[allow(clippy::too_many_lines)]
fn check_cidfont_requirements(
    doc: &lopdf::Document,
    cid_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    let cid_subtype = cid_dict
        .get_deref(b"Subtype", doc)
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(<[u8]>::to_vec);

    // 31-002: CIDFontType2 must have /CIDToGIDMap with valid value
    if cid_subtype.as_deref() == Some(b"CIDFontType2") {
        match cid_dict.get(b"CIDToGIDMap") {
            Ok(obj) => {
                // Must be /Identity (name) or a stream reference
                let is_valid = if let Ok(name) = obj.as_name() {
                    // Only /Identity is a valid name value
                    name == b"Identity"
                } else {
                    // Stream reference is valid
                    obj.as_reference().is_ok()
                };

                if is_valid {
                    results.push(CheckResult {
                        rule_id: "31-002".to_string(),
                        checkpoint: 31,
                        description: format!(
                            "Font /{font_label}: CIDFontType2 has valid /CIDToGIDMap"
                        ),
                        severity: Severity::Info,
                        outcome: CheckOutcome::Pass,
                    });
                } else {
                    let val_desc = if let Ok(name) = obj.as_name() {
                        format!("/{}", String::from_utf8_lossy(name))
                    } else {
                        "invalid value".to_string()
                    };
                    results.push(CheckResult {
                        rule_id: "31-002".to_string(),
                        checkpoint: 31,
                        description: format!(
                            "Font /{font_label}: CIDFontType2 /CIDToGIDMap is {val_desc}"
                        ),
                        severity: Severity::Error,
                        outcome: CheckOutcome::Fail {
                            message: format!(
                                "Font /{font_label}: CIDFontType2 /CIDToGIDMap must be /Identity or a valid stream, got {val_desc}"
                            ),
                            location: location.cloned(),
                        },
                    });
                }
            }
            Err(_) => {
                results.push(CheckResult {
                    rule_id: "31-002".to_string(),
                    checkpoint: 31,
                    description: format!(
                        "Font /{font_label}: CIDFontType2 missing /CIDToGIDMap"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Font /{font_label}: CIDFontType2 must have /CIDToGIDMap (Identity or stream)"
                        ),
                        location: location.cloned(),
                    },
                });
            }
        }
    }

    // 31-004: Subset CIDFonts must have /CIDSet in FontDescriptor
    check_cidset(doc, cid_dict, font_label, location, results);

    // 31-003: CIDFont must have a valid /CIDSystemInfo
    match cid_dict.get(b"CIDSystemInfo") {
        Ok(obj) => {
            let resolved = if let Ok(r) = obj.as_reference() {
                doc.get_object(r).ok()
            } else {
                Some(obj)
            };
            if let Some(si) = resolved {
                if let Ok(si_dict) = si.as_dict() {
                    let has_registry = si_dict.get(b"Registry").is_ok();
                    let has_ordering = si_dict.get(b"Ordering").is_ok();
                    let has_supplement = si_dict.get(b"Supplement").is_ok();

                    if !has_registry || !has_ordering || !has_supplement {
                        results.push(CheckResult {
                            rule_id: "31-003".to_string(),
                            checkpoint: 31,
                            description: format!(
                                "Font /{font_label}: CIDSystemInfo incomplete"
                            ),
                            severity: Severity::Error,
                            outcome: CheckOutcome::Fail {
                                message: format!(
                                    "Font /{font_label}: CIDSystemInfo must have /Registry, /Ordering, and /Supplement"
                                ),
                                location: location.cloned(),
                            },
                        });
                    }
                }
            }
        }
        Err(_) => {
            // CIDSystemInfo is required for CIDFonts
            results.push(CheckResult {
                rule_id: "31-003".to_string(),
                checkpoint: 31,
                description: format!("Font /{font_label}: CIDFont missing /CIDSystemInfo"),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Font /{font_label}: CIDFont must have /CIDSystemInfo dictionary"
                    ),
                    location: location.cloned(),
                },
            });
        }
    }
}

/// 31-004: When a `CIDFont` has a /`CIDSet` stream, it must be a valid stream.
///
/// Note: `CIDSet` presence is required by PDF/A but NOT by PDF/UA-1 in general.
/// We only validate `CIDSet` when it exists — we don't flag its absence.
fn check_cidset(
    doc: &lopdf::Document,
    cid_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    let descriptor = cid_dict
        .get_deref(b"FontDescriptor", doc)
        .ok()
        .and_then(|o| o.as_dict().ok());

    let Some(desc) = descriptor else { return };

    // Only check CIDSet when it exists
    let Ok(cidset_obj) = desc.get(b"CIDSet") else {
        return;
    };

    // CIDSet must be a stream reference
    let Ok(cidset_ref) = cidset_obj.as_reference() else {
        results.push(CheckResult {
            rule_id: "31-004".to_string(),
            checkpoint: 31,
            description: format!("Font /{font_label}: /CIDSet is not a stream reference"),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!("Font /{font_label}: /CIDSet must be a stream reference"),
                location: location.cloned(),
            },
        });
        return;
    };

    let Ok(cidset_resolved) = doc.get_object(cidset_ref) else {
        return;
    };

    // Must be a stream object
    if cidset_resolved.as_stream().is_err() {
        results.push(CheckResult {
            rule_id: "31-004".to_string(),
            checkpoint: 31,
            description: format!("Font /{font_label}: /CIDSet does not reference a valid stream"),
            severity: Severity::Error,
            outcome: CheckOutcome::Fail {
                message: format!(
                    "Font /{font_label}: /CIDSet must reference a valid stream object"
                ),
                location: location.cloned(),
            },
        });
    }
}

/// 31-003 extended: For Type0 fonts, validate the `CMap` encoding.
///
/// Checks:
/// - Predefined `CMap` name must be a known value (Identity-H, Identity-V, or
///   one of the standard CJK `CMaps` from ISO 32000-1 Table 118).
/// - If the encoding is a `CMap` stream, `WMode` in the stream must be consistent
///   with any /`WMode` in the `CIDFont`.
/// - Registry/Ordering/Supplement in the `CMap` stream's `CIDSystemInfo` must match
///   the `CIDFont`'s `CIDSystemInfo` values.
#[allow(clippy::too_many_lines)]
fn check_type0_cmap_encoding(
    doc: &lopdf::Document,
    font_dict: &lopdf::Dictionary,
    font_label: &str,
    location: Option<&Location>,
    results: &mut Vec<CheckResult>,
) {
    let Ok(encoding_obj) = font_dict.get(b"Encoding") else {
        return;
    };

    // Check if it's a predefined CMap name
    if let Ok(name) = encoding_obj.as_name() {
        if !is_valid_predefined_cmap(name) {
            let name_str = String::from_utf8_lossy(name);
            results.push(CheckResult {
                rule_id: "31-003".to_string(),
                checkpoint: 31,
                description: format!(
                    "Font /{font_label}: Encoding /{name_str} is not a valid predefined CMap"
                ),
                severity: Severity::Error,
                outcome: CheckOutcome::Fail {
                    message: format!(
                        "Font /{font_label}: CMap name /{name_str} is not in ISO 32000-1 Table 118"
                    ),
                    location: location.cloned(),
                },
            });
        }
        return;
    }

    // It's a CMap stream reference — validate its contents
    let Ok(cmap_ref) = encoding_obj.as_reference() else {
        return;
    };
    let Ok(cmap_obj) = doc.get_object(cmap_ref) else {
        return;
    };

    // Get the CMap as a stream (it should be a PDF stream object)
    let Ok(cmap_stream) = cmap_obj.as_stream() else {
        return;
    };

    // The stream's dictionary contains /WMode, /UseCMap, /CIDSystemInfo etc.
    let cmap_dict = &cmap_stream.dict;

    // Check for /UseCMap referencing non-standard CMap
    if let Ok(usecmap) = cmap_dict.get(b"UseCMap") {
        if let Ok(name) = usecmap.as_name() {
            if !is_valid_predefined_cmap(name) {
                let name_str = String::from_utf8_lossy(name);
                results.push(CheckResult {
                    rule_id: "31-003".to_string(),
                    checkpoint: 31,
                    description: format!(
                        "Font /{font_label}: /UseCMap /{name_str} is not a valid predefined CMap"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Font /{font_label}: /UseCMap references /{name_str} which is not a standard CMap"
                        ),
                        location: location.cloned(),
                    },
                });
            }
        }
    }

    // Decompress and parse the CMap stream content
    let Ok(stream_data) = cmap_stream.decompressed_content() else {
        return;
    };

    let stream_str = String::from_utf8_lossy(&stream_data);
    let cmap_csi = extract_cmap_cidsysteminfo(&stream_str);
    let cmap_stream_wmode = extract_cmap_wmode(&stream_str);

    // Check WMode consistency between CMap dictionary and stream content
    if let Ok(dict_wmode) = cmap_dict.get(b"WMode").and_then(lopdf::Object::as_i64) {
        if let Some(stream_wmode) = cmap_stream_wmode {
            if dict_wmode != stream_wmode {
                results.push(CheckResult {
                    rule_id: "31-003".to_string(),
                    checkpoint: 31,
                    description: format!(
                        "Font /{font_label}: WMode mismatch: dict={dict_wmode}, stream={stream_wmode}"
                    ),
                    severity: Severity::Error,
                    outcome: CheckOutcome::Fail {
                        message: format!(
                            "Font /{font_label}: WMode in CMap dictionary ({dict_wmode}) doesn't match WMode in CMap stream ({stream_wmode})"
                        ),
                        location: location.cloned(),
                    },
                });
            }
        }
    }

    // Check CIDSystemInfo consistency between CMap and CIDFont
    if let Some((cmap_reg, cmap_ord, _cmap_sup)) = cmap_csi {
        if let Ok(descendants) = font_dict
            .get_deref(b"DescendantFonts", doc)
            .and_then(|o| o.as_array())
        {
            for desc in descendants {
                let desc_dict = if let Ok(r) = desc.as_reference() {
                    doc.get_object(r).ok().and_then(|o| o.as_dict().ok())
                } else {
                    desc.as_dict().ok()
                };
                let Some(dd) = desc_dict else { continue };

                let csi = dd
                    .get(b"CIDSystemInfo")
                    .ok()
                    .and_then(|o| {
                        if let Ok(r) = o.as_reference() {
                            doc.get_object(r).ok()
                        } else {
                            Some(o)
                        }
                    })
                    .and_then(|o| o.as_dict().ok());

                let Some(csi_dict) = csi else { continue };

                let font_reg = csi_dict
                    .get(b"Registry")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .map(|s| String::from_utf8_lossy(s).to_string());
                let font_ord = csi_dict
                    .get(b"Ordering")
                    .ok()
                    .and_then(|o| o.as_str().ok())
                    .map(|s| String::from_utf8_lossy(s).to_string());
                let _font_sup = csi_dict
                    .get(b"Supplement")
                    .ok()
                    .and_then(|o| o.as_i64().ok());

                // Only compare when the CMap claims to use an Adobe CJK collection
                // and the CIDFont also claims an Adobe collection
                let cmap_is_adobe_cjk = cmap_reg == "Adobe"
                    && matches!(cmap_ord.as_str(), "Japan1" | "GB1" | "CNS1" | "Korea1");

                if cmap_is_adobe_cjk {
                    if let Some(ref fr) = font_reg {
                        // Case-sensitive comparison per spec
                        if *fr != cmap_reg {
                            results.push(CheckResult {
                                rule_id: "31-003".to_string(),
                                checkpoint: 31,
                                description: format!(
                                    "Font /{font_label}: Registry mismatch: CMap={cmap_reg}, CIDFont={fr}"
                                ),
                                severity: Severity::Error,
                                outcome: CheckOutcome::Fail {
                                    message: format!(
                                        "Font /{font_label}: CMap Registry ({cmap_reg}) doesn't match CIDFont Registry ({fr})"
                                    ),
                                    location: location.cloned(),
                                },
                            });
                        }
                    }
                    if let Some(ref fo) = font_ord {
                        if *fo != cmap_ord {
                            results.push(CheckResult {
                                rule_id: "31-003".to_string(),
                                checkpoint: 31,
                                description: format!(
                                    "Font /{font_label}: Ordering mismatch: CMap={cmap_ord}, CIDFont={fo}"
                                ),
                                severity: Severity::Error,
                                outcome: CheckOutcome::Fail {
                                    message: format!(
                                        "Font /{font_label}: CMap Ordering ({cmap_ord}) doesn't match CIDFont Ordering ({fo})"
                                    ),
                                    location: location.cloned(),
                                },
                            });
                        }
                    }
                    // The CIDFont Supplement must NOT exceed the CMap Supplement.
                    // A CIDFont with a higher Supplement than the CMap references CIDs
                    // the CMap cannot map. A lower Supplement is valid (pass-d confirms).
                    if let Some(font_sup) = _font_sup {
                        if font_sup > _cmap_sup {
                            results.push(CheckResult {
                                rule_id: "31-003".to_string(),
                                checkpoint: 31,
                                description: format!(
                                    "Font /{font_label}: CIDFont Supplement ({font_sup}) exceeds CMap Supplement ({_cmap_sup})"
                                ),
                                severity: Severity::Error,
                                outcome: CheckOutcome::Fail {
                                    message: format!(
                                        "Font /{font_label}: CIDFont CIDSystemInfo Supplement ({font_sup}) is greater than CMap CIDSystemInfo Supplement ({_cmap_sup}) — CIDFont may reference CIDs the CMap cannot map"
                                    ),
                                    location: location.cloned(),
                                },
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Check if a `CMap` name is a valid predefined `CMap` from ISO 32000-1 Table 118.
fn is_valid_predefined_cmap(name: &[u8]) -> bool {
    matches!(
        name,
        // Identity
        b"Identity-H" | b"Identity-V"
        // Adobe-GB1 (Simplified Chinese)
        | b"GB-EUC-H" | b"GB-EUC-V"
        | b"GBpc-EUC-H" | b"GBpc-EUC-V"
        | b"GBK-EUC-H" | b"GBK-EUC-V"
        | b"GBKp-EUC-H" | b"GBKp-EUC-V"
        | b"GBK2K-H" | b"GBK2K-V"
        | b"UniGB-UCS2-H" | b"UniGB-UCS2-V"
        | b"UniGB-UTF16-H" | b"UniGB-UTF16-V"
        // Adobe-CNS1 (Traditional Chinese)
        | b"B5pc-H" | b"B5pc-V"
        | b"HKscs-B5-H" | b"HKscs-B5-V"
        | b"ETen-B5-H" | b"ETen-B5-V"
        | b"ETenms-B5-H" | b"ETenms-B5-V"
        | b"CNS-EUC-H" | b"CNS-EUC-V"
        | b"UniCNS-UCS2-H" | b"UniCNS-UCS2-V"
        | b"UniCNS-UTF16-H" | b"UniCNS-UTF16-V"
        // Adobe-Japan1 (Japanese)
        | b"83pv-RKSJ-H"
        | b"90ms-RKSJ-H" | b"90ms-RKSJ-V"
        | b"90msp-RKSJ-H" | b"90msp-RKSJ-V"
        | b"90pv-RKSJ-H"
        | b"Add-RKSJ-H" | b"Add-RKSJ-V"
        | b"EUC-H" | b"EUC-V"
        | b"Ext-RKSJ-H" | b"Ext-RKSJ-V"
        | b"H" | b"V"
        | b"UniJIS-UCS2-H" | b"UniJIS-UCS2-V"
        | b"UniJIS-UCS2-HW-H" | b"UniJIS-UCS2-HW-V"
        | b"UniJIS-UTF16-H" | b"UniJIS-UTF16-V"
        // Adobe-Korea1 (Korean)
        | b"KSC-EUC-H" | b"KSC-EUC-V"
        | b"KSCms-UHC-H" | b"KSCms-UHC-V"
        | b"KSCms-UHC-HW-H" | b"KSCms-UHC-HW-V"
        | b"KSCpc-EUC-H"
        | b"UniKS-UCS2-H" | b"UniKS-UCS2-V"
        | b"UniKS-UTF16-H" | b"UniKS-UTF16-V"
    )
}

/// Extract `CIDSystemInfo` (Registry, Ordering, Supplement) from a `CMap` stream.
fn extract_cmap_cidsysteminfo(stream: &str) -> Option<(String, String, i64)> {
    // Look for /CIDSystemInfo in the CMap program
    // Format: /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 6 >> def
    // Or PostScript style: /CIDSystemInfo 3 dict dup /Registry (Adobe) put ...

    // Try dict-style first
    let reg = stream.find("/Registry").and_then(|pos| {
        let after = &stream[pos..];
        let start = after.find('(')? + 1;
        let end = after[start..].find(')')? + start;
        Some(after[start..end].to_string())
    });
    let ord = stream.find("/Ordering").and_then(|pos| {
        let after = &stream[pos..];
        let start = after.find('(')? + 1;
        let end = after[start..].find(')')? + start;
        Some(after[start..end].to_string())
    });
    let sup = stream.find("/Supplement").and_then(|pos| {
        let after = &stream[pos + 11..].trim_start();
        after.split_whitespace().next()?.parse::<i64>().ok()
    });

    match (reg, ord, sup) {
        (Some(r), Some(o), Some(s)) => Some((r, o, s)),
        _ => None,
    }
}

/// Extract `WMode` value from `CMap` stream content.
fn extract_cmap_wmode(stream: &str) -> Option<i64> {
    // Look for: /WMode 0 def  or  /WMode 1 def
    stream.find("/WMode").and_then(|pos| {
        let after = stream[pos + 6..].trim_start();
        after.split_whitespace().next()?.parse::<i64>().ok()
    })
}

/// Remove duplicate results for the same font appearing on multiple pages.
fn dedup_results(results: &mut Vec<CheckResult>) {
    let mut seen = std::collections::HashSet::new();
    results.retain(|r| {
        let key = format!("{}:{}", r.rule_id, r.description);
        seen.insert(key)
    });
}
