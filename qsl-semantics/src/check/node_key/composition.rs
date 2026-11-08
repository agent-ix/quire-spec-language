// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-491 "Checked-package node": the body of a
//! `correspondence`/`component_composition` node.
//!
//! The IR crate has no term for this body, so the term is QSL-owned. Its
//! arrays are in FR-491's canonical order whatever order they were built in:
//! [`CompositionTerm::new`] sorts them, and [`CompositionTerm::from_wire`]
//! keeps the order it is given so a reader can refuse an out-of-order node.

use std::collections::BTreeSet;

use quire_exact::NodeKey;

use super::{EnvironmentEntry, NodeKeyRefusal, NodeRef, SuppliesEntry};

/// A composition rule (QSpec FR-491 "Declaration form").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CompositionRule {
    /// `assume-guarantee` (FR-176).
    AssumeGuarantee,
    /// `protocol-refinement` (FR-177).
    ProtocolRefinement,
    /// `refinement-transfer` (FR-380).
    RefinementTransfer,
    /// `interface` (FR-348).
    Interface,
}

impl CompositionRule {
    /// The rule's spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AssumeGuarantee => "assume-guarantee",
            Self::ProtocolRefinement => "protocol-refinement",
            Self::RefinementTransfer => "refinement-transfer",
            Self::Interface => "interface",
        }
    }

    /// The rule a spelling names: the one place a node's `rule` is read.
    #[ix_trace_rs::string_edge]
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        [
            Self::AssumeGuarantee,
            Self::ProtocolRefinement,
            Self::RefinementTransfer,
            Self::Interface,
        ]
        .into_iter()
        .find(|rule| rule.as_str() == spelling)
    }
}

/// A component composition node's body, `{term: "component_composition",
/// rule, components, guarantees, relies, liveness_dependencies, supplies,
/// environment}` (QSpec FR-491).
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct CompositionTerm {
    rule: CompositionRule,
    components: Vec<ComponentEntry>,
    guarantees: Vec<ClauseEntry>,
    relies: Vec<PremiseEntry>,
    liveness_dependencies: Vec<PremiseEntry>,
    supplies: Vec<SupplyEntry>,
    environment: Vec<EnvironmentEntry>,
}

qsl_foundation::debug_with_struct!(CompositionTerm, "CompositionTerm" { rule: plain, components: with, guarantees: with, relies: with, liveness_dependencies: with, supplies: with, environment: with });

/// One component: `{name, package, subject}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ComponentEntry {
    name: String,
    package: String,
    subject: NodeRef,
}

qsl_foundation::debug_with_struct!(ComponentEntry, "ComponentEntry" { name: plain, package: plain, subject: with });

/// A component's temporal clause: `{component, clause}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ClauseEntry {
    component: String,
    clause: NodeRef,
}

qsl_foundation::debug_with_struct!(ClauseEntry, "ClauseEntry" { component: plain, clause: with });

/// A `rely` or `depends` premise: `{guarantee, on}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PremiseEntry {
    guarantee: ClauseEntry,
    on: ClauseEntry,
}

qsl_foundation::debug_with_struct!(PremiseEntry, "PremiseEntry" { guarantee: with, on: with });

/// A scheduling `supplies` constraint: `{kind, granularity, context,
/// operation}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct SupplyEntry {
    kind: String,
    granularity: String,
    context: NodeRef,
    operation: String,
}

qsl_foundation::debug_with_struct!(SupplyEntry, "SupplyEntry" { kind: plain, granularity: plain, context: with, operation: plain });

impl ComponentEntry {
    /// The component `name` over the package `package`, whose subject is the
    /// node `subject` of that package.
    pub fn new(name: String, package: String, subject: NodeKey) -> Self {
        Self {
            name,
            package,
            subject: NodeRef::Admitted(subject),
        }
    }

    /// The component name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The identity of the component's package.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The component's subject node.
    pub fn subject(&self) -> &NodeRef {
        &self.subject
    }
}

impl ClauseEntry {
    /// The clause `clause` of the component `component`.
    pub fn new(component: String, clause: NodeKey) -> Self {
        Self {
            component,
            clause: NodeRef::Admitted(clause),
        }
    }

    /// The component name.
    pub fn component(&self) -> &str {
        &self.component
    }

