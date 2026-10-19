---
id: SR-12193
title: "Code review of quire-spec-language PR #1034: FR-330 run_clause Temporal selection, FR-332 result-label map"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@39ed010e651f0d8cb7df9b60b9c77c943938c9d8; examples/config-version/spine.rs, qsl-eval/src/temporal/mod.rs, qsl-eval/src/temporal/observed.rs, qsl-eval/src/value/expression/mod.rs, qsl-replay/src/execute/frame.rs, qsl-replay/src/execute/state_clause.rs, qsl-replay/src/model_check/settle.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/temporal.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/receiver_frames.rs, qsl-replay/src/spine/clause/tests/temporal.rs, qsl-replay/src/spine/native_result.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/temporal_clause/condition.rs, tests/it/main.rs, tests/it/temporal_negotiation.rs, xtask/src/refinement/compare.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1034

## Summary

Ticket: QSL-510 (partial: T4 `run_clause` Temporal, FR-330 AC-1 to AC-4 and
AC-6, FR-332). Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (20 files, head `39ed010e6`). Static review
only: no cargo build, test or `make ci` was run, per the dispatch brief.

Examined:
- `ClauseRunSelection::Temporal`, `TraceInput::{Finite, Lasso}`,
  `OverObject`, `run_temporal` (name resolution, the bounded-profile/lasso
  check, empty-trace short cut, per-position FR-106 admission, `over`
  resolution at position 0, the one `evaluate_observed` call).
- `ClauseDisposition::{Temporal, TemporalRefusal}` and their `stage`,
  `category` and `truth` arms.
- The evaluator seam: `qsl_eval::temporal::evaluate_observed`,
  `ObservedLetters::{environment, atom, activation}`,
  `CheckedPackageEvaluation::evaluate_holds`, and the
  `TemporalClauseNode.conditions` map with `TypedCondition.slots`.
- `ResultLabel::Unsupported` and `ResultLabel::category(Execution)`.
- The exhaustiveness arms in `execute/frame.rs`, `execute/state_clause.rs`,
  `spine/refinement.rs`, `spine/native_result.rs`,
  `xtask/src/refinement/compare.rs` and `examples/config-version/spine.rs`,
  and the `pub(super)` widening in `tests/receiver_frames.rs`.
- Context only: FR-330, FR-332, FR-327 to FR-329, FR-109's
  `convert_outcome` and its `CallFailure` mapping in `check_clause`,
  `TemporalOutcome::category`, `TemporalRefusal::catalog_code`,
  `Category::exit_code`/`trace_exit_code`, `ClauseRoot::resolve`
  (model_check/identity.rs).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ClauseDisposition::category` maps every `TemporalRefusal` to `Category::Refusal` (exit 20), ignoring the refusal's own catalog code. `TemporalRefusal::Fault` (`runtime_invariant`, a broken invariant that FR-109 reports as `internal-failure`, exit 30) and `TemporalRefusal::ProfileNotEvaluated` (`unsupported_construct`/`expression-form`, reached by any clause under the real `TimestampedEventFiniteWindow` profile, exit 21) both report `refusal`. Every `LetterStop::Fault` that `ObservedLetters` raises (`position-has-an-environment`, `holds-condition-boolean`, `holds-condition-refused`, `emitted-clause-has-no-activation-condition`) arrives this way. Fix: route `TemporalRefusal::Fault` to `ClauseDisposition::EvaluateFault` in `run_temporal`, and take the category of the rest from `catalog_code()`. | qsl-replay/src/spine/clause.rs:350, qsl-replay/src/spine/clause/temporal.rs:201-203 |
| FND-002 | medium | A `holds` condition whose S6a result is `Outcome::Refused(_)` (a domain refusal such as `IntegerOutOfDomain`, e.g. `holds(c.value + 1 = 2)` at the integer bound) or `FamilyResult::Refused(_)` becomes the fault `holds-condition-refused`. FR-109's state-clause path reports the same S6a result as a `refusal` with its record (`convert_outcome`), keeping only `CheckedInvariant` as a fault. With FND-001, the temporal run then reports `refusal` with `runtime_invariant`/`established-invariant-broken` instead of the condition's own code. The `CallFailure` mapping also uses a `_ =>` arm, so `Storage`, `Supplied` and `UnsupportedHelper` (which `check_clause` reports as an argument refusal) become the fault `holds-condition-evaluates`, against the repo's no-wildcard rule (FR-090, `#[deny(clippy::wildcard_enum_match_arm)]` in `convert_outcome`). The `AtomReading` seam has no refusal arm, so a refused letter needs a `LetterStop` variant (or a typed `TemporalRefusal`) that carries the record. | qsl-eval/src/temporal/observed.rs:105-110, qsl-eval/src/temporal/observed.rs:129-135 |
| FND-003 | low | Once admission has run, the `over`-type fault, the `population_universe_for` failure and the `wrong-role-mapping` refusal return through `early`, which reports `ClauseRunUsage::default()`. The admission work already done (`admitted`) is dropped from the report, while an admission failure on the same path keeps it. Use `report(.., ClauseRunUsage { admission_consumed: admitted, .. })` for these three returns. | qsl-replay/src/spine/clause/temporal.rs:164-190 |
| FND-004 | low | `examples/config-version/spine.rs::write` now writes a `clause-run/1` selection `{"kind": "temporal", "name": ..}`, with no `trace` or `over`. FR-312's request wire has no temporal selection (declared as unspecified), so this invents a partial wire form instead of refusing. No case builds a Temporal selection today, so nothing writes it yet. Return an `io::Error` (`InvalidInput`) for this arm until the wire form is specified. | examples/config-version/spine.rs:342-345 |
| FND-005 | low | The two unreachable arms disagree on what a temporal selection is in a refinement check. `spine/refinement.rs` says a refinement check selects no temporal clause, so the disposition is a tool failure. `xtask/src/refinement/compare.rs` classifies it as `SelectionKind::Clause`, whose comment says its absence in the superseding revision drops a constraint (`Holds`). Neither is reachable, because no refinement case builds a Temporal selection. Keep one rule: either both treat it as a broken invariant, or the xtask comment states that the kind is never read for a temporal selection. | qsl-replay/src/spine/refinement.rs:516-520, xtask/src/refinement/compare.rs:24-26 |

