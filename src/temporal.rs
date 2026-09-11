// SPDX-License-Identifier: AGPL-3.0-only
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
mod result;
mod trace;

use crate::protocol_artifact::{wire as w, AdmittedPackage};

use activation::{Outcome, Shape};
use formula::{Evaluator, Tri};

pub use budget::{Dimension as LimitDimension, Exhaustion, Limits, Usage, ACCOUNTING_VERSION};
pub use mapping::{
    classify, Operators, Support, Target, Unmatched, OUTSTANDING_PREMISES, SUPPORT_TABLE,
};
pub use profile::{Profile, EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW};
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
    let result = run(package, declaration, trace, &mut work);
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
    }
}

/// Classify the native-to-TL mapping support of one admitted temporal
/// declaration. This emits no TL artifact and establishes no correspondence.
pub fn mapping_support(
    package: &AdmittedPackage,
    declaration: usize,
    surrounding_execution: Closure,
) -> Result<Support, Error> {
    let mut work = budget::Work::new(Limits::default());
    let subject = Subject {
        declaration,
        ..Subject::default()
    };
    work.subject = subject;
    let entry = select(package, declaration, &mut work)?;
    Ok(classify(
        entry.profile,
        Operators::of(&entry.declaration.temporal),
        surrounding_execution,
    ))
}

/// The admitted declaration, its temporal body and its selected profile.
struct Selected<'a> {
    declaration: &'a w::Declaration,
    body: &'a w::Body,
    profile: Profile,
    revision: String,
}

fn select<'a>(
    package: &'a AdmittedPackage,
    declaration: usize,
    work: &mut budget::Work,
) -> Result<Selected<'a>, Error> {
    let package = package.package();
    let entry = package
        .declarations
        .get(declaration)
        .ok_or(Refusal::Reference {
            subject: work.subject,
        })?;
    if !matches!(entry.body, w::Body::Temporal { .. }) {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject: work.subject,
        }
        .into());
    }
    let definition = package
        .definitions
        .get(usize::try_from(entry.profile).unwrap_or(usize::MAX))
        .ok_or(Refusal::Reference {
            subject: work.subject,
        })?;
    let profile = Profile::from_identity(&definition.identity).ok_or(Refusal::Binding {
        dimension: Dimension::Profile,
        subject: work.subject,
    })?;
    Ok(Selected {
        declaration: entry,
        body: &entry.body,
        profile,
        revision: definition.revision.value.clone(),
    })
}

fn run(
    package: &AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    work: &mut budget::Work,
) -> Result<Vec<Obligation>, Error> {
    let subject = Subject {
        declaration,
        ..Subject::default()
    };
    work.subject = subject;
    let selected = select(package, declaration, work)?;
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
        watermark: trace.watermark,
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
                    Err(error) => {
                        assessed.push(Obligation::Unactivated {
                            subject,
                            instance: None,
                            error,
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
