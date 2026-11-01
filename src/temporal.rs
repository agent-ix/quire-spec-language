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
mod protocol;
mod result;
mod trace;

use std::collections::{BTreeMap, BTreeSet};

use crate::protocol_artifact::{v2, wire as w, AdmittedPackage, ClockNames};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    Activation as CheckedActivation, CheckedFormula, CheckedInterval, FormulaNode, TemporalProfile,
};
use quire_checked::node::NodeKind;

use activation::{Outcome, Shape};
use formula::{Evaluator, Tri};

pub use budget::{Dimension as LimitDimension, Exhaustion, Limits, Usage, ACCOUNTING_VERSION};
pub use mapping::{
    classify, AuthenticatedSelection, CheckedClassification, CheckedClassificationV2,
    CheckedRetained, Classification, Operators, Retained, Support, Target, Unmatched,
    OUTSTANDING_PREMISES,
};
pub use profile::{Profile, EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW};
pub use progress::{AuthenticatedBinding, Binding, Ledger, Progress};
use progress::{CheckedAuthenticatedBinding, CheckedBinding};
pub use protocol::{
    ProtocolEvaluation, ProtocolEvaluationId, ProtocolEvaluationInput, ProtocolLedger,
    ProtocolRefusal, ProtocolSelection, SemanticTriggerIdentity,
};
pub use result::{
    Activation, Assessment, Basis, Capture, Closure, Completeness, Dimension, Error, Execution,
    Incomplete, Obligation, Premises, Refusal, Report, Subject, Support as DecisionSupport, Truth,
};
pub use trace::{
    CaptureInput, ClockBinding, Eviction, Evidence, OrderKey, Position, Trace, Trigger,
};

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
        package.clock_names(),
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
        settled_siblings: work.settled_siblings,
    }
}

/// Evaluate a temporal clause directly from its checked package graph.
///
/// The formula, profile, activation shape and clock binding are read from S3's
/// checked nodes. The checked graph is the authority; this path does not emit,
/// serialize, or reconstruct a protocol artifact.
pub fn evaluate_checked(
    package: &CheckedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
) -> Report {
    let mut work = budget::Work::new(limits);
    let result = run_checked(
        package,
        declaration,
        trace,
        trace.watermark,
        &mut work,
        None,
    );
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated: None,
        settled_siblings: work.settled_siblings,
    }
}

/// Evaluate an S3 checked temporal clause after joining it to one strict `/2`
/// admitted temporal selection.
///
/// The join requires the checked source identity, declaration name and source
/// span to match the independently admitted artifact. The selected profile
/// and clock name must also match before the v2 clock fields are authenticated.
/// All checks happen before temporal positions are visited. The formula and
/// activation are evaluated from `checked`, never reconstructed from the v2
/// artifact.
pub fn evaluate_checked_v2(
    checked: &CheckedPackage,
    admitted: &v2::AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
) -> Report {
    let mut work = budget::Work::checked_v2(limits);
    let mut authenticated = None;
    let result = authenticate_checked_v2(checked, admitted, declaration, trace, &mut work)
        .and_then(|(binding, _)| {
            authenticated = Some(binding);
            run_checked(
                checked,
                declaration,
                trace,
                trace.watermark,
                &mut work,
                None,
            )
        });
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated,
        settled_siblings: work.settled_siblings,
    }
}

/// Checked-package counterpart of [`evaluate_with_progress`].
pub fn evaluate_checked_with_progress(
    package: &CheckedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
    ledger: &mut Ledger,
) -> Report {
    let mut work = budget::Work::new(limits);
    let binding = package
        .graph()
        .temporal_clauses()
        .get(declaration)
        .map(|node| {
            CheckedBinding::of(
                declaration,
                node.identity(),
                package.graph().source().clone(),
                trace,
            )
        })
        .ok_or(Refusal::Reference {
            subject: declaration_subject(declaration),
        });
    let result = match binding {
        Ok(binding) => match ledger.record_checked(binding.clone(), Progress::of(trace)) {
            Ok(retained) => {
                let instances = ledger.checked_instances.entry(binding).or_default();
                run_checked(
                    package,
                    declaration,
                    trace,
                    retained.watermark,
                    &mut work,
                    Some(instances),
                )
            }
            Err(refusal) => Err(refusal.into()),
        },
        Err(refusal) => Err(refusal.into()),
    };
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated: None,
        settled_siblings: work.settled_siblings,
    }
}

