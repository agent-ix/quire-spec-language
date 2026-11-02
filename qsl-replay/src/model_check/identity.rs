// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337-AC-5 and FR-338 Inputs: a clause's identity, resolved from the
//! package.
//!
//! A TP-1 clause's key contexts name its identity triple and its `over`
//! node. The triple is the `claim` occurrence of the clause's
//! `temporal`/`temporal_clause` node, and the `over` node is the
//! `value`/`parameter` node that clause node's body references. Neither
//! comes from a caller: [`ClauseIdentity::resolve`] emits the request's
//! package, reads the bytes back through the I2 reader and returns what
//! the I2 lookup ([`resolve_clause_root`]) finds there. A clause the
//! package does not declare, or that the lookup does not find, is
//! `clause-root-unresolved` ([`ClauseRootRefusal`]).

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::{
    emit_checked_with_temporal_selections_and_cancel, read_import_view, resolve_clause_root,
    resolve_hyper_root, resolve_state_graph_root, AdmittedPackages, CheckedPackage,
    ClauseRootUnresolved, EmitRefusal, TemporalClockSelection, V2ReadLimits,
};
use qsl_semantics::check::{
    CheckedHyperClause, CheckedStateGraphClaim, CheckedTemporalClause, TemporalClauseNode,
};
use qsl_semantics::library::LibraryName;
use quire_exact::NodeKey;
use quire_exact::{Cancel, CancelCause};

/// The `claim` occurrence of a checked clause `node`: its node id and claim
/// origin (ADR-013 O-07).
pub fn node_occurrence(node: &TemporalClauseNode) -> OccurrenceKey {
    OccurrenceKey::new(node.identity().clone(), node.claim().clone())
}

/// A clause's identity in its package: the checked occurrence and the node of
/// its `over` parameter, `None` for a clause with no `over`.
///
/// Only [`ClauseIdentity::resolve`] makes one: a request cannot name it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseIdentity {
    occurrence: OccurrenceKey,
    over: Option<NodeKey>,
}

impl ClauseIdentity {
    /// The clause's O-07 occurrence: node, role and ordinal.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `over` parameter's checked node.
    pub fn over(&self) -> Option<NodeKey> {
        self.over.clone()
    }

    /// The identity of `clause` in `package`, read from the package's
    /// emitted `quire.checked-package/v2` bytes through the I2 lookup.
    /// `domain_packages` is FR-056's package input the read checks the
    /// package's model selections against.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal`] when `package` does not declare `clause`, does
    /// not emit or read back, or the lookup does not find the clause's root;
    /// or `cancel` stopped the emission.
    pub fn resolve(
        package: &CheckedPackage,
        domain_packages: &qsl_semantics::model::intake::PackageInput,
        clause: &CheckedTemporalClause,
        temporal_clock_selections: &[TemporalClockSelection],
        cancel: &Cancel,
    ) -> Result<Self, ClauseRootRefusal> {
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause() == clause)
            .ok_or_else(|| ClauseRootRefusal::Undeclared {
                name: clause.name().to_owned(),
            })?;
        let triple = node_occurrence(node);
        let (admitted, library) =
            read_back(package, domain_packages, temporal_clock_selections, cancel)?;
        let read = admitted
            .package(&library)
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_clause_root(read, &triple)?;
        Ok(Self {
            occurrence: root.occurrence().clone(),
            over: root.over().cloned(),
        })
    }
}

/// `package` emitted under `cancel` and read back through the I2 reader,
/// with the library name the read recorded it under.
fn read_back(
    package: &CheckedPackage,
    domain_packages: &qsl_semantics::model::intake::PackageInput,
    temporal_clock_selections: &[TemporalClockSelection],
    cancel: &Cancel,
) -> Result<(AdmittedPackages, LibraryName), ClauseRootRefusal> {
    let emission = emit_checked_with_temporal_selections_and_cancel(
        package,
        temporal_clock_selections,
        cancel,
    )
    .map_err(|refusal| match refusal {
        EmitRefusal::Cancelled { cause } => ClauseRootRefusal::Cancelled(cause),
        other => ClauseRootRefusal::Package(other.to_string()),
    })?;
    let library = LibraryName::new(emission.package().name())
        .map_err(|refusal| ClauseRootRefusal::Package(format!("{refusal:?}")))?;
    let mut admitted = AdmittedPackages::default();
    read_import_view(
        package,
        &emission,
        library.clone(),
        domain_packages,
        &mut admitted,
        V2ReadLimits::default(),
    )
    .map_err(|refusal| ClauseRootRefusal::Package(format!("{refusal:?}")))?;
    Ok((admitted, library))
}

