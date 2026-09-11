---
id: SR-253
title: "Code and Rust review of validated state-scalar projection"
type: SpecReview
analysis: code-review
scope: "src/lowering.rs; src/lowering/inputs.rs; src/lowering/target.rs; tests/state_scalar_lowering.rs"
review_set: subset
---
## Summary

Current recheck: correction `09c50a5` against integrated baseline `87b35ea`,
narrowed to the four external findings on the earlier revision. This is a
finding recheck, not a fresh whole-repository review; the earlier PASS for
revision `b789eed` is superseded as a current verdict and retained below only as
historical context. Applied the actual agent-skills code-review Rust lane,
rust-review and rust-style with repository conventions. No AssuranceProfile
document exists in the repository, and no deny.toml.

**Claude finding-recheck verdict: the four external findings are resolved in code.**
Subsequent local integration verification passes the full feature suites and
the remaining checks recorded below. The residual observations are nonblocking lows.

Finding 1 (typed input correspondence). `InputProjectionCode::InvalidCorrespondence`
is replaced by `Invariant(InputProjectionInvariant)` with eleven typed reasons;
`InputProjectionError::message` is gone, so no classification can depend on
Display text. `MissingArenaValue`, `NonPrimitiveValue` and `ArenaIndexWidth`
are distinct variants, each carrying the original `ValueId`. Defensive failures
retain the exact original `ProjectedRead` (`Option<Box<ProjectedRead>>`),
including linked declaration, model digest and both observations; the three
caller stops carry `read: None`. Atomic no-partial-input semantics and completed
`work` survive on every path.

Reachability was established against the real API rather than assumed.
`ValidatedContext`'s fields are all `pub(super)`/`pub(in crate::runtime)` and
its sole construction site is `runtime::validation::validate_retaining`, which
returns a context only when `budget.failed()` is false; every `Budget::observe`
sets `invalid` or `incomplete`, so no diagnostic can be emitted and still yield
a context. `inventory::select` is exhaustive over (selection, clause kind) and
issues `WrongSnapshot` for `Current` against a pre/post clause and for
`Invocation` against an invariant; `linking::native::check_input_scope` refuses
an invocation input outside its operation's parameters with `MissingDeclaration`.
The external review's Current-context/captured-`Input` scenario therefore does
not survive public validation, and no other invalid correspondence was found to
be reachable after a successful `inputs()` prerequisite. The remaining branches
are correctly documented as internal invariants; tests exercise their public
rejection prerequisites. No caller-repair category was invented, and no test
fabricates a context to execute a defensive branch.

Finding 2 (integer capability). `ProjectionTarget::admits_integers` is an
explicit, positive, exhaustive `match` with no wildcard, and `admits` is now
expressed through it, so target policy and type admission share one fact. The
two `!= BooleanOracleV1` guards and the `binary` guard call it. Because
`ProjectionTarget` is `#[non_exhaustive]` only for downstream crates, a new
in-crate variant fails to compile until it states a decision. All three targets'
Boolean/integer behavior and primitive values are pinned by the new
`every_projection_target_keeps_its_boolean_and_signed_integer_capability`.

Finding 3 (alias-search work). `charge_node` runs before each candidate ordinal
is taken and formatted, so real collisions are charged; the reused-alias path
(`field_aliases` hit) charges nothing. Charging stays inside the existing 10,000
node ceiling with a stated `ResourceExhausted` at the ceiling, and does not touch
`max_depth`, so actual AST depth is preserved. The new
`real_model_alias_collisions_consume_bounded_work_before_candidate_creation`
declares sixteen real `nativeField*` model values and pins the exact ceiling
(5 nodes + 17 candidates), exhaustion during candidate search at the `self.n`
span, the one-short whole-package case failing in `other_rule` at `true`, and a
fresh retry producing identical bytes. The existing atomicity test now pins
alias reuse (`expressions().len() + 1`). No limit was raised to fit a test and
no new budget framework was introduced.

Finding 4 (public error evolution). `InputProjectionError` is `#[non_exhaustive]`;
`InputProjectionCode` and the new `InputProjectionInvariant` are `#[non_exhaustive]`
and `thiserror`-derived with typed payloads, not strings. No synthetic error owner
was added.

One intentional public behavior change is documented rather than silent: a caller
that previously sized `LoweringLimits::nodes` exactly from the AST now needs
alias headroom. The `LoweringLimits::nodes`/`LoweringUsage::nodes` doc comments
and a new FR-034 behavior line state it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No blocking scoped code/Rust finding remains. Full backend/activation qualification is separate. | FR-034; TC-112; Task-020 |
| FND-002 | low | The `Cancelled` stop asserts `work > 0` but not `read.is_none()`, unlike the `ContextMismatch` and `ResourceExhausted` stops. Behavior is correct (`Budget::error` sets `read: None`); the pin is missing. | TC-112; tests/state_scalar_lowering.rs |
| FND-003 | low | Pre-existing, outside the four findings: sibling `LoweringError`/`LoweringCode` keep all-public fields without `#[non_exhaustive]`, and `LoweringCode::InvalidCorrespondence` still carries its discriminant in `message: String` across the `field()` refusals — the pattern finding 1 fixed at the input boundary. Not a regression from this correction. | src/lowering.rs |

## Validation

Current correction evidence (root's runs, not this reviewer's):

- Focused suite log `/tmp/quire-backlog-26-focused.log`: 26 tests passed, 0
  failed, across integer_lowering (7), lower_command (6), native_lowering (5)
  and state_scalar_lowering (8), including the three new traced tests. This is a
  focused subset, not a feature or qualification suite.
- Subsequent local author verification of the integrated five-PR stack plus
  merged PR58: 576 minimal-feature tests and 592 all-feature tests pass, with
  zero failures and four inherited assurance ignores per lane, including five
  doctests. Both strict all-targets Clippy configurations, fmt, minimal
  binary/example build, warnings-denied rustdoc, both Rust fixture audits and
  scoped Quire validation (432/432) pass. Logs: `/tmp/quire-backlog-final-*.log`.
  The initial integration run exposed two newer composed tests expecting a
  10,001-element model to reach type checking. PR31's integration correction
  now asserts the earlier typed native-admission refusal and preserves the real
  10,000-element success paths; both affected suites pass all 27 tests. This
  integration disposition is the author's verification after Claude's recheck,
  not an additional independent review or a change to the production ceiling.

Historical, superseded: at `b789eed` the full all-feature suite passed 347
ordinary tests plus three compile-fail doctests and minimal features passed 331
plus three, with four assurance tests ignored in each. Those counts describe the
pre-correction revision and are not carried forward as this correction's result.

This recheck marks neither the PR, the parent plan, the numeric backend nor
activation qualification complete. No hosted workflow ran; `workflow_dispatch`
remains the only hosted trigger.