/// Checked-package counterpart of [`evaluate_with_progress_v2`].
///
/// Progress is retained under the strict artifact declaration, selected
/// definition identity/revision and exact authenticated clock configuration.
/// A different admitted selection cannot settle this checked clause.
pub fn evaluate_checked_with_progress_v2(
    checked: &CheckedPackage,
    admitted: &v2::AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    limits: Limits,
    ledger: &mut Ledger,
) -> Report {
    let mut work = budget::Work::checked_v2(limits);
    let mut authenticated = None;
    let result = authenticate_checked_v2(checked, admitted, declaration, trace, &mut work)
        .and_then(|(binding, _)| {
            authenticated = Some(binding.clone());
            let checked_binding = checked
                .graph()
                .temporal_clauses()
                .get(declaration)
                .map(|node| {
                    CheckedBinding::of(
                        declaration,
                        node.identity(),
                        checked.graph().source().clone(),
                        trace,
                    )
                })
                .ok_or(Refusal::Reference {
                    subject: declaration_subject(declaration),
                })?;
            let ledger_binding = CheckedAuthenticatedBinding::of(checked_binding, binding);
            ledger
                .record_checked_authenticated(ledger_binding.clone(), Progress::of(trace))
                .map_err(Error::from)
                .and_then(|retained| {
                    let instances = ledger
                        .checked_authenticated_instances
                        .entry(ledger_binding)
                        .or_default();
                    run_checked(
                        checked,
                        declaration,
                        trace,
                        retained.watermark,
                        &mut work,
                        Some(instances),
                    )
                })
        });
    Report {
        result,
        limits: work.limits,
        usage: work.usage,
        authenticated,
        settled_siblings: work.settled_siblings,
    }
}

