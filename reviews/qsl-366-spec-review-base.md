---
id: SR-961
title: "QSL-366 base spec review of ADR-018, FR-123 to FR-128 and TC-518 to TC-523"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9b546eb069048b46d0c365f85c4481c0a3012115; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md; spec/usecase/US-015-check-a-temporal-property-over-every-behaviour-of-a-model.md; spec/functional/FR-123..FR-128; spec/test-cases/TC-518..TC-523; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
---

## Summary

Ticket: QSL-366, PR quire-spec-language#562, head 9b546eb0. Review set:
base checklist plus integrity, scope-boundary and EARS (SR-962 to SR-964).
This file holds the base checklist, the owner-ruling check and the
technical-soundness check.

`quire validate --scope . "spec/**/*.md"` reports no error or grammar
warning in any file this PR adds or changes; the 7 failing documents are
pre-existing and untouched.

**Rulings (QSL-366 comment by Peter Krenesky, 2026-10-01, read as data).**
RU-1 explicit-state first, RU-2 unmarked `whole` with `each` written on the
§6 variant, RU-3 deadlocks reported by default with a per-model opt-out, and
RU-4 mixed formulas with the ADR-014 TR-3 amendment, the cost (IV-5) and the
limit (IV-6) are all reflected. No "open question" wording is left in §1 to
§11. Two leftovers: the Status line (FND-011) and the STD-131 item the
ruling asks for (FND-010). The ruling names "the FR-341 rule"; QSpec
FR-341 (infinite-trace) has no interval rule, and the rule sits in QSpec
FR-090, FR-250-AC-6 and FR-255, which is where QS-13 points. The ADR is
right and the ruling text misnames the FR.

**Soundness checked and found correct.** The §6 worked example (9 model
states, 15 product states, two accepting SCCs, `each` proves and `whole`
refutes with the 3-step `upd(a)` lasso, shipped unit refuted under `each`,
bounded variant with a 5-step prefix). The weak-fairness SCC filter FA-4,
including the stutter edge at terminal states and model-level (not
product-level) enabledness. Machine closure of weak fairness. DL-1 to DL-7
against SM-4 terminal stutter, and DL-6's no-change rule. The
`max_automaton_states` limit settling V-7 (QSpec FR-341 `failed` with
execution `resource-incomplete`, ADR-014 A-5). The §11 example (12 states,
`b + 2` in general). FR-125-AC-1 to AC-3 and FR-126-AC-1, AC-3, AC-4 values.

