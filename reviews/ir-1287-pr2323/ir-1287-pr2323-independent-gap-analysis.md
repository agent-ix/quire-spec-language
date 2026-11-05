---
id: SR-20261104990002
title: QSL PR 2323 independent gap-analysis (IR-1287)
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; IR-1287; PR 2323; FR-492-AC-8..13, FR-314
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-492
  type: references
- target: ix://agent-ix/quire-spec-language/FR-314
  type: references
---

## Summary

All six added criteria (FR-492-AC-8..13) have value-asserting, TC-1275-tagged tests that fail under the mutations tried. One gap: the exact IR-1120 situation (well-formed Alethe proof with an unsupported rule) is not asserted through the new public path.

## Verdict

PASS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No facade test asserts that check_smt_proof returns SmtProofCheck::Unverifiable (never Verified) for a proof whose first unchecked rule is unsupported (the live cvc5 `symm` case). FR-314 text added by this PR requires unknown-rule bytes to reach the checker unchanged; AC-10 tests only empty (NotRefutation) and 0xff (Malformed). A mutation mapping Unverifiable to Verified inside the delegation path would be caught only by the pre-existing lib checker tests, not by the facade binder. Add a facade test with a checked-valid base proof followed by an unsupported rule, asserting Unverifiable with the step index. | FR-492-AC-10, FR-314 |

## Coverage

Plan completion: not assessed. Spec binders: AC-8 to AC-13 each covered by tagged tests in qsl-replay/tests/it/refinement_query_facade.rs and the private handoff unit tests. No solver-result mapping exists in this PR (bytes assembly plus delegation to the FR-314 checker), so no sat/unsat/unknown/timeout path can become proved here.
