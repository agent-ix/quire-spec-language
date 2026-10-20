---
id: SR-13370
title: 'Spec review of PR #1072: FR-141 Stopped, StepResult, read precedence, base'
type: SpecReview
analysis: base
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
---

## Summary

Ticket: QSL-544. Spec-only amendments from the #1071 review (SR-13362/13363): FR-141 gains `Stopped(ReachedLimit)`, `check_initial(index)`, `ConcreteStep.position`, `StepResult{verdict, abstract_successors}` and a joint read precedence; FR-142 ends the run on `Stopped`; FR-140's population order names node-key digest order. This review checks wording, EARS, behaviour-not-changelog and whether the new ACs and TC steps can be checked by hand. make check-spec-validation exits 0 with 17 warnings, none in FR-140/141/142 or TC-546/555. tools/check-index-completeness.sh passes. git diff --check is clean. `quoin matrix` (0.28.3) lists FR-141-AC-9, AC-10 and AC-11 as `untagged`, which is expected for a spec-only PR: #1071 carries the tests. Clean units: the `index` and `ConcreteStep.position` contract (position 1 for the first step, matching FR-142's post-state index), the `StepResult` shape and FR-142's `max_transitions` counting, and FR-142's `Stopped` sentence, apart from FND-001's label wording. The new statements are EARS-conformant (ubiquitous and unwanted-behaviour forms).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-141-Stopped-limits | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-joint-read | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-Stopped | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-abstract-successors | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-check_initial-rule | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-9 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-10 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-141-AC-11 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| FR-140-read-order | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-142-Fault-Stopped | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| FR-142-limits | spec/functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md | examined |
| TC-546-step-6 | spec/test-cases/TC-546-check-step-decides-each-concrete-step.md | examined |
| TC-546-expected-6 | spec/test-cases/TC-546-check-step-decides-each-concrete-step.md | examined |
| TC-555-title | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md | examined |
| TC-555-step-5-6 | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md | examined |
| TC-555-expected-5-6 | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md | examined |
| FR-127-V-7 | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md | context_only |
| FR-126-stop-limits | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |
| QSpec-FR-379-budget-row | quire-specification spec/functional/analysis/FR-379-report-and-replay-refinement-counterexamples.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | 'FR-144 settles the item per FR-127's V-7 `incomplete`, not `failed`' (and TC-555 step 5 'never `failed`') does not say which field. FR-127's V-7 row gives the QSpec FR-360 label `failed` with execution `resource-incomplete`, `TerminalValue::Incomplete(LimitReached{limit, value, setting})` and O-16 category incomplete; QSpec FR-379 gives the FR-341 label `incomplete`. A test that reads the record's label gets `failed` from a correct FR-127 map and fails the AC. What separates a budget stop from AC-8's fault is the execution disposition and the terminal value. State the expected record: execution `resource-incomplete`, `TerminalValue::Incomplete(LimitReached{EvaluationMeter, 0, setting})`, category incomplete, not `TerminalValue::Failed`. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:228 |
| FND-002 | medium | The FR-142 half of AC-9 cannot fail for the defect it targets. FR-142's request has one `ModelCheckLimits` and gives no separate abstract meter, and QSL opens the abstract system with the concrete meter. With a budget of zero the concrete expansion of the first `RingIsQueue` state stops first, with the same `Stopped{ResourceExhausted, {EvaluationMeter, 0}}` (as FR-126-AC-5 does for `Counter`), before `check_step` runs. A broken `Stopped` arm, or one still mapped to `Fault`, would pass. 'From `head = 1`, `size = 2`' is also not something FR-142 can start from. The 65,537-value case names no fixture: it needs an abstract operation whose result is not bound by `binds_result`, since a bound result is never ordered. Give the FR-142 half a budget the concrete side passes and the abstract contract does not, or test it at `check_step` only, and name the 65,537-value variant. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:228 |
| FND-003 | medium | AC-10 fixes the receiver, an argument and a post-state row 'each to a stated outcome', and TC-555 step 6 uses 'an evaluator that fixes' them. Neither names a fixture. `concrete_system` is FR-120's concrete `ModelSystem`, not a seam, and FR-141 states no way to inject an outcome. The #1071 review already found no public way to inject one between two rows (gap-analysis FND-006). An incomplete receiver beside an undefined argument needs a receiver that runs out of a budget the argument does not, and the AC does not show how. 'The first of two undefined reads' names neither read. Name the fixture rows, for example the receiver through an `only` condition over a budget that the argument `g(self.index)` with `s0` fits in, or state the test seam the TC may use. Then name the two undefined reads and the one expected. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:229 |
| FND-004 | medium | AC-11 names no fixture. Every TC-546 fixture has one concrete initial state, so 'the initial state at index 2' does not exist in any of them. 'A step at position 3' gives no step. '`StepResult.abstract_successors` equals the number of abstract successors the call computed' is self-referential: it gives no number, so a test can compare the field only with itself. TC-546 step 6's expected result ('equal to the successors computed') has the same problem. Name a subject with at least three initial states, the rejected step, and an exact expected count, for example Coin `toss` with `side` hidden giving 2. Also say what counts as a successor on the abstract-operation path, which judges one candidate `a'` and does not expand. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:230 |
| FND-005 | low | The `Stopped` bullet covers `check_initial` ('while `check_step` or `check_initial` decides'), but no AC tests it. Under the initial-state rule, `check_initial` takes the abstract initial states from FR-106 snapshots and compares state keys, so it evaluates nothing on the abstract side and cannot exhaust the meter or `max_candidates`. Either drop `check_initial` from the bullet or name the evaluation that can stop it and add a case. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:193-198 |
| FND-006 | low | '; it uses no out-parameter' describes the change from the code PR's `&mut u64` parameter, not the system's behaviour. A future reader of FR-141 has no out-parameter to compare against. The return type in Inputs already says it, so delete the clause. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:199-201 |
| FND-007 | low | TC-555 is titled 'A refused or incomplete mapping row or argument leaves the refinement undetermined, and an undefined update refutes'. Its scope now also covers FR-141-AC-9 (an abstract-side budget stop, which leaves no item undetermined) and AC-10 (read precedence). The title and the Description no longer describe what it verifies. Retitle it, or move steps 5 and 6 to a new TC. | spec/test-cases/TC-555-a-refused-or-incomplete-mapping-row-leaves-the-refinement-undetermined.md:3 |

## Verdict

Findings: 4 medium, 3 low; no high. The new ACs (AC-9 to AC-11) are not yet checkable as written: no fixtures, a self-referential count, and an FR-142 clause the concrete side passes on its own. Fix FND-001 to FND-004 before merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | AC-9's `HeavyEnq` requires only that 'every concrete contract evaluation fits in B'. The mapping rows (`items` over the slots) and the step-row arguments are also evaluated under the concrete meter B, so a fixture that meets the AC as written can return `Undetermined(MappingUndetermined)` before the abstract side runs. 'After the concrete expansion completed its states' also misdescribes FR-142, which runs `check_step` on each edge as it explores, so the stop comes at the first `put` edge, not after all concrete states. Say 'every concrete contract, mapping-row and step-row-argument evaluation fits in B', and 'after the concrete expansion of the initial state completed'. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:241 |
| FND-009 | low | In `Coin3`, indexes 0 and 1 both have `tossed` false. Two distinct initial states must then differ in `shown` or `face`, and those are mapped by name. So index 1 maps to a non-initial abstract state and also fails `InitialNotAbstract`, and FR-142 over `Coin3` would report index 1, not 2. 'Coin's `toss` ... from its initial state' does not say which of the three. Give index 1's field values (or make it pass), and say `toss` runs from index 0. | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md:243 |
| FND-010 | low | FR-145-AC-6 replays 'a payload whose prefix reaches `HeavyEnq`'s `put(1)` step' but does not say where the payload comes from. Under B, `HeavyEnq` stops with no counterexample, and under a larger meter it has a counterexample only if it fails. FR-145's use case says a hand-edited counterexample does not count. FR-149-AC-5's 'certificate for `HeavyEnq`' likewise needs `HeavyEnq` to hold under a larger meter, and the AC does not state that. Name the sources: for example, a `HeavyEnq` variant with the broken `take` run under a larger meter and replayed under B, and the certificate from a `Holds` run under a larger meter. | spec/functional/FR-145-replay-a-refinement-counterexample.md:128; spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:139 |

## Dispositions

Round 1, head 989dfbe67 (fix commit 989dfbe67 over merge e74b107e3 of main fe54ec4b6). Renumbered from SR-13367 to SR-13370: quire-specification#291 holds SR-13367 to SR-13369. make check-spec-validation exits 0 (17 warnings, none in the touched files); tools/check-index-completeness.sh passes; git diff --check is clean; quoin matrix lists the new ACs (FR-140-AC-15, FR-141-AC-9 to AC-11, FR-145-AC-6, FR-149-AC-5) as untagged, which is expected for a spec-only PR. Counts checked by hand: AC-11's lost-update `commitB` judges one identity `inc(c)` against one `a'`, so 1; `Coin` `toss` with `side` hidden judges `toss(c)` against the two candidates `side` 0 and 1, so 2. Engine probe on the #1071 code (qsl-544-part2 1c3b7fae6, `qsl-replay/tests/it/refinement_step.rs` world): `commitB` from (1, 0, f, 0, t) at position 3 returns `Fails(AbstractStepRejected{position: 3, inc(c), Postcondition})` with `abstract_successors` 1. The `toss` count of 2 is hand-derived only: that code accepts a single abstract state, and the hidden-field candidate set is unbuilt. HeavyEnq and Reads are checked by hand against FR-120's fresh-meter-per-clause rule and the joint-read order; the engine has no `Stopped` arm yet.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

Verdict after round 1: all 7 original findings fixed; 3 new low findings (FND-008 to FND-010) on the new fixtures. No blocker from this lane.
