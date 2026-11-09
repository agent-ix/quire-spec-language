// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-563: the parity runner over test backends that delegate to the
//! interpreter and differ from it in one chosen way.

use std::cell::Cell;
use std::rc::Rc;

use ix_trace_rs::trace;
use qsl_conformance::{
    run_parity, run_parity_with_reference, FindingKind, Member, ParityInputs, ParityLimits,
    ParityRefusal,
};
use qsl_eval::execution::{
    first_outside, BackendCoverage, CallLimits, CoverageOperation, CoverageType, EntrySelection,
    ExecutionBackend, Interpreter, PrepareFailure, PrepareLimits, PreparedEntries, SuppliedBackend,
};
use qsl_eval::value::QualifiedName;
use qsl_package::CheckedPackage;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_eval::{
    CallFailure, Evaluation, ExecutionBackendChoice, FamilyOutcome, UnsupportedCause,
    UnsupportedEntry,
};
use quire_exact::{Cancel, CancelCause, Outcome, ScalarLimits, Undefined, Value};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::location::{ExpressionLocation, ExpressionOwner};

type Tamper =
    Rc<dyn Fn(&str, usize, Result<Evaluation, CallFailure>) -> Result<Evaluation, CallFailure>>;

/// A backend that runs the interpreter and changes its results with `tamper`.
#[derive(Clone)]
struct Delegate {
    tamper: Tamper,
    calls: Rc<Cell<usize>>,
    prepares: Rc<Cell<usize>>,
    fail_prepare_at: Option<usize>,
    /// Entries `first_outside` finds outside this coverage are listed
    /// unsupported.
    unsupported_outside: Option<Rc<BackendCoverage>>,
}

impl Delegate {
    fn plain() -> Self {
        Self {
            tamper: Rc::new(|_, _, result| result),
            calls: Rc::new(Cell::new(0)),
            prepares: Rc::new(Cell::new(0)),
            fail_prepare_at: None,
            unsupported_outside: None,
        }
    }

    fn tampering(
        tamper: impl Fn(&str, usize, Result<Evaluation, CallFailure>) -> Result<Evaluation, CallFailure>
            + 'static,
    ) -> Self {
        Self {
            tamper: Rc::new(tamper),
            ..Self::plain()
        }
    }
}

struct DelegatePrepared<'p> {
    inner: <Interpreter as ExecutionBackend>::Prepared<'p>,
    package: &'p CheckedPackage,
    owner: Delegate,
    unsupported: Vec<(EntrySelection, UnsupportedEntry)>,
}

impl SuppliedBackend for Delegate {
    fn prepare_entries<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Box<dyn PreparedEntries + 'p>, PrepareFailure> {
        let n = self.prepares.get();
        self.prepares.set(n + 1);
        if self.fail_prepare_at == Some(n) {
            return Err(PrepareFailure::Limit(
                qsl_foundation::diagnostic::LimitExceeded::new(
                    qsl_foundation::Setting::PrepareCodeBytes,
                    0,
                    1,
                ),
            ));
        }
        let inner = Interpreter.prepare(package, entries, limits, cancel)?;
        let mut unsupported = Vec::new();
        if let Some(coverage) = &self.unsupported_outside {
            for selection in entries {
                let graph = package.graph();
                let Some(callable) = graph.callable(&selection.name.to_string()) else {
                    continue;
                };
                let function = graph
                    .function_by_identity(callable.identity.clone())
                    .expect("a body");
                if let Some(outside) = first_outside(callable.parameters, function.body, coverage) {
                    unsupported.push((
                        selection.clone(),
                        UnsupportedEntry {
                            backend: ExecutionBackendChoice::Jit,
                            cause: UnsupportedCause::Untranslatable {
                                construct: outside.construct,
                                location: outside.location,
                            },
                        },
                    ));
                }
            }
        }
        Ok(Box::new(DelegatePrepared {
            inner,
            package,
            owner: self.clone(),
            unsupported,
        }))
    }
}

impl PreparedEntries for DelegatePrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &self.unsupported
    }

    fn call(
        &self,
        entry: &QualifiedName,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        limits: &CallLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        let _ = self.package;
        let n = self.owner.calls.get();
        self.owner.calls.set(n + 1);
        let result = self
            .inner
            .call(entry, arguments, environment, limits, cancel);
        (self.owner.tamper)(&entry.to_string(), n, result)
    }

    fn evaluate(
        &self,
        clause: &QualifiedName,
        input: &AdmittedObservations,
        accounting: &ScalarLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        self.inner.evaluate(clause, input, accounting, cancel)
    }
}

fn inputs(count: u64) -> ParityInputs {
    ParityInputs {
        count,
        ..ParityInputs::default()
    }
}

