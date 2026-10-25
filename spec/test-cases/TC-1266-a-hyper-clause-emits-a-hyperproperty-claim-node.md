---
id: TC-1266
title: "A hyper clause emits a hyperproperty claim node, its slots, conjuncts and body"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: verifies
---
# TC-1266: A hyper clause emits a hyperproperty claim node, its slots, conjuncts and body

## Description

Verify that S4 emits a checked hyper clause over `behaviours` as QSpec
FR-402's `claim`/`hyperproperty` node with its body wrapper, slot
parameter nodes, `match` conjunct terms, operation anchors and fairness
aggregates, that the clause exposes its `claim` occurrence, and that clauses
with no encoding are refused.

Scope: FR-488-AC-1 to FR-488-AC-21.

## Test Procedure

Use ADR-023 §1's `NonInterference` and `SavesPower` over §8's vault and
device models, and FR-487's `Linked`.

1. Compile `NonInterference`; read the `hyperproperty` node's body, its body
   wrapper, the slot nodes with their occurrences, and read the package back
   through the I2 reader.
2. Compile `SavesPower`, and a clause with an alias that binds no object
   parameter.
3. Read `NonInterference`'s `match` aggregate; compile the `match` variants of
   FR-488-AC-8, a clause with a constant conjunct, and a clause with
   interleaved universal and existential conjuncts.
4. Compile `NonInterference` with `align skip { V::Vault::mix }` and with two
   operations, beside a state clause naming `mix`.
5. Compile the variants of FR-488-AC-11 and compare NodeIds; compile two
   structurally equal declarations, reformatted and renamed variants, and a
   renamed trace variable.
6. Compile `NonInterference` with `fair weak V::Vault::step` on `a`, with
   `fair step`, `fair weak whole step`, on `b`, and with a `systems_interface`
   fairness declaration.
7. Read `CheckedHyperClause::occurrence()` after assembly, the I2 reader's
   triple, FR-315-AC-12's `clause()`, and the lookup refusals.
8. List the I2 reader's temporal clause roots and look up the body wrapper as
   a clause root, also through FR-338's resolution.
9. Compile `Linked` with the two `B` atoms of FR-488-AC-16 and AC-17, and
   atoms whose read has no slot root or is rooted in two components.
10. Compile a timed-profile clause, one that is also an alias with no
    parameter, and one with two refusals.
11. Compile bounded-profile and `until` clauses (FR-488-AC-19, AC-20).
12. Compile `NonInterference` at and one below the node ceiling.
13. Admit each emitted package through QSL's I2 admission.

Tag the tests `#[trace("TC-1266", "FR-488-AC-n")]`.

## Expected Results

- Step 1: the seven bindings of FR-488-AC-1, AC-2 and AC-3; the wrapper and
  slots of AC-4 and AC-5.
- Step 2: FR-488-AC-6's subjects, parameters and slots; the alias with no
  parameter is refused `unsupported_construct`/`not-yet-implemented`
  (AC-18).
- Step 3: FR-488-AC-7, AC-8 and AC-9's conjunct terms, slot nodes, literals
  and source order.
- Step 4: FR-488-AC-10's anchor references.
- Step 5: FR-488-AC-11's different NodeIds and AC-12's equal NodeIds,
  ordinals and the renamed-variable difference.
- Step 6: FR-488-AC-13's fairness aggregates and the refused
  `systems_interface` declaration.
- Step 7: FR-488-AC-14's equal occurrences and lookup refusals.
- Step 8: FR-488-AC-15: the wrapper is not a temporal clause root, and
  looking it up returns `clause-root-unresolved`.
- Step 9: the first `Linked` emits (AC-16); the second, the rootless read and
  the two-component read are refused (AC-17).
- Step 10: FR-488-AC-18: each is refused at the first refusal in order.
- Step 11: FR-488-AC-19 and AC-20.
- Step 12: FR-488-AC-21.
- Step 13: no admission refusal.
