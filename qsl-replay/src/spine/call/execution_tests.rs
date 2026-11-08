// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-279, FR-294, FR-296 (TC-760, TC-778, TC-781): the execution-backend
//! seam over the spine-run fixture, function path.
use super::*;
use crate::outcome::OutcomeDocument;
use crate::spine::DependencyInput;
use ix_trace_rs::trace;
use qsl_eval::execution::{
    CallLimits, EntrySelection, ExecuteLimits, ExecuteRequest, ExecuteSelection, ExecutionBackend,
    Interpreter, PrepareFailure, PrepareLimits, PrepareMeter, PreparedEntries, SuppliedBackends,
};
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, SourceIdentity};
use quire_eval::{
    CallFailure, Evaluation, ExecutionBackendChoice, PrepareLimitReached, PrepareSetting,
    UnsupportedCause, UnsupportedEntry, UntranslatableConstruct,
};
use quire_exact::{Cancel, CancelCause, Integer, Meter, Value};
use quire_semantic_value::catalog::{Category, InternalFault};
use quire_semantic_value::location::{ExpressionLocation, ExpressionOwner};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

const FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-run.native");

fn checked() -> front_end::CheckedUnit {
    let source = SourceIdentity::new("agent-ix", "execution-seam");
    let cancel = Cancel::new();
    let parsed = front_end::parse(
        &front_end::ParseRequest {
            source: &source,
            path: "spine-run.native",
            bytes: FIXTURE.as_bytes(),
        },
        Default::default(),
        &cancel,
    )
    .expect("parses")
    .into_value();
    let models = front_end::select(&parsed, &BTreeMap::new(), Default::default(), &cancel)
        .expect("selects")
        .into_value();
    front_end::check(
        &parsed,
        "fixture".to_owned(),
        &models,
        &DependencyInput::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect("checks")
    .into_value()
}

fn name(function: &str) -> QualifiedName {
    QualifiedName::unqualified(function).expect("an identifier")
}

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn request(
    function: &str,
    arguments: Vec<Value>,
    backend: ExecutionBackendChoice,
) -> ExecuteRequest {
    ExecuteRequest {
        selection: ExecuteSelection::Function {
            name: name(function),
            arguments,
        },
        environment: ObjectEnvironment::default(),
        backend,
    }
}

fn limits() -> ExecuteLimits {
    ExecuteLimits {
        accounting: default_accounting(DEFAULT_WORK_UNITS),
        supplied: Default::default(),
        prepare: PrepareLimits::default(),
    }
}

fn direct(
    unit: &front_end::CheckedUnit,
    function: &str,
    arguments: Vec<Value>,
) -> Result<Evaluation, CallFailure> {
    let mut meter = Meter::new(default_accounting(DEFAULT_WORK_UNITS));
    unit.package().call(
        &name(function),
        arguments,
        &ObjectEnvironment::default(),
        &mut meter,
    )
}

fn location() -> ExpressionLocation {
    ExpressionLocation::root(ExpressionOwner::Body {
        function: "id".to_owned(),
        index: 2,
    })
}

/// A test backend standing in for `aot` or `jit`. It runs the interpreter
/// for the entries it supports, lists `unsupported` entries as such, and
/// counts its `prepare` calls and the calls that reach the interpreter.
struct Stub {
    unsupported: Vec<(QualifiedName, UnsupportedCause)>,
    prepared: Cell<usize>,
    interpreted: Rc<Cell<usize>>,
    refuse: Option<PrepareFailure>,
}

impl Stub {
    fn new() -> Self {
        Self {
            unsupported: Vec::new(),
            prepared: Cell::new(0),
            interpreted: Rc::new(Cell::new(0)),
            refuse: None,
        }
    }
}

struct StubPrepared<'p> {
    interpreted: Rc<Cell<usize>>,
    inner: <Interpreter as ExecutionBackend>::Prepared<'p>,
    unsupported: Vec<(EntrySelection, UnsupportedEntry)>,
}

