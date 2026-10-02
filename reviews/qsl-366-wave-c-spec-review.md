---
id: SR-1120
title: "QSL-366 Wave C spec review of ADR-018 and FR-123 to FR-128 (#562)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e6a5fb5690e54e057e39cfd2d1a72744355d6473; git diff origin/main...HEAD over spec/: ADR-011, ADR-013, ADR-014, ADR-016 amendments; ADR-018; FR-072, FR-101; FR-123 to FR-128; US-015; TC-453 to TC-455, TC-518 to TC-523, TC-536 to TC-539; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
---

## Summary

Ticket: QSL-366. PR quire-spec-language#562, the base of a stack of 10.
One review file for the PR, covering spec-review base, EARS, integrity and
scope-boundary checks over the PR diff only.

Checked against the eight owner rulings recorded on QSL-366: explicit-state
first (RU-1); unmarked fairness is weak whole (FA-6); deadlocks reported by
default with the `terminal any` opt-out (DL-1 to DL-7); mixed formulas
admitted (IV-1 to IV-7); Büchi cost and the interval limit stated (IV-5,
IV-6); undefined evaluation refutes with `UndefinedEvaluation{where, cause}`
(UE-1 to UE-6); default limits 1e7 states and 1e8 transitions (FR-101,
FR-126); `max_depth` is the search horizon `k` (§1 "Depth is a method
parameter", FR-126). ADR-018 and FR-123 to FR-128 follow every ruling.

`quire validate` on every changed spec file passes (one pre-existing
`ac:vague-response` warning on FR-101-AC-14); the index check passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-101 still makes depth a limit kind. The PR's new Inputs text calls `max_depth` the search horizon, but the same paragraph says "A run that reaches a limit names it and its value in `Outcome::Bounded`", and FR-101-AC-7 still has a run that reaches `max_depth` return `Outcome::Bounded` "with that `Limit`", category incomplete. ADR-018 §3 keeps `explore::Outcome::category()` on that map. This contradicts the QSL-366 ruling that `max_depth` is the horizon `k`, not a limit kind. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:63-71,333; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:246-247 |
| FND-002 | medium | The base PR cites artifacts that exist only in later PRs of the stack. FR-126 adds `CheckSubject::Protocol`, `max_live_instances`, `max_store_buffer` and `max_messages`, citing FR-205, FR-209, FR-215 and FR-225, and FR-126-AC-8 and TC-536 test those members. ADR-018 adds TP-5, `MappingUndetermined` (V-6), `RefinementFailure` (CX-2) and the IV-7 transfer rule, citing ADR-020 RE-1 to RE-4, RC-1, CO-3 and CO-4. None of FR-205, FR-209, FR-215, FR-225 or ADR-020 exists in this tree or on main. Each amendment belongs in the PR that adds the record it cites. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:63,74-76,93-97,299,319; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:126,151,161,281,476 |
| FND-003 | medium | The FR-101 sampler change belongs to another lane. It adds `choice` to the sampler preimage and replaces the FR-101-AC-3 vectors (step-0 digest `d5160380…`). It cites "QSpec TC-210's vector", but QSpec main's TC-210 still carries `cb7d4b3b…` with no `choice` member. The new vector exists only on QSpec branch `spec/wave-b-q4-prob-time` (probabilistic, STD-137). This is not QSL-366 temporal work, and it moves TC-454 from implemented to 🚧. Dropping the `1-draft.1` revision pin is the QSL-395 ruling and is correct; the `choice` member is not. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:246-268,329; spec/test-cases/TC-454-the-pinned-sampler-reproduces-its-vectors-and-sampled-traces-replay.md |
| FND-004 | medium | FR-124 classifies a terminal state as intended when `P` evaluates `true` and "as **deadlocked** otherwise". So a `terminal when P` that evaluates undefined classifies the state as deadlocked. ADR-018 UE-1 and FR-126-AC-9 settle that case as a counterexample of kind `UndefinedEvaluation{where: 3, cause: division-by-zero}`, not `Deadlock`. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:89-91 |
| FND-005 | medium | FR-125 defines a claim's letter as "the value of every atom of the claim at that position". It omits UE-1's third part, DL-1's `P` at a terminal state for the deadlock-freedom item. FR-128 replays an `UndefinedEvaluation` counterexample by evaluating "the letters of the replayed positions … by FR-125", and its `Deadlock` branch handles only `P` false or absent. So the FR-126-AC-9 deadlock-freedom counterexample (`terminal when 6 / (3 - c.value) = 0`) has no replay rule that reproduces it, and no FR-128 AC covers it. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:138-140; spec/functional/FR-128-replay-a-model-counterexample.md:140-155 |
| FND-006 | low | FR-126 pairs each initial model state with "the automaton's successor from its initial state on position 0", in the singular. The TP-4 automaton is a nondeterministic generalized Büchi automaton with a set of successors, and ADR-018 §6 and FR-126-AC-1 need `(0, 0, q1)` as an initial product state beside `(0, 0, q0)`. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:174-175 |
| FND-007 | low | The rewritten FR-101 sampler paragraph keeps "STD-109 settles it upstream" in requirement prose. Ticket ids belong only in References. TC-454's title and index row still say "pinned sampler" after the pin was removed. | spec/functional/FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md:266; spec/tests.md:233 |

## Verdict

Not mergeable as it stands. ADR-018 and FR-123 to FR-128 are consistent
with every recorded ruling. EARS phrasing holds; every AC has a behaviour TC
with concrete values; there are no new pins, ledgers, caps or compat paths;
and QSpec semantics are cited (FR-125's restatement is marked informative).

The worked examples check out by hand:
- §6's 15-state product and both fairness verdicts;
- the SM-8 lasso example;
- FR-126-AC-5's `max_depth` 2/3 boundary;
- FR-130's inputs to the stack.

To fix:
- FND-001: FR-101 still treats depth as a limit kind, against a named ruling.
- FND-002: forward references to records in later PRs of the stack.
- FND-003: an off-lane sampler change.
- FND-004 and FND-005: leave an undefined `terminal when` predicate without one consistent classification and replay path.

## Dispositions

Round 1, reviewed at 18a9c22bd1783e2d40eaf3528e662f943183e2b8.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 18a9c22b |
| FND-002 | fixed | 18a9c22b |
| FND-003 | accepted-no-change | The team leader kept the `choice` member to align FR-101 with the FR-181 preimage of QSpec #175 (agent-ix/quire-specification, open draft, STD-137/139). The vector is real on #175 and I recomputed the step-0 digest `d5160380…`. FR-101-AC-3 now says it is TC-210 as amended by QSpec FR-181. It agrees with QSpec main only if #175 merges before #562. |
| FND-004 | fixed | 18a9c22b |
| FND-005 | fixed | 18a9c22b |
| FND-006 | fixed | 18a9c22b |
| FND-007 | fixed | 18a9c22b |
