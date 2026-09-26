// SPDX-License-Identifier: AGPL-3.0-or-later
//! The claims of a `Value` function declaration and their requirement
//! records (FR-062 "Requirement records of a value function", FR-057's
//! claim-form table, ADR-014 §4 "Operation application claims").
//!
//! A function body carries one `value-validity` claim for each scalar
//! operation application in it: the application is defined and its result
//! lies in its result bound, for every assignment of its extent roots under
//! which its path condition holds. `check` finds each claim site in the
//! checked body and classifies its extent there ([`claims_of`]), before any
//! node is lowered. Once lowering has keyed every node, [`key_claims`]
//! pairs each site with the `expression` occurrence recorded at the site's
//! own [`Location`], names each extent root by its parameter node, and
//! writes one [`RequirementRecord`] per site.
//!
//! **Scalar operation application.** An application whose operation
//! identity is in the `integer`, `rational`, `decimal`, `ieee`, `quantity`,
//! `numeric`, `text` or `enum` family, other than `quire.op.numeric.narrow`.
//! [`claims_of`] decides it from the checked node's kind, and [`key_claims`]
//! from the lowered node's operation identity; the two must agree on every
//! site, or keying faults.
//!
//! **Result bound.** The target range of the narrow that directly wraps the
//! application, otherwise the application's own result type.
//!
//! **Path condition.** The guards the occurrence is checked under, outermost
//! first: an enclosing `if`'s condition (true in `then`, false in
//! `otherwise`), and the left operand of an enclosing `and`, `or` or
//! `implies` whose right operand holds the occurrence (true for `and` and
//! `implies`, false for `or`). A query filter is no guard.
//!
//! **Extent roots.** Each function parameter and each query, `count`,
//! `sum`, `fold` (accumulator and element) or `reduce` binder read in the
//! application's operand subtrees or in a guard of its path condition,
//! other than a binder bound inside the application's own subtree. A read
//! of a `let` binder contributes the roots its bound value reads; a
//! literal contributes none.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::bound::DomainKey;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::InternalFault;
use quire_exact::{IntegerInterval, NodeKey, Origin, ValueType};

use super::ir::{coerce_builds_narrow, scalar_conversion_target, Node, NodeKind, Slot, Visit};
use super::lowering::SemanticGraph;
use super::node_key::SemanticTerm;
use super::refusal::{KeyFault, Location, Origin as CheckOrigin};
use super::{family::OccurrenceMap, Capability};
use crate::family::{classify_domains, ClaimExtent, ClassifyFailure, DomainKind, Requirements};
use crate::value::declaration::TypeEnvironment;

/// The fault stage name this module reports.
const STAGE: &str = "check.requirements";

/// The narrowing conversion: never a claim of its own.
const NARROW: &str = "quire.op.numeric.narrow";

/// The operation families whose applications are claims.
const SCALAR_FAMILIES: [&str; 8] = [
    "quire.op.integer.",
    "quire.op.rational.",
    "quire.op.decimal.",
    "quire.op.ieee.",
    "quire.op.quantity.",
    "quire.op.numeric.",
    "quire.op.text.",
    "quire.op.enum.",
];

/// Whether a lowered application of `identity` is a scalar operation
/// application.
fn is_scalar_identity(identity: &str) -> bool {
    identity != NARROW
        && SCALAR_FAMILIES
            .iter()
            .any(|family| identity.starts_with(family))
}

/// A binder of a checked body: the node that binds it, by location, and the
/// slot it binds. A function's parameters are bound at its body's root
/// location, each at its own slot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct BinderSite {
    /// The location of the binding node.
    pub(crate) binder: Location,
    /// The bound slot.
    pub(crate) slot: Slot,
}

/// One guard of a claim site's path condition, as `check` walked it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SiteGuard {
    location: Location,
    holds: bool,
}

impl SiteGuard {
    /// The guard node's location.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// The outcome the guard is required to have.
    pub fn holds(&self) -> bool {
        self.holds
    }
}

