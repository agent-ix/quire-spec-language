// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-940: the one sealed read view the analysis engines read a checked
//! package through.
//!
//! [`AnalysisView`] is the table of FR-531 "What the entry derives from the
//! bytes" as a trait. It is implemented for [`AdmittedPackageRead`] (the strict
//! reader's admitted read-back of an in-process package's own emission) and
//! for [`BarePackage`] (a package admitted from bytes), and for no other
//! type: in particular not for [`crate::CheckedPackage`], so nothing read from
//! bytes becomes a checked package (FR-087 R-10). Both implementations read
//! the admitted [`CheckedPackageV2`], never a source-side value, so they
//! return equal values for a package and the bytes it was written to.
//!
//! Each engine family ports onto this trait by adding the accessor for the
//! fact row it reads; an accessor is a provided method over [`package`]
//! so an added fact needs no change to either implementation.
//!
//! [`package`]: AnalysisView::package

use std::collections::{BTreeSet, VecDeque};

use filament_contract_model::{CheckedNodeId, CheckedPackageV2, ClaimNameRow, TemporalClauseName};
use fsl_semantics::library::LibraryName;
use fsl_semantics::model::domain_package::DomainPackageRef;

use crate::checked_views::{self, TemporalSubjectView, ViewRefusal};
use crate::{AdmittedPackageRead, BarePackage};

/// The packages a view's package depends on, each once, nearest first, in
/// lock-selection order. The root package is not listed.
pub type DependencyClosure<'p> = Vec<&'p CheckedPackageV2>;

mod sealed {
    /// Private to `fsl_package`: no other type implements the view.
    pub trait Sealed {}
}

/// What an analysis engine reads of a checked package (FR-940).
///
/// The trait is sealed: `AdmittedPackageRead` and `BarePackage` are its only
/// implementors.
///
/// ```compile_fail,E0277
/// struct Local;
/// impl fsl_package::AnalysisView for Local {
///     fn package(&self) -> &filament_contract_model::CheckedPackageV2 {
///         unreachable!()
///     }
///     fn identity(&self) -> &fsl_semantics::library::LibraryName {
///         unreachable!()
///     }
/// }
/// ```
///
/// A checked package is not a view.
///
/// ```compile_fail,E0277
/// fn takes(_: &dyn fsl_package::AnalysisView) {}
/// fn pass(package: &fsl_package::CheckedPackage) {
///     takes(package);
/// }
/// ```
pub trait AnalysisView: sealed::Sealed {
    /// The admitted package every other accessor reads.
    fn package(&self) -> &CheckedPackageV2;

    /// The package's identity: its admitted `name()`, which the strict
    /// reader keeps nonempty (FR-940 `identity`).
    fn identity(&self) -> &LibraryName;

    /// The model identities the package selects, ascending (FR-533).
    fn model_selections(&self) -> Vec<DomainPackageRef> {
        self.package()
            .selected_model_packages()
            .map(|identity| DomainPackageRef {
                identity: identity.to_owned(),
            })
            .collect()
    }

