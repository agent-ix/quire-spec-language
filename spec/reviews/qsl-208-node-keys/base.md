---
id: SR-523
title: "Base review of the QSL-208 node-key and expression-lowering requirements"
type: SpecReview
analysis: base
scope: "Commit d96591be: FR-092, FR-093, TC-413 to TC-416, and the FR-065, FR-091, ADR-011, ADR-013, TC-163, spec.md, tests.md and US-005 edits"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-413
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-414
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-415
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The IDs are well formed and unused before this commit: FR-092, FR-093,
TC-413 to TC-416, `{FR}-AC-N` and `{FR}-CON-N`. `quire validate` passes on the
six new files. Every AC and CON has a TC, and the `tests.md` rows list the
same scope as each TC's Scope line. FR-092 has 8 ACs, all covered by TC-413 and
TC-414, plus CON-2 covered by TC-413. FR-093 has 7 ACs: AC-1 to AC-6 are in
TC-415 and AC-7 is in TC-416, with CON-1 in TC-415 and CON-2 in TC-416. US-005
and `spec.md` index both new FRs.

The golden vectors were recomputed with a scratch script: SHA-256 over the
listed bytes, after checking that each block equals its own RFC 8785
re-serialization. All 20 blocks (T1 to T8, D1, D2, P1 to P3, L1, L2, F1, E1,
F2, E2, E3) are canonical, each hashes to its `Key:` line and to its summary
table row, and every digest a block cites is the key of the vector it names.
T4, D1, D2, P1 to P3 and L1 cite T1, T2 and T3. F1 cites L1. E1 cites P1 and
P2. F2 cites E1. E2 cites F2 and L1. E3 cites P1 and P3.

The gaps are in criteria that the checker cannot meet, error conditions with
no code, and node forms with no criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-093-AC-4's `c1` cannot pass. `coerce` returns the node unchanged when the target `Int[..]` contains the source range, so `x: Int[0, 9]` returned as `Int[0, 10]` produces no `Coerce` node. `c1`'s body is then a `Local` read: a reference to `x`'s parameter node, not a `convert` node. For the same reason, the `Coerce` row's `quire.op.numeric.convert` arm can never be reached: `Coerce` is built only for a non-containing target, and FR-149 classifies that as range narrowing. Fix: change `c1` to `convert<Int[0, 10]>(x)`, which checks to `ConvertScalar` and `numeric.convert`. Map `Coerce` to `quire.op.numeric.narrow` alone. State that an admitted widening builds no node. | FR-093:117, :178; TC-415 step 4; `qsl-semantics/src/check/check.rs:494-515` |
| FND-002 | medium | FR-093-AC-3 requires "a fixture function whose body holds that checked node" for every row of the application table. That includes `Pre`, which in a function body is refused as `wrong_snapshot`/`forbidden-pre-read` (FR-091-OQ-1). The `Attribute`, `AllInstances`, `Lookup` and `Dispatch` rows are also `StateModel` constructs. TC-415 step 3 hands those rows to "that family's own tests" and names none, so they have no backing test. Fix: limit AC-3 to the `Value`-family rows, and name the TC or clause context that backs each remaining row. | FR-093:137-141, :177; TC-415 step 3; FR-091 Rulings OQ-1 |
| FND-003 | medium | Error conditions have no catalog code. FR-092 has a "depth refusal" and says that an empty qualified name, binding name or `semantic_form` "refuses and yields no key". FR-093 has a missing law refusing "with a typed cause". FR-092-AC-7 and FR-093-AC-6 check that a refusal happens but not its code. Fix: name each code and cause as FR-091's Catalog codes table does, for example `resource_exhausted` for the depth refusal, and an `invalid_package` cause for a missing law, matching the reader's `operation-law-missing`. Assert the code in AC-7 and AC-6. | FR-092:106-109, :414; FR-093:148-151, :180 |
| FND-004 | medium | Several node forms have no criterion and no vector: a function in a recursion group (`recursive_function`, the `recursion` member, `group_reference`), a function with a `decreases` binding, a tuple type, an `Option` field, a `Rational[..]` and a `Decimal[..]` bounded type, and a rational literal spelled `"n/d"`. FR-092's rules for these are therefore untested. The recursion case also has a key collision (SR-525 FND-001). Fix: add vectors and AC rows for at least one recursive function with a measure, one tuple, one record with an optional field, and one rational literal. | FR-092:85, :94-104, :184-195, :120-131 |
| FND-005 | low | The `spec.md` rows read "implemented by QSL-156 A4b". Both FRs' Status sections say "Not implemented". Fix: write "to be implemented by QSL-156 A4b", as the FR-091 row does ("not yet implemented"). | spec.md:472-473 |
| FND-006 | low | FR-092 cites ADR-012 §5.1 (in CON-2) and FR-093 cites it (in CON-1), but neither frontmatter lists ADR-012 among its relationships. Fix: add `ADR-012` as `depends_on` to both. | FR-092:1-17, :402; FR-093:1-16, :168 |
| FND-007 | low | The parameter-node body names its literals ("the name as a text literal", "the binder's level as an integer literal") but does not give their `type`. The type-node rule at :114 covers only type-node bodies. The vectors use T3 and T2. Fix: say that a parameter node's `name` literal is typed by the builtin text scalar node and its `level` literal by the `Integer` node. | FR-092:114-116, :163-164 |

## Resolution

All findings are fixed in the PR. FND-001: `c1` now asserts no convert node,
`Coerce` lowers only to `quire.op.numeric.narrow`, and `c3` covers
`quire.op.numeric.convert`. FND-002: FR-093-AC-3 covers the `Value` rows, and
FR-093-AC-8 covers the `Attribute`, `AllInstances`, `Lookup`, `Dispatch` and
`Pre` rows over the FR-153 and FR-151 fixtures. FND-003: the depth refusal is
`resource_exhausted`/`insufficient-next-charge`, the missing law
`missing_declaration`/`missing-selection`, the recursion collision
`unknown_required_feature`/`unsupported-feature`, and an empty name an
internal fault; AC-7 and AC-6 assert the codes. FND-004: vectors T9 to T12,
D3 to D5, P4, L3 and F3 and FR-092-AC-9 and AC-10; a recursive function's key
is FR-092-OQ-1. FND-005 to FND-007: applied as proposed.