/// Where a claim site's result bound comes from.
#[derive(Clone, Debug, Eq, PartialEq)]
enum SiteBound {
    /// The target range of the narrow that directly wraps the application.
    Narrowed(IntegerInterval),
    /// The application's own result type.
    Own(ValueType),
}

/// The checked site a claim covers (ADR-012 §2's `ClaimSite`): the
/// application's location, its result bound and its path condition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimSite {
    location: Location,
    bound: SiteBound,
    path_condition: Vec<SiteGuard>,
}

impl ClaimSite {
    /// The checked application's location.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// The application's result bound: the target range of the narrow that
    /// wraps it, otherwise its own result type.
    pub fn result_bound(&self) -> ValueType {
        match &self.bound {
            SiteBound::Narrowed(interval) => ValueType::Int(interval.clone()),
            SiteBound::Own(value_type) => value_type.clone(),
        }
    }

    /// The path condition, outermost guard first.
    pub fn path_condition(&self) -> &[SiteGuard] {
        &self.path_condition
    }
}

/// One `value-validity` claim of a `Value` function declaration: its site
/// and its extent, classified by `check`, each unbounded domain keyed by
/// the binder that carries it until `check` names the binder's parameter
/// node, once lowering has keyed it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueClaim {
    site: ClaimSite,
    domains: BTreeMap<(BinderSite, Vec<u32>), DomainKind>,
}

impl ValueClaim {
    /// The site the claim covers.
    pub fn site(&self) -> &ClaimSite {
        &self.site
    }

    /// The claim's capability kind (FR-057): always `value-validity`.
    pub fn kind(&self) -> Capability {
        Capability::ValueValidity
    }

    /// Whether the claim has no unbounded domain.
    pub fn is_bounded(&self) -> bool {
        self.domains.is_empty()
    }
}

/// A claim's result bound in its record: the checked type and its FR-092
/// type node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultBound {
    value_type: ValueType,
    node: WireNodeId,
}

impl ResultBound {
    /// The result bound's checked type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// The result bound's type node.
    pub fn node(&self) -> WireNodeId {
        self.node
    }
}

/// One guard of a record's path condition: the guard node's `expression`
/// occurrence and its required outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathGuard {
    occurrence: OccurrenceKey,
    holds: bool,
}

impl PathGuard {
    /// The guard node's occurrence.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The outcome the guard is required to have.
    pub fn holds(&self) -> bool {
        self.holds
    }
}

/// One requirement record of the checked package (ADR-012 §13.5, ADR-011
/// E7), keyed in [`super::CheckedGraph::requirements`] by its application's
/// `expression` occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequirementRecord {
    requirements: Requirements,
    result_bound: ResultBound,
    path_condition: Vec<PathGuard>,
}

impl RequirementRecord {
    /// The claim's capability kind and extent, each unbounded domain keyed
    /// by its root's parameter node.
    pub fn requirements(&self) -> &Requirements {
        &self.requirements
    }

    /// The application's result bound.
    pub fn result_bound(&self) -> &ResultBound {
        &self.result_bound
    }

    /// The path condition, outermost guard first.
    pub fn path_condition(&self) -> &[PathGuard] {
        &self.path_condition
    }
}