impl ExecutionBackend for Stub {
    type Prepared<'p> = StubPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        self.prepared.set(self.prepared.get() + 1);
        if let Some(failure) = &self.refuse {
            return Err(failure.clone());
        }
        let inner = Interpreter.prepare(package, entries, limits, cancel)?;
        let unsupported = entries
            .iter()
            .filter_map(|entry| {
                let EntrySelection::Function(function) = entry;
                self.unsupported
                    .iter()
                    .find(|(listed, _)| listed == function)
                    .map(|(_, cause)| {
                        (
                            entry.clone(),
                            UnsupportedEntry {
                                backend: ExecutionBackendChoice::Jit,
                                cause: cause.clone(),
                            },
                        )
                    })
            })
            .collect();
        Ok(StubPrepared {
            interpreted: Rc::clone(&self.interpreted),
            inner,
            unsupported,
        })
    }
}

impl PreparedEntries for StubPrepared<'_> {
    fn unsupported(&self) -> &[(EntrySelection, UnsupportedEntry)] {
        &self.unsupported
    }

    fn call(
        &self,
        entry: &EntrySelection,
        arguments: &[Value],
        environment: &ObjectEnvironment,
        limits: &CallLimits,
        cancel: &Cancel,
    ) -> Result<Evaluation, CallFailure> {
        if let Some((_, unsupported)) = self.unsupported.iter().find(|(listed, _)| listed == entry)
        {
            return Err(CallFailure::Unsupported(unsupported.clone()));
        }
        self.interpreted.set(self.interpreted.get() + 1);
        self.inner
            .call(entry, arguments, environment, limits, cancel)
    }
}

fn seven_is_7(result: &Result<Evaluation, CallFailure>) {
    match result {
        Ok(Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))),
            ..
        }) => assert_eq!(*value, Integer::from(7_i64)),
        other => panic!("expected the integer 7, got {other:?}"),
    }
}

/// FR-279-AC-1 and FR-294-AC-1 (function half): `execute` on the interpreter
/// returns what `CheckedPackage::call` returns for `lt(5, 3)`, `flag(1)` and
/// `id(4)`, and the same `CallFailure::Input` for `id(12)`.
#[trace("TC-760", "FR-279-AC-1")]
#[trace("TC-778", "FR-294-AC-1")]
#[test]
fn the_interpreter_returns_what_the_package_call_returns() {
    let unit = checked();
    let cases = [
        ("lt", vec![integer(5), integer(3)]),
        ("flag", vec![Value::Boolean(true)]),
        ("id", vec![integer(4)]),
        ("id", vec![integer(12)]),
    ];
    for (function, arguments) in cases {
        let through = qsl_eval::execution::execute(
            unit.package(),
            &request(
                function,
                arguments.clone(),
                ExecutionBackendChoice::Interpreter,
            ),
            &SuppliedBackends::default(),
            &limits(),
            &Cancel::new(),
        );
        let expected = direct(&unit, function, arguments);
        assert_eq!(
            format!("{through:?}"),
            format!("{expected:?}"),
            "{function}"
        );
    }
    let lt = qsl_eval::execution::execute(
        unit.package(),
        &request(
            "lt",
            vec![integer(5), integer(3)],
            ExecutionBackendChoice::Interpreter,
        ),
        &SuppliedBackends::default(),
        &limits(),
        &Cancel::new(),
    );
    assert!(matches!(
        lt,
        Ok(Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))),
            ..
        })
    ));
    let refused = qsl_eval::execution::execute(
        unit.package(),
        &request("id", vec![integer(12)], ExecutionBackendChoice::Interpreter),
        &SuppliedBackends::default(),
        &limits(),
        &Cancel::new(),
    );
    assert!(matches!(refused, Err(CallFailure::Input(_))), "{refused:?}");
}

