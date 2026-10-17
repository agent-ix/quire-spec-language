---
id: SR-12054
title: "Gap analysis of quire-spec-language PR #911: state-machine contexts and the Context selection"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b7769a7d707df92277294378d0fec051dbc4f6a1; qsl-eval/tests/it/machine_contexts.rs, qsl-semantics/tests/it/native_snapshot_values.rs, qsl-eval/tests/it/tc235_business_meaning_runtime.rs, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-1045-a-snapshot-carries-the-state-machine-context-of-an-invocation.md, spec/test-cases/TC-1044-text-value-types-in-the-model-document-and-typed-frame-comparison.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #911

## Summary

Ticket: QSL-802. A manual check of acceptance criteria against tests over
`git diff origin/main...HEAD`. No build was run and `quoin matrix` was not
run. Plan completion: not assessed.

Examined: FR-106-AC-41 to AC-47 and AC-49, against TC-1045 steps 1 to 7 and
TC-1044 step 4 / Expected Results. Each AC has at least one test tagged
`#[trace("TC-1045"|"TC-1044", "FR-106-AC-n")]`. AC-48 is excluded by brief.

Bindings: `a_context_selection_admits_one_side_of_the_invocation` → AC-41;
`side_pre_ignores_the_post_side_and_the_parameters` → AC-42;
`a_snapshot_must_hold_exactly_one_context_of_the_machine` → AC-43;
`each_context_defect_refuses_with_its_own_cause` → AC-44;
`the_post_state_range_witness_reaches_a_context` and
`a_context_field_follows_the_value_rules_of_an_object_field` → AC-45;
`the_frame_check_compares_contexts` and
`self_and_the_selection_must_agree_with_the_receiver` → AC-46;
`a_context_reference_closes_against_the_snapshot` → AC-47;
`the_frame_compares_a_record_fields_text_by_its_retained_sequence` → AC-49.
All are correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The AC-47 value-bound oracle comes from the code under test, so it cannot fail when contexts go uncounted. The test takes `exact = owner(..).usage.values` and asserts that `exact` admits and `exact - 1` refuses. If `read_contexts` stopped charging an entry or its field values, `exact` would just be smaller and both asserts would still pass, because the `Q` population's object keeps the count above zero. AC-47's claim "each context entry and each of its field values counts toward `observation.values`" is therefore untested. The refusal is also not checked for naming `observation.values`. Fix: assert the literal count of the `P1`/`O1` document, as TC-1038's `values_used == 17` test does, or compare it with the same document minus the context. Assert the limit name. | qsl-eval/tests/it/machine_contexts.rs:1358-1380 |
| FND-002 | medium | The AC-41 reordering assertions check only the boolean verdict. AC-41 and TC-1045 step 1 require that reordering `fields` members and `contexts` entries gives the same typed observation. The test asserts only `verdict == true` on the reordered documents. A reorder that read the wrong entry, or admitted a different observation that still evaluates `true`, would pass. The whitespace-reorder digest equality is not exercised either. Fix: compare the admitted observation's environment/self values across the original and reordered documents, and add the whitespace case to the digest assertion. | qsl-eval/tests/it/machine_contexts.rs:487-521 |
| FND-003 | medium | AC-45's Text row has no test. AC-45 says a context field of a record, String, UUID, Timestamp or Text type follows the object-field value rules and payload order. The `Owned` fixture declares no Text field, and `a_context_field_follows_the_value_rules_of_an_object_field` covers String, UUID, Timestamp and record only. Fix: add a `Text[..]` field to `Owned` with one payload-defect case at the context path. | qsl-eval/tests/it/machine_contexts.rs:1406-1471 |

## Verdict

FIX: 3 coverage/oracle findings, no AC unmapped.

Clean:
- AC-42: post absent, post not JSON, `parameters` ill-formed, and
  `result`/`created`/`deleted` removed all admit side `pre`. Side `post`
  without its snapshot returns `Incomplete` with the exact code and cause.
- AC-43: wrong-context naming the machine and `p1`/`q1` for all five
  selections. The duplicate and specialization cases refuse at entry 1.
  Billing is ignored. `refused` panics on `Incomplete`, so "never Incomplete"
  is enforced.
- AC-44: one case per condition, with exact code, cause and named member. The
  object-before-context and first-entry-before-second ordering are asserted.
- AC-46: the wide and narrow frames, side `pre` with no frame check, the
  reordered equal context, both `self` kinds, and the five wrong selections.
- AC-47: dangling, incomplete, listed, and the `Spare` and `text: 5` cases.
  The `text: 5` case uses a dangling `p9`, which proves 6.12 runs before
  closure.
- AC-49: the existing record-street test is now tagged to TC-1044/AC-49 and
  asserts both the nfc no-change and the different-street violation.

## Dispositions

Reviewed at 5f0bdccdc34d94dcd338da7f9085c482e709334e.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 13e2c36ca: literal `exact = 9`, asserted against `usage.values`, and the refusal asserted to name `setting` `observation.values` |
| FND-002 | fixed | 13e2c36ca: compares the admitted `placed`/`cancelled` Booleans, label, populations and usage across original, reordered and whitespace-respaced documents |
| FND-003 | fixed | 13e2c36ca: `Owned.title: Title` (Text, nfc, 1..8) with admit, `text: 5`, String-for-Text and over-length cases at the context path |