/// Whether checked `node` lowers to a scalar operation application.
#[deny(clippy::wildcard_enum_match_arm)]
fn is_scalar_application(node: &Node) -> bool {
    match &node.kind {
        NodeKind::Arithmetic(..)
        | NodeKind::Negate(_)
        | NodeKind::Divide { .. }
        | NodeKind::Rational { .. }
        | NodeKind::RationalNegate(..)
        | NodeKind::Decimal { .. }
        | NodeKind::DecimalNegate(..)
        | NodeKind::Ieee(..)
        | NodeKind::Quantity(..)
        | NodeKind::ConvertDecimal(..)
        | NodeKind::Order(..)
        | NodeKind::IeeeToRational(..) => true,
        NodeKind::ConvertScalar(target, operand) => {
            scalar_conversion_target(target, &operand.value_type).is_some()
        }
        NodeKind::Equality(_, _, left, _) => scalar_equality(&left.value_type),
        NodeKind::Literal(_)
        | NodeKind::Coerce(..)
        | NodeKind::Local(_)
        | NodeKind::Let { .. }
        | NodeKind::If { .. }
        | NodeKind::Connective(..)
        | NodeKind::Not(_)
        | NodeKind::Field { .. }
        | NodeKind::Attribute { .. }
        | NodeKind::Present(_)
        | NodeKind::Value(_)
        | NodeKind::Call { .. }
        | NodeKind::ImportedCall { .. }
        | NodeKind::Tuple { .. }
        | NodeKind::Record { .. }
        | NodeKind::Collection { .. }
        | NodeKind::ConvertCollection { .. }
        | NodeKind::Query { .. }
        | NodeKind::Flatten(_)
        | NodeKind::Fold { .. }
        | NodeKind::Size(_)
        | NodeKind::Contains(..)
        | NodeKind::AllInstances { .. }
        | NodeKind::Lookup { .. }
        | NodeKind::Dispatch { .. }
        | NodeKind::Pre(_) => false,
        #[cfg(seam_probe)]
        NodeKind::__SeamProbe => unreachable!("never constructed outside the probe build"),
    }
}

/// Whether an equality over `compared` operands is in a scalar family
/// (FR-093 `Equality` row): Boolean, reference and structural equality are
/// not.
fn scalar_equality(compared: &ValueType) -> bool {
    match compared {
        ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Quantity(_) => true,
        ValueType::Boolean
        | ValueType::Reference(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Float(_)
        | ValueType::Population(_) => false,
    }
}

/// One guard in the walk's arena, linked to the guard enclosing it.
struct Guard {
    location: Location,
    holds: bool,
    reads: BTreeSet<BinderSite>,
    outer: Option<usize>,
}

/// One node the walk has entered and not yet left.
struct Frame<'n> {
    node: &'n Node,
    /// The innermost guard the node is walked under.
    guard: Option<usize>,
    children: Vec<&'n Node>,
    next: usize,
    /// The binders the node's finished children read.
    reads: BTreeSet<BinderSite>,
    /// The binders the first child reads, once it is finished: an `if`'s
    /// condition, a connective's left operand, a `let`'s bound value.
    first_reads: BTreeSet<BinderSite>,
    /// Where a binder this node binds is located: its own location, or an
    /// enclosing `flatten`'s, whose `flat_map` node binds it (FR-093).
    binding: Location,
}

impl<'n> Frame<'n> {
    fn new(node: &'n Node, guard: Option<usize>, binding: Location) -> Self {
        Self {
            node,
            guard,
            children: node.children(),
            next: 0,
            reads: BTreeSet::new(),
            first_reads: BTreeSet::new(),
            binding,
        }
    }
}

/// The extent roots of a body's binders.
#[derive(Default)]
struct Roots {
    /// Each root binder's checked type.
    types: BTreeMap<BinderSite, ValueType>,
    /// The roots a read of each slot contributes: a root binder's slot
    /// itself, a `let` slot the roots its bound value reads.
    slots: BTreeMap<Slot, BTreeSet<BinderSite>>,
}

impl Roots {
    /// Bind `slot` as a root binder at `binder`, typed `value_type`.
    fn bind(&mut self, slot: Slot, binder: &Location, value_type: &ValueType) {
        let site = BinderSite {
            binder: binder.clone(),
            slot,
        };
        self.types.insert(site.clone(), value_type.clone());
        self.slots.insert(slot, BTreeSet::from([site]));
    }
}

