// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-521 row 5: the public temporal surface kept until `quire-protocol` and
//! Contract IR repoint.
//!
//! The v1 evaluator is retired; its successors are specified by FR-327 to
//! FR-329 and FR-501 to FR-527 and implemented in `qsl-eval`. What remains is
//! [`evaluate_v2`] and its [`Report`], the [`Trace`] inputs, the
//! [`Closure`]/[`Completeness`]/[`Execution`] dimensions, and
//! [`mapping_support_v2`] with its [`Classification`]. The contract is in
//! [native-temporal-evaluation.md](https://github.com/agent-ix/quire-spec-language/blob/main/docs/native-temporal-evaluation.md).
//!
//! There is no public entry point accepting a freely constructed wire package.
//! Admission through [`crate::protocol_artifact::read`] is the constructor-private
//! evidence that the declaration was compiled rather than asserted.

mod mapping;
mod owner;
mod owner_codec;
mod profile;
mod result;
mod trace;
mod valuation;
pub use owner::{
    evaluate_v2, CheckedSubject, NativeReportError, ObservationRef, Report, ReportOutcome,
    ReportPredecessorLimits, ReportPredecessorResource, ReportPredecessorUsage, ReportSubject,
    ReportSupport,
};
pub use owner_codec::{BindingDimension, ReportRefusal};

use crate::protocol_artifact::{v2, wire as w};

use mapping::{classify, AuthenticatedSelection, Operators, Retained};
pub use mapping::{Classification, Support, Target, Unmatched, OUTSTANDING_PREMISES};
pub use profile::{Profile, EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW};
pub use result::{Closure, Completeness, Dimension, Error, Execution, Refusal, Subject};
pub use trace::{
    CaptureInput, ClockBinding, Eviction, Evidence, OrderKey, Position, Trace, Trigger,
};
pub use valuation::{
    MissingValuation, ValuationCause, ValuationEvidence, ValuationInstance, ValuationRow,
};

/// Classify mapping support from an authenticated version-2 definition/profile.
///
/// The retained selection is resolved through the declaration's admitted
/// temporal binding, not through the declaration alone: a declaration with no
/// binding is refused, and so is one whose binding selects a definition other
/// than the declaration's own profile. The strict reader already refuses such a
/// package, so that second refusal is a re-check that keeps the evidence from
/// ever naming a definition the classifier did not use. The classification
/// retains the declaration and authenticated definition identity and revision.
/// It takes no trace, so it
/// authenticates no clock parameter map and retains none.
pub fn mapping_support_v2(
    package: &v2::AdmittedPackage,
    declaration: usize,
    surrounding_execution: Closure,
) -> Result<Classification, Error> {
    let subject = declaration_subject(declaration);
    let selected = select(package.inherited(), declaration, subject)?;
    let (_, authenticated) = authenticated_selection(package, declaration, &selected, subject)?;
    Ok(classify_selected(
        &selected,
        subject,
        surrounding_execution,
        Some(authenticated),
    ))
}

fn declaration_subject(declaration: usize) -> Subject {
    Subject {
        declaration,
        ..Subject::default()
    }
}

fn classify_selected(
    selected: &Selected<'_>,
    subject: Subject,
    surrounding_execution: Closure,
    authenticated: Option<AuthenticatedSelection>,
) -> Classification {
    Classification::new(
        classify(
            selected.profile,
            Operators::of(&selected.declaration.temporal, selected.root()),
            surrounding_execution,
        ),
        Retained {
            subject,
            name: selected.declaration.name.clone(),
            profile: selected.profile,
            activation: selected.activation.clone(),
        },
        authenticated,
    )
}

/// Resolve the definition one declaration's admitted version-2 binding selects,
/// together with the selection evidence both strict version-2 entry points
/// retain.
///
/// The strict reader admits exactly one binding per temporal declaration, in
/// declaration order, whose definition is the declaration's own profile. Every
/// departure from that is refused here as a reference refusal rather than
/// defaulted, so no entry point retains evidence for a definition it did not
/// use.
fn authenticated_selection<'a>(
    package: &'a v2::AdmittedPackage,
    declaration: usize,
    selected: &Selected<'_>,
    subject: Subject,
) -> Result<(&'a v2::wire::TemporalBinding, AuthenticatedSelection), Error> {
    let bindings = &package.package().temporal_bindings;
    let binding = u32::try_from(declaration)
        .ok()
        .and_then(|declaration| {
            bindings
                .binary_search_by_key(&declaration, |binding| binding.declaration)
                .ok()
        })
        .and_then(|index| bindings.get(index))
        .ok_or(Refusal::Reference { subject })?;
    // Admission invariant, re-checked: the binding selects the declaration's
    // own definition.
    if binding.definition != selected.declaration.profile {
        return Err(Refusal::Reference { subject }.into());
    }
    let inherited = package.inherited();
    let definition = usize::try_from(binding.definition)
        .ok()
        .and_then(|index| inherited.definitions.get(index))
        .ok_or(Refusal::Reference { subject })?;
    usize::try_from(definition.artifact)
        .ok()
        .and_then(|index| inherited.dependencies.get(index))
        .ok_or(Refusal::Reference { subject })?;
    Ok((
        binding,
        AuthenticatedSelection::new(declaration, definition),
    ))
}

/// The admitted declaration, its temporal body and its selected profile.
struct Selected<'a> {
    declaration: &'a w::Declaration,
    body: &'a w::Body,
    activation: &'a w::Activation,
    profile: Profile,
}

impl Selected<'_> {
    /// The arena index of the declaration's temporal root; an index too large
    /// for the platform reaches nothing.
    fn root(&self) -> usize {
        match self.body {
            w::Body::Temporal { root, .. } => usize::try_from(root.index).unwrap_or(usize::MAX),
            _ => usize::MAX,
        }
    }
}

fn select(
    package: &w::Package,
    declaration: usize,
    subject: Subject,
) -> Result<Selected<'_>, Error> {
    let entry = package
        .declarations
        .get(declaration)
        .ok_or(Refusal::Reference { subject })?;
    let w::Body::Temporal { activation, .. } = &entry.body else {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject,
        }
        .into());
    };
    let definition = package
        .definitions
        .get(usize::try_from(entry.profile).unwrap_or(usize::MAX))
        .ok_or(Refusal::Reference { subject })?;
    let profile = Profile::from_identity(&definition.identity).ok_or(Refusal::Binding {
        dimension: Dimension::Profile,
        subject,
    })?;
    Ok(Selected {
        declaration: entry,
        body: &entry.body,
        activation,
        profile,
    })
}
