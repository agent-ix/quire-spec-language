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

    pub(crate) fn depth(&self) -> usize {
        self.frames.len()
    }
}

/// Explicit stage-entry limits (FR-062 "Explicit limits bound every stage
/// entry, including recursion"; ADR-013 T-4). `check` is a recursive stage
/// (ADR-011 §2.3) and bounds its own recursion by `nesting_depth`, checked
/// before each recursive step -- never by the native call stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StageLimits {
    pub(crate) input_bytes: u64,
    pub(crate) nesting_depth: u64,
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
}

/// ADR-012 §2's design-level `FamilyContract`, narrowed to the parts #214's
/// one migrated family (`Value`'s function declaration/application) can
/// genuinely exercise: `check` and `package`, each required by the trait's
/// own associated-function signatures, so a family that omits either fails
/// to compile (FR-062-AC-1). `requirements` (the contract's sixth part) and
/// a typed refusal `Cause` are deferred -- see `crate::family`'s module doc
/// for `requirements`, and [`super::outcome::StageFailure`]'s doc for
/// `Cause`. Both are real ADR-012 §2 design parts a future family ticket
/// adds back, validated against a real instance rather than guessed here.
///
/// `package` has no `Result` return: `Value`'s function-declaration
/// packaging (name + identity, as v2 JSON) has no failure mode in this
/// migration's scope, so a refusal type with no real refusal would be the
/// same speculative shape `requirements`/`Cause` were. A future family whose
/// packaging can genuinely fail adds that back too.
pub(crate) trait FamilyContract {
    /// This family's parsed semantic form (typed subnodes; ADR-012 §4).
    type Form;
    /// This family's checked payload, carrying identity and provenance.
    type Checked;
    /// This family's read-only resolved declarations and type environment.
    type Declarations;

    /// Check `form` against `cx`, returning the checked node, a limit
    /// outcome or an internal fault (FR-062 "structured outcome", narrowed
    /// per this trait's own doc). `check` reads nothing outside `cx` and
    /// mutates nothing but `cx`'s meter, diagnostic sink and scope stack.
    fn check(form: Self::Form, cx: &mut CheckContext<'_, Self::Declarations>) -> CheckOutcome<Self::Checked>;

    /// Emit every v2 node `checked` requires (FR-062 "Packaging is
    /// all-or-nothing"; this trait's doc explains why there is no refusal
    /// return here). `out` accumulates emitted bytes.
    fn package(checked: &Self::Checked, out: &mut Vec<u8>);
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
    /// and `package` never do, FR-062-AC-5); `EvaluateRefusal`'s own doc
    /// explains why #214 does not add that variant yet.
    fn evaluate<'a>(
        checked: &Self::Checked,
        env: &mut Self::Env<'a>,
        meter: &mut Meter,
    ) -> Result<Self::Observed, EvaluateRefusal>;
}

/// `evaluate`'s own refusal shape: a typed refusal built only from checked
/// input -- never a CST, a token or a display string (FR-062-AC-6).
///
/// **No `Incomplete` variant.** ADR-012 §2 reserves `evaluate` as the one
/// hook allowed to return the kernel meter's `Incomplete` outcome, and that
/// part of the design is real -- but `quire_exact::Meter::charge`/
/// `charge_plan`, the only way to actually produce an `Incomplete`, are
/// `pub(crate)` inside `quire-exact` (`quire-exact/src/accounting.rs:551,
/// 595`), not exported to this crate. QSL cannot charge the kernel meter
/// through any public API today, so nothing here could construct an
/// `Incomplete` for real; this is an export gap in `quire-exact` (#213 S-1),
/// tracked separately, not a consequence of how many families are migrated.
/// The first caller with public charge access adds this variant back.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum EvaluateRefusal {
    #[error("evaluation refused: {0}")]
    Refused(String),
}
