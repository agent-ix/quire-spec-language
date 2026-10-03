// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-755 to TC-757 and TC-759: the front-end operations are typed library
//! operations over a caller-owned handle, and compose to a checked,
//! emitted package.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_eval::value::{CallFailure, QualifiedName};
use qsl_foundation::diagnostic::{Category, StageFailure, Staged};
use qsl_foundation::SourceIdentity;
use qsl_package::{emit_checked, read_import_view, AdmittedPackages};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::library::LibraryName;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Cancel, CancelCause, Integer, Outcome, Value};

use super::{
    check, execute, package, parse, select, AdmittedModels, CheckedUnit, EmittedUnit,
    ExecuteRequest, FrontEndFailure, ParseRequest, ParsedSource,
};
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
        version: "1".to_owned(),
        source: SourceIdentity::new("a", "geometry", "git", "1"),
        path: "geometry.native".to_owned(),
        bytes: format!("{HEADER}function f using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n")
            .into_bytes(),
    }
}

/// `library`'s own `package_id`, compiled alone under its own identity.
fn library_id(library: &SuppliedLibrary) -> qsl_semantics::library::PackageId {
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let parsed = parse(
        &ParseRequest {
            source: &library.source,
            path: &library.path,
            bytes: &library.bytes,
        },
        limits.source,
        &cancel,
    )
    .expect("the library parses")
    .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &cancel)
        .expect("a library with no model selects nothing")
        .into_value();
    let checked = check(
        &parsed,
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect("the library checks")
    .into_value();
    package(&checked, &cancel)
        .expect("the library emits")
        .into_value()
        .package()
        .package_id()
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
    let checked = check(&parsed, &models, dependencies, limits, &cancel)?.into_value();
    let emitted = package(&checked, &cancel)?.into_value();
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
        SpineLimits::default(),
        &cancel,
    )
    .expect("the parsed source checks again")
    .into_value();
    let reemitted = package(&rechecked, &cancel)
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

/// A handle that counts every charge made under it, and the charges made
/// once it was cancelled, from a clone that watches from another thread.
struct Watched {
    cancel: Cancel,
    charges: Arc<AtomicU64>,
    after_cancel: Arc<AtomicU64>,
}

fn watched() -> Watched {
    let charges = Arc::new(AtomicU64::new(0));
    let after_cancel = Arc::new(AtomicU64::new(0));
    let slot: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
    let cancel = {
        let (charges, after_cancel, slot) = (
            Arc::clone(&charges),
            Arc::clone(&after_cancel),
            Arc::clone(&slot),
        );
        Cancel::observing(move || {
            if slot.get().is_some_and(|handle| handle.cause().is_some()) {
                after_cancel.fetch_add(1, Ordering::SeqCst);
            }
            charges.fetch_add(1, Ordering::SeqCst);
        })
    };
    assert!(slot.set(cancel.clone()).is_ok());
    Watched {
        cancel,
        charges,
        after_cancel,
    }
}

/// Cancel `watch` with `cause` from a second thread once its observer has
/// seen `after` charges.
fn cancel_after(watch: &Watched, after: u64, cause: CancelCause) -> std::thread::JoinHandle<()> {
    let (cancel, charges) = (watch.cancel.clone(), Arc::clone(&watch.charges));
    std::thread::spawn(move || {
        while charges.load(Ordering::SeqCst) < after {
            std::thread::yield_now();
        }
        cancel.cancel(cause);
    })
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
        limits,
        &cancelled,
    ));
    assert_requested(package(&chain.checked, &cancelled));
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

/// Declarations in the generated unit of the cancelled-check test. The AC
/// names 200,000, but S1's parse time grows with the square of the unit's
/// size (a debug build measured 0.4 s at 1,000 declarations and 19.8 s at
/// 8,000), so a unit that size cannot be generated through `parse` in a test
/// run. Each declaration costs the check five charges, so 4,000 make 20,000,
/// twenty times the 1,000 the canceller waits for.
const DECLARATIONS: usize = 4_000;

/// FR-276-AC-2 (TC-757 step 2): a `check` over a generated unit of
/// declarations, cancelled with `Deadline` from a second thread once the
/// observer has seen 1,000 charges, returns `Cancelled(Deadline)` and no
/// package, and the observer records at most one charge after the
/// cancellation.
#[trace("TC-757", "FR-276-AC-2")]
#[test]
fn a_check_cancelled_from_another_thread_stops_within_one_charge() {
    let source = declarations(DECLARATIONS);
    let limits = wide(source.len());
    let live = Cancel::new();
    let parsed = parse(&request(source.as_bytes()), limits.source, &live)
        .expect("the generated unit parses")
        .into_value();
    let models = select(&parsed, &BTreeMap::new(), limits.model, &live)
        .expect("a unit with no model selects nothing")
        .into_value();

    let watch = watched();
    let canceller = cancel_after(&watch, 1_000, CancelCause::Deadline);
    let outcome = check(
        &parsed,
        &models,
        &DependencyInput::default(),
        limits,
        &watch.cancel,
    );
    canceller.join().expect("the canceller finishes");

    match outcome {
        Err(StageFailure::Cancelled(CancelCause::Deadline)) => {}
        Err(other) => panic!("expected Cancelled(Deadline), got {other:?}"),
        Ok(_) => panic!("a cancelled check returned a package"),
    }
    let charges = watch.charges.load(Ordering::SeqCst);
    assert!(charges >= 1_000, "{charges} charges");
    assert!(
        watch.after_cancel.load(Ordering::SeqCst) <= 1,
        "{} charges after the cancellation",
        watch.after_cancel.load(Ordering::SeqCst)
    );
    assert!(
        charges < 5 * u64::try_from(DECLARATIONS).expect("a small count"),
        "the check stopped before its {charges}th charge"
    );
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

    let watch = watched();
    let canceller = cancel_after(&watch, 1_000, CancelCause::Requested);
    let function = QualifiedName::unqualified("d23").expect("an identifier");
    let outcome = execute(
        &chain.checked,
        &ExecuteRequest {
            function: &function,
            arguments: &[],
            objects: &ObjectEnvironment::default(),
        },
        default_accounting(u64::MAX),
        &watch.cancel,
    );
    canceller.join().expect("the canceller finishes");

    assert_eq!(
        outcome.map(|_| ()).unwrap_err(),
        CallFailure::Cancelled(CancelCause::Requested)
    );
    assert!(
        watch.after_cancel.load(Ordering::SeqCst) <= 1,
        "{} charges after the cancellation",
        watch.after_cancel.load(Ordering::SeqCst)
    );
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
    let library_id = library_id(&geometry);
    let unit = format!(
        "{HEADER}import \"test/geometry\" version \"1\" digest \"{}\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n",
        library_id.hex()
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
        "1",
        packages,
        &mut AdmittedPackages::default(),
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
        check(&parsed, &models, &DependencyInput::default(), limits, &live)
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
    let library_id = library_id(&geometry);
    let unit = format!(
        "{HEADER}import \"test/geometry\" version \"1\" digest \"{}\" as g;\n\
         function h using v(): Boolean pure {{ true }}\n",
        library_id.hex()
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
