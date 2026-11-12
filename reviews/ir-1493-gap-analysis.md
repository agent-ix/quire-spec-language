---
id: SR-2026111205102300000002
title: "gap-analysis of IR-1493 FR-093-AC-63 Text size (quire-spec-language PR 2687)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (FR-093-AC-63); spec/test-cases/TC-1409-text-size-lowers-to-its-text-identity.md; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md (QSL-owned invariant producers); PR 2687 diff"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1409
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: references
---

# Gap analysis of IR-1493 Text size (quire-spec-language PR 2687)

## Summary

Ticket: IR-1493. Head e53f04012. Planless; criterion read from main. Plan completion: not
assessed. Each clause of FR-093-AC-63 was checked against a test that can fail:

- Typing `Int[min, max]` from `text_bounds`: `size_over_text_is_typed_by_the_text_bounds`
  accepts `Int[0, 8]`, refuses `Int[0, 7]`, and accepts `Text[2, 5]` as `Int[2, 5]` (fails if
  the min bound were dropped or the result stayed `Integer`).
- Evaluation after normalization: `size_over_text_counts_scalars_after_profile_normalization`
  asserts 0, 3 and 1 for U+0065 U+0301 under `nfc` (a byte count gives 2).
- Refusal: `size_over_a_string_or_integer_is_a_type_mismatch_at_the_operand` asserts exactly one
  refusal, `ill_typed`/`type-mismatch`, at operand depth, for `String` and `Int[0, 9]`.
- Lowering: `text_size_lowers_to_one_text_size_call_with_the_profile_law_and_mode` reads the
  bytes back through IR's strict v2 reader and asserts one `quire.op.text.size` application,
  operator `call`, `member` null, `leaves` `[]`, law `text_profile` with
  `quire.value.text.unicode-17.0.0/v1`, mode `text_profile`/`nfc`, one `reference` argument, and
  an `Int[0, 8]` result type. The same run exercises claim keying, so it also fails if
  `claims.rs` did not classify Text Size as a claim site.

All five tests carry `#[trace("FR-093-AC-63", "TC-1409")]`. No AC-63 clause is untested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The evaluator's Size producer now faults a non-Text, non-Collection operand with the new S6a invariant identifier `size-operand-expected`, which is not in FR-090's "QSL-owned invariant producers" table and has no FR-090-AC-15 producer probe in evaluate/invariants.rs. Before this PR the same condition went through `pop_collection` and returned the tabled `collection-value-expected`. The identifier is part of the fault's stable contract, so code and spec now disagree. Either add a table row and probe for `size-operand-expected`, or keep the non-Text case on the tabled `collection-value-expected` identifier. | qsl-eval/src/value/expression/evaluate.rs:309-331; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:491-502 |

## Verdict

FR-093-AC-63 is fully implemented and every clause has a test that can fail. One medium gap:
an S6a invariant identifier with no owning requirement row or probe (FND-001). Fix it in this
PR (one table row plus one probe, or reuse `collection-value-expected`) before merge.
