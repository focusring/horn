//! PDF 2.0 structure namespace helpers (ISO 32000-2 §14.7.4).
//!
//! PDF 2.0 (and therefore PDF/UA-2) lets structure elements declare a namespace
//! via `/NS`. Namespace dictionaries carry a `/NS` URI and an optional
//! `/RoleMapNS` that maps element types to types in (possibly other) namespaces.
//! Elements without `/NS` live in the default PDF 1.7 namespace.
//!
//! These helpers resolve an element's type through both `/RoleMapNS` and the
//! document-level `/RoleMap` so checks can reason about *standard* types and
//! namespaces regardless of how the producer spelled them.

/// Namespace name of the PDF 1.7 standard structure types (the default namespace).
pub const NS_PDF17: &str = "http://iso.org/pdf/ssn";
/// Namespace name of the PDF 2.0 standard structure types.
pub const NS_PDF2: &str = "http://iso.org/pdf2/ssn";
/// Namespace name of `MathML` structure elements.
pub const NS_MATHML: &str = "http://www.w3.org/1998/Math/MathML";

/// A structure type qualified by its namespace.
///
/// `namespace == None` means the default (PDF 1.7) namespace, i.e. no `/NS` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedType {
    pub name: Vec<u8>,
    pub namespace: Option<String>,
}

impl QualifiedType {
    /// Whether this type lives in one of the standard namespaces
    /// (PDF 1.7 default, PDF 2.0, or `MathML`).
    pub fn is_standard_namespace(&self) -> bool {
        match &self.namespace {
            None => true,
            Some(ns) => ns == NS_PDF17 || ns == NS_PDF2 || ns == NS_MATHML,
        }
    }

    /// Whether this is the `MathML` namespace.
    pub fn is_mathml(&self) -> bool {
        self.namespace.as_deref() == Some(NS_MATHML)
    }

    /// Whether this resolves to a standard structure type in a standard namespace.
    pub fn is_standard(&self) -> bool {
        if self.is_mathml() {
            return true;
        }
        self.is_standard_namespace() && is_standard_structure_type(&self.name)
    }
}

/// Resolve an object to a dictionary, following one level of indirection.
pub fn resolve_dict<'a>(
    doc: &'a lopdf::Document,
    obj: &'a lopdf::Object,
) -> Option<&'a lopdf::Dictionary> {
    match obj {
        lopdf::Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok(),
        lopdf::Object::Dictionary(d) => Some(d),
        _ => None,
    }
}

/// The namespace name (URI) of a namespace dictionary.
pub fn namespace_uri(ns_dict: &lopdf::Dictionary) -> Option<String> {
    let ns = ns_dict.get(b"NS").ok()?;
    let bytes = ns.as_str().or_else(|_| ns.as_name()).ok()?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// The namespace dictionary referenced by an element's `/NS` entry, if any.
pub fn element_namespace_dict<'a>(
    doc: &'a lopdf::Document,
    elem: &'a lopdf::Dictionary,
) -> Option<&'a lopdf::Dictionary> {
    resolve_dict(doc, elem.get(b"NS").ok()?)
}

/// The namespace name (URI) declared on an element via `/NS`, if any.
pub fn element_namespace(doc: &lopdf::Document, elem: &lopdf::Dictionary) -> Option<String> {
    element_namespace_dict(doc, elem).and_then(namespace_uri)
}

/// The document-level `/RoleMap` from the structure tree root, if present.
pub fn role_map<'a>(
    doc: &'a lopdf::Document,
    struct_tree_root: &'a lopdf::Dictionary,
) -> Option<&'a lopdf::Dictionary> {
    struct_tree_root
        .get_deref(b"RoleMap", doc)
        .ok()
        .and_then(|o| o.as_dict().ok())
}

/// One hop of role mapping for a qualified type.
///
/// Uses the namespace's `/RoleMapNS` when the type is in a non-standard
/// namespace, and the document `/RoleMap` for types in the default (PDF 1.7)
/// namespace. Types in the PDF 2.0 and `MathML` namespaces are never role
/// mapped: a standard namespace only contains its standard types
/// (ISO 32000-2, 14.8.6.2).
fn role_map_hop<'a>(
    doc: &'a lopdf::Document,
    role_map: Option<&'a lopdf::Dictionary>,
    current: &QualifiedType,
    ns_dict: Option<&'a lopdf::Dictionary>,
) -> Option<(QualifiedType, Option<&'a lopdf::Dictionary>)> {
    if matches!(current.namespace.as_deref(), Some(NS_PDF2 | NS_MATHML)) {
        return None;
    }
    let in_default_namespace = matches!(current.namespace.as_deref(), None | Some(NS_PDF17));
    if let Some(nsd) = ns_dict.filter(|_| !in_default_namespace) {
        let entry = nsd
            .get_deref(b"RoleMapNS", doc)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|rm| rm.get(&current.name).ok());
        if let Some(target) = entry {
            return role_map_ns_target(doc, target);
        }
    }

    // Default namespace (or no RoleMapNS entry): document-level RoleMap
    let target = role_map?.get(&current.name).ok()?.as_name().ok()?;
    Some((
        QualifiedType {
            name: target.to_vec(),
            namespace: None,
        },
        None,
    ))
}

