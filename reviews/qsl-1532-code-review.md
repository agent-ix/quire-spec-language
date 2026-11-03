---
id: SR-20261103213521
title: "Code review — QSL-1532 retained protocol successors (PR #2197)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2197 diff origin/main...ab648524a: qsl-eval/tests/it/model_reference_queries.rs, qsl-package/src/emit/tests/{protocol_requirements/fixture_w.rs,temporal_clause.rs}, qsl-semantics/src/check/protocol_clause/tests/graph.rs, qsl-semantics/tests/it/{model_population,relationship_snapshots,state_clauses}.rs, tests/it/{assessment_artifact_kinds,checked_handoff,compiled_protocol_v2}.rs"
review_set: subset
---

## Summary

Ticket: QSL-1532. Code review with the rust-review lane folded in. The PR is
tests and trace tags only: it adds FR-513-AC-9..15 / TC-1308 tags to existing
tests, extends the v2 strict-reader mutation table, and adds a Contract IR
reader identity-mutation test, a three-binding `i2.input_bytes` test, and
skipped-operand cases for the visible-fact check.

The main defect is the direction of the tags. FR-513 lists successor criteria
for retained criteria that run on retired paths. TC-1308 says the successor
tests keep the legacy tags. This PR does the opposite: it puts successor tags
(FR-513-AC-9, AC-11, AC-15) on the retired v2 and v3 tests. That makes the
matrix report the successors as covered by exactly the code that FR-513 says
will be deleted.

Only the committed PR head `ab648524a` was reviewed. The worktree also has
uncommitted edits to `qsl-package/src/emit.rs` and `temporal_clause.rs`
(an emission-side `InvalidTemporalClockValue` refusal), modified seconds
before this review. Those edits are not part of the PR and were not reviewed.

## Verdict

**CONDITIONAL**. The new test code is idiomatic, deterministic and passes, with
no panics outside assertions. The trace tags in FND-001 to FND-004 must be
corrected before merge, or they inflate FR-513 coverage. FND-005 is optional
hardening.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | high | `strict_reader_rejects_structural_and_identity_substitutions` is tagged with FR-513-AC-9, and its new cases (out-of-range declaration, foreign source, empty sequence authority, definition revision and artifact) go through the retired `protocol_artifact::v2` reader (`read_v2_bytes`) using the v2 refusal vocabulary. FR-513-AC-9 requires refusal **at emission** with `invalid_package`/`invalid-value`, and the FR-050-AC-2/AC-3 row names AC-9 as the emission successor. Fix: drop the FR-513-AC-9 and TC-1308 tags from this test, and add the clock-value and definition-substitution cases at emission in qsl-package. | tests/it/compiled_protocol_v2.rs:2937-2945; tests/it/compiled_protocol_v2.rs:2965-3071 |
| FND-002 | medium | `every_case_gets_the_refusal_of_its_row` is tagged with FR-513-AC-9, but it runs the FR-518 reader/selection mutation table against `read_checked_package`. It never refuses at emission, which is what AC-9 is about. Fix: remove the FR-513-AC-9 and TC-1308 tags. | tests/it/checked_handoff.rs:1022 |
| FND-003 | high | The TC-143 v3 test is tagged with FR-513-AC-11 even though it drives `native::admit_v3`. FR-513's FR-054 row says `protocol_artifact::v3` and TC-143 are the kept retired path, and FR-513-AC-11, the derived activation mapping of a checked package, is the successor. If v3 is deleted, AC-11 loses its only tag. Fix: remove FR-513-AC-11, FR-513-AC-15 and TC-1308 from this test, and write the successor test on the checked package's derived mapping. | tests/it/compiled_protocol_v2.rs:1313-1322 |
| FND-004 | medium | FR-513-AC-15 tags are on the TC-136, TC-138, TC-143 and TC-150 behaviour tests. AC-15 is a matrix-regression test: deleting a retained module must make the matrix test fail. None of these tests reads the matrix, and the PR body says no harness exists (QSL-1584). The tags make `quire matrix` report AC-15 as covered. Fix: remove the FR-513-AC-15 tags until the QSL-1584 harness exists. | tests/it/assessment_artifact_kinds.rs:7; tests/it/compiled_protocol_v2.rs:1313-1322; tests/it/compiled_protocol_v2.rs:2521; tests/it/compiled_protocol_v2.rs:2805-2812 |
| FND-005 | low | `protocol_package_is_independently_admitted_and_identity_mutations_refuse` asserts only `Refused(_)` for each mutation. It has no control showing that an unmutated `serde_json::to_vec(Value)` round-trip is admitted, and it does not check the cause or pointer. Any refusal passes, for example a whole-byte check or `u64::MAX` being out of range, so the test does not show the reader checks each named identity. Fix: add `admitted(&serde_json::to_vec(&unmutated), &held)` and assert each refusal's cause and pointer. | tests/it/checked_handoff.rs:1076-1130 |

## Verification

- Built the committed head `ab648524a` in a detached scratch worktree, through `locked-build.sh` with the shared `port-target`.
- Passed: `cargo test --locked --all-features -p quire-spec-language --test it`, filtered to `compiled_protocol_v2::strict_reader_rejects_structural_and_identity_substitutions` and `checked_handoff::protocol_package_is_independently_admitted_and_identity_mutations_refuse` (2 passed).
- Confirmed `serde_json` has no `preserve_order` (no `indexmap` in Cargo.lock), so `Value` re-serialisation sorts keys. FND-005 is about missing assertions, not a known false pass.
