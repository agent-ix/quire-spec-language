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
    /// Emission refused for want of the `TemporalClockSelection` of a
    /// clock-bearing temporal declaration.
    MissingSelection {
        /// The name of the temporal clause whose declaration has no
        /// selection; `None` for a temporal declaration that is not a
        /// clause of the package's graph, such as a `compensate` node.
        clause: Option<String>,
    },
    /// The caller's selections do not fit the checked package: a selection
    /// for no declaration, a repeated selection, a configuration of another
    /// profile or an invalid clock value.
    SelectionsDoNotFit,
    /// The caller's domain package documents do not admit the emitted
    /// package: they conflict with the emission's own, the dependency
    /// closure does not read back, or the strict read refuses.
    PackagesDoNotAdmit(PackagesStage),
    /// An internal failure: emission refused for a reason no caller input
    /// causes.
    Fault(InternalFault),
}

/// Where the caller's domain package documents stopped the authentication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackagesStage {
    /// A document conflicts with the one the emission evidence holds.
    Conflict,
    /// The dependency closure does not read back under the documents.
    Closure,
    /// The strict read of the emitted package refuses under the documents.
    StrictRead,
}

/// Emit `package` with `selections` (`emit_checked_with_temporal_selections`)
/// and read the bytes back strictly, under `domain_packages` and the
/// dependency closure, as the clause run of a source does.
///
/// `limits` is the published default [`SpineLimits`] of the emit and import
/// ceilings: FR-283's request takes no limit for them, so the caller passes
/// the default.
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
        EmitRefusal::SurplusTemporalClockSelection { .. }
        | EmitRefusal::DuplicateTemporalClockSelection { .. }
        | EmitRefusal::TemporalClockProfileMismatch { .. }
        | EmitRefusal::InvalidTemporalClockValue { .. } => {
            AuthenticationFailure::SelectionsDoNotFit
        }
        _ => AuthenticationFailure::Fault(InternalFault::new(
            "FR-050",
            "emission-of-a-checked-package-with-selections",
        )),
    })?;
    let mut evidence = emission.evidence().clone();
    for (identity, document) in domain_packages {
        evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .map_err(|_| AuthenticationFailure::PackagesDoNotAdmit(PackagesStage::Conflict))?;
    }
    qsl_package::supply_closure(
        package,
        domain_packages,
        limits.imports,
        limits.package.emit(),
        &mut evidence,
    )
    .map_err(|_| AuthenticationFailure::PackagesDoNotAdmit(PackagesStage::Closure))?;
    let admitted = match quire_contract_model::CheckedPackageV2::read(
        emission.package().bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        quire_contract_model::CheckedPackageV2ReadResult::Admitted(admitted) => *admitted,
        _ => {
            return Err(AuthenticationFailure::PackagesDoNotAdmit(
                PackagesStage::StrictRead,
            ))
        }
    };
    Ok(PackageAuthentication { emission, admitted })
}
