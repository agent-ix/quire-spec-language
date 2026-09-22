---
id: TC-256
title: "check and package's dependency edge is one direction, bounded to CheckedGraph and {ImportView, LibraryLock}"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-256: check and package's dependency edge is one direction, bounded to CheckedGraph and {ImportView, LibraryLock}

## Description

This is the test case for the criterion the QSL-158 owner ruling
(2026-09-21) exists to establish: that relocating `CheckedPackage` from
`check` (FR-068/M-5) to `package` (ADR-013 T-1) closes the `check` →
`package` reverse edge rather than merely moving where the violation sits.
Verify all of: `check` imports nothing from `package`; `package` imports
exactly `CheckedGraph` from `check` and no other layer-3 `check`-core type;
`check` imports exactly `ImportView` and `LibraryLock` (read-only, for
binding a qualified reference's `a::Name` qualifier against the lock's
selections — ADR-011 `:253` lists "library lock" beside the import views
as an E3 input) from `library` and no other `library` item; `check` reads
`LibraryLock` only through its existing read-only accessors, never
constructing, mutating, or holding one of its private fields; `check`
defines no type, method, or field named `CheckedPackage`;
and `value::expression` reaches `CheckedPackage` only through the single
closed re-export `pub use crate::checked_package::CheckedPackage;`. Scope:
FR-087-AC-9.

Until X-7, layer-4 `package`'s content — the canonical `CheckedPackage`,
`EmittedPackage`, the v2 emitter and the I2 byte reader — lives in the
top-level module `checked_package` (`src/checked_package.rs`,
`src/checked_package/`), and the module named `package` holds only SEAM-1
(ADR-011 §6.2, `package` row). In this test, "`package`" means layer-4
`package`, whose source is both trees; the steps name both paths.

`package`'s bound to exactly one `check`-core import holds only under a
specific design this criterion also checks for: `package`'s `CheckedPackage`
holds a `CheckedGraph` (this package's own checked declarations, S3 output)
plus the S4-only checked dependency closure, and reaches every
check-owned-type value (`Node`, `Signature`, and the rest of today's
`CheckedPackage` accessor surface) by delegating through `CheckedGraph`'s
own accessor methods (a `graph() -> &CheckedGraph` accessor, or equivalent
named delegating accessors), never by `package` importing `Node`,
`Signature`, or any other check-owned type by name itself. `value::expression`
(layer 5) remains separately and directly permitted to import such types
from `check` (layer 3) under its own row's "Depends on: 4, 3, F, K"; that
edge is not routed through `package` and is out of this criterion's count.
A design where `package`'s `CheckedPackage` re-declares or re-imports
check-owned types directly, instead of delegating through `CheckedGraph`,
would need more than one `check`-core import and fails this criterion —
following FR-068-AC-6's own precedent (PR #282 review F3) for stating the
design consequence a bound like this rests on, rather than leaving it
implicit.

Both the `use`-line scan and the resolved import graph (the same
`xtask::import_graph`-style tool FR-068-AC-9/TC-176 uses) are run: a
fully-qualified inline path such as `crate::check::Node` used without a
`use` line would satisfy a textual scan while still opening a second
`check`-core edge, so the resolved-level check is this criterion's
decisive evidence, and the textual scan is retained only as a fast,
readable first pass.

## Test Procedure

1. Search every `use` statement under `src/check/` and confirm none
   resolves into `crate::package` or `crate::checked_package`.
2. Search every `use` statement under `src/checked_package.rs`,
   `src/checked_package/`, `src/package.rs` and `src/package/` (excluding
   `src/package/features.rs` and `src/package/view.rs`'s pre-existing,
   unrelated `crate::checking::CheckedPackage` import, TC-247) and confirm
   the only `check`-core item imported is `CheckedGraph`; a second
   `check`-core import (any type, function, or constant beyond
   `CheckedGraph`) fails this step and names it.
3. Search every `use` statement under `src/check/` for imports resolving
   into `crate::library` and confirm the only items imported are
   `ImportView` and `LibraryLock`; a third `library` import fails this step
   and names it. Separately, confirm no code under `src/check/` constructs
   a `LibraryLock`, holds one of its private fields, or calls any method on
   it other than its existing read-only accessors.
4. Search the whole compiled crate for any type, method, or field named
   `CheckedPackage` defined under `src/check/`; confirm none exists.
5. Read `value::expression`'s re-export line for `CheckedPackage` and
   confirm it is exactly `pub use crate::checked_package::CheckedPackage;`
   — no glob, no additional name, and no import from any path other than
   `crate::checked_package`.
6. Run the resolved import graph over `src/checked_package/` and
   `src/package/` and confirm `package` → `check` resolves to exactly one
   name, `CheckedGraph`, at the resolved level, not only in `use` lines — a
   fully-qualified `crate::check::Node` or similar inline path used
   anywhere under either tree (bypassing a
   `use` statement) fails this step even though steps 2 and 4 would not
   catch it.
7. Read `package`'s `CheckedPackage` definition and confirm it holds a
   `CheckedGraph` field (or an equivalent single wrapping field) plus the
   S4 dependency-closure data, and that every accessor returning a
   check-owned type (`Node`, `Signature`, etc.) delegates to that
   `CheckedGraph` field's own methods rather than re-implementing the
   access itself.
8. Build the crate with a test-only edge added in each forbidden direction
   in turn (a `check` module importing something from `package`; a
   `package` module importing a second `check`-core item beyond
   `CheckedGraph`, whether by a `use` line or a fully-qualified inline
   path; a `check` module importing a third `library` item beyond
   `ImportView` and `LibraryLock`) and confirm each addition is caught by
   the FR-060 T-12 module-DAG check (`arch-lint api-surface`) or the
   resolved import graph, not merely by a manual reading of the diff.

## Expected Results

- Step 1: zero `check` → `package` imports.
- Step 2: `package`'s only textual `check`-core import is `CheckedGraph`.
- Step 3: `check`'s only `library` imports are `ImportView` and
  `LibraryLock`; `LibraryLock` is read only, never constructed, mutated, or
  reached through a private field.
- Step 4: `check` names no `CheckedPackage`.
- Step 5: the re-export is exactly the one named line.
- Step 6: the resolved import graph confirms the same one-name bound at
  the resolved level; a hidden fully-qualified edge fails this step and
  names the offending path.
- Step 7: `CheckedPackage`'s check-owned-type accessors delegate through
  its `CheckedGraph` field; an accessor that imports or re-declares a
  check-owned type directly fails this step.
- Step 8: each of the three test-only forbidden edges is caught by an
  automated check; an edge that passes silently fails this test.
