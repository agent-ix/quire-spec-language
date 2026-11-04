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

use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm};
use quire_exact::{NodeKey, ValueType};
use quire_semantic_value::catalog::InternalFault;

use crate::bare::{BarePackage, IdentityWitnessRefusal};
use crate::checked_v2::wire_graph;
use crate::equality_file::{EqualityWitnessRefusal, ReadEqualityRows};
use crate::protocol_selection::{ProtocolSlotDescriptor, SlotSelectionFailure};
use crate::protocol_view::{
    argument, control_operation, group_member, integer, reference, text, ControlOperation,
};
use crate::reader_protocol_projection::{
    node_value_type, BareWalker, TypeError, TypeReader, UnsupportedView, WalkFailure,
};

type Node = CheckedSemanticNodeV2;

/// Why a bare selection produced no slot.
#[derive(Debug)]
pub enum BareSelectionFailure {
    /// The identity rows of the supply are absent, foreign or name nothing
    /// the package admits (FR-531-AC-5); decided before any node is read.
    Identity(IdentityWitnessRefusal),
    /// The equality rows of the supply are absent, foreign or join no
    /// equality application of the slot's walk (FR-531 "Refusals").
    Equality(EqualityWitnessRefusal),
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
    origin: Option<String>,
    sources: Vec<BareSource>,
}

impl BareSlot {
    /// The structural key of the slot expression's root node.
    pub fn root(&self) -> &NodeKey {
        &self.root
    }
    /// The site name the slot's `protocol_locations` rows are keyed under:
    /// the name the node determines, the origin FR-533 "Slot naming on the
    /// bytes route" resolves for a span-named slot, or `None` for a slot with
    /// no application to locate.
    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
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
    /// not carry, so the rule of FR-533 "Slot naming on the bytes route"
    /// resolves it.
    Prefix(String, Span),
}

/// How a span-named origin spells its span after the prefix.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Span {
    /// `<start>..<end>` of a `decision` name (a visible entry or case guard).
    Decision,
    /// `<start> <end>` of a `related` name.
    Related,
}

impl Span {
    /// The numeric `(start, end)` of the remainder of an origin name.
    fn parse(self, rest: &str) -> Option<(u64, u64)> {
        let (start, end) = match self {
            Self::Decision => rest.split_once("..")?,
            Self::Related => rest.split_once(' ')?,
        };
        let number = |text: &str| {
            (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
                .then(|| text.parse().ok())
                .flatten()
        };
        Some((number(start)?, number(end)?))
    }
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
    let indexed = matches!(
        (op, member),
        (Op::Protocol, "captures")
            | (
                Op::Compensate,
                "registration_captures" | "activation_captures"
            )
            | (Op::Choice, "cases.guard" | "visible")
            | (Op::Repeat, "visible")
            | (
                Op::Send | Op::Receive | Op::Attempt | Op::Effect | Op::Event,
                "related.from" | "related.to",
            )
    );
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
            Site::Prefix(run("decision "), Span::Decision),
        ),
        (Op::Choice | Op::Repeat, "visible") => (
            at(arg("visible")?)?
                .get(index.ok_or_else(no)?)
                .ok_or_else(no)?,
            Site::Prefix(run("decision "), Span::Decision),
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
            Site::Prefix(run("related "), Span::Related),
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

/// A slot read from the package, before its origin is resolved.
struct Resolved<'p> {
    root: &'p CheckedNodeId,
    site: Site,
    sources: Vec<BareSource>,
}

/// The slots of one control that a span-named origin may be shared by, in the
/// rank order of FR-533 rule 4: `visible`, `cases.guard`, then the `related`
/// ends by record (`from[0]`, `to[0]`, `from[1]`, ...).
fn tied_order<'n>(
    node: &'n Node,
    op: ControlOperation,
    key: &NodeKey,
    package: &'n CheckedPackageV2,
) -> Vec<ProtocolSlotDescriptor> {
    let mut slots: Vec<((usize, usize), ProtocolSlotDescriptor)> = Vec::new();
    for (member, rank) in [
        ("visible", 0),
        ("cases.guard", 1),
        ("related.from", 2),
        ("related.to", 2),
    ] {
        for index in 0.. {
            let descriptor = ProtocolSlotDescriptor::new(key.clone(), member, Some(index));
            if slot_term(node, op, &descriptor, package).is_err() {
                break;
            }
            let order = match member {
                "related.from" => 2 * index,
                "related.to" => 2 * index + 1,
                _ => index,
            };
            slots.push(((rank, order), descriptor));
        }
    }
    slots.sort_by_key(|(order, _)| *order);
    slots
        .into_iter()
        .map(|(_, descriptor)| descriptor)
        .collect()
}

