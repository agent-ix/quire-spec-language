// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-490 "Checked-package node": the body of a
//! `correspondence`/`implementation_guarantee` node.
//!
//! The IR crate has no term for this body, so the term is QSL-owned. Its
//! arrays are in FR-490's canonical order whatever order they were built in:
//! [`ImplementationGuaranteeTerm::new`] sorts them, and
//! [`ImplementationGuaranteeTerm::from_wire`] keeps the order it is given so
//! a reader can refuse an out-of-order node. Each Rust path is the array of
//! its segments.

use std::collections::BTreeSet;

use quire_exact::NodeKey;

use super::{NodeKeyRefusal, NodeRef};

/// An implementation guarantee node's body, `{term:
/// "implementation_guarantee", clause, relation, initial, progress,
/// environment}` (QSpec FR-490).
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ImplementationGuaranteeTerm {
    clause: NodeRef,
    relation: NodeRef,
    initial: Vec<String>,
    progress: Vec<ProgressEntry>,
    environment: Vec<EnvironmentEntry>,
}

qsl_foundation::debug_with_struct!(ImplementationGuaranteeTerm, "ImplementationGuaranteeTerm" { clause: with, relation: with, initial: plain, progress: with, environment: with });

/// A progress measure: `{decreases: RustPath}` or `{bound: PositiveCount}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MeasureEntry {
    /// `{decreases: RustPath}`: the segments of the measure function's path.
    Decreases(Vec<String>),
    /// `{bound: PositiveCount}`.
    Bound(u64),
}

/// One `progress` entry: `{context, operation, measure}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ProgressEntry {
    context: NodeRef,
    operation: String,
    measure: MeasureEntry,
}

qsl_foundation::debug_with_struct!(ProgressEntry, "ProgressEntry" { context: with, operation: plain, measure: plain });

/// What an `environment` entry supplies: `{fairness: {kind, granularity,
/// context, operation}}` or `{assumption: NodeRef}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub enum SuppliesEntry {
    /// A scheduling premise.
    Fairness {
        /// `weak` or `strong`.
        kind: String,
        /// `whole` or `each`.
        granularity: String,
        /// The operation's declaring object type node.
        context: NodeRef,
        /// The operation name.
        operation: String,
    },
    /// An assumption premise: the `predicate` node.
    Assumption(NodeRef),
}

impl qsl_foundation::source::node_table::DebugWith for SuppliesEntry {
    fn fmt_with(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        table: &qsl_foundation::source::node_table::DebugTable,
    ) -> std::fmt::Result {
        use qsl_foundation::source::node_table::With;
        match self {
            Self::Fairness {
                kind,
                granularity,
                context,
                operation,
            } => f
                .debug_struct("Fairness")
                .field("kind", kind)
                .field("granularity", granularity)
                .field("context", &With(context, table))
                .field("operation", operation)
                .finish(),
            Self::Assumption(node) => f
                .debug_tuple("Assumption")
                .field(&With(node, table))
                .finish(),
        }
    }
}

impl std::fmt::Debug for SuppliesEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        qsl_foundation::source::node_table::standalone(f, self)
    }
}

/// One `environment` entry: `{name, supplier, supplies}`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct EnvironmentEntry {
    name: String,
    supplier: String,
    supplies: SuppliesEntry,
}

qsl_foundation::debug_with_struct!(EnvironmentEntry, "EnvironmentEntry" { name: plain, supplier: plain, supplies: with });

impl ProgressEntry {
    /// The operation's declaring object type node.
    pub fn context(&self) -> &NodeRef {
        &self.context
    }

    /// The operation name.
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// The measure.
    pub fn measure(&self) -> &MeasureEntry {
        &self.measure
    }

    /// The entry for `operation` of the object type `context`.
    pub fn new(context: NodeKey, operation: String, measure: MeasureEntry) -> Self {
        Self {
            context: NodeRef::Admitted(context),
            operation,
            measure,
        }
    }
}

impl EnvironmentEntry {
    /// The premise name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The supplier text.
    pub fn supplier(&self) -> &str {
        &self.supplier
    }

    /// What the entry supplies.
    pub fn supplies(&self) -> &SuppliesEntry {
        &self.supplies
    }

