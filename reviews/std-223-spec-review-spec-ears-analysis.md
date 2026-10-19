---
id: SR-13349
title: "EARS review of STD-223 FR-188 and FR-189 total refusal statements (PR #1063)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@0f16ce205ed71006cb003e607e5b0459b390c4cc; spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md, spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
---
# SR-13349: EARS review of STD-223 (PR #1063)

## Summary

Ticket: STD-223. EARS conformance of the three requirement statements the
diff edits: FR-188's `weighted_choice` refusal, FR-188's
`sample_probabilistic` stop, and FR-189's `TotalAboveDrawRange` mapping. The
repository grammar linter (`make check-spec-validation`) reports nothing on
either file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-188 stop statement now ends with a second, agentless obligation in a relative clause: "…SHALL stop with `SampleStop::TotalAboveDrawRange`, which settles as QSpec's `unsupported` result with the kebab-case cause…". The settlement is done by FR-189 and FR-192, not by `sample_probabilistic`, and "settles" names no subject. Drop the clause, or state it as a reference ("FR-189 maps this stop") rather than as part of this SHALL. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:90-93 |

## Verdict

One low finding. Examined and clean. The FR-188 `weighted_choice` refusal is
an unwanted-behaviour statement with a named agent and outcome; its
"as QSpec FR-181-AC-14 states" rationale is behaviour, not a changelog.
FR-189's mapping adds a "whose wire cause is" clause. That clause describes
the returned value; it adds no second obligation. The bullet's existing
pair of SHALLs and its stop-as-subject phrasing predate this diff and are out
of scope.