/// FR-279-AC-3: the `outcome` of the run document is the rendering of
/// `execute`'s result for the same function, arguments and accounting.
#[trace("TC-760", "FR-279-AC-3")]
#[test]
fn the_run_outcome_is_the_rendering_of_executes_result() {
    let unit = checked();
    let sources = vec![unit.source().clone()];
    let cases = [
        (
            "lt",
            vec![("a", 5), ("b", 3)],
            vec![integer(5), integer(3)],
            1_000_000,
        ),
        ("id", vec![("x", 4)], vec![integer(4)], 1_000_000),
        ("seven", vec![], vec![], 0),
    ];
    for (function, named, values, work_units) in cases {
        let call = Call::new(
            function.to_owned(),
            named
                .iter()
                .map(|(parameter, value)| CallArgument {
                    parameter: (*parameter).to_owned(),
                    value: integer(*value),
                })
                .collect(),
            default_accounting(work_units),
            Default::default(),
        );
        let ran = run(
            SourceIdentity::new("agent-ix", "execution-seam"),
            "fixture".to_owned(),
            "spine-run.native",
            FIXTURE.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
            &ObjectEnvironment::default(),
            &Cancel::new(),
        );
        let mut execute_limits = limits();
        execute_limits.accounting = default_accounting(work_units);
        let evaluation = qsl_eval::execution::execute(
            unit.package(),
            &request(function, values, ExecutionBackendChoice::Interpreter),
            &SuppliedBackends::default(),
            &execute_limits,
            &Cancel::new(),
        )
        .expect("the interpreter completes or reports incomplete");
        let rendered = convert_outcome(evaluation, unit.package().graph(), &sources);
        assert_eq!(format!("{ran:?}"), format!("{rendered:?}"), "{function}");
    }
}

/// FR-279-AC-4: a supplied `aot` stand-in whose `prepare` reaches
/// `prepare.code_bytes` yields `CallFailure::Limit` with the leaf's record,
/// category incomplete (exit 22); a `Fault` keeps its `InternalFault`.
#[trace("TC-760", "FR-279-AC-4")]
#[test]
fn a_prepare_limit_and_a_prepare_fault_reach_the_call_failure() {
    let unit = checked();
    let mut aot = Stub::new();
    aot.refuse = Some(PrepareFailure::Limit(LimitExceeded::new(
        Setting::PrepareCodeBytes,
        1,
        2,
    )));
    let backends = SuppliedBackends {
        aot: Some(&aot),
        jit: None,
    };
    let failure = qsl_eval::execution::execute(
        unit.package(),
        &request("seven", vec![], ExecutionBackendChoice::Aot),
        &backends,
        &limits(),
        &Cancel::new(),
    )
    .expect_err("prepare reached its limit");
    let expected = PrepareLimitReached {
        setting: PrepareSetting::CodeBytes,
        bound: 1,
        actual: 2,
    };
    assert_eq!(failure.category(), Category::Incomplete);
    assert_eq!(failure.category().exit_code(), 22);
    match &failure {
        CallFailure::Limit(reached) => {
            assert_eq!(*reached, expected);
            assert_eq!(reached.setting.name(), "prepare.code_bytes");
            assert_eq!(reached.code(), "stage_limit_exceeded");
        }
        other => panic!("expected Limit, got {other:?}"),
    }
    let refusal = convert_call_failure(failure);
    assert_eq!(refusal.code(), Code::StageLimitExceeded);
    assert_eq!(refusal.category(), Category::Incomplete);

    let fault = InternalFault::new("aot", "prepare-broke");
    aot.refuse = Some(PrepareFailure::Fault(fault.clone()));
    let backends = SuppliedBackends {
        aot: Some(&aot),
        jit: None,
    };
    let failure = qsl_eval::execution::execute(
        unit.package(),
        &request("seven", vec![], ExecutionBackendChoice::Aot),
        &backends,
        &limits(),
        &Cancel::new(),
    )
    .expect_err("prepare faulted");
    assert!(
        matches!(failure, CallFailure::Fault(ref got) if *got == fault),
        "{failure:?}"
    );
}

fn selected(functions: &[&str]) -> Vec<EntrySelection> {
    functions
        .iter()
        .map(|function| EntrySelection::Function(name(function)))
        .collect()
}

