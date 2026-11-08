// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-760, TC-778 (FR-279-AC-2, FR-294-AC-1, FR-294-AC-4): `execute` and the
//! interpreter's `evaluate` over FR-109's ConfigVersion unit, with the
//! observations FR-106 admits from the healthy-parent and violating-parent
//! snapshots.
use super::*;
use ix_trace_rs::trace;
use qsl_eval::execution::{
    execute, CallLimits, EntrySelection, ExecuteLimits, ExecuteRequest, ExecuteSelection,
    ExecutionBackend, Interpreter, PrepareFailure, PrepareLimits, PreparedEntries,
    SuppliedBackends,
};
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_semantics::model::observation::{AdmittedObservations, ObservationLimits, Provisions};
use quire_eval::{
    ExecutionBackendChoice, UnsupportedCause, UnsupportedEntry, UntranslatableConstruct,
};
use quire_exact::{Cancel, CancelCause, ScalarLimits};

fn name(clause: &str) -> QualifiedName {
    QualifiedName::unqualified(clause).expect("an identifier")
}

/// The observations FR-106 admits for `clause` from a current snapshot with
/// `root` at `root_version` and `child` at `child_version`, self `child`.
fn admitted(
    compiled: &ComposedUnit,
    clause: &str,
    root_version: i64,
    child_version: i64,
) -> AdmittedObservations {
    let (_, packages) = config_version_unit_and_packages();
    let label = config_version_document_ref("current-snap");
    let bytes = config_version_snapshot(&label, root_version, Some(child_version));
    let mut snapshots = BTreeMap::new();
    snapshots.insert(
        qsl_semantics::model::observation::document_reference(&bytes)
            .expect("a document names itself"),
        bytes,
    );
    let graph = compiled.package.graph();
    admit_clause_observations(
        graph,
        graph.state_clause(clause).expect("the clause is declared"),
        &packages,
        SpineLimits::default().model,
        &Provisions {
            snapshots: &snapshots,
            invocations: &snapshots,
        },
        &config_version_current_selection(clause, label, "child"),
        ObservationLimits::default(),
    )
    .expect("FR-106 admits the snapshot")
}

fn compiled() -> ComposedUnit {
    let (unit, packages) = config_version_unit_and_packages();
    compose(
        source(),
        "clause-run-config-version.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the ConfigVersion unit compiles")
}

fn accounting() -> ScalarLimits {
    default_accounting(1_000_000)
}

fn limits() -> ExecuteLimits {
    ExecuteLimits {
        accounting: accounting(),
        supplied: Default::default(),
        prepare: PrepareLimits::default(),
    }
}

fn clause_request(clause: &str, input: AdmittedObservations) -> ExecuteRequest {
    ExecuteRequest {
        selection: ExecuteSelection::Clause {
            clause: name(clause),
            input,
        },
        backend: ExecutionBackendChoice::Interpreter,
    }
}

/// What `evaluate_clause` returns for the same clause, observations and
/// accounting limits.
fn reference(
    compiled: &ComposedUnit,
    clause: &str,
    input: &AdmittedObservations,
) -> Result<quire_eval::Evaluation, CallFailure> {
    let mut meter = Meter::new(accounting());
    compiled
        .package
        .evaluate_clause(&name(clause), input, &mut meter)
        .map(|clause| clause.evaluation)
}

/// FR-279-AC-2: `execute` with the interpreter and a clause selection naming
/// `ParentOrder`, over the observations FR-106 admits from the healthy-parent
/// and violating-parent snapshots, returns the `evaluation` member of
/// `evaluate_clause`; its outcome is `Completed(Boolean(true))` and
/// `Completed(Boolean(false))`, equal to the `Evaluate` disposition
/// `run_clause` reports for the same request.
#[trace("TC-760", "FR-279-AC-2")]
#[test]
fn execute_a_clause_returns_what_evaluate_clause_returns() {
    let compiled = compiled();
    for (root, healthy) in [(1, true), (3, false)] {
        let input = admitted(&compiled, "ParentOrder", root, 2);
        let through = execute(
            &compiled.package,
            &clause_request("ParentOrder", input.clone()),
            &SuppliedBackends::default(),
            &limits(),
            &Cancel::new(),
        );
        let expected = reference(&compiled, "ParentOrder", &input);
        assert_eq!(format!("{through:?}"), format!("{expected:?}"), "{root}");
        let value = boolean_outcome(through.expect("the clause evaluates"));
        assert_eq!(value, healthy);
        let run = run_config_version_current("ParentOrder", root, Some(2), "child");
        assert_eq!(value, boolean_disposition(&run), "run_clause agrees");
    }
}

