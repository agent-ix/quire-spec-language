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
//! `check` builds from it -- into [`crate::check::family`], since FR-068-
//! AC-3 forbids `check` importing anything from `value::expression`; see
//! that module's own doc for why the split runs through `family.rs` even
//! though FR-068 itself only names seven of `value::expression`'s eight
//! submodules. `ValueFunctionFamily` is re-exported from there
//! ([`crate::check::ValueFunctionFamily`]) and this module implements its
//! evaluation half over it -- layer 5 depending on layer 3 is the
//! permitted direction (ADR-011 §6.1).

use qsl_attrs::string_edge;

use quire_exact::NodeKey;

use crate::check::ValueFunctionFamily;

/// ADR-013 O-11: a non-empty sequence of identifiers, `::`-separated on
/// display -- the layer-6 `replay` facade's (and, for this ticket,
/// [`super::CheckedPackage::call`]'s) only function-selection key. Never a
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

fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

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
/// `pub` [`decode_function_package_v2`], from bytes an external caller
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
/// [`mint_declaration_identity`]'s doc) -- it exists to demonstrate, and let
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
/// family marker type. [`super::CheckedPackage::call`] is this environment's
/// one real (non-test) constructor.
pub(crate) struct EvaluationEnv<'a> {
    pub(crate) package: &'a super::CheckedPackage,
    pub(crate) objects: &'a super::super::reference::ObjectEnvironment,
    pub(crate) arguments: Option<Vec<super::super::composite::Value>>,
    pub(crate) local_meter: &'a mut quire_exact::Meter,
}

impl crate::family::ReferenceEvaluation for ValueFunctionFamily {
    type Observed = super::Evaluation;
    type Env<'a> = EvaluationEnv<'a>;

    /// FR-062-AC-6: reads only `checked` (a bare identity) and `env`'s
    /// checked package/object environment/meter -- no CST, token or
    /// display string. `meter` (the shared kernel meter every family's
    /// `evaluate` takes) is genuinely charged now (QSL-153): one
    /// `ChargePoint::FunctionCall` -- "one checked function call"'s own
    /// documented meaning, matching what this hook is about to run -- per
    /// call, denied into `EvaluateFailure::Incomplete` exactly when the
    /// caller-configured `meter` cannot afford it.
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
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut EvaluationEnv<'a>,
        meter: &mut quire_exact::Meter,
    ) -> Result<super::Evaluation, crate::family::EvaluateFailure> {
        let function = env
            .package
            .graph()
            .function_by_identity(*checked)
            .ok_or(crate::family::EvaluateRefusal::UnknownIdentity { identity: *checked })?;
        meter
            .charge(quire_exact::Charge::new(
                quire_exact::ChargePoint::FunctionCall,
            ))
            .map_err(crate::family::EvaluateFailure::Incomplete)?;
        // PR #262 review (F17): `.take().unwrap_or_default()` used to
        // silently evaluate with zero arguments if `arguments` were ever
        // `None` -- which, since `take()` itself leaves it `None`, is
        // exactly what a second `evaluate` call on the same `env` would
        // hit, and it would look like a legitimate zero-argument call
        // rather than the reused-env bug it actually is.
        // `EvaluationEnv`'s one real constructor (`CheckedPackage::call`)
        // always builds a fresh env with `Some(arguments)` and calls
        // `evaluate` exactly once, so this refusal is unreached today; it
        // exists so a future second call surfaces as a typed refusal
        // instead of a wrong, silent answer. Its own distinct variant, not
        // `UnknownIdentity`'s (PR #262 review, finding F3, this round): the
        // two are different conditions, and the one caller mapping this
        // into a public `InputRefusal` (`value::expression::mod.rs`'s
        // `CheckedPackage::call`) must be able to tell them apart.
        let arguments = env
            .arguments
            .take()
            .ok_or(crate::family::EvaluateRefusal::EnvironmentAlreadyConsumed)?;
        let callables = env.package.callables();
        Ok(super::evaluate::Machine::new(
            env.package.graph().scope(),
            &callables,
            env.objects,
            env.local_meter,
            env.package.graph().dispatch_tables(),
        )
        .run(function.body, function.slots, arguments))
    }
}

