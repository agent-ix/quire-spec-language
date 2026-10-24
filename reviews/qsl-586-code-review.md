---
id: SR-12191
title: "Code review of quire-spec-language PR #1031: hybrid solver result to verdict (FR-250-AC-2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5395e3fdda8a5e2bed7dfd11421e9114146316df; PR #1031 diff against origin/main: qsl-replay/src/{hybrid,lib,outcome,proof_result}.rs; qsl-replay/src/model_check/limits.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1031

## Summary

Ticket: QSL-586 (partial: FR-250-AC-2 only). One code-review pass with the
rust-review lane folded in, over `git diff origin/main...HEAD` (5 files,
+392/-2). Read-only: nothing was built or run, at the dispatcher's
instruction. QSpec read at its current main, cited by id.

Checked and clean:

- `SolverResult::terminal_value()` matches FR-250's table row for row.
  `Stopped(limit)` settles `Incomplete(LimitReached{limit, value})` for any
  method. Otherwise only `(DeltaComplete, Unsat)` and
  `(FlowpipeOutwardRounded, EnclosureDisjoint)` pass `is_sound_for` and settle
  `Proved{BoundedSolver{method, horizon, jumps}, Uncertified}`. Every other
  pair of the 3 x 5 non-stopped grid falls to the exhaustive
  `Inconclusive(SolverInconclusive)` arm. No arm builds `Refuted`, and
  `Certification::Certified` appears nowhere in `hybrid.rs`.
- Soundness comes from the method kind alone. `SolverMethod` has no
  provider-claimed soundness field (FR-250 Behavior, QSpec FR-193).
- Certified or refuted paths: `ProofBasis` and `TerminalValue` derive only
  `Serialize`, so no wire reader can admit a `bounded-solver` basis.
  `TerminalRecord` has no public constructor that takes an arbitrary `Proved`
  value: `proved()` builds only `Checks`/`Certified`, and `from_value` is
  `pub(crate)` and is used only by test fixtures. The new `ProofBasis::BoundedSolver`
  joins the `Success` row of `category()` and touches no other match. Nothing
  outside this workspace matches on `ProofBasis`, `InconclusiveCause` or
  `ModelCheckLimit` (I searched every checkout in `~/dev`). See FND-004 for
  the one type-level gap.
- `basis()` returns FR-243's column exactly: `closed-scope` for proved,
  `unsettled` for inconclusive and `unavailable` for incomplete.
- `outcome.rs`: `SolverInconclusive` serializes as
  `{"kind":"solver-inconclusive"}` with category `inconclusive`. The
  `LimitReached` wire carries `limit`, `value` and `setting`.
- `SolverHorizon::new` reduces correctly. `gcd(0, d) = d`, so zero becomes
  `0/1`. The divisor is at least 1, so the `unwrap_or` fallback can never
  fire. The fields are private, so the reduced form always holds.
- Trace tags: four tests carry `#[trace("TC-705", "FR-250-AC-2")]` and each
  can fail on an AC-2 clause. (1) The δ-complete `Unsat` with horizon 10 and
  jumps 3 is asserted as the exact typed value and the exact wire item.
  (2) `DeltaSat`, `EnclosureDisjoint` from `FlowpipeUnrounded`,
  `EnclosureMeetsUnsafe`, `Unknown` and a method mismatch are each asserted as
  `Inconclusive(SolverInconclusive)` with basis `unsettled`. (3) `Stopped` is
  asserted on the exact wire `limit-reached` item. (4) The grid test really
  enumerates all 3 x 6 method/outcome cells, and it asserts its own claim
  (no `refuted`, no `certified`); it does not echo the implementation. It
  checks only the negative property, though (FND-003). Nothing is
  over-tagged: no AC-1 tag exists, and `a_horizon_is_reduced` carries no tag.
- "Delete `sound: true`": `git grep` on origin/main finds no `sound: true`
  field in any `.rs` file, so there was nothing to delete.
- No `unsafe`. The production code has no `unwrap`, `expect` or index panic,
  and no lossy integer conversion.

## Verdict