/// FR-294-AC-1 (clause half): the interpreter prepared over the unit with
/// `ParentOrder` and `NoCycle` selected returns, from `evaluate` of
/// `ParentOrder`, the `evaluation` member of `evaluate_clause`; observations
/// admitted for `NoCycle` return `CallFailure::Input(ObservationsMismatch)`,
/// as `evaluate_clause` does.
#[trace("TC-778", "FR-294-AC-1")]
#[test]
fn the_interpreter_evaluates_a_prepared_clause() {
    let compiled = compiled();
    let entries = [
        EntrySelection::Clause(name("ParentOrder")),
        EntrySelection::Clause(name("NoCycle")),
    ];
    let prepared = Interpreter
        .prepare(
            &compiled.package,
            &entries,
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .expect("the interpreter prepares every entry");
    assert!(prepared.unsupported().is_empty());
    for root in [1, 3] {
        let input = admitted(&compiled, "ParentOrder", root, 2);
        let evaluated =
            prepared.evaluate(&name("ParentOrder"), &input, &accounting(), &Cancel::new());
        let expected = reference(&compiled, "ParentOrder", &input);
        assert_eq!(format!("{evaluated:?}"), format!("{expected:?}"));
    }
    let other = admitted(&compiled, "NoCycle", 1, 2);
    let mismatch = prepared
        .evaluate(&name("ParentOrder"), &other, &accounting(), &Cancel::new())
        .expect_err("observations admitted for another clause");
    assert!(
        matches!(
            mismatch,
            CallFailure::Input(InputRefusal::ObservationsMismatch)
        ),
        "{mismatch:?}"
    );
    assert_eq!(
        format!("{mismatch:?}"),
        format!(
            "{:?}",
            reference(&compiled, "ParentOrder", &other).unwrap_err()
        )
    );
}

/// FR-294-AC-4 (clause half): `evaluate` with a handle already cancelled
/// returns `CallFailure::Cancelled`.
#[trace("TC-778", "FR-294-AC-4")]
#[test]
fn a_cancelled_handle_stops_evaluate() {
    let compiled = compiled();
    let input = admitted(&compiled, "ParentOrder", 1, 2);
    let entries = [EntrySelection::Clause(name("ParentOrder"))];
    let prepared = Interpreter
        .prepare(
            &compiled.package,
            &entries,
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .expect("prepares");
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    assert_eq!(
        prepared
            .evaluate(&name("ParentOrder"), &input, &accounting(), &cancel)
            .expect_err("cancelled"),
        CallFailure::Cancelled(CancelCause::Requested)
    );
    assert_eq!(
        Interpreter
            .prepare(
                &compiled.package,
                &entries,
                &PrepareLimits::default(),
                &cancel
            )
            .err(),
        Some(PrepareFailure::Cancelled(CancelCause::Requested))
    );
}

/// A backend that prepares nothing: it lists every selected entry
/// unsupported.
struct Refuses;

struct RefusesPrepared(Vec<(EntrySelection, UnsupportedEntry)>);

impl ExecutionBackend for Refuses {
    type Prepared<'p> = RefusesPrepared;

    fn prepare(
        &self,
        _package: &qsl_package::CheckedPackage,
        entries: &[EntrySelection],
        _limits: &PrepareLimits,
        _cancel: &Cancel,
    ) -> Result<RefusesPrepared, PrepareFailure> {
        Ok(RefusesPrepared(
            entries
                .iter()
                .map(|entry| {
                    (
                        entry.clone(),
                        UnsupportedEntry {
                            backend: ExecutionBackendChoice::Jit,
                            cause: UnsupportedCause::Untranslatable {
                                construct: UntranslatableConstruct::ExpressionForm(
                                    "let".to_owned(),
                                ),
                                location: quire_semantic_value::location::ExpressionLocation::root(
                                    quire_semantic_value::location::ExpressionOwner::Expression,
                                ),
                            },
                        },
                    )
                })
                .collect(),
        ))
    }
}

impl PreparedEntries for RefusesPrepared {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &self.0
    }

    fn call(
        &self,
        _entry: &QualifiedName,
        _arguments: &[Value],
        _environment: &ObjectEnvironment,
        _limits: &CallLimits,
        _cancel: &Cancel,
    ) -> Result<quire_eval::Evaluation, CallFailure> {
        unreachable!("only clauses are selected")
    }

    fn evaluate(
        &self,
        clause: &QualifiedName,
        _input: &AdmittedObservations,
        _accounting: &ScalarLimits,
        _cancel: &Cancel,
    ) -> Result<quire_eval::Evaluation, CallFailure> {
        let entry = self
            .0
            .iter()
            .find(|(listed, _)| listed.name() == clause)
            .expect("the clause was selected");
        Err(CallFailure::Unsupported(entry.1.clone()))
    }
}

/// FR-294-AC-3 (clause half): `execute` of a clause the chosen backend lists
/// unsupported returns that `UnsupportedEntry`.
#[trace("TC-778", "FR-294-AC-3")]
#[test]
fn a_clause_the_backend_lists_unsupported_is_returned() {
    let compiled = compiled();
    let input = admitted(&compiled, "ParentOrder", 1, 2);
    let jit = Refuses;
    let mut request = clause_request("ParentOrder", input);
    request.backend = ExecutionBackendChoice::Jit;
    let failure = execute(
        &compiled.package,
        &request,
        &SuppliedBackends {
            aot: None,
            jit: Some(&jit),
        },
        &limits(),
        &Cancel::new(),
    )
    .expect_err("the stand-in lists the clause unsupported");
    assert!(
        matches!(failure, CallFailure::Unsupported(ref entry) if entry.backend == ExecutionBackendChoice::Jit),
        "{failure:?}"
    );
}