/// Interpret a `/RoleMapNS` value: either a name (default namespace) or
/// a two-element array `[name, namespaceDict]`.
pub fn role_map_ns_target<'a>(
    doc: &'a lopdf::Document,
    target: &'a lopdf::Object,
) -> Option<(QualifiedType, Option<&'a lopdf::Dictionary>)> {
    if let Ok(name) = target.as_name() {
        return Some((
            QualifiedType {
                name: name.to_vec(),
                namespace: None,
            },
            None,
        ));
    }
    let arr = target.as_array().ok()?;
    let name = arr.first()?.as_name().ok()?;
    let ns_dict = arr.get(1).and_then(|o| resolve_dict(doc, o));
    Some((
        QualifiedType {
            name: name.to_vec(),
            namespace: ns_dict.and_then(namespace_uri),
        },
        ns_dict,
    ))
}

/// Resolve an element's structure type through role maps until it reaches a
/// standard type in a standard namespace (or the chain ends / loops).
pub fn resolve_qualified_type(
    doc: &lopdf::Document,
    role_map: Option<&lopdf::Dictionary>,
    elem: &lopdf::Dictionary,
) -> QualifiedType {
    let Some(name) = elem.get(b"S").ok().and_then(|o| o.as_name().ok()) else {
        return QualifiedType {
            name: Vec::new(),
            namespace: None,
        };
    };

    let mut ns_dict = element_namespace_dict(doc, elem);
    let mut current = QualifiedType {
        name: name.to_vec(),
        namespace: ns_dict.and_then(namespace_uri),
    };

    for _ in 0..32 {
        if current.is_standard() {
            break;
        }
        match role_map_hop(doc, role_map, &current, ns_dict) {
            Some((next, next_ns)) if next != current => {
                current = next;
                ns_dict = next_ns;
            }
            _ => break,
        }
    }

    current
}

/// Standard structure types from ISO 32000-1 (Tables 333-338) and ISO 32000-2 (Tables 364-370),
/// plus the `Math` name some producers use for `MathML` containers.
pub fn is_standard_structure_type(name: &[u8]) -> bool {
    // Hn with n >= 1 (PDF 2.0 allows levels beyond H6)
    if let Some(digits) = name.strip_prefix(b"H") {
        if !digits.is_empty() && digits.iter().all(u8::is_ascii_digit) && digits[0] != b'0' {
            return true;
        }
    }

    matches!(
        name,
        // Grouping elements
        b"Document" | b"DocumentFragment" | b"Part" | b"Art" | b"Sect" | b"Div"
        | b"BlockQuote" | b"Caption" | b"TOC" | b"TOCI" | b"Index"
        | b"NonStruct" | b"Private" | b"Aside" | b"Title"
        // Block-level structure
        | b"H" | b"P" | b"L" | b"LI" | b"Lbl" | b"LBody"
        // Table elements
        | b"Table" | b"TR" | b"TH" | b"TD" | b"THead" | b"TBody" | b"TFoot"
        // Inline elements
        | b"Span" | b"Quote" | b"Note" | b"Reference" | b"BibEntry"
        | b"Code" | b"Link" | b"Annot" | b"FENote" | b"Sub" | b"Em" | b"Strong"
        // Illustration elements
        | b"Figure" | b"Formula" | b"Form"
        // Ruby/Warichu
        | b"Ruby" | b"RB" | b"RT" | b"RP"
        | b"Warichu" | b"WT" | b"WP"
        // PDF 2.0 artifact structure element
        | b"Artifact"
        // MathML container as used by some producers
        | b"Math"
        // StructTreeRoot itself
        | b"StructTreeRoot"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_levels_are_standard() {
        assert!(is_standard_structure_type(b"H1"));
        assert!(is_standard_structure_type(b"H7"));
        assert!(is_standard_structure_type(b"H"));
        assert!(!is_standard_structure_type(b"H0"));
        assert!(!is_standard_structure_type(b"Hx"));
        assert!(!is_standard_structure_type(b"Heading"));
    }

    #[test]
    fn qualified_type_standardness() {
        let default_p = QualifiedType {
            name: b"P".to_vec(),
            namespace: None,
        };
        assert!(default_p.is_standard());

        let custom = QualifiedType {
            name: b"P".to_vec(),
            namespace: Some("http://example.com/ns".to_string()),
        };
        assert!(!custom.is_standard());

        let math = QualifiedType {
            name: b"mrow".to_vec(),
            namespace: Some(NS_MATHML.to_string()),
        };
        assert!(math.is_standard());
    }
}
