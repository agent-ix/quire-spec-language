---
id: TC-1327
title: "A hyper clause emits its name row and clock binding on its body wrapper"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-322
    type: verifies
---
# TC-1327: A hyper clause emits its name row and clock binding on its body wrapper

## Description

Verify that a hyper clause's name and clock rows are keyed by the body wrapper at the assembled `claim` occurrence ordinal.

Scope: FR-488-AC-24 and FR-488-AC-25.

## Test Procedure

1. Compile `NonInterference` and read `temporal_clause_names` and `temporal_bindings`.
2. Compile two structurally equal hyper clauses with different names, then rename one.
3. Compile under event-position false-extension with a declared clock, then with the clock selection absent, unused and profile-mismatched; compile under infinite-trace.

Tag the tests `#[trace("TC-1327", "FR-488-AC-24", "FR-488-AC-25", "FR-322-AC-344", "FR-322-AC-361")]`.

## Expected Results

- Step 1: one name row at the wrapper's NodeId and the assembled occurrence's ordinal with the name `NonInterference`; the `hyperproperty` node has none.
- Step 2: one wrapper, two rows at distinct ordinals; renaming changes no NodeId.
- Step 3: one binding at the same key with the wrapper's clock text and the declared configuration; each inconsistent selection refuses the compile; under infinite-trace no binding.