impl<'p> BarePackage<'p> {
    /// Select the slot `descriptor` names from the package's nodes, with the
    /// binders its expression reads and the origin FR-533 "Slot naming on
    /// the bytes route" resolves (FR-533). Charges nothing and takes no
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
        let supply = self.supply().ok_or(BareSelectionFailure::Identity(
            IdentityWitnessRefusal::Missing,
        ))?;
        let identities = supply
            .identities
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
        let walker = BareWalker::new(
            package,
            identities.clone(),
            supply.equality,
            package_work_limit(),
        )
        .map_err(SlotSelectionFailure::Fault)?;
        let reader = TypeReader {
            admitted: package,
            identities: &identities,
            nodes: &nodes,
            graph: &graph,
            work_limit: package_work_limit(),
        };
        let resolve =
            |descriptor: &ProtocolSlotDescriptor| -> Result<Resolved<'_>, BareSelectionFailure> {
                let (term, site) = slot_term(node, op, descriptor, package)?;
                let root = reference(term)
                    .ok_or_else(|| fault("selected-protocol-expression-has-a-keyed-root"))?;
                // The binders: `value`/`parameter` nodes of the root's
                // dependency closure, stopping at a binder and at a function
                // (FR-390 rule 3).
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
                check_equality(package, supply.equality, &seen)?;
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
                        level: usize::try_from(level)
                            .map_err(|_| fault("bare-binder-level-fits"))?,
                        value_type,
                        binder: key,
                    });
                }
                sources.sort_by(|left, right| {
                    (left.level, &left.binder).cmp(&(right.level, &right.binder))
                });
                for (index, source) in sources.iter_mut().enumerate() {
                    source.index = index;
                }
                Ok(Resolved {
                    root,
                    site,
                    sources,
                })
            };
        let slot = resolve(descriptor)?;
        let root = graph
            .key(slot.root)
            .map_err(|_| fault("bare-package-has-structural-keys"))?;
        let origin = match &slot.site {
            Site::Exact(name) => Some(name.clone()),
            Site::Prefix(prefix, span) => {
                let binders = |sources: &[BareSource]| -> Vec<NodeKey> {
                    sources.iter().map(|source| source.binder.clone()).collect()
                };
                let visits = |root: &NodeKey, sources: &[BareSource]| {
                    walker
                        .visits(root, &binders(sources))
                        .map_err(|failure| match failure {
                            WalkFailure::Unsupported(view) => {
                                BareSelectionFailure::Unsupported(Box::new(view))
                            }
                            WalkFailure::Equality(refusal) => {
                                BareSelectionFailure::Equality(refusal)
                            }
                            WalkFailure::Fault(failure) => {
                                SlotSelectionFailure::Fault(failure).into()
                            }
                        })
                };
                let own = visits(&root, &slot.sources)?;
                if own.is_empty() {
                    // A leaf root: no application, no row, no origin.
                    None
                } else {
                    // Rule 2: the origins of the span family whose rows
                    // name exactly the applications of this walk.
                    let rows = walker.origin_rows();
                    let candidates = ordered_candidates(
                        rows.iter()
                            .filter(|(_, held)| **held == own)
                            .map(|(name, _)| *name),
                        prefix,
                        *span,
                    );
                    // Rule 4: this slot's rank among the control's slots
                    // that share its walk.
                    let mut rank = 0;
                    for other in tied_order(node, op, descriptor.node(), package) {
                        if other.member() == descriptor.member()
                            && other.index() == descriptor.index()
                        {
                            break;
                        }
                        let Ok(sibling) = resolve(&other) else {
                            continue;
                        };
                        let Ok(key) = graph.key(sibling.root) else {
                            continue;
                        };
                        if matches!(&sibling.site, Site::Prefix(held, _) if held == prefix)
                            && visits(&key, &sibling.sources)? == own
                        {
                            rank += 1;
                        }
                    }
                    match candidates.get(rank) {
                        Some(name) => Some((*name).to_owned()),
                        None => {
                            // Rule 5.
                            let first = own
                                .iter()
                                .next()
                                .map(|(application, count)| (application, *count));
                            let (application, count) =
                                first.ok_or_else(|| fault("bare-walk-holds-an-application"))?;
                            let key = walker
                                .key(application)
                                .map_err(SlotSelectionFailure::Fault)?;
                            return Err(BareSelectionFailure::Unsupported(Box::new(
                                UnsupportedView::missing_origin(package.name(), prefix, key, count),
                            )));
                        }
                    }
                }
            }
        };
        Ok(BareSlot {
            root,
            origin,
            sources: slot.sources,
        })
    }
}