/// The binder sites `node` binds, at `binding`: a query's slot, a fold's
/// accumulator and element. A read of one of them is a root only of the
/// applications inside `node`.
fn bound_by(node: &Node, binding: &Location) -> Vec<BinderSite> {
    let site = |slot: Slot| BinderSite {
        binder: binding.clone(),
        slot,
    };
    match &node.kind {
        NodeKind::Query { slot, .. } => vec![site(*slot)],
        NodeKind::Fold {
            accumulator,
            binder,
            ..
        } => vec![site(*accumulator), site(*binder)],
        _ => Vec::new(),
    }
}

/// The walk state a claim reads: the guard arena, each root's type, the
/// type environment and the position ceiling of one classification.
struct Classify<'w> {
    guards: &'w [Guard],
    root_types: &'w BTreeMap<BinderSite, ValueType>,
    types: &'w TypeEnvironment,
    position_limit: u64,
}

/// The element type of a query or fold source.
fn element_type(source: &Node) -> Result<&ValueType, ClassifyFailure> {
    match &source.value_type {
        ValueType::Collection(collection) => Ok(collection.element()),
        _ => Err(ClassifyFailure::Fault(InternalFault::new(
            STAGE,
            "binder-source-is-a-collection",
        ))),
    }
}

/// The claims of a function whose checked body is `body`, whose parameters
/// are `parameters` and whose body root is at `location`: one per scalar
/// operation application in the body, in the order the walk leaves them,
/// each extent classified over `types` with at most `position_limit` type
/// positions per claim.
///
/// The walk keeps its pending nodes on an explicit stack.
pub(crate) fn claims_of(
    body: &Node,
    parameters: &[(String, ValueType)],
    location: &Location,
    types: &TypeEnvironment,
    position_limit: u64,
) -> Result<Vec<ValueClaim>, ClassifyFailure> {
    let mut roots = Roots::default();
    for (slot, (_, value_type)) in parameters.iter().enumerate() {
        roots.bind(slot, location, value_type);
    }
    let mut guards: Vec<Guard> = Vec::new();
    let mut claims = Vec::new();
    let mut stack = vec![Frame::new(body, None, body.location.clone())];
    while let Some(frame) = stack.last_mut() {
        if let Some(&child) = frame.children.get(frame.next) {
            let index = frame.next;
            frame.next += 1;
            let mut guard = frame.guard;
            let mut guard_on = |location: &Location, holds: bool, reads: &BTreeSet<BinderSite>| {
                guards.push(Guard {
                    location: location.clone(),
                    holds,
                    reads: reads.clone(),
                    outer: frame.guard,
                });
                guard = Some(guards.len() - 1);
            };
            match (&frame.node.kind, index) {
                (NodeKind::If { condition, .. }, 1 | 2) => {
                    guard_on(&condition.location, index == 1, &frame.first_reads);
                }
                (NodeKind::Connective(connective, left, _), 1) => {
                    let holds = !matches!(connective, super::ir::Connective::Or);
                    guard_on(&left.location, holds, &frame.first_reads);
                }
                (NodeKind::Let { slot, .. }, 1) => {
                    roots.slots.insert(*slot, frame.first_reads.clone());
                }
                (NodeKind::Query { slot, source, .. }, 1) => {
                    roots.bind(*slot, &frame.binding, element_type(source)?);
                }
                (
                    NodeKind::Fold {
                        accumulator,
                        binder,
                        source,
                        ..
                    },
                    _,
                ) if index + 1 == frame.children.len() => {
                    roots.bind(*accumulator, &frame.binding, &frame.node.value_type);
                    roots.bind(*binder, &frame.binding, element_type(source)?);
                }
                _ => {}
            }
            let binding = match (&frame.node.kind, &child.kind) {
                (
                    NodeKind::Flatten(_),
                    NodeKind::Query {
                        visit: Visit::Map, ..
                    },
                ) => frame.node.location.clone(),
                _ => child.location.clone(),
            };
            stack.push(Frame::new(child, guard, binding));
            continue;
        }
        let Some(frame) = stack.pop() else { break };
        let mut reads = frame.reads;
        if let NodeKind::Local(slot) = &frame.node.kind {
            let Some(read) = roots.slots.get(slot) else {
                return Err(ClassifyFailure::Fault(InternalFault::new(
                    STAGE,
                    "local-slot-bound",
                )));
            };
            reads.extend(read.iter().cloned());
        }
        if is_scalar_application(frame.node) {
            let narrow = stack.last().and_then(|parent| match &parent.node.kind {
                NodeKind::Coerce(operand, interval)
                    if coerce_builds_narrow(&operand.value_type, interval) =>
                {
                    Some(interval)
                }
                _ => None,
            });
            let walk = Classify {
                guards: &guards,
                root_types: &roots.types,
                types,
                position_limit,
            };
            claims.push(walk.claim(frame.node, narrow, frame.guard, &reads)?);
        }
        for bound in bound_by(frame.node, &frame.binding) {
            reads.remove(&bound);
        }
        if let Some(parent) = stack.last_mut() {
            if parent.next == 1 {
                parent.first_reads.clone_from(&reads);
            }
            parent.reads.extend(reads);
        }
    }
    Ok(claims)
}

