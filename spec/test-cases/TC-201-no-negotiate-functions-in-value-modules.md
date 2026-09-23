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

**Steps 1-3 are retired.** They described a source scan over `value::ieee`
and `value::division` as modules; neither module exists any more.
`value::division` was deleted by QSL-131 K2 (#339); `value::ieee` was
deleted by QSL-131 O3. There is no module left for a source scan of it to
read, so the two-module-scoped scans these steps described have no
successor and are not replaced. Step 4 is unaffected — it was already
crate-wide, not module-scoped — and remains exactly what the nine
`compile_fail,E0432` doctests above enforce, at the whole-crate level.

## Test Procedure

1. ~~After the removal change, run a source scan over `value::ieee` and
   `value::division` for any function definition whose name matches the
   pattern `negotiate_*`.~~ Retired: neither module exists (see Description).
2. ~~Run a source scan over the same two modules for a type definition named
   `IeeeBackendCapabilities`.~~ Retired: neither module exists (see
   Description).
3. ~~Run a source scan over the same two modules for any function whose
   parameters or return type reference a backend-capability-predicate
   shape (a set of `(capability, mode)` pairs consumed to decide backend
   support), regardless of its name, to catch a rename-only removal.~~
   Retired: neither module exists (see Description).
4. Confirm no call site in the crate references `negotiate_ieee`,
   `negotiate_integer_division`, or `IeeeBackendCapabilities`.

## Expected Results

Steps 1-3: retired (see Description); this test has no expected results for
them. Step 4 finds no remaining reference anywhere in the crate: the
doctest suite fails to compile as an unexpected pass, not a `compile_fail`,
if any of the nine removed names becomes resolvable again.