## Verdict

MERGEABLE on code-review grounds. No high findings. FND-001 and FND-002 are
medium: both give a wrong category or record only on non-main paths (faults,
a timestamped profile, a condition that refuses). Fix them in this PR if the
fix round is cheap. Otherwise ticket them. The lows are recorded.

What holds:
- Package identity and emission are unchanged. `TemporalClauseNode.conditions`
  keeps the checked bodies that `check_conditions` already produced and that
  lowering already consumed. `lowered.key`, the `claim` occurrence and the
  requirement records are computed exactly as before. Emission does not
  serialize `TemporalClauseNode`. `ClauseRoot::resolve` compares
  `node.clause()` (the `CheckedTemporalClause`), not the node, so dropping
  `Eq` and adding the map to `PartialEq` does not change any lookup.
- `evaluate_holds` runs the stored condition on the one S6a `Machine`, with
  the `over` reference at slot 0 and the typer's own slot count. Every read
  goes to the position's admitted `ObjectEnvironment`. A node that is not a
  `holds` atom of the clause is a fault, not a guess.
- `ObservedLetters::environment` wraps a lasso index with `checked_rem` and
  `checked_add`, and a missing slot is a fault. There are no `as` casts or
  panics on the production path.
- `run_temporal` follows FR-330's order. It resolves the name (stage
  `select`), refuses a bounded profile over a lasso (stage `admit`,
  `invalid-value`) and admits every position in order, reporting the first
  failure. It resolves `over` in position 0 (`wrong-role-mapping`) and
  evaluates once. Every report carries `basis: Unavailable` and no witness,
  which is FR-332's rule that evidence from a trace settles no item.
- `truth()` is `Some` only for `Holds`/`Violated`, so `Pending`, `Undefined`,
  `Incomplete` and `MissingFairnessPremise` carry no truth, as FR-109 does.
- `ResultLabel::category` matches the ADR-014 A-5 map for every label:
  `proved` success, `refuted` violation, `inconclusive` inconclusive,
  `unsupported` unsupported, `failed`+`resource-incomplete` incomplete, and
  `failed`+`completed`/`failed` internal failure. The match is exhaustive over
  `(label, execution)` with no wildcard. The new `Unsupported` label is never
  constructed by `settle_model_check`, and no existing match over
  `ResultLabel` takes a wildcard, so no current record changes.
- The exhaustiveness arms in `execute/frame.rs` and `execute/state_clause.rs`
  join the existing "selection kind that cannot reach here" fault arms. No
  behaviour changes, because those replays never select a temporal clause.
  The `native_result.rs` no-op arm sits behind `write_clause_run_result`'s
  up-front `TemporalDisposition` refusal, and `Disposition::of` is private
  with that one caller, so no temporal record is half-written.
- The `pub(super)` widening in `tests/receiver_frames.rs` is test-only and
  stays inside the `clause::tests` module.
- Rust lane: no `unsafe`, and the new public enums carry doc comments. The
  `seam_probe_eval_downstream` arm follows the crate's existing pattern.

## Dispositions

Round 1, reviewed at `adbda88e4b1c444eec5c5389fef9062ab7f781f6` (the fix
commit `adbda88e4` on top of a merge of main). Static review only.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | adbda88e4: `TemporalRefusal`'s category now comes from `Code::from_code(refusal.catalog_code().code())`, so `ProfileNotEvaluated` is unsupported. `run_temporal` sends `TemporalRefusal::Fault` to `EvaluateFault` (internal failure). The new untagged test `a_temporal_refusal_takes_its_category_from_its_catalog_code` checks refusal, unsupported and internal failure. |
| FND-002 | fixed | adbda88e4: a refused condition is kept as `HoldsStop::Refused(evaluation)`, and `run_temporal` reports it through `convert_outcome`, exactly as FR-109 does (refusal with its record; `CheckedInvariant` stays a fault). A failed call is kept as `HoldsStop::Failed` and mapped by `convert_call_failure`. The `_ =>` arm is gone. No test drives a refusing condition, and FR-330 states no AC for this path. The fix reuses FR-109's tested mapping, so this is accepted without a new test. |
| FND-003 | fixed | adbda88e4: the three post-admission returns go through `late`, which keeps `admission_consumed: admitted`. |
| FND-004 | fixed | adbda88e4: the Temporal arm returns `io::Error(InvalidInput, "a temporal selection has no clause-run/1 wire form")`. |
| FND-005 | fixed | adbda88e4: `SelectionKind::Temporal` is its own kind. `(Admitted, Absent)` gives `ToolFailure` for it, which agrees with `classify_disposition`'s tool-failure arm. The compare table test's `KINDS` still lists three kinds, so the new row is not exercised. The arm cannot be reached, so this is not a new finding. |
