---
id: SR-1381
title: "Gap analysis of quire-spec-language PR #660: composite witness values encode once (QSL-647)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2219a6a2b88b5a93156d70bb2af10377da2074a0; PR #660 diff against origin/main: qsl-replay/src/witness/value_text.rs, qsl-replay/src/witness/value_text/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-736
    type: reviews
---
# Gap analysis of quire-spec-language PR #660

## Summary

Ticket: QSL-647. Planless gap analysis over the PR diff. Plan completion: not assessed.

The PR changes one production file (qsl-replay/src/witness/value_text.rs) and adds two tests. It has no spec change. The ticket's done-criterion is that each set or bag element's canonical bytes are computed once per decode at any depth, with a test showing linear encoding work.

Examined:
- FR-070 Decode refusal list and FR-070-AC-9 (set/bag/ordered-set order and distinctness refusals): examined. The span-based check still refuses each case; the existing TC-905 FR-070-AC-9 tests pass at the head.
- FR-070-AC-10 and FR-263-AC-1 (TC-736, 100,000-long recursive list on a 512 KiB stack): examined. The tests pass at the head, but see SR-1380 FND-001 for their run time.
- FR-263 Behavior and TC-736 Test Procedure: examined for the trace tags of the two new tests.
- FR-358 (composite bounded-shadow settlement): context_only. It sets refusal precedence for settlement rows only, not for decode faults, so the changed fault order (a shape fault now reported before an element-order fault) does not conflict with it. FR-070 lists decode faults with no precedence, and `DecodeRefusal::Malformed` carries one fault, so the order change is allowed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Both new tests are tagged `#[trace("TC-736", "FR-263-AC-1")]`, but neither verifies FR-263-AC-1 (a request replays to the proving verdict and its recompiled `package_id` matches). `tc_736_the_element_order_check_reads_spans` checks FR-070-AC-9 refusals (set out of order, duplicate set and ordered-set elements) and belongs under TC-905/FR-070-AC-9. `tc_736_nested_sets_encode_each_value_once` checks the encoder's work bound, which no AC states (FND-002). The tags inflate FR-263-AC-1 coverage with tests that do not exercise it. | qsl-replay/src/witness/value_text/tests.rs:474, qsl-replay/src/witness/value_text/tests.rs:498 |
| FND-002 | medium | The ticket's done-criterion (each value's canonical bytes computed once; encoding work linear in output size, not multiplied by depth) is not stated by any FR or AC. FR-070 and FR-263 describe depth only as stack use. With no requirement, the work bound has no AC for a test to trace to and nothing in the spec stops a later change from regressing it, as SR-1380 FND-001 shows already happened for every nested composite. Add an AC (FR-070 or FR-263) that states the bound over encode and decode, and trace the work-bound test to it. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-263-replay-at-any-depth-under-the-request-limits.md |

## Verdict

Two medium findings, both on traceability. The decode refusal coverage (FR-070-AC-9) is intact. Not mergeable until both are fixed, together with SR-1380.
