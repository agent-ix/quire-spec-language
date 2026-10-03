// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-062-AC-1, ADR-012 §2: the static `FamilyContract` trait and the
//! mutable typing context (`CheckContext`) every family's `check` receives.
//! Its S6a subtrait, `ReferenceEvaluation`, is layer 5's
//! (`value::expression::s6a`, ADR-011 §6.2 `family` row).

use super::outcome::CheckOutcome;
use qsl_foundation::diagnostic::{LimitExceeded, LimitKind};
use quire_exact::Meter;

/// A diagnostic a family's `check` records against the scope it was raised
/// in. The shared layer defines no family-specific diagnostic content; this
/// is the sink's own record shape (a message plus the scope name active when
/// it was raised), independent of any one family's `Cause` enum.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub(crate) scope: String,
    pub(crate) message: String,
}

/// The only mutable diagnostic sink `check` may write through
/// (FR-062-AC-3): an ordinary append-only log, read back for assertions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticSink {
    entries: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub(crate) fn record(&mut self, scope: &ScopeStack, message: impl Into<String>) {
        self.entries.push(Diagnostic {
            scope: scope.path(),
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
    /// not exist. `test-support` makes it reachable from the layer-5
    /// evaluator's tests across the `qsl-semantics` crate boundary.
    #[cfg(any(test, feature = "test-support"))]
    pub fn entries(&self) -> &[Diagnostic] {
        &self.entries
    }
}

/// The only mutable scope stack `check` may push/pop through
/// (FR-062-AC-3). Named scopes only -- no family-specific payload.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ScopeStack {
    frames: Vec<String>,
}

impl ScopeStack {
    pub(crate) fn enter(&mut self, name: impl Into<String>) {
        self.frames.push(name.into());
    }

    pub(crate) fn leave(&mut self) {
        self.frames.pop();
    }

    /// Every frame from the outermost, joined with `/` (`<root>` when
    /// empty), so a diagnostic names the whole stack it was raised under.
    pub(crate) fn path(&self) -> String {
        if self.frames.is_empty() {
            "<root>".to_owned()
        } else {
            self.frames.join("/")
        }
    }
}

/// Explicit stage-entry limits (FR-062 "Explicit resource limits bound
/// every stage entry, at any depth"; ADR-013 T-4): one declaration's input
/// bytes and node count. No limit bounds nesting depth (ADR-030 D-1,
/// FR-258): every walk `check` makes runs over an explicit heap stack whose
/// growth these limits and the contract meter charge.
///
/// - **Producer**: `crate::check::family::measure_declaration`'s preimage
///   pass builds a length-prefixed byte buffer over the declaration's
///   structure and walks every [`qsl_forms::Expression`] node in it to do
///   so. `input_bytes` is that buffer's own logical byte length
///   (accumulated as the pass writes; see `DeclarationMeter`'s own doc);
///   `node_count` is the number of `Expression` nodes the same pass visits.
/// - **Consumer**: `CheckContext::check_input_bytes` and
///   `CheckContext::check_node_count` each compare their metric against
///   this struct's matching field and return `LimitKind`'s matching variant
///   on the first one exceeded. `ValueFunctionFamily::check`
///   (`crate::check::family`) calls both before minting succeeds.
///
/// **The work budget is not a field here.** `LimitKind::WorkBudget` is
/// produced through `CheckContext::meter`, the shared kernel budget every
/// family's `check` receives (FR-062 "checked input"):
/// `ValueFunctionFamily::check` charges it one `ChargePoint::
/// DeclarationCheck`, sized by the same preimage pass's field-write count,
/// and maps a denied charge to `Limit{WorkBudget}` naming the meter's own
/// configured `work_units` bound. `input_bytes`/`node_count` bound one
/// declaration's own shape; the work budget bounds the checking stage's
/// total spend.
///
/// `input_bytes`/`node_count`'s one production call site
/// (`crate::check::mod::PackageDeclarations::check`) reads both from the
/// caller's `CheckingLimits`, whose defaults are NFR-011's finite ceilings
/// (`CheckingLimits::with_input_bytes` and `CheckingLimits::new` set them);
/// the mechanism is exercised directly against tight fixtures
/// (`qsl-eval/src/value/expression/family.rs`'s `family_contract_tests`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageLimits {
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
pub struct CheckContext<'a, D> {
    declarations: &'a D,
    limits: StageLimits,
    pub(crate) meter: &'a mut Meter,
    pub(crate) diagnostics: &'a mut DiagnosticSink,
    pub(crate) scopes: &'a mut ScopeStack,
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

    /// Refuse `amount` (a declaration's own preimage byte length,
    /// `StageLimits`'s own doc) once it exceeds `limits.input_bytes`
    /// (restoring the deleted `input_bytes` field with a real
    /// consumer).
    pub(crate) fn check_input_bytes(&self, amount: u64) -> Result<(), LimitExceeded> {
        if amount > self.limits.input_bytes {
            return Err(LimitExceeded::new(
                LimitKind::InputBytes,
                self.limits.input_bytes,
                u128::from(amount),
            ));
        }
        Ok(())
    }

    /// Refuse `amount` (a declaration's own visited `Expression` node
    /// count) once it exceeds `limits.node_count`.
    pub(crate) fn check_node_count(&self, amount: u64) -> Result<(), LimitExceeded> {
        if amount > self.limits.node_count {
            return Err(LimitExceeded::new(
                LimitKind::NodeCount,
                self.limits.node_count,
                u128::from(amount),
            ));
        }
        Ok(())
    }
}

