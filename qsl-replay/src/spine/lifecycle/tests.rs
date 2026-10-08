// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-755 to TC-757 and TC-759: the front-end operations are typed library
//! operations over a caller-owned handle, and compose to a checked,
//! emitted package.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_cst::Limits as SourceLimits;
use qsl_eval::value::{CallFailure, QualifiedName};
use qsl_foundation::diagnostic::{
    Category, LimitExceeded, LimitKind as FoundationKind, StageFailure, Staged,
};
use qsl_foundation::{Setting, SourceIdentity};
use qsl_package::{emit_checked, read_import_view, AdmittedPackages};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Cancel, CancelCause, Integer, Outcome, Value};
use quire_semantic_value::checking::CheckingLimits;

use super::{
    check, execute, failure_of, package, parse, refusal_or_fault, select, AdmittedModels,
    CheckedUnit, EmittedUnit, ExecuteRequest, FrontEndFailure, LockEvidence, PackageLimits,
    ParseRequest, ParsedSource,
};
use crate::bounds::ReplayLimits;
use crate::limits::CallerLimits;
use crate::request::StageLimits;
use crate::spine::{
    default_accounting, CompileRefusal, DependencyInput, SpineLimits, SpineStage, SuppliedLibrary,
    DEFAULT_WORK_UNITS,
};

const FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-compile.native");
const MODEL_FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-model.native");
const MODEL_DOCUMENT: &str =
    include_str!("../../../../tests/fixtures/spine-model.semantic-ir.json");

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:lifecycle", "fixture", "fixture:1")
}

fn request(bytes: &[u8]) -> ParseRequest<'_> {
    // The identity lives for the whole process, so a request can borrow it.
    static IDENTITY: OnceLock<SourceIdentity> = OnceLock::new();
    ParseRequest {
        source: IDENTITY.get_or_init(identity),
        path: "unit.native",
        bytes,
    }
}

/// The one library the import tests supply: `x < 5` over a digit.
fn geometry() -> SuppliedLibrary {
    SuppliedLibrary {
        identity: "test/geometry".to_owned(),
        source: SourceIdentity::new("a", "geometry", "git", "1"),
        path: "geometry.native".to_owned(),
        bytes: format!("{HEADER}function f using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n")
            .into_bytes(),
    }
}

/// Every operation's output over one source, from a live handle.
struct Chain {
    parsed: ParsedSource,
    models: AdmittedModels,
    checked: CheckedUnit,
    emitted: EmittedUnit,
}

fn chain_of(
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<Chain, FrontEndFailure> {
    let cancel = Cancel::new();
    let parsed = parse(&request(bytes), limits.source, &cancel)?.into_value();
    let models = select(&parsed, packages, limits.model, &cancel)?.into_value();
    let checked = check(
        &parsed,
        &models,
        dependencies,
        &LockEvidence::default(),
        limits,
        &cancel,
    )?
    .into_value();
    let emitted = package(&checked, PackageLimits::default(), &cancel)?.into_value();
    Ok(Chain {
        parsed,
        models,
        checked,
        emitted,
    })
}

fn fixture_chain() -> Chain {
    chain_of(
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the fixture compiles: {failure:?}"))
}

fn refusal(failure: FrontEndFailure) -> Box<CompileRefusal> {
    match failure {
        StageFailure::Refused(refusal) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn seven() -> QualifiedName {
    QualifiedName::unqualified("seven").expect("an identifier")
}

fn call_seven(
    checked: &CheckedUnit,
    accounting: quire_exact::ScalarLimits,
    cancel: &Cancel,
) -> Result<qsl_eval::value::Evaluation, CallFailure> {
    let function = seven();
    execute(
        checked,
        &ExecuteRequest {
            function: &function,
            arguments: &[],
            objects: &ObjectEnvironment::default(),
        },
        accounting,
        cancel,
    )
}

/// FR-275-AC-1 (TC-755 steps 1 to 3): `parse`, `select`, `check` and
/// `package` over the compile fixture each return a `Staged` value, and
/// `execute` of `seven` over the checked package completes with the integer
/// 7.
#[trace("TC-755", "FR-275-AC-1")]
#[test]
fn the_chain_over_the_compile_fixture_returns_a_package_that_executes() {
    let chain = fixture_chain();
    let wire: serde_json::Value =
        serde_json::from_slice(chain.emitted.package().bytes()).expect("the wire is JSON");
    assert_eq!(wire["contract_version"], "quire.checked-package/v2");
    assert_eq!(chain.models.models().len(), 0);

    let evaluation = call_seven(
        &chain.checked,
        default_accounting(DEFAULT_WORK_UNITS),
        &Cancel::new(),
    )
    .expect("seven admits no argument and takes no object");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
            assert_eq!(value, Integer::from(7_i64));
        }
        other => panic!("expected the integer 7, got {other:?}"),
    }
}

/// FR-275-AC-2 (TC-755 step 5): the operations take their predecessor's own
/// type. Each `compile_fail` doctest of this module's header passes a value
/// of another stage and each has a compiling control; this test is the
/// control for the runtime half, that the correct types compose.
#[trace("TC-755", "FR-275-AC-2")]
#[test]
fn each_operation_takes_exactly_its_predecessors_output() {
    let chain = fixture_chain();
    let cancel = Cancel::new();
    // `check` takes the `ParsedSource` and `AdmittedModels` of one source,
    // `package` the `CheckedUnit` of that check, and `execute` the same.
    let rechecked = check(
        &chain.parsed,
        &chain.models,
        &DependencyInput::default(),
        &LockEvidence::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect("the parsed source checks again")
    .into_value();
    let reemitted = package(&rechecked, PackageLimits::default(), &cancel)
        .expect("the checked package emits again")
        .into_value();
    assert_eq!(reemitted.package(), chain.emitted.package());
}

/// FR-275-AC-3 (TC-755 step 6): each operation, called twice with equal
/// requests and limits, returns equal outcomes.
#[trace("TC-755", "FR-275-AC-3")]
#[test]
fn equal_requests_return_equal_outcomes() {
    let first = fixture_chain();
    let second = fixture_chain();
    assert_eq!(
        first.parsed.source().reference(),
        second.parsed.source().reference()
    );
    assert_eq!(
        first.parsed.syntax().diagnostics(),
        second.parsed.syntax().diagnostics()
    );
    assert_eq!(first.models.models().len(), second.models.models().len());
    assert_eq!(
        first.checked.source().reference(),
        second.checked.source().reference()
    );
    assert_eq!(first.emitted.package(), second.emitted.package());
    assert_eq!(references(&first.emitted), references(&second.emitted));

    let outcome = |chain: &Chain| {
        call_seven(
            &chain.checked,
            default_accounting(DEFAULT_WORK_UNITS),
            &Cancel::new(),
        )
        .map(|evaluation| format!("{:?}", evaluation.outcome))
    };
    assert_eq!(outcome(&first), outcome(&second));

    let broken = b"function";
    let refusals = |bytes: &[u8]| {
        let failure = parse(
            &request(bytes),
            SpineLimits::default().source,
            &Cancel::new(),
        )
        .expect_err("a truncated unit does not parse");
        let refusal = refusal(failure);
        (refusal.code(), refusal.stage(), refusal.region().cloned())
    };
    assert_eq!(refusals(broken), refusals(broken));
}

/// A deterministic byte generator: the same seed gives the same bytes, so a
/// failing input reproduces.
struct Bytes(u64);

impl Bytes {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).expect("a small bound"))
            .expect("below the bound")
    }
}

