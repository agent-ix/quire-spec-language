---
id: TC-172
title: "check's real import graph has no edge into value::expression or into checking"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-172: check's real import graph has no edge into value::expression or into checking

## Description

Verify, against the compiled crate's actual `use` lines — not an ADR table or
a prose claim — that no file under `check` imports anything from
`value::expression`'s evaluation-only surface (`evaluate`, `Machine`,
`Callable`, `Evaluation`, `InputRefusal`, `LocatedLoss`, `ValueLoss`), and
that `check` and the pre-existing `checking` module (the SEAM-1/SEAM-2
module, untouched by this move) remain two distinct modules with no content
crossing between them. This test catches a reverse edge that would make the
"split" a repackaging rather than a real dependency cut — for example,
`check` calling back into `evaluate::Machine` for a shortcut, or a careless
implementation writing the new checking content into the existing `checking`
module by name confusion, or merging the two. Scope: FR-068-AC-3.

## Test Procedure

1. Enumerate every `.rs` file under `src/check/`.
2. Parse or grep each file's `use` statements (including glob imports) and
   resolve each import path against the crate's module tree.
3. Flag any import that resolves into `value::expression::evaluate`, or
   names `Machine`, `Callable`, `Evaluation`, `InputRefusal`, `LocatedLoss`
   or `ValueLoss` from any path.
4. Flag any import that resolves into `checking` (the pre-existing SEAM-1/
   SEAM-2 module), and separately confirm `src/checking/`'s file list is
   byte-for-byte unchanged from the pre-move baseline (no file added,
   removed or content-changed as part of this requirement's implementation).
5. Compile the crate and confirm the flagged-absent conditions hold at the
   resolved (post-macro-expansion) level, not only at the textual `use`-line
   level, in case a macro or re-export obscures a textual scan.

## Expected Results

- Step 3: zero flagged imports; any import resolving to the named
  evaluation-only symbols fails this step, naming the file and the import.
- Step 4: zero flagged imports into `checking`, and `src/checking/`'s file
  list and contents are unchanged from the pre-move baseline; any diff
  fails this step.
- Step 5: the resolved import graph confirms the same absence a textual scan
  found; a discrepancy (a hidden edge a textual scan misses) fails this
  step and names the actual resolved path.
