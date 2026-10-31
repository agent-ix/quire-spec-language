---
id: SR-1918003
title: "spec review of PR #1918 (QSL-1282): FR-101 and TC-454 marker removal and AC-5 wording"
type: SpecReview
analysis: base
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1918 spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md, spec/test-cases/TC-454-the-sampler-reproduces-its-vectors-and-sampled-traces-replay.md"
review_set: subset
---
# SR-1918003: spec review of PR #1918 (QSL-1282)

## Summary

Ticket: QSL-1282. The diff removes three `BLOCKED-BY QSL-1282` markers. It adds "each `Step` records it" to FR-101's trace paragraph. It extends AC-5's `MissingTransition` clause to cover "a transition, or an ordinal, the current state does not offer".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The prose matches the code. Replay picks by transition identity and then by canonical-order ordinal (trace.rs:194-202). It compares no recorded key. It refuses an ordinal out of range with `MissingTransition`, and `Step` has an `ordinal` field. The TC-454 step 7 parenthetical still names the retired `KeyMismatch` correctly. Before this PR, FR-101 already said `Trace.initial` and `Step.key` are `StateKey`s, while the code holds `DigestRecord`s. That is outside this diff and is not raised here.
