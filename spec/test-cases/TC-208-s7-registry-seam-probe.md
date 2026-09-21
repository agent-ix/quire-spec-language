---
id: TC-208
title: "The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: verifies
---
# TC-208: The S7 seam probe fails to compile the registry arm on an unhandled capability-kind variant

## Description

Verify that the registry's match over the canonical `Capability` type is
exhaustive at compile time: under the `seam-probe` cargo feature, adding a
probe variant to the capability-kind enum causes the registry's
advertisement-check and candidate-matching arms to fail to compile with
`E0004`, at exactly the checked-in expected locations. Scope: FR-080-AC-5.

Catches an implementation that matches on capability kind with a wildcard
`_` arm (which would silently accept a new capability kind as "no match"
rather than force a compile error), and a seam-probe wiring that adds the
probe variant to the enum but never builds the registry crate under the
feature, so the intended `E0004` never actually fires and the checked-in
location list is vacuously satisfied by not being checked at all.

## Test Procedure

1. Confirm the registry crate is included in `xtask seam-probe`'s build
   scope under the `seam-probe` feature.
2. Run `xtask seam-probe` against the unmodified tree and confirm it
   passes (the probe variant, gated behind the feature, produces the
   expected checked-in `E0004` set and no others).
3. Temporarily add a `#[cfg(not(feature = "seam-probe"))]` wildcard `_` arm
   to the registry's capability-kind match (simulating a regression that
   defeats exhaustiveness) and re-run `xtask seam-probe`.
4. Temporarily remove the registry's match site from the checked-in
   expected-location list without changing the code, and re-run
   `xtask seam-probe`.

## Expected Results

Step 2 passes, and its report includes the registry's advertisement-check
and candidate-matching arms among the reported `E0004` locations. Step 3
fails: with the wildcard arm added, the registry crate compiles under the
probe feature and produces no `E0004` at that site, so `xtask seam-probe`
reports the expected location as missing and exits non-zero. Step 4 also
fails: the still-produced `E0004` location is no longer in the checked-in
list, so `xtask seam-probe` reports an unexpected location and exits
non-zero.