impl Classify<'_> {
    /// The claim at scalar application `node`, wrapped by a narrow into
    /// `narrow` when one wraps it, under the innermost guard `guard`, whose
    /// operand subtrees read `reads`.
    fn claim(
        &self,
        node: &Node,
        narrow: Option<&IntegerInterval>,
        guard: Option<usize>,
        reads: &BTreeSet<BinderSite>,
    ) -> Result<ValueClaim, ClassifyFailure> {
        let fault = |invariant| ClassifyFailure::Fault(InternalFault::new(STAGE, invariant));
        let mut roots = reads.clone();
        let mut path_condition = Vec::new();
        let mut at = guard;
        while let Some(index) = at {
            let guard = self
                .guards
                .get(index)
                .ok_or_else(|| fault("guard-in-arena"))?;
            roots.extend(guard.reads.iter().cloned());
            path_condition.push(SiteGuard {
                location: guard.location.clone(),
                holds: guard.holds,
            });
            at = guard.outer;
        }
        path_condition.reverse();
        let roots: Vec<BinderSite> = roots.into_iter().collect();
        let mut typed = Vec::with_capacity(roots.len());
        for root in &roots {
            typed.push(
                self.root_types
                    .get(root)
                    .ok_or_else(|| fault("extent-root-typed"))?,
            );
        }
        let mut domains = BTreeMap::new();
        for ((root, path), kind) in classify_domains(&typed, self.types, self.position_limit)? {
            let root = roots
                .get(root)
                .ok_or_else(|| fault("domain-root-classified"))?;
            domains.insert((root.clone(), path), kind);
        }
        let bound = match narrow {
            Some(interval) => SiteBound::Narrowed(interval.clone()),
            None => SiteBound::Own(node.value_type.clone()),
        };
        Ok(ValueClaim {
            site: ClaimSite {
                location: node.location.clone(),
                bound,
                path_condition,
            },
            domains,
        })
    }
}

