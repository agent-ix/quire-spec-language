---
id: TC-247
title: "The canonical EmittedPackage/CheckedPackage stay distinct from their pre-existing namesakes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-247: The canonical EmittedPackage/CheckedPackage stay distinct from their pre-existing namesakes

## Description

Verify that the canonical `EmittedPackage` this requirement defines in
layer-4 `package` (ADR-013 T-1) and the pre-existing
`protocol_artifact::native::EmittedPackage` (SEAM-3, `src/protocol_artifact/native/mod.rs:68`)
remain two separate types under two separate module paths, with no
re-export placing both into one import scope, and that this requirement
neither renames nor deletes the SEAM-3 type (its disposition is ADR-011
§7.3's own SEAM-3 schedule, not this requirement's). This two-module
coexistence is not asserted as a permanent steady state: ADR-011 §4
records that the two-public-types-named-`CheckedPackage` rule is met by
*deleting* the native-v1 type with SEAM-1, and the expiry condition for
`EmittedPackage`'s coexistence is that same SEAM-3 deletion, not an
indefinite state this test case treats as final. Scope: FR-087-AC-8.

Also verify the equivalent claim for `CheckedPackage`: `src/package/features.rs:9`
and `src/package/view.rs:13` already import the lane-private
`checking::CheckedPackage<'a>` by its bare name, inside the very module
tree this requirement makes define its own canonical `CheckedPackage`, and
this requirement leaves that import untouched, with no scope in which the
two names collide. This two-name coexistence is likewise not a permanent
steady state: ADR-011 §8 (Q209-1) records that `checking`'s lane retires or
converges into its replacement ("each other lane is deleted with its
replacement"), and the expiry condition for `features.rs`/`view.rs`'s
`crate::checking::CheckedPackage` import is that lane's own deletion, at
which point only the canonical `CheckedPackage` remains and this test
case's steps 6-7 no longer apply. Scope: FR-087-AC-10.

## Test Procedure

1. Search the whole compiled crate for every definition of a type named
   `EmittedPackage` and record each one's module path and field list.
2. Confirm exactly two definitions exist: one in layer-4 `package` (this
   requirement's), one in `protocol_artifact::native` (SEAM-3, unchanged) —
   this holds only until SEAM-3's own deletion PR removes the native-v1
   type (AC-8's stated expiry condition); once that PR lands, exactly one
   definition (this requirement's canonical `EmittedPackage`) remains, and
   that is the expiry condition being met, not a failure of this step.
3. Confirm neither module re-exports the other's `EmittedPackage` under a
   glob or a named `pub use` that would put both into the same import
   scope (for example, a `use crate::package::*;` combined with
   `use crate::protocol_artifact::native::*;` in the same scope resolving
   ambiguously).
4. Confirm the two `EmittedPackage` types' field lists and methods are
   unrelated (no shared trait implementation that would make them
   interchangeable at a call site expecting one or the other).
5. Confirm `src/protocol_artifact/native/mod.rs` is unchanged by this
   requirement's implementation (a diff against the pre-implementation tree
   touches no line in that file).
6. Read `src/package/features.rs:9` and `src/package/view.rs:13` and
   confirm each still imports `CheckedPackage` from `crate::checking`
   (unrenamed, unaliased) after this requirement's implementation, for as
   long as `checking`'s lane exists (ADR-011 §8, Q209-1); this step does
   not apply once that lane's own convergence PR deletes it.
7. Confirm no file in `package`'s own module tree brings both
   `crate::checking::CheckedPackage` and this requirement's canonical
   `package::CheckedPackage` into the same scope under the bare name
   `CheckedPackage` (which would be a compile-time ambiguity, not merely a
   readability concern).

## Expected Results

- Steps 1-2: exactly two `EmittedPackage` definitions, at the two named
  module paths, for as long as SEAM-3's `protocol_artifact::native::EmittedPackage`
  exists; a third definition, or a merge into one, fails this step, but
  exactly one definition remaining after SEAM-3's own deletion PR lands is
  this criterion's expiry condition being met, not a failure.
- Step 3: no import scope resolves both `EmittedPackage` names ambiguously.
- Step 4: no shared shape or trait makes the two `EmittedPackage`s
  interchangeable.
- Step 5: `protocol_artifact/native/mod.rs` is untouched.
- Step 6: while `checking`'s lane exists, both files' `crate::checking::CheckedPackage`
  imports are present and unchanged; an import that was renamed, removed,
  or redirected to the canonical type fails this step before that lane's
  own deletion, and this step no longer applies after it.
- Step 7: no scope resolves both `CheckedPackage` names ambiguously.
