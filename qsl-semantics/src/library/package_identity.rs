// SPDX-License-Identifier: AGPL-3.0-or-later
//! Structural reading of a CheckedPackage V2 `quire.checked-package-id/v2`
//! identity preimage: its version, required members and the
//! `identity_projection` nodes from which FR-307 export node keys are derived.
//!
//! FR-087: relocated from `value::package_identity`. Node ids read from the
//! preimage are structural `quire_exact::NodeKey`s.

use std::collections::{BTreeMap, BTreeSet};

use quire_canonical::{Node, NodeRef};
use quire_contract_model::CheckedNodeTag;

use crate::value::semantic_node::is_qualified_name;
use quire_exact::{NodeKey, PathSegment};

/// The identity preimage version constant.
pub const PACKAGE_ID_VERSION: &str = "quire.checked-package-id/v2";
/// The node projection schema version constant.
pub(crate) const NODE_SCHEMA_VERSION: &str = "quire.checked-semantic-graph/v2";

const PREIMAGE_REQUIRED: [&str; 7] = [
    "version",
    "edition",
    "profile_selections",
    "definition_selections",
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

/// `owner` is the node's structural owner (QSpec FR-322 "Owner member
/// schema"), whose closed shape IR's reader validates before this one runs.
///
/// `declaration` is the projection node's own name member (checked-package-v2
/// README.md's "Declarations" section): a named, source-declared node carries
/// `declaration: {qualified_name}` exactly when it has a declaration source
/// occurrence. On a nominal node it must equal the nominal
/// `qualified_declaration`, and every export resolves only through this
/// member, never through `nominal_identity_preimage` directly.
///
/// `owner_model` is a protocol clause's exact local model declaration edge
/// (QSpec FR-322, FR-464 "owner_model"): the identity projection carries it
/// as the node does, a node id, and IR's reader has already checked that it
/// is present exactly on a clause.
const NODE_OPTIONAL: [&str; 5] = [
    "recursion_group",
    "nominal_identity_preimage",
    "declaration",
    "owner",
    "owner_model",
];

/// Why an identity preimage is structurally malformed.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PreimageDefect {
    /// The bytes are not JSON: `quire-canonical`'s reader refused them at
    /// byte `offset`.
    Malformed {
        /// The byte offset where the fault starts.
        offset: usize,
    },
    /// Reading or encoding the bytes could not reserve memory (FR-259 B6).
    /// It names no bound: the read sets no byte limit of its own, since the
    /// bytes are already held whole under the package reader's artifact byte
    /// limit.
    AllocationFailed {
        /// The size in bytes of the reservation that failed.
        requested: usize,
    },
    /// The shared reader refused the bytes for a reason that is neither a
    /// located malformed fault nor an allocation failure. It claims no byte
    /// offset; `reason` is the reader's own account.
    ReaderRefused {
        /// The reader's refusal, as it states it.
        reason: String,
    },
    /// The bytes are JSON but not one JSON object.
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
    /// Projection node `index` repeats the node id of an earlier node. FR-322:
    /// node identifiers are unique within the graph.
    DuplicateNode {
        /// The repeating node's index in `identity_projection`.
        index: usize,
        /// The repeated node id, as its `Display` spelling.
        node: String,
    },
    /// Two projection nodes carry equal `declaration.qualified_name` values
    /// (`ambiguous-name`). `nodes` holds both node ids, as their `Display`
    /// spellings, in ascending structural order.
    AmbiguousDeclaration {
        /// The repeated qualified name.
        name: String,
        /// Both node ids, in ascending structural order.
        nodes: [String; 2],
    },
    /// A node's top-level `declaration.qualified_name` disagrees with its
    /// nominal `qualified_declaration`, or is absent while a nominal
    /// `qualified_declaration` is present (`declaration-nominal-mismatch`).
    DeclarationNominalMismatch {
        /// The node's id, as its `Display` spelling.
        node: String,
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
    /// `node_id` is not a structural node id `{package, path, ordinal}`.
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

/// The validated declarations of one identity preimage: each projection
/// node that carries a top-level `declaration`, keyed by its wire node id,
/// with the `declaration.qualified_name` it spells (joined with `::`) as
/// data. It offers no lookup that takes a name: resolving a name to a node
/// id is the importing package's own check stage (FR-087-AC-4; ADR-013
/// R-06).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProjectedDeclarations(BTreeMap<NodeKey, String>);

impl ProjectedDeclarations {
    /// Every declared name, in ascending order: FR-307's "a package's local
    /// declarations are exactly its exports" (`library` module doc), read by
    /// the layer-4 `package` I2 reader (ADR-011 §4) before it populates a
    /// freshly wire-read [`crate::library::LibraryPackage::exports`].
    #[allow(
        dead_code,
        reason = "no production caller yet: reachable only through `library::declared_exports`, itself unused pending ADR-011 §4's round trip (QSL-347) to wire the I2 reader in"
    )]
    pub(crate) fn declared_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.0.values().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// The first of `exports`, in order, that no declaration spells. A
    /// membership test over the declared names: it yields a name, never a
    /// node id.
    pub(crate) fn undeclared<'a>(&self, exports: &'a [String]) -> Option<&'a str> {
        let declared: BTreeSet<&str> = self.0.values().map(String::as_str).collect();
        exports
            .iter()
            .map(String::as_str)
            .find(|export| !declared.contains(export))
    }

    /// Keep only the declarations whose name is one of `exports`. A
    /// membership filter over the entries: it takes names and yields no
    /// node id.
    pub(crate) fn retain_exported(&mut self, exports: &[String]) {
        let exported: BTreeSet<&str> = exports.iter().map(String::as_str).collect();
        self.0.retain(|_, name| exported.contains(name.as_str()));
    }

    /// Every declaration as a `(NodeKey, name)` entry, in ascending
    /// node-id order.
    #[cfg(test)]
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&NodeKey, &str)> + '_ {
        self.0.iter().map(|(node, name)| (node, name.as_str()))
    }

    /// Consume `self` into its node-id-keyed entries, for
    /// [`super::ImportView`]'s own exported-declaration data (FR-087-AC-4).
    pub(crate) fn into_entries(self) -> BTreeMap<NodeKey, String> {
        self.0
    }
}

