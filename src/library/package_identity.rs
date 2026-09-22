// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural reading of a CheckedPackage V2 `quire.checked-package-id/v2`
//! identity preimage: its version, required members and the
//! `identity_projection` nodes from which FR-307 export node keys are derived.
//!
//! FR-087 (#213 S-3a, owner ruling on QSL-158, 2026-09-21, item 3(d)):
//! relocated from `value::package_identity`. Every field or return value
//! here that is built from wire-read or preimage-read data is typed
//! [`super::WireNodeId`], never `quire_exact::NodeKey` (ADR-013 R-10, O-04):
//! a wire node id becomes a `NodeKey` only by lookup in a QSL checked
//! package, a lookup this module never performs.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::digest::WireNodeId;
use crate::value::node::{is_qualified_name, NODE_KEY_DOMAIN};

/// The identity preimage version constant.
pub(crate) const PACKAGE_ID_VERSION: &str = "quire.checked-package-id/v2";
/// The node projection schema version constant.
pub(crate) const NODE_SCHEMA_VERSION: &str = "quire.checked-semantic-graph/v2";

const PREIMAGE_REQUIRED: [&str; 8] = [
    "version",
    "edition",
    "profile_selections",
    "definition_selections",
    "model_selections",
    "required_features",
    "dependency_selections",
    "identity_projection",
];

const NODE_REQUIRED: [&str; 7] = [
    "node_id",
    "schema_version",
    "node_tag",
    "semantic_form",
    "semantic_type",
    "dependencies",
    "body",
];

/// `declaration` is the projection node's own name member (checked-package-v2
/// README.md's "Declarations" section): a named, source-declared node carries
/// `declaration: {qualified_name}` exactly when it has a declaration source
/// occurrence. On a nominal node it must equal the nominal
/// `qualified_declaration`, and every export resolves only through this
/// member, never through `nominal_identity_preimage` directly.
const NODE_OPTIONAL: [&str; 3] = [
    "recursion_group",
    "nominal_identity_preimage",
    "declaration",
];