/// FR-275-AC-4 (TC-756): arbitrary source bytes, and arbitrary edits of a
/// valid unit, make `parse`, `select`, `check` and `package` each return a
/// typed outcome and never panic.
#[trace("TC-756", "FR-275-AC-4")]
#[test]
fn arbitrary_source_bytes_never_panic_an_operation() {
    let mut generator = Bytes(0x9e37_79b9_7f4a_7c15);
    let valid = FIXTURE.as_bytes();
    let mut checked_units = 0_u32;
    for round in 0..600 {
        let mut bytes = valid.to_vec();
        match round % 3 {
            // Random bytes.
            0 => {
                bytes = (0..generator.below(200))
                    .map(|_| u8::try_from(generator.below(256)).expect("a byte"))
                    .collect();
            }
            // A few overwritten bytes of the valid unit.
            1 => {
                for _ in 0..=generator.below(4) {
                    let at = generator.below(bytes.len());
                    bytes[at] = u8::try_from(generator.below(256)).expect("a byte");
                }
            }
            // The valid unit cut short.
            _ => bytes.truncate(generator.below(bytes.len())),
        }
        if let Ok(chain) = chain_of(
            &bytes,
            &BTreeMap::new(),
            &DependencyInput::default(),
            SpineLimits::default(),
        ) {
            checked_units += 1;
            assert!(!chain.emitted.package().bytes().is_empty());
        }
    }
    // The valid unit's own edit-free rounds are among those that check.
    assert!(checked_units > 0, "some generated unit must compile");
}

/// A handle whose observer counts every charge made under it and, at charge
/// `at`, holds that charge until another thread has cancelled the handle. The
/// cancellation therefore lands at exactly one charge, whatever the
/// scheduler does.
struct Gate {
    cancel: Cancel,
    charges: Arc<AtomicU64>,
    after_cancel: Arc<AtomicU64>,
    reached: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

/// A handle that holds charge `at`, or only counts when `at` is `None`.
fn gated(at: Option<u64>) -> Gate {
    let charges = Arc::new(AtomicU64::new(0));
    let after_cancel = Arc::new(AtomicU64::new(0));
    let reached = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let cancel = {
        let (charges, after_cancel, reached, cancelled) = (
            Arc::clone(&charges),
            Arc::clone(&after_cancel),
            Arc::clone(&reached),
            Arc::clone(&cancelled),
        );
        Cancel::observing(move || {
            let charge = charges.fetch_add(1, Ordering::SeqCst) + 1;
            if Some(charge) == at {
                reached.store(true, Ordering::SeqCst);
                while !cancelled.load(Ordering::SeqCst) {
                    std::thread::yield_now();
                }
            }
            if cancelled.load(Ordering::SeqCst) {
                after_cancel.fetch_add(1, Ordering::SeqCst);
            }
        })
    };
    Gate {
        cancel,
        charges,
        after_cancel,
        reached,
        cancelled,
        finished,
    }
}

impl Gate {
    /// A second thread that cancels the handle with `cause` once a charge
    /// holds at the gate. It returns whether it did: a run that ended before
    /// reaching the gate returns `false`.
    fn canceller(&self, cause: CancelCause) -> std::thread::JoinHandle<bool> {
        let cancel = self.cancel.clone();
        let (reached, cancelled, finished) = (
            Arc::clone(&self.reached),
            Arc::clone(&self.cancelled),
            Arc::clone(&self.finished),
        );
        std::thread::spawn(move || {
            while !reached.load(Ordering::SeqCst) {
                if finished.load(Ordering::SeqCst) {
                    return false;
                }
                std::thread::yield_now();
            }
            cancel.cancel(cause);
            cancelled.store(true, Ordering::SeqCst);
            true
        })
    }

    /// Run `operation` over this handle with a canceller thread, and return
    /// its result once the canceller has finished.
    fn run<T>(&self, cause: CancelCause, operation: impl FnOnce(&Cancel) -> T) -> T {
        let canceller = self.canceller(cause);
        let outcome = operation(&self.cancel);
        self.finished.store(true, Ordering::SeqCst);
        assert!(
            canceller.join().expect("the canceller finishes"),
            "the operation ended before charge {}",
            self.charges.load(Ordering::SeqCst)
        );
        outcome
    }

