---
id: SR-13371
title: 'Spec review of PR #1072: FR-141 Stopped, StepResult, read precedence, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@37a472e7f; git diff refs/remotes/origin/main...37a472e7f (PR #1072, base main 891825c45): spec/functional/FR-140, FR-141, FR-142; spec/test-cases/TC-546, TC-555; read against QSpec main 9d7268c43'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-141
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-140
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-142
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-145
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-149
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-120
  type: reviews
---

## Summary

Ticket: QSL-544. Consistency of the amended FR-140/141/142 with FR-120, FR-126, FR-127 V-7, FR-144, FR-145, FR-146 (QSL-548) and FR-149, and with QSpec FR-375, FR-377, FR-379 and FR-383 on QSpec main 9d7268c43. Checks against QSpec: FR-375 does sort node body arrays by the node-key digest of the first named declaration (:397-400). FR-377's failure precedence (:85-99) does rank undefined above refused, incomplete or unmapped, and puts a fault outside the order with the item `failed`. FR-383 (:193) does settle an internal fault `failed`. FR-120's 65,536-value rule says only 'the same way' (:449-450), and FR-127's V-7 row maps a reached limit to `incomplete` (category). The new `Stopped` arm leaves the Fault arm and FR-141-AC-8 intact: the Fault bullet still excludes `Fails`, `Undefined` and `Undetermined`; the `Stopped` bullet excludes `Fault`; FR-144's Fault-to-`failed` rule is unchanged; and FR-142 keeps the two endings apart. QSL-546's FR-139 has no order or verdict rule these amendments touch.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-141-Stopped-limits | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-joint-read | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-Stopped | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-Fault | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-8 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-140-read-order | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-142-Fault-Stopped | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| FR-142-limits | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| FR-145-Prefix | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-149-Limits | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | examined |
| FR-127-V-7 | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md | examined |
| FR-146-row-order | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md | examined |
| FR-120-result-domain | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | examined |
| FR-126-stop-limits | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-140-EvalFailure | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| QSpec-FR-375-node-order | quire-specification spec/functional/analysis/FR-375-declare-model-refinement.md | examined |
| QSpec-FR-377-failure-precedence | quire-specification spec/functional/analysis/FR-377-decide-model-refinement.md | examined |
| QSpec-FR-379-budget-row | quire-specification spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The joint-read rule makes only 'the first read that is refused or incomplete' undetermined. FR-140's `EvalFailure` has two more undetermined outcomes: `OutOfType` (a history update leaves its declared type, which can occur on the post-history read) and `UnmappedType` (a post-state or reference read of an unmapped type). QSpec FR-377 says 'refused, incomplete or unmapped', and FR-379 counts an out-of-type history update. With an out-of-type post history and an incomplete post-state row, the rule as written reports the row and skips the history. Two implementers would report different objects. Say 'the first read that is undetermined (refused, incomplete, out of type or unmapped)'. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:151-159 |
| FND-002 | medium | Reading the receiver, arguments, post history and post-state 'together' means a step can have an undefined read and a `MappingFailure::Fault` read. The joint-read bullet then says it SHALL return `Undefined` for the first undefined read, and the Fault bullet says a Fault SHALL return `StepVerdict::Fault`. The two SHALLs conflict. QSpec FR-377 settles it: a fault 'takes no part in the order' and the item settles `failed`. FR-140-AC-14 states this for rows inside one `map_state`, but FR-141-AC-8 covers only an earlier undefined *step*, not an earlier undefined read in the same step. State that a fault in any read returns `Fault` whatever the other reads return, and add that case to AC-8 or AC-10. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:151-159 |
| FND-003 | low | The bullet applies FR-140's precedence '(QSpec FR-377 "Failure precedence") across all of them'. QSpec FR-377's rule covers only 'the rows of one `map(s, h)`', and FR-379 lists argument, history and map outcomes without ordering them. So ordering the receiver, arguments, post history and post-state is a QSL rule, and the citation reads as if QSpec stated it. QSL is now deterministic, but the cross-engine divergence the #1071 review found (code-review FND-002) stays open in QSpec. Say 'QSL extends FR-140's precedence to ...', and record that QSpec FR-377/379 needs the same order. The planner can file that, since this review cannot create tickets. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:151-159 |
| FND-004 | medium | The rule that a result domain of more than 65,536 values reports `MaxCandidates` is stated only in FR-141, for the abstract side. FR-120 says only that such a domain 'cannot be ordered and stops the expansion the same way', and names no limit. FR-126 and FR-142 map `MaxCandidates` only for 'reached `max_candidates`'. On main, the concrete expansion of such a domain stops with no limit (`stop(EXHAUSTED)`, `qsl-eval/src/simulation/model/frames.rs:843`), which the engine turns into `Stopped{ResourceExhausted, None}`, and FR-127 has no V-7 row for that. So one domain reports differently on the two sides, and the `value` is unstated: the configured `max_candidates` was never reached. 'stops the expansion on "the same way"' is also garbled. Put the rule and its value in FR-120 and FR-126 for both sides, and have FR-141 cite it. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:104-107 |
| FND-005 | medium | FR-141 names FR-142, FR-143, FR-145 and FR-149 as its callers, but only FR-142 says what a `StepVerdict::Stopped` does. FR-145 replay runs `check_step` along the prefix and settles `inconclusive`, `ReplayParity` for 'a failure of a different kind ... or no failure'. A replay stopped by the abstract meter would then read as a parity disagreement, not a budget stop, and FR-145 has no rule for it. FR-149's Limits bullet maps only its own position and edge counts, not a `check_step` that stops. Add a `Stopped` rule to FR-145 (settle V-7 with the limit, never `ReplayParity`) and to FR-149 (its `Stopped` with that limit). | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:193-198 |
| FND-006 | medium | 'the populations in ascending node-key digest order of their declarations' does not say which declaration. A population row `population Q::cells from R::slots` names two: the abstract population and its concrete source. QSpec FR-375 sorts by 'the node-key digest of its first named declaration, then by member name', and FR-146 (merged in QSL-548) uses that exact wording for `steps`. An implementer who reads 'their' as the concrete source population sorts differently, and the reported row and object then differ (the #1071 review's code-review FND-003). No AC fixes the order: no FR-140 AC has two failing populations, so the #1071 divergence would still pass. Use FR-146's wording ('the row's first named declaration, the abstract population') and add an AC with two populations whose digest order differs from declaration-key order. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:158-162 |

## Verdict

Findings: 5 medium, 1 low; no high. The Stopped arm is sound for FR-142 and leaves the Fault arm and FR-141-AC-8 intact. But FR-145 and FR-149 do not handle it, the 65,536-value rule lives on one side only, and the joint read precedence leaves out OutOfType, UnmappedType and Fault. Fix FND-001, FND-002, FND-004, FND-005 and FND-006 before merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | FR-149's output is `CertificateCheck::Stopped(IncompleteCause, ModelCheckLimit)`. `ModelCheckLimit` is the bare limit kind, so the result has no value field. The new Limits sentence ('naming that limit and its value') and FR-149-AC-5 ('with value B') require a value the type cannot carry, and FR-144 needs one to settle V-7 as `Incomplete(LimitReached{limit, value, setting})`. AC-5's 'value B' cannot be checked against the declared output. Make the arm `Stopped(IncompleteCause, ReachedLimit)` (FR-126's `{limit, value}`), as FR-141's `Stopped` already carries. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:83,119-123,139 |

## Dispositions

Round 1, head 989dfbe67 (fix commit 989dfbe67 over merge e74b107e3 of main fe54ec4b6). Renumbered from SR-13368 to SR-13371: quire-specification#291 holds SR-13367 to SR-13369. make check-spec-validation exits 0 (17 warnings, none in the touched files); tools/check-index-completeness.sh passes; git diff --check is clean; quoin matrix lists the new ACs (FR-140-AC-15, FR-141-AC-9 to AC-11, FR-145-AC-6, FR-149-AC-5) as untagged, which is expected for a spec-only PR. Counts checked by hand: AC-11's lost-update `commitB` judges one identity `inc(c)` against one `a'`, so 1; `Coin` `toss` with `side` hidden judges `toss(c)` against the two candidates `side` 0 and 1, so 2. Engine probe on the #1071 code (qsl-544-part2 1c3b7fae6, `qsl-replay/tests/it/refinement_step.rs` world): `commitB` from (1, 0, f, 0, t) at position 3 returns `Fails(AbstractStepRejected{position: 3, inc(c), Postcondition})` with `abstract_successors` 1. The `toss` count of 2 is hand-derived only: that code accepts a single abstract state, and the hidden-field candidate set is unbuilt. HeavyEnq and Reads are checked by hand against FR-120's fresh-meter-per-clause rule and the joint-read order; the engine has no `Stopped` arm yet.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |

Verdict after round 1: all 6 original findings fixed; 1 new medium (FND-007): FR-149's `Stopped` carries no value while AC-5 and the Limits bullet require one. Not mergeable until FND-007 is fixed.