    /// The clause node.
    pub fn clause(&self) -> &NodeRef {
        &self.clause
    }

    fn sort_key(&self) -> (&str, &NodeRef) {
        (&self.component, &self.clause)
    }
}

impl PremiseEntry {
    /// The premise of `guarantee` on the peer `on`.
    pub fn new(guarantee: ClauseEntry, on: ClauseEntry) -> Self {
        Self { guarantee, on }
    }

    /// The guarantee that carries the premise.
    pub fn guarantee(&self) -> &ClauseEntry {
        &self.guarantee
    }

    /// The peer guarantee the premise is on.
    pub fn on(&self) -> &ClauseEntry {
        &self.on
    }

    fn sort_key(&self) -> ((&str, &NodeRef), (&str, &NodeRef)) {
        (self.guarantee.sort_key(), self.on.sort_key())
    }
}

impl SupplyEntry {
    /// The supplied constraint over `operation` of the object type `context`.
    pub fn new(kind: String, granularity: String, context: NodeKey, operation: String) -> Self {
        Self {
            kind,
            granularity,
            context: NodeRef::Admitted(context),
            operation,
        }
    }

    /// `weak` or `strong`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// `whole` or `each`.
    pub fn granularity(&self) -> &str {
        &self.granularity
    }

    /// The operation's declaring object type node.
    pub fn context(&self) -> &NodeRef {
        &self.context
    }

    /// The operation name.
    pub fn operation(&self) -> &str {
        &self.operation
    }

    fn sort_key(&self) -> (&NodeRef, &str, &str, &str) {
        (
            &self.context,
            &self.operation,
            &self.kind,
            &self.granularity,
        )
    }
}

impl CompositionTerm {
    /// The body holding these entries, each array in FR-491's canonical
    /// order: `components` by `name` bytes, `guarantees`, `relies` and
    /// `liveness_dependencies` by their first `component` name then `clause`
    /// node id, then the second member's, `supplies` by `context` then
    /// `operation`, and `environment` by `name`.
    pub fn new(
        rule: CompositionRule,
        mut components: Vec<ComponentEntry>,
        mut guarantees: Vec<ClauseEntry>,
        mut relies: Vec<PremiseEntry>,
        mut liveness_dependencies: Vec<PremiseEntry>,
        mut supplies: Vec<SupplyEntry>,
        mut environment: Vec<EnvironmentEntry>,
    ) -> Self {
        components.sort_by(|left, right| left.name.cmp(&right.name));
        guarantees.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        relies.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        liveness_dependencies.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        supplies.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        environment.sort_by(|left, right| left.name().cmp(right.name()));
        Self::from_wire(
            rule,
            components,
            guarantees,
            relies,
            liveness_dependencies,
            supplies,
            environment,
        )
    }

    /// The body holding these entries in exactly the order given, as a
    /// reader found them: [`Self::is_canonical`] says whether the order is
    /// FR-491's.
    pub fn from_wire(
        rule: CompositionRule,
        components: Vec<ComponentEntry>,
        guarantees: Vec<ClauseEntry>,
        relies: Vec<PremiseEntry>,
        liveness_dependencies: Vec<PremiseEntry>,
        supplies: Vec<SupplyEntry>,
        environment: Vec<EnvironmentEntry>,
    ) -> Self {
        Self {
            rule,
            components,
            guarantees,
            relies,
            liveness_dependencies,
            supplies,
            environment,
        }
    }

    /// The selected rule.
    pub fn rule(&self) -> CompositionRule {
        self.rule
    }

    /// The `components` entries.
    pub fn components(&self) -> &[ComponentEntry] {
        &self.components
    }

    /// The `guarantees` entries.
    pub fn guarantees(&self) -> &[ClauseEntry] {
        &self.guarantees
    }

    /// The `relies` entries.
    pub fn relies(&self) -> &[PremiseEntry] {
        &self.relies
    }

    /// The `liveness_dependencies` entries.
    pub fn liveness_dependencies(&self) -> &[PremiseEntry] {
        &self.liveness_dependencies
    }

    /// The `supplies` entries.
    pub fn supplies(&self) -> &[SupplyEntry] {
        &self.supplies
    }

    /// The `environment` entries.
    pub fn environment(&self) -> &[EnvironmentEntry] {
        &self.environment
    }