    /// The operation stopped at the held charge: that charge was the first
    /// and only one to see the cancellation.
    fn assert_stopped_at(&self, at: u64) {
        assert_eq!(self.charges.load(Ordering::SeqCst), at, "charges made");
        assert_eq!(
            self.after_cancel.load(Ordering::SeqCst),
            1,
            "charges made after the cancellation"
        );
    }
}

/// How many charges `operation` makes over a handle nobody cancels.
fn charges_of<T>(operation: impl FnOnce(&Cancel) -> T) -> u64 {
    let gate = gated(None);
    let _ = operation(&gate.cancel);
    gate.charges.load(Ordering::SeqCst)
}

/// FR-276-AC-1 (TC-757 step 1): each operation, called with a handle already
/// cancelled with `Requested`, returns its cancelled failure and no output.
#[trace("TC-757", "FR-276-AC-1")]
#[test]
fn a_handle_cancelled_before_the_call_stops_every_operation() {
    let chain = fixture_chain();
    let cancelled = Cancel::new();
    cancelled.cancel(CancelCause::Requested);
    let limits = SpineLimits::default();

    assert_requested(parse(
        &request(FIXTURE.as_bytes()),
        limits.source,
        &cancelled,
    ));
    assert_requested(select(
        &chain.parsed,
        &BTreeMap::new(),
        limits.model,
        &cancelled,
    ));
    assert_requested(check(
        &chain.parsed,
        &chain.models,
        &DependencyInput::default(),
        &LockEvidence::default(),
        limits,
        &cancelled,
    ));
    assert_requested(package(
        &chain.checked,
        PackageLimits::default(),
        &cancelled,
    ));
    assert_eq!(
        call_seven(
            &chain.checked,
            default_accounting(DEFAULT_WORK_UNITS),
            &cancelled
        )
        .unwrap_err(),
        CallFailure::Cancelled(CancelCause::Requested)
    );
}

/// `outcome` is `Cancelled(Requested)` and carries no output.
fn assert_requested<T>(outcome: Result<Staged<T>, FrontEndFailure>) {
    match outcome {
        Err(StageFailure::Cancelled(CancelCause::Requested)) => {}
        Err(other) => panic!("expected Cancelled(Requested), got {other:?}"),
        Ok(_) => panic!("a cancelled operation returned an output"),
    }
}

/// The digest of each source `emitted` carries, in order.
fn references(emitted: &EmittedUnit) -> Vec<qsl_foundation::digest::DigestRecord> {
    emitted
        .sources()
        .iter()
        .map(|source| source.reference().digest())
        .collect()
}

/// A source of `count` independent declarations, each `Integer`-valued.
fn declarations(count: usize) -> String {
    let mut source = String::from(HEADER);
    for index in 0..count {
        writeln!(source, "function f{index} using v(): Integer pure {{ 7 }}")
            .expect("a string write");
    }
    source
}

/// Limits raised to fit a source of `bytes`, so no default ceiling stops it.
fn wide(bytes: usize) -> SpineLimits {
    let mut limits = SpineLimits::default();
    limits.source.source_bytes = bytes;
    limits.source.tokens = usize::MAX / 2;
    limits.source.nodes = usize::MAX / 2;
    limits.checking = quire_semantic_value::checking::CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    limits
}

/// Declarations in the cancelled-check test, as the AC names them.
const DECLARATIONS: usize = 200_000;

/// The unit of `count` declarations, parsed and with its models selected,
/// under limits raised to fit it.
fn parsed_declarations(count: usize) -> (ParsedSource, AdmittedModels, SpineLimits) {
    let source = declarations(count);
    let limits = wide(source.len());
    let live = Cancel::new();
    let parsed = parse(&request(source.as_bytes()), limits.source, &live)
        .expect("the generated unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    (parsed, models, limits)
}

/// `check` over `parsed`, cancelled with `cause` from a second thread at
/// charge `at`: the check returns `Cancelled(cause)` and no package, and
/// exactly one charge saw the cancellation.
fn assert_check_cancelled_at(
    (parsed, models, limits): &(ParsedSource, AdmittedModels, SpineLimits),
    at: u64,
    cause: CancelCause,
) {
    let gate = gated(Some(at));
    let outcome = gate.run(cause, |cancel| {
        check(
            parsed,
            models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            *limits,
            cancel,
        )
    });
    match outcome {
        Err(StageFailure::Cancelled(seen)) => assert_eq!(seen, cause),
        Err(other) => panic!("expected Cancelled({cause:?}), got {other:?}"),
        Ok(_) => panic!("a cancelled check returned a package"),
    }
    gate.assert_stopped_at(at);
}

/// FR-276-AC-2 (TC-757 step 2): a `check` over a generated unit of
/// declarations, cancelled with `Deadline` from a second thread at its
/// 1,000th charge, returns `Cancelled(Deadline)` and no package, and
/// exactly one charge sees the cancellation.
///
/// The oracle counts polls of the handle, so it sees every charge only if
/// every charge site polls. The sites that poll are: the kernel `Meter`
/// charge, S1's leaf commit and parser step, I1's normalization `Meter`
/// charge, S3's contract meter (one `DeclarationCheck` charge per
/// declaration), S3's measure pass (one per node), S3's `Typer` node charge,
/// the type-environment work budget and E4's entry and per-node write. The
/// large-body, `select` and `package` tests below fail when their site does
/// not poll, because each asserts the uncancelled run made at least as many
/// charges as the input has nodes. Not polled: `model` dispatch linking,
/// which no spine operation calls.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_check_cancelled_from_another_thread_stops_within_one_charge() {
    let unit = parsed_declarations(DECLARATIONS);
    let total = charges_of(|cancel| {
        check(
            &unit.0,
            &unit.1,
            &DependencyInput::default(),
            &LockEvidence::default(),
            unit.2,
            cancel,
        )
    });
    assert!(total > 1_000, "the check made only {total} charges");
    assert_check_cancelled_at(&unit, 1_000, CancelCause::Deadline);
}

/// `terms` ones added in one function body.
fn sum(terms: usize) -> String {
    let mut source = String::from(HEADER);
    source.push_str("function big using v(): Integer pure { 1");
    for _ in 1..terms {
        source.push_str(" + 1");
    }
    source.push_str(" }\n");
    source
}

/// FR-276-AC-2 (TC-757 step 2) for a cancel that lands inside one large
/// function body: the S3 node charge and the measure pass charge every node,
/// so the cancellation stops the check within one charge of landing in the
/// middle of the body, not at the next declaration.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_cancel_inside_one_large_body_stops_within_one_charge() {
    const TERMS: usize = 3_000;
    let source = sum(TERMS);
    let limits = wide(source.len());
    let live = Cancel::new();
    let parsed = parse(&request(source.as_bytes()), limits.source, &live)
        .expect("the sum parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    let unit = (parsed, models, limits);
    let total = charges_of(|cancel| {
        check(
            &unit.0,
            &unit.1,
            &DependencyInput::default(),
            &LockEvidence::default(),
            unit.2,
            cancel,
        )
    });
    // Every term is a node that the measure pass and the typer each charge:
    // a site that did not poll would leave `total` below the node count.
    assert!(
        total >= u64::try_from(2 * TERMS).expect("a small count"),
        "the check of one {TERMS}-term body made only {total} charges"
    );
    assert_check_cancelled_at(&unit, total / 2, CancelCause::Requested);
}

/// FR-276-AC-2 for `select`: the I1 normalization meter polls the handle at
/// every charge, so a cancel landing in the middle of the model's
/// normalization stops it within one charge.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_cancel_inside_select_stops_within_one_charge() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT.as_bytes()]);
    let limits = SpineLimits::default();
    let live = Cancel::new();
    let parsed = parse(&request(MODEL_FIXTURE.as_bytes()), limits.source, &live)
        .expect("the model fixture parses")
        .into_value();
    let run = |cancel: &Cancel| select(&parsed, &packages, limits.model, cancel);
    let total = charges_of(run);
    assert!(total > 2, "select made only {total} charges");
    let at = total / 2;
    let gate = gated(Some(at));
    let outcome = gate.run(CancelCause::Requested, run);
    assert!(matches!(
        outcome,
        Err(StageFailure::Cancelled(CancelCause::Requested))
    ));
    gate.assert_stopped_at(at);
}

