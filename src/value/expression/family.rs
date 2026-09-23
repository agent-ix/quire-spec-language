// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-065): the S6a/evaluation half of `Value`'s
//! function-declaration and function-application checked-family glue.
//!
//! This module holds `QualifiedName` (the layer-6 `replay`/[`super::
//! CheckedPackage::call`] lookup key), S4 linking
//! (`link_function_identity`), the v2 emit/decode codec, and
//! [`ValueFunctionFamily`]'s [`crate::family::ReferenceEvaluation`] half.
//! ADR-011 §7.3 M-5 (QSL-139/FR-068) moved this module's checking-only
//! half -- identity minting, [`crate::check::PackageDeclarations::check`]'s
//! own [`crate::family::FamilyContract`] hook, and the `OccurrenceMap`
//! `check` builds from it -- into `check::family`, since FR-068-
//! AC-3 forbids `check` importing anything from `value::expression`; see
//! that module's own doc for why the split runs through `family.rs` even
//! though FR-068 itself only names seven of `value::expression`'s eight
//! submodules. `ValueFunctionFamily` is re-exported from there
//! ([`crate::check::ValueFunctionFamily`]) and this module implements its
//! evaluation half over it -- layer 5 depending on layer 3 is the
//! permitted direction (ADR-011 §6.1).

use qsl_attrs::string_edge;

use qsl_foundation::diagnostic::InternalFault;
use quire_exact::{is_identifier, NodeKey};

use super::super::composite::Value;
use crate::check::ValueFunctionFamily;

/// ADR-013 O-11: a non-empty sequence of identifiers, `::`-separated on
/// display -- the layer-6 `replay` facade's (and, for this ticket,
/// [`super::CheckedPackageEvaluation::call`]'s) only function-selection key. Never a
/// bare `&str`; the one allowed name lookup (R-06) resolves this against a
/// checked package's declarations, and nothing compares it as a display
/// string (FR-065-AC-6).
///
/// `Serialize`/`Deserialize` (PR #262 review, finding F6) round-trip through
/// the same `::`-joined spelling [`std::fmt::Display`] and [`std::str::FromStr`]
/// already use, via `#[serde(try_from = "String", into = "String")]` --
/// `emit_v2`/`decode_v2`'s wire `name` field is typed on `QualifiedName`
/// itself now, not a bare `String` a caller has to re-parse and re-validate
/// (the same hole `call`'s own `&QualifiedName` parameter closes one
/// function over).
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
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

impl TryFrom<String> for QualifiedName {
    type Error = InvalidQualifiedName;

    fn try_from(spelling: String) -> Result<Self, Self::Error> {
        spelling.parse()
    }
}

impl From<QualifiedName> for String {
    fn from(name: QualifiedName) -> Self {
        name.to_string()
    }
}

/// One entry in the checked-package producer's minimal v2 encoding: a
/// declared function's qualified name and its checked identity.
///
/// `deny_unknown_fields` (PR #262 review, finding F16): decoded through the
/// `pub` [`crate::value::decode_function_package_v2`], from bytes an external caller
/// supplies, not only from this crate's own `emit_v2` output -- an unknown
/// field should refuse, not silently disappear.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionEntryV2 {
    /// Typed on `QualifiedName` (PR #262 review, finding F6), not a bare
    /// `String`: an entry whose wire spelling is not a valid qualified name
    /// fails `serde_json::from_slice` itself (via `QualifiedName`'s own
    /// `#[serde(try_from = "String")]`), which `decode_v2` already maps to
    /// `DecodeV2Error::Malformed` -- no separate malformed-name variant
    /// needed.
    name: QualifiedName,
    identity: String,
}

/// `quire.checked-function-package/v2`: this ticket's self-consistent v2
/// encoding for function-declaration identity. It is not a claim of
/// conformance to the external `quire.checked-package-id/v2` schema (see
/// `check::family::mint_declaration_identity`'s doc) -- it exists to demonstrate, and let
/// a test assert, that identity survives check, S4 linking and a v2
/// emit/decode round trip unchanged (FR-065-AC-2).
const FUNCTION_PACKAGE_V2_VERSION: &str = "quire.checked-function-package/v2";

/// `deny_unknown_fields` (PR #262 review, finding F16): see
/// [`FunctionEntryV2`]'s own doc -- same reason, same externally-decoded
/// input.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionPackageV2 {
    version: String,
    functions: Vec<FunctionEntryV2>,
}

/// S4 linking, for this migration's minimal scope: a checked function's
/// identity carried forward unchanged. Complete-V1 has no cross-package
/// import to resolve for function declarations, so linking here is
/// genuinely a pass-through -- named and exercised as its own step (rather
/// than folded into `check`) so FR-065-AC-2's three checkpoints (after
/// `check`, after linking, after v2 decode) are three real, distinct calls.
pub(crate) fn link_function_identity(identity: NodeKey) -> NodeKey {
    identity
}

/// Emit v2 bytes for a linked set of (qualified name, identity) pairs.
pub(crate) fn emit_v2(functions: &[(QualifiedName, NodeKey)]) -> Vec<u8> {
    let package = FunctionPackageV2 {
        version: FUNCTION_PACKAGE_V2_VERSION.to_owned(),
        functions: functions
            .iter()
            .map(|(name, identity)| FunctionEntryV2 {
                name: name.clone(),
                identity: identity.to_string(),
            })
            .collect(),
    };
    serde_json::to_vec(&package).expect("FunctionPackageV2 always serializes")
}

