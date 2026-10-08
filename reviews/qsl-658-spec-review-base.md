---
id: SR-1383
title: "Spec review of quire-spec-language PR #666 (QSL-658): FR-261-AC-3 states ADR-030 D-4.10's criterion"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@48a6a04a26b55c01feee1416e8fb4d19c178bae8; PR #666 diff against origin/main; spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md (AC-3, deleted Status); spec/test-cases/TC-733-library-and-observation-reads-judge-deep-documents-on-content.md (step 3, expected result, deleted Status); checked against qsl-semantics/tests/it/state_clauses.rs a_value_nested_100_000_deep_is_counted_as_observation_values and its harness"
review_set: base
---
# Spec review of quire-spec-language PR #666

## Summary

Ticket: QSL-658. The PR rewords FR-261-AC-3 from "admits a 100,000-deep value" to ADR-030 D-4.10's criterion: the observation document reader reads the value, counts it as `observation.values` and judges it on content (`invalid_runtime_input`/`wrong-value-kind`). It rewrites TC-733 step 3 to match and deletes the FR-261 and TC-733 Status sections that deferred admission to QSL-639.

Examined:
- FR-261-AC-3, new text (examined)
- FR-261 Status section, deleted (examined)
- TC-733 step 3 procedure and expected result (examined)
- TC-733 Status section, deleted (examined)
- TC-733 Description (examined)
- ADR-030 D-4.10 criterion (context_only)
- The test `a_value_nested_100_000_deep_is_counted_as_observation_values`, `nest_present`, `run_tc465_current_text`, `run_tc465_with_clauses_under` (512 KiB thread) and `assert_tc465_refused` (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-261-AC-3 says the refusal comes "with `observation.values` one below the document's value count", naming its bound and the count reached. The test does not do that. It sets the bound to 50,000, well below the document's count of more than 100,000, and asserts `bound` 50000 and `actual` 50001. One below the document's count would give bound N-1 and actual N. That clause is not asserted. TC-733 step 3 says the same thing. Reword both to what the test asserts: a bound the nested value alone crosses (50,000) refuses naming `observation.values`, bound 50000 and count reached 50001. | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:63; spec/test-cases/TC-733-library-and-observation-reads-judge-deep-documents-on-content.md:33,46-47 |
| FND-002 | medium | FR-261-AC-3 says the reader refuses `invalid_runtime_input`/`wrong-value-kind` "at that field". The test checks only the code and cause through `assert_tc465_refused`. It never reads `record.fields["field"]`, though the reader sets that field (`document.rs:432`, `.with("field", field)`) and sibling TC-465 tests assert it. TC-733's expected result has the same "at that field" claim. Either add the `field` assertion (`parent`) or drop "at that field". | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:63; qsl-semantics/tests/it/state_clauses.rs:3561 |
| FND-003 | low | TC-733 step 3 prescribes three runs in order: raised, then one below, then raised again. Its expected result is wrong-value-kind, then the values refusal, then wrong-value-kind "again", "after the whole value is read and counted". The test makes two runs: the lowered bound first, then `raised` once. Neither the third run nor "the whole value is read" before judging is asserted. The 50,001 count shows only that the walk passes 50,000 levels. FR-261-AC-3's "judges the field again" assumes the same earlier run. Reword step 3 and its expected result to the two runs the test makes, and drop "after the whole value is read". | spec/test-cases/TC-733-library-and-observation-reads-judge-deep-documents-on-content.md:30-34,44-47 |
| FND-004 | low | TC-733's Description still ends "...and admit deep documents". After this PR, none of TC-733's steps admits a deep document. Step 1 refuses `MemberType("edition")`, step 2 proceeds to a wrong-value-kind refusal, and step 3 refuses wrong-value-kind. This is the last spec text that reads the deep observation value as admitted. Say "and judge deep documents on their content", matching the title. | spec/test-cases/TC-733-library-and-observation-reads-judge-deep-documents-on-content.md:13-15 |

## Verdict

The direction matches the approved scope and ADR-030 D-4.10. The criterion is now read, count and judge on content, not admission. Both QSL-639 Status sections are gone, and no spec text outside `reviews/` mentions QSL-639 any more. No nested-Option declaration and no reader code was added. The reworded AC matches the test on four points:
- the 100,000 depth (`nest_present(.., 100_000)`)
- the 512 KiB thread (the harness spawns admission with `stack_size(512 * 1024)`)
- `observation.input_bytes` and `observation.values` raised through the builder (4,000,000 / 1,000,000)
- the `observation.values` refusal (`stage_limit_exceeded`/`node-count-exceeded`) naming its setting, bound and count

It does not match on the "one below the document's value count" clause (FND-001) or on "at that field" (FND-002). TC-733 also prescribes a third run the test doesn't make (FND-003). FR-261 and TC-733 pass `quire validate`.

Two medium findings and two low. **Not mergeable until FND-001 to FND-004 are fixed.** All four are wording fixes, plus at most one assertion line for FND-002.
