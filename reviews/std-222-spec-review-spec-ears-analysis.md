---
id: SR-13343
title: "EARS review of STD-222 FR-188 sampler statements (PR #1061)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@4f66d6d1e7ecc32ac0a376c3f76fc93dacd37706; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
---
# SR-13343: EARS review of STD-222 (PR #1061)

## Summary

Ticket: STD-222. EARS conformance of the statements the diff adds or edits in
FR-188: the Description sentence on one-item choices, the selection bullet (the
one-item clause and the above-2^256 refusal), the sampler-identity bullet, the
choice-index bullet, the step-order bullet and the crate-home bullet. The
repository grammar linter (`make check-spec-validation`) reports nothing on
FR-188.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The above-2^256 refusal is an unwanted-behaviour obligation written as an indicative with two SHALLs and two agents (`weighted_choice SHALL refuse … and sample_probabilistic SHALL stop`). Split it into two "If the scaled total `N` exceeds 2^256, then … SHALL …" statements. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:78-81 |
| FND-002 | low | The one-item rule is stated twice as separate SHALLs: "A choice over one item SHALL select index 0 with no draw" in the selection bullet, and again in the choice-index bullet. Keep one. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:77,87-91 |

## Verdict

Two low findings. Examined and clean for EARS form: the sampler-identity
bullet (an unwanted-behaviour refusal with a named agent and outcome), the
choice-index bullet, the step-order bullet and the crate-home bullet (a
ubiquitous statement with a named subject; its content is SR-13342 FND-008).
