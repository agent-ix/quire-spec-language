---
id: SR-13071
title: "Code review (Rust lane) of QSL-551 part 1 FR-175 lockstep tuple evaluator (PR #1021)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@429b36783; qsl-eval/src/temporal/{engine,mod,outcome,tuple}.rs, qsl-eval/tests/it/{hyper_tuple,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-600
    type: reviews
---
# SR-13071: Code review (Rust lane) of QSL-551 part 1 FR-175 lockstep tuple evaluator (PR #1021)

## Summary

Ticket: QSL-551 (part 1, FR-175 only). PR quire-spec-language#1021 at head 429b36783, scoped to `git diff origin/main...HEAD` (6 files). The rust-review lane is folded into this file. Gates run on the worktree: `cargo test -p qsl-eval` (573 + 83 pass), `cargo clippy -p qsl-eval --all-targets -D warnings` clean, `cargo fmt -p qsl-eval --check` clean. `make ci` was not run, per the brief. Two findings were reproduced with probe tests in a scratch copy of the tree (not in the worktree): /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1021-probe/repo/qsl-eval/tests/it/hyper_tuple.rs.

## Verdict

**Not mergeable as is.** FND-001 is a wrong `trace_position` on a case FR-175-AC-2 itself names (a past operator under `always` on a lasso).

The engine `Subject` refactor in `engine.rs` changes no behaviour. `Subject::of_clause` copies exactly the five accessors `run`, `Evaluator::new` and `execute` read before (formula, profile, activation, horizon, fairness), `evaluate`/`evaluate_at`/`unrolling` keep their public signatures, and no old path, shim, re-export or fallback is left. The whole qsl-eval suite passes unchanged. The only leftover is `unrolling_of` (FND-003).

