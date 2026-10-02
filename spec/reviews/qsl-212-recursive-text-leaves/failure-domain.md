---
id: SR-614
title: "Failure-domain review of the QSL-212 recursive text-leaf list"
type: SpecReview
analysis: failure-domain
scope: "Commit 2a931a10: FR-093 Text leaves (the walk, its depth bound, the recursion leaf, the missing-selection refusal), its Dependencies on FR-322's LeafSegment, ADR-013 QC-24's text-leaf clauses, and TC-415 steps 8 and 9"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-415
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The checklist was run over the text-leaf walk.

- **Topological robustness.** Recursion no longer runs the walk to the
  depth limit. Every path is a simple path through the declared composites,
  and it ends at the first reentry. Sibling branches, an `Option<Node>` root
  (E17, `d` = 1), mutual recursion (E16) and a recursive composite with no
  text (`List`) behave as stated. Tuple cycles cannot arise (FR-143).
- **Entity identity.** A leaf list is a function of the checked type only. It
  names no node and no group ordinal, so it cannot make two application
  nodes of different groups collide, and it adds no new FR-092-OQ-1 case.
- **Evaluation purity.** The walk reads only checked types and the lock
  evidence's text definition.
- **Extension points.** Rule 6 ("any other type: nothing") covers model
  references, populations, quantities and enums. A new `ValueType` variant
  gets no leaves unless someone adds a rule. That matches FR-322's "text
  leaf" definition.

Three failure modes are unstated: the width of the walk is unbounded, the
outcome at today's v2 readers is unstated, and the refusal order when two
refusals apply is unstated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Nothing bounds the width of the walk. The "Finite" property bounds each path, and the depth limit bounds depth, but the list holds one entry per simple path to a text type. For `n` records that each hold a text field and an optional field of every other record, that is on the order of `(n-1)!` leaves. At `n` = 12 it is about 10^8 leaves, written into one application preimage, from about 40 lines of source. A DAG of non-recursive records that each hold two fields of the next record already doubles per record. No `CheckingLimits` charge covers leaves, so `check` runs out of time or memory instead of refusing. Fix: in Text leaves, add "Each appended leaf, text or recursion, costs one unit of the check stage's node budget (`CheckingLimits`). A walk that would pass it refuses with `resource_exhausted`/`insufficient-next-charge` naming that limit, and yields no node." Add an AC: with the node limit set to 1, `eq` of the Recursive text-leaf vectors (two leaves) refuses that way. Add it to TC-415 step 9. | FR-093:248-254, :203-225; FR-092 depth/node limit (`CheckingLimits`); `qsl-156-a4b` `lowering.rs` `text_leaves` (depth check only) |
| FND-002 | medium | The text never says what a current v2 reader does with the new leaves. QSpec's published `LeafSegment` pattern (`^(field:…\|position:…\|inner)$`, `node-identity-preimage.schema.json` at `e72756f`) rejects `recursion:<d>`. So a schema-validating reader refuses every package that compares or `contains` a recursive record with text. FR-322 says a reader checks "then the leaves" against the type. A reader that derives an optional field's leaf as `[field:t]` disagrees with QSL's `[field:t, inner]`, so its outcome is undefined. `quire-contract-ir`'s `check_leaves` today skips a mode-`null` leaf and any path of more than one segment, so it neither checks nor refuses either form. Fix: add a sentence to FR-093's Dependencies bullet on the `recursion:d` and `inner` segments: "Until QSpec adopts QC-24, a v2 reader that validates against the published `LeafSegment` pattern refuses a package holding a recursion leaf, and a reader that derives an optional field's leaf without `inner` refuses its leaves." Add the same to QC-24's Blocked-work cell. Keep TC-416's AC-7 fixtures free of both forms, as they are today. | FR-093:589-592; ADR-013 QC-24; QSpec `proposals/checked-package-v2/node-identity-preimage.schema.json` `LeafSegment`; QSpec FR-322 lines 108-125; `quire-contract-ir` `crates/quire-contract-model/src/checked_package/v2/operations.rs` `check_leaves` |
| FND-003 | low | The refusal order is unstated when both refusals apply. A structural comparison over a type whose walk passes the depth limit (or the budget of FND-001), checked with lock evidence that has no text-profile definition, can refuse two ways. The walk can refuse with `resource_exhausted`, or the leaf law can refuse with `missing-selection`. The A4b code walks first and reads the law per leaf afterwards, so `resource_exhausted` wins, but FR-093 does not say so. Fix: after the "at least one text leaf" paragraph, add "`check` completes the walk, or refuses on its limit, before it reads any leaf's law, so a resource refusal comes before `missing-selection`." | FR-093:269-273, :183-188; `qsl-156-a4b` `lowering.rs` `leaves` |

## Resolution

All findings are fixed. FND-001: each appended leaf costs one unit of the check stage's node limit, and passing it refuses with `resource_exhausted`/`insufficient-next-charge`; FR-093-AC-11 and TC-415 step 9 check `eq` under a node limit that admits the package's nodes but not its two leaves. FND-002: FR-093's Dependencies bullet and QC-24 state that a reader validating the published `LeafSegment` pattern refuses a recursion leaf and a reader deriving an optional field's leaf without `inner` refuses its leaves; QC-24's Blocks cell names those readers. TC-416's AC-7 fixtures hold neither form. FND-003: FR-093 states that `check` completes the walk, or refuses on a limit, before it reads any leaf's law.
