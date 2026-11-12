---
id: SR-2026111117463800000003
title: "base review of the Text size lowering (FR-093-AC-63, TC-1409)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (Size rows, FR-093-AC-63); spec/test-cases/TC-1409-text-size-lowers-to-its-text-identity.md; PR #2685"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1409
    type: reviews
---

## Summary

Ticket: IR-1493. Base checklist over the PR diff: the split `Size` collection row,
the new `Size` over `Text[min, max; profile]` lowering row, FR-093-AC-63 and TC-1409.
The Text row matches the `quire.op.text.size` entry of
quire-verification-contracts `contracts/checked-operation-catalog-v1.json`
(origin/main 1bf72e6, read as reference only): operator `call`, operands `["text"]`,
result `integer`, laws `["text_profile"]`, mode `text_profile`, member `null`,
leaves `null`, no constraints. AC-63 is testable and can fail on node shape,
identity, law, mode, result type and refusal; its evaluation clause is weak. No files
are copied between repos (QSpec FR-141 and the catalog are cited, not reproduced),
and no hash, digest or pin antipattern is introduced. `quire validate` passes on both
changed files. Criterion-strength analysis (Jev) was not run: no Jev client is
installed on this machine; its can-fail judgment is folded into this base review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-63's evaluation inputs `""` and `"abc"` cannot fail an implementation that counts UTF-8 bytes or pre-normalization scalars instead of QSpec FR-141's scalar count after the profile's normalization; add an input such as `Text("é")` under `nfc` (retained `"é"`, size `1`; 2 raw scalars, 2 bytes) | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1101; spec/test-cases/TC-1409-text-size-lowers-to-its-text-identity.md:26 |
| FND-002 | low | AC-63's verification cell reads `Test` with no TC id, while every neighbouring FR-093 AC names its TC (`Test (TC-415)` etc.); it should read `Test (TC-1409)` | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1101 |

## Verdict

Mergeable after FND-001 (one extra evaluation input in AC-63 and TC-1409 step 2) and
FND-002 (one-word trace fix). The lowering row, typing rule (`Int[min, max]` from
`text_bounds`, sound because QSpec FR-141 applies the declared bounds to the same
scalar length), refusal (`ill_typed`/`type-mismatch`, matching the checker's existing
`Size` refusal), and the String exclusion are consistent with the catalog and with
FR-093's existing text rows (`Order`, `Equality`, `ConvertText`).
