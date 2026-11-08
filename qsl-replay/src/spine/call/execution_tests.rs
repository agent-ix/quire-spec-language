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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-run.native");

fn checked() -> front_end::CheckedUnit {
    checked_source("spine-run.native", FIXTURE)
}

fn checked_source(path: &str, text: &str) -> front_end::CheckedUnit {
    let source = SourceIdentity::new("agent-ix", "execution-seam");
    let cancel = Cancel::new();
    let parsed = front_end::parse(
        &front_end::ParseRequest {
            source: &source,
            path,
            bytes: text.as_bytes(),
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

/// The `outcome` member `spine-run-result/1` renders for a completed or
/// incomplete call (FR-100): the member the CLI writes for these outcomes.
fn document_outcome(outcome: &CallOutcome) -> serde_json::Value {
    match outcome {
        CallOutcome::Completed(CallValue::Boolean(value)) => {
            serde_json::json!({"kind": "completed", "value": {"kind": "boolean", "value": value}})
        }
        CallOutcome::Completed(CallValue::Integer(value)) => serde_json::json!({
            "kind": "completed",
            "value": {"kind": "integer", "decimal": value.to_string()},
        }),
        CallOutcome::Incomplete(incomplete) => {
            serde_json::json!({"kind": "incomplete", "limit": incomplete.limits_field()})
        }
        other => panic!("the AC-1, AC-4 and AC-6 requests complete or stop: {other:?}"),
    }
}

/// FR-279-AC-3: for the FR-100-AC-1 request (`seven` -> 7), the FR-100-AC-4
/// requests (`lt`, `flag`, `id`) and the FR-100-AC-6 request (`seven` at
/// zero work units), the `outcome` member of the run document equals the
/// rendering of `execute`'s result for the same source, function, arguments
/// and accounting limits.
#[trace("TC-760", "FR-279-AC-3")]
#[test]
fn the_run_outcome_is_the_rendering_of_executes_result() {
    let unit = checked();
    let sources = vec![unit.source().clone()];
    let cases = [
        ("seven", vec![], vec![], 1_000_000, "completed"),
        (
            "lt",
            vec![("b", integer(3)), ("a", integer(5))],
            vec![integer(5), integer(3)],
            1_000_000,
            "completed",
        ),
        (
            "flag",
            vec![("b", Value::Boolean(true))],
            vec![Value::Boolean(true)],
            1_000_000,
            "completed",
        ),
        (
            "id",
            vec![("x", integer(4))],
            vec![integer(4)],
            1_000_000,
            "completed",
        ),
        ("seven", vec![], vec![], 0, "incomplete"),
    ];
    let expected = [
        serde_json::json!({"kind": "completed", "value": {"kind": "integer", "decimal": "7"}}),
        serde_json::json!({"kind": "completed", "value": {"kind": "boolean", "value": false}}),
        serde_json::json!({"kind": "completed", "value": {"kind": "boolean", "value": true}}),
        serde_json::json!({"kind": "completed", "value": {"kind": "integer", "decimal": "4"}}),
        serde_json::json!({"kind": "incomplete", "limit": "work_units"}),
    ];
    for ((function, named, values, work_units, kind), expected) in cases.into_iter().zip(expected) {
        let call = Call::new(
            function.to_owned(),
            named
                .iter()
                .map(|(parameter, value)| CallArgument {
                    parameter: (*parameter).to_owned(),
                    value: value.clone(),
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
        )
        .expect("the request completes or stops incomplete");
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
        let rendered = convert_outcome(evaluation, unit.package().graph(), &sources)
            .expect("the evaluation renders");
        assert_eq!(
            document_outcome(&ran),
            document_outcome(&rendered),
            "{function}"
        );
        assert_eq!(document_outcome(&ran), expected, "{function} {kind}");
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

/// FR-279-AC-4: a prepare limit naming a setting that is not a prepare
/// setting, or a counter past `u64`, is a backend fault, never a prepare limit
/// of another setting.
#[trace("TC-760", "FR-279-AC-4")]
#[test]
fn a_limit_that_is_not_a_prepare_limit_is_a_fault() {
    let unit = checked();
    for limit in [
        LimitExceeded::new(Setting::S1InputBytes, 1, 2),
        LimitExceeded::new(Setting::PrepareCodeBytes, 1, u128::from(u64::MAX) + 1),
    ] {
        let mut aot = Stub::new();
        aot.refuse = Some(PrepareFailure::Limit(limit));
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
        .expect_err("the limit is not a prepare limit");
        assert!(matches!(failure, CallFailure::Fault(_)), "{failure:?}");
    }
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

/// FR-262-AC-1's function, named `tally` because `count` is a keyword of the
/// grammar, and the entries FR-294-AC-7 prepares: one whose
/// first untranslatable construct is a `let`, one a `Decimal`, one an
/// application of `id`, and one with a `let` after that application.
const TRANSLATED: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"quire.value.complete/v1\";\n\
type Digit = Int[0, 9];\n\
function tally using v(n: Int[0, 1000000]): Integer pure decreases(n) \
{ if n = 0 then 0 else tally(n - 1) + 1 }\n\
function id using v(x: Digit): Digit pure { x }\n\
function with_let using v(a: Digit): Boolean pure { let x = a in x = a }\n\
function with_decimal using v(d: Decimal[-100, 100; 2, 2; exact]): Decimal[-100, 100; 2, 2; exact] \
pure { d }\n\
function with_call using v(a: Digit): Digit pure { id(a) }\n\
function call_then_let using v(a: Digit): Digit pure { id(let x = a in x) }\n";

/// The bytes one generated block holds per translated node.
const BLOCK_BYTES: u64 = 4;

fn translated() -> front_end::CheckedUnit {
    checked_source("translated.native", TRANSLATED)
}

/// What the translator did, in order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Step {
    ChargedNode,
    TranslatedNode,
    ChargedBlock(u64),
    KeptBlock(u64),
}

/// A test backend that follows "Prepare charges": it walks each selected
/// entry's checked body in node order, charges one work unit before it
/// translates each node and the byte length of each generated block before it
/// keeps the block, and lists the entry unsupported at the first construct it
/// does not translate.
struct Translator {
    /// The function whose application the backend does not implement.
    refuses_operation: Option<quire_exact::NodeKey>,
    /// Charges admitted so far, visible to a second thread.
    observed: Arc<AtomicUsize>,
    /// Wait for a cancel after this many admitted charges.
    pause_at: Option<usize>,
    log: Mutex<Vec<Step>>,
}

impl Translator {
    fn new() -> Self {
        Self {
            refuses_operation: None,
            observed: Arc::new(AtomicUsize::new(0)),
            pause_at: None,
            log: Mutex::new(Vec::new()),
        }
    }

    fn steps(&self) -> Vec<Step> {
        self.log.lock().expect("the log lock").clone()
    }

    /// The first construct, in node order, this backend does not translate.
    fn refusal(
        &self,
        graph: &qsl_semantics::check::CheckedGraph,
        node: qsl_semantics::check::CheckedNode<'_>,
    ) -> Option<UntranslatableConstruct> {
        use quire_checked::node::NodeKind;
        if matches!(node.kind(), NodeKind::Let { .. }) {
            return Some(UntranslatableConstruct::ExpressionForm("let".to_owned()));
        }
        if matches!(node.value_type(), quire_exact::ValueType::Decimal(_)) {
            return Some(UntranslatableConstruct::ValueType(Box::new(
                node.value_type().clone(),
            )));
        }
        if let NodeKind::Call { function, .. } = node.kind() {
            let identity = graph.function_identity_at(*function)?;
            if self.refuses_operation.as_ref() == Some(&identity) {
                return Some(UntranslatableConstruct::Operation(identity));
            }
        }
        None
    }

    /// Record an admitted charge and the work it paid for.
    fn record(&self, step: Step, effect: Step) {
        let mut log = self.log.lock().expect("the log lock");
        log.push(step);
        log.push(effect);
    }
}

impl ExecutionBackend for Translator {
    type Prepared<'p> = StubPrepared<'p>;

    fn prepare<'p>(
        &self,
        package: &'p CheckedPackage,
        entries: &[EntrySelection],
        limits: &PrepareLimits,
        cancel: &Cancel,
    ) -> Result<Self::Prepared<'p>, PrepareFailure> {
        let graph = package.graph();
        let mut meter = PrepareMeter::new(limits, cancel);
        let mut unsupported = Vec::new();
        for entry in entries {
            let EntrySelection::Function(function) = entry;
            let fault =
                |invariant| PrepareFailure::Fault(InternalFault::new("translator", invariant));
            let identity = graph
                .function_identity(&function.to_string())
                .ok_or_else(|| fault("the-entry-is-declared"))?;
            let state = graph
                .function_by_identity(identity)
                .ok_or_else(|| fault("the-entry-has-a-body"))?;
            let mut nodes = Vec::new();
            let mut pending = vec![state.body];
            let mut refused = None;
            while let Some(node) = pending.pop() {
                if let Some(construct) = self.refusal(graph, node) {
                    refused = Some((construct, node.location().clone()));
                    break;
                }
                meter.charge_node()?;
                let seen = self.observed.fetch_add(1, Ordering::SeqCst) + 1;
                self.record(Step::ChargedNode, Step::TranslatedNode);
                if self.pause_at == Some(seen) {
                    while cancel.cause().is_none() {
                        std::thread::yield_now();
                    }
                }
                nodes.push(node.id());
                pending.extend(node.children().into_iter().rev());
            }
            if let Some((construct, location)) = refused {
                unsupported.push((
                    entry.clone(),
                    UnsupportedEntry {
                        backend: ExecutionBackendChoice::Jit,
                        cause: UnsupportedCause::Untranslatable {
                            construct,
                            location,
                        },
                    },
                ));
                continue;
            }
            for _ in &nodes {
                meter.charge_block(BLOCK_BYTES)?;
                self.observed.fetch_add(1, Ordering::SeqCst);
                self.record(
                    Step::ChargedBlock(BLOCK_BYTES),
                    Step::KeptBlock(BLOCK_BYTES),
                );
            }
        }
        let inner = Interpreter.prepare(package, entries, limits, cancel)?;
        Ok(StubPrepared {
            interpreted: Rc::new(Cell::new(0)),
            inner,
            unsupported,
        })
    }
}

fn count_nodes(unit: &front_end::CheckedUnit) -> u64 {
    let graph = unit.package().graph();
    let identity = graph.function_identity("tally").expect("tally is declared");
    let state = graph
        .function_by_identity(identity)
        .expect("tally has a body");
    u64::try_from(state.body.body().len()).expect("a node count fits u64")
}

fn only(function: &str) -> Vec<EntrySelection> {
    selected(&[function])
}

/// FR-294-AC-6: a backend that follows "Prepare charges", prepared over
/// FR-262-AC-1's recursive function alone, charges one work unit per checked body node
/// it translates, each before that node, and one block charge per generated
/// block equal to its byte length, each before the block is kept. One below
/// either count it returns `Limit` naming the setting, the bound and the
/// count reached, and no prepared value; at the count it returns `Ok`.
/// Cancelled from a second thread after 3 admitted charges it returns
/// `Cancelled(Requested)` with at most one charge after the cancel.
#[trace("TC-778", "FR-294-AC-6")]
#[test]
fn prepare_charges_per_node_and_per_block_and_polls_the_handle() {
    let unit = translated();
    let nodes = count_nodes(&unit);
    assert!(
        nodes > 3,
        "tally has more than the 3 charges the cancel waits for"
    );
    let total = nodes * BLOCK_BYTES;
    let at_count = PrepareLimits {
        compile_work_units: nodes,
        code_bytes: total,
    };
    let entries = only("tally");

    let exact = Translator::new();
    exact
        .prepare(unit.package(), &entries, &at_count, &Cancel::new())
        .expect("at exactly each count prepare succeeds");
    let steps = exact.steps();
    assert_eq!(steps.len() as u64, 2 * (nodes + nodes));
    for pair in steps.chunks(2) {
        assert!(
            matches!(
                pair,
                [Step::ChargedNode, Step::TranslatedNode]
                    | [
                        Step::ChargedBlock(BLOCK_BYTES),
                        Step::KeptBlock(BLOCK_BYTES)
                    ]
            ),
            "a charge precedes its node or block: {pair:?}"
        );
    }
    assert_eq!(
        steps.iter().filter(|s| **s == Step::TranslatedNode).count() as u64,
        nodes
    );
    assert_eq!(
        steps
            .iter()
            .filter(|s| matches!(s, Step::KeptBlock(_)))
            .count() as u64,
        nodes
    );

    let below_nodes = Translator::new();
    let failure = below_nodes
        .prepare(
            unit.package(),
            &entries,
            &PrepareLimits {
                compile_work_units: nodes - 1,
                ..at_count
            },
            &Cancel::new(),
        )
        .err()
        .expect("one below the node count stops with no prepared value");
    match failure {
        PrepareFailure::Limit(limit) => {
            assert_eq!(limit.setting(), Setting::PrepareCompileWorkUnits);
            assert_eq!(limit.configured_bound(), nodes - 1);
            assert_eq!(limit.actual(), u128::from(nodes));
        }
        other => panic!("expected the node limit, got {other:?}"),
    }
    assert_eq!(
        below_nodes
            .steps()
            .iter()
            .filter(|s| **s == Step::TranslatedNode)
            .count() as u64,
        nodes - 1,
        "the refused charge translated nothing"
    );

    let below_bytes = Translator::new();
    let failure = below_bytes
        .prepare(
            unit.package(),
            &entries,
            &PrepareLimits {
                code_bytes: total - 1,
                ..at_count
            },
            &Cancel::new(),
        )
        .err()
        .expect("one below the byte total stops with no prepared value");
    match failure {
        PrepareFailure::Limit(limit) => {
            assert_eq!(limit.setting(), Setting::PrepareCodeBytes);
            assert_eq!(limit.configured_bound(), total - 1);
            assert_eq!(limit.actual(), u128::from(total));
        }
        other => panic!("expected the byte limit, got {other:?}"),
    }

    let mut paused = Translator::new();
    paused.pause_at = Some(3);
    let cancel = Cancel::new();
    let result = std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            paused
                .prepare(unit.package(), &entries, &at_count, &cancel)
                .err()
        });
        while paused.observed.load(Ordering::SeqCst) < 3 {
            std::thread::yield_now();
        }
        cancel.cancel(CancelCause::Requested);
        worker.join().expect("the worker finishes")
    });
    assert_eq!(
        result,
        Some(PrepareFailure::Cancelled(CancelCause::Requested))
    );
    assert!(paused.observed.load(Ordering::SeqCst) <= 4);
}

/// FR-294-AC-7: a backend standing in for `jit` that does not translate
/// `let`, has no representation for `Decimal` and does not implement an
/// application of `id`, prepared over four entries, lists the first
/// untranslatable construct of each, in node order, with that construct's own
/// location; the entry with a `let` after the application names the
/// application.
#[trace("TC-778", "FR-294-AC-7")]
#[test]
fn the_first_untranslatable_construct_of_each_entry_is_named() {
    let unit = translated();
    let graph = unit.package().graph();
    let operation = graph.function_identity("id").expect("id is declared");
    let mut jit = Translator::new();
    jit.refuses_operation = Some(operation.clone());
    let entries = selected(&["with_let", "with_decimal", "with_call", "call_then_let"]);
    let prepared = jit
        .prepare(
            unit.package(),
            &entries,
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .expect("an unsupported entry does not fail prepare");
    let listed = prepared.unsupported();
    assert_eq!(listed.len(), 4);
    let decimal = graph
        .function_by_identity(
            graph
                .function_identity("with_decimal")
                .expect("with_decimal"),
        )
        .expect("with_decimal has a body")
        .body
        .value_type()
        .clone();
    let expected = [
        UntranslatableConstruct::ExpressionForm("let".to_owned()),
        UntranslatableConstruct::ValueType(Box::new(decimal)),
        UntranslatableConstruct::Operation(operation.clone()),
        UntranslatableConstruct::Operation(operation),
    ];
    let mut locations = Vec::new();
    for ((entry, unsupported), (selected_entry, construct)) in
        listed.iter().zip(entries.iter().zip(expected))
    {
        assert_eq!(entry, selected_entry);
        assert_eq!(unsupported.backend, ExecutionBackendChoice::Jit);
        let UnsupportedCause::Untranslatable {
            construct: named,
            location,
        } = &unsupported.cause
        else {
            panic!("expected Untranslatable, got {unsupported:?}");
        };
        assert_eq!(*named, construct);
        locations.push(location.clone());
    }
    // Each location is that construct's own: the `with_call` and `call_then_let`
    // applications sit in different bodies, so no two entries share one.
    for (index, location) in locations.iter().enumerate() {
        assert!(
            locations[index + 1..].iter().all(|other| other != location),
            "entry {index} shares its location"
        );
    }
    let payload = listed[2].1.payload();
    assert_eq!(payload.construct, Some("operation"));
    assert_eq!(payload.location, Some(&locations[2]));
}

/// FR-279, FR-255: the caller's `prepare` limits on the `Call` reach the
/// supplied backend through the spine.
#[trace("TC-760", "FR-279-AC-4")]
#[test]
fn the_calls_prepare_limits_reach_the_supplied_backend() {
    let unit = translated();
    let nodes = count_nodes(&unit);
    let jit = Translator::new();
    let backends = SuppliedBackends {
        aot: None,
        jit: Some(&jit),
    };
    let spine = |prepare: PrepareLimits| {
        let mut call = Call::new(
            "tally".to_owned(),
            vec![CallArgument {
                parameter: "n".to_owned(),
                value: integer(3),
            }],
            default_accounting(1_000_000),
            Default::default(),
        );
        call.backend = ExecutionBackendChoice::Jit;
        call.prepare = prepare;
        run_with_backends(
            SourceIdentity::new("agent-ix", "execution-seam"),
            "fixture".to_owned(),
            "translated.native",
            TRANSLATED.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
            &call,
            &ObjectEnvironment::default(),
            &backends,
            &Cancel::new(),
        )
    };
    let refusal = spine(PrepareLimits {
        compile_work_units: nodes - 1,
        ..PrepareLimits::default()
    })
    .expect_err("one node below the count refuses");
    match *refusal {
        RunRefusal::PrepareLimit(reached) => {
            assert_eq!(reached.setting, PrepareSetting::CompileWorkUnits);
            assert_eq!(reached.bound, nodes - 1);
            assert_eq!(reached.actual, nodes);
        }
        other => panic!("expected PrepareLimit, got {other:?}"),
    }
    let refusal = spine(PrepareLimits {
        code_bytes: nodes * BLOCK_BYTES - 1,
        ..PrepareLimits::default()
    })
    .expect_err("one byte below the total refuses");
    assert!(matches!(
        *refusal,
        RunRefusal::PrepareLimit(PrepareLimitReached {
            setting: PrepareSetting::CodeBytes,
            ..
        })
    ));
    let completed = spine(PrepareLimits {
        compile_work_units: nodes,
        code_bytes: nodes * BLOCK_BYTES,
    })
    .expect("at the counts the call completes");
    assert!(
        matches!(completed, CallOutcome::Completed(CallValue::Integer(ref v)) if v.to_string() == "3"),
        "{completed:?}"
    );
}

/// FR-294, FR-279: the native result's refusal record carries the
/// untranslatable construct's `backend`, `construct`, `construct_name` and
/// `location`, and a reached prepare limit's `setting`, `bound` and `actual`.
#[trace("TC-778", "FR-294-AC-7")]
#[test]
fn the_native_result_renders_the_unsupported_and_prepare_limit_refusals() {
    let report = |refusal: RunRefusal| crate::spine::ClauseRunReport {
        usage: Default::default(),
        basis: crate::proof_result::SettlementBasis::Unavailable,
        witness: None,
        consequent_source_events: Default::default(),
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection: None,
        refused_at: None,
        ledger: None,
        disposition: crate::spine::ClauseDisposition::ArgumentRefusal(Box::new(refusal)),
    };
    let fields = |refusal: RunRefusal| -> serde_json::Value {
        let bytes = crate::spine::write_clause_run_result(&report(refusal)).expect("writes");
        let document: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        document["disposition"]["refusal"].clone()
    };
    let unsupported = fields(RunRefusal::Unsupported(UnsupportedEntry {
        backend: ExecutionBackendChoice::Jit,
        cause: UnsupportedCause::Untranslatable {
            construct: UntranslatableConstruct::ExpressionForm("let".to_owned()),
            location: location(),
        },
    }));
    assert_eq!(unsupported["code"], "unimplemented_capability");
    assert_eq!(unsupported["fields"]["backend"], "jit");
    assert_eq!(unsupported["fields"]["construct"], "expression-form");
    assert_eq!(unsupported["fields"]["construct_name"], "let");
    let rendered: serde_json::Value = serde_json::from_str(
        unsupported["fields"]["location"]
            .as_str()
            .expect("a location field"),
    )
    .expect("the location is JSON");
    assert_eq!(
        rendered,
        serde_json::json!({
            "origin": {"kind": "body", "function": "id", "index": 2},
            "path": [],
        })
    );
    let limit = fields(RunRefusal::PrepareLimit(PrepareLimitReached {
        setting: PrepareSetting::CodeBytes,
        bound: 1,
        actual: 2,
    }));
    assert_eq!(limit["code"], "stage_limit_exceeded");
    assert_eq!(limit["fields"]["setting"], "prepare.code_bytes");
    assert_eq!(limit["fields"]["bound"], "1");
    assert_eq!(limit["fields"]["actual"], "2");
}

/// FR-294-AC-2: the control for the `compile_fail` doctests in
/// `qsl_eval::execution`: a checked package, the only input `prepare`
/// admits, prepares.
#[trace("TC-778", "FR-294-AC-2")]
#[test]
fn prepare_admits_a_checked_package() {
    let unit = checked();
    assert!(Interpreter
        .prepare(
            unit.package(),
            &selected(&["seven"]),
            &PrepareLimits::default(),
            &Cancel::new(),
        )
        .is_ok());
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