/// Classify the checked clause only after matching its exact S3 source locus
/// and profile to a strict `/2` admitted selection.
///
/// No trace is supplied, so this operation authenticates no parameter map.
pub fn mapping_support_checked_v2(
    checked: &CheckedPackage,
    admitted: &v2::AdmittedPackage,
    declaration: usize,
    surrounding_execution: Closure,
) -> Result<CheckedClassificationV2, Error> {
    let subject = declaration_subject(declaration);
    let (artifact_declaration, selected) =
        checked_v2_selection(checked, admitted, declaration, subject)?;
    let (binding, authenticated) = authenticated_selection(
        admitted,
        artifact_declaration,
        &selected,
        Subject {
            declaration: artifact_declaration,
            ..Subject::default()
        },
    )?;
    let classification = mapping_support_checked(checked, declaration, surrounding_execution)?;
    Ok(CheckedClassificationV2 {
        support: classification.support,
        retained: classification.retained,
        authenticated,
        clock_name: binding.clock_name.clone(),
        clock: binding.clock.clone(),
    })
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
            package.clock_names(),
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
        settled_siblings: work.settled_siblings,
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

/// Classify FR-045 directly from an S3 checked temporal-clause node.
pub fn mapping_support_checked(
    package: &CheckedPackage,
    declaration: usize,
    surrounding_execution: Closure,
) -> Result<CheckedClassification, Error> {
    let subject = declaration_subject(declaration);
    let node = package
        .graph()
        .temporal_clauses()
        .get(declaration)
        .ok_or(Refusal::Reference { subject })?;
    let clause = node.clause();
    let operators = checked_operators(clause.formula());
    let support = match clause.profile() {
        TemporalProfile::EventPositionFalseExtension => {
            classify(Profile::EventPosition, operators, surrounding_execution)
        }
        TemporalProfile::FixedSampleFalseExtension => {
            classify(Profile::FixedSample, operators, surrounding_execution)
        }
        TemporalProfile::TimestampedEventFiniteWindow => {
            let mut dimensions = vec![Unmatched::FiniteWindow];
            if operators.past && operators.future {
                dimensions.push(Unmatched::MixedDirection);
            }
            Support::Unsupported { dimensions }
        }
        TemporalProfile::InfiniteTrace => {
            let mut dimensions = vec![Unmatched::InfiniteTrace];
            if operators.past && operators.future {
                dimensions.push(Unmatched::MixedDirection);
            }
            Support::Unsupported { dimensions }
        }
        TemporalProfile::Timed => {
            let mut dimensions = vec![Unmatched::TimedProfile];
            if operators.past && operators.future {
                dimensions.push(Unmatched::MixedDirection);
            }
            Support::Unsupported { dimensions }
        }
    };
    Ok(CheckedClassification {
        support,
        retained: CheckedRetained {
            identity: node.identity(),
            name: clause.name().to_owned(),
            profile: clause.profile(),
            activation: clause.activation(),
        },
    })
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
            package.clock_names(),
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
        settled_siblings: work.settled_siblings,
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
                    package.clock_names(),
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
        settled_siblings: work.settled_siblings,
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

/// The subject naming one whole declaration.
fn declaration_subject(declaration: usize) -> Subject {
    Subject {
        declaration,
        ..Subject::default()
    }
}

/// Find the admitted declaration whose exact source, name and byte span name
/// the selected checked temporal node. The v2 declaration index is returned
/// separately because it indexes the artifact binding table, not S3 nodes.
fn checked_v2_selection<'a>(
    checked: &CheckedPackage,
    admitted: &'a v2::AdmittedPackage,
    declaration: usize,
    subject: Subject,
) -> Result<(usize, Selected<'a>), Error> {
    let node = checked
        .graph()
        .temporal_clauses()
        .get(declaration)
        .ok_or(Refusal::Reference { subject })?;
    let clause = node.clause();
    let source = checked.graph().source();
    let span = clause.span();
    let start = u32::try_from(span.start).map_err(|_| Refusal::Reference { subject })?;
    let end = u32::try_from(span.end).map_err(|_| Refusal::Reference { subject })?;
    let package = admitted.inherited();
    let matches = package
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            candidate.name == clause.name()
                && candidate.locus.span.start == start
                && candidate.locus.span.end == end
                && package
                    .sources
                    .get(usize::try_from(candidate.locus.source).unwrap_or(usize::MAX))
                    .is_some_and(|origin| {
                        origin.artifact.authority == source.authority()
                            && origin.artifact.identity == source.identity()
                    })
        });
    let mut matches = matches;
    let (artifact_declaration, _) = matches.next().ok_or(Refusal::Reference { subject })?;
    if matches.next().is_some() {
        return Err(Refusal::Reference { subject }.into());
    }
    let selected = select(package, artifact_declaration, subject)?;
    if selected.profile.identity() != clause.profile().identity() {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject,
        }
        .into());
    }
    let artifact_declaration_index =
        u32::try_from(artifact_declaration).map_err(|_| Refusal::Reference { subject })?;
    let temporal_binding = admitted
        .package()
        .temporal_bindings
        .binary_search_by_key(&artifact_declaration_index, |binding| binding.declaration)
        .ok()
        .and_then(|index| admitted.package().temporal_bindings.get(index))
        .ok_or(Refusal::Reference { subject })?;
    if clause.clock() != temporal_binding.clock_name {
        return Err(Refusal::Binding {
            dimension: Dimension::Clock,
            subject,
        }
        .into());
    }
    Ok((artifact_declaration, selected))
}

/// Authenticate the strict artifact's exact clock inputs against the checked
/// clause before evaluation begins, then return evidence identifying the same
/// selected definition and configuration.
fn authenticate_checked_v2(
    checked: &CheckedPackage,
    admitted: &v2::AdmittedPackage,
    declaration: usize,
    trace: &Trace,
    work: &mut budget::Work,
) -> Result<(progress::AuthenticatedBinding, usize), Error> {
    let subject = declaration_subject(declaration);
    work.subject = subject;
    let (artifact_declaration, selected) =
        checked_v2_selection(checked, admitted, declaration, subject)?;
    if trace.clock.profile_identity != selected.profile.identity() {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject,
        }
        .into());
    }
    let clock_name = admitted
        .clock_names()
        .get(artifact_declaration)
        .ok_or(Refusal::Reference { subject })?;
    if trace.clock.name != clock_name {
        return Err(Refusal::Binding {
            dimension: Dimension::Clock,
            subject,
        }
        .into());
    }
    let (binding, selection) =
        authenticated_selection(admitted, artifact_declaration, &selected, subject)?;
    let expected = expected_parameters(&binding.clock, subject)?;
    if trace.clock.parameters != expected {
        return Err(Refusal::Binding {
            dimension: parameter_dimension(&binding.clock, trace, &expected),
            subject,
        }
        .into());
    }
    Ok((
        progress::AuthenticatedBinding {
            declaration: selection.declaration(),
            definition_identity: selection.definition_identity().into(),
            definition_revision: selection.definition_revision().value.clone(),
            clock: clock_name.into(),
            parameters: expected,
        },
        artifact_declaration,
    ))
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

