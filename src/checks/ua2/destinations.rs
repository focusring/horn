//! PDF/UA-2 intra-document destinations (ISO 14289-2, 8.8).
//!
//! PDF 2.0 introduces *structure destinations*: destination arrays whose
//! first element is a structure element instead of a page. PDF/UA-2 requires
//! every intra-document destination — outline items, link annotations and
//! `GoTo` actions — to be a structure destination, so that assistive
//! technology lands on the right place in the logical structure rather than
//! on a page position.
//!
//! - `ua2:8.8-1`: a destination (`/SD`, or `/Dest` when no `/SD` is given) is not a
//!   structure destination
//! - `ua2:8.8-2`: a `GoTo` action has no structure destination (`/SD`)

use super::{decode_text_string, fail, pass, resolve, resolve_dict};
use crate::checks::Check;
use crate::document::HornDocument;
use crate::model::{CheckResult, Standard};
use anyhow::Result;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashSet;

pub struct Ua2DestinationChecks;

impl Check for Ua2DestinationChecks {
    fn id(&self) -> &'static str {
        "ua2-destinations"
    }

    fn checkpoint(&self) -> u8 {
        27
    }

    fn rules(&self) -> &'static [&'static str] {
        &["ua2:8.8-1", "ua2:8.8-2"]
    }

    fn description(&self) -> &'static str {
        "PDF/UA-2: outline, link and GoTo destinations are structure destinations"
    }

    fn supports(&self, standard: Standard) -> bool {
        standard == Standard::Ua2
    }

    fn run(&self, doc: &mut HornDocument) -> Result<Vec<CheckResult>> {
        let mut results = Vec::new();
        let lopdf = doc.lopdf();
        let Ok(catalog) = lopdf.catalog() else {
            return Ok(results);
        };

        let mut checker = Checker {
            doc: lopdf,
            names: NamedDestinations::load(lopdf, catalog),
            destinations: 0,
            goto_actions: 0,
            results: Vec::new(),
        };

        // Outline items
        if let Some(outlines) = catalog
            .get(b"Outlines")
            .ok()
            .and_then(|o| resolve_dict(lopdf, o))
        {
            let mut visited = HashSet::new();
            if let Ok(first) = outlines.get(b"First") {
                checker.walk_outline(first, &mut visited, 0);
            }
        }

        // Link annotations
        for (page_num, page_id) in lopdf.get_pages() {
            let Ok(page) = lopdf.get_dictionary(page_id) else {
                continue;
            };
            let Some(annots) = page
                .get(b"Annots")
                .ok()
                .and_then(|o| resolve(lopdf, o))
                .and_then(|o| o.as_array().ok())
            else {
                continue;
            };
            for annot_ref in annots {
                let Some(annot) = resolve_dict(lopdf, annot_ref) else {
                    continue;
                };
                if annot.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) != Some(b"Link") {
                    continue;
                }
                let id = annot_ref
                    .as_reference()
                    .map_or_else(|_| String::new(), |r| format!(" (obj {}.{})", r.0, r.1));
                checker.check_holder(annot, &format!("Page {page_num}: Link annotation{id}"));
            }
        }

        let Checker {
            destinations,
            goto_actions,
            results: found,
            ..
        } = checker;
        let dest_failures = found.iter().filter(|r| r.rule_id == "ua2:8.8-1").count();
        let goto_failures = found.iter().filter(|r| r.rule_id == "ua2:8.8-2").count();
        results.extend(found);
        if destinations > 0 && dest_failures == 0 {
            results.push(pass(
                "ua2:8.8-1",
                format!(
                    "All {destinations} intra-document destination(s) are structure destinations"
                ),
            ));
        }
        if goto_actions > 0 && goto_failures == 0 {
            results.push(pass(
                "ua2:8.8-2",
                format!("All {goto_actions} GoTo action(s) carry a structure destination (/SD)"),
            ));
        }

        Ok(results)
    }
}

struct Checker<'a> {
    doc: &'a Document,
    names: NamedDestinations<'a>,
    destinations: usize,
    goto_actions: usize,
    results: Vec<CheckResult>,
}

impl<'a> Checker<'a> {
    fn walk_outline(
        &mut self,
        item_obj: &'a Object,
        visited: &mut HashSet<ObjectId>,
        depth: usize,
    ) {
        if depth > 64 {
            return;
        }
        let mut current = Some(item_obj);
        let mut count = 0usize;
        while let Some(obj) = current {
            count += 1;
            if count > 10_000 {
                return;
            }
            if let Ok(id) = obj.as_reference() {
                if !visited.insert(id) {
                    return;
                }
            }
            let Some(item) = resolve_dict(self.doc, obj) else {
                return;
            };
            let title = item
                .get(b"Title")
                .ok()
                .and_then(|o| resolve(self.doc, o))
                .and_then(|o| o.as_str().ok())
                .map(decode_text_string)
                .unwrap_or_default();
            let shown: String = title.chars().take(50).collect();
            self.check_holder(item, &format!("Outline item \"{shown}\""));

            if let Ok(first) = item.get(b"First") {
                self.walk_outline(first, visited, depth + 1);
            }
            current = item.get(b"Next").ok();
        }
    }

