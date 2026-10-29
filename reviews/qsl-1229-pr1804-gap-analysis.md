---
id: SR-1804002
title: "gap analysis of PR #1804 (QSL-1229): fixture digest purge"
type: SpecReview
analysis: gap-analysis
date: 2026-10-29
scope: "agent-ix/quire-spec-language; PR #1804 at d4a0e4f28: tests rebuilt at test time (TC-090, TC-101, TC-885 and the spine-model consumers)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1804002: gap analysis of PR #1804 (QSL-1229)

## Summary

Question: do the tests now built at test time still test real behaviour?

- **TC-090 (`fixed_minimal_control_and_multiple_clause_vectors_match_the_public_producer`).** It still compares the producer's package bytes, digest and canonical identity with `vectors::expected_inner`. That recipe composes the JSON from literals and hashes it with `sha2` under the explicit domain prefix; nothing is read from the producer. The deleted assertion compared the recipe with frozen files (a pin), and nothing behavioural was lost.
- **TC-101.** It still compares the produced artifact with the historical one. The historical source digest is now derived with `Sha256` from `model-source.json`, so the producer's source-digest computation is still checked independently.
- **Spine-model consumers.** These test compilation, lifecycle and selection behaviour, not the digest. The two `unit_selecting` tests still splice a different digest and assert its refusal.
- **The independent check of `sha256-jcs` document digests** stays in `identity_golden_vectors.rs`, which recomputes with `sha2` directly.
- **TC-885.** Its fixture list still covers every legacy native-package source, composed over the model.

No test went vacuous. Mutating a `PLACEHOLDER` to a non-zero digest leaves the qsl-analyze `counter` tests green, which confirms those members are never compared.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.