/// The asserted clock name must be the declaration's admitted clock name; a
/// declaration with no admitted clock name (its clock binding lacks the
/// `clock:` spelling) matches no asserted name.
fn check_clock_name(admitted: Option<&str>, asserted: &str, subject: Subject) -> Result<(), Error> {
    if admitted == Some(asserted) {
        Ok(())
    } else {
        Err(Refusal::Binding {
            dimension: Dimension::Clock,
            subject,
        }
        .into())
    }
}

fn run(
    package: &w::Package,
    clocks: &ClockNames,
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
    // A missing clock binding is a dangling reference; a clock binding whose
    // name lacks the `clock:` spelling names no clock and is a clock-binding
    // mismatch, exactly as when the name was parsed here.
    selected
        .declaration
        .bindings
        .get(usize::try_from(*clock).unwrap_or(usize::MAX))
        .ok_or(Refusal::Reference { subject })?;
    check_clock_name(clocks.get(declaration), &trace.clock.name, subject)?;

    let shape = Shape {
        origin: matches!(activation, w::Activation::Origin { .. }),
        guarded: matches!(activation, w::Activation::Each { guard, .. } if guard.0.is_some()),
        captures: captures.len(),
    };
    let root = usize::try_from(root.index).map_err(|_| Refusal::Reference { subject })?;
    let operations: Vec<_> = selected
        .declaration
        .temporal
        .iter()
        .map(|node| node.operation.clone())
        .collect();
    run_view(
        EvaluationView {
            declaration,
            profile: selected.profile,
            shape,
            root,
            operations,
            instance_bound_atoms: BTreeSet::new(),
        },
        trace,
        watermark,
        work,
        None,
    )
}

struct EvaluationView {
    declaration: usize,
    profile: Profile,
    shape: Shape,
    root: usize,
    operations: Vec<w::TemporalOperation>,
    instance_bound_atoms: BTreeSet<u32>,
}

enum LedgerUpdate {
    Insert(activation::Instance),
    AppendReceipts {
        identity: String,
        receipts: Vec<String>,
    },
}