/// Why an identity preimage is structurally malformed.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PreimageDefect {
    /// The bytes are not one JSON object.
    NotObject,
    /// The bytes are not the canonical serialization of their JSON value, so
    /// one logical preimage could carry two `package_id`s. See
    /// `project_declarations` for the exact form enforced.
    NonCanonical,
    /// A required member is absent.
    MissingMember(&'static str),
    /// A member the schema does not define is present.
    UnknownMember(String),
    /// `version` is not `quire.checked-package-id/v2`.
    Version,
    /// A member other than `identity_projection` has the wrong JSON type.
    MemberType(&'static str),
    /// `identity_projection` is not a non-empty array.
    EmptyProjection,
    /// Projection node `index` is malformed.
    Node {
        /// The node's index in `identity_projection`.
        index: usize,
        /// What is malformed.
        defect: NodeDefect,
    },
    /// Projection node `index` does not follow its predecessor in strictly
    /// ascending node id order.
    NodeOrder {
        /// The node's index in `identity_projection`.
        index: usize,
    },
    /// Two projection nodes carry equal `declaration.qualified_name` values
    /// (`ambiguous-name`). `nodes` holds both wire node ids in ascending
    /// digest order.
    AmbiguousDeclaration {
        /// The repeated qualified name.
        name: String,
        /// Both wire node ids, in ascending digest order.
        nodes: [WireNodeId; 2],
    },
    /// A node's top-level `declaration.qualified_name` disagrees with its
    /// nominal `qualified_declaration`, or is absent while a nominal
    /// `qualified_declaration` is present (`declaration-nominal-mismatch`).
    DeclarationNominalMismatch {
        /// The node's wire id.
        node: WireNodeId,
        /// The node's top-level `declaration.qualified_name`, or `None` when
        /// `declaration` is absent.
        declared: Option<String>,
        /// The node's nominal `qualified_declaration`.
        nominal: String,
    },
}

/// Why one projection node is malformed.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum NodeDefect {
    /// The node is not a JSON object.
    NotObject,
    /// A required member is absent.
    MissingMember(&'static str),
    /// A member the schema does not define is present.
    UnknownMember(String),
    /// `node_id` is not `{domain: quire.checked-semantic-node/v1, digest}`
    /// with a 64-lowercase-hex digest.
    NodeId,
    /// `schema_version` is not `quire.checked-semantic-graph/v2`.
    SchemaVersion,
    /// `node_tag` is not a V2 semantic graph node tag.
    NodeTag,
    /// A nominal `qualified_declaration` or a top-level `declaration` is not
    /// a non-empty identifier array, or `declaration` is not exactly
    /// `{qualified_name}`.
    Declaration,
}

/// The validated wire node ids of one identity preimage, by the top-level
/// `declaration.qualified_name` each node spells, spelled with `::`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProjectedDeclarations(BTreeMap<String, WireNodeId>);

impl ProjectedDeclarations {
    /// The wire node id declaring `name`.
    pub(crate) fn node(&self, name: &str) -> Option<WireNodeId> {
        self.0.get(name).copied()
    }

    /// The declarations of `exports` only, or the first export no nominal
    /// declaration spells.
    pub(crate) fn select<'a>(&self, exports: &'a [String]) -> Result<Self, &'a str> {
        exports
            .iter()
            .map(|name| {
                self.node(name)
                    .map(|key| (name.clone(), key))
                    .ok_or(name.as_str())
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

fn members<'a>(
    value: &'a Value,
    required: &[&'static str],
    optional: &[&'static str],
) -> Result<&'a Map<String, Value>, MemberDefect> {
    let object = value.as_object().ok_or(MemberDefect::NotObject)?;
    if let Some(missing) = required.iter().find(|name| !object.contains_key(**name)) {
        return Err(MemberDefect::Missing(missing));
    }
    if let Some(unknown) = object
        .keys()
        .find(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(MemberDefect::Unknown(unknown.clone()));
    }
    Ok(object)
}

enum MemberDefect {
    NotObject,
    Missing(&'static str),
    Unknown(String),
}

/// The wire node id `{domain: quire.checked-semantic-node/v1, digest}`
/// object names, without becoming a `NodeKey` (R-10, O-04).
fn wire_node_id(value: &Value) -> Option<WireNodeId> {
    let object = value.as_object()?;
    if object.len() != 2 || object.get("domain")?.as_str()? != NODE_KEY_DOMAIN {
        return None;
    }
    WireNodeId::from_hex(object.get("digest")?.as_str()?)
}

/// The node tags of the V2 semantic graph.
const NODE_TAGS: [&str; 13] = [
    "scalar_type",
    "composite_type",
    "bounded_domain",
    "value",
    "expression",
    "function",
    "model",
    "relation",
    "state",
    "temporal",
    "protocol",
    "claim",
    "correspondence",
];

fn qualified(segments: Vec<String>) -> Result<String, NodeDefect> {
    if is_qualified_name(&segments) {
        Ok(segments.join("::"))
    } else {
        Err(NodeDefect::Declaration)
    }
}

/// The `qualified_declaration` of a nominal identity preimage, when the
/// preimage form carries one (an enum member does not).
fn nominal_declaration(nominal: &Value) -> Result<Option<String>, NodeDefect> {
    let Some(segments) = nominal
        .as_object()
        .ok_or(NodeDefect::Declaration)?
        .get("qualified_declaration")
    else {
        return Ok(None);
    };
    let segments = segments
        .as_array()
        .ok_or(NodeDefect::Declaration)?
        .iter()
        .map(|segment| segment.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or(NodeDefect::Declaration)?;
    qualified(segments).map(Some)
}

/// The `qualified_name` of a top-level `declaration` member, which must be
/// exactly `{qualified_name}`.
fn declared_name(value: &Value) -> Result<String, NodeDefect> {
    let object = value.as_object().ok_or(NodeDefect::Declaration)?;
    if object.len() != 1 {
        return Err(NodeDefect::Declaration);
    }
    let segments = object
        .get("qualified_name")
        .and_then(Value::as_array)
        .ok_or(NodeDefect::Declaration)?
        .iter()
        .map(|segment| segment.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or(NodeDefect::Declaration)?;
    qualified(segments)
}

/// A projection node's wire id and its shape-valid `declaration
/// .qualified_name` and nominal `qualified_declaration`, before the
/// cross-node checks that order them: whether `declared` and `nominal`
/// agree, and whether `declared` repeats an earlier node's.
struct NodeShape {
    key: WireNodeId,
    declared: Option<String>,
    nominal: Option<String>,
}

/// The shape-valid `declaration.qualified_name` and nominal
/// `qualified_declaration` of one node's `declaration` and
/// `nominal_identity_preimage` members, without checking whether they agree.
fn declaration_shape(
    node: &Map<String, Value>,
) -> Result<(Option<String>, Option<String>), NodeDefect> {
    let nominal = node
        .get("nominal_identity_preimage")
        .map(nominal_declaration)
        .transpose()?
        .flatten();
    let declared = node.get("declaration").map(declared_name).transpose()?;
    Ok((declared, nominal))
}

fn projected_node(value: &Value) -> Result<NodeShape, NodeDefect> {
    let node = members(value, &NODE_REQUIRED, &NODE_OPTIONAL).map_err(|defect| match defect {
        MemberDefect::NotObject => NodeDefect::NotObject,
        MemberDefect::Missing(name) => NodeDefect::MissingMember(name),
        MemberDefect::Unknown(name) => NodeDefect::UnknownMember(name),
    })?;
    let key = node
        .get("node_id")
        .and_then(wire_node_id)
        .ok_or(NodeDefect::NodeId)?;
    if node.get("schema_version").and_then(Value::as_str) != Some(NODE_SCHEMA_VERSION) {
        return Err(NodeDefect::SchemaVersion);
    }
    node.get("node_tag")
        .and_then(Value::as_str)
        .filter(|tag| NODE_TAGS.contains(tag))
        .ok_or(NodeDefect::NodeTag)?;
    let (declared, nominal) = declaration_shape(node)?;
    Ok(NodeShape {
        key,
        declared,
        nominal,
    })
}

/// `shape`'s `declaration-nominal-mismatch` defect, if its top-level
/// `declaration.qualified_name` disagrees with its nominal
/// `qualified_declaration`, or is absent while a nominal
/// `qualified_declaration` is present.
fn declaration_mismatch(shape: &NodeShape) -> Option<PreimageDefect> {
    match (&shape.declared, &shape.nominal) {
        (Some(declared), Some(nominal)) if declared != nominal => {
            Some(PreimageDefect::DeclarationNominalMismatch {
                node: shape.key,
                declared: Some(declared.clone()),
                nominal: nominal.clone(),
            })
        }
        (None, Some(nominal)) => Some(PreimageDefect::DeclarationNominalMismatch {
            node: shape.key,
            declared: None,
            nominal: nominal.clone(),
        }),
        _ => None,
    }
}

/// Validate `bytes` as an identity preimage and derive the wire node id of
/// each nominal declaration in its `identity_projection`.
///
/// Canonicity is enforced by requiring `bytes` to be byte-identical to the
/// re-serialization of the JSON value they parse to: object members in
/// ascending UTF-8 byte order, no insignificant whitespace and one string and
/// number spelling per value. That is RFC 8785 JCS for every preimage whose
/// member names are ASCII, which is every name this schema defines and every
/// name QSpec 82f84d3 gives a projection node. It is stricter than JCS, never
/// weaker: a preimage that JCS would order differently (member names outside
/// ASCII, whose UTF-16 code-unit order differs from their UTF-8 byte order) is
/// refused as [`PreimageDefect::NonCanonical`] rather than admitted under a
/// second `package_id`. Admitting those names needs a real JCS encoder, which
/// is the Complete-V1 writer's contract, not this reader's.
pub(crate) fn project_declarations(bytes: &[u8]) -> Result<ProjectedDeclarations, PreimageDefect> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| PreimageDefect::NotObject)?;
    let preimage = members(&value, &PREIMAGE_REQUIRED, &[]).map_err(|defect| match defect {
        MemberDefect::NotObject => PreimageDefect::NotObject,
        MemberDefect::Missing(name) => PreimageDefect::MissingMember(name),
        MemberDefect::Unknown(name) => PreimageDefect::UnknownMember(name),
    })?;
    if serde_json::to_vec(&value).ok().as_deref() != Some(bytes) {
        return Err(PreimageDefect::NonCanonical);
    }
    if preimage.get("version").and_then(Value::as_str) != Some(PACKAGE_ID_VERSION) {
        return Err(PreimageDefect::Version);
    }
    if !preimage.get("edition").is_some_and(Value::is_object) {
        return Err(PreimageDefect::MemberType("edition"));
    }
    if let Some(name) = PREIMAGE_REQUIRED[2..7]
        .iter()
        .find(|name| !preimage.get(**name).is_some_and(Value::is_array))
    {
        return Err(PreimageDefect::MemberType(name));
    }
    let nodes = preimage
        .get("identity_projection")
        .and_then(Value::as_array)
        .filter(|nodes| !nodes.is_empty())
        .ok_or(PreimageDefect::EmptyProjection)?;

    // Pass 1 (native-diagnostics.md's refusal order, step 1): every node's
    // own shape, and the projection's strictly ascending node-id order. This
    // pass runs to completion across every node before pass 2 begins, so a
    // schema refusal anywhere in the projection outranks a mismatch or an
    // ambiguity found by scanning fewer nodes.
    let mut shapes = Vec::with_capacity(nodes.len());
    let mut previous: Option<WireNodeId> = None;
    for (index, node) in nodes.iter().enumerate() {
        let shape =
            projected_node(node).map_err(|defect| PreimageDefect::Node { index, defect })?;
        if previous.is_some_and(|previous| previous >= shape.key) {
            return Err(PreimageDefect::NodeOrder { index });
        }
        previous = Some(shape.key);
        shapes.push(shape);
    }

    // Pass 2 (step 4): declaration-nominal-mismatch, in ascending node-id
    // order (`shapes` is already ordered that way by pass 1's NodeOrder
    // check). This pass runs to completion across every node before pass 3
    // begins, so a mismatch anywhere outranks an ambiguity at an earlier
    // node.
    for shape in &shapes {
        if let Some(defect) = declaration_mismatch(shape) {
            return Err(defect);
        }
    }

    // Pass 3 (step 5): ambiguous-name, in ascending node-id order.
    let mut declarations = BTreeMap::new();
    for shape in &shapes {
        if let Some(declared) = &shape.declared {
            if let Some(earlier) = declarations.insert(declared.clone(), shape.key) {
                return Err(PreimageDefect::AmbiguousDeclaration {
                    name: declared.clone(),
                    nodes: [earlier, shape.key],
                });
            }
        }
    }
    Ok(ProjectedDeclarations(declarations))
}