    /// Check the destination(s) of an outline item or link annotation.
    fn check_holder(&mut self, dict: &'a Dictionary, label: &str) {
        if let Ok(sd) = dict.get(b"SD") {
            self.destinations += 1;
            if !self.is_struct_dest(sd) {
                self.results.push(fail(
                    "ua2:8.8-1",
                    format!("{label}: /SD is not a structure destination (its first element is not a structure element)"),
                    None,
                ));
            }
        } else if let Ok(dest) = dict.get(b"Dest") {
            self.destinations += 1;
            if !self.is_struct_dest(dest) {
                self.results.push(fail(
                    "ua2:8.8-1",
                    format!("{label}: destination is a page destination, not a structure destination — add an /SD entry"),
                    None,
                ));
            }
        }

        // Action chain (/A, then /Next)
        let mut action = dict.get(b"A").ok().and_then(|o| resolve_dict(self.doc, o));
        let mut hops = 0;
        while let Some(a) = action {
            hops += 1;
            if hops > 16 {
                break;
            }
            let kind = a
                .get(b"S")
                .ok()
                .and_then(|o| o.as_name().ok())
                .unwrap_or(b"");
            if kind == b"GoTo" {
                self.goto_actions += 1;
                if let Ok(sd) = a.get(b"SD") {
                    self.destinations += 1;
                    if !self.is_struct_dest(sd) {
                        self.results.push(fail(
                            "ua2:8.8-1",
                            format!("{label}: GoTo action /SD is not a structure destination"),
                            None,
                        ));
                    }
                } else if !a.get(b"D").ok().is_some_and(|d| self.is_struct_dest(d)) {
                    self.results.push(fail(
                        "ua2:8.8-2",
                        format!("{label}: GoTo action has no structure destination (/SD)"),
                        None,
                    ));
                }
            }
            action = a
                .get(b"Next")
                .ok()
                .and_then(|o| resolve(self.doc, o))
                .and_then(|o| match o {
                    Object::Dictionary(d) => Some(d),
                    Object::Array(arr) => arr.first().and_then(|f| resolve_dict(self.doc, f)),
                    _ => None,
                });
        }
    }

    /// Whether a destination (explicit array, named destination, or destination
    /// dictionary) is a structure destination.
    fn is_struct_dest(&self, obj: &'a Object) -> bool {
        self.is_struct_dest_depth(obj, 0)
    }

    fn is_struct_dest_depth(&self, obj: &'a Object, depth: usize) -> bool {
        if depth > 4 {
            return false;
        }
        let Some(resolved) = resolve(self.doc, obj) else {
            return false;
        };
        match resolved {
            Object::Array(arr) => arr
                .first()
                .and_then(|first| resolve_dict(self.doc, first))
                .is_some_and(is_struct_elem),
            Object::Dictionary(d) => d
                .get(b"SD")
                .or_else(|_| d.get(b"D"))
                .ok()
                .is_some_and(|inner| self.is_struct_dest_depth(inner, depth + 1)),
            Object::Name(name) => self
                .names
                .lookup(name)
                .is_some_and(|v| self.is_struct_dest_depth(v, depth + 1)),
            Object::String(bytes, _) => self
                .names
                .lookup(bytes)
                .is_some_and(|v| self.is_struct_dest_depth(v, depth + 1)),
            _ => false,
        }
    }
}

fn is_struct_elem(dict: &Dictionary) -> bool {
    dict.get(b"Type").ok().and_then(|o| o.as_name().ok()) == Some(b"StructElem")
        || (dict.get(b"S").is_ok()
            && dict.get(b"Type").ok().and_then(|o| o.as_name().ok()) != Some(b"Page"))
}

/// Named destinations from the catalog `/Dests` dictionary and the `/Names /Dests` name tree.
struct NamedDestinations<'a> {
    doc: &'a Document,
    dests: Option<&'a Dictionary>,
    tree: Option<&'a Dictionary>,
}

impl<'a> NamedDestinations<'a> {
    fn load(doc: &'a Document, catalog: &'a Dictionary) -> Self {
        Self {
            doc,
            dests: catalog
                .get(b"Dests")
                .ok()
                .and_then(|o| resolve_dict(doc, o)),
            tree: catalog
                .get(b"Names")
                .ok()
                .and_then(|o| resolve_dict(doc, o))
                .and_then(|n| n.get(b"Dests").ok())
                .and_then(|o| resolve_dict(doc, o)),
        }
    }

    fn lookup(&self, name: &[u8]) -> Option<&'a Object> {
        if let Some(v) = self.dests.and_then(|d| d.get(name).ok()) {
            return Some(v);
        }
        self.tree.and_then(|t| self.lookup_in_tree(t, name, 0))
    }

    fn lookup_in_tree(
        &self,
        node: &'a Dictionary,
        name: &[u8],
        depth: usize,
    ) -> Option<&'a Object> {
        if depth > 32 {
            return None;
        }
        if let Some(names) = node
            .get(b"Names")
            .ok()
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_array().ok())
        {
            for pair in names.chunks(2) {
                if pair.len() == 2 && pair[0].as_str().ok() == Some(name) {
                    return Some(&pair[1]);
                }
            }
        }
        let kids = node
            .get(b"Kids")
            .ok()
            .and_then(|o| resolve(self.doc, o))
            .and_then(|o| o.as_array().ok())?;
        kids.iter()
            .filter_map(|k| resolve_dict(self.doc, k))
            .find_map(|child| self.lookup_in_tree(child, name, depth + 1))
    }
}
