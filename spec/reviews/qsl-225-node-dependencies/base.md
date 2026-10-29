---
id: SR-615
title: "Base review of the QSL-225 node dependencies rule and the v2 fixture comparison"
type: SpecReview
analysis: base
scope: "Uncommitted QSL-225 diff on 3c0eea23: FR-093 (Node dependencies, Comparison with QSpec's v2 positive fixtures, AC-12, AC-13, the QSpec FR-322 Dependencies bullet, Status), TC-416 (purpose, scope, steps 6 and 7, expected results, Status), ADR-013 (C-03, QC-27, TK-08), and the spec.md and tests.md rows and paragraph"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The new IDs are well formed and unused before this change: FR-093-AC-12,
FR-093-AC-13 and ADR-013 QC-27. They follow AC-11 and QC-26. TC-416's scope,
the tests.md TC-416 row and FR-093 paragraph, and the spec.md FR-093 row all
name AC-12 and AC-13. Every new AC has a TC. The new requirement sentence
uses SHALL.

The concrete claims were checked against the vectors and fixtures.

- **FR-092 vectors.** Each list in FR-093-AC-12 and TC-416 step 6 was
  rebuilt from the vector preimage bytes, with every in-group
  `group_reference` replaced by the member's key. E1 references P1 (`8380…`)
  and P2 (`5554…`), so it lists P2, P1. F2 lists P2, P1, E1 (`a988…`). E2
  lists L1 (`03a8…`), F2 (`2597…`), P1. T4 lists T2, its `semantic_type`. G6
  lists G4 (`23cd…`, ordinal 1) and E13 (`f1b8…`). G9 lists G8, its
  `semantic_type`. T1, L1 and P1 list none. All of these hold, in ascending
  digest order.
- **QSpec fixtures.** A scratch script applied rules 1 to 5 to every node of
  the five `positive-*.json` fixtures at `proposals/checked-package-v2/fixtures`
  and compared the result with each node's `dependencies`.
  `positive-operation-identities.json` (58 nodes),
  `positive-control-operations.json` (6), `positive-clause-operations.json`
  (10) and `positive-nominal-identities.json` (4) match at every node.
  `positive-all-families.json` (26) differs at exactly `cccc…`
  (`bounded_domain`/`integer_range`: expected `aaaa…`, written `[]`),
  `eeee…` (`expression`/`reference`) and `7070…`
  (`correspondence`/`source_locus`), both expected `dddd…` and written `[]`.
  This is what QC-27 and the FR-093 Dependencies bullet say. In every
  fixture, a node whose `semantic_type` is another node lists that type only
  when it is a `bounded_domain` or an enum value or unit that rule 4 names.
  Rule 3 therefore separates the fixtures' behaviour, and it is not
  vacuous.
- **Placeholders.** Every non-application node in the two compared fixtures
  has a repeated-byte placeholder digest, as the Comparison section says.
- **IR.** At `quire-contract-ir` `c0ba691`, `v2/mod.rs` (about line 1685)
  and `lower.rs` build their edges and closure from `semantic_type`,
  `dependencies` and body targets together, as the Node dependencies
  paragraph says. `identity.rs` joins the `enum_value`, dimension and unit
  `dependencies` exactly, which matches rule 4.

The findings are one dependency gap in AC-13, one coverage gap in AC-12, and
one missing table row.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-13 requires "IR's v2 reader admits each emitted package", but IR's reader at `c0ba691` refuses any emitted package that holds a parameter node. Its closed `ValueForm` (`vocabulary.rs`) has no `parameter`, so `CheckedNodeKind::decode` refuses a `value`/`parameter` node as `invalid_semantic_graph` (`mod.rs` about line 1620). Only a function whose operands are all literals avoids that. `model.all_instances`, `model.lookup` and `model.dispatch_call` cannot: they take `Population`, `Reference` or receiver parameters, and they also need QC-25's model nodes. ADR-013 QC-24 and QC-25 list "IR's reading of function and parameter nodes" and "of model-owned nodes" as blocked work. FR-093's Status and TC-416's Status name only the emitter and the lock law as blockers. Fix: add to TC-416 Status: "Step 7's admission half also waits on IR's v2 reader reading ADR-013 QC-24's `value`/`parameter` and function nodes and QC-25's model nodes. IR's closed `ValueForm` at `c0ba691` has no `parameter`, so it refuses such a package as `invalid_semantic_graph`." In FR-093's Status, after "AC-12, AC-13 and CON-2 (TC-416) are unbacked", add: "AC-13's IR admission also waits on IR reading the QC-24 and QC-25 node shapes." | FR-093-AC-13, FR-093 Status; TC-416 step 7, Status; ADR-013 QC-24, QC-25; `quire-contract-ir` `c0ba691` `v2/vocabulary.rs` `ValueForm`, `v2/mod.rs` ~1620 |
| FND-002 | low | AC-12's rebuilt-list equality covers "every node of the checked package of AC-7" only. TC-416 step 6 rebuilds every node of steps 1 and 5, so the TC is broader than the AC it backs. The pinned lists skip the group members whose lists come from in-body group references, the case the "On the wire" paragraph exists for. Only G6 and G9 are pinned, and G9's list comes from rule 3, not rule 1. Fix: in AC-12, change the first sentence to "For every node of the checked package of AC-7, of the package of the recursive `f`, and of a package holding `record Tree { kids: Sequence<Tree>[0, 3]; }`, ...". Then add "G4 lists G5 and P4; G5 lists L1, E11 and G6; G7 lists G9; G8 lists G7." Add the same four lists to TC-416's Step 6 expected result, and make step 6 rebuild every node of the `Tree` emission. The digest order was checked: G5 `3d8a…` < P4 `ebe6…`; L1 `03a8…` < E11 `68fb…` < G6 `8cd0…`. | FR-093-AC-12; TC-416 step 6 and expected result; FR-092 G4, G5, G7, G8 |
| FND-003 | low | The per-node-kind table has no row for an enum declaration node (`scalar_type`/`enum`). QSL emits one for each enum an `enum_value` names (FR-092 rule 1). Rule 4 names no node for it, and its body is `aggregate{[]}`, so its list is `[]`, as in QSpec's `positive-nominal-identities.json` `7928…`. Every other emitted kind has a row, so a reader of the table can take it for complete. Fix: add the row "`scalar_type` `enum` (an enum declaration) \| `[]`; its nominal preimage names no node". | FR-093 Node dependencies table; FR-092 rule 1 |

## Resolution

Resolved in the same change: every finding is fixed as its recommended fix states.
