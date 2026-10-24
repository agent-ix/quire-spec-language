---
id: SR-16619
title: 'Gap analysis of PR #1424: production ModelLetters reader (FR-125-AC-11, AC-12)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@c51644c15; git diff origin/main...c51644c15 (PR #1424, branch qsl981-modelletters-reader): qsl-eval/src/simulation/model/letters.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/tests/it/model_instances.rs; criteria FR-125-AC-11, FR-125-AC-12; ticket QSL-981'
review_set: subset
date: '2026-10-24'
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-125
---

## Summary

Ticket: QSL-981. PR: quire-spec-language#1424, head c51644c15.

This is a planless gap analysis of the criteria the ticket names, FR-125-AC-11 and FR-125-AC-12. It checks them against the computed matrix (`quire matrix --scope . --format json`, quire 0.36.2), the tagged tests and the new source. Before this PR, neither criterion had a binder on main.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-125-AC-12 is bound by a test that asserts only part of it, and the computed matrix now reports the criterion `tagged`. The test asserts the instances `k` and `t`, the read of `k` as `Dog`, the unguarded `undefined-evaluation` refutation naming `t`, and the Cat-only subject not being `NoInstance`. It does not assert the `dogs` population, the instance `d` or the order `k, t, d`. Nor does it assert that the guarded `always holds(G implies x.age >= 0)` settles `proved`, or that the partial guard is false at `d`. The test's own doc says these are not covered. Fix: drop `FR-125-AC-12` from the `#[trace]` (keep `TC-520`) and leave the criterion untagged for QSL-1038, or split the criterion so a tag binds only what is asserted. | qsl-analyze/tests/it/model_instances.rs:724 |

## Verdict

CONDITIONAL. The one finding is medium, but it is a false coverage claim in the matrix, so fix it in this PR: dropping the tag is a one-line change.

FR-125-AC-11 is fully asserted:

- For a `Switch`, a `Lamp` and the population's first member type `Lamp`, both `x = r` (as `exists(o in allInstances<Thing>(PM) : o = x)`) and `contains(allInstances<Thing>(PM), x)` read true.
- `check_model` holds end to end through the production letters.
- The P1/P2 key `c`, held only by P2, reads `contains(allInstances<Thing>(P2), x)` true.
- A mutant that binds within the instance's own population fails this test.

QSL-1038 also mentions an AC-11 half that needs two covering populations. The test's reading of "P1 and P2 of Thing" as two populations that overlap `Thing` matches the FR-125 term "overlaps", so no AC-11 finding is raised.

## Coverage

- FR-125-AC-11: tagged (qsl-analyze/tests/it/model_instances.rs:675). Fully asserted.
- FR-125-AC-12: tagged (qsl-analyze/tests/it/model_instances.rs:726). Partially asserted (FND-001).
- FR-125-AC-9: untagged both before and after this PR. It is out of this ticket's scope, and the test-file header records it as not covered.
- Reverse gap: none. `ClauseLetters` and `clause_letters` are owned by FR-125 Instancing and FR-107. They have no caller outside tests, as `check_model` has none (QSL-981 states this).
- Stubs: none. `activation` returns `True` only for the unguarded clauses `clause_letters` admits, and `activation_reads` returns the empty set for the same clauses, which is the modelled answer.
- Semantic review: done inline, without fan-out, through the red checks recorded in SR-16618.
- Plan completion: not assessed
