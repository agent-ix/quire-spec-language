---
id: SR-1295
title: "Code review of quire-spec-language PR #630: B7 flat v2 read and IR decimal-string integers"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e28bc06358a790a8e27a74b6208956ede2ddfb19; PR #630 diff against origin/main: Cargo.lock, qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests.rs, qsl-package/src/emit/tests/admission_corpus.rs, src/lowering/wire.rs, examples/author_native_package_vectors.rs, tests/** (vector and expectation moves), tests/fixtures/native-package/controls.*"
review_set: subset
---
# Code review of quire-spec-language PR #630

## Summary

Ticket: QSL-488 (B7), with the IR-274 part B consumer. PR: quire-spec-language#630.
Rust lane (rust-review) folded in.

Checked and clean:
- Cargo.lock: quire-contract-model moves to IR main 6fb6e974, which contains
  IR #278's merge (1117eba6). quire-canonical 5dc4e12d, quire-verification-contracts
  ec4563ff and quire-walk 89d05df2 match IR main's own lock. One copy of each.
- `V2ReadLimits.depth`, its clamp, `enforced()` and the `CheckedPackageLimit::Depth`
  mapping are deleted with no shim. `effective_limits` is now the caller's limits as given.
- `src/lowering/wire.rs`: `Decimal<T>` writes `collect_str` of an `i64`, which is
  always in IR's `IntegerString` grammar `^(0|-?[1-9][0-9]*)$` (no `-0`, no leading
  zero). IR main's `WireExpressionKind::IntegerLiteral.value` and the input wire's
  integer `minimum`/`maximum` are `IntegerString`, so the members QSL emits match
  what IR reads. QSL emits no rational type or literal on this wire.
- Rounding control: the old `9007199254740993` vs `...992` control caught a
  producer that sends the revision through `f64`. IR now refuses revisions above
  2^53 and every integer up to 2^53 is exact in `f64`, so that defect can no
  longer happen for a revision. The "revision off by one" replacement is a fair
  control. For the decimal-string members the `i64::MAX` bound in
  `integer_members_serialize_as_decimal_strings` catches a float cast.
- FR-264-AC-3 and AC-5 tests pass locally (qsl-package lib, 40 filtered tests).

## Verdict

Request changes: two medium and two low findings. The B7 deletion and the IR bump
are correct. The schemas still admit values the decoder now refuses, and the TC-138
frozen count is version tracking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The published schemas still allow revisions and byte offsets up to 18446744073709551615, but the decoder now refuses anything above 2^53 (IR `MAX_EXACT_INTEGER`). The runtime schema test was edited to use 2^53 instead of adding a refused 2^53+1 case, so nothing tests the schema against the decoder at the new boundary. Set these `maximum`s to 9007199254740992 and add a 2^53+1 case that the schema refuses. | schemas/native-linked-package-1.schema.json:103; schemas/native-linked-package-1.schema.json:122; schemas/native-linked-package-1.schema.json:264; schemas/native-state-input-1.schema.json:54; tests/runtime_reading_cases/schema.rs:270-286 |
| FND-002 | medium | TC-138 freezes the exact tuple `(4_825, 2_580, 1_168_366, 85_289)`. Its comment is a list of every past reason the number moved. The PR changed the number to the newly observed value (+456) without working it out independently, so the assert passes for whatever the code produces and only records the figure. The loop below it already tests the real behaviour: a limit of amount-1 fails in that dimension, the exact amount passes, limits above the hard cap clamp, and a retry matches. Delete the frozen tuple and its comment, or replace it with a relation (for example `byte_work >= emitted.bytes().len()`). | tests/it/compiled_protocol_v2.rs:2509-2537 |
| FND-003 | low | `maximum_revision_survives_...` replaced the u64-overflow spelling `18446744073709551616` with `9007199254740993`. That swaps the parse-overflow path for the range check, though TC-083 asks for "malformed/overflowed integers". Keep both, and add `18446744073709551615`, which is now refused too. | tests/package_reading_cases/raw.rs:193 |
| FND-004 | low | No test sends an integer bound or literal above 2^53 from QSL's lowering wire through IR's wire decoder. The wire.rs test checks serialization only, and the strict-reader test in integer_lowering uses 0..1000. Add an `i64::MAX` bound case to the strict-reader round trip. | src/lowering/wire.rs:359-417; tests/it/integer_lowering.rs:129-138 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4c0f5cc54ce95f2e0dd9b3ec9129235c9d02b60e |
| FND-002 | fixed | 4c0f5cc54ce95f2e0dd9b3ec9129235c9d02b60e |
| FND-003 | fixed | 4c0f5cc54ce95f2e0dd9b3ec9129235c9d02b60e |
| FND-004 | fixed | 4c0f5cc54ce95f2e0dd9b3ec9129235c9d02b60e |
