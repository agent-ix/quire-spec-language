---
id: SR-16205
title: "spec-review of FR-448 backend-agreement gate and the O-16 agreement note (quire-spec-language PR 1298)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a03f3697; spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md; spec/test-cases/TC-1193-the-backend-agreement-gate-fails-on-conflicting-backend-verdicts.md; spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-448
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1193
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: references
---

# Spec review of FR-448 backend-agreement gate and the O-16 agreement note (quire-spec-language PR 1298)

## Summary

Ticket: STD-230. This is one spec-review pass over
`git diff origin/main...a03f3697`: the new FR-448, a new "Agreement across
backends" paragraph in ADR-013 O-16, TC-1193, and an index entry in
spec.md. The oracles were:

- ADR-013 O-16 (category table, the ADR-018 §1 and ADR-028 amendments),
  O-20, O-21, O-24, C-09 and T-13;
- ADR-018 V-2 and V-5 (the verdict table) and SM-7;
- ADR-012 §7.1 and §7.2 (parties, candidates, negotiation);
- FR-075 (candidate sets and the request builder);
- FR-350, FR-351 and FR-353 (scope, preflight, exit contract);
- FR-295 and TC-779, as the execution-backend analogue, and FR-402.

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation`
exits 0, with no finding on FR-448, TC-1193, ADR-013 or spec.md.
origin/main (5ba013c7) is an ancestor of the head. FR-448 and TC-1193
appear in no other open PR, and no other open PR edits ADR-013. Four open
PRs (#683, #686, #703, #995) edit spec.md in other places.

Examined, clean:

- **Which records are definite matches O-16.** `proved` means any basis in
  the success category, so a vacuous `Checks{0}` proof is excluded.
  `refuted` means a counterexample whose replay settles
  `ReproducedWithEvaluatedWitness`. `tested`, `measured` (ADR-024),
  `replay_parity`, `replay_refused` and `failed` are not definite. This
  agrees with C-09 and SM-7.
- **A reproduced counterexample conflicts with an unbounded proof at any
  bound.** This is sound: a replayed witness is a real violation.
- **No preference, certification or majority rule settles a conflict.**
  This is the right rule for a cross-check, because `Trusted` and
  `Uncertified` proofs are still compared.
- **Exit codes 0/1/2 match FR-353's contract**, and the preflight reuses
  FR-353's argument, corpus and scope refusals. The report is ordered by
  vector id and then request index (AC-6), which is deterministic.
- **TC-1193's deliberately wrong backend (step 2) mirrors TC-779**, and
  AC-1, AC-2, AC-5 and AC-6 are testable as written.
- **Both lead defaults and both open owner questions are flagged** in the
  PR body. Owner question 1 (how CG's Kani backend reaches the registry
  when QSL never builds CG) is a real dependency. Until it is answered, the
  gate can compare only QSL-registered backends.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The bounded-proof rule treats `BoundedComplete{depth}` as a proof that "says nothing beyond its bound", which contradicts ADR-018 V-2. V-2 issues `Proved{basis: BoundedComplete{depth: k}}` only when the unrolling reaches the formula's horizon, "so every behaviour is decided". A proof that covers only depth k is V-5, `Inconclusive(BoundReached{depth: k})`, which is already not definite. So when a reproduced counterexample (at any bound) meets a `BoundedComplete` proof, one backend is wrong. Under this PR the gate reports `not-compared` and exits 0, which is exactly the wrong verdict the gate exists to catch (AC-3's first clause, TC-1193 step 3). Fix: compare a `BoundedComplete` `proved` record like any other `proved` record. Drop the basis half of "a record is bounded". Turn AC-3's depth-3-against-bound-5 case into `conflict`, exit 1. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:537-544; spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md:104-106, :139; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:147, :150 | wrong-requirement |
| FND-002 | high | The bounded-mode half of the rule compares quantities that cannot be compared, and AC-3 rests on a bound that no record carries. For a `bounded`-mode run the note takes "the item's result bound" as the bound. FR-075 defines that as the `WireNodeId` of the record's FR-092 type node, not a depth, and every candidate of one item shares it. "Same bound" is therefore never false between two bounded-mode records of one item, and it is ill-typed against a `BoundedComplete` depth. A `refuted` `TerminalValue` records no bound, so "a refutation produced at bound 5" has no field to read. O-20 says a bounded-mode `proved` holds only over the declared finite domain, and that domain is part of the obligation identity (O-09). So the sound rule is not equal bounds. A reproduced counterexample conflicts with a bounded-mode proof when its witness lies inside that proof's declared domain; otherwise the pair is `not-compared`, naming the domain. Fix: replace the bound comparison with that domain rule, and restate AC-3 in terms of declared domains and recorded witnesses. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:537-544; spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md:139; spec/functional/FR-075-compute-candidates-from-registered-backends.md:143; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:882 | wrong-requirement |
| FND-003 | medium | The gate runs every candidate without negotiation. FR-075 computes candidates without looking at mode: the advertised mode is compared against the item's extent later, by CG's `negotiate_*`, which settles a disposition per item (ADR-012 §7.2 step 3). FR-448 goes straight from the candidate set to "run every candidate". It does not say what happens to a candidate that would negotiate `requires-bound`, `unsupported` or `invalid-request`. Two implementers would differ: one runs it, the other skips it. Fix: for each candidate, build the request naming that `BackendId` (FR-075's named path gives a one-backend candidate set) and negotiate it. Run only candidates settled `supported`. Record every other disposition as a non-definite record of its O-16 category. State whether the "two or more candidates" in-scope test and `no-comparable-item` count candidates before or after negotiation. | spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md:75-82, :117-119; spec/decisions/ADR-012-semantic-family-extension-contracts.md:696-740 | missing-requirement |
| FND-004 | medium | Who settles a counterexample is unclear next to C-09 and T-13. FR-448 has the gate replay "every counterexample a backend returns" and settle it under O-16. But O-24 and C-09 make the map from a Kani or model outcome plus its E9 replay into `TerminalValue` CG-owned, and the orchestrating driver (T-13) runs the replay and passes both results. A provider that follows that contract hands QSL a terminal record that is already settled. The spec does not say whether the gate receives raw outcomes (then which C-09 implementation does it call?) or terminal records (then a second replay checks the provider, and its disagreement needs a rule). Fix: state what a backend returns to the gate, and make the gate's replay either the settlement or an explicit re-check with its own outcome. This ties to owner question 1. | spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md:84-92; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:943, :1088 | missing-requirement |
| FND-005 | medium | No rule covers a vector whose compile does not return a checked package. Scope compiles every `language` vector in scope, and the corpus holds refusal, just-outside and incomplete classes whose expected result is a compile refusal or limit. FR-448 does not say whether such a vector yields no item, is reported, or refuses (exit 2), nor how a compile panic or internal fault settles. "Refused before any backend runs" also has no rule for a compile fault during that phase. Fix: items come only from vectors whose `compile` returns a checked package. A compile refusal or limit yields no item and is listed in the report. A compile fault or panic is a tool failure with an exit status the spec states. | spec/functional/FR-448-cross-check-verification-backends-over-the-conformance-corpus.md:75-82, :121-131 | missing-requirement |
| FND-006 | low | TC-1193's title and Description say it "catches a deliberately wrong backend", but the step 4 wrong backend (a counterexample the evaluator does not reproduce) exits 0, because lead default 2 reports `replay_parity` and does not fail. Only the step 2 backend is caught. Fix: narrow the title to the step 2 backend, or change step 4's expected exit if the owner rules that `replay_parity` fails the gate (owner question 2). | spec/test-cases/TC-1193-the-backend-agreement-gate-fails-on-conflicting-backend-verdicts.md:3, :34-36, :54-56 | wrong-requirement |

## Verdict

Not mergeable. FND-001 and FND-002 make the agreement relation unsound in
the direction that matters: it misses real conflicts. ADR-018 V-2's
`BoundedComplete` is a complete proof, and the bounded-mode comparison
reads a type node and a bound that no record carries. Replace both with
"every success-category `proved` is compared, and a bounded-mode proof is
compared over its declared domain". The rest of the relation is sound: only
`proved` against replay-reproduced `refuted`, an unbounded proof against a
counterexample at any bound, and no tie-break. FND-003 to FND-005 are
unspecified paths (negotiation, C-09 ownership, compile failures). Fix
FND-001 to FND-005 in this PR. FND-006 follows owner question 2.
