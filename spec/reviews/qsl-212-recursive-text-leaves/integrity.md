---
id: SR-613
title: "Integrity review of the QSL-212 recursive text-leaf list and the Pre ownership change"
type: SpecReview
analysis: integrity
scope: "Commit 2a931a10: FR-093 Text leaves (rules 1 to 6, reachability, the optional-field inner, the expanded leaf set and the four properties), the Pre paragraph and AC-8, FR-094's postcondition paragraph, and ADR-013 QC-24's text-leaf clauses"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The walk is well defined and the vectors follow it. The four properties
mostly hold. The scenarios below were checked against rules 1 to 6 by hand
and with a scratch walker.

- **Finite.** Holds. A composite is open only while the walk is inside it, so
  every path is a simple path through the declared composites. Its length is
  bounded by the number of composites times the segments of one field type.
  Cycles through a tuple position cannot occur, because FR-143 refuses them
  (FR-092 Recursion groups). Rule 3's tuple case is therefore unreachable but
  harmless. A collection of the composite ends the path at rule 3 after one
  `inner`, as an `Option` does.
- **Sibling branches.** Handled. Rule 4 closes `C` after its fields, so
  `record Two { x: Node; y: Node; }` walks `Node` twice, and each walk ends in
  `recursion:1`.
- **Text reached only through an ancestor.** Handled. Reachability ignores
  open composites, so a composite whose only route to `Text` runs through an
  open ancestor still gets its recursion leaf. The least-fixpoint expanded
  set then reaches the ancestor's text leaves through the ancestor's own
  recursion leaf.
- **Type alias.** Walks as its resolved type, consistent with FR-092's "a type
  alias introduces no type node".
- **At least one text leaf.** Holds. A shortest route to `Text` is a simple
  path, and the walk explores every simple path.
- **Independent of keys and group order.** Holds. `d` is a prefix length of
  the leaf's own path, and no leaf names a node.

The Pre decision agrees with ADR-012. §4.3 says "`Pre` is legal only in an
operation postcondition and moves to `ProtocolClause` with #218", and has
said so since ARCH-11. §3 gives `ProtocolClause` "operation clauses". The
code agrees: `DeclaredClauseKind` has no postcondition variant, and
`check_postcondition_expression` checks a postcondition as a standalone
expression that is never keyed into the package graph. The integrity gaps
are one false property claim, one rationale that proves too much, and one
proof that is only half stated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The claim "Unchanged for a type with no recursion" is false for an optional text field. Rule 4 walks `f?: V` at `field:f` + `inner`, so `record R { t?: Text[..] }` now gives `[field:t, inner]`. The A4b walk gives `[field:t]` today. FR-322 names no `inner` step for an optional field. QC-24 and FR-093's Dependencies call the `inner` a QSL proposal. So the bullet's "as FR-322 lists them" contradicts the same commit's Dependencies. QC-24's "so a type with no recursion keeps its leaves" repeats the claim. Fix: rewrite the bullet as "**Without recursion.** A walk that reaches no open composite appends no recursion leaf. Its leaves are FR-322's text leaves, except that an optional field's path passes through `inner` (a QSL proposal, ADR-013 QC-24)." In QC-24, change "so a type with no recursion keeps its leaves" to "so a type with no recursion gets no recursion leaf". | FR-093:259-263, :219-221, :589-592; ADR-013 QC-24; QSpec FR-322 lines 108-113 |
| FND-002 | medium | The reason FR-093 gives for moving `Pre` also covers preconditions. It says "A postcondition is a `ProtocolClause` subnode (ADR-012 §3, §4.3), not a `Value` declaration". But ADR-012 §1's checked-type table assigns "checked operation header and checked clause subnodes (frame, scoped anchor, pre- and postcondition)" to `ProtocolClause`. §4.1 lists preconditions and postconditions alike as typed subnodes. FR-094 still keys precondition clause functions in the `Value` lowering. So the cited rule does not separate the two clauses FR-093 and FR-094 treat differently. What does separate them is ADR-012 §4.3's `Pre` sentence and the code fact that `DeclaredClauseKind` has no postcondition arm. Fix: in FR-093, replace the sentence with "`pre(...)` is legal only in an operation postcondition, and ADR-012 §4.3 moves `Pre` with the `ProtocolClause` lowering (#218). `check` checks a postcondition through `check_postcondition_expression` and keys no node for it, and `DeclaredClauseKind` has no postcondition arm." In FR-094's new paragraph, add: "ADR-012 §1 assigns the precondition subnode to `ProtocolClause` as well; #218 decides whether precondition clause functions move from this requirement." | FR-093:190-199; FR-094:217-220; ADR-012 §1 table (line 143), §4.1, §4.3; `qsl-forms/src/syntax.rs` `DeclaredClauseKind` |
| FND-003 | low | The expanded-leaf-set paragraph is ambiguous, and the "Injective" argument is half stated. (1) "The value at its path `q`" does not say whether `q` includes the final `recursion:d` segment. It must not, or `q` names no value. (2) The paragraph shows only soundness: each re-rooted leaf is a real text leaf. The property needs completeness too: every text leaf of the unfolding is in the set. Completeness holds, by induction on path length: a path that passes a reentry is `q + s`, and `r + s` is a strictly shorter path to a leaf of the same type. (3) "Injective" names the wrong property. What is shown is that the list is a lossless encoding of the text-leaf set. Fix: write "For a recursion leaf whose path is `q` + `recursion:d`, let `r` be the first `d` segments of `q`". Say "each text leaf in the set", not "each member". Add one sentence: "Every text leaf of the unfolding is in the set: a path through a reentry at `q` is `q` + `s`, and `r` + `s` is a shorter path to a leaf of the same type, so induction on path length ends at a text leaf of the list." Rename the bullet "**Lossless.**" | FR-093:238-244, :255-258 |

## Resolution

All findings are fixed. FND-001: the bullet is now **Without recursion** and names the optional-field `inner` as a QSL proposal (QC-24); QC-24 says a type with no recursion gets no recursion leaf. FND-002: FR-093 grounds the `Pre` move in ADR-012 §4.3 and in `pre(...)` being legal only in a postcondition, which `check_postcondition_expression` checks without keying a node and `DeclaredClauseKind` has no arm for; FR-094 adds that ADR-012 §1 assigns the precondition subnode to `ProtocolClause` too and #218 decides whether precondition clause functions move. FND-003: the expanded-set paragraph defines `q` and `r` explicitly, adds the completeness induction, and the property is renamed **Lossless**.
