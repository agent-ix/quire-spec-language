---
id: SR-749
title: "PR 494 gap analysis against FR-100 (QSL-292)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-100 (Status, outcome mapping, kernel-record table, undefined table, internal failure, AC-9, AC-10); TC-452; TC-500; FR-096 lines 278-288 and 474-486 (report only); qsl-replay/src/spine/call.rs; qsl-replay/src/spine/call/tests.rs; src/command/output.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/tests/it/collection_queries.rs; qsl-foundation/tests/kernel_refusal_record.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-452
    type: reviews
---
## Summary

Ticket: QSL-292. PR: quire-spec-language#494. This checks the coder's three claims that #490 had already done the work, and the reworded FR-100 Status.

- **`sum-out-of-domain` among the undefined reasons: true.** It appears at qsl-replay/src/spine/call.rs:454, src/command/output.rs:684 and qsl-replay/src/spine/call/tests.rs:560.
- **Spine assertions for all 12 kernel records: partly true.** All twelve are converted. `CardinalityOutOfBound` and `ForeignReference` assert their exact fields and locus. The other ten assert only the code and that an `expected` key exists (FND-001).
- **Empty-sum rule in FR-100-AC-10: partly true.** The code implements it (evaluate.rs:1619-1628), and it is tested at the evaluator under TC-500/FR-096-AC-14. No test carries TC-452 step 5 or FR-100-AC-10 (FND-002).
- **FR-100 Status: true of the code at head.** Every kernel refusal but `CheckedInvariant` builds a record. A seed, a running total, or an empty sum whose `N` excludes 0 is `SumOutOfDomain` (evaluate.rs:1512-1545 and 1619-1628). The claim to have implemented the ACs is only as strong as the tests behind it, which is what FND-001 and FND-002 cover.
- **FR-096 (another lane, report only, not a finding):**
  - Lines 284-285 are current: they point to FR-100's `sum-out-of-domain` row, which exists.
  - Lines 481-482 ("conversion to an `InternalFault` is not built") are true of the evaluator. `spine::run` does convert (call.rs:550-552), and FR-100:380-382 reads FR-096 that way. This is accurate but could be clearer if it said "in the evaluator".

## Verdict

**Changes requested.** Two medium test gaps, both in tests that the PR's Status rewrite claims are done, and one low stale-status finding. The code itself is correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | For the ten revision-1-draft.8 kernel records, the TC-452 step 4 spine test (FR-100-AC-9, ticket bullet 1) asserts only `code` and `fields.contains_key("expected")`. It does not assert the exact `fields`, the `locus`, or TC-452's payloads: three `DivisionPairOutOfDomain` flag pairs (only `(false, false)` is built), `Text[1, 8; binary-utf8]`, and `IeeeNotExact` binary64 with `overflow,inexact`. Exact fields are covered only at the kernel level (qsl-foundation/tests/kernel_refusal_record.rs, TC-428). | qsl-replay/src/spine/call/tests.rs:432-504 |
| FND-002 | medium | FR-100-AC-10 (TC-452 step 5) has no test that traces it. The only empty-sum test is tagged `TC-500`/`FR-096-AC-14`. It uses `Int[1, 3]` rather than `Pos = Int[1, 9]`, stops at `Evaluation` without the outcome mapping or exit 20, and does not test the non-empty case (`q` holding `4` completes `"4"`). | qsl-eval/tests/it/collection_queries.rs:1475-1507; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:83-91,165-168 |
| FND-003 | low | TC-452 Status still says step 4's ten kernel records and step 5's `sum-out-of-domain` reason are "not yet implemented". That contradicts FR-100's reworded Status in this PR. Update it once the FND-001 and FND-002 tests land. | spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:172-175 |
| FND-004 | low | (Round 2, new.) FR-100 never states that `run` cannot reach `Undefined::SumOutOfDomain`. Linked checking refuses any `sum` whose prefix interval is not proved within `N` (`Definedness` over every function body; `prefix_sums` includes 0 for a possibly empty source), so a real program refuses at `check` with `undefined_expression`/`unproved-range`. FR-096:286-288 states this; FR-100's undefined table and its sum paragraphs present the row as a `run` outcome. AC-10 is correctly scoped ("checked under `CheckMode::Kernel`"), so neither the AC nor the test is wrong. Add one sentence to FR-100 after the empty-`sum` paragraph saying the row exists for the mapping's totality and is reached only by a kernel-checked expression. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:194-206 |

## Dispositions

Round 2, reviewed at the fix commit.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | all ten records (twelve cases, counting the three `DivisionPairOutOfDomain` flag pairs) now assert the exact code, cause, exact `fields` with TC-452's payloads, the locus digest and byte span, and the location (qsl-replay/src/spine/call/tests.rs:432-598) |
| FND-002 | fixed | `tc_452_step_5_sum_over_pos_is_sum_out_of_domain_or_completes` (qsl-replay/src/spine/call/tests.rs:793) is traced `TC-452`/`FR-100-AC-10`. It checks `sum<Pos>` for `Pos = Int[1, 9]` under `CheckMode::Kernel` and evaluates it for real: the empty `q` and `[9, 9]` each give `SumOutOfDomain` at the `sum` node, which `convert_outcome` maps to `sum-out-of-domain`; `[4]` completes `"4"`. The render and exit 20 compose with `undefined_kernel_reasons_render_and_exit_20` (src/command/output.rs:678). Not going through `spine::run` is correct: see FND-004. The `qsl-semantics` `test-support` dev-dependency follows the workspace pattern (qsl-eval, qsl-package and the root crate do the same) and is guarded by `no_shipped_dependency_enables_test_support`, which passes. |
| FND-003 | fixed | TC-452 Status now says step 4's records and step 5 passed under QSL-245 and QSL-292 (spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:172-176) |
| FND-004 | open | New in round 2, low, not blocking: a one-sentence FR-100 clarification. |

Round 3. The change is spec text only; `quire validate` on FR-100 is clean.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | FR-100:207-211 now states that `run` never yields `sum-out-of-domain`, that linked checking refuses with `unproved-range` (FR-096), that the row keeps the mapping total, and that only a `CheckMode::Kernel` expression evaluated at S6a reaches it (FR-100-AC-10). This matches `Definedness`/`prefix_sums` in qsl-semantics/src/check/facts.rs. |