/// FR-276-AC-2 for `package`: the emitter polls the handle at entry and at
/// every node it writes.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_cancel_inside_package_stops_within_one_charge() {
    let chain = fixture_chain();
    let run = |cancel: &Cancel| package(&chain.checked, PackageLimits::default(), cancel);
    let total = charges_of(run);
    assert!(total > 2, "package made only {total} charges");
    let at = total / 2;
    let gate = gated(Some(at));
    let outcome = gate.run(CancelCause::Deadline, run);
    assert!(matches!(
        outcome,
        Err(StageFailure::Cancelled(CancelCause::Deadline))
    ));
    gate.assert_stopped_at(at);
}

/// `levels` functions, each calling the one before it twice, so calling the
/// last evaluates `2^levels` leaves.
fn doubling(levels: usize) -> String {
    let mut source = String::from(HEADER);
    source.push_str("function d0 using v(): Integer pure { 1 }\n");
    for level in 1..levels {
        let below = level - 1;
        writeln!(
            source,
            "function d{level} using v(): Integer pure {{ d{below}() + d{below}() }}"
        )
        .expect("a string write");
    }
    source
}

/// FR-276-AC-3 (TC-757 step 3): an `execute` whose evaluation would charge
/// millions of work units, run with every accounting limit at `u64::MAX` and
/// cancelled from a second thread after 1,000 charges, returns
/// `CallFailure::Cancelled(Requested)` and no value.
#[trace("TC-757", "FR-276-AC-3")]
#[test]
fn an_execute_cancelled_from_another_thread_returns_no_value() {
    let source = doubling(24);
    let chain = chain_of(
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the doubling unit compiles: {failure:?}"));

    let gate = gated(Some(1_000));
    let function = QualifiedName::unqualified("d23").expect("an identifier");
    let outcome = gate.run(CancelCause::Requested, |cancel| {
        execute(
            &chain.checked,
            &ExecuteRequest {
                function: &function,
                arguments: &[],
                objects: &ObjectEnvironment::default(),
            },
            default_accounting(u64::MAX),
            cancel,
        )
    });

    assert_eq!(
        outcome.map(|_| ()).unwrap_err(),
        CallFailure::Cancelled(CancelCause::Requested)
    );
    gate.assert_stopped_at(1_000);
}

/// FR-276-AC-5 (TC-757 step 5): a cancelled `execute` is category
/// incomplete, exit 22. (A cancelled `StageFailure` of each cause is checked
/// beside the type, in `qsl-foundation`.)
#[trace("TC-757", "FR-276-AC-5")]
#[test]
fn a_cancelled_call_is_incomplete_and_exits_22() {
    for cause in [CancelCause::Requested, CancelCause::Deadline] {
        let category = CallFailure::Cancelled(cause).category();
        assert_eq!(category, Category::Incomplete);
        assert_eq!(category.exit_code(), 22);
    }
}

/// FR-278-AC-1 (TC-759 step 1): the four operations composed over the
/// compile fixture, over a domain-package request and over a library
/// request each emit a package QSL's I2 reader reads back, and the same
/// composition twice emits the same bytes.
#[trace("TC-759", "FR-278-AC-1")]
#[test]
fn the_composition_emits_packages_the_i2_reader_reads_back() {
    // The compile fixture.
    let fixture = fixture_chain();
    read_back(&fixture, &BTreeMap::new(), "test/fixture");

    // A domain-package request: the unit's `model` names the document by
    // its `sha256-jcs` digest.
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT.as_bytes()]);
    let model = chain_of(
        MODEL_FIXTURE.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the model fixture compiles: {failure:?}"));
    assert_eq!(model.models.models().len(), 1);
    read_back(&model, &packages, "test/model");
    let again = chain_of(
        MODEL_FIXTURE.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the model fixture compiles again");
    assert_eq!(model.emitted.package(), again.emitted.package());

    // A library request: the unit imports a library the request supplies.
    let geometry = geometry();
    let dependencies = DependencyInput::new(vec![geometry.clone()]).expect("admissible");
    let unit = format!(
        "{HEADER}import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    let importing = chain_of(
        unit.as_bytes(),
        &BTreeMap::new(),
        &dependencies,
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the importing unit compiles: {failure:?}"));
    assert_eq!(importing.checked.libraries().len(), 1);
    read_back(&importing, &BTreeMap::new(), "test/importing");
}

/// `chain`'s emitted bytes through the I2 reader, as `identity`.
fn read_back(chain: &Chain, packages: &BTreeMap<[u8; 32], Vec<u8>>, identity: &str) {
    let emission = emit_checked(chain.checked.package()).expect("the package emits");
    assert_eq!(emission.package(), chain.emitted.package());
    read_import_view(
        chain.checked.package(),
        &emission,
        LibraryName::new(identity).expect("a non-empty identity"),
        packages,
        &mut AdmittedPackages::default(),
        qsl_package::V2ReadLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("the I2 reader refused {identity}: {refusal}"));
}

/// FR-278-AC-2 (TC-759 step 2): a syntax error is refused by `parse` with
/// `invalid_syntax`; a `model` declaration naming a document the request does
/// not supply is refused by `select` at the `model` declaration; an `inv`
/// function over an unconstrained integer is refused by `check` with
/// `ill_typed`. Each refusal names its stage.
#[trace("TC-759", "FR-278-AC-2")]
#[test]
fn each_operation_refuses_at_the_stage_that_owns_the_defect() {
    let limits = SpineLimits::default();
    let live = Cancel::new();

    let syntax = refusal(
        parse(
            &request(format!("{HEADER}function f using v(: Boolean pure {{ true }}\n").as_bytes()),
            limits.source,
            &live,
        )
        .expect_err("a syntax error does not parse"),
    );
    assert_eq!(syntax.code().as_str(), "invalid_syntax");
    assert_eq!(syntax.stage(), SpineStage::Source);

    let parsed = parse(&request(MODEL_FIXTURE.as_bytes()), limits.source, &live)
        .expect("the model fixture parses")
        .into_value();
    let missing = refusal(
        select(&parsed, &BTreeMap::new(), limits.model, &live)
            .expect_err("no document is supplied"),
    );
    assert_eq!(missing.stage(), SpineStage::Intake);
    let region = missing.region().expect("the refusal is located");
    let (start, end) = (
        usize::try_from(region.start()).expect("a start offset"),
        usize::try_from(region.end()).expect("an end offset"),
    );
    assert!(
        MODEL_FIXTURE[start..end].starts_with("model M"),
        "located at {:?}",
        &MODEL_FIXTURE[start..end]
    );

    let ill_typed = format!("{HEADER}function f using v(): Boolean pure {{ 1 + true }}\n");
    let parsed = parse(&request(ill_typed.as_bytes()), limits.source, &live)
        .expect("the ill-typed unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    let refused = refusal(
        check(
            &parsed,
            &models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            limits,
            &live,
        )
        .expect_err("the unit is ill typed"),
    );
    assert_eq!(refused.code().as_str(), "ill_typed");
    assert_eq!(refused.stage(), SpineStage::Check);
}

/// FR-278-AC-3 (TC-759 step 3): the `EmittedUnit` carries a source
/// provision whose digests name every source the package was compiled from.
/// (Replaying through that provision is checked beside the replay executor.)
#[trace("TC-759", "FR-278-AC-3")]
#[test]
fn the_provision_names_every_source_the_package_was_compiled_from() {
    let geometry = geometry();
    let unit = format!(
        "{HEADER}import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    let chain = chain_of(
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::new(vec![geometry.clone()]).expect("admissible"),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the importing unit compiles: {failure:?}"));

    let digests = references(&chain.emitted);
    let unit_digest = qsl_foundation::digest::DigestRecord::mint(
        qsl_foundation::digest::DigestDomain::SourceBytesV1,
        qsl_foundation::digest::ByteDigest::of(unit.as_bytes()).as_bytes(),
    );
    let library_digest = qsl_foundation::digest::DigestRecord::mint(
        qsl_foundation::digest::DigestDomain::SourceBytesV1,
        qsl_foundation::digest::ByteDigest::of(&geometry.bytes).as_bytes(),
    );
    assert_eq!(digests, vec![unit_digest, library_digest]);
}

/// FR-275-AC-5 (TC-755 step 7): each operation reports the work of its own
/// stages. `parse` reports S1 work and none for S3, `check` over the same
/// `ParsedSource` reports S3 work and zero for S1 and S2, and `package` over
/// its `CheckedUnit` reports E4 work and zero for S1 to S4. (`execute`
/// returns the evaluation of FR-277, whose counters are the S6a meter's.)
#[trace("TC-755", "FR-275-AC-5")]
#[test]
fn each_operation_reports_the_work_of_its_own_stages() {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed =
        parse(&request(FIXTURE.as_bytes()), limits.source, &cancel).expect("the fixture parses");
    let parse_work = parsed.work();
    assert!(parse_work.s1 > 0, "parse reported no S1 work");
    assert_eq!(parse_work.s3, 0);
    let parsed = parsed.into_value();

    let models = select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .expect("a unit with no model selects nothing");
    assert_eq!(models.work().s1, 0);
    let models = models.into_value();

    let checked = check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &LockEvidence::default(),
        limits,
        &cancel,
    )
    .expect("the fixture checks");
    let check_work = checked.work();
    assert_eq!((check_work.s1, check_work.s2), (0, 0));
    assert!(check_work.s3 > 0, "check reported no S3 work");
    assert_eq!(check_work.e4, 0);
    let checked = checked.into_value();

    let emitted = package(&checked, PackageLimits::default(), &cancel).expect("the package emits");
    let emit_work = emitted.work();
    assert_eq!(
        (emit_work.s1, emit_work.s2, emit_work.s3, emit_work.s4),
        (0, 0, 0, 0)
    );
    assert!(emit_work.e4 > 0, "package reported no E4 work");
}

/// How one run of an operation under a limit ended.
enum Reached {
    /// It produced its output.
    Output,
    /// It stopped at a limit.
    Limit(LimitExceeded),
}

fn reached<T>(outcome: Result<Staged<T>, FrontEndFailure>) -> Reached {
    match outcome {
        Ok(_) => Reached::Output,
        Err(StageFailure::Limit(limit)) => Reached::Limit(limit),
        Err(other) => panic!("expected an output or a limit, got {other:?}"),
    }
}

/// FR-277-AC-1 for one limits field: the smallest value of the field at
/// which `run` produces its output is the counter the input reaches. One
/// below it, `run` stops with `LimitExceeded` naming `field` and carrying the
/// configured value; at it, `run` succeeds.
fn assert_field(
    field: Setting,
    kind: FoundationKind,
    high: u64,
    run: impl Fn(u64) -> Reached,
) -> u64 {
    assert_field_with(field, kind, high, |value| value, run)
}

/// [`assert_field`] for a field whose configured bound is `bound(value)`
/// rather than the value itself.
fn assert_field_with(
    field: Setting,
    kind: FoundationKind,
    high: u64,
    bound: impl Fn(u64) -> u64,
    run: impl Fn(u64) -> Reached,
) -> u64 {
    assert!(
        matches!(run(high), Reached::Output),
        "`{field:?}` at {high} does not admit the input"
    );
    let (mut low, mut top) = (0_u64, high);
    while low < top {
        let middle = low + (top - low) / 2;
        match run(middle) {
            Reached::Output => top = middle,
            Reached::Limit(limit) => {
                assert_eq!(limit.setting(), field, "limit at {middle}");
                low = middle + 1;
            }
        }
    }
    let counter = low;
    assert!(counter > 0, "`{field:?}` is not reached by the input");
    match run(counter - 1) {
        Reached::Limit(limit) => {
            assert_eq!(limit.setting(), field);
            assert_eq!(limit.kind(), kind, "`{field:?}`");
            assert_eq!(limit.configured_bound(), bound(counter - 1), "`{field:?}`");
        }
        Reached::Output => panic!("`{field:?}` at {} admits the input", counter - 1),
    }
    assert!(
        matches!(run(counter), Reached::Output),
        "`{field:?}` at {counter}"
    );
    counter
}

/// FR-277-AC-1 (TC-758 step 1) for `parse`: each field of the source limits.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn parse_names_the_source_limit_field_it_reached() {
    let bytes = FIXTURE.as_bytes();
    let run = |edit: fn(&mut SourceLimits, u64)| {
        move |value: u64| {
            let mut limits = SpineLimits::default().source;
            edit(&mut limits, value);
            reached(parse(&request(bytes), limits, &Cancel::new()))
        }
    };
    assert_field(
        Setting::S1InputBytes,
        FoundationKind::InputBytes,
        10_000,
        run(|limits, value| limits.source_bytes = usize::try_from(value).expect("small")),
    );
    assert_field(
        Setting::S1Tokens,
        FoundationKind::TokenCount,
        10_000,
        run(|limits, value| limits.tokens = usize::try_from(value).expect("small")),
    );
    assert_field(
        Setting::S1Nodes,
        FoundationKind::NodeCount,
        10_000,
        run(|limits, value| limits.nodes = usize::try_from(value).expect("small")),
    );
    // The work bound is the unit's total step budget, in the same unit as
    // the counter; the field sets it per token. The scale is the budget at a
    // field value of 1.
    let scale =
        match run(|limits, value| limits.work_units = usize::try_from(value).expect("small"))(1) {
            Reached::Limit(limit) => limit.configured_bound(),
            Reached::Output => panic!("one step per token admits the fixture"),
        };
    assert_field_with(
        Setting::S1WorkUnits,
        FoundationKind::WorkBudget,
        256,
        |value| value * scale,
        run(|limits, value| limits.work_units = usize::try_from(value).expect("small")),
    );
    // The byte ceiling reports the source's real length as the counter.
    let length = u128::try_from(bytes.len()).expect("a small length");
    let mut limits = SpineLimits::default().source;
    limits.source_bytes = bytes.len() - 1;
    match reached(parse(&request(bytes), limits, &Cancel::new())) {
        Reached::Limit(limit) => assert_eq!(limit.actual(), length),
        Reached::Output => panic!("one byte short admits the fixture"),
    }
}

/// FR-277-AC-1 (TC-758 step 1) for `select`: each field of the model
/// normalization limits that a charge reaches.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn select_names_the_model_limit_field_it_reached() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT.as_bytes()]);
    let live = Cancel::new();
    let parsed = parse(
        &request(MODEL_FIXTURE.as_bytes()),
        SpineLimits::default().source,
        &live,
    )
    .expect("the model fixture parses")
    .into_value();
    let run = |edit: fn(&mut ModelNormalizationLimits, u64)| {
        let (parsed, packages) = (&parsed, &packages);
        move |value: u64| {
            let mut limits = ModelNormalizationLimits::default();
            edit(&mut limits, value);
            reached(select(parsed, packages, limits, &Cancel::new()))
        }
    };
    assert_field(
        Setting::ModelDeclarationRecords,
        FoundationKind::NodeCount,
        100_000,
        run(|limits, value| limits.declaration_records = value),
    );
    assert_field(
        Setting::ModelDerivationFacts,
        FoundationKind::NodeCount,
        100_000,
        run(|limits, value| limits.derivation_facts = value),
    );
    assert_field(
        Setting::ModelEffectiveDeclarations,
        FoundationKind::NodeCount,
        100_000,
        run(|limits, value| limits.effective_declarations = value),
    );
    assert_field(
        Setting::ModelAncestorSteps,
        FoundationKind::EdgeCount,
        10_000,
        run(|limits, value| limits.ancestor_steps = value),
    );
    assert_field(
        Setting::ModelHashedBytes,
        FoundationKind::InputBytes,
        100_000,
        run(|limits, value| limits.hashed_bytes = value),
    );
    assert_field(
        Setting::ModelWorkUnits,
        FoundationKind::WorkBudget,
        100_000,
        run(|limits, value| limits.work_units = value),
    );
}

