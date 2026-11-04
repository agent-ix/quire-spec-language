// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the slot selection and
//! the binder frame of a protocol slot, read from the package's own nodes
//! and rows alone.
//!
//! The source selector (`protocol_selection::select`) reads the checked
//! protocol of a `CheckedPackage`; this reads the `protocol` node bodies
//! (FR-383's layout), the `protocol_locations` rows and the binder nodes of
//! the root's dependency closure (FR-390 "Selection and supply admission",
//! rule 3). Nothing here names a `CheckedPackage`, a `CheckedBody` or a
//! source graph, and nothing takes a `Meter`.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedNodeId, CheckedPackageV2, CheckedProtocolLocationOrigin, CheckedSemanticNodeV2,
    CheckedTerm,
};
use quire_exact::{NodeKey, ValueType};
use quire_semantic_value::catalog::InternalFault;

use crate::bare::{BarePackage, IdentityWitnessRefusal};
use crate::checked_v2::wire_graph;
use crate::protocol_selection::{ProtocolSlotDescriptor, SlotSelectionFailure};
use crate::protocol_view::{
    argument, control_operation, group_member, integer, reference, text, ControlOperation,
};
use crate::reader_protocol_projection::{node_value_type, TypeError, TypeReader, UnsupportedView};

type Node = CheckedSemanticNodeV2;

/// Why a bare selection produced no slot.
#[derive(Debug)]
pub enum BareSelectionFailure {
    /// The identity rows of the supply are absent, foreign or name nothing
    /// the package admits (FR-531-AC-5); decided before any node is read.
    Identity(IdentityWitnessRefusal),
    /// The descriptor names no slot (FR-390 "Selection and supply admission",
    /// rules 1 and 2).
    Slot(SlotSelectionFailure),
    /// A binder type has no kernel representation in the admitted closure.
    Unsupported(Box<UnsupportedView>),
}

impl From<SlotSelectionFailure> for BareSelectionFailure {
    fn from(failure: SlotSelectionFailure) -> Self {
        Self::Slot(failure)
    }
}

/// One binder a slot reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BareSource {
    index: usize,
    name: String,
    level: usize,
    value_type: ValueType,
    binder: NodeKey,
}

impl BareSource {
    /// The source's position in the slot's frame.
    pub fn index(&self) -> usize {
        self.index
    }
    /// The declared name the binder node carries.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The binder's level.
    pub fn level(&self) -> usize {
        self.level
    }
    /// The binder's kernel value type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }
    /// The binder's structural key.
    pub fn binder(&self) -> &NodeKey {
        &self.binder
    }
}

/// A protocol slot selected from the package's nodes.
#[derive(Clone, Debug)]
pub struct BareSlot {
    root: NodeKey,
    origins: Vec<String>,
    sources: Vec<BareSource>,
}

impl BareSlot {
    /// The structural key of the slot expression's root node.
    pub fn root(&self) -> &NodeKey {
        &self.root
    }
    /// The site names the slot may be located under, which key its
    /// `protocol_locations` rows: one for a name the node determines; none
    /// for a span-named slot whose root holds no row (a leaf, so the walk has
    /// no term to pair); several for a span-named slot whose root shares its
    /// rows with another slot of the control, which the bytes cannot tell
    /// apart (QSL-1641 plan Q1: open for the spec lane).
    pub fn origins(&self) -> &[String] {
        &self.origins
    }
    /// The binders the expression reads, ascending by (level, key); each
    /// source's `index` is its position in that order.
    pub fn sources(&self) -> &[BareSource] {
        &self.sources
    }
    /// The frame length: the binders read.
    pub fn frame_len(&self) -> usize {
        self.sources.len()
    }
}

fn absent(descriptor: &ProtocolSlotDescriptor) -> SlotSelectionFailure {
    SlotSelectionFailure::MissingMember {
        name: descriptor.member().to_owned(),
        index: descriptor.index(),
    }
}

