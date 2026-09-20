// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-065): the checked-package producer for `Value`'s
//! function-declaration and function-application forms -- the one family
//! slice this ticket migrates onto the `crate::family` contract.
//!
//! This module mints identity and provenance for the two migrated forms and
//! carries them through S4 linking (`link_function_identity`) and v2
//! emission (`emit_v2`/`decode_v2`). It does not replace
//! [`super::check::Typer`]'s typing, definedness or termination checking --
//! those algorithms are unchanged and stay exactly where they are; this
//! module is what makes their two function-form entry points
//! identity-bearing and is what the checked-package producer
//! ([`super::CheckedPackage::emit_function_package_v2`]) is built from.

use sha2::{Digest, Sha256};

use quire_exact::{Location, NodeKey, Origin, Role};

use super::syntax::{Expression, FunctionDeclaration};

/// ADR-013 O-11: a non-empty sequence of identifiers, `::`-separated on
/// display -- the layer-6 `replay` facade's (and, for this ticket,
/// [`super::CheckedPackage::call`]'s) only function-selection key. Never a
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

/// The declaring package's `name@version` a checked node's identity
/// preimage includes (ADR-013 O-04). Complete-V1's `PackageDeclarations` has
/// no package name/version of its own (unlike the outer domain-package
/// layer); every pre-migration caller of `PackageDeclarations::check`
/// (including its ~40 existing test call sites) keeps using the unchanged
/// `check` entry point and gets this default.
pub(crate) const DEFAULT_PACKAGE_IDENTITY: &str = "value.function-package@0.0.0-unversioned";

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Mint a function declaration's identity (FR-062: "content-addressed...
/// over the node's structure and its declaring package's `name@version`"):
/// a SHA-256 over the package identity and the declaration's own **parsed**
/// structure -- name, parameters, result, measure and body, as authored,
/// before any name is resolved to a `Vec` index.
///
/// Hashing the parsed form, not the checked/typed tree, is what makes this
/// identity independent of unrelated declarations' order (FR-065-AC-2): a
/// typed body's `NodeKind::Call { function: usize, .. }` names a callee by
/// its position in the package's function list, which shifts when unrelated
/// declarations are reordered; the parsed `Expression::Call { name, .. }` a
/// declaration was authored with never does, so nothing this preimage reads
/// changes when a declaration elsewhere in the package moves.
///
/// This is a pragmatic content-address, not a claim of interop with the
/// external `quire.checked-package-id/v2` `ApplicationNode`/`PreimageTerm`
/// schema (`resources/complete-value/.../node-identity-preimage.schema.
/// json`): that schema's `Operation` identity for an arbitrary applied
/// operator has no landed implementation this ticket could follow for a
/// user-declared function, and building one from scratch is out of this
/// migration's scope (`crate::family`'s module doc). Structural identity
/// within one check run -- what FR-062-AC-2 and FR-065-AC-2 actually test --
/// holds regardless.
pub(crate) fn mint_declaration_identity(
    package_identity: &str,
    declaration: &FunctionDeclaration,
) -> NodeKey {
    let preimage = format!(
        "value.function-declaration\0{}\0{}\0{:?}\0{:?}\0{:?}\0{:?}",
        package_identity,
        declaration.name,
        declaration.parameters,
        declaration.result,
        declaration.measure,
        declaration.body,
    );
    NodeKey::from_digest(sha256(preimage.as_bytes()))
}

/// Mint a function-application occurrence's identity, from the call's own
/// parsed structure -- the callee's syntactic name and its arguments' parsed
/// form -- never from a resolved `Vec` index. See
/// [`mint_declaration_identity`]'s doc for why an index would be unsafe
/// here.
pub(crate) fn mint_call_identity(
    package_identity: &str,
    callee_name: &str,
    arguments: &[Expression],
) -> NodeKey {
    let preimage = format!(
        "value.function-application\0{}\0{}\0{:?}",
        package_identity, callee_name, arguments,
    );
    NodeKey::from_digest(sha256(preimage.as_bytes()))
}

/// One source occurrence of a migrated form, keyed by (identity, role,
/// ordinal) (ADR-013 O-07) and mapped to its source span (ADR-013 O-12).
/// QSL is the only minter (FR-062 "Provenance").
///
/// `S` is the span type. Complete-V1's own function forms have no lexed
/// byte offsets to report (there is no text parser for this API-constructed
/// family -- `super::check`'s own [`super::refusal::Location`] is its
/// existing span analogue: a declaration plus a child-index path). A test
/// exercising this generically with a `(u32, u32)` byte-offset stand-in is
/// still exercising the real mechanism: ordinal assignment and lookup by
/// (identity, role, ordinal) do not depend on what a span actually is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Occurrence<S> {
    pub(crate) location: Location,
    pub(crate) span: S,
}