/// The wire id of lowered node `key`.
fn wire(key: NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

/// Key each claim of `claims` by the `expression` occurrence of its scalar
/// application recorded at its site's own location, and write its record
/// (ADR-012 §13.5, ADR-013 O-07): its extent's roots named by the parameter
/// nodes `binders` holds, its result bound's type node and its guards'
/// occurrences, all read from `graph` and `occurrences` after lowering.
///
/// A site is paired by location, never by the order sites or occurrences
/// were produced. A claim is never dropped: a site with no scalar
/// application occurrence at its location (one with only a `generated`
/// occurrence included), a narrowed site with no narrow over its
/// application, a guard or root with no node, two claims at one occurrence,
/// or a scalar application occurrence in a function body that no claim
/// names, is `KeyFault::UnkeyableRequirements`.
pub(crate) fn key_claims(
    claims: Vec<ValueClaim>,
    occurrences: &OccurrenceMap<Location>,
    graph: &SemanticGraph,
    binders: &BTreeMap<BinderSite, NodeKey>,
) -> Result<BTreeMap<OccurrenceKey, RequirementRecord>, KeyFault> {
    let unkeyable = || KeyFault::UnkeyableRequirements;
    let mut at: BTreeMap<&Location, Vec<(NodeKey, Origin)>> = BTreeMap::new();
    for (node, origin, location) in occurrences.iter() {
        if origin.role().as_str() == "expression" {
            at.entry(location).or_default().push((node, origin));
        }
    }
    let application = |key: NodeKey| match graph.node(key).map(|node| node.body()) {
        Some(SemanticTerm::Application {
            operation,
            arguments,
            ..
        }) => Some((operation.identity(), arguments.as_slice())),
        _ => None,
    };
    let scalar =
        |key: NodeKey| application(key).is_some_and(|(identity, _)| is_scalar_identity(identity));
    let mut records = BTreeMap::new();
    for claim in claims {
        let site = claim.site;
        let here = at.get(&site.location).map_or(&[][..], Vec::as_slice);
        let mut applications = here.iter().filter(|(key, _)| scalar(*key));
        let (Some((key, origin)), None) = (applications.next(), applications.next()) else {
            return Err(unkeyable());
        };
        let bound_type = if matches!(site.bound, SiteBound::Narrowed(_)) {
            here.iter()
                .find(|(narrow, _)| {
                    application(*narrow).is_some_and(|(identity, arguments)| {
                        identity == NARROW
                            && matches!(arguments, [SemanticTerm::Reference { target }] if target.0 == *key)
                    })
                })
                .and_then(|(narrow, _)| graph.node(*narrow)?.semantic_type())
        } else {
            graph.node(*key).and_then(|node| node.semantic_type())
        }
        .ok_or_else(unkeyable)?;
        let mut path_condition = Vec::with_capacity(site.path_condition.len());
        for guard in &site.path_condition {
            let here = at.get(&guard.location).map_or(&[][..], Vec::as_slice);
            let (node, origin) = outermost(here, graph).ok_or_else(unkeyable)?;
            path_condition.push(PathGuard {
                occurrence: OccurrenceKey::new(wire(node), origin),
                holds: guard.holds,
            });
        }
        let mut domains = BTreeMap::new();
        for ((root, path), kind) in claim.domains {
            let node = binders.get(&root).ok_or_else(unkeyable)?;
            domains.insert(DomainKey::new(wire(*node), path), kind);
        }
        let record = RequirementRecord {
            requirements: Requirements::new(
                Capability::ValueValidity,
                ClaimExtent::from_domains(domains),
            ),
            result_bound: ResultBound {
                value_type: site.result_bound(),
                node: wire(bound_type),
            },
            path_condition,
        };
        if records
            .insert(OccurrenceKey::new(wire(*key), origin.clone()), record)
            .is_some()
        {
            return Err(unkeyable());
        }
    }
    for (location, here) in &at {
        if !matches!(location.origin, CheckOrigin::Body { .. }) {
            continue;
        }
        for (key, origin) in here {
            if scalar(*key)
                && !records.contains_key(&OccurrenceKey::new(wire(*key), origin.clone()))
            {
                return Err(unkeyable());
            }
        }
    }
    Ok(records)
}

/// The one occurrence in `here` whose node no other node in `here` names:
/// the node lowered from the checked node at that location, when a node
/// lowered with it (a `deref` under its `record.project`) shares it.
fn outermost(here: &[(NodeKey, Origin)], graph: &SemanticGraph) -> Option<(NodeKey, Origin)> {
    let names = |outer: NodeKey, inner: NodeKey| {
        let mut named = false;
        if let Some(node) = graph.node(outer) {
            node.body().for_each_key(&mut |key| named |= key == inner);
        }
        named
    };
    let mut outer = here.iter().filter(|(inner, _)| {
        !here
            .iter()
            .any(|(other, _)| other != inner && names(*other, *inner))
    });
    match (outer.next(), outer.next()) {
        (Some(found), None) => Some(found.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