fn fault(invariant: &'static str) -> SlotSelectionFailure {
    SlotSelectionFailure::Fault(InternalFault::new("FR-390", invariant))
}

/// How the member is named in the site-name registry (FR-383).
enum Site {
    /// The whole name is known from the node.
    Exact(String),
    /// Only a prefix is known; the name ends in a source span the bytes do
    /// not carry, so the root's rows decide.
    Prefix(String),
}

fn names(node: &Node) -> Vec<&str> {
    node.declaration
        .as_ref()
        .map(|declaration| declaration.qualified_name.iter().map(|s| &**s).collect())
        .unwrap_or_default()
}

fn clause_name(node: &Node) -> String {
    // The clause is `P`; every other protocol node leads with `P` too.
    names(node)
        .first()
        .map(|s| (*s).to_owned())
        .unwrap_or_default()
}

/// The term a slot's member names and the site it is located under.
fn slot_term<'n>(
    node: &'n Node,
    op: ControlOperation,
    descriptor: &ProtocolSlotDescriptor,
    package: &'n CheckedPackageV2,
) -> Result<(&'n CheckedTerm, Site), SlotSelectionFailure> {
    use ControlOperation as Op;
    let (member, index) = (descriptor.member(), descriptor.index());
    let no = || absent(descriptor);
    let indexed = match (op, member) {
        (Op::Protocol, "captures")
        | (Op::Compensate, "registration_captures" | "activation_captures")
        | (Op::Choice, "cases.guard" | "visible")
        | (Op::Repeat, "visible")
        | (
            Op::Send | Op::Receive | Op::Attempt | Op::Effect | Op::Event,
            "related.from" | "related.to",
        ) => true,
        _ => false,
    };
    if indexed != index.is_some() {
        return Err(no());
    }
    let at = |term: &'n CheckedTerm| -> Result<&'n [CheckedTerm], SlotSelectionFailure> {
        match term {
            CheckedTerm::Aggregate { members } => Ok(members),
            _ => Err(fault("selected-protocol-member-is-a-list")),
        }
    };
    let p = clause_name(node);
    let path = || {
        names(node)
            .iter()
            .skip(2)
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let run = |what: &str| format!("protocol {p} run {} {what}", path());
    let arg = |name: &str| argument(node, name).ok_or_else(no);
    let member_of = |term: &'n CheckedTerm, name: &str| group_member(term, name).ok_or_else(no);
    // The Nth record of a tuple argument, as its Group.
    let record = |name: &str| -> Result<&'n CheckedTerm, SlotSelectionFailure> {
        at(arg(name)?)?.get(index.ok_or_else(no)?).ok_or_else(no)
    };
    let capture_name = |record: &'n CheckedTerm| -> Result<String, SlotSelectionFailure> {
        let binder = reference(member_of(record, "binder")?).ok_or_else(no)?;
        let node = package
            .graph()
            .nodes
            .iter()
            .find(|candidate| &candidate.node_id == binder)
            .ok_or_else(no)?;
        group_member(&node.body, "name")
            .and_then(text)
            .map(str::to_owned)
            .ok_or_else(no)
    };
    Ok(match (op, member) {
        (Op::Protocol, "activation.guard") => (
            member_of(arg("activation")?, "guard")?,
            Site::Exact(format!("protocol {p} activation guard")),
        ),
        (Op::Protocol, "captures") => {
            let record = record("captures")?;
            let name = capture_name(record)?;
            (
                member_of(record, "initializer")?,
                Site::Exact(format!("protocol {p} capture {name} initializer")),
            )
        }
        (Op::Role, "replication.until") => (
            member_of(arg("replication")?, "until")?,
            Site::Exact(format!(
                "protocol {p} role {} until",
                names(node).get(2).ok_or_else(no)?
            )),
        ),
        (Op::Channel, "ordering.key") => (
            member_of(arg("ordering")?, "key")?,
            Site::Exact(format!(
                "protocol {p} channel {} key",
                names(node).get(2).ok_or_else(no)?
            )),
        ),
        (Op::Compensate, member) => {
            let name = names(node).get(2).copied().ok_or_else(no)?.to_owned();
            let stage = |stage: &str, position: usize| {
                Site::Exact(format!("protocol {p} compensate {name} {stage} {position}"))
            };
            match member {
                "registration_captures" => (
                    member_of(record("registration_captures")?, "initializer")?,
                    stage("registration-capture", index.ok_or_else(no)?),
                ),
                "activation_captures" => (
                    member_of(record("activation_captures")?, "initializer")?,
                    stage("activation-capture", index.ok_or_else(no)?),
                ),
                "trigger.guard" => (
                    member_of(arg("trigger")?, "guard")?,
                    stage("trigger-guard", 0),
                ),
                "retry.relation" => (member_of(arg("retry")?, "relation")?, stage("retry", 0)),
                "recovery.predicate" => (
                    member_of(arg("recovery")?, "predicate")?,
                    stage("recovery", 0),
                ),
                _ => return Err(no()),
            }
        }
        (Op::Choice, "cases.guard") => (
            member_of(record("cases")?, "guard")?,
            Site::Prefix(run("decision ")),
        ),
        (Op::Choice | Op::Repeat, "visible") => (
            at(arg("visible")?)?
                .get(index.ok_or_else(no)?)
                .ok_or_else(no)?,
            Site::Prefix(run("")),
        ),
        (Op::Repeat, "guard") => (arg("guard")?, Site::Exact(run("body"))),
        (Op::Check, "condition") => (arg("condition")?, Site::Exact(run("body"))),
        (
            Op::Send | Op::Receive | Op::Attempt | Op::Effect | Op::Event | Op::Commit,
            "constraint",
        ) => (arg("constraint")?, Site::Exact(run("body"))),
        (
            Op::Send | Op::Receive | Op::Attempt | Op::Effect | Op::Event,
            "related.from" | "related.to",
        ) => (
            member_of(record("related")?, &member[8..])?,
            Site::Prefix(run("related ")),
        ),
        (Op::Finish, "constraint") => (
            arg("constraint")?,
            Site::Exact(format!(
                "protocol {p} finish {} body",
                names(node).get(2).ok_or_else(no)?
            )),
        ),
        (Op::Terminal, "condition") => {
            let [condition] = at(arg("condition")?)? else {
                return Err(no());
            };
            (
                condition,
                Site::Exact(format!("protocol {p} terminal body")),
            )
        }
        _ => return Err(no()),
    })
}

