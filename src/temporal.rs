// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043/FR-044/FR-045: bounded evaluation of an emitted native temporal body.
//!
//! The public contract is in
//! [native-temporal-evaluation.md](https://github.com/agent-ix/quire-spec-language/blob/main/docs/native-temporal-evaluation.md).
//!
//! Agent E owns native temporal meaning; the rules implemented here restate the
//! owned shared rules for verification, and the shared rule governs on
//! divergence. Agent F owns observation transport and completeness authority;
//! the trace is a caller-supplied input whose assertions are retained as
//! premises rather than verified. Agent B owns protocol results; nothing here
//! produces one.
//!
//! There is no public entry point accepting a freely constructed wire package.
//! Admission through [`crate::protocol_artifact::read`] is the constructor-private
//! evidence that the declaration was compiled rather than asserted.

mod activation;
mod budget;
mod formula;
mod mapping;
mod profile;
mod progress;
mod result;
mod trace;

use std::collections::BTreeMap;

use crate::protocol_artifact::{v2, wire as w, AdmittedPackage};

use activation::{Outcome, Shape};
use formula::{Evaluator, Tri};

pub use budget::{Dimension as LimitDimension, Exhaustion, Limits, Usage, ACCOUNTING_VERSION};
pub use mapping::{
    classify, AuthenticatedSelection, Classification, Operators, Retained, Support, Target,
    Unmatched, OUTSTANDING_PREMISES, SUPPORT_TABLE,
};
pub use profile::{Profile, EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW};
pub use progress::{AuthenticatedBinding, Binding, Ledger, Progress};
pub use result::{
    Activation, Assessment, Basis, Capture, Closure, Completeness, Dimension, Error, Execution,
    Incomplete, Obligation, Premises, Refusal, Report, Subject, Support as DecisionSupport, Truth,
};
pub use trace::{
    CaptureInput, ClockBinding, Eviction, Evidence, OrderKey, Position, Trace, Trigger,
};

/// The `clock:` prefix the emitter gives a temporal clock binding requirement.
const CLOCK_PREFIX: &str = "clock:";

/// Evaluate one admitted temporal declaration against one caller-supplied trace.
///
/// The outer error is a whole-declaration stop: a binding refusal, an
/// order-sensitivity refusal, a contradiction or an exhausted ceiling. Activation
/// failing for one instance leaves every sibling inspectable as its own
/// [`Obligation`].
pub fn evaluate(
    package: &AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
) -> Report {
    let mut work = budget::Work::new(limits);
    let result = run(
        package.package(),
        declaration,
        trace,
        trace.watermark,
        &mut work,
    );
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated: None,
    }
}

/// Evaluate against a trace whose progress is retained across evaluations under
/// its own binding.
///
/// The trace's watermark, completeness and decision-scope closure are recorded
/// in `ledger` under this declaration, clock binding name and asserted profile
/// identity. A watermark regressing under that binding, or a completeness
/// assertion revised in conflict under it, is a typed contradiction refusal: the
/// retained progress is not rolled back, the retained closure is not restamped
/// and the earlier result is neither reused nor rewritten. Progress asserted
/// under any other declaration, clock or profile is a different binding and
/// settles nothing here.
pub fn evaluate_with_progress(
    package: &AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
    ledger: &mut Ledger,
) -> Report {
    let mut work = budget::Work::new(limits);
    let result = match ledger.record(Binding::of(declaration, trace), Progress::of(trace)) {
        Ok(retained) => run(
            package.package(),
            declaration,
            trace,
            retained.watermark,
            &mut work,
        ),
        Err(refusal) => Err(refusal.into()),
    };
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated: None,
    }
}

/// Classify the native-to-TL mapping support of one admitted temporal
/// declaration. This emits no TL artifact and establishes no correspondence.
///
/// A version-1 package authenticates no temporal definition selection, so the
/// classification retains none.
pub fn mapping_support(
    package: &AdmittedPackage,
    declaration: usize,
    surrounding_execution: Closure,
) -> Result<Classification, Error> {
    let subject = declaration_subject(declaration);
    let selected = select(package.package(), declaration, subject)?;
    Ok(classify_selected(
        &selected,
        subject,
        surrounding_execution,
        None,
    ))
}

