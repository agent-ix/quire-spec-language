---
id: SR-1367
title: "Gap analysis of quire-spec-language PR #657: FR-033-AC-6 wire round trip and IR-662 adaptation (QSL-648)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@713238dc0b663181a2aa311a6776e09ea663c277; PR #657 own diff b90d17b13...713238dc0; FR-033-AC-6 as planned in open PR #655 (QSL-642, head 90bc007a7) and TC-912; FR-041-AC-3; tests/it/integer_lowering.rs, src/lowering/wire.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: reviews
---
# Gap analysis of quire-spec-language PR #657

## Summary

Ticket: QSL-648. Planless gap analysis. Plan completion: not assessed.

FR-033-AC-6 is not in `spec/` at this head. #654 added it and #656 reverted it, and open PR #655 (QSL-642) carries it again with TC-912. By ruling, its test is planned in this PR, so this analysis judges the new test against #655's AC text. `quoin matrix` at 713238dc0 lists FR-033-AC-1..AC-5 as tagged, with no AC-6 row.

The new test `wide_integer_bounds_and_literals_round_trip_through_irs_strict_reader` lowers an `i64::MAX`-bounded declaration. It rewrites the JSON at the wire, replacing every integer type whose maximum is i64::MAX and every `integer_literal` value with the strings under test. It then asserts that the strict IR reader reconstructs them exactly. On the reader side this is a real oracle. If the rewrite stopped matching, the reader would return the original bounds and the assertion would fail, and every case asserts both bound strings and the literal string. On the producer side it is not: QSL never serializes any of these values, because the test wrote them in.

Examined:
- FR-033-AC-6 (examined, text from #655): "A declaration typed with bounds 0 and 9223372036854775808 (i64::MAX + 1) and a literal 9223372036854775808 serialize with `value_type.maximum` and `value` both `\"9223372036854775808\"`, and the actual strict IR reader reconstructs the declaration's bounds and the literal's value exactly. Bounds 0 and 18446744073709551615 (u64::MAX) ... round-trip the same way ... A declaration bounded by i128::MIN and i128::MAX with literals at both extremes round-trips too ..."
- FR-033 statement (examined, #655): "The compiler shall write each such bound and value on the IR wire as its canonical decimal string."
- FR-041-AC-3 (examined): too-large denominator refuses. See SR-1366 FND-001 for the lost refusal coverage.
- TC-912 (context_only, #655)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-033-AC-6 test checks only the IR reader. The AC's first clause says the values "serialize with `value_type.maximum` and `value` both" as the given strings, and the test writes those strings into the JSON itself, so QSL's `IntegerType`/`decimal` serializer is never run on a value wider than i64. A serializer that narrowed or mis-spelled values above i64 would pass. Source cannot author such bounds yet (QSL-642), but the producer can be driven directly. Extend the `src/lowering/wire.rs` unit test, which already builds an `ir::IntegerType` and an `IntegerLiteral` at `i64::MIN`, to i64::MAX+1, u64::MAX, i128::MIN and i128::MAX, and assert the serialized `minimum`/`maximum`/`value` strings. Then feed those bytes to the strict reader, or keep the integration test for the reader half. | tests/it/integer_lowering.rs:184-258; src/lowering/wire.rs:397-416 |
| FND-002 | low | The new test claims FR-033-AC-6 only in its doc comment and has no `#[trace(...)]` tag, so the matrix will not bind it once #655 lands the AC and TC-912. Add `#[trace("TC-912", "FR-033-AC-6")]` to whichever test carries the AC, after #655 merges or in whichever PR lands second. | tests/it/integer_lowering.rs:184-190 |

## Verdict

One medium and one low finding. The reader-side round trip is a real, divergence-sensitive oracle. The producer-side serialization clause of FR-033-AC-6 has no test. Mergeable once FND-001 is fixed in this PR. FND-002 is due as soon as the AC exists in `spec/`.