impl<'p> BarePackage<'p> {
    /// Select the slot `descriptor` names from the package's nodes, with the
    /// binders its expression reads (FR-533). Charges nothing and takes no
    /// meter.
    ///
    /// # Errors
    ///
    /// [`BareSelectionFailure`].
    pub fn select_protocol_slot(
        &self,
        descriptor: &ProtocolSlotDescriptor,
    ) -> Result<BareSlot, BareSelectionFailure> {
        let package = self.package();
        let rows = self
            .supply()
            .ok_or(BareSelectionFailure::Identity(
                IdentityWitnessRefusal::Missing,
            ))?
            .identities;
        let identities = rows
            .selected(package)
            .map_err(BareSelectionFailure::Identity)?;
        let graph = wire_graph(package).map_err(|_| fault("bare-package-has-structural-keys"))?;
        let nodes: BTreeMap<&CheckedNodeId, &Node> = package
            .graph()
            .nodes
            .iter()
            .map(|node| (&node.node_id, node))
            .collect();
        let mut by_key: BTreeMap<NodeKey, &CheckedNodeId> = BTreeMap::new();
        for id in nodes.keys() {
            by_key.insert(
                graph
                    .key(id)
                    .map_err(|_| fault("bare-package-has-structural-keys"))?,
                id,
            );
        }
        let node = by_key
            .get(descriptor.node())
            .and_then(|id| nodes.get(id))
            .copied()
            .ok_or_else(|| SlotSelectionFailure::MissingNode(descriptor.node().clone()))?;
        if &*node.node_tag != "protocol" {
            return Err(SlotSelectionFailure::MalformedNode(descriptor.node().clone()).into());
        }
        // A protocol node outside the member table (a queue, a reference
        // carrier, a sequence, a role without `until`) has no expression.
        let op = control_operation(node).map_err(|_| absent(descriptor))?;
        let (term, site) = slot_term(node, op, descriptor, package)?;
        let root = reference(term)
            .ok_or_else(|| fault("selected-protocol-expression-has-a-keyed-root"))?;
        let origins = origin_of(package, root, site)?;

        // The binders: `value`/`parameter` nodes of the root's dependency
        // closure, stopping at a binder and at a function (FR-390 rule 3).
        let mut found: BTreeSet<&CheckedNodeId> = BTreeSet::new();
        let mut seen: BTreeSet<&CheckedNodeId> = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(next) = nodes.get(id) else { continue };
            match (&*next.node_tag, &*next.semantic_form) {
                ("value", "parameter") => {
                    found.insert(id);
                }
                ("expression", _) | ("value", _) => pending.extend(&next.dependencies),
                _ => {}
            }
        }
        let reader = TypeReader {
            admitted: package,
            identities: &identities,
            nodes: &nodes,
            graph: &graph,
            work_limit: package_work_limit(),
        };
        let mut sources = Vec::with_capacity(found.len());
        for id in found {
            let binder = nodes
                .get(id)
                .copied()
                .ok_or_else(|| fault("bare-binder-is-held"))?;
            let key = graph
                .key(id)
                .map_err(|_| fault("bare-package-has-structural-keys"))?;
            let name = group_member(&binder.body, "name").and_then(text);
            let level = group_member(&binder.body, "level").and_then(integer);
            let (Some(name), Some(level)) = (name, level) else {
                return Err(fault("bare-binder-has-a-name-and-a-level").into());
            };
            let value_type = match node_value_type(&reader, id) {
                Ok(value_type) => value_type,
                Err(TypeError::Unsupported) => {
                    return Err(BareSelectionFailure::Unsupported(Box::new(
                        UnsupportedView::missing_type(package.name(), key),
                    )))
                }
                Err(TypeError::Fault(failure)) => {
                    return Err(SlotSelectionFailure::Fault(failure).into())
                }
            };
            sources.push(BareSource {
                index: 0,
                name: name.to_owned(),
                level: usize::try_from(level).map_err(|_| fault("bare-binder-level-fits"))?,
                value_type,
                binder: key,
            });
        }
        sources
            .sort_by(|left, right| (left.level, &left.binder).cmp(&(right.level, &right.binder)));
        for (index, source) in sources.iter_mut().enumerate() {
            source.index = index;
        }
        let root = graph
            .key(root)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        Ok(BareSlot {
            root,
            origins,
            sources,
        })
    }
}

/// The projection work bound: the I2 reader's default.
fn package_work_limit() -> u64 {
    crate::checked_v2::V2ReadLimits::default().work
}

/// The site names of the slot rooted at `root` (FR-383 site names).
fn origin_of(
    package: &CheckedPackageV2,
    root: &CheckedNodeId,
    site: Site,
) -> Result<Vec<String>, SlotSelectionFailure> {
    Ok(match site {
        Site::Exact(name) => vec![name],
        Site::Prefix(prefix) => package
            .checked_term_data()
            .map_or(&[][..], |data| data.protocol_locations.as_slice())
            .iter()
            .filter(|row| &row.application == root)
            .filter_map(|row| match &row.location.origin {
                CheckedProtocolLocationOrigin::TypeDeclaration { name } => Some(name.to_string()),
                _ => None,
            })
            .filter(|name| name.starts_with(&prefix) && !name.ends_with(" body"))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    })
}