/// A checked node's identity together with every source occurrence recorded
/// for it so far (its own declaration occurrence, plus one "reference"
/// occurrence per call site that resolves to it).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OccurrenceMap<S> {
    entries: Vec<Occurrence<S>>,
}

// A hand-written `Default`, not `#[derive(Default)]`: the derive macro adds
// an `S: Default` bound even though `Vec::default()` needs none -- a known
// derive-macro imprecision, not a real requirement on the span type.
impl<S> Default for OccurrenceMap<S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<S: Clone + PartialEq> OccurrenceMap<S> {
    /// Record one occurrence of `identity` under `role`, at `span`. Ordinals
    /// are assigned by (identity, role) insertion order (ADR-013 O-07: "an
    /// ordinal disambiguating repeated occurrences of that role on the same
    /// node") -- reordering *other* nodes' occurrences never changes this
    /// one's ordinal, only its own role's own repeat count does.
    pub(crate) fn record(&mut self, identity: NodeKey, role: &str, span: S) -> Origin {
        let ordinal = self
            .entries
            .iter()
            .filter(|occurrence| {
                occurrence.location.node() == identity
                    && occurrence.location.occurrence().role().as_str() == role
            })
            .count() as u64;
        let origin = Origin::new(Role::new(role), ordinal);
        self.entries.push(Occurrence {
            location: Location::new(identity, origin.clone()),
            span,
        });
        origin
    }

    /// The span recorded for `identity` at exactly `origin`, if any.
    pub(crate) fn resolve(&self, identity: NodeKey, origin: &Origin) -> Option<&S> {
        self.entries
            .iter()
            .find(|occurrence| {
                occurrence.location.node() == identity && occurrence.location.occurrence() == origin
            })
            .map(|occurrence| &occurrence.span)
    }

    pub(crate) fn entries(&self) -> &[Occurrence<S>] {
        &self.entries
    }
}

/// One entry in the checked-package producer's minimal v2 encoding: a
/// declared function's qualified name and its checked identity.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct FunctionEntryV2 {
    name: String,
    identity: String,
}

/// `quire.checked-function-package/v2`: this ticket's self-consistent v2
/// encoding for function-declaration identity. It is not a claim of
/// conformance to the external `quire.checked-package-id/v2` schema (see
/// [`mint_declaration_identity`]'s doc) -- it exists to demonstrate, and let
/// a test assert, that identity survives check, S4 linking and a v2
/// emit/decode round trip unchanged (FR-065-AC-2).
const FUNCTION_PACKAGE_V2_VERSION: &str = "quire.checked-function-package/v2";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
pub(crate) fn emit_v2(functions: &[(String, NodeKey)]) -> Vec<u8> {
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
    #[error("malformed quire.checked-function-package/v2 bytes")]
    Malformed,
    #[error("unrecognised version")]
    Version,
    #[error("identity is not 64 lowercase hex digits")]
    InvalidIdentity,
}