/// `value`'s members as `(name, value)` pairs, when it is an object.
fn object_members<'d>(
    value: NodeRef<'d>,
) -> Option<impl ExactSizeIterator<Item = (&'d str, NodeRef<'d>)>> {
    match value.node() {
        Node::Object(members) => Some(members),
        _ => None,
    }
}

/// `value`'s elements, when it is an array.
fn array_items<'d>(value: NodeRef<'d>) -> Option<impl ExactSizeIterator<Item = NodeRef<'d>>> {
    match value.node() {
        Node::Array(items) => Some(items),
        _ => None,
    }
}

/// `value` as a string, when it is one.
fn string_of(value: NodeRef<'_>) -> Option<&str> {
    match value.node() {
        Node::String(text) => Some(text),
        _ => None,
    }
}

fn is_object(value: NodeRef<'_>) -> bool {
    matches!(value.node(), Node::Object(_))
}

fn is_array(value: NodeRef<'_>) -> bool {
    matches!(value.node(), Node::Array(_))
}

fn members<'d>(
    value: NodeRef<'d>,
    required: &[&'static str],
    optional: &[&'static str],
) -> Result<NodeRef<'d>, MemberDefect> {
    let names = object_members(value).ok_or(MemberDefect::NotObject)?;
    let present: Vec<&str> = names.map(|(name, _)| name).collect();
    if let Some(missing) = required.iter().find(|name| !present.contains(name)) {
        return Err(MemberDefect::Missing(missing));
    }
    if let Some(unknown) = present
        .iter()
        .find(|key| !required.contains(key) && !optional.contains(key))
    {
        return Err(MemberDefect::Unknown((*unknown).to_owned()));
    }
    Ok(value)
}

enum MemberDefect {
    NotObject,
    Missing(&'static str),
    Unknown(String),
}

/// One node id being read: its package, the path read so far, the path
/// elements still to read and its ordinal.
struct IdFrame<'d> {
    package: &'d str,
    path: Vec<PathSegment>,
    remaining: Box<dyn Iterator<Item = NodeRef<'d>> + 'd>,
    ordinal: u64,
}

