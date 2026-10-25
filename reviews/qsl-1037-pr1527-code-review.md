---
id: SR-17204
title: "QSL-1037 code and Rust review of the on each guard S3 check and S4 emission (quire-spec-language PR 1527)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a35be7dc256a30941b6be8ef5256a445f41950c4; git diff origin/main...HEAD (9 files); qsl-semantics/src/check/{temporal_clause.rs,temporal_clause/condition.rs,temporal_clause/tests.rs,lowering/temporal.rs,mod.rs,assemble.rs,refusal.rs}; qsl-replay/src/spine.rs; qsl-package/src/emit/tests/temporal_clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
---

## Summary

Ticket: QSL-1037 (PR quire-spec-language#1527, PR 1 of 2). `/code-review`
with the `/rust-review` lane over `git diff origin/main...HEAD` at
`a35be7dc2`, in a private detached worktree. Spec: FR-337 "`on each`
activation at S3/S4" and AC-6/AC-7, TC-1265 (#1524), with FR-125-AC-18,
FR-126, FR-218, FR-338, FR-362, ADR-033 and TC-1201 read for context.

Checked and found sound:

- S3 typing. The trigger is typed with the other parameters. `T != U`
  refuses `ill_typed`/`type-mismatch` at `T`'s type form. A binder named as
  `over` refuses `conflicting_declaration`/`duplicate-declaration` at the
  binder name and carries the `over` name span as `first`. The guard is typed
  by the same `type_condition` as a `holds` condition: `ClauseKind::Invariant`,
  a population position, FR-104 definedness over the current-state
  observation, with `over` at slot 0 and `q` at slot 1. The guard is typed
  before the `holds` conditions. `q` is not bound for `holds`, so a read of
  `q` in the body is `missing-name`.
- AC-7 codes. `when (c.value)` gives `non-boolean-root` at the guard.
  `c.value = c.value + 1` gives `unproved-range` at the sum. The `mod` guard
  checks. `M::Absent` gives `missing-name`. `Integer` gives `type-mismatch`.
  `on each (c: ..)` gives `duplicate-declaration` at the binder. A
  non-Boolean guard and a non-Boolean body refuse once, at the guard. A
  capture is still `NotEmitted`. The tests assert each code, tag and region
  start.
- The two new `TemporalCause` variants (`TriggerType`, `DuplicateTrigger`)
  map to the existing `ill_typed`/`type-mismatch` and
  `conflicting_declaration`/`duplicate-declaration` pairs, and the
  definedness path reuses the existing causes. FR-337 says the rule "adds no
  refusal cause" and calls these "new sites of the existing ... causes", so
  this reading is consistent: no new code/tag pair or wire field. The
  variants are internal carriers, as `DuplicateTimeSource { first, span }`
  already is. Not a finding.
- S4 lowering. `kind` is `each`. `trigger` is a `LeafTerm::reference` to a
  level-1 `parameter` node typed `over_type`, and its `expression` and
  `type` occurrences sit at the binder name. `guard` is lowered with
  `Binders` `[scope[0], trigger@1]` and is bound only when written. Key
  stability: a guard-less clause, `when (true)`, a guarded clause and
  `on origin` give four keys and four `package_id`s. A changed guard and a
  renamed binder change the key. Reformatting changes neither. The test
  compares real digests.
- `qsl-package/src/emit.rs` (draco's S4 emitter) is untouched. The change
  goes through the generic lowering, so no S4 emitter code changed.
- Trace tags are `#[trace(...)]` only. No `/// Trace:` lines were added.
- No new production `unwrap`/`expect`/`panic!`/indexing.

Reviewer probes (private worktree, reverted after; never pushed):

- P1: `qsl-analyze` `model_instances` dogs world. `always holds(false)`
  `on each (y: M::Dog) when (false)` compiled from source, then run through
  `ModelSystem::clause_letters` and `check_model`. Result: `Ok(Violated {
  over_binding: k, prefix: [] ... })`. A guard that is never true activates
  nowhere, so the clause should hold.
- P2: `qsl-replay` spine `run_clause` `Temporal` over a finite trace 0,1,2.
  `eventually[0,1] holds(c.value = 2)` `on each (q: Counters::Counter) when
  (c.value = 1)` gives `EvaluateFault(InternalFault { stage:
  "temporal.observed", invariant:
  "emitted-clause-has-no-activation-condition" })`.

On main, both clauses were refused `NotEmitted` at compile time.

Mutations (12, run against `cargo test -p qsl-package -p qsl-semantics --lib
temporal_clause`) are recorded in SR-17205 (gap analysis).

## Verdict

FAIL, on FND-001. The S3 check and S4 emission match FR-337-AC-6/AC-7, and
the tests are real. But removing the `on each` arm of `is_emitted` lets
guarded clauses compile, and today both production consumers misread them.
EN-1's safety path returns a wrong `Violated`, and the FR-330 trace path
returns an internal fault. PR 2 is said to cover the letters. Either land the
two PRs together, or keep a guarded clause refused at these seams (or
`NotEmitted`) until PR 2. FND-002 and FND-003 are low.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | `is_emitted` no longer refuses `on each`, so a guarded clause now reaches consumers that ignore its guard. `ClauseLetters::activation` returns `True` unconditionally, and EN-1's safety path has no `GuardedActivation` refusal (only TP-4 liveness does). Probe P1: `when (false)` + `always holds(false)` returns `Violated` instead of holding vacuously. `ObservedLetters::activation` faults, so FR-330's `run_clause` returns an internal fault for every guarded clause (probe P2). On main both were refused `NotEmitted`. PR 2's stated scope ("letters, remove GuardedActivation") and the FR-337 notes do not mention the observed-trace seam | qsl-semantics/src/check/temporal_clause.rs:1552-1556; qsl-eval/src/simulation/model/letters.rs:269-282; qsl-eval/src/temporal/observed.rs:178-189; qsl-analyze/src/model_check/check.rs:1321-1327 |
| FND-002 | low | Check order is not FR-337's "activation, the binder's type and name and then the guard, before the clause's holds conditions". Capture parameter types are resolved in the same loop before the trigger type/name checks. Interval, fairness and profile checks in `check_clause` also run before the guard is typed in `check_conditions`. So `on each (q: Integer)` with a capture of unresolved type refuses at the capture (body), and a non-Boolean guard with an inverted interval refuses at the interval | qsl-semantics/src/check/temporal_clause.rs:831-837,875-908; qsl-semantics/src/check/temporal_clause/condition.rs:76-107 |
| FND-003 | low | Comments the PR makes false are left in place: "An emitted clause activates `on origin` (FR-337)" (letters.rs `clause_letters` doc and `activation`, observed.rs `activation`), "S4 emits no `on each` clause" (qsl-analyze model_world.rs, model_check_safety.rs module doc), and check.rs's module doc claim that a guarded clause "is refused before any state is explored" (true only for TP-4) | qsl-eval/src/simulation/model/letters.rs:91-96,269-270; qsl-eval/src/temporal/observed.rs:183-184; qsl-analyze/tests/it/model_world.rs:282-284; qsl-analyze/tests/it/model_check_safety.rs:11-13,22-23; qsl-analyze/src/model_check/check.rs:64-66 |

## New findings (disposition pass 1)

Disposition pass 1 reviews head `bbc203e57`. It contains the fix commit
`26a3f56b0` (the former PR 2 folded in) and a merge of main through #1529.
Probe P1 was re-run and now gives `Holds`; `when (false)` holds vacuously.
Probe P2 was re-run over the traces 0,1,2 / 0,0,0 / 1,1,1 / 1,2,2 and gives
Holds / Holds / Violated at 0 / Holds, which is correct for the guard
`q.value = 1` and the property `eventually[0,1] holds(c.value = 2)`.

The liveness and POR changes are sound:

- The lasso letter reads the `when` condition before the atoms (UE-1). An
  undefined guard records `UndefinedLocus::Activation` and stops the atom
  reads.
- `Atoms.guarded` is taken from `automaton.when()`, which `translate` sets
  only for `Activation::Each { guarded: true }`. So POR adds the guard's
  footprint exactly for guarded clauses.
- `derive_guard_reads` maps both binders (slots 0 and 1, the only `Param`s
  of that scope) to `Receiver`, which matches their aliasing.
- The safety path still issues no closure certificate for a guarded clause
  (P1 gives an empty `ClosureCertificate`), as before.

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-004 | low | The refactor into `ObservedLetters::reading` relabels the `Evaluated(Refused) / FamilyEvaluated(Refused)` arm from the refused code (`holds-condition-refused`) to the Boolean code (`holds-condition-boolean` / `activation-condition-boolean`). The arm is unreachable today because `refused` returns first, but it now names the wrong fault if it ever becomes reachable | qsl-eval/src/temporal/observed.rs:173-174 |

## New findings (disposition pass 2)

Disposition pass 2 reviews follow-up PR quire-spec-language#1538 at head
`3637c7461` (branch `qsl-1037-guarded-activation`). #1527 was merged at
`763d8b525`, before the round-1 fixes. #1538 contains `16a052900` plus a
merge of main through #1535.

Probes re-run on `3637c7461`:

- P1 gives `Holds`.
- P2 over the traces 0,1,2 / 0,0,0 / 1,1,1 / 1,2,2 gives Holds / Holds /
  Violated at 0 / Holds.

Merge-resolution regions reviewed:

- `evaluate.rs`. Main changed the absent-parameter state to a per-slot
  `Vec<AbsentParameter>`. `with_absent_parameters` now pushes one
  `key: None` entry per slot with the same binding, so slots 0 and 1 both
  carry provenance and `absent_key_halt` falls back to the first entry's
  binding. Sound.
- `temporal_clause.rs`. The check order is unchanged: the trigger type and
  name are checked at loop position 1, before the capture types. Only a
  blank line was lost.
- `expression/mod.rs`. `evaluate_guard` sits beside main's
  `evaluate_hyper_holds` and `evaluate_step_predicate`, unchanged.
- `condition.rs` and the emit test are taken from the branch side, with no
  semantic change.
- `observed.rs`. The resolution keeps main's (#1531, QSL-902) first-undefined
  recording in `atom` only. FND-005 below.

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-005 | medium | After the merge with QSL-902, `ObservedLetters` records `ObservedRun.undefined` only from `atom`, and that record is typed `(FormulaNodeId, ..)`. An `on each` guard that is undefined at the deciding position (`activation`) records nothing, or a later undefined atom is recorded in its place. FR-331 observed replay settles an `UndefinedEvaluation` packet only when `actual_locus` is `Some` and equal to the packet's `locus`, so a guarded clause whose guard is undefined can never be `reproduced-with-evaluated-witness`; it always settles `inconclusive`/`ReplayParity`. The packet's `locus: FormulaNodeId` also has no way to name the activation, and FR-331 states no activation locus, so this needs a spec ruling (an activation locus, like `UndefinedLocus::Activation` in EN-1) or an explicit refusal of guarded clauses in observed replay. Deferral to a ticket is acceptable | qsl-eval/src/temporal/observed.rs:47-48,250-276; qsl-replay/src/result.rs:849-858; qsl-replay/src/execute.rs:1049 |

## Dispositions

| FND | outcome | reason |
|-----|---------|--------|
| FND-001 | fixed | GuardedActivation is removed. `ClauseLetters::activation` evaluates the guard (`evaluate_activation_guard`, including the absent-key form), and `activation_reads` gives its footprint. `ObservedLetters::activation` evaluates the guard through `evaluate_guard`. Liveness reads `when`. Probes P1 and P2 are now correct |
| FND-002 | fixed | The trigger type and name are checked at position 1 of the parameter loop, before the capture types. When the body refuses (a `Temporal` cause after the binder, or `NotEmitted`), `guard_refusal` types the guard first. Covered by `the_activation_is_checked_before_the_captures_and_the_body` |
| FND-003 | fixed | The stale comments are rewritten (letters.rs, observed.rs, model_world.rs, model_check_safety.rs, check.rs module doc). A grep finds none left |
| FND-004 | fixed | `Evaluated(Refused) / FamilyEvaluated(Refused) => Err(stop(refused_code))` (16a052900) |
