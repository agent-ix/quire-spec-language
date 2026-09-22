// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-062-AC-1, ADR-012 §2: the static `FamilyContract`/`ReferenceEvaluation`
//! traits and the mutable typing context (`CheckContext`) every family's
//! `check` receives.

use super::outcome::CheckOutcome;
use qsl_foundation::diagnostic::InternalFault;
use quire_exact::Meter;

/// A diagnostic a family's `check` records against the scope it was raised
/// in. The shared layer defines no family-specific diagnostic content; this
/// is the sink's own record shape (a message plus the scope name active when
/// it was raised), independent of any one family's `Cause` enum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Diagnostic {
    pub(crate) scope: String,
    pub(crate) message: String,
}

/// The only mutable diagnostic sink `check` may write through
/// (FR-062-AC-3): an ordinary append-only log, read back for assertions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DiagnosticSink {
    entries: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub(crate) fn record(&mut self, scope: &ScopeStack, message: impl Into<String>) {
        self.entries.push(Diagnostic {
            scope: scope.current().to_owned(),
            message: message.into(),
        });
    }

    /// Read back for test assertions only (PR #262 review, coordinator
    /// round 3): this crate's one non-test caller was a fabricated
    /// coherence check, deleted (`value/expression/mod.rs`'s own doc at
    /// its former call site). `#[cfg(test)]`, not `#[allow(dead_code)]`:
    /// this is genuinely test-only infrastructure -- `check`'s real
    /// production callers never need to read the sink back, only write
    /// through it -- so the compiler is told that directly rather than
    /// having the lint silenced over a real (non-test) reader that does
    /// not exist.
    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[Diagnostic] {
        &self.entries
    }
}

/// The only mutable scope stack `check` may push/pop through
/// (FR-062-AC-3). Named scopes only -- no family-specific payload.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScopeStack {
    frames: Vec<String>,
}

impl ScopeStack {
    pub(crate) fn enter(&mut self, name: impl Into<String>) {
        self.frames.push(name.into());
    }

    pub(crate) fn leave(&mut self) {
        self.frames.pop();
    }

    pub(crate) fn current(&self) -> &str {
        self.frames.last().map(String::as_str).unwrap_or("<root>")
    }
}