fn run(delegate: &Delegate, count: u64) -> qsl_conformance::ParityReport {
    run_parity(
        &[(ExecutionBackendChoice::Jit, delegate, None)],
        &inputs(count),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs")
}

#[trace("TC-1386", "FR-563-AC-1")]
#[test]
fn a_backend_that_delegates_to_the_interpreter_passes() {
    let report = run(&Delegate::plain(), 25);
    assert_eq!(
        report.total_findings(),
        0,
        "{:?}",
        report.backends[0].findings
    );
    assert!(report.backends[0].compared > 0);
    assert_eq!(report.exit_code(), 0);
}

/// The generator is a pure function of seed and count (FR-295).
#[trace("TC-1386")]
#[test]
fn the_generator_yields_byte_equal_units_for_the_same_seed_and_count() {
    let (a, discarded_a) = qsl_conformance::generator::generate(7, 12);
    let (b, discarded_b) = qsl_conformance::generator::generate(7, 12);
    assert_eq!(discarded_a, discarded_b);
    assert_eq!(a.len(), 12);
    for ((x, _), (y, _)) in a.iter().zip(&b) {
        assert_eq!(x.source, y.source);
        assert_eq!(format!("{:?}", x.functions), format!("{:?}", y.functions));
    }
}

fn first_finding(delegate: &Delegate) -> qsl_conformance::Finding {
    let report = run(delegate, 6);
    assert_eq!(report.exit_code(), 1);
    report.backends[0].findings[0].clone()
}

fn location() -> ExpressionLocation {
    ExpressionLocation::root(ExpressionOwner::Body {
        function: "f0".to_owned(),
        index: 0,
    })
}

#[trace("TC-1386")]
#[test]
fn a_backend_that_differs_in_one_member_fails_with_a_difference_naming_it() {
    let cases: Vec<(Member, Delegate)> = vec![
        (
            Member::OkAgainstErr,
            Delegate::tampering(|_, n, r| {
                if n < 2 {
                    Err(CallFailure::Cancelled(CancelCause::Requested))
                } else {
                    r
                }
            }),
        ),
        (
            Member::OutcomeKind,
            Delegate::tampering(|_, n, r| {
                if n < 2 {
                    r.map(|mut e| {
                        e.outcome =
                            FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::NoneValue));
                        e
                    })
                } else {
                    r
                }
            }),
        ),
        (
            Member::Location,
            Delegate::tampering(|_, n, r| {
                if n < 2 {
                    r.map(|mut e| {
                        e.location = Some(location());
                        e
                    })
                } else {
                    r
                }
            }),
        ),
        (
            Member::CompletedValue,
            Delegate::tampering(|_, n, r| {
                if n < 2 {
                    r.map(|mut e| {
                        if let FamilyOutcome::Evaluated(Outcome::Completed(v)) = &mut e.outcome {
                            *v = match v {
                                Value::Boolean(b) => Value::Boolean(!*b),
                                _ => Value::Integer(quire_exact::Integer::from(123_456_i64)),
                            };
                        }
                        e
                    })
                } else {
                    r
                }
            }),
        ),
    ];
    for (member, delegate) in cases {
        let finding = first_finding(&delegate);
        assert_eq!(finding.kind, FindingKind::Difference(member));
        assert_eq!(finding.seed, qsl_conformance::DEFAULT_SEED);
        assert!(!finding.source.is_empty() && !finding.entry.is_empty());
        assert!(finding.reference != finding.result);
    }
    // A backend `Fault` where the reference returned anything else.
    let faulty = Delegate::tampering(|_, n, r| {
        if n < 2 {
            Err(CallFailure::Fault(InternalFault::new("test", "injected")))
        } else {
            r
        }
    });
    assert_eq!(
        first_finding(&faulty).kind,
        FindingKind::Difference(Member::OkAgainstErr)
    );
}

#[trace("TC-1386", "FR-563-AC-3")]
#[test]
fn a_backend_whose_result_alternates_fails_with_nondeterministic() {
    let flip = Rc::new(Cell::new(false));
    let delegate = Delegate::tampering({
        let flip = Rc::clone(&flip);
        move |_, _, r| {
            flip.set(!flip.get());
            if flip.get() {
                r.map(|mut e| {
                    if let FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(b))) =
                        &mut e.outcome
                    {
                        *b = !*b;
                    } else if let FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(_))) =
                        &e.outcome
                    {
                        e.outcome = FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(
                            quire_exact::Integer::from(9_999_i64),
                        )));
                    }
                    e
                })
            } else {
                r
            }
        }
    });
    assert_eq!(first_finding(&delegate).kind, FindingKind::Nondeterministic);
}