/// Why v2 bytes could not be decoded.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecodeV2Error {
    /// The bytes did not parse as `FunctionPackageV2`'s JSON shape at all
    /// (`serde_json::from_slice` failed).
    #[error("malformed quire.checked-function-package/v2 bytes")]
    Malformed,
    /// The bytes parsed, but the package's `version` field did not match
    /// this module's `FUNCTION_PACKAGE_V2_VERSION`.
    #[error("unrecognised version")]
    Version,
    // PR #262 review (F16): this used to say "64 lowercase hex digits",
    // which `decode_hex_32` never enforced -- it accepts either case via
    // `is_ascii_hexdigit`. `emit_v2` always emits lowercase
    // (`NodeKey`'s `Display` impl, `quire-exact/src/node.rs`), so nothing
    // this crate produces is ever uppercase, but decode is genuinely
    // case-insensitive; the message now says what the code does.
    /// An entry's `identity` string was not 64 hex digits (either case
    /// accepted; see the note above on why the message doesn't say
    /// "lowercase").
    #[error("identity is not 64 hex digits")]
    InvalidIdentity,
}

/// Decode v2 bytes back into (qualified name, identity) pairs, refusing an
/// unrecognised version or a malformed identity rather than guessing.
///
/// `#[string_edge]` (PR #262 review, finding F5): the `package.version !=
/// FUNCTION_PACKAGE_V2_VERSION` wire-version gate below is one of ADR-012
/// §9's own listed edges ("the typed v2 reader"), but compares against a
/// named `const`, not a literal, so `xtask string-edge`'s literal-operand
/// scan cannot see it -- it was unmarked and undetected until this pass.
/// Marking it declares this reader as the sanctioned edge ADR-012 §9
/// already lists it as, independent of whatever the scanner's own
/// const-operand blind spot does or does not catch.
#[string_edge]
pub(crate) fn decode_v2(bytes: &[u8]) -> Result<Vec<(QualifiedName, NodeKey)>, DecodeV2Error> {
    let package: FunctionPackageV2 =
        serde_json::from_slice(bytes).map_err(|_| DecodeV2Error::Malformed)?;
    if package.version != FUNCTION_PACKAGE_V2_VERSION {
        return Err(DecodeV2Error::Version);
    }
    package
        .functions
        .into_iter()
        .map(|entry| {
            let bytes = decode_hex_32(&entry.identity).ok_or(DecodeV2Error::InvalidIdentity)?;
            Ok((entry.name, NodeKey::from_digest(bytes)))
        })
        .collect()
}