/// Explicit stage-entry limits (FR-062 "Explicit limits bound every stage
/// entry, including recursion"; ADR-013 T-4). `check` is a recursive stage
/// (ADR-011 §2.3) and bounds its own recursion by `nesting_depth`, checked
/// before each recursive step -- never by the native call stack.
///
/// **`input_bytes` and `node_count` restored (QSL-153).** PR #262 review
/// deleted these, along with `work_budget`: nothing in #214's one migrated
/// stage entry produced or read them. QSL-153 restores them with a real
/// producer and a real consumer that changes behaviour (ADR-012 §14.1's own
/// row for this ticket), matching the shape [`CheckContext::enter_nesting`]
/// already established for `nesting_depth`:
///
/// - **Producer**: `crate::check::family::mint_declaration_identity`'s own
///   preimage pass already builds a length-prefixed byte buffer
///   over the declaration's structure and walks every [`crate::forms::
///   Expression`] node in it to do so -- real work this contract already
///   does, not a synthetic counter added only to satisfy this struct.
///   `input_bytes` is that buffer's own logical byte length (accumulated as
///   the pass writes, not read back from the buffer afterward -- see
///   `Preimage`'s own doc for why); `node_count` is the number of
///   `Expression` nodes the same pass visits.
/// - **Consumer**: [`CheckContext::check_input_bytes`] and
///   [`CheckContext::check_node_count`] each compare their metric against
///   this struct's matching field and return
///   [`StageLimitKind`](super::outcome::StageLimitKind)'s matching variant
///   on the first one exceeded, exactly like `enter_nesting`'s own
///   `NestingDepth` case -- `ValueFunctionFamily::check`
///   (`crate::check::family`) calls both before minting succeeds, so a
///   declaration whose preimage is too large or has too many nodes is
///   refused with a `Limit` outcome naming the exhausted kind, not admitted
///   silently.
///
/// **`work_budget` is not a field here (PR #302 review finding 3).** An
/// earlier version of this struct also carried `work_budget: u64`, compared
/// against the same preimage pass's own field-write count -- but "how many
/// times the encoder wrote" is not a caller-configured budget in any
/// meaningful sense; two declarations of equal real complexity could differ
/// in write count for reasons internal to the encoding, not to any resource
/// a caller actually wants to bound. `StageLimitKind::WorkBudget` is
/// restored instead through [`CheckContext::meter`] -- the *shared kernel*
/// budget every family's `check` already receives (FR-062 "checked input"):
/// `ValueFunctionFamily::check` charges it one `ChargePoint::
/// DeclarationCheck`, sized by the same preimage pass's field-write count,
/// and maps a denied charge to `Limit{WorkBudget}` naming the meter's own
/// configured `work_units` bound. This is a real, cumulative budget across
/// every declaration `check` runs against one `meter` instance, not a
/// per-declaration high-water field re-read from scratch each time --
/// `nesting_depth`/`input_bytes`/`node_count` bound one declaration's own
/// shape; `work_budget` bounds the checking stage's total spend.
///
/// `input_bytes`/`node_count`'s one production call site
/// (`crate::check::mod::PackageDeclarations::check`) configures both as
/// unlimited (`u64::MAX`) by default, the same real-default shape
/// `nesting_depth` itself carried before a caller-configurable knob existed
/// for it (`CheckingLimits::default()`, `crate::check::check`) --
/// `input_bytes` now has one (`CheckingLimits::with_input_bytes`); the
/// mechanism is real and exercised directly against tight fixtures
/// (`src/value/expression/family.rs`'s `family_contract_tests`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StageLimits {
    pub(crate) nesting_depth: u64,
    /// Maximum length-prefixed preimage byte count for one checked
    /// declaration.
    pub(crate) input_bytes: u64,
    /// Maximum `Expression` node count for one checked declaration.
    pub(crate) node_count: u64,
}

/// The mutable typing context every family's `check` receives (ADR-012 §2,
/// FR-062 "checked input"). `declarations` and `limits` are read-only
/// through it; `meter`, `diagnostics` and `scopes` are the only mutable
/// parts. `check` reads no global or thread-local state -- everything it can
/// observe or mutate is reachable only through this one `&mut` parameter.
///
/// `D` is the family's own read-only resolved-declarations/type-environment
/// type; the shared contract takes no position on its shape.
pub(crate) struct CheckContext<'a, D> {
    declarations: &'a D,
    limits: StageLimits,
    pub(crate) meter: &'a mut Meter,
    pub(crate) diagnostics: &'a mut DiagnosticSink,
    pub(crate) scopes: &'a mut ScopeStack,
    depth: u64,
}

impl<'a, D> CheckContext<'a, D> {
    pub(crate) fn new(
        declarations: &'a D,
        limits: StageLimits,
        meter: &'a mut Meter,
        diagnostics: &'a mut DiagnosticSink,
        scopes: &'a mut ScopeStack,
    ) -> Self {
        Self {
            declarations,
            limits,
            meter,
            diagnostics,
            scopes,
            depth: 0,
        }
    }