fn coverage(with_let: bool) -> BackendCoverage {
    let forms: Vec<&'static str> = ["literal", "operator", "if", "call"]
        .into_iter()
        .chain(with_let.then_some("let"))
        .collect();
    let operations = [
        "+", "-", "*", "negate", "mod", "<", "<=", ">", ">=", "=", "!=", "and", "or", "implies",
        "not",
    ]
    .into_iter()
    .flat_map(|operator| {
        [
            CoverageType::BoundedInteger,
            CoverageType::UnboundedInteger,
            CoverageType::Boolean,
        ]
        .into_iter()
        .map(move |operand| CoverageOperation { operator, operand })
    });
    BackendCoverage::new(
        forms,
        [
            CoverageType::Boolean,
            CoverageType::BoundedInteger,
            CoverageType::UnboundedInteger,
        ],
        operations,
    )
}

/// The backend lists every entry that uses `let` unsupported.
fn no_let_backend() -> Delegate {
    Delegate {
        unsupported_outside: Some(Rc::new(coverage(false))),
        ..Delegate::plain()
    }
}

#[trace("TC-1387", "FR-563-AC-4")]
#[test]
fn a_coverage_that_lists_let_while_the_backend_refuses_it_is_a_coverage_gap() {
    let backend = no_let_backend();
    let lists_let = coverage(true);
    let report = run_parity(
        &[(ExecutionBackendChoice::Jit, &backend, Some(&lists_let))],
        &inputs(25),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs");
    let gaps = report.backends[0]
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::CoverageGap)
        .count();
    assert!(gaps > 0, "{:?}", report.backends[0].unsupported.len());
    assert_eq!(report.exit_code(), 1);

    let omits_let = coverage(false);
    let report = run_parity(
        &[(ExecutionBackendChoice::Jit, &backend, Some(&omits_let))],
        &inputs(25),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs");
    assert_eq!(
        report.total_findings(),
        0,
        "{:?}",
        report.backends[0].findings
    );
    assert!(!report.backends[0].unsupported.is_empty());
}

#[trace("TC-1387", "FR-563-AC-5")]
#[test]
fn a_prepare_failure_names_the_unit_and_a_backend_with_nothing_compared_is_empty() {
    let mut failing = Delegate::plain();
    failing.fail_prepare_at = Some(1);
    let report = run(&failing, 4);
    let failed: Vec<_> = report.backends[0]
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::PrepareFailed)
        .collect();
    assert_eq!(failed.len(), 1);

    // Every entry unsupported and no coverage supplied.
    let none = Delegate {
        unsupported_outside: Some(Rc::new(BackendCoverage::new([], [], []))),
        ..Delegate::plain()
    };
    let report = run(&none, 4);
    assert!(report.backends[0]
        .findings
        .iter()
        .any(|f| f.kind == FindingKind::EmptyComparison));
}

#[trace("TC-1387", "FR-563-AC-6")]
#[test]
fn the_gate_refuses_without_a_backend_or_with_an_unreadable_corpus() {
    let refusal = run_parity(&[], &inputs(1), &ParityLimits::default(), &Cancel::new());
    assert_eq!(refusal.unwrap_err(), ParityRefusal::EmptyScope);
    let backend = Delegate::plain();
    let corpus = ParityInputs {
        corpus: Some("/nonexistent/qsl-corpus".into()),
        ..inputs(1)
    };
    let refusal = run_parity(
        &[(ExecutionBackendChoice::Jit, &backend, None)],
        &corpus,
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .unwrap_err();
    assert!(matches!(refusal, ParityRefusal::UnreadableCorpus(_)));
    assert_eq!(refusal.exit_code(), 2);
}

#[trace("TC-1387", "FR-563-AC-6")]
#[test]
fn a_reference_fault_is_a_reference_fault_finding() {
    let reference = Delegate::tampering(|_, n, r| {
        if n == 0 {
            Err(CallFailure::Fault(InternalFault::new("test", "oracle")))
        } else {
            r
        }
    });
    let backend = Delegate::plain();
    let report = run_parity_with_reference(
        &reference,
        &[(ExecutionBackendChoice::Jit, &backend, None)],
        &inputs(3),
        &ParityLimits::default(),
        &Cancel::new(),
    )
    .expect("the gate runs");
    let faults = report.backends[0]
        .findings
        .iter()
        .filter(|f| f.kind == FindingKind::ReferenceFault)
        .count();
    assert!(faults >= 1);
    assert_eq!(report.exit_code(), 1);
}

#[trace("TC-1386", "FR-563-AC-7")]
#[test]
fn a_report_holds_100_findings_of_150_and_the_run_reaches_every_unit() {
    let delegate = Delegate::tampering(|_, n, r| {
        if n < 300 {
            r.map(|mut e| {
                e.location = Some(location());
                e
            })
        } else {
            r
        }
    });
    let report = run(&delegate, 40);
    let backend = &report.backends[0];
    assert_eq!(backend.total_findings, 150);
    assert_eq!(backend.findings.len(), 100);
    assert_eq!(report.units, 40);
}
