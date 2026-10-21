---
id: SR-12289
title: 'Gap analysis of PR #1170: admitted union keys and member ids (QSL-945, FR-396-AC-6), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@f3eb22fe0; git diff origin/main...f3eb22fe0 (PR #1170): FR-396, TC-1090, qsl-replay/src/admitted.rs, qsl-replay/tests/admitted_composite_*.rs, qsl-replay/src/execute/tests/composite.rs; ticket QSL-945'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-396
  type: reviews
---

## Summary

Ticket: QSL-945. Planless gap analysis of the QSL-945 acceptance checks (ticket text treated as data) and FR-396-AC-1..AC-6 against the tagged tests and the code at f3eb22fe0. Plan completion: not assessed.

| Check | Result |
| --- | --- |
| QSL-945 #1: a root-only consumer gets the union key and member ids with no decoding or hashing | Partial. It can read them, but it cannot seal or admit a union with them without retyping a `VariantId` into a `NodeKey` (FND-001; code-review SR-12288 FND-001). |
| QSL-945 #2: same-shaped unions have distinct keys; repeated lookups are stable | Met: `union_members_are_the_producers_minted_variants_in_declared_order` (Shape/Twin, recompile). |
| QSL-945 #3: foreign and unknown ids refuse typed; enum/record/tuple unchanged | Met: `a_dependencys_composite_is_foreign` (Pick), `composite_lookups_refuse_unknown_and_wrong_kind_ids`, AC-4 tests. |
| QSL-945 #4: titan:kani:plan confirms CG uses it | Not verifiable here; open. |
| FR-396-AC-6 members equal the producer's | Met. The oracle mints FR-441 ids independently and checks them against the sealed bindings. |
| FR-396-AC-6 witness node id replays, kernel key refuses | Met: `admitted_union_node_is_what_replay_converts_a_union_witness_against`. |
| FR-396-AC-6 consumer declares a closed environment | Met as written, but the AC is weak (FND-001). |

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-396-AC-2 | spec/functional/FR-396-*.md | examined |
| FR-396-AC-3 | spec/functional/FR-396-*.md | examined |
| FR-396-AC-6 | spec/functional/FR-396-*.md | examined |
| TC-1090 step 6 | spec/test-cases/TC-1090-*.md | examined |
| union_members / composite_table | qsl-replay/src/admitted.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The AC-6 consumer test cannot fail for the capability CG needs. `a_consumer_gets_a_unions_key_and_member_ids_from_the_context` builds a `TypeEnvironment` whose union is unsealed (`UnionMemberDeclaration::resolved`) and never constructs or admits a union value. The probe measured that such an environment admits no union value at all. The record/tuple AC-1 test, by contrast, builds and admits values. The test does not use the facade's `VariantId`s, so wrong or swapped ids would still pass it. | qsl-replay/tests/admitted_composite_root_only.rs:110-153 |
| FND-002 | low | Part of the payload oracle restates the facade. `spine_members` reads `member.positions()` from the same `TypeEnvironment` declaration that `union_members` copies. The literal assertions `[Composite(left.key()), Boolean]` and `[Composite(pair.key()), Boolean]` make up for it for one member each. Nullary members and `Integer` positions rely on the restated oracle. | qsl-replay/tests/admitted_composite_facade.rs:250-280 |
| FND-003 | low | Missing case: no test has a union payload whose position is another union or a tuple inside the facade/replay comparison (the root-only file has a tuple payload only). The FR's Outputs claim "a record, tuple or union position is `ValueType::Composite` of that declaration's kernel key", and for a union position that is `key()`, not the settled `node()`. That is the case most likely to confuse, and no test pins it. | qsl-replay/tests/admitted_composite_facade.rs:28-36 |

## Verdict

Not mergeable on FND-001, the same defect as SR-12288 FND-001, seen from the test side. Checks 2 and 3 are met with independent oracles. Check 4 needs the Kani lead's confirmation after the fix.
