---
id: TC-176
title: "The interim model -> check edge stays bounded to two files and thirteen names, imported directly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-176: The interim `model` → `check` edge stays bounded to two files and thirteen names, imported directly

**RETIRED by TC-262 (FR-074, ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** This
test case verified FR-068-AC-9/FR-068-CON-5 (both now retired, see FR-068):
that the *interim* `model` → `check` edge stayed bounded to exactly two
files and thirteen names while M-2 had not yet landed. FR-074 is M-2: it
moved the code that made those two files need the edge at all into `check`
itself, so the edge this test case bounded no longer exists — the resolved
`model` → `check` edge set is now empty, not "bounded to two files and
thirteen names." TC-262 verifies the post-M-2 inverse (the edge set is
empty) in the corresponding xtask test (`real_model_check_edge_is_empty`,
`#[trace("TC-262", "FR-074-AC-3")]`, replacing
`real_model_check_edge_is_bounded_to_two_files_and_thirteen_names_all_direct`).
This document's Test Procedure and Expected Results below are preserved
unchanged as the record of what this test case verified before M-2, per
this repository's rule against silently deleting a retired test case.

## Description

Verify the reverse edge FR-068's move opens — `model` (layer-3-earlier)
depending on `check` (layer-3-later), forbidden by ADR-011 §6.1's
intra-layer-3 order until M-2 (QSL-7) closes it — stays exactly as narrow as
FR-068-AC-9 and FR-068-CON-5 declare it: `model/checked_dispatch.rs`
importing four relocated names and `model/conformance.rs` importing nine,
both directly from `crate::check`, and no third `model` file gaining any
edge into `check` at all. This test catches two distinct wrong
implementations that each pass a textual `use`-line scan of `model` while
failing the real requirement: (1) a third `model` file quietly picking up a
`check`-derived name once `check` exists, invisible to a scan of only the
two named files; and (2) either named file reaching its thirteen names
through `crate::value`'s continued aggregate re-export (which FR-068-CON-4
permits for every *other* re-exporter) instead of importing `crate::check`
directly — a form that satisfies a textual scan of `model`'s own `use` lines
(they still say `crate::value`) while the edge stays real, and hidden, at
resolution. FR-068-CON-5 exists precisely to forbid this second form for
these two files; this test is written to fail on it, not only on the first.
Scope: FR-068-AC-9, FR-068-CON-5.

## Test Procedure

1. Read every `use` statement in `model/checked_dispatch.rs` and confirm
   `DispatchCandidate`, `DispatchOperation`, `DispatchTable` and
   `PackageDeclarations` are each imported by a `use crate::check::{...}`
   (or equivalent single-name `use crate::check::X;`) line — not by a `use
   crate::value::{...}` line, even though `value`'s aggregate re-export
   would also compile.
2. Read every `use` statement in `model/conformance.rs` and confirm
   `established_field_fact`, `Connective`, `Established`, `Location`,
   `Node`, `NodeKind`, `OrderedKind`, `Origin` and `ProvedInterval` are each
   imported the same way, directly from `crate::check`.
3. Confirm `checked_dispatch.rs`'s relocated-name import list holds exactly
   four names, no more, and `conformance.rs`'s holds exactly nine, no more;
   record any name beyond these thirteen that either file imports from
   `crate::check`.
4. Enumerate every other file under `src/model/` (every file besides
   `checked_dispatch.rs` and `conformance.rs`) and search for any `use`
   statement resolving into `crate::check`, whether written directly or
   reached through `crate::value`'s aggregate re-export.
5. Compile the crate and resolve the actual import graph at the
   post-macro-expansion level, not only the textual `use`-line level: for
   each of the thirteen names, confirm the resolved definition site is
   `crate::check` and the resolved import edge originates in
   `checked_dispatch.rs` or `conformance.rs` specifically, not in a third
   file that happens to re-import one of these names from `value` (which
   would still resolve into `check` per FR-068-CON-4's `value` aggregation,
   without appearing as a `model` → `check` edge at all in a textual scan of
   `model`, and must therefore be checked at the resolved level to be ruled
   out).
6. Confirm every other symbol either file imports is unaffected:
   `checked_dispatch.rs`'s `BinaryOperator`, `DeclaredClauseKind`,
   `Expression`, `FieldInitializer`, `FunctionDeclaration`, `NodeKey` and
   `ValueType`, and `conformance.rs`'s `OrderingOperator`, `Value` and
   `ValueType`, remain imported exactly as they are today.

## Expected Results

- Steps 1-2: all thirteen names resolve to a direct `use crate::check::...`
  in their respective file; any name still reached through `use
  crate::value::...` fails this step, naming the file and the name, even
  though the crate still compiles in that form.
- Step 3: `checked_dispatch.rs` imports exactly four relocated names and
  `conformance.rs` exactly nine; either file's list growing beyond its own
  count fails this step.
- Step 4: no third `model` file imports anything from `crate::check`,
  directly or through `crate::value`; any such edge fails this step and
  names the offending file and import.
- Step 5: the resolved import graph shows `model` → `check` bounded to
  exactly `checked_dispatch.rs` (four names) and `conformance.rs` (nine
  names); a hidden edge a textual scan misses — for example a third file
  reaching a relocated name through `value`'s continued aggregation — fails
  this step and names the actual resolved path.
- Step 6: both files' untouched imports are unchanged; any diff to them
  fails this step.