**Value test on CX-2 / FR-128 post-state digests: keep.** One transition
identity can have several post-states (FR-120-AC-1: `increment()` has three
successors under one identity), and FR-101-AC-5 replay picks the successor
whose state-key digest equals the recorded one. Without the digest, replay
cannot tell which successor the counterexample took. It is a functional
successor selector, not a tracking pin. Recording the whole post-state would
also work but is larger and adds nothing. The wording that frames it as a
staleness check is FND-008. The result-side echo of digests is FND-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | IV-2 and FR-125 say a position past the represented lasso "is the loop position it wraps to". That holds for atom reads and future operators. It is wrong for any subformula with a past operator: the truth of `once[1,1] p` at position `n+L` differs from its truth at loop position `n` (history differs). Example: loop `(0,0) (1,0) (2,0)`; `always eventually (va = 0 and once[1,1] va = 2)` is true (position 3), but wrapping position 3 to position 0, where `once` reaches before position 0, evaluates it false. SM-1 makes this evaluator the semantics every engine and replay answers to, so the rule gives wrong results. State that wrapping applies to state reads, and that a subformula with past reach `r` is periodic only after the prefix plus `ceil(r / L)` loop iterations, which the evaluator unrolls. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:469; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:83-87 |
| FND-002 | medium | V-5 means "no counterexample of length at most `k`" for every engine. FR-126 returns `BoundReached` when `max_depth` is reached in the first phase and never says the second phase runs over the partial retained graph. For a TP-4 item a short lasso can exist inside depth `k` and would be missed, so V-5's claim is unproved. State that, at `max_depth`, a TP-4 item runs the SCC phase over the retained graph and returns `Violated` when it finds a fair accepting cycle, and `BoundReached` only otherwise. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:141-143; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:220-223 |
| FND-003 | medium | The safety first phase is underspecified. EN-1 and FR-126 detect a TP-1/TP-3 violation by "reaching a rejecting monitor state", but SM-6 and FR-126 build a monitor only for TP-2. TP-1 and TP-3 get a nondeterministic generalized Büchi automaton of the negation, which has no rejecting state. Name the automaton used for TP-1/TP-3, such as a deterministic bad-prefix monitor for the safety fragment, and its violation condition. Also missing: how the TP-2 monitor resolves pending obligations at a terminal model state under the closed-boundary rule. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:108-133; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:187,194 |
| FND-004 | medium | The IV-5 state bound (`f = b + 1` for lower bound 0, product `c x f1 x ... x fn`) assumes a counter construction that merges subsumed obligations. IV-3 says the translation expands intervals into `X` chains and then builds a generalized Büchi automaton. A standard tableau over those chains keeps each pending `X^d` obligation separately, up to `2^(b+1)` states per operator. The bound is unproved for the translation as specified, and the exact counts that ACs depend on (FR-126-AC-1's 15 product states, FR-126-AC-5's count of 50, the §11 example's 12) depend on the construction. Specify the counter construction in IV-3, or state the general bound. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:470,472 |
| FND-005 | low | §3 tests "each non-trivial SCC that contains an accepting state". A generalized Büchi automaton needs a state of every acceptance set. FR-126 states it correctly, and so does ADR-019 SR-2(a). | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:198-201 |
| FND-006 | medium | FR-127 makes `model_check` own "the one map" from `ModelCheckOutcome` to `TerminalValue`, and FR-127-AC-1 tests all eight rows. But `ModelCheckOutcome` (FR-126) has only `Holds{basis: Exhaustive}` and `Undecided(UndecidedSuccessor or NoInitialState)`. It cannot express V-2, V-3 or `InductionNotClosed`, whose inputs come from the SMT backend through CG's map (DS-2). As written, AC-1's V-2, V-3 and `InductionNotClosed` rows have no QSL input to test. Either `Holds` carries any `ProofBasis` and `Undecided` any cause, or those rows belong to CG's map and leave AC-1. | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:22-26,74-85,104 |
| FND-007 | medium | FR-128-AC-2 replays "the same payload" against the `Counter` unit with `terminal when`. That unit has a different package identity (FR-124-AC-1, DL-1), and FR-128 refuses a `package_id` mismatch by FR-098's rule (FR-128-AC-4). Read as the same envelope, the expected `inconclusive`, `Verdicts` contradicts that refusal. State that the payload goes in an envelope carrying the `When` package's identities. | spec/functional/FR-128-replay-a-model-counterexample.md:132 |
| FND-008 | low | CX-3 and FR-128 refuse "when a recomputed post-state digest differs". With several successors per transition identity there is no single recomputed post-state. The digest selects among them (FR-101-AC-5), and the refusal is "no successor of that identity has the recorded digest" (FR-101 `KeyMismatch`). State the digest's selector role and use that refusal wording. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:259-260; spec/functional/FR-128-replay-a-model-counterexample.md:99-101 |
| FND-009 | low | Value test on FR-128 Outputs: the result "retains ... the initial state's document identity and digest, each replayed post-state digest". These echo the envelope and the byte provision, and no requirement reads them from the result. They fail the value test, so delete them. The envelope's own digests stay (FND-008). | spec/functional/FR-128-replay-a-model-counterexample.md:73-77 |
| FND-010 | medium | Ruling 4 asks for the QSpec interval change to be "an added item on STD-131", and References says "Its QSpec half, QS-1 to QS-13, is Linear STD-131". Measured on 2026-10-01: STD-131's description lists 11 items and has no comments. It has no deadlock item (QS-12), no interval item (QS-13), and no QS-10 (f) or (g) vectors. Add them to STD-131. Until then the References claim is untrue. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:576-577 |
| FND-011 | low | Status still reads "Draft for plan-lead review of the key decisions" after the owner ruled on every open question. This is leftover pending wording. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:57 |
| FND-012 | low | RU-3's Ruling cell adds "A model marks its intended terminal states". The owner ruled a per-model opt-out for a model that halts on purpose. `terminal when P` is the author's design, argued in Alternatives. Move the sentence to Rationale or "Where it lands" so the ruling column holds only the ruling. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:449 |
| FND-013 | low | §1 classes the depth `k` (FR-101 `max_depth`) as an ADR-014 B-5 value, but IV-6 classes `max_automaton_states`, in the same `ModelCheckLimits`, as a B-2 run limit. ADR-014's rule is that a limit is classed by the type that carries it. Pick one class for EN-1's limits. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:167-171,473 |
| FND-014 | low | Coverage gaps (error-path rule). These stated behaviours have no AC or TC: `Stopped` by `max_transitions` and by a clause meter (FR-126); a TP-2 `on each` item (FR-126); a replay that faults settling `failed` (FR-127); a `Violated` whose replay refuses settling `ReplayRefused` through a refusal other than a digest mismatch (FR-127). | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:141-145; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:88-89 |

## Verdict

Not mergeable yet. FND-001 is a wrong result in the semantics everything
answers to. FND-002 to FND-004, FND-006, FND-007 and FND-010 are real gaps.
The rest are wording. The checklist is otherwise clean:
- ids are well formed and sequential;
- US-015 carries the story and four examples;
- every FR has a use case and ACs;
- every AC traces to a TC in `tests.md`;
- ticket ids appear only in References;
- the index rows exist.

**Stacked branches (information, not findings against #562).** Fixing these
findings does not force a structural change in #563, #564 or #568:
- FND-004 (IV-5) changes the bound ADR-019 SR-5 and SR-9 cite.
- FND-002 (phase two at `max_depth`) carries over to ADR-021's reduced graph under its depth rule RV-5.
- FND-001 (past operators over a lasso) bears on ADR-020 RC-2, which recomputes history values along a replayed lasso.
- ADR-019 SR-2(a) already uses the wording FND-005 asks for.
