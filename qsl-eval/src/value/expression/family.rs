// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-065): the S6a/evaluation half of `Value`'s
//! function-declaration and function-application checked-family glue.
//!
//! This module holds `QualifiedName` (the layer-6 `replay`/[`super::
//! CheckedPackage::call`] lookup key) and [`ValueFunctionFamily`]'s
//! [`super::s6a::ReferenceEvaluation`] half. The checked-package producer
//! FR-065-AC-2 verifies against is `qsl_package::emit_checked` plus its I2
//! reader (`qsl-package/src/emit/tests.rs`); S4 linking is
//! `qsl_package::CheckedPackage::link`, not a step this module repeats.
//! ADR-011 §7.3 M-5 (QSL-139/FR-068) moved this module's checking-only
//! half -- identity minting, [`qsl_semantics::check::PackageDeclarations::check`]'s
//! own [`qsl_semantics::family::FamilyContract`] hook, and the `OccurrenceMap`
//! `check` builds from it -- into `check::family`, since FR-068-
//! AC-3 forbids `check` importing anything from `value::expression`; see
//! that module's own doc for why the split runs through `family.rs` even
//! though FR-068 itself only names seven of `value::expression`'s eight
//! submodules. `ValueFunctionFamily` is re-exported from there
//! ([`qsl_semantics::check::ValueFunctionFamily`]) and this module implements its
//! evaluation half over it -- layer 5 depending on layer 3 is the
//! permitted direction (ADR-011 §6.1).

use qsl_foundation::diagnostic::InternalFault;
use quire_exact::{is_identifier, NodeKey, Value};

use qsl_semantics::check::ValueFunctionFamily;

/// ADR-013 O-11: a non-empty sequence of identifiers, `::`-separated on
/// display -- the layer-6 `replay` facade's (and, for this ticket,
/// [`super::CheckedPackageEvaluation::call`]'s) only function-selection key. Never a
/// bare `&str`; the one allowed name lookup (R-06) resolves this against a
/// checked package's declarations, and nothing compares it as a display
/// string (FR-065-AC-6).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QualifiedName(Box<[String]>);

/// A `QualifiedName` must be one or more identifiers; this segment sequence
/// is not.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("a qualified name is a non-empty sequence of identifiers")]
pub struct InvalidQualifiedName;

impl QualifiedName {
    /// A qualified name from its segments, refusing anything that is not a
    /// non-empty sequence of identifiers.
    pub fn new(segments: Vec<String>) -> Result<Self, InvalidQualifiedName> {
        if !segments.is_empty() && segments.iter().all(|segment| is_identifier(segment)) {
            Ok(Self(segments.into_boxed_slice()))
        } else {
            Err(InvalidQualifiedName)
        }
    }

    /// A single-segment qualified name: an ordinary unqualified function
    /// name, the only shape `Value`'s Complete-V1 function family has.
    pub fn unqualified(name: impl Into<String>) -> Result<Self, InvalidQualifiedName> {
        Self::new(vec![name.into()])
    }

    pub(crate) fn segments(&self) -> &[String] {
        &self.0
    }

    /// This name's segments as a plain unqualified name, when it has
    /// exactly one segment. Complete-V1 declares no qualified names, so a
    /// multi-segment `QualifiedName` resolves against no declaration here.
    pub(crate) fn as_unqualified(&self) -> Option<&str> {
        match &self.0[..] {
            [name] => Some(name.as_str()),
            _ => None,
        }
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.segments().join("::"))
    }
}

impl std::str::FromStr for QualifiedName {
    type Err = InvalidQualifiedName;

    /// Parse `Display`'s own `::`-joined spelling back into segments.
    fn from_str(spelling: &str) -> Result<Self, Self::Err> {
        Self::new(spelling.split("::").map(str::to_owned).collect())
    }
}

/// Seals [`super::CheckedPackageEvaluation`]: [`super::CheckedPackage`] is
/// its one implementor, and an implementation for any other type would have
/// no meaning. Declared here because `value::expression`'s own `mod.rs`
/// declares only `evaluate` and `family` (TC-170).
pub(super) mod sealed {
    /// The private supertrait of [`super::super::CheckedPackageEvaluation`].
    pub trait Sealed {}
    impl Sealed for super::super::CheckedPackage {}
}

