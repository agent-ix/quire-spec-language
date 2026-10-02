---
id: SR-1070
title: "QSL-381 spec review of PR 577: ADR-030, FR-255 to FR-264 and the depth deletions"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9d240032be4eac1df486d3c224cbd48273053dc5; diff origin/main...9d240032; spec/decisions/ADR-030; spec/functional/FR-255..FR-264; amended FR-008, FR-018, FR-025, FR-040, FR-047, FR-056, FR-062, FR-068, FR-082, FR-083, FR-091, FR-092, FR-093, FR-096, FR-098, FR-101, FR-102, FR-106, FR-111; NFR-001, NFR-006, NFR-007, NFR-009, NFR-011, NFR-012; TC-720..TC-739 and amended TCs; US-027; spec.md; tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
---
## Summary

Ticket: QSL-381. Base spec review of the final head 9d240032 (includes the
FR-082 edge-count rewrite, the NFR-006/NFR-009 no-clamping change and the
16777216-edge family_steps/ancestor_steps default). Checked against owner
rulings RU-1..RU-3 (no depth limit kind, no depth knob, shared reader in
quire-canonical, defaults stay and the outcome names the setting) and QSpec
FR-460, FR-461, FR-322 AC-39..42 and FR-323 on spec/wave-b-q8-lifecycle-depth.
`quire validate` on the 79 changed .md files exits 0;
`tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-092-AC-7 still ends "a chain of 1,000 stops with `nesting-depth-exceeded`". That cause is deleted by FR-096 (seven LimitKinds) and ADR-030 D-1; TC-413 was already updated to "no outcome names a depth". Replace the last sentence: the 1,000-record chain checks (or stops on s3.nodes/s3.work_units naming its setting). | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:932 |
| FND-002 | high | A library import depth limit survives. FR-099 says the S4 source resolution compiles each imported library "within `DependencyLimits::depth`"; ADR-017:417 classifies "`Import` other than its depth limit"; FR-087:565 still lists `PackageLimits` depth although FR-111 deleted it. ADR-030 D-7 3a deletes the library resolution depth. Replace with an edge/definition count limit that has a FR-255 setting. | spec/functional/FR-099-compile-against-supplied-libraries.md:157; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:417; spec/functional/FR-087-typestate-and-cross-package-node-key.md:565 |
| FND-003 | medium | FR-255 says every configurable resource limit has a setting name and AC-3 says the union of names equals the table, but the table omits configurable limits this PR touches: `TypeEnvironmentLimits` ancestor_steps and work_units (FR-082, reported `stage_limit_exceeded`, so Behavior 1 requires a setting), `ancestor_steps`/`family_steps` (NFR-012, FR-083), the NFR-006/NFR-007/NFR-009 native limits, FR-101 exploration limits and DependencyLimits. Either add rows or scope FR-255 to the limits it lists. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:25-75,121 |
| FND-004 | medium | FR-255's Default column gives references, not values, for intake.input_bytes ("the semantic-IR reader's input byte default"), library.definitions ("FR-111's published default") and the five i2.* rows ("IR's bounded read default"). FR-255-AC-6 asserts the effective limits "equal those defaults", which cannot be checked against the spec, and D-1 item 5 requires a published default. State the numbers (or the exact constant they equal). | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:52,56,62-66,124 |
| FND-005 | medium | NFR-007 (rewritten in this PR) keeps a fixed cap: "The native v1 package path ... clamps elevated options to the defaults." The rule is no fixed caps; a caller limit is raised and used as given (as NFR-006/NFR-009 now say). Delete the clamp. | spec/non-functional/NFR-007-bound-native-packages.md:31-34 |
| FND-006 | medium | FR-025 (amended in this PR) keeps caller-lowered-only ceilings of 1 MiB source and 10,000 entries and "callers can inspect the clamped limits". Same ruling as FND-005: published defaults the caller can raise. | spec/functional/FR-025-compile-rule-model-source.md:21,39,44-45 |
| FND-007 | low | FR-255 restates QSpec FR-461 normatively (the dotted naming rule and the exact rendering string in Behavior 3) while citing FR-461 only in References. Cite FR-461 for the rule and rendering, and keep only QSL's table, builders and CLI parsing. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:38,92-97 |

## Verdict

Changes requested (two high, four medium). The design matches the rulings:
no new depth limit kind or depth knob is introduced, defaults are unchanged,
every new limit outcome names limit, value and setting, and the follow-up
commit correctly turns ancestor_steps/family_steps into charged edge limits
and removes the NFR-006/NFR-009 clamps. But one AC in the diff still
asserts a depth cause (FR-092-AC-7), a library depth limit survives in
FR-099, two fixed size caps remain in amended text, and FR-255's table is
neither complete nor fully valued.
