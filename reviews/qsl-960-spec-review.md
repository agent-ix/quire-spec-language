---
id: SR-12309
title: "Spec review of quire-spec-language PR #1197: FR-399 and TC-1093 (QSL-960)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@ae46f1869a9416e06a7ae44519c7e306a902b4c3; spec/functional/FR-399-set-a-named-stage-limit-through-the-replay-root.md; spec/test-cases/TC-1093-a-consumer-sets-a-named-stage-limit-through-the-replay-root.md; spec/spec.md (index row and contains edge)"
review_set: subset
---
# Spec review of quire-spec-language PR #1197

## Summary

Ticket: QSL-960. Covers FR-399 (Description, Behavior, AC-1..AC-4), TC-1093
and the spec.md index row and `contains` edge.

- `quire validate` passes on both files.
- The relationships resolve: FR-394, FR-255, FR-263, FR-060 and ADR-011.
- Behavior plainly states the pairing between `s1.input_bytes` and
  `ReplayLimits.input_bytes`, and that the request decoder and `with_named`
  share one check. The code matches both statements.

## Verdict

Mergeable. There are no high findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-399 says a source larger than 1 MiB is blocked by `s1.input_bytes` alone ("cannot raise the input allowance"). Measured: a source of real declarations reaches the `s1.tokens` default (100000) at about 267 KB and the `s1.nodes` default (50000) at about 300 KB, long before 1 MiB. The FR should say a large real source also needs `s1.tokens` and `s1.nodes` raised by name. TC-1093 step 1 fixes the comment-padded fixture in place, so it can never show this. | spec/functional/FR-399-set-a-named-stage-limit-through-the-replay-root.md:22-31, 55-58 |
| FND-002 | low | FR-399-AC-2 makes two claims in one criterion: the source compiles with both limits raised, and it is refused as AboveReader with the default reader limit. The two could be separate ACs. | spec/functional/FR-399-set-a-named-stage-limit-through-the-replay-root.md:65 |

## Dispositions

| FND | Outcome | Sha |
| --- | --- | --- |
| FND-001 | fixed | 9ffbfbb16 |
| FND-002 | fixed | 9ffbfbb16 |