/// FR-294-AC-3: a `jit` stand-in that cannot run the third of three selected
/// entries prepares `Ok`, lists exactly that entry, runs the other two as the
/// interpreter does, and returns `Unsupported` for the third.
#[trace("TC-778", "FR-294-AC-3")]
#[test]
fn an_untranslatable_entry_is_listed_and_the_others_run() {
    let unit = checked();
    let cause = UnsupportedCause::Untranslatable {
        construct: UntranslatableConstruct::ExpressionForm("let".to_owned()),
        location: location(),
    };
    let mut jit = Stub::new();
    jit.unsupported = vec![(name("id"), cause.clone())];
    let entries = selected(&["lt", "flag", "id"]);
    let prepared = jit
        .prepare(
            unit.package(),
            &entries,
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .expect("an unsupported entry does not fail prepare");
    let expected = UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause,
    };
    assert_eq!(
        prepared.unsupported(),
        &[(entries[2].clone(), expected.clone())]
    );
    let call = |entry: &EntrySelection, arguments: &[Value]| {
        prepared.call(
            entry,
            arguments,
            &ObjectEnvironment::default(),
            &CallLimits {
                accounting: default_accounting(DEFAULT_WORK_UNITS),
                supplied: Default::default(),
            },
            &Cancel::new(),
        )
    };
    let lt = call(&entries[0], &[integer(5), integer(3)]);
    assert_eq!(
        format!("{lt:?}"),
        format!("{:?}", direct(&unit, "lt", vec![integer(5), integer(3)]))
    );
    let flag = call(&entries[1], &[Value::Boolean(true)]);
    assert_eq!(
        format!("{flag:?}"),
        format!("{:?}", direct(&unit, "flag", vec![Value::Boolean(true)]))
    );
    assert_eq!(
        call(&entries[2], &[integer(4)]).expect_err("unsupported"),
        CallFailure::Unsupported(expected)
    );
}

/// FR-294-AC-4: a handle already cancelled returns
/// `PrepareFailure::Cancelled` and `CallFailure::Cancelled`.
#[trace("TC-778", "FR-294-AC-4")]
#[test]
fn a_cancelled_handle_stops_prepare_and_call() {
    let unit = checked();
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let entries = selected(&["seven"]);
    assert_eq!(
        Interpreter
            .prepare(unit.package(), &entries, &PrepareLimits::default(), &cancel)
            .err(),
        Some(PrepareFailure::Cancelled(CancelCause::Requested))
    );
    let prepared = Interpreter
        .prepare(
            unit.package(),
            &entries,
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .expect("prepares");
    let called = prepared.call(
        &entries[0],
        &[],
        &ObjectEnvironment::default(),
        &CallLimits {
            accounting: default_accounting(DEFAULT_WORK_UNITS),
            supplied: Default::default(),
        },
        &cancel,
    );
    assert_eq!(
        called.expect_err("cancelled"),
        CallFailure::Cancelled(CancelCause::Requested)
    );
}

/// A test backend that charges one compile work unit per entry.
struct PerEntry;

impl ExecutionBackend for PerEntry {
    type Prepared<'p> = <Interpreter as ExecutionBackend>::Prepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        let mut meter = PrepareMeter::new(limits, cancel);
        for _ in entries {
            meter.charge_node()?;
        }
        Interpreter.prepare(package, entries, limits, cancel)
    }
}

/// FR-294-AC-5: the published defaults, a limit naming
/// `prepare.compile_work_units` at configured value 2 and counter 3, and the
/// interpreter prepared with both fields 0.
#[trace("TC-778", "FR-294-AC-5")]
#[test]
fn prepare_limits_default_and_bound_the_work_units() {
    let defaults = PrepareLimits::default();
    assert_eq!(defaults.compile_work_units, 16_777_216);
    assert_eq!(defaults.code_bytes, 67_108_864);
    let unit = checked();
    let entries = selected(&["lt", "flag", "id"]);
    let bounded = PrepareLimits {
        compile_work_units: 2,
        ..defaults
    };
    let failure = PerEntry
        .prepare(unit.package(), &entries, &bounded, &Cancel::new())
        .expect_err("the third charge exceeds 2");
    match failure {
        PrepareFailure::Limit(limit) => {
            assert_eq!(limit.setting(), Setting::PrepareCompileWorkUnits);
            assert_eq!(limit.configured_bound(), 2);
            assert_eq!(limit.actual(), 3);
        }
        other => panic!("expected Limit, got {other:?}"),
    }
    assert!(PerEntry
        .prepare(unit.package(), &entries, &defaults, &Cancel::new())
        .is_ok());
    let zero = PrepareLimits {
        compile_work_units: 0,
        code_bytes: 0,
    };
    assert!(Interpreter
        .prepare(unit.package(), &entries, &zero, &Cancel::new())
        .is_ok());
}

