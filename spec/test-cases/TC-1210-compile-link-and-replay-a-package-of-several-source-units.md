---
id: TC-1210
title: "Compile, link and replay a package of several source units"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
# TC-1210: Compile, link and replay a package of several source units

## Description

Verify that a checked package compiles from several source units in one
package namespace with unit-local aliases. Verify that its own clauses,
dispatch and one-body rule read every unit, that the admitted package does
not depend on request order while refusals follow source order, and that a
several-unit package reference replays.

Scope: FR-480-AC-1 to FR-480-AC-8, FR-027-AC-14.

## Test Procedure

Every fixture unit starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v` unless a step says otherwise. Units have the authority `a` and
the identities their step names.

1. Compile QSpec TC-196 D10's split fixture, `U1` (D01, `PX`, `pre PA`) and
   `U2` (`pre PB1`, `pre PB2` through `U2`'s own `model M`), as `[U1, U2]`
   and `[U2, U1]`; then with the two bodies' names swapped; then with `PB1`
   in `U1`, again in both orders. Evaluate `I` for `d1` in each package,
   compare the package bytes, and read the occurrences of the shared `false`
   body node.
2. Compile FR-480-AC-2's `[B, A]`; then with `Point` also in `B`; then with
   `import A` in `B`; then with a dependency input supplying a library whose
   source has `A`'s (authority, identity).
3. Compile FR-480-AC-3's two-alias units with distinct bodies; with `B`'s
   body on `A`'s operation.
4. Through the FR-278 composition, compile FR-480-AC-4's profile-order and
   profile-set mismatches between two `1-draft` units, its `v`/`w` alias
   pair, the empty list and the duplicate source identity.
5. Compile AC-2's package with `Point` moved to `B`, and compare `Point`'s
   owner, node id, source-map region and the `package_id`.
6. Compile FR-480-AC-6's two documents in both orders.
7. Replay step 1's first package through FR-098 with the canonical
   reference, the reversed reference, a repeated source, `U2` dropped and a
   definition document.
8. Read AC-2's checked units and inspect the source reference and type report
   carried by each.
9. Run FR-027-AC-14's native-compile/1 requests: the `[U1, U2]` array, the
   reversed array, an array with a `0-draft` second unit, an empty array and
   the over-`dependent_bytes` array.

Tag the tests `#[trace("TC-1210", "FR-480-AC-n")]`, and step 9 `#[trace("TC-1210", "FR-027-AC-14")]`.

## Expected Results

- Step 1: D10's results (`absent-key`, then `precondition-false`) with equal
  charges in every order and placement; byte-equal packages for both
  orders of each placement; `sources` in canonical unit order; the
  shared `false` node's occurrences numbered `U1`'s first in both orders.
- Step 2: checks, `px`'s parameter typed by `A`'s `Point`;
  `ambiguous_declaration`/`ambiguous-name` naming both declarations; FR-099's
  unsupplied-import refusal; `invalid_package`/`conflicting-definition`
  before any parse, naming `A` and the library entry.
- Step 3: links; `conflicting_declaration`/`duplicate-declaration`, no
  package.
- Step 4: `invalid_package`/`conflicting-definition` at the later header for
  each mismatch; the alias pair checks; `invalid_package`/`invalid-value`;
  `invalid_package`/`conflicting-definition` before any parse.
- Step 5: `Point` owned by `B`, a different node id and `package_id`, and
  its region in `B`'s source.
- Step 6: row 1 at `z`, then row 6 at `u_self`.
- Step 7: the same `package_id` and the D10 call replays, for the
  canonical and the reversed reference; `invalid_package`/`invalid-value`
  at `/package/sources` before any recompile for the repeated source; `stale_dependency`/`content-mismatch` on the
  `package_id` comparison; a refusal before any recompile.
- Step 8: two checked units, each carrying its own source reference and a type
  report from that source's parse; the API provides no separate report/source
  rebinding operation.
- Step 9: the FR-278 bytes, equal for both arrays, exit 0;
  `unknown_edition` at the second unit's edition literal, naming its file;
  `invalid-request`; an intake refusal naming `dependent_bytes`.