    /// Whether every array is strictly ascending in FR-491's key (no
    /// duplicate key).
    pub fn is_canonical(&self) -> bool {
        self.components
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name)
            && self
                .guarantees
                .windows(2)
                .all(|pair| pair[0].sort_key() < pair[1].sort_key())
            && self
                .relies
                .windows(2)
                .all(|pair| pair[0].sort_key() < pair[1].sort_key())
            && self
                .liveness_dependencies
                .windows(2)
                .all(|pair| pair[0].sort_key() < pair[1].sort_key())
            && self
                .supplies
                .windows(2)
                .all(|pair| pair[0].sort_key() < pair[1].sort_key())
            && self
                .environment
                .windows(2)
                .all(|pair| pair[0].name() < pair[1].name())
    }

    /// Every admitted `subject`, `clause`, `context` and `assumption` target:
    /// the node's `dependencies`. A pending target is a lowering fault and
    /// cannot be emitted.
    pub fn targets(&self) -> Result<BTreeSet<NodeKey>, NodeKeyRefusal> {
        let mut references: Vec<&NodeRef> = Vec::new();
        self.visit(&mut |reference| references.push(reference));
        references
            .into_iter()
            .map(|reference| {
                reference
                    .admitted()
                    .cloned()
                    .ok_or(NodeKeyRefusal::UnresolvedPlaceholder)
            })
            .collect()
    }

    fn visit<'a>(&'a self, visit: &mut impl FnMut(&'a NodeRef)) {
        for component in &self.components {
            visit(&component.subject);
        }
        for guarantee in &self.guarantees {
            visit(&guarantee.clause);
        }
        for premise in self.relies.iter().chain(&self.liveness_dependencies) {
            visit(&premise.guarantee.clause);
            visit(&premise.on.clause);
        }
        for supply in &self.supplies {
            visit(&supply.context);
        }
        for entry in &self.environment {
            match entry.supplies() {
                SuppliesEntry::Fairness { context, .. } => visit(context),
                SuppliesEntry::Assumption(node) => visit(node),
            }
        }
    }

    /// Call `visit` with every node reference this term holds.
    pub(crate) fn for_each_ref(&self, visit: &mut impl FnMut(&NodeRef)) {
        self.visit(&mut |reference| visit(reference));
    }

    /// Remap every node reference.
    pub(crate) fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        fn clause(entry: &ClauseEntry, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> ClauseEntry {
            ClauseEntry {
                component: entry.component.clone(),
                clause: map(&entry.clause),
            }
        }
        fn premise(
            entry: &PremiseEntry,
            map: &mut impl FnMut(&NodeRef) -> NodeRef,
        ) -> PremiseEntry {
            PremiseEntry {
                guarantee: clause(&entry.guarantee, map),
                on: clause(&entry.on, map),
            }
        }
        let components = self
            .components
            .iter()
            .map(|entry| ComponentEntry {
                subject: map(&entry.subject),
                ..entry.clone()
            })
            .collect();
        let guarantees = self
            .guarantees
            .iter()
            .map(|entry| clause(entry, map))
            .collect();
        let relies = self
            .relies
            .iter()
            .map(|entry| premise(entry, map))
            .collect();
        let liveness_dependencies = self
            .liveness_dependencies
            .iter()
            .map(|entry| premise(entry, map))
            .collect();
        let supplies = self
            .supplies
            .iter()
            .map(|entry| SupplyEntry {
                context: map(&entry.context),
                ..entry.clone()
            })
            .collect();
        let environment = self
            .environment
            .iter()
            .map(|entry| {
                EnvironmentEntry::new(
                    entry.name().to_owned(),
                    entry.supplier().to_owned(),
                    match entry.supplies() {
                        SuppliesEntry::Fairness {
                            kind,
                            granularity,
                            context,
                            operation,
                        } => SuppliesEntry::Fairness {
                            kind: kind.clone(),
                            granularity: granularity.clone(),
                            context: map(context),
                            operation: operation.clone(),
                        },
                        SuppliesEntry::Assumption(node) => SuppliesEntry::Assumption(map(node)),
                    },
                )
            })
            .collect();
        Self {
            rule: self.rule,
            components,
            guarantees,
            relies,
            liveness_dependencies,
            supplies,
            environment,
        }
    }
}