fn open_id(value: NodeRef<'_>) -> Option<IdFrame<'_>> {
    if object_members(value)?.len() != 3 {
        return None;
    }
    let package = string_of(value.get("package")?)?;
    let Node::Number(number) = value.get("ordinal")?.node() else {
        return None;
    };
    let ordinal = number.text().parse().ok()?;
    let items = array_items(value.get("path")?)?;
    let mut path = Vec::new();
    path.try_reserve_exact(items.len()).ok()?;
    Some(IdFrame {
        package,
        path,
        remaining: Box::new(items),
        ordinal,
    })
}

/// The structural node id `{package, path, ordinal}` a value spells, read
/// with an explicit stack (a path element is a name or a nested node id).
fn wire_node_id(value: NodeRef<'_>) -> Option<NodeKey> {
    let mut stack = vec![open_id(value)?];
    loop {
        let frame = stack.last_mut()?;
        if let Some(element) = frame.remaining.next() {
            match string_of(element) {
                Some(name) => frame.path.push(PathSegment::Text(name.to_owned())),
                None => stack.push(open_id(element)?),
            }
            continue;
        }
        let done = stack.pop()?;
        let key = NodeKey::new(done.package, done.path, done.ordinal).ok()?;
        match stack.last_mut() {
            Some(parent) => parent.path.push(PathSegment::Node(key)),
            None => return Some(key),
        }
    }
}

fn qualified(segments: Vec<String>) -> Result<String, NodeDefect> {
    if is_qualified_name(&segments) {
        Ok(segments.join("::"))
    } else {
        Err(NodeDefect::Declaration)
    }
}

/// The `qualified_declaration` of a nominal identity preimage, when the
/// preimage form carries one (an enum member does not).
fn nominal_declaration(nominal: NodeRef<'_>) -> Result<Option<String>, NodeDefect> {
    if !is_object(nominal) {
        return Err(NodeDefect::Declaration);
    }
    let Some(segments) = nominal.get("qualified_declaration") else {
        return Ok(None);
    };
    let segments = array_items(segments)
        .ok_or(NodeDefect::Declaration)?
        .map(|segment| string_of(segment).map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or(NodeDefect::Declaration)?;
    qualified(segments).map(Some)
}

/// The `qualified_name` of a top-level `declaration` member, which must be
/// exactly `{qualified_name}`.
fn declared_name(value: NodeRef<'_>) -> Result<String, NodeDefect> {
    let object = object_members(value).ok_or(NodeDefect::Declaration)?;
    if object.len() != 1 {
        return Err(NodeDefect::Declaration);
    }
    let segments = value
        .get("qualified_name")
        .and_then(array_items)
        .ok_or(NodeDefect::Declaration)?
        .map(|segment| string_of(segment).map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or(NodeDefect::Declaration)?;
    qualified(segments)
}

/// A projection node's wire id and its shape-valid `declaration
/// .qualified_name` and nominal `qualified_declaration`, before the
/// cross-node checks that order them: whether `declared` and `nominal`
/// agree, and whether `declared` repeats an earlier node's.
struct NodeShape {
    key: NodeKey,
    declared: Option<String>,
    nominal: Option<String>,
}

/// The shape-valid `declaration.qualified_name` and nominal
/// `qualified_declaration` of one node's `declaration` and
/// `nominal_identity_preimage` members, without checking whether they agree.
fn declaration_shape(node: NodeRef<'_>) -> Result<(Option<String>, Option<String>), NodeDefect> {
    let nominal = node
        .get("nominal_identity_preimage")
        .map(nominal_declaration)
        .transpose()?
        .flatten();
    let declared = node.get("declaration").map(declared_name).transpose()?;
    Ok((declared, nominal))
}

fn projected_node(value: NodeRef<'_>) -> Result<NodeShape, NodeDefect> {
    let node = members(value, &NODE_REQUIRED, &NODE_OPTIONAL).map_err(|defect| match defect {
        MemberDefect::NotObject => NodeDefect::NotObject,
        MemberDefect::Missing(name) => NodeDefect::MissingMember(name),
        MemberDefect::Unknown(name) => NodeDefect::UnknownMember(name),
    })?;
    let key = node
        .get("node_id")
        .and_then(wire_node_id)
        .ok_or(NodeDefect::NodeId)?;
    if node.get("schema_version").and_then(string_of) != Some(NODE_SCHEMA_VERSION) {
        return Err(NodeDefect::SchemaVersion);
    }
    if node
        .get("owner_model")
        .is_some_and(|owner_model| wire_node_id(owner_model).is_none())
    {
        return Err(NodeDefect::NodeId);
    }
    node.get("node_tag")
        .and_then(string_of)
        .and_then(CheckedNodeTag::from_wire)
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
                node: shape.key.to_string(),
                declared: Some(declared.clone()),
                nominal: nominal.clone(),
            })
        }
        (None, Some(nominal)) => Some(PreimageDefect::DeclarationNominalMismatch {
            node: shape.key.to_string(),
            declared: None,
            nominal: nominal.clone(),
        }),
        _ => None,
    }
}

