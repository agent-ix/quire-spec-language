// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural reading of a CheckedPackage V2 `quire.checked-package-id/v2`
//! identity preimage: its version, required members and the
//! `identity_projection` nodes from which FR-307 export node keys are derived.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::node::{is_qualified_name, NodeKey, NODE_KEY_DOMAIN};

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

const NODE_OPTIONAL: [&str; 2] = ["recursion_group", "nominal_identity_preimage"];

/// Why an identity preimage is structurally malformed.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PreimageDefect {
    /// The bytes are not one JSON object.
    NotObject,
    /// The bytes are not the canonical serialization of their JSON value, so
    /// one logical preimage could carry two `package_id`s. See
    /// [`project_declarations`] for the exact form enforced.
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
    /// Projection node `index` repeats an earlier node's qualified declaration.
    DuplicateDeclaration {
        /// The node's index in `identity_projection`.
        index: usize,
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
    /// A nominal `qualified_declaration` is not a non-empty identifier array.
    Declaration,
}

/// The validated node keys of one identity preimage, by nominal qualified
/// declaration spelled with `::`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProjectedDeclarations(BTreeMap<String, NodeKey>);

impl ProjectedDeclarations {
    /// The node key declaring `name`.
    pub(crate) fn node(&self, name: &str) -> Option<NodeKey> {
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

fn node_key(value: &Value) -> Option<NodeKey> {
    let object = value.as_object()?;
    if object.len() != 2 || object.get("domain")?.as_str()? != NODE_KEY_DOMAIN {
        return None;
    }
    NodeKey::from_hex(object.get("digest")?.as_str()?)
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

/// The nominal `qualified_declaration` a projection node carries. Complete V1
/// at this pin names no other node, so only enum, dimension and unit nodes
/// declare an exportable name.
fn declaration(node: &Map<String, Value>) -> Result<Option<String>, NodeDefect> {
    node.get("node_tag")
        .and_then(Value::as_str)
        .filter(|tag| NODE_TAGS.contains(tag))
        .ok_or(NodeDefect::NodeTag)?;
    Ok(node
        .get("nominal_identity_preimage")
        .map(nominal_declaration)
        .transpose()?
        .flatten())
}

fn projected_node(value: &Value) -> Result<(NodeKey, Option<String>), NodeDefect> {
    let node = members(value, &NODE_REQUIRED, &NODE_OPTIONAL).map_err(|defect| match defect {
        MemberDefect::NotObject => NodeDefect::NotObject,
        MemberDefect::Missing(name) => NodeDefect::MissingMember(name),
        MemberDefect::Unknown(name) => NodeDefect::UnknownMember(name),
    })?;
    let key = node
        .get("node_id")
        .and_then(node_key)
        .ok_or(NodeDefect::NodeId)?;
    if node.get("schema_version").and_then(Value::as_str) != Some(NODE_SCHEMA_VERSION) {
        return Err(NodeDefect::SchemaVersion);
    }
    Ok((key, declaration(node)?))
}

/// Validate `bytes` as an identity preimage and derive the node key of each
/// nominal declaration in its `identity_projection`.
///
/// Canonicity is enforced by requiring `bytes` to be byte-identical to the
/// re-serialization of the JSON value they parse to: object members in
/// ascending UTF-8 byte order, no insignificant whitespace and one string and
/// number spelling per value. That is RFC 8785 JCS for every preimage whose
/// member names are ASCII, which is every name this schema defines and every
/// name QSpec 7d7943a gives a projection node. It is stricter than JCS, never
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
    let mut declarations = BTreeMap::new();
    let mut previous: Option<NodeKey> = None;
    for (index, node) in nodes.iter().enumerate() {
        let (key, declared) =
            projected_node(node).map_err(|defect| PreimageDefect::Node { index, defect })?;
        if previous.is_some_and(|previous| previous >= key) {
            return Err(PreimageDefect::NodeOrder { index });
        }
        previous = Some(key);
        if let Some(declared) = declared {
            if declarations.insert(declared, key).is_some() {
                return Err(PreimageDefect::DuplicateDeclaration { index });
            }
        }
    }
    Ok(ProjectedDeclarations(declarations))
}