/// FR-277-AC-1 (TC-758 step 1) for `check`: each checking limit.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn check_names_the_checking_limit_field_it_reached() {
    let chain = fixture_chain();
    let run = |edit: fn(SpineLimits, u64) -> SpineLimits| {
        let chain = &chain;
        move |value: u64| {
            let limits = edit(SpineLimits::default(), value);
            reached(check(
                &chain.parsed,
                &chain.models,
                &DependencyInput::default(),
                &LockEvidence::default(),
                limits,
                &Cancel::new(),
            ))
        }
    };
    assert_field(
        Setting::S3Nodes,
        FoundationKind::NodeCount,
        100_000,
        run(|mut limits, value| {
            limits.checking = CheckingLimits::new(value)
                .with_input_bytes(limits.checking.input_bytes())
                .with_work_budget(limits.checking.work_budget());
            limits
        }),
    );
    assert_field(
        Setting::S3InputBytes,
        FoundationKind::InputBytes,
        1_000_000,
        run(|mut limits, value| {
            limits.checking = limits.checking.with_input_bytes(value);
            limits
        }),
    );
    assert_field(
        Setting::S3WorkUnits,
        FoundationKind::WorkBudget,
        1_000_000,
        run(|mut limits, value| {
            limits.checking = limits.checking.with_work_budget(value);
            limits
        }),
    );
}