/// The test body: 5 checked nodes translated to blocks of these byte lengths.
const NODES: usize = 5;
const BLOCKS: [u64; 2] = [10, 20];

fn translate(
    limits: &PrepareLimits,
    cancel: &Cancel,
    observed: &std::sync::atomic::AtomicUsize,
    pause_at: Option<usize>,
) -> Result<(), PrepareFailure> {
    use std::sync::atomic::Ordering;
    let mut meter = PrepareMeter::new(limits, cancel);
    for _ in 0..NODES {
        meter.charge_node()?;
        let seen = observed.fetch_add(1, Ordering::SeqCst) + 1;
        if pause_at == Some(seen) {
            while cancel.cause().is_none() {
                std::thread::yield_now();
            }
        }
    }
    for block in BLOCKS {
        meter.charge_block(block)?;
    }
    Ok(())
}

/// FR-294-AC-6: charges come one per node and one per block, each before the
/// work; a bound one below the count names its setting with the counter the
/// charge reached; at the count it succeeds; a cancel from a second thread
/// after 3 charges returns `Cancelled(Requested)` with at most one more
/// charge. The body here is the test backend's own five nodes and two
/// blocks.
#[trace("TC-778", "FR-294-AC-6")]
#[test]
fn prepare_charges_per_node_and_per_block_and_polls_the_handle() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let total: u64 = BLOCKS.iter().sum();
    let at_count = PrepareLimits {
        compile_work_units: NODES as u64,
        code_bytes: total,
    };
    let observed = AtomicUsize::new(0);
    assert_eq!(
        translate(&at_count, &Cancel::new(), &observed, None),
        Ok(())
    );

    let below_nodes = PrepareLimits {
        compile_work_units: NODES as u64 - 1,
        ..at_count
    };
    match translate(&below_nodes, &Cancel::new(), &AtomicUsize::new(0), None) {
        Err(PrepareFailure::Limit(limit)) => {
            assert_eq!(limit.setting(), Setting::PrepareCompileWorkUnits);
            assert_eq!(limit.configured_bound(), NODES as u64 - 1);
            assert_eq!(limit.actual(), NODES as u128);
        }
        other => panic!("expected the node limit, got {other:?}"),
    }
    let below_bytes = PrepareLimits {
        code_bytes: total - 1,
        ..at_count
    };
    match translate(&below_bytes, &Cancel::new(), &AtomicUsize::new(0), None) {
        Err(PrepareFailure::Limit(limit)) => {
            assert_eq!(limit.setting(), Setting::PrepareCodeBytes);
            assert_eq!(limit.configured_bound(), total - 1);
            assert_eq!(limit.actual(), u128::from(total));
        }
        other => panic!("expected the byte limit, got {other:?}"),
    }

    let cancel = Cancel::new();
    let observed = AtomicUsize::new(0);
    let result = std::thread::scope(|scope| {
        let worker = scope.spawn(|| translate(&at_count, &cancel, &observed, Some(3)));
        while observed.load(Ordering::SeqCst) < 3 {
            std::thread::yield_now();
        }
        cancel.cancel(CancelCause::Requested);
        worker.join().expect("the worker finishes")
    });
    assert_eq!(
        result,
        Err(PrepareFailure::Cancelled(CancelCause::Requested))
    );
    assert!(observed.load(Ordering::SeqCst) <= 4);
}

