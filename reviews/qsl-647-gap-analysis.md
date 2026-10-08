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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The PR adds FR-070-AC-13 to TC-905 in spec/tests.md and tags `tc_905_encoding_work_is_linear_in_depth` with `#[trace("TC-905", "FR-070-AC-13")]`, but TC-905 itself is unchanged: its Scope line (line 20) lists FR-070-AC-8, AC-9, AC-11 and AC-12 only, its Test Procedure has no step for the work bound, its Expected Results say nothing about it, and its tag list (lines 72-74) omits FR-070-AC-13. The test case that is said to verify AC-13 does not describe how. Fix: add FR-070-AC-13 to Scope, a step (encode nested sets, options and sequences at depth d and 2d, count work, check the ratio and the output bound, check `value_text_len` equals the text's length), its expected result, and the tag. | spec/test-cases/TC-905-composite-witness-value-text-decodes-and-refuses.md:20, spec/test-cases/TC-905-composite-witness-value-text-decodes-and-refuses.md:72-74 |
| FND-004 | medium | FR-070-AC-13 bounds "the counted work" but never says what is counted. The test counts bytes the shell writer produces plus bytes compared while sorting; another implementer could count nodes visited, allocations or copies, and get a different ratio for the same encoder. The bound is only as strong as that choice (a copy the counter omits is invisible, which was SR-1380 FND-002's failure). Its last sentence, "The value text's length is counted without building the text", is not checked by any test; the test only checks `value_text_len` equals the text's length. Fix: state what is counted (every byte written into an encoding buffer or copied between buffers, plus every byte compared to order set and bag elements), and either drop the last sentence or state it as a checkable property. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:242 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-070-AC-13 still claims "The value text's length is counted from the finished bytes without building the text", and TC-905's Description repeats it, but none of the AC's stated checks can fail if `value_text_len` builds the text. The test reads the buffers of `encode_pieces`, not what `value_text_len` allocates, so `value_text_len` written as `self.to_value_text().map_or(usize::MAX, \|t\| t.len())` passes every condition in AC-13 and TC-905 step 6. The crate forbids `unsafe`, so a counting allocator is not available to check the claim directly. Fix: make the clause say only what is checked ("counted from the encoder's finished bytes: for a text value of ...") in FR-070-AC-13 and TC-905's Description, or accept the clause as an unverified design statement. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:242, spec/test-cases/TC-905-composite-witness-value-text-decodes-and-refuses.md:18-20, qsl-replay/src/witness/value_text/tests.rs:545-565 |

## Dispositions

Round 1, reviewed at `2d987b0db83e89b0c60a5dd344cae16c8911e919`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6cce4788: the two tests are renamed and retagged. `tc_905_the_element_order_check_reads_spans` is `#[trace("TC-905", "FR-070-AC-9")]`, which is what it checks (set, bag and ordered-set order and distinctness refusals). The work-bound test is `tc_905_encoding_work_is_linear_in_depth` with `#[trace("TC-905", "FR-070-AC-13")]`. Neither claims FR-263-AC-1 any more. |
| FND-002 | fixed | 6cce4788: FR-070-AC-13 now states the work bound over nested sets, options and sequences, and the work-bound test traces to it. TC-905's own text was not updated (FND-003) and the AC leaves "counted work" undefined (FND-004). |
| FND-003 | still-open | New this round (see New findings); no fix yet. |
| FND-004 | still-open | New this round (see New findings); no fix yet. |

Round 2, reviewed at `db471524f23aea363f960b34e273f578443eb5f5`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | 3f5c6e90 and 9197ee57: TC-905 Scope lists FR-070-AC-13; step 5 (work at depths 5,000 and 10,000 over nested sets, options and sequences on a 512 KiB stack, then a round trip) and step 6 (the 1,000,000 `;` text) are in the procedure with matching expected results, and the tag list includes `#[trace("TC-905", "FR-070-AC-13")]`. Both AC-13 tests carry that tag. |
| FND-004 | fixed | 3f5c6e90 and fc0307fb: FR-070-AC-13 defines the counted work as the bytes the JCS writer produces plus the bytes compared while ordering set and bag elements, and the output as the JCS bytes before escaping, which is exactly what `Work` counts. The shapes, depths and bounds it states are the ones `tc_905_encoding_work_is_linear_in_depth` asserts. The last sentence is now stated as checks `tc_905_value_text_len_counts_without_building_the_text` makes (3,000,026; one finished value; buffers at least 1,000,026; each capacity below 3,000,026). Those checks do not establish the clause they follow; that residue is FND-005. |
| FND-005 | still-open | New this round (see New findings); no fix yet. |