/// Evaluate one strict version-2 declaration after authenticating its exact clock.
///
/// Authentication gates the run: a trace whose clock identity does not match the
/// admitted binding is refused before any evaluation work. The authenticated
/// binding is retained on the report, so a version-2 result is distinguishable
/// from an unauthenticated version-1 result without re-deriving it. A report
/// refused by authentication itself retains none, because nothing was
/// authenticated.
pub fn evaluate_v2(
    package: &v2::AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
) -> Report {
    let mut work = budget::Work::new(limits);
    let mut authenticated = None;
    let result = authenticate_v2(package, declaration, trace, &mut work).and_then(|binding| {
        authenticated = Some(binding);
        run(
            package.inherited(),
            declaration,
            trace,
            trace.watermark,
            &mut work,
        )
    });
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated,
    }
}

/// Evaluate version-2 input while retaining progress under its authenticated identity.
pub fn evaluate_with_progress_v2(
    package: &v2::AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
    ledger: &mut Ledger,
) -> Report {
    let mut work = budget::Work::new(limits);
    let mut authenticated = None;
    let result = authenticate_v2(package, declaration, trace, &mut work).and_then(|binding| {
        authenticated = Some(binding.clone());
        ledger
            .record_authenticated(binding, Progress::of(trace))
            .map_err(Error::from)
            .and_then(|retained| {
                run(
                    package.inherited(),
                    declaration,
                    trace,
                    retained.watermark,
                    &mut work,
                )
            })
    });
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated,
    }
}

/// Classify mapping support from an authenticated version-2 definition/profile.
///
/// The retained selection is resolved through the declaration's admitted
/// temporal binding, not through the declaration alone: a declaration with no
/// binding is refused, and so is one whose binding selects a definition other
/// than the declaration's own profile. The strict reader already refuses such a
/// package, so that second refusal is a re-check that keeps the evidence from
/// ever naming a definition the classifier did not use. The classification
/// retains the authenticated package digest, definition identity, namespaced
/// revision and definition artifact digest. It takes no trace, so it
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

/// The subject naming one whole declaration.
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
            Operators::of(&selected.declaration.temporal),
            surrounding_execution,
        ),
        Retained {
            subject,
            name: selected.declaration.name.clone(),
            profile: selected.profile,
            profile_revision: selected.revision.clone(),
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
    let artifact = usize::try_from(definition.artifact)
        .ok()
        .and_then(|index| inherited.dependencies.get(index))
        .ok_or(Refusal::Reference { subject })?
        .artifact
        .digest;
    Ok((
        binding,
        AuthenticatedSelection::new(package.digest(), declaration, definition, artifact),
    ))
}

/// The admitted declaration, its temporal body and its selected profile.
struct Selected<'a> {
    declaration: &'a w::Declaration,
    body: &'a w::Body,
    activation: &'a w::Activation,
    profile: Profile,
    revision: String,
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
        revision: definition.revision.value.clone(),
    })
}