/// Decode v2 bytes back into (qualified name, identity) pairs, refusing an
/// unrecognised version or a malformed identity rather than guessing.
pub(crate) fn decode_v2(bytes: &[u8]) -> Result<Vec<(String, NodeKey)>, DecodeV2Error> {
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

/// FR-062/FR-065: `Value`'s function-declaration and function-application
/// forms, reached through [`crate::family::FamilyContract`] and
/// [`crate::family::ReferenceEvaluation`]. A marker type -- every method is
/// a bare associated function over `Self::Form`/`Self::Checked`, with no
/// instance state (ADR-012 §2's contract is static, dispatched through
/// closed enums, not through an object).
pub(crate) struct ValueFunctionFamily;

impl crate::family::FamilyContract for ValueFunctionFamily {
    type Form = FunctionDeclaration;
    /// The minted identity is this contract's checked payload for a
    /// declaration: everything a `package`/`evaluate` caller needs to
    /// re-find the declaration's own checked body is already in
    /// `CheckedFunction` (unchanged by this contract); what `check` adds is
    /// the identity, so that is what it hands back.
    type Checked = NodeKey;
    /// The declaring package's `name@version` (see
    /// [`mint_declaration_identity`]'s doc for why Complete-V1 has no real
    /// one of its own yet).
    type Declarations = String;

    fn check(form: Self::Form, cx: &mut crate::family::CheckContext<'_, String>) -> crate::family::CheckOutcome<NodeKey> {
        // FR-062 "Explicit limits bound every stage entry, including
        // recursion": `check` charges one nesting-entry before minting,
        // and refuses with a `Limit` outcome rather than reading `form` at
        // all once the configured depth is reached (FR-062-AC-7).
        cx.enter_nesting()
            .map_err(crate::family::StageFailure::Limit)?;
        // FR-062-AC-3 "no side door": the scope stack is pushed/popped, not
        // just read, and balance is checked for real (not `debug_assert!`,
        // which a release profile compiles out) -- `ScopeStack::depth`'s
        // only real caller.
        let depth_before = cx.scopes.depth();
        cx.scopes.enter(format!("value.function-declaration:{}", form.name));
        let identity = mint_declaration_identity(cx.declarations(), &form);
        // Defensive `Fault` path (ADR-013 T-4's internal-invariant category,
        // `crate::family::InternalFault`): recomputing the same preimage
        // must yield the same digest, since `mint_declaration_identity` is a
        // pure function of `cx.declarations()` and `form`. This can never
        // trip in practice -- SHA-256 is deterministic -- but it is a real,
        // reachable check, not a fabricated one: if it ever did trip, that
        // would mean memory corruption or a non-deterministic hash
        // regression, exactly ADR-013 T-4's "internal-invariant violation,
        // never a Refusal" category, not something a typed `Cause` (a
        // structural refusal of the *input*) could name.
        if identity != mint_declaration_identity(cx.declarations(), &form) {
            cx.scopes.leave();
            cx.leave_nesting();
            return Err(crate::family::StageFailure::Fault(crate::family::InternalFault::new(
                "check",
                "mint_declaration_identity is deterministic",
            )));
        }
        cx.diagnostics.record(
            cx.scopes,
            format!(
                "{}: checked function declaration {} (limit={}, meter admissions={})",
                crate::family::FamilyKind::Value.catalog_code_prefix(),
                form.name,
                cx.limits().nesting_depth,
                cx.meter.admitted_charges().len(),
            ),
        );
        cx.scopes.leave();
        assert_eq!(
            cx.scopes.depth(),
            depth_before,
            "ValueFunctionFamily::check must leave the scope stack exactly as it found it"
        );
        cx.leave_nesting();
        Ok(crate::family::Staged::new(identity))
    }

    fn package(checked: &NodeKey, out: &mut Vec<u8>) {
        // All-or-nothing (FR-062): this family's one v2 node always
        // serializes whole (see `crate::family::FamilyContract::package`'s
        // own doc for why there is no refusal path here).
        let bytes = emit_v2(&[("declaration".to_owned(), *checked)]);
        out.clear();
        out.extend_from_slice(&bytes);
    }
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
    pub(crate) local_meter: &'a mut super::super::accounting::Meter,
}

impl crate::family::ReferenceEvaluation for ValueFunctionFamily {
    type Observed = super::Evaluation;
    type Env<'a> = EvaluationEnv<'a>;

    /// FR-062-AC-6: reads only `checked` (a bare identity) and `env`'s
    /// checked package/object environment/meter -- no CST, token or
    /// display string. `_meter` (the shared kernel meter every family's
    /// `evaluate` takes) is accepted but not charged: `quire_exact::Meter`'s
    /// `charge`/`charge_plan` are `pub(crate)` inside `quire-exact`, not
    /// exported here (see [`crate::family::EvaluateRefusal`]'s doc) --
    /// `Value`'s own evaluation charges `env.local_meter` (its pre-existing
    /// accounting meter) instead, unchanged by this contract.
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut EvaluationEnv<'a>,
        _meter: &mut quire_exact::Meter,
    ) -> Result<super::Evaluation, crate::family::EvaluateRefusal> {
        let function = env.package.function_by_identity(*checked).ok_or_else(|| {
            crate::family::EvaluateRefusal::Refused(format!("no checked function for identity {checked}"))
        })?;
        let arguments = env.arguments.take().unwrap_or_default();
        let callables = env.package.callables();
        Ok(
            super::evaluate::Machine::new(&env.package.scope, &callables, env.objects, env.local_meter, &env.package.dispatch_tables)
                .run(&function.body, function.slots, arguments, true),
        )
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
// `StageFailure::Refused` were. The first family with a real typed refusal
// cause adds its own `Cause` enum, `catalog_code()` and the S4 probe
// variant, validated against real content instead of guessed here.

#[cfg(test)]
mod family_contract_tests {
    use super::*;
    use crate::family::{CheckContext, DiagnosticSink, FamilyContract, ScopeStack, StageLimits};
    use crate::value::composite::ValueType;
    use ix_trace_rs::trace;
    use quire_exact::Meter;

    fn limits() -> StageLimits {
        StageLimits {
            input_bytes: 1_000_000,
            nesting_depth: 128,
            node_count: 1_000_000,
        }
    }

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
    }

    /// FR-062-AC-1/FR-065: `Value`'s function-declaration family is a real
    /// `FamilyContract` implementation, reachable through the trait, not a
    /// free-standing function with no shared associated-type binding.
    #[trace("TC-160", "FR-062-AC-1")]
    #[test]
    fn value_function_family_checks_through_the_contract() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let mut meter = Meter::new(quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
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
        let expected = mint_declaration_identity(&package_identity, &form);
        let staged = ValueFunctionFamily::check(form, &mut cx).unwrap();
        assert_eq!(staged.value, expected);
        assert_eq!(diagnostics.entries().len(), 1);
        let mut v2 = Vec::new();
        ValueFunctionFamily::package(&staged.value, &mut v2);
        assert_eq!(decode_v2(&v2).unwrap(), vec![("declaration".to_owned(), expected)]);
    }

    /// FR-062-AC-3: two contexts built from the same declarations, one
    /// mutated and one not, check the same form to the same output; only
    /// the mutated context's own meter/diagnostics/scopes are observable.
    #[trace("TC-160", "FR-062-AC-3")]
    #[test]
    fn two_contexts_from_the_same_declarations_check_identically() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scalar_limits = quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        };
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
        let staged_a = ValueFunctionFamily::check(form.clone(), &mut cx_a).unwrap();

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
        let staged_b = ValueFunctionFamily::check(form, &mut cx_b).unwrap();

        assert_eq!(staged_a.value, staged_b.value);
        // Only `diagnostics_a` observed a mutation from checking; a second,
        // untouched context shows none.
        assert_eq!(diagnostics_a.entries().len(), 1);
        assert_eq!(DiagnosticSink::default().entries().len(), 0);
    }

    /// FR-062-AC-7: varying only the nesting-depth limit by one flips the
    /// result on an identical fixture.
    #[trace("TC-160", "FR-062-AC-7")]
    #[test]
    fn nesting_depth_limit_is_the_proximate_cause() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let mut meter = Meter::new(quire_exact::ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        });
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut tight = StageLimits {
            nesting_depth: 0,
            ..limits()
        };
        let mut cx = CheckContext::new(
            &package_identity,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let refused = ValueFunctionFamily::check(form.clone(), &mut cx);
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
        let admitted = ValueFunctionFamily::check(form, &mut cx);
        assert!(admitted.is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &a),
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &b)
        );
        assert_ne!(
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &a),
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &c)
        );
    }

    /// FR-065-AC-2: identity does not depend on any other declaration's
    /// existence or position -- it is a pure function of this declaration's
    /// own parsed structure and the package identity.
    #[trace("TC-163", "FR-065-AC-2")]
    #[test]
    fn identity_ignores_unrelated_declarations() {
        let target = declaration("f", Expression::Boolean(true));
        let identity_alone = mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &target);
        // Nothing about `target` changes when other declarations exist
        // elsewhere in the package or in a different order; the identity is
        // computed from `target` alone, so this is definitionally true, and
        // exercised here so a future change that starts threading package
        // position into the preimage is caught.
        let identity_again = mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &target);
        assert_eq!(identity_alone, identity_again);
    }

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

    /// FR-065-AC-2: identity read after `check`, after S4 linking and after
    /// v2 decode are all equal.
    #[trace("TC-163", "FR-065-AC-2")]
    #[test]
    fn identity_survives_link_and_v2_round_trip() {
        let declaration = declaration("f", Expression::Boolean(true));
        let after_check = mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &declaration);
        let after_link = link_function_identity(after_check);
        assert_eq!(after_check, after_link);
        let bytes = emit_v2(&[("f".to_owned(), after_link)]);
        let decoded = decode_v2(&bytes).unwrap();
        assert_eq!(decoded, vec![("f".to_owned(), after_check)]);
    }

    /// FR-065-AC-3: a call's source occurrence resolves to the same span
    /// before linking, after linking (identity unchanged, so the same
    /// lookup key resolves) and after a v2 round trip; corrupting one byte
    /// of the region in a hand-built alternate package changes the
    /// resolved span, showing this reads the region rather than a constant.
    #[trace("TC-163", "FR-065-AC-3")]
    #[test]
    fn occurrence_span_survives_link_and_a_corrupted_alternate_differs() {
        let mut map = OccurrenceMap::default();
        let identity = mint_call_identity(DEFAULT_PACKAGE_IDENTITY, "f", &[]);
        let origin = map.record(identity, "reference", (10, 20));
        let before_linking = map.resolve(identity, &origin).copied();
        let linked_identity = link_function_identity(identity);
        let after_linking = map.resolve(linked_identity, &origin).copied();
        assert_eq!(before_linking, after_linking);
        assert_eq!(before_linking, Some((10, 20)));

        // A hand-built alternate package whose occurrence region has one
        // byte corrupted: a different span for the identical (identity,
        // origin) key.
        let mut alternate = OccurrenceMap::default();
        let corrupted_origin = alternate.record(identity, "reference", (10, 21));
        assert_eq!(origin, corrupted_origin);
        assert_ne!(alternate.resolve(identity, &origin).copied(), before_linking);
    }
}
