// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283, FR-522: the emission a clock-bearing trace is authenticated
//! against, built from a checked package the caller already holds.

use qsl_package::{
    emit_checked_with_temporal_selections_and_cancel, CheckedPackage, Emission, EmitRefusal,
    TemporalClockSelection,
};
use qsl_semantics::model::intake::PackageInput;
use quire_exact::{Cancel, CancelCause};
use quire_semantic_value::catalog::InternalFault;

use super::temporal::TemporalAuthentication;
use crate::spine::SpineLimits;

/// A checked package emitted with the caller's clock selections and strictly
/// read back: the `temporal_bindings` rows and the admitted package a
/// `Progressed` or `Timestamped` trace's binding is compared with.
pub struct PackageAuthentication {
    emission: Emission,
    admitted: quire_contract_model::CheckedPackageV2,
}

impl PackageAuthentication {
    /// What [`run_package_temporal`](super::run_package_temporal) and
    /// [`authenticate_package_trace`](super::authenticate_package_trace)
    /// authenticate a trace against.
    pub fn temporal(&self) -> TemporalAuthentication<'_> {
        TemporalAuthentication {
            clock_rows: self.emission.temporal_binding_rows(),
            admitted_package: &self.admitted,
        }
    }
}

/// Why no [`PackageAuthentication`] was built.
#[derive(Debug)]
pub enum AuthenticationFailure {
    /// The caller's handle stopped the emitter.
    Cancelled(CancelCause),
    /// Emission refused for want of the `TemporalClockSelection` of the
    /// temporal declaration at `declaration`.
    MissingSelection {
        /// The name of the temporal clause whose declaration has no
        /// selection; `None` for a temporal declaration that is not a
        /// clause of the package's graph, such as a `compensate` node.
        clause: Option<String>,
    },
    /// Emission refused for another reason: the selections do not fit the
    /// package.
    Refused(Box<EmitRefusal>),
    /// An internal failure: the emitted bytes do not read back.
    Fault(InternalFault),
}

/// Emit `package` with `selections` (`emit_checked_with_temporal_selections`)
/// and read the bytes back strictly, under `domain_packages` and the
/// dependency closure, as the clause run of a source does.
///
/// # Errors
///
/// [`AuthenticationFailure`] when emission refuses, is cancelled, or does not
/// read back.
pub fn authenticate_emission(
    package: &CheckedPackage,
    selections: &[TemporalClockSelection],
    domain_packages: &PackageInput,
    limits: &SpineLimits,
    cancel: &Cancel,
) -> Result<PackageAuthentication, AuthenticationFailure> {
    let emission = emit_checked_with_temporal_selections_and_cancel(
        package,
        selections,
        cancel,
        limits.package.emit(),
    )
    .map_err(|refusal| match refusal {
        EmitRefusal::Cancelled { cause } => AuthenticationFailure::Cancelled(cause),
        EmitRefusal::MissingTemporalClockSelection { source_ref, span } => {
            AuthenticationFailure::MissingSelection {
                clause: package
                    .graph()
                    .temporal_clauses()
                    .iter()
                    .find(|node| {
                        node.clause().span() == span
                            && super::temporal::clause_source_ref(package, node).as_ref()
                                == Some(&source_ref)
                    })
                    .map(|node| node.clause().name().to_owned()),
            }
        }
        other => AuthenticationFailure::Refused(Box::new(other)),
    })?;
    let fault = |invariant: &'static str| {
        AuthenticationFailure::Fault(InternalFault::new("FR-050", invariant))
    };
    let mut evidence = emission.evidence().clone();
    for (identity, document) in domain_packages {
        evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .map_err(|_| fault("domain-package-documents-do-not-conflict"))?;
    }
    qsl_package::supply_closure(
        package,
        domain_packages,
        limits.imports,
        limits.package.emit(),
        &mut evidence,
    )
    .map_err(|_| fault("dependency-closure-reads-back"))?;
    let admitted = match quire_contract_model::CheckedPackageV2::read(
        emission.package().bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        quire_contract_model::CheckedPackageV2ReadResult::Admitted(admitted) => *admitted,
        _ => return Err(fault("emitted-checked-package-strict-read-admits")),
    };
    Ok(PackageAuthentication { emission, admitted })
}
