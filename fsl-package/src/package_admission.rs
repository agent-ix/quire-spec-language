// SPDX-License-Identifier: AGPL-3.0-or-later
//! The strict `filament.checked-package/v2` admission a request that selects a
//! package by its bytes uses (FR-027, FR-850): the bytes are read by IR's
//! reader with the request's supplied domain package documents as evidence,
//! with no fallback to source compilation.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use filament_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageRefusal, CheckedPackageRefusalCode, CheckedPackageV2,
};
use fsl_foundation::diagnostic::{Code, StageFailure};
use fsl_semantics::value::DefinitionRole;

use crate::checked_v2::{ImportViewRefusal, V2ReadLimits, V2ReadRefusal};

/// A package the strict reader refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageAdmissionRefusal {
    /// Original cumulative read-budget denial, when admission exhausted it.
    pub limit: Option<fsl_foundation::diagnostic::LimitExceeded>,
    /// The catalogued code: `missing_import` where no document is supplied
    /// under a selected domain package identity, `resource_exhausted` where
    /// a read ceiling was reached, `stage_limit_exceeded` for a shared-budget
    /// denial, and `invalid_package` for every other refusal.
    pub code: Code,
    /// The reader's typed cause tag, such as `missing-selection`.
    pub cause: Option<String>,
    /// The RFC 6901 pointer the reader names, absent for a byte-stream defect.
    pub pointer: Option<String>,
    /// The domain or library identity a `missing-selection` refusal names.
    pub identity: Option<String>,
}

/// Read `bytes` as a `filament.checked-package/v2` artifact, with `documents`
/// (domain package documents by their declared identity) as the evidence its
/// model selections are read against.
///
/// # Errors
///
/// [`PackageAdmissionRefusal`] for bytes the reader refuses, an unsupplied
/// selected domain package, a supplied document its selection does not admit,
/// or a read ceiling reached.
pub fn admit_checked_package(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    limits: V2ReadLimits,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    admit_checked_package_with_dependencies(bytes, documents, &BTreeMap::new(), limits)
}

/// Admit a package with authenticated direct library artifacts. The strict
/// reader verifies the supplied identities and retains their shared closure.
///
/// # Errors
/// Returns the original strict reader refusal or a conflicting evidence refusal.
pub fn admit_checked_package_with_dependencies(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    dependencies: &BTreeMap<String, Arc<CheckedPackageV2>>,
    limits: V2ReadLimits,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    let evidence = checked_package_evidence(documents, dependencies)?;
    match read_checked_package(bytes, limits.for_ir(), &evidence) {
        CheckedPackageDispatchResult::AdmittedV2(package) => Ok(*package),
        CheckedPackageDispatchResult::Refused(refusal) => {
            Err(of_reader_refusal(&refusal).naming_identity_in(Some(bytes)))
        }
        CheckedPackageDispatchResult::Incomplete(incomplete) => Err(PackageAdmissionRefusal {
            limit: None,
            code: Code::ResourceExhausted,
            cause: None,
            pointer: incomplete.path.map(|path| path.as_str().to_owned()),
            identity: None,
        }),
        CheckedPackageDispatchResult::CatalogUnavailable(_) => Err(PackageAdmissionRefusal {
            limit: None,
            code: Code::InvalidPackage,
            cause: None,
            pointer: None,
            identity: None,
        }),
    }
}

/// Strict admission consuming the invocation's shared work allowance.
///
/// # Errors
/// Preserves strict-reader refusals and cumulative typed budget denials.
pub fn admit_checked_package_with_budget(
    bytes: &[u8],
    documents: &BTreeMap<String, Vec<u8>>,
    dependencies: &BTreeMap<String, Arc<CheckedPackageV2>>,
    limits: V2ReadLimits,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    let evidence = checked_package_evidence(documents, dependencies)?;
    admit_checked_package_with_evidence(bytes, &evidence, limits, budget)
}

