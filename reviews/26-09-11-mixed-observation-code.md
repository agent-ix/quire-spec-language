---
id: SR-369
title: "Code review of mixed received and own-attempt observation tests"
type: SpecReview
analysis: code-review
scope: "tests/native_mixed_observation_choices.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Actual Claude Sonnet code/Rust review inspected the new test file, repository
instructions, the actual code-review and rust-review skills, shared test setup,
wire records, decision implementation, FR-042 and TC-121. Its result is retained
at `/tmp/quire-mixed-observation-review.jsonl` (session
`9125689e-4c40-44d7-a33a-e87e4b972a32`). The reviewer ran no gates;
execution results below came from the separate parent gate run.

Both tests exercise actual parse, binding, type admission and proof discharge
before native family admission. The positive combines one received record and
one same-owner attempt record, checks their distinct source/binder/anchor and
model/operation owners, and emits bytes accepted by the independent reader.
Five negative cases isolate foreign ownership, missing/composite visible facts,
and a partition failure without substituting wire-only fixtures or stubs.

## Verdict

**CONDITIONAL** — the actual reviewer reported only low findings and no
correctness blocker. The trace overclaim is corrected; optional assertion
breadth remains outside this test-only increment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Resolved: removed FR-042-AC-8 from the refusal table. It establishes the AC-5 family-proof refusal, not an unsupported projection coexisting with a separately admitted valid declaration. | tests/native_mixed_observation_choices.rs; FR-042-AC-5; FR-042-AC-8 |
| FND-002 | low | Optional breadth: Branch/Join assertions establish expected edges but do not independently count duplicates, matching existing sibling test conventions. No production change or gate weakening is introduced. | tests/native_mixed_observation_choices.rs |
| FND-003 | low | Optional breadth: the test does not independently assert the attempt runtime-instance binding ordinal. The review also referred to a receive instance ordinal; the wire Receive variant instead carries channel/send handles, which this test checks. | tests/native_mixed_observation_choices.rs; src/protocol_artifact/wire.rs |

## Validation

Parent gates completed with 2 focused tests passed, 0 failed; formatting and
both strict Clippy configurations passed. The output is retained at
`/tmp/quire-mixed-observation-gates.log`. The subsequent source change removes
only the incorrect trace tag; its direct rustfmt check passed. Quire validated
both review artifacts: 2/2 grammar-clean, zero grammar findings, exit 0. Its
loader also emitted duplicate-archetype and duplicate-inverse-edge diagnostics;
grammar success does not resolve those configuration diagnostics. SR-370
retains the inherited corpus matrix gaps; neither review claims
full FR-042 acceptance or B runtime conformance.