/// FR-294-AC-7: a backend's translator names, for each entry, the first
/// construct in node order it cannot translate, with its kind, identity and
/// location. This stand-in refuses `let`, `Decimal` and one operation.
#[trace("TC-778", "FR-294-AC-7")]
#[test]
fn the_first_untranslatable_construct_of_each_entry_is_named() {
    let operation = quire_exact::NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text("ops".to_owned())],
        7,
    )
    .expect("a node key");
    let decimal = ValueType::Decimal(
        quire_exact::DecimalType::new(
            Integer::from(0_i64),
            Integer::from(10_i64),
            0,
            2,
            quire_exact::RoundingMode::Exact,
        )
        .expect("a decimal type"),
    );
    let refuses = |construct: &UntranslatableConstruct| match construct {
        UntranslatableConstruct::ExpressionForm(form) => form == "let",
        UntranslatableConstruct::ValueType(value_type) => {
            matches!(**value_type, ValueType::Decimal(_))
        }
        UntranslatableConstruct::Operation(key) => *key == operation,
    };
    let form = |name: &str| UntranslatableConstruct::ExpressionForm(name.to_owned());
    let entries = [
        ("first", vec![form("literal"), form("let"), form("call")]),
        (
            "second",
            vec![
                UntranslatableConstruct::ValueType(Box::new(ValueType::Integer)),
                UntranslatableConstruct::ValueType(Box::new(decimal.clone())),
            ],
        ),
        (
            "third",
            vec![
                form("if"),
                UntranslatableConstruct::Operation(operation.clone()),
                form("let"),
            ],
        ),
    ];
    let expected = [
        ("let".to_owned(), "expression-form"),
        ("Decimal".to_owned(), "value-type"),
        (operation.to_string(), "operation"),
    ];
    for ((function, nodes), (name, kind)) in entries.into_iter().zip(expected) {
        let first = nodes
            .into_iter()
            .find(|construct| refuses(construct))
            .expect("each entry has an untranslatable construct");
        let entry = UnsupportedEntry {
            backend: ExecutionBackendChoice::Jit,
            cause: UnsupportedCause::Untranslatable {
                construct: first,
                location: location(),
            },
        };
        let payload = entry.payload();
        assert_eq!(payload.backend, "jit", "{function}");
        assert_eq!(payload.construct, Some(kind), "{function}");
        assert_eq!(payload.construct_name, Some(name), "{function}");
        assert_eq!(payload.location, Some(&location()), "{function}");
        assert_eq!(entry.catalog_code().code(), "unimplemented_capability");
    }
}

