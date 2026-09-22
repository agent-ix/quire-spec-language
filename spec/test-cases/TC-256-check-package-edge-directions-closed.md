---
id: TC-256
title: "check and package's dependency edge is one direction, and CheckedPackage wraps CheckedGraph"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-256: check and package's dependency edge is one direction, and CheckedPackage wraps CheckedGraph

**Amended by the layer-rule ruling (2026-09-22).** Two item counts are
retired from this test: "`package` imports exactly one `check`-core item,
`CheckedGraph`" and "`check` imports exactly `ImportView` and `LibraryLock`
from `library`". The first passes a `package` that copies check-owned data
into re-declared types while importing only `CheckedGraph`, and fails a
`package` that imports `Node` to name an accessor's return type, which is the
permitted layer-4-on-layer-3 direction. The second caps a permitted edge:
`library` precedes `check` core in ADR-011 §6.1's layer-3 order, and `check`
imports nothing from `library` today. What these counts stood in for is
checked directly below (step 5) or elsewhere: FR-087-AC-6 and FR-060 T12-B
hold the no-minting-from-wire-ids property. FR-087-AC-9 gives the reasons.

## Description

This is the test case for the criterion the QSL-158 owner ruling
(2026-09-21) exists to establish: that relocating `CheckedPackage` from
`check` (FR-068/M-5) to `package` (ADR-013 T-1) closes the `check` →
`package` reverse edge rather than merely moving where the violation sits.
Verify all of: `check` imports nothing from `package`; `check` defines no
type, method, or field named `CheckedPackage`; `package`'s `CheckedPackage`
is built by the S4 link step from a `CheckedGraph`, holds it as a field, and
reaches check-owned state through it without re-declaring or copying any
check-owned type; `check` reads `LibraryLock` only through its read-only
accessors, never constructing or mutating one; and `value::expression`
reaches `CheckedPackage` only through the single closed re-export
`pub use crate::checked_package::CheckedPackage;`. Scope: FR-087-AC-9.

Until X-7, layer-4 `package`'s content — the canonical `CheckedPackage`,
`EmittedPackage`, the v2 emitter and the I2 byte reader — lives in the
top-level module `checked_package` (`src/checked_package.rs`,
`src/checked_package/`), and the module named `package` holds only SEAM-1
(ADR-011 §6.2, `package` row). In this test, "`package`" means layer-4
`package`, whose source is both trees; the steps name both paths.

The wrapping design is what this test guards: S4's output is made from S3's
output and nothing else, and every checked declaration has one owner (ADR-013
T-1). How many `check` items `package` imports is not checked. `package`
depending on `check` is layer 4's permitted direction.

## Test Procedure

1. Scan every shipped `use` line and every `crate::`-rooted inline path under
   `src/check/` and confirm none resolves into `crate::package` or
   `crate::checked_package`. This is FR-068-AC-6's layer rule; TC-175 runs
   the same scan, and this step reconfirms it.
2. Search `src/check/` for any type, method, or field named
   `CheckedPackage`; confirm none exists.
3. Read `value::expression`'s re-export line for `CheckedPackage` and
   confirm it is exactly `pub use crate::checked_package::CheckedPackage;`
   — no glob, no additional name, and no import from any path other than
   `crate::checked_package`.
4. Confirm no code under `src/check/` constructs a `LibraryLock`, mutates
   one, or calls any method on it other than its read-only accessors.
5. Read `package`'s `CheckedPackage` definition and confirm: it holds a
   `CheckedGraph` field (this package's own checked declarations) plus the
   S4 dependency-closure data; its only constructor is the S4 link step,
   taking a `CheckedGraph`; every accessor that returns check-owned state
   delegates to that `CheckedGraph` field; and no type under
   `src/checked_package/` or `src/package/` re-declares or copies a
   check-owned type (`Node`, `Signature`, and the rest).
6. Adverse checks, run against fixture trees: a `check` module importing
   `crate::checked_package::CheckedPackage` by `use` line, and the same edge
   written as an inline path, each fail step 1 through the FR-068-AC-6 gate
   (`xtask` import graph), not merely by a manual reading of the diff.

## Expected Results

- Step 1: zero `check` → `package` edges, at `use`-line and inline-path
  level.
- Step 2: `check` names no `CheckedPackage`.
- Step 3: the re-export is exactly the one named line.
- Step 4: `LibraryLock` is only read by `check`.
- Step 5: `CheckedPackage` wraps a `CheckedGraph`, is built only from one,
  and delegates to it; a re-declared or copied check-owned type fails this
  step and names it.
- Step 6: both adverse edges are caught by the automated gate; an edge that
  passes silently fails this test.
