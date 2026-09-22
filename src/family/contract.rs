// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-062-AC-1, ADR-012 §2: the static `FamilyContract`/`ReferenceEvaluation`
//! traits and the mutable typing context (`CheckContext`) every family's
//! `check` receives.

use super::outcome::CheckOutcome;
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
    /// This family's read-only resolved declarations and type environment.
    type Declarations;

    /// Check `form` against `cx`, returning the checked node or a limit
    /// outcome (FR-062 "structured outcome", narrowed per this trait's own
    /// doc and [`super::outcome::StageFailure`]'s). `check` reads nothing
    /// outside `cx` and `form`, and mutates nothing but `cx`'s meter,
    /// diagnostic sink and scope stack.
    ///
    /// `form` is a reference (PR #262 review, finding F9): `check` and
    /// everything it calls only ever read `form`, never need to own or move
    /// out of it, and the caller (`PackageDeclarations::check`) needs its
    /// own copy afterward for the unchanged `Typer`-based typing pass --
    /// taking `Self::Form` by value forced that caller to `.clone()` a deep
    /// AST purely to satisfy this signature.
    fn check(
        form: &Self::Form,
        cx: &mut CheckContext<'_, Self::Declarations>,
    ) -> CheckOutcome<Self::Checked>;
}

/// ADR-012 §2's `ReferenceEvaluation`: the `evaluate` hook every family
/// implements except `Relation` (which has no native evaluation -- ADR-012
/// §2's `FamilyNotNativelyEvaluable` arm covers that case at the S6a seam,
/// not through this trait).
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

    /// Evaluate `checked` under `env` and `meter`. ADR-012 §2 reserves this
    /// hook alone for returning a meter-budget `Incomplete` outcome (`check`
    /// never does, FR-062-AC-5); [`EvaluateFailure::Incomplete`] is that
    /// outcome (QSL-153, once `quire_exact::Meter::charge` was exported --
    /// `EvaluateRefusal`'s own doc explains why #214 could not add it).
    fn evaluate<'a>(
        checked: &Self::Checked,
        env: &mut Self::Env<'a>,
        meter: &mut Meter,
    ) -> Result<Self::Observed, EvaluateFailure>;
}

/// `evaluate`'s own refusal shape: constructed only from checked input --
/// never by reading a CST, a token or a display string (FR-062-AC-6 is
/// about what `evaluate` reads, not what a refusal's own message renders
/// as; each variant's payload is a rendered failure message, not something
/// read back in as input).
///
/// **Two distinct variants, not one `Refused(String)` (PR #262 review,
/// finding F3, this round).** An earlier version had exactly one
/// `Refused(String)` variant standing for two different conditions --
/// `UnknownIdentity` below (a public "this identity is not in this
/// package" input error) and `EnvironmentAlreadyConsumed` (a purely
/// internal invariant: this crate's own code called `evaluate` twice on one
/// `EvaluationEnv`). The one caller mapping this into a public
/// `InputRefusal` (`value::expression::mod.rs`'s `CheckedPackage::call`)
/// matched `Refused` once and turned *both* into `InputRefusal::
/// UnknownFunction`, so the internal-invariant message ("evaluate called
/// more than once...") could have surfaced to a caller asking a perfectly
/// ordinary "does this function exist" question. The variant set is the
/// API; splitting it lets that one call site treat the two conditions
/// differently, which it now does.
///
/// **`UnknownIdentity`'s one call site is not reachable today (PR #262
/// review, an earlier round's finding F8, carried over unchanged by the
/// split above).** `ValueFunctionFamily::evaluate` constructs it only when
/// `env.package.function_by_identity(*checked)` finds nothing, but
/// `CheckedPackage::call` -- `EvaluationEnv`'s one real (non-test)
/// constructor -- always passes an identity it just read out of the same
/// `self.functions` list `function_by_identity` searches, so that lookup
/// cannot fail through `call()`. Unlike the deleted `StageFailure::Fault`
/// self-comparison, this is not a tautology (`function_by_identity`
/// genuinely does an `Option`-returning search, not a compile-time-known
/// constant), and removing the `Result` would force `evaluate` to panic on
/// a lookup miss instead -- worse than an unreached refusal path, not
/// better, given #262's own ruling that a structured outcome undone by a
/// panic at its one call site is the wrong trade. `UnknownIdentity`
/// therefore stays, kept honest about being unreached by any caller today
/// rather than presented as exercised: #243 (the layer-6 `replay` facade
/// widening) resolves `checked` from a bare identity with no prior name
/// lookup, and is the named, tracked owner of the change that would
/// genuinely trigger this path -- not a guess at whoever comes first.
///
/// **`EnvironmentAlreadyConsumed` is also unreached through `call()`**, for
/// the opposite reason: `call()` always builds a *fresh* `EvaluationEnv`
/// with `Some(arguments)` and calls `evaluate` exactly once, so its own
/// construction rules this condition out. It stays a typed `Result` variant
/// rather than an `unwrap`/`expect` inside `evaluate` itself, because
/// `evaluate` is reachable from test code that deliberately reuses one
/// `EvaluationEnv` across two calls (`value::expression::family`'s
/// `evaluate_refuses_a_second_call_on_the_same_env`) -- panicking there
/// would just move the "surfaces as a typed refusal, not a wrong silent
/// answer" property this variant exists for out of `evaluate` and into a
/// panic the test would have to catch instead.
///
/// **No `Incomplete` variant here (QSL-153).** ADR-012 §2 reserves
/// `evaluate` as the one hook allowed to return the kernel meter's
/// `Incomplete` outcome; `EvaluateFailure` (below), not this type, carries
/// it -- `EvaluateRefusal` stays exactly the two checked-input-only
/// refusals it always was, both constructed from `checked`/`env` alone,
/// never from a meter's charge result. `quire_exact::Meter::charge`/
/// `charge_plan` are `pub` (QSL-166), which is what makes
/// `EvaluateFailure::Incomplete` constructible for real.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EvaluateRefusal {
    /// No checked function is admitted for `identity`.
    #[error("no checked function for identity {identity}")]
    UnknownIdentity {
        /// The identity `evaluate` could not resolve.
        identity: quire_exact::NodeKey,
    },
    /// `evaluate` was called a second time on an `EvaluationEnv` whose
    /// `arguments` an earlier call already consumed via `Option::take`.
    #[error(
        "evaluate called more than once on the same EvaluationEnv \
         (arguments already consumed by an earlier call)"
    )]
    EnvironmentAlreadyConsumed,
}

/// `evaluate`'s full failure shape (QSL-153): a typed refusal built only
/// from checked input, or the shared kernel meter's own `Incomplete` --
/// distinct types, per ADR-012 §2's own split ("a refusal... or `Incomplete`,
/// a meter-budget outcome", FR-062-AC-5), not two constructors folded into
/// one `Refused`/`String` shape.
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
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EvaluateFailure {
    /// A typed refusal built only from checked input.
    #[error(transparent)]
    Refused(#[from] EvaluateRefusal),
    /// The shared kernel meter's budget is exhausted (FR-062-AC-5's
    /// `Incomplete` half).
    #[error("evaluate exhausted the kernel meter budget: {0:?}")]
    Incomplete(quire_exact::Incomplete),
}