/// `ValueFunctionFamily`'s real evaluation environment (ADR-012 §2's
/// `evaluate` hook, FR-062-AC-1/AC-6): the checked package `checked`'s
/// identity resolves against and the caller's object environment -- both
/// borrowed for the one call, never owned by the family marker type. The
/// meter is not part of it: the hook's own `meter` parameter meters the
/// whole call (QSL-206). [`super::CheckedPackageEvaluation::call`] is this
/// environment's one real (non-test) constructor.
pub(crate) struct EvaluationEnv<'a> {
    pub(crate) package: &'a super::CheckedPackage,
    pub(crate) objects: &'a qsl_semantics::model::object_environment::ObjectEnvironment,
    pub(crate) arguments: Option<Vec<Value>>,
    /// The last hook call's `Evaluation.location` (FR-090-OQ-3 ruling): the
    /// hook's `EvalOutcome` holds no location, so the hook records it here
    /// on every `Ok` return and [`super::CheckedPackage::call`] reads it.
    pub(crate) location: Option<qsl_semantics::check::Location>,
    /// The last hook call's `Evaluation.losses`, recorded like `location`.
    pub(crate) losses: Vec<super::evaluate::LocatedLoss>,
}

impl<'a> EvaluationEnv<'a> {
    /// A fresh environment holding `arguments`, with no location and no
    /// losses recorded.
    pub(crate) fn new(
        package: &'a super::CheckedPackage,
        objects: &'a qsl_semantics::model::object_environment::ObjectEnvironment,
        arguments: Vec<Value>,
    ) -> Self {
        Self {
            package,
            objects,
            arguments: Some(arguments),
            location: None,
            losses: Vec::new(),
        }
    }
}

impl super::s6a::ReferenceEvaluation for ValueFunctionFamily {
    type Observed = Value;
    type Env<'a> = EvaluationEnv<'a>;
    type Key = NodeKey;