// FR-062-AC-8/FR-063-AC-6 (ADR-012 §5.1 S4, "each family Cause enum's
// catalog_code()") is deferred, not delivered by this ticket, and there is
// deliberately no `DeclarationCause` type here to carry it.
//
// An earlier version of this file kept an uninhabited `DeclarationCause`
// with a `#[cfg(seam_probe)]` probe variant, on the theory that the probe
// build would demonstrate S4 the same way it demonstrates S1 (`FamilyKind`).
// It does not: `FamilyKind`'s probe works because six real variants and
// real production `match`es already exist, so adding a seventh variant
// breaks matches nothing else could reach otherwise -- a real author has to
// notice. `DeclarationCause` had no real variant and no production `match`
// beside its own `catalog_code()`; the only thing that could ever fail
// under the probe was that same function gaining one more arm. That is the
// construct testing itself, not a seam. Value's function-declaration
// `check` mints identity unconditionally once nesting is charged, so it has
// no typed refusal cause -- a family with zero causes cannot demonstrate a
// per-family cause seam, the same shape `Requirements`, `PackageRefusal` and
// `StageFailure::Refused` were. QSL-152 owns adding a real `Cause` enum,
// `catalog_code()` and the S4 probe variant for the first family that has
// one, validated against real content instead of guessed here
// (FR-062-AC-8's S4 seam-probe coverage).

#[cfg(test)]
mod family_contract_tests {
    use super::*;
    use crate::check::{
        mint_declaration_identity, CheckingLimits, PackageDeclarations, DEFAULT_PACKAGE_IDENTITY,
        SCALAR_LIMITS_UNLIMITED,
    };
    use crate::family::{
        CheckContext, DiagnosticSink, EvaluateFailure, EvaluateRefusal, FamilyContract,
        ReferenceEvaluation, ScopeStack, StageLimits,
    };
    use crate::forms::{Expression, FunctionDeclaration};
    use crate::value::composite::{TypeEnvironment, ValueType};
    use crate::value::reference::ObjectEnvironment;
    use ix_trace_rs::trace;
    use quire_exact::Meter;

    // `EvaluationEnv::local_meter` (`ValueFunctionFamily::evaluate`'s
    // pre-existing accounting path) and `ReferenceEvaluation::evaluate`'s
    // own `_meter` parameter are both `quire_exact::Meter` since QSL-166
    // (previously two distinct types with the same name in different
    // crates) -- `evaluate_refuses_a_second_call_on_the_same_env` still
    // needs two separate *instances*, one per role.