fn run_view(
    view: EvaluationView,
    trace: &Trace,
    watermark: i64,
    work: &mut budget::Work,
    mut retained_instances: Option<&mut BTreeMap<String, activation::Instance>>,
) -> Result<Vec<Obligation>, Error> {
    let subject = declaration_subject(view.declaration);
    let premises = Premises {
        profile: view.profile,
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
    let ordered = trace.ordered();
    match activation::resolve(
        view.shape,
        trace,
        view.declaration,
        work,
        retained_instances.as_deref_mut(),
    )? {
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
            let mut ledger_updates = Vec::new();
            for (ordinal, resolved) in instances.into_iter().enumerate() {
                let subject = Subject {
                    instance: ordinal,
                    ..subject
                };
                work.subject = subject;
                let resolved = match resolved {
                    activation::Resolved::Failed(failed) => {
                        if let Some(receipt) = failed.pending_receipt {
                            ledger_updates.push(LedgerUpdate::AppendReceipts {
                                identity: failed.identity.clone(),
                                receipts: vec![receipt],
                            });
                        }
                        assessed.push(Obligation::Unactivated {
                            subject,
                            instance: failed.identity,
                            error: failed.error,
                        });
                        continue;
                    }
                    resolved => resolved,
                };
                let instance_identity = match &resolved {
                    activation::Resolved::Owned(instance) => instance.identity.as_str(),
                    activation::Resolved::Retained { identity, .. } => identity,
                    activation::Resolved::Failed(_) => unreachable!("failed activation handled"),
                };
                let mut evaluator = Evaluator {
                    profile: view.profile,
                    trace,
                    ordered: ordered.clone(),
                    nodes: &view.operations,
                    declaration: view.declaration,
                    instance: ordinal,
                    instance_identity,
                    instance_bound_atoms: &view.instance_bound_atoms,
                    watermark,
                    support: Vec::new(),
                    retained_valuations: BTreeSet::new(),
                };
                let outcome = evaluator.root(view.root, work);
                let support = std::mem::take(&mut evaluator.support);
                drop(evaluator);
                let (truth, basis, incomplete) = match outcome {
                    Ok(value) => {
                        let (truth, basis) = settle(value, trace);
                        (Some(truth), Some(basis), None)
                    }
                    Err(Error::Incomplete(incomplete)) => {
                        (None, Some(Basis::Unavailable), Some(incomplete))
                    }
                    Err(error) => {
                        work.settled_siblings
                            .extend(assessed.iter().filter_map(|obligation| match obligation {
                                Obligation::Assessed(assessment) if assessment.truth.is_some() => {
                                    Some(obligation.clone())
                                }
                                _ => None,
                            }));
                        return Err(error);
                    }
                };
                let instance = match resolved {
                    activation::Resolved::Owned(instance) => {
                        if retained_instances.is_some() {
                            ledger_updates.push(LedgerUpdate::Insert(instance.clone()));
                        }
                        instance
                    }
                    activation::Resolved::Retained { identity, receipts } => {
                        let saved = retained_instances
                            .as_deref()
                            .and_then(|retained| retained.get(&identity))
                            .expect("the retained instance remains in the ledger");
                        let mut restored = saved.clone();
                        restored.receipts.extend(receipts.iter().cloned());
                        ledger_updates.push(LedgerUpdate::AppendReceipts { identity, receipts });
                        restored
                    }
                    activation::Resolved::Failed(_) => unreachable!("failed activation handled"),
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
                    support,
                    premises: premises.clone(),
                })));
            }
            if let Some(retained) = retained_instances {
                for update in ledger_updates {
                    match update {
                        LedgerUpdate::Insert(instance) => {
                            retained.insert(instance.identity.clone(), instance);
                        }
                        LedgerUpdate::AppendReceipts { identity, receipts } => {
                            if let Some(saved) = retained.get_mut(&identity) {
                                saved.receipts.extend(receipts);
                            }
                        }
                    }
                }
            }
            Ok(assessed)
        }
    }
}

fn run_checked(
    package: &CheckedPackage,
    declaration: usize,
    trace: &Trace,
    watermark: i64,
    work: &mut budget::Work,
    retained_instances: Option<&mut BTreeMap<String, activation::Instance>>,
) -> Result<Vec<Obligation>, Error> {
    let subject = declaration_subject(declaration);
    work.subject = subject;
    let node = package
        .graph()
        .temporal_clauses()
        .get(declaration)
        .ok_or(Refusal::Reference { subject })?;
    let clause = node.clause();
    let profile = checked_profile(clause.profile()).ok_or(Refusal::Binding {
        dimension: Dimension::Profile,
        subject,
    })?;
    if trace.clock.profile_identity != clause.profile().identity() {
        return Err(Refusal::Binding {
            dimension: Dimension::Profile,
            subject,
        }
        .into());
    }
    check_clock_name(Some(clause.clock()), &trace.clock.name, subject)?;
    let operations = checked_operations(clause.formula(), subject)?;
    let instance_bound_atoms = checked_instance_bound_atoms(clause, node);
    let root = clause.formula().root().index();
    let activation = clause.activation();
    let shape = Shape {
        origin: matches!(activation, CheckedActivation::Origin),
        guarded: matches!(activation, CheckedActivation::Each { guarded: true }),
        captures: clause.captures().len(),
    };
    if shape.guarded != node.guard().is_some()
        || (0..shape.captures).any(|capture| node.capture(capture).is_none())
        || clause
            .formula()
            .nodes()
            .iter()
            .enumerate()
            .any(|(index, formula)| {
                matches!(formula, FormulaNode::Holds { .. })
                    && clause
                        .formula()
                        .node_id(index)
                        .and_then(|atom| node.condition(atom))
                        .is_none()
            })
    {
        return Err(Refusal::Reference { subject }.into());
    }
    run_view(
        EvaluationView {
            declaration,
            profile,
            shape,
            root,
            operations,
            instance_bound_atoms,
        },
        trace,
        watermark,
        work,
        retained_instances,
    )
}