/// The origins among `names` that the control's span family spells with
/// `prefix` and a span of `form`, in ascending numeric `(start, end)` order,
/// parsing both as decimal integers and not comparing the names' bytes
/// (FR-533 rule 4).
fn ordered_candidates<'a>(
    names: impl Iterator<Item = &'a str>,
    prefix: &str,
    form: Span,
) -> Vec<&'a str> {
    let mut spanned: Vec<((u64, u64), &str)> = names
        .filter_map(|name| Some((form.parse(name.strip_prefix(prefix)?)?, name)))
        .collect();
    spanned.sort();
    spanned.into_iter().map(|(_, name)| name).collect()
}

/// The projection work bound: the I2 reader's default.
fn package_work_limit() -> u64 {
    crate::checked_v2::V2ReadLimits::default().work
}

/// The equality witness of a slot whose walk holds an `==` or `!=`
/// application (FR-531 "Refusals"): decided from the closure before any node
/// is projected, so a `Package` refusal precedes a `Node` one (FR-390-AC-67).
fn check_equality(
    package: &CheckedPackageV2,
    equality: Option<&ReadEqualityRows>,
    closure: &BTreeSet<&CheckedNodeId>,
) -> Result<(), BareSelectionFailure> {
    let held: BTreeSet<&CheckedNodeId> = package
        .checked_term_data()
        .map(|data| {
            data.equality_decisions
                .iter()
                .map(|decision| &decision.application)
                .collect()
        })
        .unwrap_or_default();
    let used: Vec<&&CheckedNodeId> = closure.iter().filter(|id| held.contains(**id)).collect();
    if used.is_empty() {
        return Ok(());
    }
    let rows = equality.ok_or(BareSelectionFailure::Equality(
        EqualityWitnessRefusal::Missing,
    ))?;
    for application in used {
        rows.decision(package, application, package_work_limit())
            .map_err(BareSelectionFailure::Equality)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// FR-533-AC-12, FR-533-AC-14: candidates are ordered by numeric
    /// `(start, end)`, not by the bytes of their names, for a `decision` and
    /// a `related` span, and a name outside the family is no candidate.
    #[trace("FR-533-AC-12", "FR-533-AC-14", "TC-1333")]
    #[test]
    fn candidates_are_ordered_numerically_by_start_then_end() {
        let decision = [
            "protocol P run M C decision 1000..1004",
            "protocol P run M C decision 999..1005",
            "protocol P run M C decision 1001..1002",
            "protocol P run M C related 1 2",
            "protocol P run M D decision 1..2",
            "protocol P run M C decision x..9",
        ];
        let got = ordered_candidates(
            decision.iter().copied(),
            "protocol P run M C decision ",
            Span::Decision,
        );
        assert_eq!(
            got,
            [
                "protocol P run M C decision 999..1005",
                "protocol P run M C decision 1000..1004",
                "protocol P run M C decision 1001..1002",
            ]
        );
        let related = [
            "protocol P run M G related 100 105",
            "protocol P run M G related 99 100",
        ];
        assert_eq!(
            ordered_candidates(
                related.iter().copied(),
                "protocol P run M G related ",
                Span::Related
            ),
            [
                "protocol P run M G related 99 100",
                "protocol P run M G related 100 105"
            ],
            "related.from[0] takes `99 100`, related.to[0] takes `100 105`"
        );
        assert_eq!(Span::Related.parse("1  2"), None);
        assert_eq!(Span::Decision.parse("1..2..3"), None);
    }
}
