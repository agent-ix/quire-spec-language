// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036/040/042: native source compilation, separate from wire-data admission.
//! Only completed source binding, native typing, authored definedness discharge
//! and supported family checks can construct the private emission authority.
mod channels;
mod context;
mod controls;
mod families;
mod layout;
mod metadata;
mod populations;
mod runtime;
mod temporal_v2;
mod types;
mod values;

use super::{wire, work::Work, AdmittedModel, Candidate, Limits, Report, SuppliedDependency};
use crate::checking::composed::proofs::ProofReport;
use crate::formal_source::FormalSource;

pub use temporal_v2::{admit_v2, AdmissionV2, TemporalSelection};

/// Explicit external namespace selection for an actual authored formal source.
#[derive(Clone, Copy, Debug)]
pub struct SourceSelection<'a> {
    /// Exact immutable source reference; native identity is retained separately.
    pub artifact: &'a wire::ArtifactRef,
    /// Original bytes and authored formal owner used by definedness discharge.
    pub source: &'a FormalSource,
    /// External namespace for that formal owner's actual IR revision value.
    pub revision_namespace: &'a str,
}
/// Complete independently supplied immutable inputs. No discovery or observations.
#[derive(Clone, Copy, Debug)]
pub struct Selections<'a> {
    /// Exact selected compiler/consumer contract dependency.
    pub contract: &'a wire::ArtifactRef,
    /// Exact accepted semantic baseline dependency.
    pub baseline: &'a wire::ArtifactRef,
    /// Explicit implementation, revision and immutable binary selection.
    pub producer: &'a wire::Producer,
    /// Complete original source inventory; authored owners must match discharge.
    pub sources: &'a [SourceSelection<'a>],
    /// Complete exact-byte dependency inventory, including registered rules.
    pub dependencies: &'a [SuppliedDependency<'a>],
    /// Actual admitted models already consumed by the binding report.
    pub models: &'a [AdmittedModel<'a>],
    /// Namespace for registered semantic revisions, separate from source artifacts.
    pub definition_revision_namespace: &'a str,
    /// Namespace for the exact authored requirement's numeric IR revision.
    pub requirement_revision_namespace: &'a str,
}
/// Completed source-derived family admission; callers cannot grant it to wire data.
#[derive(Debug)]
pub struct FamilyAdmission {
    package: wire::Package,
}
impl FamilyAdmission {
    /// Read-only graph produced from the original typed source, never a proof witness.
    pub fn package(&self) -> &wire::Package {
        &self.package
    }
}
/// Bytes emitted from completed native family admission, distinct from a candidate.
#[derive(Debug)]
pub struct EmittedPackage {
    candidate: Candidate,
}
impl EmittedPackage {
    /// Complete canonical native-emitted bytes.
    pub fn bytes(&self) -> &[u8] {
        self.candidate.bytes()
    }
    /// Raw-byte digest of the complete native-emitted payload.
    pub fn digest(&self) -> crate::ByteDigest {
        self.candidate.digest()
    }
}
/// Check actual source/type/proof owners and lower only supported complete families.
pub fn admit(
    proofs: &ProofReport<'_, '_, '_>,
    selections: &Selections<'_>,
    limits: Limits,
) -> Report<FamilyAdmission> {
    let mut work = Work::new(limits);
    let result = (|| {
        let package = metadata::lower(proofs, selections, &mut work)?;
        super::validate::package(&package, &mut work)?;
        Ok(FamilyAdmission { package })
    })();
    super::report(work, result)
}
/// Canonically encode the private completed native graph with independent limits.
///
/// A freely decoded wire package cannot supply completed native admission.
/// ```compile_fail,E0308
/// use quire_spec_language::protocol_artifact::{native, wire, Limits};
/// fn cannot_emit_wire(package: &wire::Package) {
///     let _ = native::emit(package, Limits::default());
/// }
/// ```
pub fn emit(admitted: &FamilyAdmission, limits: Limits) -> Report<EmittedPackage> {
    let report = super::encode_candidate(&admitted.package, limits);
    Report {
        result: report.result.map(|candidate| EmittedPackage { candidate }),
        limits: report.limits,
        usage: report.usage,
        locus: report.locus,
    }
}