    fn limits() -> StageLimits {
        StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        }
    }

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
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
    #[test]
    fn value_function_family_checks_through_the_contract() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &package_identity,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let (expected, _) = mint_declaration_identity(&package_identity, &form, u64::MAX);
        let staged = ValueFunctionFamily::check(&form, &mut cx).unwrap();
        assert_eq!(staged.value, expected);
        assert_eq!(diagnostics.entries().len(), 1);
        let declaration_name = QualifiedName::unqualified("declaration").unwrap();
        let v2 = emit_v2(&[(declaration_name.clone(), staged.value)]);
        assert_eq!(decode_v2(&v2).unwrap(), vec![(declaration_name, expected)]);
    }

    /// PR #262 review, finding F17 (round 3, item 8): a second `evaluate`
    /// call on the same `EvaluationEnv` -- whose `arguments` the first call
    /// already consumed via `.take()` -- refuses with a typed
    /// `EvaluateRefusal` rather than silently evaluating with zero
    /// arguments. `EvaluationEnv`'s one real (non-test) constructor
    /// (`CheckedPackage::call`) never reaches this: it always builds a
    /// fresh env with `Some(arguments)` and calls `evaluate` exactly once,
    /// so the guard is unreached through that path. This test bypasses
    /// that constructor -- the same thing the F17 finding's own fix
    /// verified by hand and then reverted, leaving the fix itself
    /// unguarded -- and lands the guard with a real second call.
    #[test]
    fn evaluate_refuses_a_second_call_on_the_same_env() {
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
        let package = crate::package::CheckedPackage::link(graph, std::collections::BTreeMap::new());
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = EvaluationEnv {
            package: &package,
            objects: &objects,
            arguments: Some(Vec::new()),
            local_meter: &mut local_meter,
        };
        let mut contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect("first call, with real arguments still present, evaluates cleanly");
        let refusal = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect_err("second call on the same env, arguments already consumed, must refuse");
        assert_eq!(
            refusal,
            EvaluateFailure::Refused(EvaluateRefusal::EnvironmentAlreadyConsumed)
        );
    }

    /// FR-062-AC-5's `Incomplete` half (QSL-153): `evaluate` genuinely
    /// charges its own kernel `meter` parameter (`ChargePoint::FunctionCall`,
    /// `ValueFunctionFamily::evaluate`'s own doc) -- given a meter whose
    /// `work_units` limit is already exhausted, that charge is denied and
    /// `evaluate` returns `EvaluateFailure::Incomplete`, a real
    /// `quire_exact::Incomplete`, never folded into `EvaluateRefusal`.
    /// `check`, run against the same declaration, admits it normally: its
    /// return type (`CheckOutcome`/`StageFailure`) has no `Incomplete`
    /// variant to return in the first place, so the two hooks cannot be
    /// confused by construction, not merely by this test's assertions.
    ///
    /// **Strengthened (PR #302 review finding 7).** An earlier version only
    /// asserted the outer `Err(EvaluateFailure::Incomplete(_))` shape, which
    /// a mismatched-point or mismatched-counter denial would also satisfy.
    /// This asserts the denied record's own fields -- `charge_point` is
    /// exactly `FunctionCall` (not some other point this meter happened to
    /// deny), `limit_kind` is `WorkUnits` (the counter `work_units: 0`
    /// actually bounds, not `TextInputBytes` or another counter this fixture
    /// never touches) and `limit` is `0` (the exact configured bound, not
    /// merely "some limit") -- and that `env.arguments` is still `Some`:
    /// `evaluate` charges `meter` *before* `env.arguments.take()`
    /// (`ValueFunctionFamily::evaluate`'s own body), so a denied charge must
    /// never have consumed them. A version that charged `meter` after
    /// `take()` would leave `arguments` `None` here while still returning
    /// `Incomplete` -- this assertion is what would catch that reordering.
    #[trace("TC-160", "FR-062-AC-5")]
    #[test]
    fn evaluate_returns_incomplete_when_the_meter_is_exhausted() {
        let package = PackageDeclarations {
            functions: vec![declaration("f", Expression::Boolean(true))],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks cleanly, never Incomplete");
        let identity = package
            .function_identity("f")
            .expect("f is declared in this package");
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = EvaluationEnv {
            package: &package,
            objects: &objects,
            arguments: Some(Vec::new()),
            local_meter: &mut local_meter,
        };
        let exhausted_limits = quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        };
        let mut contract_meter = Meter::new(exhausted_limits);
        let outcome = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter);
        match outcome {
            Err(EvaluateFailure::Incomplete(record)) => {
                assert_eq!(record.charge_point, quire_exact::ChargePoint::FunctionCall);
                assert_eq!(record.limit_kind, quire_exact::LimitKind::WorkUnits);
                assert_eq!(record.limit, 0);
            }
            other => panic!("expected EvaluateFailure::Incomplete, got {other:?}"),
        }
        assert!(
            env.arguments.is_some(),
            "a denied charge must not have consumed env.arguments"
        );
    }

    /// PR #302 review finding 2: a *nested* nested call denies against
    /// `env.local_meter` (real production behaviour, not the contract-level
    /// `meter` parameter's own admission charge exercised above) surfaces as
    /// the pre-existing kernel `Outcome::Incomplete`, inside a successful
    /// `Evaluation`, from `Machine::run`'s own `charge_call` -- never
    /// `EvaluateFailure::Incomplete`. `caller`'s body calls `callee`; with
    /// `local_meter`'s `work_units` already exhausted, the nested call
    /// inside `caller`'s own body (not the top-level call to `caller`
    /// itself, which the unlimited `contract_meter` here admits) is what is
    /// denied.
    #[test]
    fn evaluate_returns_incomplete_when_a_nested_calls_local_meter_is_exhausted() {
        let package = PackageDeclarations {
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
        let identity = package
            .function_identity("caller")
            .expect("caller is declared in this package");
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let exhausted_limits = quire_exact::ScalarLimits {
            work_units: 0,
            ..SCALAR_LIMITS_UNLIMITED
        };
        let mut local_meter = Meter::new(exhausted_limits);
        let mut env = EvaluationEnv {
            package: &package,
            objects: &objects,
            arguments: Some(Vec::new()),
            local_meter: &mut local_meter,
        };
        let mut contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let outcome = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect(
            "the top-level call's own admission charge is against contract_meter, unlimited here",
        );
        assert!(
            matches!(outcome.outcome, crate::value::Outcome::Incomplete(_)),
            "expected kernel Outcome::Incomplete from the nested call's own denied charge, got {:?}",
            outcome.outcome
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

        let mut meter_a = Meter::new(scalar_limits);
        let mut diagnostics_a = DiagnosticSink::default();
        let mut scopes_a = ScopeStack::default();
        let mut cx_a = CheckContext::new(
            &package_identity,
            limits(),
            &mut meter_a,
            &mut diagnostics_a,
            &mut scopes_a,
        );
        ValueFunctionFamily::check(&form, &mut cx_a).unwrap();

        let mut meter_b = Meter::new(scalar_limits);
        let mut diagnostics_b = DiagnosticSink::default();
        let mut scopes_b = ScopeStack::default();
        let mut cx_b = CheckContext::new(
            &package_identity,
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

    /// **Tagged `FR-062-AC-5` (QSL-153), still not `FR-062-AC-7`.** AC-5's
    /// `Limit` half only requires that *some* limit be reached and reported
    /// as a `Limit` outcome naming its kind, not a refusal, checked node or
    /// `Incomplete` -- exactly what this test shows below, structurally
    /// (`StageFailure` has no `Refused`/`Incomplete` variant to confuse
    /// `Limit` with). It does not attempt AC-7's stronger, distinct claim
    /// (a fixture nested to a real depth D, limit varied by exactly one at
    /// D): this test varies only the nesting-depth limit (0 vs 1) against
    /// `check`, which calls `enter_nesting` exactly once per top-level
    /// declaration -- `check` performs no recursive descent of its own, so
    /// `depth` never exceeds 1 and the predicate this test exercises
    /// reduces to `0 >= nesting_depth`. Mutation confirms it: deleting
    /// `self.depth += 1` from `CheckContext::enter_nesting` (removing the
    /// nesting bound entirely) leaves this test passing unchanged, because
    /// it never calls `enter_nesting` more than once to observe the
    /// increment. QSL-148 owns AC-7's real recursive-descent fixture.
    #[trace("TC-160", "FR-062-AC-5")]
    #[test]
    fn nesting_depth_limit_is_the_proximate_cause() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut tight = StageLimits {
            nesting_depth: 0,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };
        let mut cx = CheckContext::new(
            &package_identity,
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
            &package_identity,
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
        let form = declaration("f", Expression::Boolean(true));
        let (_, metrics) = mint_declaration_identity(&package_identity, &form, u64::MAX);
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
                &package_identity,
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
                &package_identity,
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
            &package_identity,
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
            &package_identity,
            limits,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        assert!(ValueFunctionFamily::check(&form, &mut cx).is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{mint_declaration_identity, OccurrenceMap, DEFAULT_PACKAGE_IDENTITY};
    use crate::forms::{Expression, FunctionDeclaration};
    use crate::value::composite::ValueType;
    use ix_trace_rs::trace;

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
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
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &a, u64::MAX).0,
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &b, u64::MAX).0
        );
        assert_ne!(
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &a, u64::MAX).0,
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &c, u64::MAX).0
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
        let (after_check, _) =
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &declaration, u64::MAX);
        let name = QualifiedName::unqualified("f").unwrap();
        let bytes = emit_v2(&[(name.clone(), after_check)]);
        let decoded = decode_v2(&bytes).unwrap();
        assert_eq!(decoded, vec![(name, after_check)]);
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