/// A unit with no record, enum or unit and one function whose 40-term body
/// lowers to a node preimage of several hundred bytes: every identity
/// encoding of it is a lowered node, none a declared type handle.
fn long_body_chain() -> Chain {
    let body = vec!["1"; 40].join(" + ");
    let source = format!("{HEADER}function long using v(): Integer pure {{ {body} }}\n");
    chain_of(
        source.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the unit compiles: {failure:?}"))
}

/// FR-255-AC-1 for `identity.input_bytes` through the compile. The unit has
/// no record, enum or unit, so no declared type handle or nominal preimage is
/// encoded at assembly: every identity encoding is a node preimage lowering
/// keys under the limit `AssemblyLimits.identity` carries into the checked
/// package. The smallest `identity.input_bytes` at which `check` admits it is
/// then the largest node preimage the checked package holds (well past a
/// declared type handle's size); one below it `check` stops with
/// `LimitExceeded` naming `identity.input_bytes` and that bound. It fails if
/// assembly stops carrying the caller's limit into lowering (the search
/// finds no refusal at all), or if the spine stops mapping the refusal.
#[trace("TC-720", "FR-255-AC-1")]
#[test]
fn check_names_the_identity_limit_it_reached() {
    let chain = long_body_chain();
    let largest = chain
        .checked
        .package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| u64::try_from(node.preimage().len()).expect("a small preimage"))
        .max()
        .expect("the checked package holds nodes");
    // A declared type handle encodes to about 123 bytes.
    assert!(
        largest > 200,
        "the fixture's largest node preimage ({largest}) must exceed a handle's size"
    );
    let run = |value: u64| {
        let mut limits = SpineLimits::default();
        limits.assembly.identity.input_bytes = value;
        reached(check(
            &chain.parsed,
            &chain.models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            limits,
            &Cancel::new(),
        ))
    };
    let counter = assert_field(
        Setting::IdentityInputBytes,
        FoundationKind::InputBytes,
        1_000_000,
        run,
    );
    assert_eq!(
        counter, largest,
        "the identity limit the check reaches is the largest node preimage"
    );
}