    /// Reads only `checked` (a bare identity) and `env`'s
    /// checked package/object environment; this hook's own body touches no
    /// CST, token or display string, though `env.package` is a
    /// `&CheckedPackage`, which exposes string-shaped accessors this hook
    /// simply does not call -- so this is a description of what the code
    /// does today, not a type-level guarantee (FR-062-AC-6's second
    /// sentence stays unbacked; see FR-062's own Status).
    /// `meter` (the shared kernel meter every family's `evaluate`
    /// takes) is charged one `ChargePoint::FunctionCall` -- "one checked
    /// function call"'s own documented meaning, matching what this hook is
    /// about to run -- per call, denied into
    /// `Ok(EvalOutcome::Kernel(Outcome::Incomplete(_)))` exactly when the
    /// caller-configured `meter` cannot afford it (FR-090-AC-1).
    ///
    /// **One meter for the whole call (QSL-206).** The body then runs
    /// against that same `meter`, which charges every nested call and value
    /// operation. So the caller's meter counts the top-level call's own
    /// `function.call`, first, followed by everything the body does. An
    /// earlier version ran the body against a second meter held in `env`
    /// and charged the top-level call to a meter `CheckedPackage::call`
    /// created and then dropped, so the caller never saw that charge.
    /// `Machine::run` charges no entry-level `function.call` of its own
    /// (PR #302 review finding 2), so the call is still charged exactly once.
    ///
    /// **`location` and `losses` are recorded in `env` (FR-090-OQ-3
    /// ruling).** The hook's `EvalOutcome` holds neither (ADR-012 §2). The
    /// hook resets both at entry and writes both on every `Ok` return: a
    /// denied entry charge records `None` and no losses, since no node ran.
    /// A consumed `env` faults before the meter is charged, so a reused env
    /// never reports an earlier call's location or losses.
    ///
    /// FR-063-AC-7: `#[deny(...)]` closes the `_ => unsupported(...)` escape
    /// hatch this function's own `FamilyOutcome` match -- the seam probe's
    /// evidence for "the one match over `crate::family::FamilyOutcome`"
    /// (`checked_in_locations`'s own doc) -- would otherwise let slip past
    /// unnoticed by the probe alone.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut EvaluationEnv<'a>,
        meter: &mut quire_exact::Meter,
    ) -> Result<qsl_semantics::family::EvalOutcome<Value>, InternalFault> {
        env.location = None;
        env.losses.clear();
        // FR-090-AC-3 (ADR-013 T-4): `call` always resolves `checked` from
        // this same package's declarations before calling `evaluate`, so a
        // lookup miss here means the two lookups disagreed -- a broken S6a
        // invariant, never a caller-input refusal (`call` supplied nothing
        // wrong). `call` forwards it as `CallFailure::Fault`.
        let function = env
            .package
            .graph()
            .function_by_identity(*checked)
            .ok_or_else(|| InternalFault::new("S6a", "checked-identity-not-resolved-by-package"))?;
        // A second `evaluate` call on the same `env` finds its arguments
        // already taken: a broken S6a invariant (FR-090-AC-3), with its own
        // identifier, never a silent zero-argument evaluation. Checked
        // before the meter charge, so a consumed env always faults.
        // `CheckedPackage::call` builds a fresh env per call, so this is
        // unreached through it.
        let Some(arguments) = env.arguments.take() else {
            return Err(InternalFault::new(
                "S6a",
                "evaluation-environment-arguments-already-consumed",
            ));
        };
        if let Err(incomplete) = meter.charge(quire_exact::Charge::new(
            quire_exact::ChargePoint::FunctionCall,
        )) {
            // Nothing ran: the arguments stay unconsumed.
            env.arguments = Some(arguments);
            return Ok(qsl_semantics::family::EvalOutcome::Kernel(
                quire_exact::Outcome::Incomplete(incomplete),
            ));
        }
        let evaluation = super::evaluate::Machine::new(
            env.package.graph().scope(),
            env.package.graph(),
            env.objects,
            meter,
            env.package.graph().dispatch_tables(),
        )
        .run(function.body, function.slots, arguments)?;
        env.location = evaluation.location;
        env.losses = evaluation.losses;
        match evaluation.outcome {
            qsl_semantics::family::FamilyOutcome::Evaluated(outcome) => {
                Ok(qsl_semantics::family::EvalOutcome::Kernel(outcome))
            }
            qsl_semantics::family::FamilyOutcome::FamilyEvaluated(result) => {
                Ok(qsl_semantics::family::EvalOutcome::Family(result))
            }
            // FR-063: no arm for the probe variant under `--cfg seam_probe`
            // alone (`E0004`, this seam's evidence).
            //
            // The arm below exists only in the probe's build of the crates
            // above `qsl-eval` (`--cfg seam_probe_eval_downstream`, QSL-5):
            // `qsl-replay` depends on this crate, so it must compile there
            // for the root crate's own seams to be reached at all.
            #[cfg(seam_probe_eval_downstream)]
            qsl_semantics::family::FamilyOutcome::__SeamProbe => {
                unreachable!("never constructed outside the probe build")
            }
        }
    }
}