The rest of `tuple.rs` is sound: the joint stem (max) and period (lcm via gcd with `checked_mul`), prefix-then-loop component indexing, the terminal stutter flag (`joint >= len`), the `TupleArity` refusal and its catalog code, and the Kleene-monotone bisection on safety bodies (the engine's Prefix mode returns `Pending` for a non-safety body, so a liveness body correctly falls through to the last represented position).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `false_position` searches only joint prefixes up to `represented` (stem + period). The engine unrolls a body with past reach R to n + m*L positions (SM-8), so a past-operator body can be falsified only in an unrolled copy. Then `bad(represented)` is `Pending` and the function returns `represented - 1`, a position where the body is not yet false. Reproduced: `always (holds(c.value @ a = 1) implies not once[2,2] holds(c.value @ b = 2))` over `[lasso(0,2), lasso(0,2)]` with a=[1,0], b=[2,0] returns `Violated{at:1}`. The same behaviour written as `[lasso(0,4), lasso(0,4)]` returns `Violated{at:2}`, the correct first fixing position. So `trace_position` depends on how the lasso is written, which breaks FR-175 Outputs and "a function of the body, the traces". Fix: use the unrolled length as the search bound (`unrolling_of(subject, trace)?.positions`; `Lockstep` already reads any joint position by wrapping). Add this case to TC-600's past-operator test. | qsl-eval/src/temporal/tuple.rs:183-214 |
| FND-002 | medium | Locating `trace_position` re-runs the whole evaluator 1 + log2(n) times on finite prefixes. Each run re-reads every atom through the caller's letters and charges the clause meter again. Measured on an 8-period tuple violated at joint position 6: 40 work units and 33 atom reads, against 9 units and 8 reads for the holding tuple. FR-175 "Work" says one TR-5 unit per (temporal node, joint position) visit. As written, a meter sized for the evaluation can stop (Incomplete) a tuple whose verdict was already reached within budget, and letters that do real work (FR-107's clause evaluator) pay for it again on every run. Find the first fixing position in one pass (for example, memoise the letters once and do the prefix search off the meter), or get the spec to state that the position search is charged. | qsl-eval/src/temporal/tuple.rs:189-214, 246-249 |
| FND-003 | low | `unrolling_of` was split out of `unrolling` as `pub(super)`, but its only caller is `unrolling` itself; `tuple.rs` never uses it. It is an unused generalisation left by the refactor. Either inline it back, or use it for FND-001's search bound, which would give it a reason to exist. | qsl-eval/src/temporal/engine.rs:281-297 |
| FND-004 | low | `evaluate_tuple` checks the tuple's arity but not the clause's alignment. A `CheckedHyperClause` with a non-empty `align_skip()` (HP-6, projection-aligned, FR-178) is evaluated in lockstep without any signal, which is not its meaning. FR-175 covers lockstep only. Refuse a clause with `align skip`, or state at the API that the caller must not pass one. | qsl-eval/src/temporal/tuple.rs:226-238 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The position search reads joint positions from `represented` up to the unrolled length through the caller's `TupleLetters`, and charges them to the search's own `UNBOUNDED` meter. The `readings` cache is keyed by joint position, so joint position 2 on a period-2 tuple is a fresh read of the same component positions as joint position 0. Probe: the past-operator tuple `[lasso(0,2), lasso(0,2)]` reads atoms at joint positions 2 and 3, after the clause-metered pass read only 0 and 1. Any work those letter calls do (FR-107's clause evaluator) is outside the clause meter, against FR-175 "Work" ("a clause meter SHALL stop it"). The doc comment's "no atom the first pass read is read twice" also holds only by index. Fix: before the cache lookup, map a joint position at or past `represented` to its congruent represented position (prefix + (j - prefix) mod period). Every search read is then a cache hit, and the separate meter is no longer needed. | qsl-eval/src/temporal/tuple.rs:150-169, 215-237 |
| FND-006 | low | `UNBOUNDED` copies, field by field, an all-`u64::MAX` `ScalarLimits` that `qsl_semantics::check::SCALAR_LIMITS_UNLIMITED` already provides. A field later added to `ScalarLimits` has to be kept in step in two places. Reuse the existing constant, or drop the separate meter altogether under FND-005's fix. | qsl-eval/src/temporal/tuple.rs:190-202 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | Regression of FND-002, which round 1 had fixed. The position search runs on the caller's meter again, so locating `trace_position` re-charges node visits for every bisection step. Probe on an 8-period tuple violated at joint position 6: 40 work units, against 9 for the same tuple when it holds. A work limit of 11, above what the evaluation needs, returns `Incomplete` instead of `Violated`. FR-175 "Work" asks for one unit per (temporal node, joint position) visit. `the_position_search_reads_each_represented_position_once_on_the_callers_meter` asserts only that `work` fits and `work - 1` does not, so it pins the over-charge rather than the rule. Every lasso read is now cached by its matching represented position, so the search no longer reaches the caller's letters (except FND-008's terminal path). The search can therefore go back on its own meter without hiding letter work. Alternatively, get the spec to state that the search is charged. | qsl-eval/src/temporal/tuple.rs:222-275, 312-319 |
| FND-008 | medium | Missed in the review pass. `Terminal { len }` is indexed as the lasso `(len - 1, 1)`, so the loop-entry joint position is the terminal state reached by a real step. The lasso evaluation reads every later joint step through that entry with `stutter = false`. Probe: `[Terminal{len:2}, lasso(0,1)]` gives stutter flags only `(0,[F,F])` and `(1,[F,F])`, never `true`, although `a` stutters at every joint step from 2 on (FR-175 "Terminal stutter", AC-3 "at every later joint step"). The AC-3 test sees `true` only because `b`'s period of 3 makes joint positions 2 and 3 represented. The position search does see the flag: `congruent` refuses the match there, so search steps make fresh letter reads at joint positions 2 and 3 with `stutter = true`. The main pass and the search therefore read different letters for the same position, and `false_position`'s doc ("Every atom it reads is a reading the first pass made") is false. Fix: give a terminal component one stutter position after its last state, as the lasso (len, 1) reading index len - 1. Add an AC-3 case whose other component has period 1. | qsl-eval/src/temporal/tuple.rs:55-77, 163-176 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The position search runs on a new `Meter` built from the caller's limits with no work cap. A `Meter` built that way has no cancel handle (and no injected denial), so a caller's FR-276 cancel is not seen during the search. `quire_exact::Meter` has no accessor to copy the handle across. The search's cost is bounded at about 1 + log2(bound) evaluations, so cancellation is delayed rather than lost, hence low. Fix: pass the caller's `Cancel` into `evaluate_tuple`, or add a `Meter` method that forks an uncapped meter sharing the handle. Otherwise defer with a reason. | qsl-eval/src/temporal/tuple.rs:248-252 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | `evaluate_tuple` now takes `cancel: Option<&Cancel>` beside a `meter` that may already carry the same handle. Nothing ties the two together. A caller that builds the meter `with_cancel(c)` but passes `None` (the obvious call, as every existing test does) gets a position search that is never cancelled, and passing a different handle splits the two. The doc says "`cancel` is the handle `meter` polls", but that is a convention, not something the code checks. It is a workaround for `quire_exact::Meter` having no way to read its own cancel handle. Follow-up: add a `Meter` accessor (or a method that makes an uncapped copy) in quire-exact and drop the parameter; until then, defer with that reason. The doc sentence at line 300 also runs past the wrap width. | qsl-eval/src/temporal/tuple.rs:296-308 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Partly fixed. The search now runs to the unrolled length, and the reproduced unrolled-copy case returns `Violated{at:2}`; the new test fails on the old `false_position` (`at:1` against `at:2`). But a violation at the last unrolled position whose bounded-future subformula reads past it still falls back to `represented - 1`. Probe: `always ((holds(c.value @ a = 1) and once[2,2] holds(c.value @ b = 2)) implies eventually[1,1] holds(c.value @ a = 1))` over `[lasso(0,2), lasso(0,2)]` with a=[0,1], b=[0,2] returns `Violated{at:1}`, while the same behaviour as `[lasso(0,4), lasso(0,4)]` returns `Violated{at:4}`. Extend the search bound past the unrolled length by the body's future reach (or grow it until the prefix is bad), and add this case to the past-operator test. |
| FND-002 | fixed | fixed at dc63ebd86 |
| FND-003 | fixed | fixed at dc63ebd86 |
| FND-004 | fixed | fixed at dc63ebd86 |
| FND-001 | fixed | fixed at 158d7c97c |
| FND-005 | fixed | fixed at 158d7c97c |
| FND-006 | fixed | fixed at 158d7c97c |
| FND-007 | fixed | fixed at f79c167d5 |
| FND-008 | fixed | fixed at f79c167d5 |
| FND-009 | fixed | fixed at 864eebeff |