    /// The dependency closure, read from the direct dependencies the package
    /// retains from its read evidence (FR-531).
    fn dependencies(&self) -> DependencyClosure<'_> {
        closure(self.package())
    }

    /// The package's FR-449 activation map: each protocol clause with the
    /// temporal clause its `requirements` name, in the package's row order.
    fn activation_relation(&self) -> Vec<(CheckedNodeId, CheckedNodeId)> {
        checked_views::activation_relation(self.package())
    }

    /// FR-940: the package-native execution structure of one protocol
    /// declaration, read from this view's admitted protocol nodes. This is
    /// independent of any source `CheckedProtocol` retained by an in-process
    /// compiler.
    ///
    /// # Errors
    ///
    /// [`protocol_view::ViewRefusal`] when the clause or one of its controls
    /// lacks a readable package form.
    fn protocol_execution(
        &self,
        protocol: &CheckedNodeId,
    ) -> Result<crate::protocol_view::ProtocolExecutionView<'_>, crate::protocol_view::ViewRefusal>
    {
        crate::protocol_view::protocol_execution_view(self.package(), protocol)
    }

    /// The package-native execution structure of the one protocol
    /// declaration whose qualified name is `name`, or `None` when the
    /// package has no unique declaration by that name.
    fn protocol_execution_named(
        &self,
        name: &str,
    ) -> Result<
        Option<crate::protocol_view::ProtocolExecutionView<'_>>,
        crate::protocol_view::ViewRefusal,
    > {
        let mut matches = self
            .package()
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| node.semantic_form.as_ref() == "protocol_clause")
            .filter(|node| {
                node.declaration.as_ref().is_some_and(|declaration| {
                    let parts: Vec<&str> = declaration
                        .qualified_name
                        .iter()
                        .map(AsRef::as_ref)
                        .collect();
                    parts.join("::") == name || parts.last().copied() == Some(name)
                })
            });
        let Some(node) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Ok(None);
        }
        self.protocol_execution(&node.node_id).map(Some)
    }
    /// The temporal subject of the clause node `clause` (FR-515): its
    /// identity, `over`, activation, captures, formula operations, operand
    /// edges and `holds` atoms.
    ///
    /// # Errors
    ///
    /// [`ViewRefusal`] when the node is no readable temporal clause.
    fn temporal_subject(
        &self,
        clause: &CheckedNodeId,
    ) -> Result<TemporalSubjectView<'_>, ViewRefusal> {
        checked_views::temporal_subject(self.package(), clause)
    }

    /// The declared name of each temporal clause occurrence: the package's
    /// `temporal_clause_names` rows, in row order (FR-531 "clause name";
    /// FR-850 claim lookup). Empty when the package carries none.
    fn clause_names(&self) -> &[TemporalClauseName] {
        self.package().temporal_clause_names().unwrap_or_default()
    }

    /// The admitted `claim_names` rows, in package order (FR-940 "Claim
    /// names"). Empty when the package carries none.
    fn claim_names(&self) -> &[ClaimNameRow] {
        self.package().claim_names().unwrap_or_default()
    }

    /// FR-940 "Claim names": the occurrence a request's claim `name` names,
    /// resolved against the rows of both name tables (FSpec FR-322 "One claim
    /// namespace"). A temporal clause is reached by its bare name; any other
    /// claim by its `qualified_name`, whole (`a::Name`) or by a bare name that
    /// is the last segment of one declaration's rows. Among the nodes that
    /// declaration's rows name, the answer is its own node (the one whose
    /// declaration set nests strictly inside every other's) at its claim row:
    /// the least `claim` occurrence, else the least row (FSpec FR-322 "Rows
    /// of one declaration"). `None` for a name no row carries or one that
    /// names no single declaration.
    fn claim_name(&self, name: &str) -> Option<ClaimOccurrence> {
        if let Some(row) = self
            .clause_names()
            .iter()
            .find(|row| row.name.as_ref() == name)
        {
            return Some(ClaimOccurrence {
                declaration: row.declaration.clone(),
                occurrence: filament_contract_model::CheckedOccurrence {
                    role: filament_contract_model::CheckedOccurrenceRole::Claim,
                    ordinal: row.occurrence,
                },
            });
        }
        own_claim_row(self.claim_names(), name)
    }

    /// The claim nodes whose name the package does not carry: each node
    /// with a `claim` occurrence that is neither named by a
    /// `temporal_clause_names` or `claim_names` row, nor a named
    /// declaration, nor a `pre` or `post` state clause (which no request
    /// names, FR-625). A package whose writer names every claim (FSpec FR-322
    /// "Claim names") has none (FR-850 claim lookup).
    fn unnamed_claims(&self) -> Vec<&CheckedNodeId> {
        use filament_contract_model::{CheckedNodePathSegment, CheckedOccurrenceRole};
        let named: Vec<&CheckedNodeId> = self
            .clause_names()
            .iter()
            .map(|row| &row.declaration)
            .chain(self.claim_names().iter().map(|row| &row.declaration))
            .collect();
        self.package()
            .resolved_graph()
            .nodes
            .iter()
            .filter(|node| {
                node.occurrences
                    .iter()
                    .any(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim)
                    && node.declaration.is_none()
                    && !named.contains(&&node.node_id)
            })
            .filter(|node| {
                let contract = node.semantic_form.as_ref() == "state_clause"
                    && !matches!(
                        node.node_id.path.get(2),
                        Some(CheckedNodePathSegment::Name(kind)) if kind.as_ref() == "invariant"
                    );
                !contract
            })
            .map(|node| &node.node_id)
            .collect()
    }

    /// Each named declaration's `declaration.qualified_name`, its segments
    /// joined by `::`, with its node, in the package's node order (FSpec
    /// FR-322; FR-850 claim lookup).
    fn declared_names(&self) -> Vec<(String, &CheckedNodeId)> {
        self.package()
            .resolved_graph()
            .nodes
            .iter()
            .filter_map(|node| {
                let declaration = node.declaration.as_ref()?;
                let name = declaration
                    .qualified_name
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<&str>>()
                    .join("::");
                Some((name, &node.node_id))
            })
            .collect()
    }
}