// FR-062-AC-8/FR-063-AC-6 (ADR-012 §5.1 S4, "each family Cause enum's
// catalog_code()" seam-probe coverage) is real now, not deferred. QSL-148
// gives `Value`'s function-declaration family a real `Cause`:
// `ValueFunctionFamily::Cause = qsl_semantics::check::CheckRefusal`
// (`qsl_semantics::check::family`), returned through
// `qsl_foundation::diagnostic::StageFailure::Refused` when `check` genuinely refuses
// (an ill-typed or undefined body). `CheckRefusal`'s own `catalog_code()`
// mapping (`CheckCause::code`/`CheckCause::cause`, `src/check/refusal.rs`)
// already existed and was exhaustive by construction -- it is `Value`'s
// pre-existing checking-refusal vocabulary, not a new enum authored to fill
// this associated type. QSL-152 wires FR-063's S4 seam probe onto it:
// `CheckCause::code` (`qsl-semantics/src/check/refusal.rs`) carries a
// `#[cfg(seam_probe)]` variant with no arm, the same shape
// `FamilyKind::catalog_code_prefix`'s S1 probe already used, and
// `xtask::seam_probe::checked_in_locations` checks it in as this
// repository's one S4 location (FR-062-AC-8, TC-161); `CheckCause::cause`
// carries the probe variant's arm instead. Doing the same for whichever of
// the other five families migrates a real `Cause` next remains open work,
// but S4 itself is no longer unimplemented.
//
// An earlier version of this file instead kept an uninhabited
// `DeclarationCause` with a `#[cfg(seam_probe)]` probe variant, on the
// theory that the probe build would demonstrate S4 the same way it
// demonstrates S1 (`FamilyKind`). It did not: `FamilyKind`'s probe works
// because six real variants and real production `match`es already exist, so
// adding a seventh variant breaks matches nothing else could reach
// otherwise -- a real author has to notice. `DeclarationCause` had no real
// variant and no production `match` beside its own `catalog_code()`; the
// only thing that could ever fail under the probe was that same function
// gaining one more arm. That was the construct testing itself, not a seam,
// which is why it was deleted rather than reused now that a real cause
// exists.

#[cfg(test)]
mod family_contract_tests {
    use super::super::s6a::ReferenceEvaluation;
    use super::*;
    use ix_trace_rs::trace;
    use qsl_forms::{Expression, FunctionDeclaration, TypeForm};
    use qsl_semantics::check::{
        declaration, declaration_signature, declarations_for, empty_scope, fixture_source, limits,
        root_location, CheckingLimits, PackageDeclarations, Signatures, SCALAR_LIMITS_UNLIMITED,
    };
    use qsl_semantics::family::{DiagnosticSink, EvalOutcome, FamilyContract, ScopeStack};
    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use qsl_semantics::value::declaration::TypeEnvironment;
    use quire_exact::Meter;

