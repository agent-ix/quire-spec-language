---
id: SR-13084
title: "Integrity review of PR #1030: ADR-018 §6 frame and FR-126 rows"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@ffc3b9d9f; git diff origin/main...HEAD: ADR-018 §6, FR-126-AC-1/AC-2, TC-521"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
---

## Summary

Ticket: STD-204. Consistency of the changed §6 text with FR-120 (unchanged), FR-154, FR-126 and TC-521, and of the ADR with the AC rows. ADR, AC-1 and TC-521 agree on the receiver-scoped frame for both units; AC ids are unchanged. The `mod 3` type-check rationale was confirmed by probe (IllTyped/TypeMismatch). Out of scope, for the planner: FR-156-AC-3's `Ring` unit spells `self.v = (pre(self.v) + 1) mod 3` against `v: Int[0, 2]`, the form this PR's ADR text (and the probe) say does not type-check; that AC is not in this diff.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-126-AC-1 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-2 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| ADR-018 §6 Frame | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 post clause | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 Subject | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 Verdict under fair weak each | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 A bounded variant | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| TC-521 Test Procedure fixtures | spec/test-cases/TC-521-explicit-state-model-check.md | examined |
| FR-120 candidate count | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| FR-154 | spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "one expansion would then enumerate 1001 x 1001 candidates, and the run would stop at `max_candidates`" misstates FR-120's count and states a setting-dependent stop as fact. FR-120 counts every candidate and evaluated (candidate, result) pair over all applications of an expansion, so the shipped frame charges 2 x 1001^2 candidates plus pairs per expansion (two receivers), and the run stops only when `max_candidates` is below that: `ExpansionLimits` is caller-supplied with no published default, and V-7 says the request raises it. Also, the limit is FR-120's (FR-126 only reports it). Say "each application enumerates 1001 x 1001 candidates, so the run stops at any `max_candidates` below about two million (FR-120)". | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:365-369; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:432-444 |
| FND-002 | low | "a state has exactly one successor per receiver and result choice" contradicts FR-120 and §6's own Subject paragraph: a candidate pairs with result values in order and the first accepted pair fixes the transition's result, so there is one successor per receiver, not one per (receiver, result). Drop "and result choice". | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:370-373,404-406; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:426-430 |
| FND-003 | low | `modifies_receiver [versionNumber]` is neither of FR-154's spellings: the frame is written `modifies self.versionNumber`, and `modifies_receiver` is the model-document member holding field node identities. ADR §6, FR-126-AC-1 and TC-521 use the hybrid. Write `modifies self.versionNumber` (the `modifies_receiver` member, FR-154). | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:370; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:352; spec/test-cases/TC-521-explicit-state-model-check.md:23; spec/functional/FR-154-scope-a-modifies-entry-to-the-receiver.md |

## Verdict

Findings: FND-001 (low), FND-002 (low), FND-003 (low). The engine-measured rows (15/30 product states, the no-granularity lasso shape, the shipped-unit `each` loop) match the spec; FR-120 is unchanged and the receiver-scoped frame is FR-154's; AC ids are kept and the text reads as behaviour.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The Subject paragraph says the reported binding "is the ranking of `over` bindings (QSpec's shared grammar and FR-126 "Counterexamples")", but FR-126 "Counterexamples" ranks no `over` bindings: it orders the BFS to the first passing SCC and the CX-5 loop walk only. That missing rule is STD-203 item 1, still open. Cite the ranking as not yet specified (or name only QSpec), so the pointer does not claim a rule FR-126 lacks. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:412-417; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:298-330 |

## Dispositions

Round 1, against head 439d37769 (fix commit 6cdb4afe3).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | With two objects over `Int[0, 1000]` each application would then enumerate 1001 x 1001 candidates, so an expansion charges about two million candidates against the subject's `max_candidates` (FR-120). The example, and the shipped unit as this section uses it, frame `attemptUpdate` as `modifies self.versionNumber` (FR-154's receiver scope) instead: an update writes the receiver's `versionNumber` an |
| FND-002 | fixed | ... so a state has one successor per receiver. |
| FND-003 | fixed | ADR-018 §6: "frame `attemptUpdate` as `modifies self.versionNumber` (FR-154's receiver scope)"; FR-126-AC-1 and TC-521: "framed `modifies self.versionNumber` (FR-154)". |