/// Decode `quire.checked-function-package/v2` bytes emitted by
/// [`super::CheckedPackageEvaluation::emit_function_package_v2`] back into
/// (qualified name, identity) pairs, for a caller verifying identity
/// survived the round trip (FR-065-AC-2). The public entry point to
/// `decode_v2`.
pub fn decode_function_package_v2(
    bytes: &[u8],
) -> Result<Vec<(QualifiedName, NodeKey)>, DecodeV2Error> {
    decode_v2(bytes)
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

fn decode_hex_32(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (pairs, []) = hex.as_bytes().as_chunks::<2>() else {
        return None;
    };
    let mut out = [0_u8; 32];
    for (slot, pair) in out.iter_mut().zip(pairs) {
        let text = std::str::from_utf8(pair).ok()?;
        *slot = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

/// `ValueFunctionFamily`'s real evaluation environment (ADR-012 §2's
/// `evaluate` hook, FR-062-AC-1/AC-6): the checked package `checked`'s
/// identity resolves against, the caller's object environment and its own
/// accounting meter -- all borrowed for the one call, never owned by the
/// family marker type. [`super::CheckedPackageEvaluation::call`] is this
/// environment's one real (non-test) constructor.
pub(crate) struct EvaluationEnv<'a> {
    pub(crate) package: &'a super::CheckedPackage,
    pub(crate) objects: &'a super::super::reference::ObjectEnvironment,
    pub(crate) arguments: Option<Vec<super::super::composite::Value>>,
    pub(crate) local_meter: &'a mut quire_exact::Meter,
    /// The last hook call's `Evaluation.location` (FR-090-OQ-3 ruling): the
    /// hook's `EvalOutcome` holds no location, so the hook records it here
    /// on every `Ok` return and [`super::CheckedPackage::call`] reads it.
    pub(crate) location: Option<crate::check::Location>,
    /// The last hook call's `Evaluation.losses`, recorded like `location`.
    pub(crate) losses: Vec<super::evaluate::LocatedLoss>,
}

impl<'a> EvaluationEnv<'a> {
    /// A fresh environment holding `arguments`, with no location and no
    /// losses recorded.
    pub(crate) fn new(
        package: &'a super::CheckedPackage,
        objects: &'a super::super::reference::ObjectEnvironment,
        arguments: Vec<super::super::composite::Value>,
        local_meter: &'a mut quire_exact::Meter,
    ) -> Self {
        Self {
            package,
            objects,
            arguments: Some(arguments),
            local_meter,
            location: None,
            losses: Vec::new(),
        }
    }
}

impl crate::family::ReferenceEvaluation for ValueFunctionFamily {
    type Observed = Value;
    type Env<'a> = EvaluationEnv<'a>;
    type Key = NodeKey;

    /// FR-062-AC-6: reads only `checked` (a bare identity) and `env`'s
    /// checked package/object environment/meter -- no CST, token or
    /// display string. `meter` (the shared kernel meter every family's
    /// `evaluate` takes) is genuinely charged now (QSL-153): one
    /// `ChargePoint::FunctionCall` -- "one checked function call"'s own
    /// documented meaning, matching what this hook is about to run -- per
    /// call, denied into `Ok(EvalOutcome::Kernel(Outcome::Incomplete(_)))`
    /// exactly when the caller-configured `meter` cannot afford it
    /// (FR-090-AC-1).
    ///
    /// **This is the top-level call's one and only `function.call` charge
    /// (PR #302 review finding 2).** An earlier version *also* charged
    /// `function.call` a second time, against `env.local_meter`, inside
    /// `Machine::run` itself (a `call: bool` flag charged once up front
    /// whenever the root was a function body) -- the same named point,
    /// charged twice for one logical call, against two different meter
    /// instances. `Machine::run` no longer takes that flag; see its own doc.
    /// `Value`'s own value-level evaluation still charges `env.local_meter`
    /// (its pre-existing accounting meter, `quire_exact::Meter` since
    /// QSL-166 -- the same type as `meter`, a separate instance) for every
    /// *nested* call and value operation the body performs, unchanged by
    /// this contract; the two meters bound two different things (this
    /// hook's own admission to run at all, versus the work its body does
    /// once running), and neither restates the other's charge.
    ///
    /// **`location` and `losses` are recorded in `env` (FR-090-OQ-3
    /// ruling).** The hook's `EvalOutcome` holds neither (ADR-012 §2). The
    /// hook resets both at entry and writes both on every `Ok` return: a
    /// denied entry charge records `None` and no losses, since no node ran.
    /// A consumed `env` faults before the meter is charged, so a reused env
    /// never reports an earlier call's location or losses.
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut EvaluationEnv<'a>,
        meter: &mut quire_exact::Meter,
    ) -> Result<crate::family::EvalOutcome<Value>, InternalFault> {
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
            return Ok(crate::family::EvalOutcome::Kernel(
                quire_exact::Outcome::Incomplete(incomplete),
            ));
        }
        let callables = super::callables(env.package);
        let evaluation = super::evaluate::Machine::new(
            env.package.graph().scope(),
            &callables,
            env.objects,
            env.local_meter,
            env.package.graph().dispatch_tables(),
        )
        .run(function.body, function.slots, arguments)?;
        env.location = evaluation.location;
        env.losses = evaluation.losses;
        match evaluation.outcome {
            super::FamilyOutcome::Evaluated(outcome) => {
                Ok(crate::family::EvalOutcome::Kernel(outcome))
            }
            super::FamilyOutcome::FamilyEvaluated(result) => {
                Ok(crate::family::EvalOutcome::Family(result))
            }
        }
    }
}