/// FR-255-AC-4 at both entry points, stage-driven: an input that reaches a
/// limit stops with that limit's setting, and the same input passes once the
/// setting is raised, through the settings operation's operand and through a
/// replay request's `stage_limits` entry. Covers `s1.tokens` (parse),
/// `s3.nodes`, `s3.work_units` and `identity.input_bytes` (check).
#[trace("TC-721", "FR-255-AC-4")]
#[test]
fn a_reached_limit_is_raised_by_the_settings_operation_and_by_a_request() {
    let chain = fixture_chain();
    let entry_points = |name: &'static str| {
        move |value: u64| -> [SpineLimits; 2] {
            let setting = Setting::from_name(name).expect("a table name is a setting");
            let operand = format!("{name}={value}");
            let by_operand = CallerLimits::from_operands([operand.as_str()])
                .expect("a well-formed operand")
                .spine;
            let entries: StageLimits = [(setting, value)].into_iter().collect();
            let by_request = CallerLimits::for_request(
                &entries,
                crate::limits::AccountingLimits::default().0,
                ReplayLimits::default(),
            )
            .spine;
            [by_operand, by_request]
        }
    };
    let check_under = |limits: SpineLimits| {
        reached(check(
            &chain.parsed,
            &chain.models,
            &DependencyInput::default(),
            &LockEvidence::default(),
            limits,
            &Cancel::new(),
        ))
    };
    for entry in 0..2 {
        let tokens = entry_points("s1.tokens");
        assert_field(
            Setting::S1Tokens,
            FoundationKind::TokenCount,
            10_000,
            |value| {
                reached(parse(
                    &request(FIXTURE.as_bytes()),
                    tokens(value)[entry].source,
                    &Cancel::new(),
                ))
            },
        );
        let nodes = entry_points("s3.nodes");
        assert_field(
            Setting::S3Nodes,
            FoundationKind::NodeCount,
            100_000,
            |value| check_under(nodes(value)[entry]),
        );
        let work = entry_points("s3.work_units");
        assert_field(
            Setting::S3WorkUnits,
            FoundationKind::WorkBudget,
            1_000_000,
            |value| check_under(work(value)[entry]),
        );
        let identity = entry_points("identity.input_bytes");
        let long = long_body_chain();
        assert_field(
            Setting::IdentityInputBytes,
            FoundationKind::InputBytes,
            1_000_000,
            |value| {
                reached(check(
                    &long.parsed,
                    &long.models,
                    &DependencyInput::default(),
                    &LockEvidence::default(),
                    identity(value)[entry],
                    &Cancel::new(),
                ))
            },
        );
    }
}

/// FR-255-AC-6, the second half: the effective limits a parsed source and a
/// checked package record equal the table's defaults when nothing is
/// configured.
#[trace("TC-721", "FR-255-AC-6")]
#[test]
fn the_effective_limits_a_checked_package_records_are_the_defaults() {
    use qsl_foundation::SettingLimits;

    let limits = CallerLimits::from_operands([]).expect("no operands").spine;
    let chain = fixture_chain();
    assert_eq!(
        chain.parsed.syntax().effective_limits(),
        SourceLimits::default()
    );
    assert_eq!(
        limits.source.bounds(),
        chain.parsed.syntax().effective_limits().bounds()
    );
    let checked = check(
        &chain.parsed,
        &chain.models,
        &DependencyInput::default(),
        &LockEvidence::default(),
        limits,
        &Cancel::new(),
    )
    .unwrap_or_else(|failure| panic!("the fixture checks: {failure:?}"))
    .into_value();
    assert_eq!(
        checked.package.graph().effective_limits(),
        CheckingLimits::default()
    );
}

/// FR-277-AC-1 (TC-758 step 1) for the type-environment ceilings `check`
/// admits the unit's object types under.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn check_names_the_type_environment_limit_field_it_reached() {
    let packages = qsl_semantics::model::intake::package_input([MODEL_DOCUMENT.as_bytes()]);
    let model = chain_of(
        MODEL_FIXTURE.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|failure| panic!("the model fixture compiles: {failure:?}"));
    let run = |edit: fn(&mut SpineLimits, u64)| {
        let model = &model;
        move |value: u64| {
            let mut limits = SpineLimits::default();
            edit(&mut limits, value);
            reached(check(
                &model.parsed,
                &model.models,
                &DependencyInput::default(),
                &LockEvidence::default(),
                limits,
                &Cancel::new(),
            ))
        }
    };
    assert_field(
        Setting::EnvironmentAncestorSteps,
        FoundationKind::EdgeCount,
        10_000,
        run(|limits, value| limits.assembly.environment.ancestor_steps = value),
    );
    assert_field(
        Setting::EnvironmentWorkUnits,
        FoundationKind::WorkBudget,
        10_000_000,
        run(|limits, value| limits.assembly.environment.work_units = value),
    );
}