/// Read against invocation-owned evidence without copying domain documents or
/// dependency graphs at each package boundary. The strict reader retains only
/// the dependencies its lock selects.
///
/// # Errors
/// Preserves strict-reader refusals and cumulative typed budget denials.
pub fn admit_checked_package_with_evidence(
    bytes: &[u8],
    evidence: &CheckedPackageEvidence,
    limits: V2ReadLimits,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<CheckedPackageV2, PackageAdmissionRefusal> {
    use filament_contract_model::CheckedPackageBudgetedDispatchResult as Read;
    let denial = |limit, actual, pointer| PackageAdmissionRefusal {
        limit: Some(fsl_foundation::diagnostic::LimitExceeded::new(
            fsl_foundation::Setting::I2WorkUnits,
            limit,
            actual,
        )),
        code: Code::StageLimitExceeded,
        cause: Some("insufficient-next-charge".to_owned()),
        pointer,
        identity: None,
    };
    match filament_contract_model::read_checked_package_with_budget(
        bytes,
        limits.for_ir(),
        evidence,
        budget,
    ) {
        Read::AdmittedV2(package) => Ok(*package),
        Read::Refused(refusal) => Err(of_reader_refusal(&refusal).naming_identity_in(Some(bytes))),
        Read::BudgetIncomplete(incomplete) => Err(denial(
            incomplete.limit,
            u128::from(incomplete.consumed),
            incomplete.path.map(|path| path.as_str().to_owned()),
        )),
        Read::Exhausted(exhausted) => Err(denial(
            exhausted.configured_bound,
            u128::from(budget.consumed().work).saturating_add(exhausted.denied.amount),
            None,
        )),
        Read::Incomplete(incomplete) => Err(PackageAdmissionRefusal {
            limit: None,
            code: Code::ResourceExhausted,
            cause: None,
            pointer: incomplete.path.map(|path| path.as_str().to_owned()),
            identity: None,
        }),
        Read::CatalogUnavailable(_) => Err(PackageAdmissionRefusal {
            limit: None,
            code: Code::InvalidPackage,
            cause: None,
            pointer: None,
            identity: None,
        }),
    }
}

/// Own the invocation's domain documents once and supply authenticated library
/// values by shared ownership. Additional admitted libraries can be inserted
/// through IR's existing evidence API before subsequent reads.
///
/// # Errors
/// Refuses conflicting documents or library identities through IR's insertion API.
pub fn checked_package_evidence(
    documents: &BTreeMap<String, Vec<u8>>,
    dependencies: &BTreeMap<String, Arc<CheckedPackageV2>>,
) -> Result<CheckedPackageEvidence, PackageAdmissionRefusal> {
    let mut evidence = CheckedPackageEvidence::new();
    // The one feature FSL emits and so the one it reads: a package that
    // requires another is refused by the reader.
    evidence.support_feature(DefinitionRole::Root.identity());
    for (identity, document) in documents {
        if evidence
            .insert_domain_package_document(identity.as_str(), document.as_slice())
            .is_err()
        {
            return Err(PackageAdmissionRefusal {
                limit: None,
                code: Code::InvalidPackage,
                cause: Some("conflicting-definition".to_owned()),
                pointer: None,
                identity: Some(identity.clone()),
            });
        }
    }
    for (identity, dependency) in dependencies {
        evidence
            .insert_dependency_package(identity.as_str(), Arc::clone(dependency))
            .map_err(|_| PackageAdmissionRefusal {
                limit: None,
                code: Code::InvalidPackage,
                cause: Some("conflicting-definition".to_owned()),
                pointer: None,
                identity: Some(identity.clone()),
            })?;
    }
    Ok(evidence)
}

/// The refusal IR's reader raised, with its pointer and typed cause. The
/// identity at fault is the caller's to read from the bytes.
fn of_reader_refusal(refusal: &CheckedPackageRefusal) -> PackageAdmissionRefusal {
    let pointer = refusal
        .loci
        .first()
        .and_then(|locus| locus.path.as_ref())
        .map(|path| path.as_str().to_owned());
    let cause = refusal.cause.as_ref().and_then(|cause| {
        serde_json::to_value(cause)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
    });
    // FR-940 "Claim names": a name shared by two `claim_names` rows is the one
    // ambiguous declaration, whichever reader found it. QSL-2545: the
    // contract-ir reader still reports `invalid_package` here; delete this
    // remap once the reader returns `ambiguous_declaration` itself.
    let ambiguous_claim = cause.as_deref() == Some("ambiguous-name")
        && pointer
            .as_deref()
            .is_some_and(|pointer| pointer.starts_with("/claim_names/"));
    let code = if refusal.code == CheckedPackageRefusalCode::MissingImport {
        Code::MissingImport
    } else if ambiguous_claim {
        Code::AmbiguousDeclaration
    } else {
        Code::InvalidPackage
    };
    PackageAdmissionRefusal {
        limit: None,
        code,
        cause,
        pointer,
        identity: None,
    }
}

impl PackageAdmissionRefusal {
    /// The refusal spelled `code/cause for identity at pointer`, each part
    /// when present.
    #[must_use]
    pub fn label(&self) -> String {
        let mut label = self.code.as_str().to_owned();
        if let Some(cause) = &self.cause {
            label.push('/');
            label.push_str(cause);
        }
        if let Some(identity) = &self.identity {
            label.push_str(" for ");
            label.push_str(identity);
        }
        if let Some(pointer) = &self.pointer {
            label.push_str(" at ");
            label.push_str(pointer);
        }
        label
    }

    /// The cause as a catalog spelling: the reader's own cause, else the
    /// catalog default of the code (`insufficient-next-charge` for a reached
    /// ceiling, `missing-selection` for a missing import, `invalid-value`
    /// otherwise). The reader's causes are a closed set, so interning them
    /// holds a bounded number of strings for the process.
    #[must_use]
    pub fn catalog_cause(&self) -> &'static str {
        use std::sync::{Mutex, OnceLock};
        static CAUSES: OnceLock<Mutex<BTreeSet<&'static str>>> = OnceLock::new();
        match &self.cause {
            Some(cause) => {
                let mut known = CAUSES
                    .get_or_init(Mutex::default)
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(known) = known.get(cause.as_str()) {
                    return known;
                }
                let leaked: &'static str = Box::leak(cause.clone().into_boxed_str());
                known.insert(leaked);
                leaked
            }
            None => match self.code {
                Code::ResourceExhausted => "insufficient-next-charge",
                Code::MissingImport => "missing-selection",
                _ => "invalid-value",
            },
        }
    }

    /// The strict reader's refusal inside a failed evaluation read of a
    /// package the run compiled, with the reader's code, cause and pointer;
    /// `bytes` are the package bytes the read refused, from which a missing
    /// import names its identity. `None` for a failure that is no admission
    /// refusal: an emission that did not complete, a broken invariant, a
    /// cancellation.
    #[must_use]
    pub fn of_import_view(refusal: &ImportViewRefusal, bytes: Option<&[u8]>) -> Option<Self> {
        let ImportViewRefusal::Read { refusal, .. } = refusal else {
            return None;
        };
        let reader = |refusal: &V2ReadRefusal| match refusal {
            V2ReadRefusal::Envelope { refusal, .. } => Some(of_reader_refusal(refusal)),
            V2ReadRefusal::Fault(_) => None,
            other => Some(Self {
                limit: None,
                code: other.code(),
                cause: None,
                pointer: None,
                identity: None,
            }),
        };
        let exhausted = || Self {
            limit: None,
            code: Code::ResourceExhausted,
            cause: None,
            pointer: None,
            identity: None,
        };
        let found = match refusal.as_ref() {
            StageFailure::Refused(refusal) => reader(refusal),
            StageFailure::RefusedWithPayload(refusal, _) => reader(refusal),
            StageFailure::Limit(_) | StageFailure::LimitWithPayload(_, _) => Some(exhausted()),
            StageFailure::Cancelled(_) | StageFailure::Fault(_) => None,
        };
        found.map(|found| found.naming_identity_in(bytes))
    }

    /// This refusal, with the identity a missing import names read from the
    /// package `bytes` at its pointer.
    fn naming_identity_in(mut self, bytes: Option<&[u8]>) -> Self {
        if self.code == Code::MissingImport {
            self.identity = self
                .pointer
                .as_deref()
                .zip(bytes)
                .and_then(|(pointer, bytes)| identity_at(bytes, pointer));
        }
        self
    }
}

/// The domain package identity the value at `pointer` of the package JSON
/// names: the value itself when it is a string, else its `identity` member.
fn identity_at(bytes: &[u8], pointer: &str) -> Option<String> {
    let document: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let value = document.pointer(pointer)?;
    value
        .as_str()
        .or_else(|| value.get("identity").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}