/// A state-graph claim's identity in its package: the checked occurrence and
/// the node of its `over` parameter, `None` for a claim with no `over`
/// (FR-410 "Behavior").
///
/// Only [`StateGraphIdentity::resolve`] makes one: a request cannot name it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphIdentity {
    occurrence: OccurrenceKey,
    over: Option<NodeKey>,
}

impl StateGraphIdentity {
    /// The claim's O-07 occurrence: node, role and ordinal.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `over` parameter's checked node.
    pub fn over(&self) -> Option<NodeKey> {
        self.over.clone()
    }

    /// The identity of `claim` in `package`, read from the package's emitted
    /// `quire.checked-package/v2` bytes through the I2 state-graph lookup.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal`] when `claim` carries no occurrence, the package
    /// does not emit or read back, or the lookup does not find the claim's
    /// triple; or `cancel` stopped the emission.
    pub fn resolve(
        package: &CheckedPackage,
        domain_packages: &qsl_semantics::model::intake::PackageInput,
        claim: &CheckedStateGraphClaim,
        cancel: &Cancel,
    ) -> Result<Self, ClauseRootRefusal> {
        let triple = claim
            .occurrence()
            .ok_or_else(|| ClauseRootRefusal::Undeclared {
                name: claim.name().to_owned(),
            })?;
        let (admitted, library) = read_back(package, domain_packages, &[], cancel)?;
        let read = admitted
            .package(&library)
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_state_graph_root(read, triple)
            .map_err(|fault| ClauseRootRefusal::Package(format!("{fault:?}")))??;
        Ok(Self {
            occurrence: root.occurrence().clone(),
            over: root.over().cloned(),
        })
    }
}

/// An HP-5 clause's identity in its package, resolved through FR-488's I2
/// hyper-root lookup. A replay request cannot supply this identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperIdentity {
    occurrence: OccurrenceKey,
}

impl HyperIdentity {
    /// The checked `claim` occurrence of the hyper clause.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// Resolve the assembled hyper clause through the emitted package's I2
    /// reader and FR-488's hyper-root lookup.
    ///
    /// # Errors
    ///
    /// [`ClauseRootRefusal`] when the package does not assemble and read back
    /// the clause's claim root, or when `cancel` stops emission.
    pub fn resolve(
        package: &CheckedPackage,
        domain_packages: &qsl_semantics::model::intake::PackageInput,
        clause: &CheckedHyperClause,
        cancel: &Cancel,
    ) -> Result<Self, ClauseRootRefusal> {
        let assembled = package
            .graph()
            .hyper_clauses()
            .iter()
            .find(|candidate| {
                candidate.name() == clause.name()
                    && candidate.assembled_occurrence() == clause.assembled_occurrence()
            })
            .ok_or_else(|| ClauseRootRefusal::Undeclared {
                name: clause.name().to_owned(),
            })?;
        let occurrence = assembled.assembled_occurrence().cloned().ok_or_else(|| {
            ClauseRootRefusal::Undeclared {
                name: clause.name().to_owned(),
            }
        })?;
        let (admitted, library) = read_back(package, domain_packages, &[], cancel)?;
        let read = admitted
            .package(&library)
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_hyper_root(read, &occurrence)
            .map_err(|fault| ClauseRootRefusal::Package(format!("{fault:?}")))?;
        if root.occurrence() != &occurrence {
            return Err(ClauseRootRefusal::Package(
                "the I2 hyper root changed the checked occurrence".to_owned(),
            ));
        }
        Ok(Self { occurrence })
    }
}

/// `clause-root-unresolved` (FR-337 "Behavior", FR-338): the request's
/// package gives the clause no root, so the request is at fault and no key
/// context exists.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClauseRootRefusal {
    /// The package declares no clause equal to the request's.
    #[error("clause-root-unresolved: the package declares no clause {name}")]
    Undeclared {
        /// The request's clause name.
        name: String,
    },
    /// The I2 lookup does not find the clause's triple in the emitted
    /// package.
    #[error("clause-root-unresolved: {0}")]
    Unresolved(#[from] ClauseRootUnresolved),
    /// The package did not emit or did not read back, so it has no root to
    /// look up.
    #[error("clause-root-unresolved: the package did not emit or read: {0}")]
    Package(String),
    /// The handle stopped the emission.
    #[error("the emission was cancelled: {0:?}")]
    Cancelled(CancelCause),
}