fn run(
    package: &w::Package,
    declaration: usize,
    trace: &Trace,
    watermark: i64,
    work: &mut budget::Work,
) -> Result<Vec<Obligation>, Error> {
    let subject = declaration_subject(declaration);
    work.subject = subject;
    let selected = select(package, declaration, subject)?;
    let w::Body::Temporal {
        clock,
        activation,
        captures,
        root,
        ..
    } = selected.body
    else {
        return Err(Refusal::Reference { subject }.into());
    };

    // The trace asserts a profile and clock; neither is inferred, defaulted or
    // nearest-matched from it.
    if trace.clock.profile_identity != selected.profile.identity() {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject,
        }
        .into());
    }
    if trace.clock.profile_revision != selected.revision {
        return Err(Refusal::Binding {
            dimension: Dimension::ProfileRevision,
            subject,
        }
        .into());
    }
    let binding = selected
        .declaration
        .bindings
        .get(usize::try_from(*clock).unwrap_or(usize::MAX))
        .ok_or(Refusal::Reference { subject })?;
    if binding.name.strip_prefix(CLOCK_PREFIX) != Some(trace.clock.name.as_str()) {
        return Err(Refusal::Binding {
            dimension: Dimension::Clock,
            subject,
        }
        .into());
    }

    let shape = Shape {
        origin: matches!(activation, w::Activation::Origin { .. }),
        guarded: matches!(activation, w::Activation::Each { guard, .. } if guard.0.is_some()),
        captures: captures.len(),
    };
    let premises = Premises {
        profile: selected.profile,
        profile_revision: selected.revision.clone(),
        clock: trace.clock.name.clone(),
        clock_parameters: trace.clock.parameters.clone(),
        watermark,
        decision_scope: trace.decision_scope,
        surrounding_execution: trace.surrounding_execution,
        execution: trace.execution,
        completeness: trace.completeness,
        authoritative_origin: trace.authoritative_origin,
        limits: work.limits,
    };

    let root = usize::try_from(root.index).map_err(|_| Refusal::Reference { subject })?;
    let ordered = trace.ordered();

    match activation::resolve(shape, trace, declaration, work)? {
        Outcome::Disposition(disposition) => Ok(vec![Obligation::Assessed(Box::new(Assessment {
            subject,
            instance: None,
            receipts: Vec::new(),
            captures: Vec::new(),
            capture_evaluations: 0,
            activation: disposition,
            truth: None,
            basis: None,
            incomplete: None,
            support: Vec::new(),
            premises,
        }))]),
        Outcome::Instances(instances) => {
            let mut assessed = Vec::with_capacity(instances.len());
            for (ordinal, instance) in instances.into_iter().enumerate() {
                let subject = Subject {
                    instance: ordinal,
                    ..subject
                };
                work.subject = subject;
                let instance = match instance {
                    Ok(instance) => instance,
                    Err(failed) => {
                        assessed.push(Obligation::Unactivated {
                            subject,
                            instance: failed.identity,
                            error: failed.error,
                        });
                        continue;
                    }
                };
                let mut evaluator = Evaluator {
                    profile: selected.profile,
                    trace,
                    ordered: ordered.clone(),
                    nodes: &selected.declaration.temporal,
                    declaration,
                    instance: ordinal,
                    watermark,
                    support: Vec::new(),
                };
                let outcome = evaluator.root(root, work);
                work.charge(LimitDimension::Retention, evaluator.support.len())?;
                let (truth, basis, incomplete) = match outcome {
                    Ok(value) => {
                        let (truth, basis) = settle(value, trace);
                        (Some(truth), Some(basis), None)
                    }
                    // A required fact inside the decision support is missing, so
                    // the truth is unavailable. It is not false and not pending.
                    Err(Error::Incomplete(incomplete)) => {
                        (None, Some(Basis::Unavailable), Some(incomplete))
                    }
                    Err(error) => return Err(error),
                };
                assessed.push(Obligation::Assessed(Box::new(Assessment {
                    subject,
                    instance: Some(instance.identity),
                    receipts: instance.receipts,
                    captures: instance.captures,
                    capture_evaluations: instance.capture_evaluations,
                    activation: Activation::Active,
                    truth,
                    basis,
                    incomplete,
                    support: evaluator.support,
                    premises: premises.clone(),
                })));
            }
            Ok(assessed)
        }
    }
}

fn canonical_number(value: &w::Number, subject: Subject) -> Result<String, Error> {
    let value = value
        .checked()
        .map_err(|_| Refusal::Reference { subject })?;
    serde_json::to_string(&value).map_err(|_| Refusal::Reference { subject }.into())
}

fn expected_parameters(
    configuration: &v2::wire::ClockConfiguration,
    subject: Subject,
) -> Result<BTreeMap<String, String>, Error> {
    let mut parameters = BTreeMap::new();
    match configuration {
        v2::wire::ClockConfiguration::EventPosition { sequence_authority } => {
            parameters.insert("sequence_authority".into(), sequence_authority.clone());
        }
        v2::wire::ClockConfiguration::FixedSample {
            epoch,
            period,
            unit,
        } => {
            parameters.insert("epoch".into(), canonical_number(epoch, subject)?);
            parameters.insert("period".into(), canonical_number(period, subject)?);
            parameters.insert("unit".into(), unit.clone());
        }
        v2::wire::ClockConfiguration::TimestampedEvent { timestamp_unit } => {
            parameters.insert("timestamp_unit".into(), timestamp_unit.clone());
        }
    }
    Ok(parameters)
}

