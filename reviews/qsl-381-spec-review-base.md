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

## New findings (disposition pass 1)

Reviewed at 76e47d3a96aacde91f7964e872d50dcd87dfbc15. Delta 9d240032..76e47d3a (16a9154f fix round, 99139757 plan-lead items, 76e47d3a round-2 rulings), limited to lines the delta changed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-259 item 3 and ADR-030 D-4.4 keep `IDENTITY_LIMITS` as a fixed byte ceiling ("keeps its byte ceiling"; "SHALL carry a byte limit only"; a site may only pass a "tighter" budget). It has no FR-255 setting and no way to raise it, so it is the fixed cap the no-caps ruling removes everywhere else in this PR. Give it a setting and published default, or have every caller pass its stage's own input-bytes limit. | spec/functional/FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md:52-54; spec/decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md:264 |
| FND-009 | low | FR-040's edited Behavior line still says the compiler exposes "a versioned checking-accounting contract". Version wording on a contract is the tracking language this PR removes elsewhere. Drop "versioned". | spec/functional/FR-040-check-composed-values.md:116 |

## New findings (disposition pass 3)

Reviewed at 4c79ab4f6ac98d895ea3de29a835de9e32d1386c (rebased onto main 2ece5712), limited to the delta since 25dff5cd.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | FR-356 item 5 and AC-5 say "no crate in the core SHALL depend on `stacker` or on the `maybe_grow` wrapper". But item 3 puts `maybe_grow` in `quire-walk`, and 6e5bbc8d adds `quire-walk` to ADR-029 CB-2's qualified-core crate set ("without its `std` feature"), so a core crate defines the wrapper and optionally depends on `stacker`. AC-5 as written fails against the design. The "without its `std` feature" guard also does not hold in a workspace build: Cargo unifies features, so any non-core crate that enables `quire-walk/std` turns `stacker` on for the core's `quire-walk` in the same build. Either move `maybe_grow` to its own non-core crate, so the core's crate set never contains it, or restate item 5 and AC-5 as testable facts: no core crate calls `maybe_grow`, and the core's crates built on their own resolve `quire-walk` without `std`. | spec/functional/FR-356-walk-nested-structures-through-one-iterative-walker-toolkit.md:55-75,95; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:200 |
| FND-011 | medium | FR-056 makes `Text` the native text name (QSL-290), but the same paragraph and AC-11 then refuse every `ix://quire/native/Text` `typeRef` with `unsupported_construct`/`declaration-form`, because the semantic-IR constraint vocabulary cannot carry `Text`'s `profile`. So a domain-package field, parameter or result of type `Text` still cannot pass through intake. That is the dead-arm gap QSL-290 reports, now refused by design rather than fixed. Text fields are a common need. Define how intake reads a `Text` member's `profile` (for example, the model's selected value profile, or a constraint-vocabulary entry the semantic IR gains), or record the refusal as a temporary gap with its owning ticket. Do not specify the refusal as the meaning. | spec/functional/FR-056-admit-domain-package-model-declarations.md:256-266,306 |
| FND-012 | low | FR-031:29 says the decoded native/formal pair "performs no revision conversion". That states an absence, not a behaviour, and with STD-150 merged in QSpec, source revisions are no longer part of source identity at all. Delete the clause. (QSL FR-001 still lists four source labels, including `revision_namespace` and `revision`. Aligning FR-001 with STD-150 is outside this diff.) | spec/functional/FR-031-run-extracted-native-source.md:29 |

## Dispositions

Round 1, reviewed at 76e47d3a96aacde91f7964e872d50dcd87dfbc15.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 16a9154f: FR-092-AC-7 no longer names a depth cause. |
| FND-002 | fixed | 16a9154f: DependencyLimits bounds the import graph by libraries, edges and bytes; FR-087 and ADR-017 drop the depth row. |
| FND-003 | fixed | 16a9154f: FR-255 lists every limit of the named requirements, including `types.ancestor_steps` and `types.work_units`. |
| FND-004 | fixed | 16a9154f: FR-255 gives concrete defaults. |
| FND-005 | fixed | 16a9154f: NFR-007 no longer clamps. |
| FND-006 | fixed | 16a9154f: FR-025 limits are caller-configured with published defaults. |
| FND-007 | fixed | 16a9154f: FR-255 cites QSpec FR-461 for the naming rule. |

Round 2, reviewed at 25dff5cd4dcd490169c3666748f7e695996ac181.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-008 | fixed | 5554c06c: `IDENTITY_LIMITS` is the published default of the caller's `identity.input_bytes` limit, with an FR-255 row; ADR-030 matches. |
| FND-009 | fixed | 5554c06c: FR-040 drops 'versioned'. |

Round 3, reviewed at 4c79ab4f6ac98d895ea3de29a835de9e32d1386c. No finding had a still-open or missing outcome before this round.
