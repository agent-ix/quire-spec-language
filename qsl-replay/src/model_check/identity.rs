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

use std::collections::BTreeMap;

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::{
    emit_checked_with_cancel, read_import_view, resolve_clause_root, AdmittedPackages,
    CheckedPackage, ClauseRootUnresolved, EmitRefusal, V2ReadLimits,
};
use qsl_semantics::check::{CheckedTemporalClause, TemporalClauseNode};
use qsl_semantics::library::LibraryName;
use quire_exact::{Cancel, CancelCause};

/// The `claim` occurrence of a checked clause `node`: its node id and claim
/// origin (ADR-013 O-07).
pub fn node_occurrence(node: &TemporalClauseNode) -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest(*node.identity().as_bytes()),
        node.claim().clone(),
    )
}

/// A clause's identity in its package: the checked occurrence and the node of
/// its `over` parameter, `None` for a clause with no `over`.
///
/// Only [`ClauseIdentity::resolve`] makes one: a request cannot name it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseIdentity {
    occurrence: OccurrenceKey,
    over: Option<WireNodeId>,
}

impl ClauseIdentity {
    /// The clause's O-07 occurrence: node, role and ordinal.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `over` parameter's checked node.
    pub fn over(&self) -> Option<WireNodeId> {
        self.over
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
        domain_packages: &BTreeMap<[u8; 32], Vec<u8>>,
        clause: &CheckedTemporalClause,
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
        let emission =
            emit_checked_with_cancel(package, cancel).map_err(|refusal| match refusal {
                EmitRefusal::Cancelled { cause } => ClauseRootRefusal::Cancelled(cause),
                other => ClauseRootRefusal::Package(other.to_string()),
            })?;
        let library = LibraryName::new("package")
            .map_err(|refusal| ClauseRootRefusal::Package(format!("{refusal:?}")))?;
        let mut admitted = AdmittedPackages::default();
        read_import_view(
            package,
            &emission,
            library,
            domain_packages,
            &mut admitted,
            V2ReadLimits::default(),
        )
        .map_err(|refusal| ClauseRootRefusal::Package(format!("{refusal:?}")))?;
        let read = admitted
            .package(&emission.package().package_id())
            .ok_or_else(|| ClauseRootRefusal::Package("the read recorded no package".to_owned()))?;
        let root = resolve_clause_root(read, &triple)?;
        Ok(Self {
            occurrence: root.occurrence().clone(),
            over: root.over(),
        })
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
