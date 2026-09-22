---
id: TC-244
title: "compile_fail matrix over every forbidden typestate construction (R-10, O-15)"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-244: compile_fail matrix over every forbidden typestate construction (R-10, O-15)

## Description

ADR-013 R-10 (`:94`) and O-15's evidence rule (§1, "Evidence for the
rules") require `compile_fail` tests "on every public constructor path"
and "for every forbidden construction." This test enumerates that matrix
explicitly, rather than relying on TC-243's one-doctest-per-type coverage
to happen to also cover every forbidden *conversion* between types. The
matrix is drawn from O-15's own Public type, Invariants and Conversions
rows (`:332-335`: "no conversion from an unchecked or wire-admitted value,"
"no conversion into a checked type from wire bytes") and from R-10 itself
(`:94`: "Bytes read from any wire ... never become checked typestate").

**Vantage.** A `compile_fail` doctest links the crate externally, so each
row below demonstrates only that a value *outside the whole crate* cannot
perform the named construction. It cannot demonstrate that an in-crate
sibling module (for example `library` or `package` itself, reaching past
its own accessor surface) cannot perform it internally; that stronger,
decisive claim is FR-087-AC-2's whole-crate source scan (Inspection: every
function returning `CheckedGraph` or `CheckedPackage` is enumerated and
confirmed to live inside the S3 checker or the S4 link step), which this
test's doctests supplement rather than replace.

**Error codes.** A bare `compile_fail` passes on any compilation failure —
a typo, a stale import, an unrelated error — and proves nothing about the
specific forbidden path a row names. Each row below states the exact
`rustc` code the doctest must fail with, following this repository's own
precedent of code-qualified `compile_fail` attributes (`src/temporal/mapping.rs:125`
is `compile_fail,E0451`; `src/package.rs:188,195` and
`src/protocol_artifact/native/mod.rs:98` are `compile_fail,E0308`).
`src/temporal/mapping.rs:125`, not `src/package.rs:188`, is this test's own
precedent for the private-field-construction rows (1, 2, 6; `E0451`), since
`src/package.rs:188` is an `E0308` type-mismatch case, not a privacy case.
Rows 3-5 name a *conversion*, not a type mismatch: they attempt
`CheckedPackage::from(v)`/`v.into()` and expect `E0277` (trait bound not
satisfied), because a direct type-mismatch attempt fails with `E0308`
regardless of whether the forbidden `From`/`Into` impl exists, and so
would not detect the violation these rows exist to catch.

Scope: FR-087-AC-2.

## Test Procedure

0. Run FR-087-AC-2's own primary evidence: a whole-crate source scan of
   every function returning `CheckedGraph` or `CheckedPackage`, confirming
   each one is defined inside the S3 checker or the S4 link step. This
   scan, not the doctests below, is FR-087-AC-2's decisive evidence
   (Description, "Vantage"): it rules out an in-crate bypass that a
   doctest's external vantage cannot see. The doctests in steps 1-6
   supplement this scan with the crate-external case; they do not replace
   it.

For each row below, write one `compile_fail,<code>` doctest attempting the
named construction, placed at the relevant type's public documentation
site, and confirm it fails with exactly the named code (not merely "some"
compile error):

1. `ParsedSource` → `CheckedGraph` by attempting to construct `CheckedGraph`
   directly, as a struct literal, from outside `check` — expected code
   `E0451` (field of `CheckedGraph` is private), the same shape as
   `src/temporal/mapping.rs:125`.
2. `CheckedGraph` → `CheckedPackage` by attempting to construct
   `CheckedPackage` directly, as a struct literal, from outside `package` —
   expected code `E0451`.
3. `VerifiedPackage` → `CheckedPackage` by attempting
   `CheckedPackage::from(verified_package_value)` or
   `verified_package_value.into()` — expected code `E0277` (trait bound
   `CheckedPackage: From<VerifiedPackage>` not satisfied). A direct
   type-mismatch attempt (passing the value where `CheckedPackage` is
   required, without going through `From`/`Into`) is not used for this row:
   it fails with `E0308` regardless of whether a forbidden `impl
   From<VerifiedPackage> for CheckedPackage` exists elsewhere in the crate,
   so it would stay green even if that exact R-10 violation were added.
   Only a `From`/`Into` attempt exercises the conversion path this row
   names.
4. `ImportView` → `CheckedPackage` by the same means as row 3
   (`CheckedPackage::from(import_view_value)` / `.into()`) — expected code
   `E0277`.
5. A `protocol_artifact`-read value → `CheckedGraph` or `CheckedPackage` by
   the same means as row 3 — expected code `E0277`.
6. `EmittedPackage`'s wire bytes decoded into a `CheckedPackage` by
   attempting to construct `CheckedPackage` directly, as a struct literal,
   from decoded byte values, bypassing the verified binding and the S1-S4
   recompile the executor uses instead (ADR-013 T-2) — expected code
   `E0451`, the same private-field failure as rows 1-2, since no
   constructor accepting decoded bytes is exposed at all.

## Expected Results

- Step 0: the whole-crate scan confirms every `CheckedGraph`/`CheckedPackage`-
  returning function is defined inside the S3 checker or the S4 link step;
  any function returning either type from outside those two locations
  fails this step and names the function.
- Steps 1-6: each of the six doctests fails to compile with exactly its
  named code. A doctest that compiles successfully, or that fails with a
  different code than the row names (for example a stale-import or syntax
  error that would pass this test's letter while proving nothing about the
  forbidden construction), fails this test and names the row.
