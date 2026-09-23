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
`library` precedes `check` core in ADR-011 §6.1's layer-3 order. What these counts stood in for is
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
accessors, never constructing or mutating one; and the root crate, where
`value::expression` lives, re-exports no layer-crate item, `CheckedPackage`
included. Scope: FR-087-AC-9.

Layer-4 `package` is the crate `qsl-package` (X-7, QSL-182): the canonical
`CheckedPackage`, `EmittedPackage`, the v2 emitter and the I2 byte reader
are in `qsl-package/src/`, and the root crate's module named `package` holds
only SEAM-1 (ADR-011 §6.2, `package` row). In this test, "`package`" means
layer-4 `package`; the steps name its path.

The wrapping design is what this test guards: S4's output is made from S3's
output and nothing else, and every checked declaration has one owner (ADR-013
T-1). How many `check` items `package` imports is not checked. `package`
depending on `check` is layer 4's permitted direction.

## Test Procedure

1. Confirm `check` cannot import `package`. **Amended by QSL-182.** Layer-4
   `package` is the crate `qsl-package`, which depends on `qsl-semantics`,
   where `check` is. Cargo refuses the reverse edge as a dependency cycle, so
   no `use` line or inline path under `qsl-semantics/src/check/` can resolve
   into `qsl_package`. TC-390 confirms the crate edges from `cargo
   metadata`: `qsl-semantics` names no `qsl-package` dependency of either
   kind.
2. Search `qsl-semantics/src/check/` for any type, method, or field named
   `CheckedPackage`; confirm none exists.
3. **Amended by QSL-182.** Scan every `pub use` under the root crate's
   `src/` and confirm none is rooted at a layer crate: `value::expression`
   imports `CheckedPackage` with a private `use qsl_package::CheckedPackage;`
   and re-exports it nowhere (ADR-011 §7.2, §4 as amended). The scan is
   `tests/it/layer_crate_reexports.rs`. (Before QSL-182 this step required
   exactly `pub use crate::checked_package::CheckedPackage;`.)
4. Confirm no code under `qsl-semantics/src/check/` constructs a `LibraryLock`, mutates
   one, or calls any method on it other than its read-only accessors.
5. Read `package`'s `CheckedPackage` definition and confirm: it holds a
   `CheckedGraph` field (this package's own checked declarations) plus the
   S4 dependency-closure data; its only constructor is the S4 link step,
   taking a `CheckedGraph`; every accessor that returns check-owned state
   delegates to that `CheckedGraph` field; and no type under
   `qsl-package/src/` or `src/package/` re-declares or copies a
   check-owned type (`Node`, `Signature`, and the rest).
6. Adverse checks: a `qsl-semantics` dependency on `qsl-package` (normal or
   dev) fails TC-390's crate-edge check, and a planted `pub use` of a layer
   crate fails step 3's scan fixture. **Amended by QSL-182.** A `check`
   module importing `qsl_package::CheckedPackage` does not compile, so it
   needs no fixture of its own.

## Expected Results

- Step 1: zero `check` → `package` edges, at `use`-line and inline-path
  level.
- Step 2: `check` names no `CheckedPackage`.
- Step 3: no root-crate `pub use` is rooted at a layer crate.
- Step 4: `LibraryLock` is only read by `check`.
- Step 5: `CheckedPackage` wraps a `CheckedGraph`, is built only from one,
  and delegates to it; a re-declared or copied check-owned type fails this
  step and names it.
- Step 6: both adverse cases are caught by the automated gates; a case that
  passes silently fails this test.