/// The (`declaration`, `occurrence`) a claim name resolves to (FR-940).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimOccurrence {
    /// The claim-bearing node.
    pub declaration: CheckedNodeId,
    /// One of its `declaration` or `claim` occurrences.
    pub occurrence: filament_contract_model::CheckedOccurrence,
}

/// A row's declaration key (FSpec FR-322 "Rows of one declaration"): its
/// `declaration_region` as (`source`, `start`, `end`).
fn declaration_key(row: &ClaimNameRow) -> (&str, &str, u64, u64) {
    let region = &row.declaration_region;
    (
        region.source.authority.as_ref(),
        region.source.identity.as_ref(),
        region.start,
        region.end,
    )
}

/// The claim row of the one declaration whose `qualified_name` `name`
/// names (whole, or by its last segment), on that declaration's own node.
pub(crate) fn own_claim_row(rows: &[ClaimNameRow], name: &str) -> Option<ClaimOccurrence> {
    use filament_contract_model::CheckedOccurrenceRole;
    let path: Vec<&str> = name.split("::").collect();
    let matches = |row: &ClaimNameRow| {
        let qualified: Vec<&str> = row.qualified_name.iter().map(AsRef::as_ref).collect();
        if path.len() == 1 {
            qualified.last() == path.last()
        } else {
            qualified == path
        }
    };
    let named: Vec<&ClaimNameRow> = rows.iter().filter(|row| matches(row)).collect();
    let first = named.first()?;
    // One declaration: every matching row shares its declaration key.
    if named
        .iter()
        .any(|row| declaration_key(row) != declaration_key(first))
    {
        return None;
    }
    let mut nodes: Vec<&CheckedNodeId> = named.iter().map(|row| &row.declaration).collect();
    nodes.sort();
    nodes.dedup();
    let set = |node: &CheckedNodeId| -> BTreeSet<(&str, &str, u64, u64)> {
        rows.iter()
            .filter(|row| row.declaration == *node)
            .map(declaration_key)
            .collect()
    };
    let own = nodes.iter().copied().find(|node| {
        let own = set(node);
        nodes.iter().filter(|other| **other != *node).all(|other| {
            let other = set(other);
            own.is_subset(&other) && own != other
        })
    })?;
    let on_own: Vec<&&ClaimNameRow> = named.iter().filter(|row| row.declaration == *own).collect();
    let row = on_own
        .iter()
        .filter(|row| row.occurrence.role == CheckedOccurrenceRole::Claim)
        .min_by(|left, right| left.occurrence.cmp(&right.occurrence))
        .or_else(|| {
            on_own
                .iter()
                .min_by(|left, right| left.occurrence.cmp(&right.occurrence))
        })?;
    Some(ClaimOccurrence {
        declaration: row.declaration.clone(),
        occurrence: row.occurrence.clone(),
    })
}

/// Walk each retained package once, preserving nearest-first lock order.
pub(crate) fn closure(package: &CheckedPackageV2) -> DependencyClosure<'_> {
    let mut closure: DependencyClosure<'_> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut queue: VecDeque<&CheckedPackageV2> = package.direct_dependencies().collect();
    while let Some(next) = queue.pop_front() {
        if !seen.insert(std::ptr::from_ref(next)) {
            continue;
        }
        closure.push(next);
        queue.extend(next.direct_dependencies());
    }
    closure
}

impl sealed::Sealed for AdmittedPackageRead<'_> {}
impl sealed::Sealed for BarePackage<'_> {}

impl AnalysisView for AdmittedPackageRead<'_> {
    fn package(&self) -> &CheckedPackageV2 {
        self.admitted()
    }

    fn identity(&self) -> &LibraryName {
        self.admitted_identity()
    }
}

impl AnalysisView for BarePackage<'_> {
    fn package(&self) -> &CheckedPackageV2 {
        BarePackage::package(self)
    }

    fn identity(&self) -> &LibraryName {
        self.admitted_identity()
    }
}
