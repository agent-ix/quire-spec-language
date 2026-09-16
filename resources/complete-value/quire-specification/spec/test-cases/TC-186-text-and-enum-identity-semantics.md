---
id: TC-186
title: "Text and enum identity semantics"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-141
    type: verifies
---

# TC-186: Text and enum identity semantics

## Description

Compare normalized text and declaration-qualified enum members at valid and invalid boundaries.

## Test Procedure

Select language `ix:native`, edition `1-draft` revision `1-draft.2`, and the
`quire.value.complete/v1` and `quire.value.text.unicode-17.0.0/v1` definitions
at revision `1-draft.1`, using their exact DefinitionRefs in
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
Execute these vectors through the real
local Rust boundary:

Every `ill-typed` expectation below means type-checking
`refused { code: ill_typed }`; it is not a peer evaluator outcome.

| Vector | Inputs/profile | Expected value/disposition |
| --- | --- | --- |
| T01 | U+00E9 and `U+0065 U+0301`; `unicode-scalars` | unequal |
| T02 | the T01 strings under `nfc`, then under `nfd` | equal under each profile; retained normalized sequences are respectively U+00E9 and `U+0065 U+0301` |
| T03 | U+FB00 and `ff` under `nfkc`, `nfkd`, then `nfc` | equal under compatibility profiles; unequal under `nfc` |
| T04 | U+00E9 in `Text[1,1;nfc]` and `Text[1,1;binary-utf8]` | scalar-profile value succeeds at length one; binary value refuses because UTF-8 length is two bytes |
| T05 | empty text in `Text[0,0;nfc]`, U+0061 in `Text[0,0;nfc]`, and U+0061 in `Text[1,1;nfc]` | empty and exact-bound inputs succeed; one-over input refuses |
| T05b | source type `Text` with its required bounds omitted | `refused { code: invalid_syntax }` at source recognition; no unbounded text type |
| T06 | bytes `c3 28` presented at the UTF-8 reader boundary | refused as invalid UTF-8 before profile comparison |
| T06b | source strings `"é"` and `"\u00e9"`, then runtime payload bytes `c3 a9`, under `binary-utf8` | all three values contain bytes `c3 a9` and compare equal; source spellings remain distinct provenance |
| T07 | member spelling `READY` under declaration node `enum-A` versus node `enum-B` | ill-typed without an explicit common type; never equal from spelling/display text |
| T08 | ordered declaration `enum-A = [READY, DONE]`; compare its members | `READY < DONE`; the same ordering request for an unordered declaration refuses |
| T09 | Change enum owner/case/order while retaining old declaration/member node keys; then recompute the keys and compare old/new values | stale keys return `refused { code: invalid_semantic_graph }`; recomputed keys form a new enum identity and comparison is `refused { code: ill_typed }` without an explicit mapping |
| T10 | equal payload bytes typed once as `nfc` and once as `binary-utf8` | ill-typed without an explicit common-profile conversion; not false |
| T11 | Run T02's NFC comparison with `ScalarLimitsV1 { integer_bits: 0, decimal_digits: 0, scale_expansion: 0, text_input_bytes: 5, text_scalars: 3, normalized_scalars: 2, unit_edges: 0, value_occurrences: 2, work_units: 6, result_units: 1 }`; then deny its second `text.normalize-output` and, separately, `text.result-retain` | the exact tuple succeeds; either denied named charge is incomplete and exposes no truncated text |
| T12 | Compare `a` with `b` under a scalar profile; compare valid UTF-8 payload bytes `7f` (U+007F) with `c2 80` (U+0080) under `binary-utf8` | `a < b`; `7f` is less than `c2 80` by unsigned-byte lexicographic order |

`enum-A`, `enum-B` and `member-A` are distinct opaque I04 semantic-graph node
keys supplied by the fixture, not hashes manufactured by the evaluator. Repeat
lexicographic comparisons on normalized scalar values and unsigned bytes to
prove the profile selects the ordering domain.

Generate bounded valid scalar sequences and their NFC/NFD/NFKC/NFKD forms,
valid UTF-8 payloads, and ordered/unordered enums with stable I04 keys. The
oracle is Unicode 17 normalization plus scalar or unsigned-byte lexicographic
comparison, or declaration/member identity for enums. Shrink by scalar count,
code point, byte count and member count while preserving normalization/profile
and identity preconditions. Mutate bounds, UTF-8 validity, owner, declaration,
case, order and each applicable named accounting charge independently.

## Expected Results

Every vector returns the exact equality/order result or typed
ill-typed/refused/incomplete disposition in the table. Profile, Unicode table
identity, declaration/member identity and original input provenance remain
available. Invalid UTF-8, enum key/content substitution and exhaustion produce no replacement
text or enum value and do not affect independent siblings.
