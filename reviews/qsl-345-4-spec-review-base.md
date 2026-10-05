---
id: SR-1312
title: "Spec review of quire-spec-language PR #637: ADR-012 §15.4/§15.7 domain keys and FR-121 field and population selections"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d764bb72d0cb411f62b266d8455aa8e84b4602a9; spec/decisions/ADR-012-*.md (§15.4, §15.7, amendment line), ADR-014-*.md §4, ADR-016-*.md, spec/functional/FR-104-*.md, FR-121-*.md, FR-097-*.md (context), spec/test-cases/TC-516, TC-461, TC-436 (context), spec/spec.md, spec/tests.md"
review_set: subset
---
# Spec review of quire-spec-language PR #637

## Summary

Ticket: QSL-345 (item 4). PR: quire-spec-language#637. The authority is the
terra:QSL:plan rulings on QSL-345 dated 2026-10-01: the item-4 field-key rule
(comment fd315c8f) and option (c), the explicit subject enum (comment 19d2b0d4).

Ruling conformance, all matching:
- The subject enum is `Node{node, path}` for parameters and fields and
  `Population{member_type: WireNodeId, ordinal: u32}`. It is stated in ADR-014 §4
  and ADR-012 §15.4/§15.7, and matches ruling 19d2b0d4.
- §15.4 keys a field under the `object_type` node of the type that *declares*
  it, never the subtype it is named through. The ordinal is among that type's
  own field declarations in ascending field-name UTF-8 byte order, then the
  child-index path into the field's type. This matches ruling fd315c8f item 4
  and ruling 19d2b0d4's `[field ordinal, ...path into the field's type]`.
- §15.7 keeps the sorted population ordinal (ascending `DeclarationKey`,
  package then node, UTF-8) under `Population{member_type, ordinal}`. FR-104
  keeps the canonical member type. This matches "The ordinal stays §15.7's
  sorted population ordinal."
- FR-121's field and population Behavior bullets and AC-18..22 restate the
  rule without adding to it. ADR-016 and TC-461 are updated to the new key
  shape.

## Verdict

The ADR amendments match the rulings exactly. There are three bookkeeping
findings: a mis-dated amendment line, a stale spec.md row, and FR-097-AC-1 and
TC-436 still describing the struct-shaped key. None blocks the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | ADR-012's amendment line reads "Amended 2026-09-30", but both rulings it implements are dated 2026-10-01 on QSL-345 (field rule at 03:22Z, subject enum at 04:03Z). Change the date to 2026-10-01. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:53-55 |
| FND-002 | low | The FR-121 row in spec.md adds "a state field's ADR-012 §15.4 domain key" but leaves out the population selection (§15.7). It still says "TC-516 passed locally for AC-1 to AC-14; Remaining work (Linear QSL-352): AC-15". AC-15 to AC-22 have passing traced tests, and QSL-352 is Done. Restate it as "or a state field's or population's ADR-012 §15.4/§15.7 domain key ... TC-516 passed locally for AC-1 to AC-22" and drop the remaining-work clause. | spec/spec.md:1033 |
| FND-003 | medium | FR-097-AC-1 ("`DomainKey`s order by node, then path") and TC-436 step 2 (three `(node, path)` keys) still describe the old struct. A `Population` key has neither a node nor a path, so the AC no longer says how population keys order or whether they can equal a node key. The code orders every `Node` before every `Population` (derived `Ord`), and a new test asserts that order under this AC. Amend FR-097-AC-1 and TC-436 to "Node keys order by node, then path; every Node key precedes every Population key, and the two never compare equal", and retag per SR-1311 FND-002. | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:69; spec/test-cases/TC-436-proof-bound-and-interval-key-constructors-refuse-empty-ranges.md:9,20 |
