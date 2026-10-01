---
id: SR-910
title: "QSL-340 gap analysis of PR 542 (NFR-007, FR-097-AC-6, TC-440, checked_v2 depth)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a305f56ffc793e2470708c45af4c77fba094f3a1; spec/non-functional/NFR-007-bound-native-packages.md; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md; spec/test-cases/TC-440-qsl-extent-agrees-with-ir-requires-bound.md; qsl-package/src/checked_v2.rs; qsl-package/src/checked_v2/tests.rs; qsl-package/src/emit/extent_agreement.rs; qsl-semantics/src/family/requirements.rs (read, context); quire-contract-model at ead72675 v2/lower.rs (read, context); ticket QSL-340 body and scope comment"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-440
    type: reviews
---
## Summary

Ticket: QSL-340. PR: quire-spec-language#542 at a305f56f. No plan bundle;
the acceptance text is the ticket body, its scope comment (three items:
reference `MAXIMUM_DEPTH` instead of the 128 constant, update four docs,
rename and re-expect the depth test), FR-097-AC-6 and NFR-007.

Ticket scope: all three items are done. The constant is referenced, not
copied; `V2ReadLimits.depth`, `enforced()`, `effective_limits` and the
module-doc bullet are updated; the test is renamed and expects
`Incomplete(Depth, 128, 201)`.

Coverage measured (reviewer run, focused `qsl-package` tests, exit 0, 9
passed, 1 ignored):

- NFR-007 v2 exception, over-limit side: `depth_far_past_the_default_limit_is_incomplete_not_malformed_wire`,
  `depth_past_the_charged_maximum_is_incomplete_naming_the_maximum`.
- NFR-007 / FR-087-AC-3 recorded ceiling: `a_verified_read_records_depth_as_the_charged_maximum` (TC-253).
- FR-097-AC-6 record clause: `tc_440_qsl_extent_agrees_with_ir_requires_bound` over nine fixtures.
- FR-097-AC-6 per-application clause: `tc_440_an_unbounded_application_record_requires_a_bound_in_ir`
  (outer `+`, now `RequiresBound`) and `tc_440_operation_application_records_agree_with_ir_per_node`.
- Quantity fixture: ignored, reason accurate (fails `NamesAbsentNode` when run).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `RangedTree` expectation `("sequence", Recursive)` is fitted to IR's output, not to AC-6's second clause ("IR's first unbounded node is a form QSL names as a domain"). QSL names its `Recursive` domain at the record root: `requirements.rs:292` inserts at `entered`, the open composite's own path, and `kids` is bounded `[0, 3]`, so QSL names no sequence domain. IR returns the `kids` sequence because it is the lowest `node_id` among recursion-group members (lower.rs:446-483). The test checks only that some `Recursive` domain exists and that IR's node's `semantic_form` is the string `sequence`; it never checks that IR's node is the node QSL named. Failure: an emitter change that reorders digests makes IR name the record node and the test goes red with no semantic change, while today it passes although the named-form clause does not hold for `RangedTree`. | qsl-package/src/emit/extent_agreement.rs:236; qsl-package/src/emit/extent_agreement.rs:243-256 |
| FND-002 | medium | No test shows an elevated depth above 128 is honoured on the admit side. The PR's headline is that a caller depth above serde's old 128 cap is now charged as given, but both new tests are over-limit cases (`(128, 201)` and `(16384, 16385)`). No test reads a wire deeper than 128 under a raised limit and shows it passes the depth check (for example 200 arrays at `depth: 300` must not be `Limit(NestingDepth)`). NFR-007's Verification asks for "elevated options" per dimension. | qsl-package/src/checked_v2/tests.rs:1114-1141 |

## Verdict

Ticket scope is complete and the un-ignored tests are real reds turned
green by the IR move. Two medium gaps: the `RangedTree` agreement assertion
does not test AC-6's named-form clause, and the elevated-depth admit path has
no test. Both should be fixed in this PR (a test change for FND-002; for
FND-001 either assert IR's node is a member of the recursion group QSL named
and amend AC-6 to say so, or assert the node QSL named).

## Dispositions

Round 1, reviewed at c23e4fb596d3b14f8d350b8ea431ddcff997b3a1 (diff a305f56f..c23e4fb5). Reviewer re-ran the focused `qsl-package` depth and tc_440 tests: exit 0, 10 passed, 1 ignored.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c23e4fb5: RangedTree now expects `InRecursionGroup`: the test takes the node QSL names `Recursive` (`key.node()`), asserts it carries a `recursion_group` string, and asserts IR's first unbounded node has the same `recursion_group`; FR-097-AC-6 amended to say so. No longer tied to digest order or a form string (extent_agreement.rs:239-291) |
| FND-002 | fixed | c23e4fb5: `depth_raised_past_the_default_admits_a_wire_deeper_than_the_default` reads a 201-deep wire at `depth: 300` and expects an envelope refusal at the root pointer. Discriminating: at the default it is `Limit(NestingDepth)`, and the old serde-cap refusal (`refused_bytes(MalformedWire)`, IR 7c70041 common.rs:67-75) carries `path: None`, so either regression fails it |
