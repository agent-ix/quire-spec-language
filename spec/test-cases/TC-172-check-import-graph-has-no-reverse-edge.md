---
id: TC-172
title: "check's real import graph has no edge into value::expression or into checking, and its re-export back is bounded"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-172: check's real import graph has no edge into value::expression or into checking, and its re-export back is bounded

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
or merging the two. It also verifies FR-068-AC-10's other direction: that
`value::expression`'s own re-export back into `check` (and, after FR-087,
into `package` for the relocated `CheckedPackage`) is exactly the closed,
non-glob list AC-10 (as amended by FR-087) requires, since nothing else in
this requirement's test cases inspects it and a glob re-export would
otherwise pass every other criterion. Scope: FR-068-AC-3, FR-068-AC-10.

## Test Procedure

1. Enumerate every `.rs` file under `src/check/`.
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
6. Read `value::expression`'s own re-export line(s) naming `check`'s and
   `package`'s relocated types and record each exact name list and whether
   either is written as a glob. **Amended by FR-087 (owner ruling on
   QSL-158, 2026-09-21): `CheckedPackage` relocates out of `check` into
   layer-4 `package` (FR-087-AC-9), so the single two-name line this step
   originally checked splits into two single-name re-exports from two
   different crate-absolute paths** — `pub use crate::check::{CheckedExpression};`
   and `pub use crate::package::{CheckedPackage};` — and this step now
   reads both lines, not one.

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
- Step 6 (amended by FR-087): the two lines read exactly
  `pub use crate::check::{CheckedExpression};` and
  `pub use crate::package::{CheckedPackage};` — one name each, from their
  own named path, not a glob; a third name on either line, a glob, a path
  other than `crate::check`/`crate::package`, or `CheckedPackage` still
  named on the `crate::check` line, fails this step. (Pre-FR-087, this step
  read the single line `pub use crate::check::{CheckedPackage,
  CheckedExpression};`; that form is superseded, not merely narrowed, once
  `CheckedPackage` no longer lives in `check` at all.)
