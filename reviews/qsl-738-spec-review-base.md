---
id: SR-8723
title: "Base spec review of FR-360 variant-typed members (QSL-738)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0e8fe78c290748d5e81ba2318514920aa846a9af; spec/functional/FR-360-*.md; spec/functional/FR-056-*.md; spec/test-cases/TC-997, TC-1004, TC-1006..TC-1010; spec/tests.md; spec/spec.md"
review_set: subset
date: 2026-10-13
---
# Base spec review of FR-360 variant-typed members (QSL-738)

## Summary

Ticket: QSL-738. PR: quire-spec-language#809 (spec only). Checklist review of FR-360's eleven target-design items, eleven ACs and the FR-056 rewrite against FR-056, FR-317, FR-359, FR-106, FR-070, ADR-012 section 16, QSpec FR-141/FR-144/FR-208/FR-322 and the qsl-semantics checker.

## Scope examined

- `FR-360` (examined)
- `FR-360 item 1` (examined)
- `FR-360 item 2` (examined)
- `FR-360 item 3` (examined)
- `FR-360 item 4` (examined)
- `FR-360 item 5` (examined)
- `FR-360 item 6` (examined)
- `FR-360 item 7` (examined)
- `FR-360 item 8` (examined)
- `FR-360 item 9` (examined)
- `FR-360 item 10` (examined)
- `FR-360 item 11` (examined)
- `FR-360-AC-1` (examined)
- `FR-360-AC-2` (examined)
- `FR-360-AC-3` (examined)
- `FR-360-AC-4` (examined)
- `FR-360-AC-5` (examined)
- `FR-360-AC-6` (examined)
- `FR-360-AC-7` (examined)
- `FR-360-AC-8` (examined)
- `FR-360-AC-9` (examined)
- `FR-360-AC-10` (examined)
- `FR-360-AC-11` (examined)
- `FR-360 Behavior` (examined)
- `FR-056-AC-27` (examined)
- `FR-056-AC-28` (examined)
- `FR-056-AC-31` (examined)
- `FR-056-AC-32` (examined)
- `FR-056 variant typeRef paragraph` (examined)
- `FR-056 clause paragraph` (examined)
- `FR-056 signature paragraph` (examined)
- `FR-056 type environment paragraph` (examined)
- `FR-056-AC-9` (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An unknown variant type name refuses ill_typed/type-mismatch, which contradicts FR-056-AC-9 and FR-317. Item 4 and AC-5 (`Config::Nope::Gold`) refuse a name that resolves to no type of the alias with `ill_typed`/`type-mismatch`. FR-056-AC-9 refuses `Reference<M::Nope>` at the assembler as `missing_declaration` at `M::Nope`. FR-317 says a `q::m` with no candidate is refused 'exactly as an unresolved name is refused'. The existing native-model linker refuses an absent `Model::Enum::Variant` as `MissingDeclaration` (src/linking.rs `ExprKind::EnumValue`). FR-317's type-mismatch rule covers only an undeclared member of a type that resolved. Fix: an unknown type name, in a type position or as the qualifier of a case, refuses `missing_declaration`/`missing-name` at the name. Pin the undeclared-case cause explicitly. FR-317-AC-2 gives `type-mismatch`; the native linker gives `missing_declaration`. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:82-92, :221; FR-056-AC-9; FR-317 Behavior |
| FND-002 | high | Set/Bag order 'ascending effective identity, the order FR-056 gives the type environment declaration' is false and contradicts the enum key. FR-056 declares the cases 'in ascending UTF-8 byte order of their names' and orders the `enum` export members the same way. 'Ascending by effective identity' in FR-056-AC-31 orders the declarations, not the cases. QSpec FR-144 keys an unordered enumeration by the case identifier's bytes and 'never its declaration position' (FR-144-AC-9). The kernel already does this (qsl-eval collection_algebra `c05_enum_keys_order_unordered_members_by_identifier_bytes`). FR-359's declaration-key order applies between values of different declarations. A variant Set holds one declaration. Fix: the order is ascending UTF-8 byte order of case names, which is FR-144's unordered-enum key and FR-056's case order. Update AC-7 and TC-1008 step 4 to match. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:122-128, :223; FR-056 'Effective identity of a variant type'; QSpec FR-144-AC-9 |
| FND-003 | medium | Three refusal causes are left to the implementer: ordering, `case` over a variant, and cross-type `==`. (a) Item 7 and AC-5 say ordering refuses 'with the cause the checker gives an ordering over a native unordered enum'. The checker's `CheckRefusal::from_ill_typed` maps `UnorderedEnumOrdering` to `ill_typed`/`operator-ineligible`, so pin that. (b) Item 5 and AC-5 say a `case` over a variant 'is refused as over an enum'. ADR-012 section 16.2 confirms that no `case` over an enum exists, and section 16.5 and the SumCase row make 'scrutinee not a union' `ill_typed`/`type-mismatch` at the scrutinee, so pin that. (c) AC-6 gives only `ill_typed` for `Config::Level::Gold == Config::Tier::Gold`. The checker maps `DistinctEnumDeclarations` to `type-mismatch`, so pin `ill_typed`/`type-mismatch`. TC-1007 step 3 and TC-1008 step 2 cannot be checked until these are pinned. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:100-102, :122-125, :221, :222; qsl-semantics/src/check/refusal.rs:971-990; ADR-012 section 16.5 |
| FND-004 | medium | Item 11 treats FR-056's effective identity and FR-359 item 9's model-owned node key as one key. They are two different keys. FR-056 keys the type environment declaration and its cases by the FR-081 effective identity. FR-359 item 9 makes a model record's runtime declaration key the QSpec FR-322 'Model-owned members' step 2 node key (the ModelDeclarationNode preimage). Item 7 decides equality by effective identity, and item 9 names the type by its IR node identity. Cross-repo requirement 1 would add a `model`/`variant_type` checked node keyed by the FR-322 preimage, and its case `enum_value` nodes would be keyed by QSpec FR-141's `{declaration_node_id, case}`. FR-360 must say which key a variant value carries at runtime, in the checked IR and in the kernel, and how the two keys are joined. Both keys are alias-independent, so AC-11's claim holds either way, but the citation and the identity rule are wrong. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:113-116, :139, :160-165, :227; FR-359 item 9 |
| FND-005 | medium | No target-design item states how a case literal or a variant-typed read is lowered and emitted, yet AC-8 and TC-1009 assert that the call 'reads back'. FR-360 depends_on FR-093 but has no counterpart to FR-359 item 3, which covers emission as a model-owned node, the lowered term and the IR read-back. It does not name the checked node of a case literal (an `enum_value` whose `semantic_type` is which node?), the node a variant-typed field read projects through, or the read-back verdict. 'Reads back' in AC-8 has no stated oracle. Add an item for lowering and emission and state the read-back verdict in AC-8, including whether it depends on cross-repo requirement 1. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:129-134, :224; FR-359 item 3 |
| FND-006 | medium | The replay causes have no order, so the cause for a value with several defects is left to the implementer. FR-106 reports the first defect in a stated order (FR-106-AC-7), and for records it orders the checks as resolve `type`, then conformance, then exact fields. Item 9 lists `missing-member`, `unknown-member`, `wrong-value-kind` and `invalid-value` without an order. For example, it does not say which cause `{"variant":{"type":"<Level>"}}` gives (missing `case` and the wrong type) or `{"variant":{"type":"<Tier>","case":"Silver","x":1}}` gives. State the order (for example: tag, payload kind, missing, unknown, type, case) and add one multi-defect vector to AC-10. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:141-150, :226; FR-106 check 6 |
| FND-007 | medium | FR-056 admits any case name of at least one character, but item 4 can write a case only as a source identifier. Item 4 writes a case as `Alias::Tier::<exact UTF-8 case name>`. FR-056 and the IR schema admit a case name such as `Gold Plus`, `2x` or a keyword, which the `member-name` production cannot spell. FR-360 does not say whether such a variant type can be named in source, whether its other cases stay writable, or whether intake or the assembler refuses it. State the rule and add a vector for it. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:84-88; FR-056 'Variant types' (case `name` of at least one character) |

## Verdict

Findings. Admission, export shape, signature positions, the replay form and the witness shape are sound and consistent with FR-056, FR-106 and FR-070. Two rules contradict existing requirements (the refusal for an unknown variant type name, and the Set/Bag order), three refusal causes are left to the implementer, two different keys are treated as one, and no item says how a variant value is lowered and emitted although AC-8 asserts read-back.