fn checked_profile(profile: TemporalProfile) -> Option<Profile> {
    match profile {
        TemporalProfile::EventPositionFalseExtension => Some(Profile::EventPosition),
        TemporalProfile::FixedSampleFalseExtension => Some(Profile::FixedSample),
        TemporalProfile::TimestampedEventFiniteWindow => Some(Profile::TimestampedWindow),
        TemporalProfile::InfiniteTrace | TemporalProfile::Timed => None,
    }
}

fn checked_operators(formula: &CheckedFormula) -> Operators {
    let mut result = Operators::default();
    let mut seen = vec![false; formula.nodes().len()];
    let mut pending = vec![formula.root().index()];
    while let Some(index) = pending.pop() {
        let Some(node) = formula.nodes().get(index) else {
            continue;
        };
        if std::mem::replace(&mut seen[index], true) {
            continue;
        }
        match node {
            FormulaNode::Unary {
                operator, operand, ..
            } => {
                match operator {
                    qsl_semantics::check::UnaryTemporal::Eventually
                    | qsl_semantics::check::UnaryTemporal::Always => result.future = true,
                    qsl_semantics::check::UnaryTemporal::Once
                    | qsl_semantics::check::UnaryTemporal::Historically => result.past = true,
                }
                pending.push(operand.index());
            }
            FormulaNode::Binary {
                operator,
                left,
                right,
                ..
            } => {
                match operator {
                    qsl_semantics::check::BinaryTemporal::Until
                    | qsl_semantics::check::BinaryTemporal::Release => result.future = true,
                    qsl_semantics::check::BinaryTemporal::Since
                    | qsl_semantics::check::BinaryTemporal::Triggered => result.past = true,
                }
                pending.extend([left.index(), right.index()]);
            }
            FormulaNode::Not(value) => pending.push(value.index()),
            FormulaNode::And(left, right)
            | FormulaNode::Or(left, right)
            | FormulaNode::Implies(left, right) => {
                pending.extend([left.index(), right.index()]);
            }
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Aggregate { .. } => {}
        }
    }
    result
}

fn checked_operations(
    formula: &CheckedFormula,
    subject: Subject,
) -> Result<Vec<w::TemporalOperation>, Error> {
    let mut operations = Vec::with_capacity(formula.nodes().len());
    for (index, node) in formula.nodes().iter().enumerate() {
        let atom_index = u32::try_from(index).map_err(|_| Refusal::Reference { subject })?;
        let handle = |id: qsl_semantics::check::FormulaNodeId| w::Handle {
            declaration: 0,
            index: u32::try_from(id.index()).unwrap_or(u32::MAX),
        };
        let operation = match node {
            FormulaNode::Constant(value) => w::TemporalOperation::Constant { value: *value },
            FormulaNode::Holds { .. } => w::TemporalOperation::Holds {
                value: w::Handle {
                    declaration: 0,
                    index: atom_index,
                },
            },
            FormulaNode::Aggregate { .. } => {
                return Err(Error::Unsupported(
                    crate::checking::composed::proofs::Unsupported::ValueRepresentation,
                ))
            }
            FormulaNode::Not(inner) => w::TemporalOperation::Unary {
                operator: w::TemporalUnary::Not,
                interval: w::Nullable(None),
                value: handle(*inner),
            },
            FormulaNode::And(left, right)
            | FormulaNode::Or(left, right)
            | FormulaNode::Implies(left, right) => {
                let operator = match node {
                    FormulaNode::And(..) => w::TemporalBinary::And,
                    FormulaNode::Or(..) => w::TemporalBinary::Or,
                    _ => w::TemporalBinary::Implies,
                };
                w::TemporalOperation::Binary {
                    operator,
                    interval: w::Nullable(None),
                    left: handle(*left),
                    right: handle(*right),
                }
            }
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => w::TemporalOperation::Unary {
                operator: match operator {
                    qsl_semantics::check::UnaryTemporal::Eventually => w::TemporalUnary::Eventually,
                    qsl_semantics::check::UnaryTemporal::Always => w::TemporalUnary::Always,
                    qsl_semantics::check::UnaryTemporal::Once => w::TemporalUnary::Once,
                    qsl_semantics::check::UnaryTemporal::Historically => {
                        w::TemporalUnary::Historically
                    }
                },
                interval: checked_interval(interval.as_ref(), subject)?,
                value: handle(*operand),
            },
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => w::TemporalOperation::Binary {
                operator: match operator {
                    qsl_semantics::check::BinaryTemporal::Until => w::TemporalBinary::Until,
                    qsl_semantics::check::BinaryTemporal::Release => w::TemporalBinary::Release,
                    qsl_semantics::check::BinaryTemporal::Since => w::TemporalBinary::Since,
                    qsl_semantics::check::BinaryTemporal::Triggered => w::TemporalBinary::Triggered,
                },
                interval: checked_interval(interval.as_ref(), subject)?,
                left: handle(*left),
                right: handle(*right),
            },
        };
        operations.push(operation);
    }
    Ok(operations)
}