// FR-062-AC-8/FR-063-AC-6 (ADR-012 §5.1 S4, "each family Cause enum's
// catalog_code()" seam-probe coverage) is still deferred, though QSL-148
// gives `Value`'s function-declaration family a real `Cause` at last:
// `ValueFunctionFamily::Cause = crate::check::CheckRefusal`
// (`crate::check::family`), returned through
// `crate::family::StageFailure::Refused` when `check` genuinely refuses
// (an ill-typed or undefined body). `CheckRefusal`'s own `catalog_code()`
// mapping (`CheckCause::code`/`CheckCause::cause`, `src/check/refusal.rs`)
// already exists and is exhaustive by construction -- it is `Value`'s
// pre-existing checking-refusal vocabulary, not a new enum authored to fill
// this associated type. What remains deferred is FR-063's S4 seam probe
// itself: demonstrating, under `--cfg seam_probe`, that a *newly added*
// `CheckCause` variant with no `code()`/`cause()` arm fails to compile
// (`E0004`) the way `FamilyKind::catalog_code_prefix`'s S1 probe already
// does for a new family. `CheckCause` was not authored under that probe
// discipline (it predates FR-062's contract entirely), and wiring the S4
// probe onto it, plus doing the same for whichever of the other five
// families migrates a real `Cause` next, is QSL-152's remaining scope here
// -- not "no family has a cause yet," which QSL-148 makes no longer true.
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
    use super::*;
    use crate::check::{
        declarations_for, empty_scope, mint_resolved, root_location, CheckCause, CheckingLimitKind,
        CheckingLimits, PackageDeclarations, Signature, DEFAULT_PACKAGE_IDENTITY,
        SCALAR_LIMITS_UNLIMITED,
    };
    use crate::family::{
        CheckContext, DiagnosticSink, EvalOutcome, FamilyContract, ReferenceEvaluation, ScopeStack,
        StageLimits,
    };
    use crate::forms::{Expression, FunctionDeclaration, TypeForm};
    use crate::value::composite::ValueType;
    use crate::value::declaration::TypeEnvironment;
    use crate::value::reference::ObjectEnvironment;
    use ix_trace_rs::trace;
    use quire_exact::Meter;

    // `EvaluationEnv::local_meter` (`ValueFunctionFamily::evaluate`'s
    // pre-existing accounting path) and `ReferenceEvaluation::evaluate`'s
    // own `_meter` parameter are both `quire_exact::Meter` since QSL-166
    // (previously two distinct types with the same name in different
    // crates) -- `evaluate_faults_on_a_second_call_on_the_same_env` still
    // needs two separate *instances*, one per role.

    fn limits() -> StageLimits {
        StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        }
    }

    /// A bare `Boolean` type form.
    fn boolean_type_form() -> TypeForm {
        TypeForm::builtin(
            crate::forms::BuiltinType::Boolean,
            qsl_foundation::Span { start: 0, end: 0 },
        )
    }

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), boolean_type_form(), None, body)
    }

    /// The resolved signature of a [`declaration`] fixture (no parameters,
    /// `Boolean` result), for `declarations_for`'s `own_signature`.
    fn declaration_signature(name: &str) -> Signature {
        Signature {
            name: name.to_owned(),
            parameters: Vec::new(),
            result: ValueType::Boolean,
            callable_by_name: true,
        }
    }

    /// `Value`'s function-declaration family is a real `FamilyContract`
    /// implementation, reachable through the trait, not a free-standing
    /// function with no shared associated-type binding -- and its minted
    /// identity survives a real v2 emit/decode round trip through
    /// `family::emit_v2`/`decode_v2` directly (PR #262 review, F1/F2:
    /// no longer routed through the deleted `FamilyContract::package`,
    /// which nothing consumed). Untagged for FR-062-AC-1 (PR #262 review,
    /// finding F3): AC-1 requires all six contract parts as compile-time
    /// obligations, and this trait now has only `check` -- see FR-062's own
    /// amended Acceptance Criteria for why AC-1 is recorded unbacked rather
    /// than retagged onto a narrower claim.
    #[trace("TC-380", "FR-065-AC-7")]
    #[test]
    fn value_function_family_checks_through_the_contract() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let (expected, _) = mint_resolved(&empty_scope(), &package_identity, &form, u64::MAX);
        let staged = ValueFunctionFamily::check(&form, &mut cx).unwrap();
        assert_eq!(staged.value.identity, expected);
        assert_eq!(diagnostics.entries().len(), 1);
        let declaration_name = QualifiedName::unqualified("declaration").unwrap();
        let v2 = emit_v2(&[(declaration_name.clone(), staged.value.identity)]);
        assert_eq!(decode_v2(&v2).unwrap(), vec![(declaration_name, expected)]);
    }

    /// QSL-148's core requirement (PR #303 review, finding 1): calling
    /// `ValueFunctionFamily::check` on an ill-typed declaration -- `g() ->
    /// Boolean = 1`, an `Integer` body against a declared `Boolean` result
    /// -- returns a refusal *through the contract itself*, not `Ok` after
    /// minting an identity that says nothing about whether the body
    /// actually types. Before this ticket, `check` never inspected
    /// `form.body`'s type at all: the real typing decision was made by a
    /// `Typer` `check::mod`'s per-declaration loop constructed and drove
    /// separately, after already calling `check` and discarding nothing --
    /// so this exact fixture, checked through the contract alone, returned
    /// `Ok`. It does not any more.
    #[trace("TC-380", "FR-065-AC-7")]
    #[test]
    fn value_function_family_check_refuses_an_ill_typed_body() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("g");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = FunctionDeclaration::new(
            "g",
            Vec::new(),
            boolean_type_form(),
            None,
            Expression::Integer(quire_exact::Integer::from(1_i64)),
        );
        let refused = ValueFunctionFamily::check(&form, &mut cx).expect_err(
            "an Integer body against a declared Boolean result must refuse through the contract",
        );
        match refused {
            crate::family::StageFailure::Refused(refusal) => assert!(
                matches!(
                    refusal.cause,
                    CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
                ),
                "expected an ill-typed/type-mismatch refusal, got {refusal:?}"
            ),
            other => panic!("expected StageFailure::Refused, got {other:?}"),
        }
        // PR #303 review, finding 11: no diagnostic is recorded for a
        // refused declaration -- the "checked function declaration"
        // message asserts a real success, not an attempt.
        assert_eq!(diagnostics.entries().len(), 0);
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
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks cleanly");
        let identity = graph
            .function_identity("f")
            .expect("f is declared in this package");
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        let package = crate::checked_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = EvaluationEnv::new(&package, &objects, Vec::new(), &mut local_meter);
        let mut contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect("first call, with real arguments still present, evaluates cleanly");
        let fault = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
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
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks cleanly, never Incomplete");
        let identity = graph
            .function_identity("f")
            .expect("f is declared in this package");
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        let package = crate::checked_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = EvaluationEnv::new(&package, &objects, Vec::new(), &mut local_meter);
        let exhausted_limits = quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        };
        let mut contract_meter = Meter::new(exhausted_limits);
        let outcome = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
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
                crate::forms::BuiltinType::Decimal,
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
                operator: crate::forms::BinaryOperator::Divide,
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
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("p / q with q in [1, 9] checks cleanly");
        let rounding = graph
            .function_identity("rounding")
            .expect("rounding is declared in this package");
        let exact = graph
            .function_identity("exact")
            .expect("exact is declared in this package");
        let package = crate::checked_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let decimal = |coefficient: i64| {
            Value::Decimal(quire_exact::Decimal::new(
                quire_exact::Integer::from(coefficient),
                0,
            ))
        };
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = EvaluationEnv::new(
            &package,
            &objects,
            vec![decimal(1), decimal(3)],
            &mut local_meter,
        );
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

    /// PR #302 review finding 2: a *nested* nested call denies against
    /// `env.local_meter` (real production behaviour, not the contract-level
    /// `meter` parameter's own admission charge exercised above) surfaces as
    /// the pre-existing kernel `Outcome::Incomplete`, inside
    /// `EvalOutcome::Kernel`, from `Machine::run`'s own `charge_call` --
    /// never the top-level admission's own `Incomplete` path. `caller`'s
    /// body calls `callee`; with
    /// `local_meter`'s `work_units` already exhausted, the nested call
    /// inside `caller`'s own body (not the top-level call to `caller`
    /// itself, which the unlimited `contract_meter` here admits) is what is
    /// denied.
    #[test]
    fn evaluate_returns_incomplete_when_a_nested_calls_local_meter_is_exhausted() {
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
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("callee and caller both check cleanly");
        let identity = graph
            .function_identity("caller")
            .expect("caller is declared in this package");
        // ADR-013 T-1 (FR-087, QSL-158 S-3a): the S4 link step, over an
        // empty dependency closure -- this fixture declares no import.
        let package = crate::checked_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let exhausted_limits = quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        };
        let mut local_meter = Meter::new(exhausted_limits);
        let mut env = EvaluationEnv::new(&package, &objects, Vec::new(), &mut local_meter);
        let mut contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let outcome = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect(
            "the top-level call's own admission charge is against contract_meter, unlimited here",
        );
        assert!(
            matches!(
                outcome,
                EvalOutcome::Kernel(quire_exact::Outcome::Incomplete(_))
            ),
            "expected kernel Outcome::Incomplete from the nested call's own denied charge, got {outcome:?}"
        );
    }

    /// Two independently constructed contexts each observe exactly one
    /// diagnostic from checking the same form -- if `check` wrote through
    /// any shared/global state instead of `cx.diagnostics`, one of the two
    /// independent sinks would show zero or more than one entry (PR #262
    /// review, finding F6: the previous version of this test compared
    /// `diagnostics_a`'s count against an unrelated, freshly constructed
    /// `DiagnosticSink::default()` rather than against `diagnostics_b`, so
    /// it never actually observed `cx_b`'s own state, and separately
    /// asserted a pure function's output against itself by comparing
    /// `staged_a.value` to `staged_b.value` -- both deleted).
    ///
    /// **Untagged (PR #262 review, coordinator round 3, finding 5).** This
    /// test was tagged `FR-062-AC-3`, whose central clause is that two
    /// typing contexts checking the same declarations produce *identical
    /// checked output* -- F6 correctly deleted the `staged_a.value ==
    /// staged_b.value` self-comparison that used to (fabricatedly) stand in
    /// for that, but kept the tag on what remained: two counts, each
    /// asserted only against the literal `1` the loop below guarantees by
    /// construction, not against each other's checked output. That is a
    /// real isolation test, not an identical-output test, so it is untagged
    /// rather than left claiming to back a criterion it does not; see
    /// FR-062's own amended Acceptance Criteria for AC-3's current status.
    #[test]
    fn two_contexts_from_the_same_declarations_check_identically() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scalar_limits = SCALAR_LIMITS_UNLIMITED;
        let form = declaration("f", Expression::Boolean(true));
        let own_signature = declaration_signature("f");

        let scope_a = empty_scope();
        let location_a = root_location();
        let declarations_a = declarations_for(
            &package_identity,
            &scope_a,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location_a,
        );
        let mut meter_a = Meter::new(scalar_limits);
        let mut diagnostics_a = DiagnosticSink::default();
        let mut scopes_a = ScopeStack::default();
        let mut cx_a = CheckContext::new(
            &declarations_a,
            limits(),
            &mut meter_a,
            &mut diagnostics_a,
            &mut scopes_a,
        );
        ValueFunctionFamily::check(&form, &mut cx_a).unwrap();

        let scope_b = empty_scope();
        let location_b = root_location();
        let declarations_b = declarations_for(
            &package_identity,
            &scope_b,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location_b,
        );
        let mut meter_b = Meter::new(scalar_limits);
        let mut diagnostics_b = DiagnosticSink::default();
        let mut scopes_b = ScopeStack::default();
        let mut cx_b = CheckContext::new(
            &declarations_b,
            limits(),
            &mut meter_b,
            &mut diagnostics_b,
            &mut scopes_b,
        );
        ValueFunctionFamily::check(&form, &mut cx_b).unwrap();

        // Each independently constructed sink shows exactly its own one
        // entry -- a shared/global sink would leak entries into whichever
        // one ran second, or show two entries in one and zero in the other.
        assert_eq!(diagnostics_a.entries().len(), 1);
        assert_eq!(diagnostics_b.entries().len(), 1);
    }

    /// **Untagged.** Guards the top-level declaration-entry charge alone
    /// (limit 0 refuses, limit 1 admits a leaf-bodied declaration) --
    /// narrower than FR-062-AC-7, whose own fixture-at-depth-D requirement
    /// [`real_checker_depth_limit_is_the_proximate_cause`] below addresses
    /// (see that test's own doc for why it is untagged, rather than
    /// retagged onto this narrower charge).
    #[test]
    fn nesting_depth_limit_is_the_proximate_cause() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut tight = StageLimits {
            nesting_depth: 0,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };
        let mut cx = CheckContext::new(
            &declarations,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let refused = ValueFunctionFamily::check(&form, &mut cx);
        assert!(matches!(
            refused,
            Err(crate::family::StageFailure::Limit(_))
        ));

        tight.nesting_depth = 1;
        let mut cx = CheckContext::new(
            &declarations,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let admitted = ValueFunctionFamily::check(&form, &mut cx);
        assert!(admitted.is_ok());
    }

    /// QSL-153: `StageLimits`' restored `input_bytes`/`node_count` each have
    /// a real producer (`mint_declaration_identity`'s own preimage pass) and
    /// a real consumer (`CheckContext::check_input_bytes`/
    /// `check_node_count`, called from `ValueFunctionFamily::check`) that
    /// changes behaviour: a limit configured one below the real, measured
    /// metric refuses with `Limit` naming that exact kind; the same limit
    /// at the metric itself admits -- the same "varies by exactly one"
    /// shape `nesting_depth`'s own test uses, so the limit (not the
    /// fixture) is shown to be the proximate cause. `work_budget` is a real
    /// producer and consumer too, but through the shared kernel meter's own
    /// `work_units` charge (PR #302 review finding 3), not a `StageLimits`
    /// field -- see `work_budget_kind_refuses_from_a_denied_meter_charge`.
    #[trace("TC-160", "FR-062-AC-5")]
    #[test]
    fn stage_limits_restored_kinds_refuse_one_below_the_real_metric() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let form = declaration("f", Expression::Boolean(true));
        let (_, metrics) = mint_resolved(&empty_scope(), &package_identity, &form, u64::MAX);
        assert!(metrics.input_bytes > 0 && metrics.node_count > 0);

        let base = StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };
        let check_kind = |limits: StageLimits, expected_kind: crate::family::StageLimitKind| {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut diagnostics = DiagnosticSink::default();
            let mut scopes = ScopeStack::default();
            let mut cx = CheckContext::new(
                &declarations,
                limits,
                &mut meter,
                &mut diagnostics,
                &mut scopes,
            );
            match ValueFunctionFamily::check(&form, &mut cx) {
                Err(crate::family::StageFailure::Limit(exceeded)) => {
                    assert_eq!(exceeded.kind, expected_kind);
                }
                other => panic!("expected a Limit outcome naming {expected_kind:?}, got {other:?}"),
            }
        };
        let admits = |limits: StageLimits| {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut diagnostics = DiagnosticSink::default();
            let mut scopes = ScopeStack::default();
            let mut cx = CheckContext::new(
                &declarations,
                limits,
                &mut meter,
                &mut diagnostics,
                &mut scopes,
            );
            assert!(ValueFunctionFamily::check(&form, &mut cx).is_ok());
        };

        check_kind(
            StageLimits {
                input_bytes: metrics.input_bytes - 1,
                ..base
            },
            crate::family::StageLimitKind::InputBytes,
        );
        admits(StageLimits {
            input_bytes: metrics.input_bytes,
            ..base
        });

        check_kind(
            StageLimits {
                node_count: metrics.node_count - 1,
                ..base
            },
            crate::family::StageLimitKind::NodeCount,
        );
        admits(StageLimits {
            node_count: metrics.node_count,
            ..base
        });
    }

    /// PR #302 review finding 3: `WorkBudget` is a real `Limit` outcome
    /// produced by a *denied `cx.meter` charge* in `check` -- not by
    /// comparing the preimage's own write count against a `StageLimits`
    /// field (that field's own meaning was "how many times the encoder
    /// wrote," never a caller-configured budget). A `cx.meter` whose
    /// `work_units` limit is already exhausted denies `check`'s own
    /// `ChargePoint::DeclarationCheck` charge on the first checked
    /// declaration, mapped to `StageLimitKind::WorkBudget`; the same
    /// declaration against a meter with real headroom admits.
    #[test]
    fn work_budget_kind_refuses_from_a_denied_meter_charge() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let form = declaration("f", Expression::Boolean(true));
        let limits = StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };

        let exhausted_limits = quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        };
        let mut meter = Meter::new(exhausted_limits);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        match ValueFunctionFamily::check(&form, &mut cx) {
            Err(crate::family::StageFailure::Limit(exceeded)) => {
                assert_eq!(exceeded.kind, crate::family::StageLimitKind::WorkBudget);
            }
            other => panic!("expected a Limit outcome naming WorkBudget, got {other:?}"),
        }

        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        assert!(ValueFunctionFamily::check(&form, &mut cx).is_ok());
    }

    /// FR-062-AC-7's fixture-at-depth-D requirement, backed against
    /// `Typer`'s own pre-existing, already-correct
    /// [`crate::check::CheckingLimits`] depth bound -- not the contract's
    /// own `StageLimits.nesting_depth` (see this test's "Untagged" note
    /// below for why those are different mechanisms). A body nested to
    /// depth D (`Not(Not(Not(true)))`, four levels deep counting the
    /// `Boolean` leaf) checked through `ValueFunctionFamily::check` --
    /// reachable now that QSL-148 makes `check` call
    /// `check_declaration_body`, which drives the real `Typer` -- refuses
    /// at a configured depth of D-1 and admits at D, varying only the
    /// limit by exactly one.
    ///
    /// **Untagged for FR-062-AC-7 (PR #303 review, findings 4/5).** This
    /// replaces `real_recursive_descent_is_nesting_depth_bounded`, which
    /// backed FR-062-AC-7/TC-378 against a redundant contract-level walk
    /// (`check::family::charge_recursive_nesting`, deleted): that walk
    /// duplicated `check_declaration_body`'s own real recursion just to
    /// charge `CheckContext::enter_nesting`, and lost the real path's
    /// location and early-return behavior in the process (finding 5).
    /// AC-7's own text requires `check` to return a `Limit` outcome
    /// specifically; this test's refusal is `StageFailure::Refused
    /// (CheckRefusal { cause: ResourceExhausted { kind: Depth, .. }, .. })`
    /// -- a typed refusal through `Typer`'s pre-existing, unrelated
    /// `CheckingLimits.depth` bound, not a `StageFailure::Limit` naming the
    /// contract's own nesting-depth limit. Wiring the contract's own
    /// `CheckContext::enter_nesting` into every recursive step of
    /// `Typer::infer`/`infer_form` (not just the one top-level entry charge
    /// `check` already makes) would require threading `&mut CheckContext`
    /// through the general engine's entire recursive signature -- the same
    /// class of `Typer` entanglement QSL-148's own open question raises,
    /// reported here (see `check::family::check_application`'s own doc)
    /// rather than routed around by re-tagging this test onto AC-7.
    #[test]
    fn real_checker_depth_limit_is_the_proximate_cause() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scope = empty_scope();
        let location = root_location();
        let nested = Expression::Not(Box::new(Expression::Not(Box::new(Expression::Not(
            Box::new(Expression::Boolean(true)),
        )))));
        let form = declaration("f", nested);
        let own_signature = declaration_signature("f");

        let tight = CheckingLimits::new(u64::MAX, 3).expect("3 is within MAX_CHECKING_DEPTH");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            tight,
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let refused = ValueFunctionFamily::check(&form, &mut cx)
            .expect_err("a depth limit of 3 must refuse a body nested 4 deep");
        match refused {
            crate::family::StageFailure::Refused(refusal) => assert!(
                matches!(
                    refusal.cause,
                    CheckCause::ResourceExhausted {
                        kind: CheckingLimitKind::Depth,
                        ..
                    }
                ),
                "expected a Depth resource-exhausted refusal, got {refusal:?}"
            ),
            other => panic!("expected StageFailure::Refused, got {other:?}"),
        }

        let wide = CheckingLimits::new(u64::MAX, 4).expect("4 is within MAX_CHECKING_DEPTH");
        let declarations = declarations_for(
            &package_identity,
            &scope,
            &[],
            &own_signature,
            &[],
            wide,
            &location,
        );
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let admitted = ValueFunctionFamily::check(&form, &mut cx);
        assert!(
            admitted.is_ok(),
            "a depth limit of 4 must admit the identical body nested exactly 4 deep"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{empty_scope, mint_resolved, OccurrenceMap, DEFAULT_PACKAGE_IDENTITY};
    use crate::forms::{Expression, FunctionDeclaration, TypeForm};
    use ix_trace_rs::trace;

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(
            name,
            Vec::new(),
            TypeForm::builtin(
                crate::forms::BuiltinType::Boolean,
                qsl_foundation::Span { start: 0, end: 0 },
            ),
            None,
            body,
        )
    }

    /// FR-062-AC-2/FR-065-AC-2: two structurally identical declarations mint
    /// one identity; a name change mints a different one.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn identical_declarations_share_one_identity() {
        let a = declaration("f", Expression::Boolean(true));
        let b = declaration("f", Expression::Boolean(true));
        let c = declaration("g", Expression::Boolean(true));
        assert_eq!(
            mint_resolved(&empty_scope(), DEFAULT_PACKAGE_IDENTITY, &a, u64::MAX).0,
            mint_resolved(&empty_scope(), DEFAULT_PACKAGE_IDENTITY, &b, u64::MAX).0
        );
        assert_ne!(
            mint_resolved(&empty_scope(), DEFAULT_PACKAGE_IDENTITY, &a, u64::MAX).0,
            mint_resolved(&empty_scope(), DEFAULT_PACKAGE_IDENTITY, &c, u64::MAX).0
        );
    }

    // `identity_ignores_unrelated_declarations` (PR #262 review, coordinator
    // round 3) is deleted from here. It minted `DEFAULT_PACKAGE_IDENTITY`'s
    // identity for the same `target` twice and compared the result to
    // itself -- no second declaration was ever constructed, so FR-065-AC-2's
    // reordering clause had nothing to be independent *of*; its own comment
    // conceded "this is definitionally true". A real reordering test needs
    // two actual declarations checked in two actual orders, which
    // `mint_declaration_identity`'s single-declaration signature cannot
    // exercise -- see
    // `tests/dispatch_calls.rs`'s
    // `function_identity_survives_reordering_check_linking_and_a_v2_round_trip`,
    // built at the `PackageDeclarations::check` level instead, where
    // position could actually leak.

    /// FR-062-AC-2: two occurrences of one identity get distinct ordinals;
    /// a different identity's occurrence does not consume an ordinal from
    /// this one.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn occurrence_ordinals_are_per_identity_and_role() {
        let mut map = OccurrenceMap::default();
        let a = NodeKey::from_digest([1; 32]);
        let b = NodeKey::from_digest([2; 32]);
        let first = map.record(a, "reference", (0, 3));
        let second = map.record(a, "reference", (4, 7));
        let other = map.record(b, "reference", (8, 11));
        assert_eq!(first.ordinal(), 0);
        assert_eq!(second.ordinal(), 1);
        assert_eq!(other.ordinal(), 0);
        assert_eq!(map.resolve(a, &first), Some(&(0, 3)));
        assert_eq!(map.resolve(a, &second), Some(&(4, 7)));
        assert_eq!(map.resolve(b, &other), Some(&(8, 11)));
    }

    /// FR-065-AC-2: identity read after `check` survives a real v2
    /// emit/decode round trip unchanged. Does not exercise a distinct
    /// "after S4 linking" checkpoint (PR #262 review, finding F6):
    /// `link_function_identity` is `fn(x) -> x` for this migration's real
    /// scope (`link_function_identity`'s own doc), so comparing its input
    /// to its output is comparing a value to itself, not something a
    /// broken implementation could fail. QSL-154 owns the real before/
    /// after linking assertion, for a family whose linking is a real
    /// transformation (FR-065-AC-1/AC-3, occurrence-span survival across
    /// S4 linking) -- against this family's pass-through linking, that
    /// assertion would not have been testing anything.
    #[trace("TC-163", "FR-065-AC-2")]
    #[test]
    fn identity_survives_v2_round_trip() {
        let declaration = declaration("f", Expression::Boolean(true));
        let (after_check, _) = mint_resolved(
            &empty_scope(),
            DEFAULT_PACKAGE_IDENTITY,
            &declaration,
            u64::MAX,
        );
        let name = QualifiedName::unqualified("f").unwrap();
        let bytes = emit_v2(&[(name.clone(), after_check)]);
        let decoded = decode_v2(&bytes).unwrap();
        assert_eq!(decoded, vec![(name, after_check)]);
    }

    /// ADR-013 O-11/FR-088-AC-6: a [`QualifiedName`] is a declared preimage
    /// component, never an identity in its own right. Two entries that
    /// share an equal qualified name but were minted for different
    /// declarations carry different node ids, and a v2 round trip keeps
    /// both pairs distinct rather than collapsing them onto their shared
    /// name.
    #[trace("TC-258", "FR-088-AC-6")]
    #[test]
    fn equal_qualified_names_do_not_collapse_distinct_declarations() {
        let name = QualifiedName::unqualified("f").unwrap();
        let (first, _) = mint_resolved(
            &empty_scope(),
            DEFAULT_PACKAGE_IDENTITY,
            &declaration("f", Expression::Boolean(true)),
            u64::MAX,
        );
        let (second, _) = mint_resolved(
            &empty_scope(),
            "other-package@1.0.0",
            &declaration("f", Expression::Boolean(true)),
            u64::MAX,
        );
        assert_ne!(
            first, second,
            "distinct declarations must not share a node id"
        );

        let bytes = emit_v2(&[(name.clone(), first), (name.clone(), second)]);
        let decoded = decode_v2(&bytes).unwrap();
        assert_eq!(decoded, vec![(name.clone(), first), (name, second)]);
    }

    /// F16 (rust-review, pre-handoff pass): `#[serde(deny_unknown_fields)]`
    /// on `FunctionPackageV2`/`FunctionEntryV2` (decoded from externally
    /// supplied bytes through the `pub` `decode_function_package_v2`)
    /// actually refuses an unknown field, rather than silently dropping it.
    #[test]
    fn decode_v2_refuses_an_unknown_top_level_field() {
        let bytes =
            br#"{"version":"quire.checked-function-package/v2","functions":[],"extra":true}"#;
        assert_eq!(decode_v2(bytes), Err(DecodeV2Error::Malformed));
    }

    /// F16: same, for an unknown field on one entry rather than the
    /// top-level package.
    #[test]
    fn decode_v2_refuses_an_unknown_entry_field() {
        let bytes = br#"{"version":"quire.checked-function-package/v2","functions":[{"name":"f","identity":"00000000000000000000000000000000000000000000000000000000000000","extra":true}]}"#;
        assert_eq!(decode_v2(bytes), Err(DecodeV2Error::Malformed));
    }

    /// Existing (pre-#262-review) coverage this pass confirmed is real: an
    /// unrecognised version and a non-hex identity are each refused with
    /// their own distinct variant, not `Malformed`.
    #[test]
    fn decode_v2_distinguishes_version_and_identity_refusals() {
        let wrong_version = br#"{"version":"quire.checked-function-package/v1","functions":[]}"#;
        assert_eq!(decode_v2(wrong_version), Err(DecodeV2Error::Version));

        let bad_identity = br#"{"version":"quire.checked-function-package/v2","functions":[{"name":"f","identity":"not-hex"}]}"#;
        assert_eq!(decode_v2(bad_identity), Err(DecodeV2Error::InvalidIdentity));
    }

    /// F6 (rust-review, PR #262 review): `FunctionEntryV2.name` is typed on
    /// `QualifiedName` (`#[serde(try_from = "String")]`), so a wire `name`
    /// that is not a valid qualified name fails to deserialize at all --
    /// `serde_json::from_slice` itself returns `Err`, which `decode_v2`
    /// already maps to `Malformed`. A bare `String` field would have
    /// accepted this silently.
    #[test]
    fn decode_v2_refuses_a_non_identifier_name() {
        let identity = NodeKey::from_digest([7; 32]);
        let bytes = format!(
            r#"{{"version":"{FUNCTION_PACKAGE_V2_VERSION}","functions":[{{"name":"not an identifier","identity":"{identity}"}}]}}"#
        );
        assert_eq!(decode_v2(bytes.as_bytes()), Err(DecodeV2Error::Malformed));
    }
}