    /// Read-only resolved declarations and type environment.
    pub(crate) fn declarations(&self) -> &'a D {
        self.declarations
    }

    /// Read-only stage-entry limits.
    pub(crate) fn limits(&self) -> StageLimits {
        self.limits
    }

    /// Charge one step of recursive descent, refusing before the native
    /// stack would (FR-062-AC-7): the nesting-depth limit is the proximate
    /// cause of a refusal at exactly `limits.nesting_depth` levels, not the
    /// host stack. Bounds the caller's own explicit recursion (an iterative
    /// walk with an explicit counter satisfies this identically to native
    /// recursion, ADR-011 §2.3).
    pub(crate) fn enter_nesting(&mut self) -> Result<(), super::outcome::LimitExceeded> {
        if self.depth >= self.limits.nesting_depth {
            return Err(super::outcome::LimitExceeded::new(
                super::outcome::StageLimitKind::NestingDepth,
                self.limits.nesting_depth,
            ));
        }
        self.depth += 1;
        Ok(())
    }

    pub(crate) fn leave_nesting(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Refuse `amount` (a declaration's own preimage byte length,
    /// `StageLimits`'s own doc) once it exceeds `limits.input_bytes`
    /// (QSL-153, restoring the deleted `input_bytes` field with a real
    /// consumer).
    pub(crate) fn check_input_bytes(
        &self,
        amount: u64,
    ) -> Result<(), super::outcome::LimitExceeded> {
        if amount > self.limits.input_bytes {
            return Err(super::outcome::LimitExceeded::new(
                super::outcome::StageLimitKind::InputBytes,
                self.limits.input_bytes,
            ));
        }
        Ok(())
    }

    /// Refuse `amount` (a declaration's own visited `Expression` node
    /// count) once it exceeds `limits.node_count` (QSL-153).
    pub(crate) fn check_node_count(
        &self,
        amount: u64,
    ) -> Result<(), super::outcome::LimitExceeded> {
        if amount > self.limits.node_count {
            return Err(super::outcome::LimitExceeded::new(
                super::outcome::StageLimitKind::NodeCount,
                self.limits.node_count,
            ));
        }
        Ok(())
    }
}

/// ADR-012 §2's design-level `FamilyContract`, narrowed to the one part
/// #214's one migrated family (`Value`'s function declaration) can
/// genuinely exercise: `check`, required by the trait's own
/// associated-function signature, so a family that omits it fails to
/// compile (FR-062-AC-1, itself now unbacked past this one part -- see
/// FR-062's own amended Acceptance Criteria). `requirements` (the
/// contract's sixth part) and a typed refusal `Cause` are deferred -- see
/// `crate::family`'s module doc for `requirements`, and
/// [`super::outcome::StageFailure`]'s doc for `Cause`. Both are real
/// ADR-012 §2 design parts QSL-152 owns (FR-062-AC-1/AC-4/AC-6/AC-8/AC-9),
/// added against a real instance rather than guessed here.
///
/// **`package` is deleted (PR #262 review, findings F1/F2).** An earlier
/// version of this trait also required `package(checked: &Self::Checked,
/// out: &mut Vec<u8>)`. `ValueFunctionFamily`'s implementation emitted v2
/// bytes into `out`, but `CheckedPackage::emit_function_package_v2` (the
/// one real caller) passed it a scratch `Vec` that it never read back,
/// then built its actual returned bytes independently through
/// `family::emit_v2` -- gut `package`'s body and
/// `emit_function_package_v2`'s output is byte-identical. A hook nothing
/// consumes is the same fabricated-surface shape as the deleted
/// `requirements`, so it is deleted rather than wired up speculatively;
/// `family::emit_v2`/`decode_v2` are the real v2 emitter for this family,
/// called directly, not through this trait. FR-062's packaging-related
/// rows (the `package` mention in AC-1, and AC-9's fault-injection
/// criterion) are recorded unbacked rather than backed by an unconsumed
/// hook; QSL-152 owns both. A family whose packaging genuinely needs a
/// shared, trait-level hook (for example because several families' v2
/// nodes must compose into one all-or-nothing emission a shared caller
/// drives) adds `package` back as part of that work, with a real consumer
/// in the same change.
pub(crate) trait FamilyContract {
    /// This family's parsed semantic form (typed subnodes; ADR-012 §4).
    type Form;
    /// This family's checked payload, carrying identity and provenance.
    type Checked;
    /// This family's typed refusal cause (ADR-012 §5.1 S4), returned through
    /// [`super::outcome::StageFailure::Refused`] -- see that variant's own
    /// doc for why #214 shipped with no way to construct one and QSL-148
    /// (`Value`'s function family) is the first real instance.
    type Cause;
    /// This family's read-only resolved declarations and type environment.
    /// A GAT (`Declarations<'a>`, not a plain associated type): a family
    /// whose declarations genuinely borrow from the caller's own package
    /// state for the one `check` call (`Value`'s does -- scope, signatures
    /// and dispatch tables it does not own) ties that borrow to `check`'s
    /// own `'a`, the same way [`ReferenceEvaluation::Env`] already does for
    /// `evaluate`.
    type Declarations<'a>;

    /// Check `form` against `cx`, returning the checked node, a typed
    /// refusal or a limit outcome (FR-062 "structured outcome", per
    /// [`super::outcome::StageFailure`]'s own doc). `check` reads nothing
    /// outside `cx` and `form`, and is given no way to mutate anything except
    /// `cx`'s meter, diagnostic sink and scope stack (FR-062-AC-3, PR #303
    /// review finding N3) -- `Self::Declarations` is read-only through
    /// `cx.declarations()`, with no interior-mutability side door for a
    /// family to reach past that.
    ///
    /// `form` is a reference (PR #262 review, finding F9): `check` and
    /// everything it calls only ever read `form`, never need to own or move
    /// out of it, and the caller (`PackageDeclarations::check`) needs its
    /// own copy afterward for provenance bookkeeping -- taking `Self::Form`
    /// by value forced that caller to `.clone()` a deep AST purely to
    /// satisfy this signature.
    fn check<'a>(
        form: &Self::Form,
        cx: &mut CheckContext<'a, Self::Declarations<'a>>,
    ) -> CheckOutcome<Self::Checked, Self::Cause>;
}