/// Find checked temporal atoms whose expression reads a capture slot.
///
/// These observations are instance-specific: one trigger's capture-bound
/// predicate result cannot satisfy another trigger's proposition.
fn checked_instance_bound_atoms(
    clause: &qsl_semantics::check::CheckedTemporalClause,
    node: &qsl_semantics::check::TemporalClauseNode,
) -> BTreeSet<u32> {
    let first_capture_slot = match clause.activation() {
        CheckedActivation::Origin => 1,
        CheckedActivation::Each { .. } => 2,
    };
    let mut result = BTreeSet::new();
    for (index, formula) in clause.formula().nodes().iter().enumerate() {
        if !matches!(formula, FormulaNode::Holds { .. }) {
            continue;
        }
        let Some(atom) = clause.formula().node_id(index) else {
            continue;
        };
        let Some((root, _)) = node.condition(atom) else {
            continue;
        };
        let mut pending = vec![root];
        while let Some(expression) = pending.pop() {
            if matches!(
                expression.kind(),
                NodeKind::Local(slot) if *slot >= first_capture_slot
            ) {
                if let Ok(atom) = u32::try_from(index) {
                    result.insert(atom);
                }
                break;
            }
            pending.extend(expression.children());
        }
    }
    result
}

fn checked_interval(
    interval: Option<&CheckedInterval>,
    subject: Subject,
) -> Result<w::Nullable<w::Interval>, Error> {
    match interval {
        None => Ok(w::Nullable(None)),
        Some(CheckedInterval::Steps(interval)) => {
            let upper = match interval.upper() {
                qsl_foundation::bound::TemporalUpper::Finite(value) => value,
                qsl_foundation::bound::TemporalUpper::Open => {
                    return Err(Refusal::Binding {
                        dimension: Dimension::Interval,
                        subject,
                    }
                    .into())
                }
            };
            let lower = w::Integer(crate::protocol_artifact::NumberWire::Integer {
                decimal: interval.lower().to_string(),
            });
            let upper = w::Integer(crate::protocol_artifact::NumberWire::Integer {
                decimal: upper.to_string(),
            });
            Ok(w::Nullable(Some(w::Interval { lower, upper })))
        }
        Some(CheckedInterval::Timed(_)) => Err(Refusal::Binding {
            dimension: Dimension::Interval,
            subject,
        }
        .into()),
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
    let clock_name = package
        .clock_names()
        .get(declaration)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A clock binding without the `clock:` spelling has no admitted clock
    /// name; the evaluator refuses it as a clock-binding mismatch, the same
    /// refusal as a wrong asserted name, and accepts only the exact name.
    #[test]
    fn a_clock_binding_without_the_prefix_is_a_clock_binding_mismatch() {
        let subject = declaration_subject(0);
        for admitted in [None, Some("other")] {
            let error = check_clock_name(admitted, "orders", subject).expect_err("refused");
            assert!(
                matches!(
                    error,
                    Error::Refused(Refusal::Binding {
                        dimension: Dimension::Clock,
                        ..
                    })
                ),
                "{admitted:?}: {error:?}"
            );
        }
        assert!(check_clock_name(Some("orders"), "orders", subject).is_ok());
    }
}
