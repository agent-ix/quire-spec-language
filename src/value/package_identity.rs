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
    /// The bytes are not the RFC 8785 JCS serialization of their JSON value.
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
    /// An exported name is declared by no projection node.
    UndeclaredExport(String),
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
    /// A nominal `qualified_declaration` is not a non-empty identifier array.
    Declaration,
}

/// The validated export node keys of one identity preimage, by qualified
/// declaration spelled with `::`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProjectedDeclarations(BTreeMap<String, NodeKey>);

impl ProjectedDeclarations {
    /// The node key declaring `name`.
    pub(crate) fn node(&self, name: &str) -> Option<NodeKey> {
        self.0.get(name).copied()
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

fn declaration(node: &Map<String, Value>) -> Result<Option<String>, NodeDefect> {
    let Some(nominal) = node.get("nominal_identity_preimage") else {
        return Ok(None);
    };
    let Some(segments) = nominal
        .as_object()
        .ok_or(NodeDefect::Declaration)?
        .get("qualified_declaration")
    else {
        return Ok(None);
    };
    let segments: Vec<String> = segments
        .as_array()
        .ok_or(NodeDefect::Declaration)?
        .iter()
        .map(|segment| segment.as_str().map(str::to_owned))
        .collect::<Option<_>>()
        .ok_or(NodeDefect::Declaration)?;
    if !is_qualified_name(&segments) {
        return Err(NodeDefect::Declaration);
    }
    Ok(Some(segments.join("::")))
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
/// name in `exports` from its `identity_projection`.
pub(crate) fn project_exports(
    bytes: &[u8],
    exports: &[String],
) -> Result<ProjectedDeclarations, PreimageDefect> {
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
    let mut exported = BTreeMap::new();
    for name in exports {
        let key = declarations
            .get(name)
            .copied()
            .ok_or_else(|| PreimageDefect::UndeclaredExport(name.clone()))?;
        exported.insert(name.clone(), key);
    }
    Ok(ProjectedDeclarations(exported))
}