/// [`project_declarations`]'s defect for a refusal of the shared reader.
fn read_defect(error: quire_canonical::ReadError) -> PreimageDefect {
    match error {
        quire_canonical::ReadError::Malformed { offset, .. } => {
            PreimageDefect::Malformed { offset }
        }
        quire_canonical::ReadError::Allocation { requested } => {
            PreimageDefect::AllocationFailed { requested }
        }
        // The read sets no byte limit, so a limit cannot be reached; any
        // other refusal is reported as the reader states it, with no
        // offset it did not measure.
        other => PreimageDefect::ReaderRefused {
            reason: other.to_string(),
        },
    }
}

/// [`project_declarations`]'s defect for an error of the canonicity check's
/// encoding: an allocation failure is its own defect, and any other error
/// means the bytes have no canonical encoding.
fn encode_defect(error: quire_canonical::Error) -> PreimageDefect {
    match error {
        quire_canonical::Error::Allocation { requested } => {
            PreimageDefect::AllocationFailed { requested }
        }
        _ => PreimageDefect::NonCanonical,
    }
}

/// Validate `bytes` as an identity preimage and derive the wire node id of
/// each nominal declaration in its `identity_projection`.
///
/// The bytes are read first through `quire-canonical`'s shared reader: bytes
/// it refuses are [`PreimageDefect::Malformed`] at its byte offset.
/// Canonicity is enforced by requiring `bytes` to be byte-identical to the
/// RFC 8785 encoding of the tree the reader read, produced by
/// `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC 8785
/// implementation): object members in UTF-16 code-unit order, no
/// insignificant whitespace and one string and number spelling per value. A
/// member name outside ASCII is admitted exactly when it is in that order.
/// A number is the double its text denotes, so an integer spelling no IEEE
/// 754 double equals is not its canonical spelling and is refused as
/// [`PreimageDefect::NonCanonical`].
pub(crate) fn project_declarations(bytes: &[u8]) -> Result<ProjectedDeclarations, PreimageDefect> {
    let document = quire_canonical::read(bytes, u64::MAX).map_err(read_defect)?;
    let preimage =
        members(document.root(), &PREIMAGE_REQUIRED, &[]).map_err(|defect| match defect {
            MemberDefect::NotObject => PreimageDefect::NotObject,
            MemberDefect::Missing(name) => PreimageDefect::MissingMember(name),
            MemberDefect::Unknown(name) => PreimageDefect::UnknownMember(name),
        })?;
    // Canonical bytes are exactly as long as `bytes`, which the reader's own
    // artifact byte limit already bounded: an encoding past that length is
    // not `bytes`' canonical form (FR-259 Behavior 3).
    let canonical_bound =
        quire_canonical::Limits::new(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
    match quire_canonical::to_vec(&document, canonical_bound) {
        Ok(canonical) if canonical == bytes => {}
        Ok(_) => return Err(PreimageDefect::NonCanonical),
        Err(error) => return Err(encode_defect(error)),
    }
    if preimage.get("version").and_then(string_of) != Some(PACKAGE_ID_VERSION) {
        return Err(PreimageDefect::Version);
    }
    if !preimage.get("edition").is_some_and(is_object) {
        return Err(PreimageDefect::MemberType("edition"));
    }
    if let Some(name) = PREIMAGE_REQUIRED[2..6]
        .iter()
        .find(|name| !preimage.get(name).is_some_and(is_array))
    {
        return Err(PreimageDefect::MemberType(name));
    }
    let nodes = preimage
        .get("identity_projection")
        .and_then(array_items)
        .filter(|nodes| nodes.len() > 0)
        .ok_or(PreimageDefect::EmptyProjection)?;

    // Pass 1 (native-diagnostics.md's refusal order, step 1): every node's
    // own shape and node-id uniqueness, in projection order. This pass runs
    // to completion across every node before pass 2 begins, so a schema
    // refusal anywhere in the projection outranks a mismatch or an ambiguity
    // found by scanning fewer nodes.
    //
    // The projection keeps the semantic graph's own node order, and that
    // order is significant to `package_id` (FR-322-AC-14): it is not required
    // to be ascending by node id.
    let mut shapes = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.enumerate() {
        let shape =
            projected_node(node).map_err(|defect| PreimageDefect::Node { index, defect })?;
        shapes.push((index, shape));
    }

    // Passes 2 and 3 visit nodes in ascending node-id digest order, the
    // refusal order native-diagnostics.md and FR-322-AC-21 define. That is a
    // visit order, independent of the projection's own order. Keys are
    // unique after pass 1, so the order is total.
    let mut seen_nodes = BTreeSet::new();
    for (index, shape) in &shapes {
        if !seen_nodes.insert(&shape.key) {
            return Err(PreimageDefect::DuplicateNode {
                index: *index,
                node: shape.key.to_string(),
            });
        }
    }
    drop(seen_nodes);
    let mut shapes: Vec<NodeShape> = shapes.into_iter().map(|(_, shape)| shape).collect();
    shapes.sort_unstable_by(|a, b| a.key.cmp(&b.key));

    // Pass 2 (step 4): declaration-nominal-mismatch, in ascending node-id
    // order. This pass runs to completion across every node before pass 3
    // begins, so a mismatch anywhere outranks an ambiguity at an earlier
    // node.
    for shape in &shapes {
        if let Some(defect) = declaration_mismatch(shape) {
            return Err(defect);
        }
    }

    // Pass 3 (step 5): ambiguous-name, in ascending node-id order. This is
    // the preimage's own well-formedness (one node per declared name), not
    // a lookup: `seen` lives only for this pass.
    let mut seen: BTreeMap<&str, &NodeKey> = BTreeMap::new();
    for shape in &shapes {
        if let Some(declared) = &shape.declared {
            if let Some(earlier) = seen.insert(declared, &shape.key) {
                return Err(PreimageDefect::AmbiguousDeclaration {
                    name: declared.clone(),
                    nodes: [earlier.to_string(), shape.key.to_string()],
                });
            }
        }
    }
    let declarations = shapes
        .into_iter()
        .filter_map(|shape| shape.declared.map(|name| (shape.key, name)))
        .collect();
    Ok(ProjectedDeclarations(declarations))
}

/// Identity-preimage builders shared by this module's tests and
/// `library`'s verified-binding tests.
#[cfg(test)]
pub(super) mod fixtures {
    use quire_exact::{NodeKey, PathSegment};
    use serde_json::json;

    /// The structural node id spelled `{package, path, ordinal}` for `seed`.
    pub(crate) fn node_json(seed: &[u8]) -> serde_json::Value {
        json!({
            "package": "lib",
            "path": [String::from_utf8_lossy(seed)],
            "ordinal": 0,
        })
    }

    /// The `NodeKey` `node_json(seed)` spells.
    pub(crate) fn node_key(seed: &[u8]) -> NodeKey {
        NodeKey::new(
            "lib",
            vec![PathSegment::Text(
                String::from_utf8_lossy(seed).into_owned(),
            )],
            0,
        )
        .unwrap()
    }

    /// A single-node identity preimage: one nominal declaration node,
    /// identified by `node_seed`, declaring `qualified_name`.
    pub(crate) fn one_node_preimage(node_seed: &[u8], qualified_name: &[&str]) -> Vec<u8> {
        let reference = node_json(node_seed);
        let node = json!({
            "body": {"members": [], "term": "aggregate"},
            "dependencies": [],
            "node_id": reference,
            "node_tag": "scalar_type",
            "schema_version": "quire.checked-semantic-graph/v2",
            "semantic_form": "enum",
            "semantic_type": reference,
            "nominal_identity_preimage": {
                "members": ["READY"],
                "ordered": true,
                "owner": {"authority": "agent-ix", "identity": "library", "kind": "definition"},
                "qualified_declaration": qualified_name,
                "version": "quire.enum-declaration-node/v1",
            },
            "declaration": {"qualified_name": qualified_name},
        });
        let preimage = json!({
            "definition_selections": [],
            "dependency_selections": [],
            "edition": {
                "definition": {"authority": "agent-ix", "identity": "quire-edition"},
                "role": "edition",
            },
            "identity_projection": [node],
            "profile_selections": [],
            "required_features": ["quire.value.complete/v1"],
            "version": "quire.checked-package-id/v2",
        });
        serde_json::to_vec(&preimage).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::Value;

    use super::fixtures::{node_key, one_node_preimage};
    use super::*;

    /// The IR-owned closed vocabulary admits `extension` without a second
    /// QSL copy of the node-tag list.
    #[trace("QSpec-FR-469-AC-1")]
    #[test]
    fn an_extension_tagged_node_passes_the_node_tag_check() {
        let mut preimage: Value =
            serde_json::from_slice(&one_node_preimage(b"L::R", &["L", "R"])).unwrap();
        preimage["identity_projection"][0]["node_tag"] = Value::from("extension");
        assert!(project_declarations(&serde_json::to_vec(&preimage).unwrap()).is_ok());
    }

    /// QSpec FR-464 "owner_model": the projection keeps the edge a node id and
    /// refuses any other shape of it.
    #[trace("FR-383-AC-26")]
    #[test]
    fn a_projected_owner_model_is_a_node_id() {
        let base = one_node_preimage(b"L::R", &["L", "R"]);
        let mut preimage: Value = serde_json::from_slice(&base).unwrap();
        preimage["identity_projection"][0]["owner_model"] =
            preimage["identity_projection"][0]["node_id"].clone();
        assert!(project_declarations(&serde_json::to_vec(&preimage).unwrap()).is_ok());
        for edge in [
            Value::Null,
            Value::from("x"),
            serde_json::json!({"node": "x"}),
        ] {
            preimage["identity_projection"][0]["owner_model"] = edge;
            assert!(matches!(
                project_declarations(&serde_json::to_vec(&preimage).unwrap()),
                Err(PreimageDefect::Node {
                    defect: NodeDefect::NodeId,
                    ..
                })
            ));
        }
    }

    /// Non-string tags and strings outside IR's closed V2 vocabulary are
    /// refused as `NodeTag`; QSL has no copied node-tag inventory.
    #[trace("QSpec-FR-322-AC-122")]
    #[test]
    fn tags_outside_the_ir_vocabulary_are_refused() {
        let base = one_node_preimage(b"L::R", &["L", "R"]);
        for tag in [Value::Bool(true), Value::from("not_a_checked_node_tag")] {
            let mut preimage: Value = serde_json::from_slice(&base).unwrap();
            preimage["identity_projection"][0]["node_tag"] = tag;
            assert!(matches!(
                project_declarations(&serde_json::to_vec(&preimage).unwrap()),
                Err(PreimageDefect::Node {
                    defect: NodeDefect::NodeTag,
                    ..
                })
            ));
        }
    }

    /// A `hybrid`-tagged projection node (QSpec FR-481, FR-322-AC-13) passes
    /// the node tag check.
    #[trace("QSpec-FR-322-AC-13", "FR-447-AC-13")]
    #[test]
    fn a_hybrid_tagged_node_passes_the_node_tag_check() {
        let mut preimage: Value =
            serde_json::from_slice(&one_node_preimage(b"L::R", &["L", "R"])).unwrap();
        preimage["identity_projection"][0]["node_tag"] = Value::from("hybrid");
        assert!(project_declarations(&serde_json::to_vec(&preimage).unwrap()).is_ok());
    }

    /// A `timed_model` tag added by IR is read through the same closed
    /// vocabulary as every other checked node tag.
    #[trace("FR-447-AC-13")]
    #[test]
    fn a_timed_model_tag_passes_the_node_tag_check() {
        let mut preimage: Value =
            serde_json::from_slice(&one_node_preimage(b"L::R", &["L", "R"])).unwrap();
        preimage["identity_projection"][0]["node_tag"] = Value::from("timed_model");
        assert!(project_declarations(&serde_json::to_vec(&preimage).unwrap()).is_ok());
    }

    /// The canonicity check is the `quire-canonical`
    /// encoder's own output, so it admits a member name outside ASCII
    /// exactly when the name is in RFC 8785's UTF-16 code-unit order. `😀`
    /// (UTF-16 `D83D DE00`) sorts before U+E000 (`E000`) in RFC 8785, but
    /// after it in UTF-8 byte order (`F0` > `EE`): the RFC 8785 order is
    /// admitted, and the UTF-8 order the old re-serialize-and-compare check
    /// required is refused as non-canonical. A trailing space and a
    /// non-canonical number spelling stay refused.
    #[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
    #[test]
    fn canonicity_admits_a_non_ascii_member_name_in_rfc_8785_order() {
        let mut preimage: Value =
            serde_json::from_slice(&one_node_preimage(b"L::R", &["L", "R"])).unwrap();
        preimage["edition"]["names"] = serde_json::json!({"\u{E000}": 1, "😀": 2});
        let document =
            quire_canonical::read(&serde_json::to_vec(&preimage).unwrap(), u64::MAX).unwrap();
        let rfc_8785 =
            quire_canonical::to_vec(&document, quire_canonical::Limits::new(u64::MAX)).unwrap();
        let text = std::str::from_utf8(&rfc_8785).unwrap();
        assert!(text.contains("{\"😀\":2,\"\u{E000}\":1}"), "{text}");
        assert!(project_declarations(&rfc_8785).is_ok());

        let utf8_order = serde_json::to_vec(&preimage).unwrap();
        assert_ne!(utf8_order, rfc_8785);
        assert_eq!(
            project_declarations(&utf8_order).unwrap_err(),
            PreimageDefect::NonCanonical
        );

        let mut spaced = rfc_8785.clone();
        spaced.push(b' ');
        assert_eq!(
            project_declarations(&spaced).unwrap_err(),
            PreimageDefect::NonCanonical
        );
        let float = text.replacen("\"\u{E000}\":1", "\"\u{E000}\":1.0", 1);
        assert_eq!(
            project_declarations(float.as_bytes()).unwrap_err(),
            PreimageDefect::NonCanonical
        );
    }

    /// FR-307: the wire node id `library::package_identity` derives for a
    /// nominal declaration is the projection node's own `node_id` digest,
    /// carried as a `(NodeKey, name)` entry.
    #[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
    #[test]
    fn project_declarations_derives_the_declaring_nodes_own_node_id() {
        let bytes = one_node_preimage(b"Example::Length", &["Example", "Length"]);
        let expected = node_key(b"Example::Length");
        let declarations = project_declarations(&bytes).unwrap();
        assert_eq!(
            declarations.entries().collect::<Vec<_>>(),
            vec![(&expected, "Example::Length")]
        );
    }

    /// A migrated identity's nodes are never relabelled onto the
    /// predecessor's evidence (FR-307-AC-3): two preimages that both
    /// declare the same qualified name, from different node content, carry
    /// that name at two different wire node ids.
    #[trace("QSpec-TC-227", "QSpec-FR-307-AC-3")]
    #[test]
    fn migrated_identity_derives_a_different_node_id_for_the_same_name() {
        let entries = |seed: &[u8]| {
            project_declarations(&one_node_preimage(seed, &["L", "R"]))
                .unwrap()
                .entries()
                .map(|(node, name)| (node.try_clone().unwrap(), name.to_owned()))
                .collect::<Vec<_>>()
        };
        let old_id = node_key(b"L::R (old)");
        let new_id = node_key(b"L::R (new)");
        assert_ne!(old_id, new_id);
        assert_eq!(
            entries(b"L::R (old)"),
            vec![(old_id.try_clone().unwrap(), "L::R".to_owned())]
        );
        assert_eq!(
            entries(b"L::R (new)"),
            vec![(new_id.try_clone().unwrap(), "L::R".to_owned())]
        );
    }

    /// `undeclared` answers membership only: the first export, in list
    /// order (not sorted order), that no declaration spells. It returns a
    /// name, never a node id.
    #[trace("QSpec-TC-227", "QSpec-FR-307-AC-1")]
    #[trace("TC-254", "FR-087-AC-4")]
    #[test]
    fn undeclared_names_the_first_export_no_declaration_spells() {
        let declarations = project_declarations(&one_node_preimage(b"L::R", &["L", "R"])).unwrap();
        let exports = |names: &[&str]| {
            names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(declarations.undeclared(&exports(&["L::R"])), None);
        assert_eq!(
            declarations.undeclared(&exports(&["L::R", "L::T", "L::S"])),
            Some("L::T")
        );
    }

    /// A reader refusal that is neither a located malformed fault nor an
    /// allocation failure is reported as the reader states it, claiming no
    /// byte offset.
    #[trace("TC-733", "FR-261-AC-1")]
    #[test]
    fn a_reader_refusal_with_no_offset_claims_none() {
        let limit = quire_canonical::read(b"[]", 1).unwrap_err();
        assert!(matches!(limit, quire_canonical::ReadError::Limit(_)));
        let reason = limit.to_string();
        assert_eq!(read_defect(limit), PreimageDefect::ReaderRefused { reason });
        assert_eq!(
            read_defect(quire_canonical::read(b"{", u64::MAX).unwrap_err()),
            PreimageDefect::Malformed { offset: 1 }
        );
    }

    /// The reader's and the canonicity encoding's allocation failures are
    /// one defect, which reports `resource_exhausted`/`allocation-failed`:
    /// not a malformed or non-canonical package.
    #[trace("TC-729", "FR-259-AC-5")]
    #[test]
    fn an_allocation_failure_is_its_own_defect() {
        let expected = PreimageDefect::AllocationFailed { requested: 4096 };
        assert_eq!(
            read_defect(quire_canonical::ReadError::Allocation { requested: 4096 }),
            expected
        );
        assert_eq!(
            encode_defect(quire_canonical::Error::Allocation { requested: 4096 }),
            expected
        );
        let refusal = crate::library::LibraryRefusal::InvalidPreimage {
            library: crate::library::LibraryName::new("acme").unwrap(),
            defect: expected,
        };
        assert_eq!(
            refusal.code(),
            qsl_foundation::diagnostic::Code::ResourceExhausted
        );
        assert_eq!(refusal.cause().as_str(), "allocation-failed");
    }

    /// FR-261-AC-1: on a 512 KiB stack, a canonical preimage whose members
    /// are valid except that `edition` holds an array nested 100,000 deep
    /// refuses `MemberType("edition")`, not `NotObject`; bytes that are not
    /// JSON refuse with the offset of the first malformed byte, and a
    /// top-level array is `NotObject`.
    #[trace("TC-733", "FR-261-AC-1")]
    #[test]
    fn a_preimage_with_a_deep_edition_refuses_by_member_type() {
        let outcome = std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let depth = 100_000;
                let valid = String::from_utf8(one_node_preimage(b"L::R", &["L", "R"])).unwrap();
                let edition = r#""edition":{"definition":{"authority":"agent-ix","identity":"quire-edition"},"role":"edition"}"#;
                assert!(valid.contains(edition));
                let deep = format!(
                    "\"edition\":{}{}",
                    "[".repeat(depth),
                    "]".repeat(depth)
                );
                let bytes = valid.replacen(edition, &deep, 1);
                (
                    project_declarations(bytes.as_bytes()).unwrap_err(),
                    project_declarations(b"not json").unwrap_err(),
                    project_declarations(b"[]").unwrap_err(),
                )
            })
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(outcome.0, PreimageDefect::MemberType("edition"));
        assert_eq!(outcome.1, PreimageDefect::Malformed { offset: 0 });
        assert_eq!(outcome.2, PreimageDefect::NotObject);
    }
}
