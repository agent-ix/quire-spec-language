---
id: TC-172
title: "check's real import graph has no edge into value::expression or into checking, and value::expression re-exports no check item"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-172: check's real import graph has no edge into value::expression or into checking, and value::expression re-exports no check item

## Description

Verify, against the compiled crate's actual `use` lines — not an ADR table or
a prose claim — that no file under `check` imports anything at all from
`value::expression`, the whole module (not only the illustrative
evaluation-only names `evaluate`, `Machine`, `Callable`, `Evaluation`,
`InputRefusal`, `LocatedLoss` and `ValueLoss` FR-068-AC-3 names as examples),
and that `check` and the pre-existing `checking` module (the SEAM-1/SEAM-2
module, untouched by this move) remain two distinct modules with no content
crossing between them. `CheckMode` is deliberately not one of the flagged
names here: FR-068's Outputs allocate `CheckMode` to `check` itself (it is
used only inside the three `check_*_expression` methods this requirement
relocates), so a `check` file defining `CheckMode` is the required shape,
not an import from `value::expression` — this test flags an *import* of
`CheckMode` from `value::expression`, which would mean `check` failed to
bring its own definition, not `CheckMode`'s presence under `check`. This test
catches a reverse edge that would make the "split" a repackaging rather than
a real dependency cut — for example, `check` calling back into
`evaluate::Machine` for a shortcut, or a careless implementation writing the
new checking content into the existing `checking` module by name confusion,
or merging the two. It also verified FR-068-AC-10's other direction, `value::expression`'s re-export of `check`'s `CheckedExpression`; QSL-181 retires FR-068-AC-10 and deletes that re-export (ADR-011 §7.2: the root crate re-exports no item that moves to `qsl-semantics`), so step 6 now confirms the re-export is absent. Scope: FR-068-AC-3; FR-068-AC-10 (retired).

## Test Procedure

1. Enumerate every `.rs` file under `qsl-semantics/src/check/`.
2. Parse or grep each file's `use` statements (including glob imports) and
   resolve each import path against the crate's module tree.
3. Flag any import that resolves into `value::expression` at all — the
   whole module, not a fixed name list: `Machine`, `Callable`, `Evaluation`,
   `InputRefusal`, `LocatedLoss` and `ValueLoss` are illustrative examples of
   symbols that would trip this step, matching FR-068-AC-3's own wording
   that these names are examples, not an exhaustive deny-list; a name not on
   this list (for example a new symbol `evaluate.rs` adds later) fails this
   step exactly the same way if it resolves into `value::expression`,
   because the criterion is the module boundary, not membership in this
   name list. `CheckMode` is excluded from this list by design (see
   Description) and is not flagged merely for appearing under `check`.
4. Flag any import that resolves into `checking` (the pre-existing SEAM-1/
   SEAM-2 module), and separately confirm `src/checking/`'s file list is
   byte-for-byte unchanged from the pre-move baseline (no file added,
   removed or content-changed as part of this requirement's implementation).
5. Compile the crate and confirm the flagged-absent conditions hold at the
   resolved (post-macro-expansion) level, not only at the textual `use`-line
   level, in case a macro or re-export obscures a textual scan.
6. Read `value::expression`'s `use` lines naming `check`. **Amended by
   QSL-181 (FR-068-AC-10 retired):** `value::expression` imports
   `CheckedExpression` with a plain `use crate::check::CheckedExpression;`
   and re-exports no `check` item. `CheckedPackage`'s own re-export,
   `pub use crate::checked_package::CheckedPackage;`, is FR-087-AC-9's and
   TC-256's, not this step's. (Before QSL-181 this step read
   `pub use crate::check::{CheckedExpression};`; before FR-087 it read the
   two-name line `pub use crate::check::{CheckedPackage,
   CheckedExpression};`.)

## Expected Results

- Step 3: zero flagged imports; any import resolving into `value::expression`
  — whether or not it names one of the illustrative examples — fails this
  step, naming the file and the import.
- Step 4: zero flagged imports into `checking`, and `src/checking/`'s file
  list and contents are unchanged from the pre-move baseline; any diff
  fails this step.
- Step 5: the resolved import graph confirms the same absence a textual scan
  found; a discrepancy (a hidden edge a textual scan misses) fails this
  step and names the actual resolved path.
- Step 6 (amended by QSL-181): no `pub use` in `value::expression` names
  `crate::check`, and `check::CheckedExpression` is the type's only public
  path; a `pub use crate::check::...` line of any shape fails this step.
