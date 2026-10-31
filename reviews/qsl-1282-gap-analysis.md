---
id: SR-1918002
title: "gap analysis of PR #1918 (QSL-1282): FR-101-AC-5 replay by ordinal"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1918 git diff origin/main...36dd3b39d; FR-101-AC-5, TC-454 step 7"
review_set: subset
---
# SR-1918002: gap analysis of PR #1918 (QSL-1282)

## Summary

Ticket: QSL-1282. The criteria-to-tests check was done by hand over the diff. It was not a full `quoin matrix` run. Plan completion: not assessed.

FR-101-AC-5 clauses and their tests:
- Replay by recomputed key, and `UnknownInitial`: the tampered-initial case in the round-trip test.
- Picking by transition identity and then by ordinal: `replay_follows_the_recorded_ordinal_among_same_identity_successors`, ordinal 1 against ordinal 0.
- Ordinal out of range gives `MissingTransition`: the `forged` ordinal-2 case.
- Unknown transition gives `MissingTransition`: the `bogus` case.
- Compares no recorded key: the hand-built traces carry arbitrary keys and still replay.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-101-AC-5's producer side is untested. The AC and FR-101's prose ("each `Step` records it") make the ordinal something the sampler writes. No test asserts the ordinal `sample` records. The only sampled shared-identity trace replays with either ordinal, as SR-1918001 FND-001 shows. Fix: assert `sampled.steps[0].ordinal == 1` in the TC-454 step 7 round-trip test. | qsl-eval/tests/it/finite_simulation.rs:1510-1541; spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md:262-265 |

## Verdict

One medium gap, the same defect as SR-1918001 FND-001. The other AC-5 clauses each have a test that can fail. TC-474's deleted `wrong_digest` case tested the retired comparison, so deleting it loses no required behaviour.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | af6aaeefd adds `assert_eq!(sampled.steps[0].ordinal, 1)` to the TC-454 step 7 round-trip test and rewords its doc and inline comments to describe the ordinal. The key-`"2"` successor is second in canonical key order, so 1 is the right expected value. |
