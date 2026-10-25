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

Scope: FR-488-AC-1 to FR-488-AC-10.

## Test Procedure

Use ADR-023 §1's `NonInterference` and `SavesPower` over §8's vault and
device models, and FR-487's `Linked`.

1. Compile `NonInterference`; read the `hyperproperty` node's body, its body
   wrapper, the slot nodes with their occurrences, and read the package back
   through the I2 reader.
2. Compile `SavesPower`, and a clause with an alias that binds no object
   parameter.
3. Read `NonInterference`'s `match` aggregate; compile the `match` variants of
   FR-488-AC-4 and a clause with a constant conjunct.
4. Compile `NonInterference` with `align skip { V::Vault::mix }` and with two
   operations, beside a state clause naming `mix`.
5. Compile the variants of FR-488-AC-6 and compare NodeIds; compile two
   structurally equal declarations.
6. Compile `NonInterference` with `fair weak V::Vault::step` on `a`, with
   `fair step`, `fair weak whole step`, and on `b`.
7. Read `CheckedHyperClause::occurrence()` after assembly, the I2 reader's
   triple, FR-315-AC-12's `clause()`, the lookup refusals, and the temporal
   clause roots.
8. Compile `Linked` with the two `B` atoms of FR-488-AC-9 and a timed-profile
   clause.
9. Pass each admitted package through QSpec FR-370's reader.

Tag the tests `#[trace("TC-1266", "FR-488-AC-n")]`.

## Expected Results

- Step 1: the seven-binding aggregate of FR-488-AC-1; the wrapper and slots
  of FR-488-AC-2; the I2 reader admits the package.
- Step 2: FR-488-AC-3's subjects, parameters and slots; the alias with no
  parameter is refused `unsupported_construct`/`not-yet-implemented`.
- Step 3: FR-488-AC-4's conjunct terms, slot nodes, source order and literal.
- Step 4: FR-488-AC-5's anchor references.
- Step 5: FR-488-AC-6's different and equal NodeIds and ordinals.
- Step 6: FR-488-AC-7's fairness aggregates.
- Step 7: FR-488-AC-8's equal occurrences and lookup refusals.
- Step 8: the first `Linked` emits, the second and the timed clause are
  refused `unsupported_construct`/`not-yet-implemented`.
- Step 9: no refusal (FR-488-AC-10).
