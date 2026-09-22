---
id: TC-201
title: "value::ieee and value::division carry no negotiate_* function"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-078
    type: verifies
---
# TC-201: value::ieee and value::division carry no negotiate_* function

## Description

Verify that `value::ieee` and `value::division` define no function matching
the name pattern `negotiate_*` and no `IeeeBackendCapabilities` type, after
removal. Scope: FR-078-AC-1, FR-078-AC-2.

Catches an implementation that removes `negotiate_ieee` but leaves
`negotiate_integer_division` (or vice versa) — a partial removal that
satisfies only half the requirement — and an implementation that renames
`negotiate_ieee` to something like `ieee_backend_support` while keeping its
negotiation-shaped logic and call sites intact, which would not appear in a
scan restricted to the literal string `negotiate_`.

Verified by nine `compile_fail,E0432` doctests in `src/value/mod.rs` (module
doc comment, QSL-131), one per removed public name — `negotiate_ieee`,
`negotiate_integer_division`, `IeeeBackendCapabilities`,
`IeeeItemRequirement`, `IeeeUnsupportedCause`, `IeeeDisposition`,
`IntegerDivisionBounds`, `IntegerDivisionConsumer` and
`IntegerDivisionDisposition` — each pinning the unresolved-import diagnostic
rather than an untagged or type-error `compile_fail`, plus a positive-control
doctest confirming a surviving name (`AdmittedIeeeProfile`) still resolves
from the same path. `cargo test --doc -p quire-spec-language` runs them.

## Test Procedure

1. After the removal change, run a source scan over `value::ieee` and
   `value::division` for any function definition whose name matches the
   pattern `negotiate_*`.
2. Run a source scan over the same two modules for a type definition named
   `IeeeBackendCapabilities`.
3. Run a source scan over the same two modules for any function whose
   parameters or return type reference a backend-capability-predicate
   shape (a set of `(capability, mode)` pairs consumed to decide backend
   support), regardless of its name, to catch a rename-only removal.
4. Confirm no call site in the crate references `negotiate_ieee`,
   `negotiate_integer_division`, or `IeeeBackendCapabilities`.

## Expected Results

Steps 1 and 2 find no matches. Step 3 finds no function of that shape
remaining in either module. Step 4 finds no remaining reference anywhere in
the crate.