/// ADR-012 §2's design-level `FamilyContract`: `check` and `requirements`,
/// each required by the trait's own associated-function signature, so a
/// family that omits either fails to compile. `requirements` (the
/// contract's sixth part, FR-062-AC-4) moved with ADR-014 §11;
/// see [`crate::family::requirements`]'s module doc.
///
/// **No `package` part (ADR-012 §2 "Packaging").** A family's
/// packaging is its `check` lowering to the checked semantic graph
/// (FR-093); `qsl_package::emit_checked` writes every family's nodes, one
/// arm per node tag, and is all-or-nothing over the nodes a node names
/// (FR-062-AC-9). History: an earlier
/// version of this trait also required `package(checked: &Self::Checked,
/// out: &mut Vec<u8>)`. `ValueFunctionFamily`'s implementation emitted v2
/// bytes into `out`, but `CheckedPackage::emit_function_package_v2` (the
/// one real caller, deleted itself along with the second
/// `quire.checked-function-package/v2` producer it and `family::emit_v2`/
/// `decode_v2` made up) passed it a scratch `Vec` that it never read back,
/// then built its actual returned bytes independently through
/// `family::emit_v2` -- gut `package`'s body and
/// `emit_function_package_v2`'s output was byte-identical, so PR #262
/// review deleted it as a hook nothing consumed (findings F1/F2).
///
/// **FR-062-AC-1 (TC-160 step 1).** A family implements the whole set once;
/// the control below, with `check` taking the checked-input parameter and
/// `requirements`, compiles.
///
/// ```
/// use qsl_foundation::diagnostic::Staged;
/// use qsl_semantics::family::{CheckContext, CheckOutcome, FamilyContract};
///
/// struct Complete;
/// impl FamilyContract for Complete {
///     type Form = ();
///     type Checked = ();
///     type Cause = ();
///     type Declarations<'a> = ();
///     type Claim = ();
///     fn check<'a>(
///         _form: &(),
///         _cx: &mut CheckContext<'a, ()>,
///     ) -> CheckOutcome<(), ()> {
///         Ok(Staged::new(()))
///     }
///     fn requirements(_checked: &()) -> Vec<()> {
///         Vec::new()
///     }
/// }
/// ```
///
/// Omitting the `requirements` method does not compile:
///
/// ```compile_fail,E0046
/// use qsl_foundation::diagnostic::Staged;
/// use qsl_semantics::family::{CheckContext, CheckOutcome, FamilyContract};
///
/// struct NoRequirements;
/// impl FamilyContract for NoRequirements {
///     type Form = ();
///     type Checked = ();
///     type Cause = ();
///     type Declarations<'a> = ();
///     type Claim = ();
///     fn check<'a>(
///         _form: &(),
///         _cx: &mut CheckContext<'a, ()>,
///     ) -> CheckOutcome<(), ()> {
///         Ok(Staged::new(()))
///     }
/// }
/// ```
///
/// Nor does a `check` that omits the checked-input parameter (the typing
/// context):
///
/// ```compile_fail,E0050
/// use qsl_foundation::diagnostic::Staged;
/// use qsl_semantics::family::{CheckOutcome, FamilyContract};
///
/// struct NoTypingContext;
/// impl FamilyContract for NoTypingContext {
///     type Form = ();
///     type Checked = ();
///     type Cause = ();
///     type Declarations<'a> = ();
///     type Claim = ();
///     fn check<'a>(_form: &()) -> CheckOutcome<(), ()> {
///         Ok(Staged::new(()))
///     }
///     fn requirements(_checked: &()) -> Vec<()> {
///         Vec::new()
///     }
/// }
/// ```
pub trait FamilyContract {
    /// This family's parsed semantic form (typed subnodes; ADR-012 §4).
    type Form;
    /// This family's checked payload, carrying identity and provenance.
    type Checked;
    /// This family's typed refusal cause (ADR-012 §5.1 S4), returned through
    /// [`qsl_foundation::diagnostic::StageFailure::Refused`] -- see that variant's own
    /// doc for why #214 shipped with no way to construct one and
    /// `Value`'s function family is the first real instance.
    type Cause;
    /// This family's read-only resolved declarations and type environment.
    /// A GAT (`Declarations<'a>`, not a plain associated type): a family
    /// whose declarations genuinely borrow from the caller's own package
    /// state for the one `check` call (`Value`'s does -- scope, signatures
    /// and dispatch tables it does not own) ties that borrow to `check`'s
    /// own `'a`, the same way `value::expression`'s `ReferenceEvaluation::Env` does for
    /// `evaluate`.
    type Declarations<'a>;

    /// Check `form` against `cx`, returning the checked node, a typed
    /// refusal or a limit outcome (FR-062 "structured outcome", per
    /// [`qsl_foundation::diagnostic::StageFailure`]'s own doc). `check` reads nothing
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

    /// One claim of a checked item: its capability kind and classified
    /// extent, paired with the checked site it covers (ADR-012 §2). A
    /// family whose sites carry their own occurrence data names it here;
    /// `check` keys each claim by the occurrence recorded at its site.
    type Claim;

    /// The claims of one checked item (ADR-012 §2, FR-062-AC-4): one per
    /// claim site whose form has an FR-057 capability kind, and none for a
    /// form that has no kind. Pure and total: it reads only `checked`,
    /// which carries the extents `check` classified, and two calls on the
    /// same node return equal values.
    fn requirements(checked: &Self::Checked) -> Vec<Self::Claim>;
}