    /// `Value`'s function-declaration family is a real `FamilyContract`
    /// implementation, reachable through the trait, not a free-standing
    /// function with no shared associated-type binding, and its checked
    /// identity is FR-092 vector F1. Untagged for FR-062-AC-1 (PR #262
    /// review, finding F3): AC-1 requires all six contract parts as
    /// compile-time obligations, and this trait now has only `check` -- see
    /// FR-062's own amended Acceptance Criteria for why AC-1 is recorded
    /// unbacked rather than retagged onto a narrower claim.
    #[trace("TC-380", "FR-065-AC-7")]
    #[test]
    fn value_function_family_checks_through_the_contract() {
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let signatures = Signatures::default();
        let declarations = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = qsl_semantics::check::check_context(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        ValueFunctionFamily::check(&form, &mut cx).unwrap();
        assert_eq!(diagnostics.entries().len(), 1);
        // The admitted declaration's identity is its FR-092 function node
        // key, which `PackageDeclarations::check` mints once every
        // declaration is typed (QSL-156 A4b): for `f() -> Boolean { true }`
        // under owner (a, u), FR-092 vector F1.
        let expected = PackageDeclarations {
            functions: vec![form],
            ..PackageDeclarations::new(fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("f checks")
        .function_identity("f")
        .expect("f is declared");
        assert_eq!(
            expected.to_string(),
            "dbd06f242fc36f1ed1b5773a7e59fb89ebc862494d8512b44e84942bea153e79"
        );
    }

    /// TC-160 (FR-062-AC-4): a function declaration whose body holds no
    /// scalar operation application carries no claim, so `requirements` on
    /// its checked node is empty, and a second call on the same node is
    /// equal.
    #[trace("TC-160", "FR-062-AC-4")]
    #[test]
    fn a_function_declaration_has_no_requirements() {
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let signatures = Signatures::default();
        let declarations = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = qsl_semantics::check::check_context(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let checked = ValueFunctionFamily::check(&form, &mut cx)
            .expect("f checks")
            .into_value();
        assert!(ValueFunctionFamily::requirements(&checked).is_empty());
        assert_eq!(
            ValueFunctionFamily::requirements(&checked),
            ValueFunctionFamily::requirements(&checked)
        );
    }

    /// PR #262 review, finding F17 (round 3, item 8), amended by FR-090-AC-3
    /// (TC-384): a second `evaluate` call on the same `EvaluationEnv` --
    /// whose `arguments` the first call already consumed via `.take()` --
    /// is a broken S6a invariant, `Err(InternalFault)` naming stage `"S6a"`,
    /// never a silent zero-argument evaluation and never a
    /// `FamilyResult`/kernel-shaped refusal. `EvaluationEnv`'s one real
    /// (non-test) constructor (`CheckedPackage::call`) never reaches this:
    /// it always builds a fresh env with `Some(arguments)` and calls
    /// `evaluate` exactly once, so the guard is unreached through that
    /// path. This test bypasses that constructor -- the same thing the F17
    /// finding's own fix verified by hand and then reverted, leaving the
    /// fix itself unguarded -- and lands the guard with a real second call,
    /// asserting the S6a seam's own result directly rather than through
    /// `CheckedPackage::call`'s own match arm.
    #[trace("FR-090-AC-3", "TC-384")]
    #[test]
    fn evaluate_faults_on_a_second_call_on_the_same_env() {
        let graph = PackageDeclarations {
            functions: vec![declaration("f", Expression::Boolean(true))],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks cleanly");
        let identity = graph
            .function_identity("f")
            .expect("f is declared in this package");
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        let package = qsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut env = EvaluationEnv::new(&package, &objects, Vec::new());
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
            .expect("first call, with real arguments still present, evaluates cleanly");
        let fault = ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
            .expect_err("second call on the same env, arguments already consumed, must fault");
        assert_eq!(fault.stage(), "S6a");
        assert_eq!(
            fault.category(),
            qsl_foundation::diagnostic::Category::InternalFailure
        );
        assert_eq!(
            fault.invariant(),
            "evaluation-environment-arguments-already-consumed"
        );
    }

    /// FR-062-AC-5's `Incomplete` half (QSL-153): `evaluate` genuinely
    /// charges its own kernel `meter` parameter (`ChargePoint::FunctionCall`,
    /// `ValueFunctionFamily::evaluate`'s own doc) -- given a meter whose
    /// `work_units` limit is already exhausted, that charge is denied and
    /// `evaluate` returns `Ok(EvalOutcome::Kernel(Outcome::Incomplete(_)))`,
    /// a real `quire_exact::Incomplete`, never `Err(InternalFault)`.
    /// `check`, run against the same declaration, admits it normally: its
    /// return type (`CheckOutcome`/`StageFailure`) has no `Incomplete`
    /// variant to return in the first place, so the two hooks cannot be
    /// confused by construction, not merely by this test's assertions.
    ///
    /// The outer `Ok(EvalOutcome::Kernel(Outcome::Incomplete(_)))` shape
    /// alone would also accept a mismatched-point or mismatched-counter
    /// denial, so this asserts the denied record's own fields -- `charge_point` is
    /// exactly `FunctionCall` (not some other point this meter happened to
    /// deny), `limit_kind` is `WorkUnits` (the counter `work_units: 0`
    /// actually bounds, not `TextInputBytes` or another counter this fixture
    /// never touches) and `limit` is `0` (the exact configured bound, not
    /// merely "some limit") -- and that `env.arguments` is still `Some`: a
    /// denied charge runs nothing, so it must not consume them, and the
    /// recorded location and losses are empty (FR-090: `None` when the
    /// evaluation stopped before any node ran).
    #[trace("TC-160", "FR-062-AC-5")]
    #[test]
    fn evaluate_returns_incomplete_when_the_meter_is_exhausted() {
        let graph = PackageDeclarations {
            functions: vec![declaration("f", Expression::Boolean(true))],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks cleanly, never Incomplete");
        let identity = graph
            .function_identity("f")
            .expect("f is declared in this package");
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        let package = qsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut env = EvaluationEnv::new(&package, &objects, Vec::new());
        let exhausted_limits = quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        };
        let mut meter = Meter::new(exhausted_limits);
        let outcome = ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
            .expect("a denied admission charge is Ok(EvalOutcome::Kernel(Incomplete)), not Err");
        match outcome {
            EvalOutcome::Kernel(quire_exact::Outcome::Incomplete(record)) => {
                assert_eq!(record.charge_point, quire_exact::ChargePoint::FunctionCall);
                assert_eq!(record.limit_kind, quire_exact::LimitKind::WorkUnits);
                assert_eq!(record.limit, 0);
            }
            other => panic!("expected EvalOutcome::Kernel(Outcome::Incomplete(_)), got {other:?}"),
        }
        assert!(
            env.arguments.is_some(),
            "a denied charge must not have consumed env.arguments"
        );
        assert_eq!(env.location, None);
        assert!(env.losses.is_empty());
    }

    /// `name(p: Decimal[0..100], q: Decimal[1..9]): Decimal[0..10000; 2, 2;
    /// mode] = p / q`. With `nearest-even` a call on (1, 3) completes and
    /// records one loss; with `exact` it stops with a refusal, located at the
    /// division.
    fn decimal_division(name: &str, mode: &str) -> FunctionDeclaration {
        let decimal = |bounds: [&str; 5]| {
            TypeForm::builtin(
                qsl_forms::BuiltinType::Decimal,
                qsl_foundation::Span { start: 0, end: 0 },
            )
            .with_bounds(bounds.map(str::to_owned).to_vec())
        };
        FunctionDeclaration::new(
            name,
            vec![
                ("p".to_owned(), decimal(["0", "100", "0", "0", "exact"])),
                ("q".to_owned(), decimal(["1", "9", "0", "0", "exact"])),
            ],
            decimal(["0", "10000", "2", "2", mode]),
            None,
            Expression::Binary {
                operator: qsl_forms::BinaryOperator::Divide,
                left: Box::new(Expression::Name("p".to_owned())),
                right: Box::new(Expression::Name("q".to_owned())),
            },
        )
    }

    /// FR-090: the hook records `location` and `losses` on every `Ok`
    /// return, so a reused env never reports an earlier call's location or
    /// losses. On one env: a rounding division completes with one loss; an
    /// exact division stops with a location and no losses; a third call,
    /// with an exhausted meter, reports `Incomplete` with no location and no
    /// losses; a fourth, with its arguments consumed, faults before the
    /// meter is charged.
    #[test]
    fn a_reused_env_never_reports_an_earlier_calls_losses() {
        let graph = PackageDeclarations {
            functions: vec![
                decimal_division("rounding", "nearest-even"),
                decimal_division("exact", "exact"),
            ],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("p / q with q in [1, 9] checks cleanly");
        let rounding = graph
            .function_identity("rounding")
            .expect("rounding is declared in this package");
        let exact = graph
            .function_identity("exact")
            .expect("exact is declared in this package");
        let package = qsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let decimal = |coefficient: i64| {
            Value::Decimal(quire_exact::Decimal::new(
                quire_exact::Integer::from(coefficient),
                0,
            ))
        };
        let mut env = EvaluationEnv::new(&package, &objects, vec![decimal(1), decimal(3)]);
        let mut unlimited = Meter::new(SCALAR_LIMITS_UNLIMITED);

        let first = ValueFunctionFamily::evaluate(&rounding, &mut env, &mut unlimited)
            .expect("the rounding division completes");
        assert!(
            matches!(
                first,
                EvalOutcome::Kernel(quire_exact::Outcome::Completed(_))
            ),
            "{first:?}"
        );
        assert_eq!(env.location, None);
        assert_eq!(env.losses.len(), 1, "the division rounds 1/3");

        env.arguments = Some(vec![decimal(1), decimal(3)]);
        let second = ValueFunctionFamily::evaluate(&exact, &mut env, &mut unlimited)
            .expect("the exact division stops in Ok");
        assert!(
            matches!(
                second,
                EvalOutcome::Kernel(quire_exact::Outcome::Refused(_))
            ),
            "{second:?}"
        );
        assert!(env.location.is_some(), "a refusal is located");
        assert!(env.losses.is_empty(), "{:?}", env.losses);

        env.arguments = Some(vec![decimal(1), decimal(3)]);
        let mut exhausted = Meter::new(quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        });
        let third = ValueFunctionFamily::evaluate(&rounding, &mut env, &mut exhausted)
            .expect("a denied entry charge is Ok(Incomplete)");
        assert!(
            matches!(
                third,
                EvalOutcome::Kernel(quire_exact::Outcome::Incomplete(_))
            ),
            "{third:?}"
        );
        assert_eq!(env.location, None);
        assert!(env.losses.is_empty(), "{:?}", env.losses);

        env.arguments = None;
        let fault = ValueFunctionFamily::evaluate(&rounding, &mut env, &mut exhausted)
            .expect_err("a consumed env faults before the meter is charged");
        assert_eq!(
            fault.invariant(),
            "evaluation-environment-arguments-already-consumed"
        );
    }

    /// PR #302 review finding 2, QSL-206: a *nested* call's denied
    /// `function.call` surfaces as the kernel `Outcome::Incomplete`, inside
    /// `EvalOutcome::Kernel`, from `Machine::run`'s own `charge_call` -- not
    /// from the top-level admission. `caller`'s body calls `callee`. The
    /// meter allows one work unit: the top-level call to `caller` spends it,
    /// on the same meter the body runs against, so the nested call to
    /// `callee` is the one denied, with one unit already consumed.
    #[test]
    fn evaluate_returns_incomplete_when_a_nested_call_exhausts_the_meter() {
        let graph = PackageDeclarations {
            functions: vec![
                declaration("callee", Expression::Boolean(true)),
                declaration(
                    "caller",
                    Expression::Call {
                        name: "callee".to_owned(),
                        arguments: Vec::new(),
                    },
                ),
            ],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("callee and caller both check cleanly");
        let identity = graph
            .function_identity("caller")
            .expect("caller is declared in this package");
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        let package = qsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut env = EvaluationEnv::new(&package, &objects, Vec::new());
        let mut meter = Meter::new(quire_exact::ScalarLimits {
            work_units: 1,
            ..SCALAR_LIMITS_UNLIMITED
        });
        let outcome = ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
            .expect("a denied nested charge is Ok(EvalOutcome::Kernel(Incomplete)), not Err");
        match outcome {
            EvalOutcome::Kernel(quire_exact::Outcome::Incomplete(record)) => {
                assert_eq!(record.charge_point, quire_exact::ChargePoint::FunctionCall);
                assert_eq!(record.limit_kind, quire_exact::LimitKind::WorkUnits);
                assert_eq!(record.consumed, 1, "the top-level call spent the one unit");
            }
            other => panic!("expected EvalOutcome::Kernel(Outcome::Incomplete(_)), got {other:?}"),
        }
        assert!(
            env.location.is_some(),
            "the nested call's denial is located in the body"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_forms::Expression;
    use qsl_foundation::source::provenance::RawSourceRef;
    use qsl_semantics::check::{
        admitted_source, declaration, fixture_source, CheckingLimits, PackageDeclarations,
    };

    /// `f`'s checked identity: its FR-092 function node key, declared by
    /// `owner`'s unit.
    fn checked_identity(source: RawSourceRef) -> NodeKey {
        PackageDeclarations {
            functions: vec![declaration("f", Expression::Boolean(true))],
            ..PackageDeclarations::new(source)
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks")
        .function_identity("f")
        .expect("f is declared")
    }

    /// ADR-013 O-11/FR-088-AC-6: a [`QualifiedName`] is a declared preimage
    /// component, never an identity in its own right. Two entries that
    /// share an equal qualified name but were declared by different owners
    /// (FR-092) carry different node ids.
    #[trace("TC-258", "FR-088-AC-6")]
    #[test]
    fn equal_qualified_names_do_not_collapse_distinct_declarations() {
        let first = checked_identity(fixture_source());
        let second = checked_identity(admitted_source(
            qsl_foundation::SourceIdentity::new("a", "w", "git", "1"),
            b"",
        ));
        assert_ne!(
            first, second,
            "distinct declarations must not share a node id"
        );
    }
}