    /// The entry `name` supplied by `supplier`.
    pub fn new(name: String, supplier: String, supplies: SuppliesEntry) -> Self {
        Self {
            name,
            supplier,
            supplies,
        }
    }
}

impl ImplementationGuaranteeTerm {
    /// The body holding these entries, each array in FR-490's canonical
    /// order: `progress` by `context` node id then `operation` as UTF-8
    /// bytes, `environment` by `name` as UTF-8 bytes.
    pub fn new(
        clause: NodeKey,
        relation: NodeKey,
        initial: Vec<String>,
        mut progress: Vec<ProgressEntry>,
        mut environment: Vec<EnvironmentEntry>,
    ) -> Self {
        progress.sort_by(|left, right| {
            (&left.context, &left.operation).cmp(&(&right.context, &right.operation))
        });
        environment.sort_by(|left, right| left.name.cmp(&right.name));
        Self::from_wire(clause, relation, initial, progress, environment)
    }

    /// The body holding these entries in exactly the order given, as a
    /// reader found them: [`Self::is_canonical`] says whether the order is
    /// FR-490's.
    pub fn from_wire(
        clause: NodeKey,
        relation: NodeKey,
        initial: Vec<String>,
        progress: Vec<ProgressEntry>,
        environment: Vec<EnvironmentEntry>,
    ) -> Self {
        Self {
            clause: NodeRef::Admitted(clause),
            relation: NodeRef::Admitted(relation),
            initial,
            progress,
            environment,
        }
    }

    /// The `clause` node.
    pub fn clause(&self) -> &NodeRef {
        &self.clause
    }

    /// The `relation` node.
    pub fn relation(&self) -> &NodeRef {
        &self.relation
    }

    /// The segments of the `initial` Rust path.
    pub fn initial(&self) -> &[String] {
        &self.initial
    }

    /// The `progress` entries.
    pub fn progress(&self) -> &[ProgressEntry] {
        &self.progress
    }

    /// The `environment` entries.
    pub fn environment(&self) -> &[EnvironmentEntry] {
        &self.environment
    }

    /// Whether `progress` is ascending by `context` then `operation` and
    /// `environment` ascending by `name`, each strictly (no duplicate key).
    pub fn is_canonical(&self) -> bool {
        self.progress.windows(2).all(|pair| {
            (&pair[0].context, &pair[0].operation) < (&pair[1].context, &pair[1].operation)
        }) && self
            .environment
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name)
    }

    /// Every admitted `clause`, `relation`, `context` and `assumption`
    /// target in structural order: the node's `dependencies`. A pending
    /// target is a lowering fault and cannot be emitted.
    pub fn targets(&self) -> Result<BTreeSet<NodeKey>, NodeKeyRefusal> {
        let environment = self
            .environment
            .iter()
            .filter_map(|entry| match &entry.supplies {
                SuppliesEntry::Assumption(node) => Some(node),
                SuppliesEntry::Fairness { .. } => None,
            });
        [&self.clause, &self.relation]
            .into_iter()
            .chain(self.progress.iter().map(|entry| &entry.context))
            .chain(environment)
            .map(|reference| {
                reference
                    .admitted()
                    .cloned()
                    .ok_or(NodeKeyRefusal::UnresolvedPlaceholder)
            })
            .collect()
    }

    /// Call `visit` with every node reference this term holds.
    pub(crate) fn for_each_ref(&self, visit: &mut impl FnMut(&NodeRef)) {
        visit(&self.clause);
        visit(&self.relation);
        for entry in &self.progress {
            visit(&entry.context);
        }
        for entry in &self.environment {
            match &entry.supplies {
                SuppliesEntry::Fairness { context, .. } => visit(context),
                SuppliesEntry::Assumption(node) => visit(node),
            }
        }
    }

    /// Remap every node reference.
    pub(crate) fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        Self {
            clause: map(&self.clause),
            relation: map(&self.relation),
            initial: self.initial.clone(),
            progress: self
                .progress
                .iter()
                .map(|entry| ProgressEntry {
                    context: map(&entry.context),
                    ..entry.clone()
                })
                .collect(),
            environment: self
                .environment
                .iter()
                .map(|entry| EnvironmentEntry {
                    supplies: match &entry.supplies {
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
                    ..entry.clone()
                })
                .collect(),
        }
    }
}