/// ADR-012 §2's `ReferenceEvaluation`: the `evaluate` hook every family
/// implements except `Relation` (which has no native evaluation: S6a's input
/// type admits no `Relation`, ADR-012 §2 and FR-090-AC-4, so no S6a arm or
/// refusal exists for it).
///
/// `Env` is a GAT (`type Env<'a>`), not a plain associated type: a family's
/// real evaluation environment (for `Value`, the checked package it
/// resolves `checked`'s identity against, the caller's object environment
/// and its own accounting meter) is borrowed for the one call, not owned by
/// the family marker type.
pub(crate) trait ReferenceEvaluation: FamilyContract {
    /// The observed evaluation result (a kernel value, a state observation
    /// or a trace verdict, depending on the family).
    type Observed;
    /// The evaluation environment every family's `evaluate` reads.
    type Env<'a>;
    /// What a runtime caller actually has in hand to look `evaluate` up by
    /// (PR #303 review, finding N3): always a bare identity at call sites
    /// like `CheckedPackage::call`, which only ever stores the minted
    /// identity a checked declaration resolved to, not the full
    /// `Self::Checked` payload `check` produced it alongside. Distinct from
    /// `Self::Checked` on purpose -- QSL-148 makes `Checked` a richer struct
    /// (the minted identity together with the real checked body, so `check`
    /// can return both through its ordinary `Ok` rather than a side
    /// channel); `evaluate` still only ever needs the identity half, so it
    /// keeps its own narrower type instead of forcing every caller to carry
    /// a full checked payload just to look up an evaluation.
    type Key;

    /// Evaluate `checked` under `env` and `meter`. ADR-012 §2 reserves this
    /// hook alone for returning a meter-budget `Incomplete` outcome (`check`
    /// never does, FR-062-AC-5); [`EvaluateFailure::Incomplete`] is that
    /// outcome (QSL-153, once `quire_exact::Meter::charge` was exported).
    /// FR-090-AC-3 (ADR-013 T-4) is the other failure shape this hook
    /// returns: a broken S6a invariant, [`EvaluateFailure::Fault`], never a
    /// `FamilyResult` or a kernel-shaped refusal.
    fn evaluate<'a>(
        checked: &Self::Key,
        env: &mut Self::Env<'a>,
        meter: &mut Meter,
    ) -> Result<Self::Observed, EvaluateFailure>;
}

