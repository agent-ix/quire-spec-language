---
id: SR-609
title: "Base review of the QSL-211 recursion-group order and the FR-092 to FR-094 fixture corrections"
type: SpecReview
analysis: base
scope: "Commit 7be2d02a: FR-092 (Recursion groups, the group order, vectors L5, L6, E11 to E13, G1 to G15, AC-7, AC-11, AC-12), FR-093 (quantity.convert mode, AC-4), FR-094 (C3, P9, AC-5, AC-6), FR-065-AC-7, ADR-013 QC-24 and QC-26, and the TC-413 to TC-419 edits"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-413
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-414
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-415
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-417
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-418
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-419
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The new IDs are well formed and unused before this commit: FR-092-AC-11,
FR-092-AC-12, and vectors L5, L6, E11 to E13, G1 to G15 and FR-094 P9. Each
new AC has a TC: AC-11 and AC-12 are in TC-413's Scope line and steps 9 and
10.

A scratch script re-serialized and re-hashed every vector block. All 50
FR-092 blocks and all 39 FR-094 blocks equal their own RFC 8785 bytes, and
each block's SHA-256 equals its `Key:` line and its summary-table row. The
existing vectors T1 to T4, P4, L1, F3, E1, P7 and C1 still reproduce.

A second script rebuilt the groups from the prose rules alone, without
reading the stated preimages. It built P4, L1, L5, L6 and E11 to E13 from
the Parameter, literal and application rules, and all seven match. It then
built each group's members with placeholders and ran both refinement passes,
the order, the classes, the group-local preimages and the group digest.
Every anonymous signature, full signature, ordinal, group digest and key of
G1 to G13 matches. G14 and G15 match too, but with their labels swapped
(FND-001). Re-keying each group with its members supplied in reverse order
gives the same keys. The anonymous pass stops at round 1 for every group. The
full pass stops at round 3 for `ping`/`pong` and at round 1 for the others.

FR-094's corrected C3 is C1 with owner version `2.0.0` and receiver P9. P9 is
P7 with semantic type R2, and R2 references M2. Both rebuild exactly.

The fixtures check against the A4b checker
(`qsl-156-a4b/qsl-semantics/src/check/`):

- An integer literal is typed `Integer` whatever the hint (`infer_form`).
- `x - 1` is `Integer` (`arithmetic`).
- `x > 0` goes through `ordering`, which does not coerce.
- The call argument goes through `check_as`, which adds the narrowing
  `Coerce` into `Int[0, 9]` (E13).
- `fm`'s `flatMap` types to `Sequence<Int[0, 9]>[0, 6]` (`query` and
  `flatten`: 3 × 2).
- `facts` refuses every quantity division, and QSpec FR-146 lines 218 to 220
  state the same rule, so the FR-094-AC-6 restatement is right.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The G14 and G15 labels are swapped. G12, the conditional in `ping`, holds `group_reference` ordinal 5 (G15), and G15's call names ordinal 3, `pong`. So the bytes and key labelled G15 "`ping(x - 1)`, in `pong`" are the call to `pong` inside `ping`. The bytes and key labelled G14 are the call to `ping` inside `pong`. The independent rebuild gives ordinal 4 to the call inside `pong` and ordinal 5 to the call inside `ping`, which matches the bytes. A test that keys "the call in `ping`" and compares it with the G14 row fails. Fix: relabel G14 as "`ping(x - 1)`, in `pong`, ordinal 4" and G15 as "`pong(x - 1)`, in `ping`, ordinal 5". Change the summary table, the two headings and the G10 to G15 signature-table rows. The bytes and keys stay as they are. | FR-092:419-420, :833, :841, :887-888 |
| FND-002 | medium | FR-065-AC-7 now says the hook mints no identity and `PackageDeclarations::check` keys `f` to F1. TC-380 still expects "the checked declaration; its identity equals the independently minted identity for `f`". FR-065's Status still says AC-7 is backed by two tests written against the old wording. Fix: rewrite TC-380 steps 1 and 2 and its step-2 expected result. The hook admits `f`, and a `PackageDeclarations::check` of the unit under (`a`, `u`) gives `f` the key F1. In FR-065 Status, mark the F1 half as unbacked until A4b merges. | TC-380:16, :23, :30-31; FR-065:210, :373-379 |
| FND-003 | medium | FR-088's Status credits AC-7's within-package half to the behavior that FR-092-AC-12 now calls wrong: "a declared type's node id is its declaration's key (TC-259 step 4)". That key is the caller's `CompositeDeclaration` handle, and TC-259's `composite_declaration_becomes_a_real_checked_type_node` asserts it. The diff updates FR-092's Status but not FR-088's. Fix: in FR-088 Status, mark AC-7's within-package half as unbacked, owned by QSL-156 A4b, until the checked type node takes its FR-092 key. | FR-088:257-259; FR-092:281-291, :912, :964-967 |
| FND-004 | low | Group-order step 5 (members with equal full signatures form one class and one node, and `size` counts classes) has no vector or AC. Every vector group has one member per class. Fix: add a group whose body calls the function twice with the same argument, for example `if x > 0 then f(x - 1) and f(x - 1) else true` (an `and` of two identical calls). Assert one call node, `size` 4 (function, conditional, `and`, call), and the `and` node's two arguments as the same `group_reference` ordinal. Add it to AC-11 and TC-413 step 9. | FR-092:191-194, :911; TC-413 step 9 |
| FND-005 | low | FR-092-AC-7 says the conditionals of `f` and `g` "both key to G5", and then that no member of either group gets a key. These two claims contradict each other. TC-413 says it correctly: "each conditional's preimage alone keys to G5". Fix: in AC-7, write "each conditional's preimage hashes to G5, and the package refuses …". | FR-092:907; TC-413 step 5 expected result |
| FND-006 | low | FR-093 Status says TC-415 backs AC-4, and TC-415 Status says the tests back steps 1 to 7. Neither test checks the new AC-4 fixture `fm`: the A4b test flat-maps `s` over itself (`lowering/tests.rs` `conversions_are_classified_and_flat_map_builds_one_node`). TC-415 Status says so itself, but the "backs AC-1 to AC-6" line claims more. Fix: in FR-093 Status, list AC-4's `fm` half as unbacked. | FR-093:253-256; TC-415:74 |
| FND-007 | low | Some new text states what a thing is not, when it could state what it is. Examples: "It is not a node id"; "No preimage, checked-graph node, checked type node … holds a handle"; "The hook mints no identity". Fix: state the positive rule. For example: "a handle selects a declaration in the `TypeEnvironment`; the node id is the FR-092 key", and "`PackageDeclarations::check` mints `f`'s identity". AC-12 keeps its concrete assertion that neither supplied key appears. | FR-092:281-288; FR-065:210 |

## Resolution

All findings are fixed. FND-001: G14 is relabelled `ping(x - 1)`, in `pong`, and G15 `pong(x - 1)`, in `ping`, in the table, the headings and the signature table; bytes and keys are unchanged. FND-002: TC-380 steps 2 and its expected result now read the F1 key through `PackageDeclarations::check`, and FR-065 Status records the typing half as backed and the F1 half as asserted by QSL-156 A4b, pending merge. FND-003: FR-088 Status records that TC-259 step 4 asserts the caller key on main and that FR-092-AC-12 makes the id the FR-092 key. FND-004: AC-11 adds `h`, whose two identical calls are one node in a group of size 4. FND-005: AC-7 says the conditionals' preimages hash to G5. FND-006: FR-093 and TC-415 Status list AC-4's `fm` fixture as unbacked. FND-007: the handle paragraph and FR-065-AC-7 state the positive rule.
