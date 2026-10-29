---
id: SR-612
title: "Base review of the QSL-212 recursive text-leaf list and the Pre ownership change"
type: SpecReview
analysis: base
scope: "Commit 2a931a10: FR-093 (Application nodes leaf text, the Pre paragraph, Text leaves, Recursive text-leaf vectors T13, T14, G16 to G21, S4, S5, P10 to P16, E14 to E17, AC-8, AC-10, AC-11, Dependencies, Status), FR-094 (postcondition paragraph, Status), ADR-013 QC-24, TC-415 steps 3, 8 and 9, and the spec.md and tests.md index rows"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-415
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The new IDs are well formed and unused before this commit: FR-093-AC-10,
FR-093-AC-11, and vectors T13, T14, G16 to G21, S4, S5, P10 to P16 and E14 to
E17. They continue the numbering FR-092 and FR-094 share: FR-092 ends at T12,
G15 and E13, and FR-094 at S3 and P9. TC-415's scope, the tests.md row and
paragraph, and the spec.md FR-093 row all name AC-10 and AC-11. Every AC has a
TC.

The vectors were recomputed independently. A throwaway script rebuilt every
preimage from the FR-092 and FR-093 rules, using RFC 8785 JCS and SHA-256. It
did not start from the spec's bytes. It covered T13, T14, S4, S5, P10 to P16,
the two groups G16 and G17 and G18 to G21, and E14 to E17 with leaves from its
own walk of rules 1 to 6. All 21 preimages match byte for byte. Their keys
match both the per-vector `Key:` lines and the summary table. The anonymous
and full signatures, ordinals and group digests of both groups match.
Declaring the A/B group in reverse order gives the same keys. As controls,
the same script reproduces FR-092's T1 to T3 and G2 and G3 (keys, signatures
and group digest), and the `sha256` of QSpec's `structural-eq-record` and
`collection-contains` operation vectors at `e72756f`. Its walk gives those
vectors' leaf paths (`[field:name]` and `[]`).

AC-8 now matches what the A4b branch tests: the model-row test in
`qsl-semantics/src/check/lowering/model/tests.rs` covers `Attribute`,
`AllInstances`, `Lookup` and `Dispatch`, and says a `Pre` is checked only in a
standalone postcondition. FR-094 vectors E4 to E9 exist. The findings are one
coverage gap left by that change, one gap in the new AC edge cases, and three
editorial defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `Pre` row has no AC after the change. The old FR-093-AC-8 was the only AC that exercised it, and AC-3 excludes it. The row still states a SHALL-level lowering (`quire.op.state.pre` over `[ref(e)]`), and the new paragraph says `ProtocolClause` "builds each `Pre` read inside a postcondition by this row". A lowering arm for `NodeKind::Pre` exists at `lowering.rs:2417`, but nothing verifies it, and FR-093's Status does not say who will. Fix: add one sentence to FR-093's Status: "No FR-093 AC backs the `Pre` row; the `ProtocolClause` postcondition lowering backs it. Remaining work: #218." Add the same clause to the tests.md FR-093 paragraph. | FR-093:162, :190-199, :554 (AC-8), AC-3; FR-093 Status; `qsl-156-a4b` `qsl-semantics/src/check/lowering.rs:2417` |
| FND-002 | low | AC-10 and AC-11 leave three walk shapes untested. None of the vectors or ACs covers a recursion leaf reached through a collection rather than an `Option`, two walks of one recursive composite on sibling fields (rule 4 must close `C` after each field), or a recursion leaf whose prefix is a `field` segment. Rule 2's collection arm, rule 4's close step and a nonzero `d` behind a field segment are therefore untested. Fix: add to FR-093-AC-11, and to TC-415 step 9 and its expected result, `record Tree2 { label: Text[0, 8; binary-utf8]; kids: Sequence<Tree2>[0, 3]; }`, whose list is `field:label`, then `field:kids`, `inner`, `recursion:0`, and `record Two { x: Node; y: Node; }`, whose list is `field:x`, `field:label`; `field:x`, `field:next`, `inner`, `recursion:1`; `field:y`, `field:label`; `field:y`, `field:next`, `inner`, `recursion:1`. | FR-093:209-225, :556-557; TC-415 step 9 |
| FND-003 | low | The line `**T13**: ...` directly follows the last row of the key table (the E17 row) with no blank line. GitHub-flavoured Markdown then reads it as one more table row, so the T13 heading renders inside the table. Fix: insert a blank line between the E17 key row and `**T13**`. | FR-093:351-352 |
| FND-004 | low | The history in FR-094's Status is wrong. It says "QSL-212 placed an operation's postconditions with `ProtocolClause`". ADR-012 §4.3 has placed `Pre`, and with it postconditions, with `ProtocolClause` (#218) since ARCH-11 (3df64ff4). QSL-212 corrected FR-093-AC-8, which QSL-208 had written against that record. FR-093's Status line "the `Pre` row's owner specified under QSL-212" has the same fault. Fix: in FR-094's Status, write "ADR-012 §4.3 places an operation's postconditions with `ProtocolClause`, so the clause functions this requirement keys are exactly ...". In FR-093's Status, write "the text-leaf walk and its recursion leaf specified under QSL-212" and leave out the owner clause. | FR-094:722-725; FR-093:596-597; ADR-012 §4.3 |
| FND-005 | low | QC-24's "Blocked work" cell still says "QSL-156 A4b keys by the QSL proposal now". The row now also covers the text-leaf list and the optional-field `inner`, which FR-093's Status says A4b does not yet follow. Fix: change the cell to "QSL-156 A4b keys by the QSL proposal now, except the text-leaf walk (FR-093 Status); IR's reading of function and parameter nodes, and IR-242's in-group ordinals". | ADR-013 QC-24 (line 1098); FR-093 Status |

## Resolution

All findings are fixed. FND-001: FR-093 Status and the tests.md FR-093 paragraph say no FR-093 AC backs the `Pre` row, the `ProtocolClause` postcondition lowering backs it, Remaining work: #218. FND-002: FR-093-AC-11 and TC-415 step 9 add `Tree2` (recursion through a collection) and `Two` (two sibling walks of `Node`, each ending in `recursion:1`). FND-003: a blank line separates the key table from `**T13**`. FND-004: FR-094 Status credits ADR-012 §4.3, and FR-093 Status drops the owner clause. FND-005: QC-24's Blocks cell says A4b keys by the proposal except the text-leaf walk.