fn parameter_dimension(
    configuration: &v2::wire::ClockConfiguration,
    trace: &Trace,
    expected: &BTreeMap<String, String>,
) -> Dimension {
    if trace.clock.parameters.len() != expected.len()
        || !trace.clock.parameters.keys().eq(expected.keys())
    {
        return Dimension::ClockParameters;
    }
    match configuration {
        v2::wire::ClockConfiguration::EventPosition { .. } => Dimension::SequenceAuthority,
        v2::wire::ClockConfiguration::FixedSample { .. }
            if trace.clock.parameters.get("epoch") != expected.get("epoch") =>
        {
            Dimension::Epoch
        }
        v2::wire::ClockConfiguration::FixedSample { .. }
            if trace.clock.parameters.get("period") != expected.get("period") =>
        {
            Dimension::SamplePeriod
        }
        v2::wire::ClockConfiguration::FixedSample { .. } => Dimension::ClockUnit,
        v2::wire::ClockConfiguration::TimestampedEvent { .. } => Dimension::TimestampUnit,
    }
}

fn authenticate_v2(
    package: &v2::AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    work: &mut budget::Work,
) -> Result<progress::AuthenticatedBinding, Error> {
    let subject = declaration_subject(declaration);
    work.subject = subject;
    let selected = select(package.inherited(), declaration, subject)?;
    if trace.clock.profile_identity != selected.profile.identity() {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject,
        }
        .into());
    }
    if trace.clock.profile_revision != selected.revision {
        return Err(Refusal::Binding {
            dimension: Dimension::ProfileRevision,
            subject,
        }
        .into());
    }
    let w::Body::Temporal { clock, .. } = selected.body else {
        return Err(Refusal::Reference { subject }.into());
    };
    let clock_binding = selected
        .declaration
        .bindings
        .get(usize::try_from(*clock).unwrap_or(usize::MAX))
        .ok_or(Refusal::Reference { subject })?;
    let clock_name = clock_binding
        .name
        .strip_prefix(CLOCK_PREFIX)
        .ok_or(Refusal::Reference { subject })?;
    if trace.clock.name != clock_name {
        return Err(Refusal::Binding {
            dimension: Dimension::Clock,
            subject,
        }
        .into());
    }
    let (binding, selection) = authenticated_selection(package, declaration, &selected, subject)?;
    let expected = expected_parameters(&binding.clock, subject)?;
    if trace.clock.parameters != expected {
        return Err(Refusal::Binding {
            dimension: parameter_dimension(&binding.clock, trace, &expected),
            subject,
        }
        .into());
    }
    // One derivation for both version-2 entry points: the evaluation binding
    // restates the classification's selection and adds the clock it checked.
    Ok(progress::AuthenticatedBinding {
        package_digest: selection.package_digest().to_string(),
        declaration: selection.declaration(),
        definition_identity: selection.definition_identity().into(),
        definition_revision: selection.definition_revision().value.clone(),
        clock: clock_name.into(),
        parameters: expected,
    })
}

/// Map a three-valued evaluation onto its truth and settlement basis. An open
/// scope settles only from a decisive witness or counterexample; every other
/// open future is pending and unsettled.
fn settle(value: Tri, trace: &Trace) -> (Truth, Basis) {
    let closed =
        trace.decision_scope == Closure::Closed && trace.completeness == Completeness::Complete;
    match value {
        Tri::Unknown => (Truth::Pending, Basis::Unsettled),
        Tri::True if closed => (Truth::True, Basis::ClosedScope),
        Tri::False if closed => (Truth::False, Basis::ClosedScope),
        Tri::True => (Truth::True, Basis::DecisiveWitness),
        Tri::False => (Truth::False, Basis::DecisiveCounterexample),
    }
}