PASS, mergeable. The terminal-value map is exactly FR-250 / TC-705's table,
and no path lets a solver result settle `refuted` or `certified`. There are
three medium findings: the horizon wire diverges from FR-250's
`ExactRational`, the solver budget vocabulary is invented, and the
"not sound for its outcome" cells are not each asserted. There are also three
low findings. None is major, so none blocks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-250 Inputs types the horizon as `ExactRational`. The PR adds a local `SolverHorizon` with its own copy of `gcd`, and it serializes as bare JSON numbers (`{"numerator":10,"denominator":1}`). The repo's exact-rational wire is `{"kind":"rational","numerator":"10","denominator":"1"}`, with decimal strings so that a u64 above 2^53 keeps its precision for JSON consumers. `ExactRational` is in the root crate, which qsl-replay cannot depend on. The fix is to use a shared exact type, such as `quire_exact::Rational`, which qsl-replay already uses for `ClosedForm` evidence, with that wire form, or to amend FR-250 to name `SolverHorizon` and its wire. | qsl-replay/src/hybrid.rs:77-112 |
| FND-002 | medium | `ModelCheckLimit::SolverTime`/`SolverSteps`, their wire spellings `solver-time`/`solver-steps` and the settings `max_solver_time`/`max_solver_steps` do not appear in any QSL or QSpec spec. FR-250's `Stopped` row (after QSpec FR-193) says `setting` names the request member that raises the budget, but no request member with these names exists, and the unit of `value` (seconds? ms?) is not stated. The enum is also documented as EN-1's run limits. The fix is to spec the solver budgets in FR-250 / QSpec FR-193, or to name the provider's own limit, before this vocabulary reaches the wire. | qsl-replay/src/model_check/limits.rs:64-67; qsl-replay/src/model_check/limits.rs:79-93 |
| FND-003 | medium | Only 7 of the 18 method x outcome cells are asserted by exact value. The grid test accepts any `Proved{Uncertified}`, `Inconclusive` or `Incomplete` in every cell. A mutation that makes `is_sound_for` true for `(DeltaComplete, EnclosureDisjoint)`, `(FlowpipeUnrounded, Unsat)` or `(FlowpipeOutwardRounded, DeltaSat)` passes all four TC-705 tests. That leaves FR-250 row 2's "a result from a method not sound for its outcome" asserted for only two pairs. The fix is to give the grid an independent 18-entry expected table and assert each cell's exact `TerminalValue`. | qsl-replay/src/hybrid.rs:324-349 |
| FND-004 | low | The rule that a `BoundedSolver` proof is always `Uncertified` is held only by `terminal_value()`, not by the type. `TerminalValue::Proved{basis: BoundedSolver{..}, certification: Certified}` can be built, `category()` maps it to `success`, and `OutcomeItem` would serialize it as `"certified"`. QSpec FR-331-AC-23 says such an entry is refused. No public path builds a `TerminalRecord` from it today, so this is latent. The fix is to make the pairing unrepresentable, or to add a crate-level check where a record is built. | qsl-replay/src/proof_result.rs:396-404; qsl-replay/src/proof_result.rs:58-67 |
| FND-005 | low | `basis()` re-derives the basis through `terminal_value()`. Its unreachable arms map `Refuted`, `Unsupported`, `Declined`, `Failed` and `Tested` to `Unsettled`, which is a wrong FR-243 value for `Refuted` (`decisive-counterexample`) and `Unsupported` (`unavailable`) if one ever becomes reachable. The fix is to match on `self.outcome` and `is_sound_for` directly, so that no wrong arm exists. | qsl-replay/src/hybrid.rs:158-174 |
| FND-006 | low | The `SolverInconclusive` arm sits under the `ScalarAgrees` comment ("the claim identity and outcome stay on the typed cause"), which does not apply to a unit variant. The fix is to fold it into the unit-variant arm with `SufficientTestFailed`. Separately, the `hybrid.rs` module doc says a result with no provider settles `unsupported`, but nothing in this PR implements that (it is the FR-250-AC-1 remainder). | qsl-replay/src/outcome.rs:293-296; qsl-replay/src/hybrid.rs:2-8 |
