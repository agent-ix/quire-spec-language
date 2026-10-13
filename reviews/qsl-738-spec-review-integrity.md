---
id: SR-8724
title: "Integrity review of FR-360 variant-typed members (QSL-738)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@0e8fe78c290748d5e81ba2318514920aa846a9af; spec/functional/FR-360-*.md; spec/functional/FR-056-*.md; spec/tests.md; spec/spec.md; open PR quire-spec-language#808"
review_set: subset
date: 2026-10-13
---
# Integrity review of FR-360 variant-typed members (QSL-738)

## Summary

Ticket: QSL-738. PR: quire-spec-language#809 (spec only). Consistency, id uniqueness and index checks. FR-360 and TC-1006..TC-1010 are unique across all remote branches and open PRs; index rows are present and make check-index-completeness passes; quire validate of the changed files passes.

## Scope examined

- `spec/spec.md FR-360 row` (examined)
- `spec/tests.md TC-1006..1010 rows` (examined)
- `FR-056-AC-28` (examined)
- `FR-360 item 7` (examined)
- `FR-360 cross-repo 2` (examined)
- `FR-360` (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Open PR #808 contradicts this PR's meaning: its FR-056-AC-33 and TC-1005 still list 'a field typed by a variant type' as a malformed declaration. #808's FR-056-AC-33, TC-1005 step 2 and its test `a_field_typed_by_a_variant_type_keeps_its_origin` assert that the refusal FR-056-AC-28 now removes is still raised. A trial merge of the two heads also gives textual conflicts in the FR-056 AC table (AC-31/32 against the new AC-33 row) and in spec/tests.md (the TC-1004 row followed by the TC-1005 and TC-1006..1010 rows). Whichever PR lands second must remove the variant-field item from AC-33 and TC-1005 and drop that test. The AC2 code slice must delete it in any case. | spec/functional/FR-056-*.md AC table; PR #808 FR-056-AC-33, TC-1005 step 2, qsl-semantics/tests/it/tc235_origin_retained.rs `a_field_typed_by_a_variant_type_keeps_its_origin` |
| FND-002 | low | Unqualified `FR-141-AC-2` and `FR-141-AC-5` resolve to QSL's own FR-141, which is about deciding one concrete step and has an AC-2 and an AC-5. The meaning is QSpec FR-141 (enumerations). The relationship edge is correct, but the prose and the Dependencies sentence ('QSpec FR-208 ... and FR-141 the enumeration equality') are ambiguous. Write 'QSpec FR-141-AC-2/AC-5'. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:118, :124, :234 |
| FND-003 | low | QSL now has its own FR-360, so existing unqualified 'FR-360' references that mean QSpec FR-360 (the infinite-trace result labels) have become ambiguous. ADR-014 line 68 ('FR-360 maps proved, refuted ...') and ADR-023 HV-5 ('FR-360 `failed`') mean QSpec FR-360. Qualify them as 'QSpec FR-360' in this PR. | spec/decisions/ADR-014-*.md:68; spec/decisions/ADR-023-*.md:278 |
| FND-004 | low | Cross-repo requirement 2 reproduces the private QSpec FR-208's section and bullet wording in a public repo. The brief requires citing QSpec by id only. It quotes the 'Field types' bullet, the 'Variant types' section and the meaning-row text. Reduce it to the ids (QSpec FR-208, FR-208-AC-15) and the change requested, and keep the detail in Linear. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:177-182 |

## Verdict

Findings. The ids are unique and the indexes are complete. One open PR, #808, contradicts this PR in meaning, and some unqualified ids now point at the wrong requirement.