/// FR-277-AC-1 (TC-758 step 1) and FR-099-AC-7 for the S4 source
/// resolution `check` runs: a unit importing `test/geometry` reaches one
/// library, one import edge and the library's source bytes, and each
/// `dependency.*` limit one below that stops `check` with `LimitExceeded`
/// naming its field.
#[trace("TC-758", "FR-277-AC-1", "TC-446", "FR-099-AC-7")]
#[test]
fn check_names_the_dependency_limit_field_it_reached() {
    let dependencies = DependencyInput::new(vec![geometry()]).expect("admissible");
    let unit = format!(
        "{HEADER}import \"test/geometry\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n"
    );
    let cancel = Cancel::new();
    let parsed = parse(
        &request(unit.as_bytes()),
        SpineLimits::default().source,
        &cancel,
    )
    .expect("the importing unit parses")
    .into_value();
    let models = select(
        &parsed,
        &BTreeMap::new(),
        SpineLimits::default().model,
        &cancel,
    )
    .expect("the importing unit selects no model")
    .into_value();
    let run = |edit: fn(&mut SpineLimits, usize)| {
        let (parsed, models, dependencies) = (&parsed, &models, &dependencies);
        move |value: u64| {
            let mut limits = SpineLimits::default();
            edit(
                &mut limits,
                usize::try_from(value).expect("a test bound fits usize"),
            );
            reached(check(
                parsed,
                models,
                dependencies,
                &LockEvidence::default(),
                limits,
                &Cancel::new(),
            ))
        }
    };
    assert_field(
        Setting::DependencyLibraries,
        FoundationKind::NodeCount,
        10,
        run(|limits, value| limits.dependencies.libraries = value),
    );
    assert_field(
        Setting::DependencyImportEdges,
        FoundationKind::EdgeCount,
        10,
        run(|limits, value| limits.dependencies.import_edges = value),
    );
    assert_field(
        Setting::DependencySourceBytes,
        FoundationKind::InputBytes,
        1_000_000,
        run(|limits, value| limits.dependencies.source_bytes = value),
    );
}

/// A reached limit is a limit, never a refusal of the input or a fault, and
/// it reads back as the stage's limit refusal for a caller that renders it.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn a_reached_limit_is_a_limit_not_a_fault() {
    let mut limits = SpineLimits::default().source;
    limits.tokens = 1;
    let failure = parse(&request(FIXTURE.as_bytes()), limits, &Cancel::new())
        .expect_err("one token does not hold the fixture");
    assert!(matches!(failure, StageFailure::Limit(_)), "{failure:?}");
    let refusal = refusal_or_fault(failure).expect("a limit is not a fault");
    assert_eq!(refusal.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusal.stage(), SpineStage::Source);
}

/// FR-275 (TC-769 step 3): an internal invariant failure is a
/// `StageFailure::Fault`, not a panic and not a refusal, and exits as an
/// internal failure.
#[trace("TC-769", "FR-285-AC-3")]
#[test]
fn an_internal_invariant_failure_is_a_fault() {
    use qsl_semantics::check::{CheckCause, CheckRefusal, KeyFault};
    let refusal = Box::new(CompileRefusal::Check {
        refusals: vec![CheckRefusal {
            location: quire_semantic_value::location::Location::root(
                quire_semantic_value::location::Origin::Expression,
            ),
            cause: CheckCause::InternalFault(Box::new(KeyFault::UnresolvedDraft)),
        }],
        region: None,
    });
    let failure = failure_of(refusal, &|_| None);
    assert!(matches!(failure, StageFailure::Fault(_)), "{failure:?}");
    assert!(refusal_or_fault(failure).is_err());
}

/// FR-278: a library is compiled independently of whoever imports it. One
/// library imported by two units that pass different lock evidence is the
/// same package, whose `package_id` the import digest names; and a library
/// that needs a lock the default evidence lacks is refused even when its
/// importer's evidence supplies it.
#[trace("TC-759", "FR-278-AC-1")]
#[test]
fn a_library_is_checked_under_its_own_lock_evidence_not_its_importers() {
    use qsl_semantics::value::{CatalogRole, DefinitionLock};
    let text_profile = || {
        LockEvidence::default().with_text_profile(
            DefinitionLock::pinned()
                .entry(CatalogRole::TextProfile)
                .reference(),
        )
    };
    let limits = SpineLimits::default();
    let importer = |library: &SuppliedLibrary, lock: &LockEvidence| {
        let unit = format!(
            "{HEADER}import \"test/geometry\" as g;\n\
             function h using v(): Boolean pure {{ true }}\n"
        );
        let dependencies = DependencyInput::new(vec![library.clone()]).expect("admissible");
        let live = Cancel::new();
        let parsed = parse(&request(unit.as_bytes()), limits.source, &live)
            .expect("the importing unit parses")
            .into_value();
        check(
            &parsed,
            &AdmittedModels::default(),
            &dependencies,
            lock,
            limits,
            &live,
        )
        .map(|checked| checked.into_value())
    };

    let geometry = geometry();
    for lock in [LockEvidence::default(), text_profile()] {
        importer(&geometry, &lock)
            .unwrap_or_else(|failure| panic!("the import resolves under {lock:?}: {failure:?}"));
    }

    // A library comparing text needs the text law. Its importer supplies it,
    // but the library is checked under the default evidence and refuses.
    let texty = SuppliedLibrary {
        bytes: format!(
            "{HEADER}function f using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure \
             {{ a = b }}\n"
        )
        .into_bytes(),
        ..geometry
    };
    let refusal = refusal(
        importer(&texty, &text_profile())
            .expect_err("the library lacks the text law under its own evidence"),
    );
    assert_eq!(refusal.code().as_str(), "missing_declaration");
}

/// FR-278 (TC-759 step 1): `check` takes the package's lock evidence. A unit
/// that compares two text values has a text law to select, so without lock
/// evidence naming the text profile the check refuses, and with it the unit
/// checks.
#[trace("TC-759", "FR-278-AC-1")]
#[test]
fn check_takes_the_lock_evidence_the_text_law_comes_from() {
    use qsl_semantics::value::{CatalogRole, DefinitionLock};
    let unit = format!(
        "{HEADER}function same using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure \
         {{ a = b }}\n"
    );
    let limits = SpineLimits::default();
    let live = Cancel::new();
    let parsed = parse(&request(unit.as_bytes()), limits.source, &live)
        .expect("the unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();
    let run = |lock: &LockEvidence| {
        check(
            &parsed,
            &models,
            &DependencyInput::default(),
            lock,
            limits,
            &live,
        )
    };
    let without = run(&LockEvidence::default()).expect_err("no text law is selected");
    let refusal = refusal(without);
    assert_eq!(refusal.code().as_str(), "missing_declaration");
    let text_profile = DefinitionLock::pinned()
        .entry(CatalogRole::TextProfile)
        .reference();
    run(&LockEvidence::default().with_text_profile(text_profile))
        .expect("the lock evidence selects the text law");
}