/// FR-296-AC-1: a request that names no other backend runs `seven` on the
/// interpreter and completes with 7, through `execute` and through the spine.
#[trace("TC-781", "FR-296-AC-1")]
#[test]
fn the_interpreter_runs_seven_by_default() {
    let unit = checked();
    let result = qsl_eval::execution::execute(
        unit.package(),
        &request("seven", vec![], ExecutionBackendChoice::Interpreter),
        &SuppliedBackends::default(),
        &limits(),
        &Cancel::new(),
    );
    seven_is_7(&result);
    let call = Call::new(
        "seven".to_owned(),
        vec![],
        default_accounting(1_000_000),
        Default::default(),
    );
    assert_eq!(call.backend, ExecutionBackendChoice::Interpreter);
    let outcome = run(
        SourceIdentity::new("agent-ix", "execution-seam"),
        "fixture".to_owned(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("completes");
    assert!(
        matches!(outcome, CallOutcome::Completed(CallValue::Integer(ref v)) if v.to_string() == "7"),
        "{outcome:?}"
    );
}

/// FR-296-AC-2: choosing `jit` with `SuppliedBackends::default()` returns
/// `Unsupported` `NotSupplied`, category unsupported (exit 21); the spine
/// refuses before any stage with `unimplemented_capability`.
#[trace("TC-781", "FR-296-AC-2")]
#[test]
fn an_unsupplied_backend_refuses_as_unsupported() {
    let unit = checked();
    let failure = qsl_eval::execution::execute(
        unit.package(),
        &request("seven", vec![], ExecutionBackendChoice::Jit),
        &SuppliedBackends::default(),
        &limits(),
        &Cancel::new(),
    )
    .expect_err("jit is not supplied");
    let expected = UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause: UnsupportedCause::NotSupplied,
    };
    assert_eq!(failure, CallFailure::Unsupported(expected.clone()));
    assert_eq!(failure.category(), Category::Unsupported);
    assert_eq!(failure.category().exit_code(), 21);

    // No stage runs: empty bytes would refuse at the source stage.
    let mut call = Call::new(
        "seven".to_owned(),
        vec![],
        default_accounting(1_000_000),
        Default::default(),
    );
    call.backend = ExecutionBackendChoice::Jit;
    let refusal = run(
        SourceIdentity::new("agent-ix", "execution-seam"),
        "fixture".to_owned(),
        "spine-run.native",
        b"",
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect_err("refuses");
    assert!(matches!(*refusal, RunRefusal::Unsupported(ref entry) if *entry == expected));
    assert_eq!(refusal.code(), Code::UnimplementedCapability);
    assert_eq!(refusal.category(), Category::Unsupported);
    let document = OutcomeDocument::from_run(&Err(refusal));
    assert_eq!(document.category(), Category::Unsupported);
    assert_eq!(document.diagnostics()[0].code(), "unimplemented_capability");
    assert_eq!(
        document.diagnostics()[0].cause(),
        Some("backend-not-supplied")
    );
}

/// FR-296-AC-3: a `jit` stand-in that lists one entry unsupported makes
/// `execute` return that `UnsupportedEntry`, exit 21, with the interpreter
/// never called.
#[trace("TC-781", "FR-296-AC-3")]
#[test]
fn an_entry_the_chosen_backend_lists_unsupported_is_returned() {
    let unit = checked();
    let cause = UnsupportedCause::Untranslatable {
        construct: UntranslatableConstruct::ExpressionForm("let".to_owned()),
        location: location(),
    };
    let mut jit = Stub::new();
    jit.unsupported = vec![(name("seven"), cause.clone())];
    let backends = SuppliedBackends {
        aot: None,
        jit: Some(&jit),
    };
    let failure = qsl_eval::execution::execute(
        unit.package(),
        &request("seven", vec![], ExecutionBackendChoice::Jit),
        &backends,
        &limits(),
        &Cancel::new(),
    )
    .expect_err("seven is unsupported on the stand-in");
    let expected = UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause,
    };
    assert_eq!(failure, CallFailure::Unsupported(expected));
    assert_eq!(failure.category().exit_code(), 21);
    assert_eq!(jit.interpreted.get(), 0);
}

/// FR-296-AC-4: a `jit` stand-in passed as `SuppliedBackends { jit: Some(..) }`
/// through the blanket implementation runs `seven` when `jit` is chosen and
/// is the only backend whose `prepare` is called.
#[trace("TC-781", "FR-296-AC-4")]
#[test]
fn only_the_chosen_supplied_backend_prepares() {
    let unit = checked();
    let jit = Stub::new();
    let aot = Stub::new();
    let backends = SuppliedBackends {
        aot: Some(&aot),
        jit: Some(&jit),
    };
    let result = qsl_eval::execution::execute(
        unit.package(),
        &request("seven", vec![], ExecutionBackendChoice::Jit),
        &backends,
        &limits(),
        &Cancel::new(),
    );
    seven_is_7(&result);
    assert_eq!((jit.prepared.get(), aot.prepared.get()), (1, 0));

    // The spine passes the same choice and backends.
    let mut call = Call::new(
        "seven".to_owned(),
        vec![],
        default_accounting(1_000_000),
        Default::default(),
    );
    call.backend = ExecutionBackendChoice::Jit;
    let outcome = run_with_backends(
        SourceIdentity::new("agent-ix", "execution-seam"),
        "fixture".to_owned(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &backends,
        &Cancel::new(),
    )
    .expect("completes on the stand-in");
    assert!(
        matches!(outcome, CallOutcome::Completed(CallValue::Integer(ref v)) if v.to_string() == "7")
    );
    assert_eq!((jit.prepared.get(), aot.prepared.get()), (2, 0));
}