/// `evaluate`'s full failure shape: the shared kernel meter's own
/// `Incomplete` (QSL-153, ADR-012 §2's "a refusal... or `Incomplete`, a
/// meter-budget outcome", FR-062-AC-5), or an S6a invariant break
/// (FR-090-AC-3/AC-10, ADR-013 T-4).
///
/// **No `Refused` variant.** An earlier version also carried
/// `Refused(EvaluateRefusal)`, with two variants -- `UnknownIdentity` (no
/// checked function admitted for the identity `evaluate` was asked to run)
/// and `EnvironmentAlreadyConsumed` (this crate's own code called
/// `evaluate` twice on one `EvaluationEnv`) -- both constructed only from
/// `checked`/`env`, never from a display string. FR-090-AC-3 rules that
/// both conditions are broken S6a invariants, not caller-input refusals:
/// `ValueFunctionFamily::evaluate` (`value::expression::family.rs`)
/// constructs `Fault(InternalFault::new("S6a", ..))` directly for each,
/// with its own stable invariant identifier, so `EvaluateRefusal` had no
/// variant and no constructor left and was deleted along with it.
/// `CheckedPackage::call`'s own `map_evaluate_failure` adapter
/// (`value::expression::mod.rs`) now forwards `Fault` unchanged into
/// `CallFailure::Fault` rather than re-deriving it from a refusal.
///
/// **Real producer, not always reachable through today's one production
/// caller.** `ValueFunctionFamily::evaluate` (`crate::check::family`'s
/// evaluation half, `src/value/expression/family.rs`) charges
/// `ChargePoint::FunctionCall` against its own `meter` parameter on every
/// call -- genuine production code, not a test-only hook. `CheckedPackage::
/// call` (`src/value/expression/mod.rs`), the one real (non-test) caller of
/// `evaluate`, builds that meter with unlimited scalar limits today, so an
/// `Incomplete` can never actually surface through `call()` -- the same
/// real-mechanism-behind-an-unlimited-default shape `nesting_depth` itself
/// had before a caller-configurable knob existed for it. The mechanism is
/// exercised directly, against a deliberately tight meter, by
/// `src/value/expression/family.rs`'s `family_contract_tests` (FR-062-AC-5).
/// `call` also never reaches either `Fault` case -- it always resolves the
/// identity from this same package and builds a fresh `EvaluationEnv` --
/// so both are exercised directly against `evaluate`, bypassing `call`,
/// by `family.rs`'s `evaluate_faults_on_a_second_call_on_the_same_env` and
/// `value::expression::mod.rs`'s `s6a_invariant_breaks_are_internal_faults_
/// not_panics` (FR-090-AC-3, TC-384).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EvaluateFailure {
    /// The shared kernel meter's budget is exhausted (FR-062-AC-5's
    /// `Incomplete` half).
    #[error("evaluate exhausted the kernel meter budget: {0:?}")]
    Incomplete(quire_exact::Incomplete),
    /// FR-090-AC-3/AC-10 (ADR-013 T-4): a broken S6a invariant -- never a
    /// `FamilyResult`/kernel `Refused`. `Value`'s own `evaluate` hook
    /// (`value::expression::family::ValueFunctionFamily::evaluate`) raises
    /// this directly for its own two invariants (an unresolved identity, or
    /// a second call on one `EvaluationEnv`), and forwards it unchanged from
    /// `Machine::run` (`value::expression::evaluate.rs`), which raises it
    /// when a checked program's own `Value::Population` argument cannot be
    /// resolved past the point `CheckedPackage::call`'s own `validate`
    /// already admitted it -- both internal-only conditions, never a
    /// caller-input refusal.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}
