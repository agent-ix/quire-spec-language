---
id: SR-631
title: "Failure-domain review of ADR-015 compile and replay against dependencies"
type: SpecReview
analysis: failure-domain
scope: "git diff origin/main...6874dc53 on qsl-255-dep-design: ADR-015, FR-099, TC-446, and the amendments to FR-001, FR-027, FR-071, FR-098, TC-186, TC-444, ADR-011, ADR-013, spec/spec.md and spec/tests.md; checked against QSpec qspec-dep-design at 076a2fd (FR-307, FR-322, FR-323) and QSL PR #445 at 5f13e891"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
---
# SR-631: Failure-domain review of ADR-015

## Summary

Reviewed the QSL-255 part (b) change at commit 6874dc53 on branch
`qsl-255-dep-design`. The review set is the base checklist plus the
failure-domain analysis. The change adds ADR-015, FR-099 and TC-446, and
amends FR-001, FR-027, FR-071, FR-098, TC-186, TC-444, ADR-011 and ADR-013.
QSL was compared with the matching QSpec change at 076a2fd (FR-307, FR-322
`dependency_reference`, FR-323 `package.dependencies`) and with PR #445 at
5f13e891, which lands first.

Base checklist: the IDs are well formed and unused (FR-099, TC-446,
FR-027-AC-10, FR-071-AC-9, FR-098-AC-7). Every new or rewritten criterion has
a TC step, and the `spec/tests.md` rows match. The relationships resolve. The
refusal codes and causes named in the change all exist in
`docs/native-error-codes.md` and `qsl-cst/src/diagnostic.rs`:
`missing_import`, `stale_dependency`, `invalid_package`, `invalid_digest`
(`HostCause::SelectionDigest`), `missing-selection`, `revision-mismatch`,
`byte-digest-mismatch`, `definition-cycle`, `conflicting-definition` and
`invalid-value`. `DependencyIdentityMismatch` exists only as #445's
`LinkRefusal` variant (`qsl-package/src/checked.rs:190`). It maps to
`stale_dependency`/`byte-digest-mismatch`, as the change says.

All five coder gaps are decided. The dependency input and the labels (D-1,
FR-001) are decided. So are the bare-hex digest held as a `DigestRecord` claim
(D-2, consistent with ADR-013 O-02 and QSpec FR-307), the string library
identity (D-3, consistent with FR-322), replay dependency entries (D-4,
consistent with FR-323) and E3 typing through the dependency's `CheckedGraph`
by lookup (D-5). Layering holds. The resolution runs in layer-6 spine code,
and E3 receives only layer-3 values (`ImportView`, `CheckedGraph`). No
`NodeKey` is minted from a wire id, and no wire data is typed as checked
(R-10, FB-03, FB-13).

Three findings are high. First, the resolution refuses a conflicting diamond
as `stale_dependency`, but QSpec FR-307 requires
`invalid_package`/`conflicting-definition`. Second, FR-098-AC-7 and TC-444
expect `invalid-value` for a removed dependency entry, but D-4's own order
refuses `missing_import` first. Third, D-5's rule for a dependency-declared
type in the importing graph cannot pass FR-322's identity recomputation or its
occurrence rule. The medium findings cover the identity of a source owner
shared across one compile, unnamed refusal shapes in the spine and in replay,
resource bounds, and the text #445 leaves behind.

## Verdict

REVISE. FND-001 to FND-003 change decided rules or make an AC fail against
the code the ADR specifies. Fix them before implementation starts. The medium
findings can go in the same revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A conflicting diamond refuses `stale_dependency`, not FR-307's `invalid_package`/`conflicting-definition` with both paths; `link_with`'s diamond rule becomes unreachable | ADR-015 D-1; FR-099 Behavior, AC-3; TC-446 step 3; QSpec FR-307 l.102-108, FR-307-AC-4; FR-087-AC-14 (#445) |
| FND-002 | high | A removed replay dependency entry refuses `missing_import` during the recompile, so FR-098-AC-7, TC-444 step 7 and QSpec BP-08 expect a refusal the ordering cannot produce | ADR-015 D-1 step 1, D-4 steps 3 and 5; FR-098-AC-7; TC-444 step 7; QSpec FR-323-AC-7, TC-277 BP-08 |
| FND-003 | high | D-5 copies a dependency-declared type into the importing graph with no `declaration` member and no occurrence, which breaks FR-322's node-id recomputation and its occurrence rules; no AC covers a declared type | ADR-015 D-5 last paragraph; FR-099-AC-5; QSpec FR-322 `declaration`, NodeBase `occurrences` minItems 1, DeclarationOccurrenceRule |
| FND-004 | medium | Nothing refuses two sources of one compile that share a `SourceOwner`, so two revisions of one owner can meet in one check, against ADR-013 O-04 | ADR-015 D-1; FR-001 amendment; ADR-013 O-04 Equality |
| FND-005 | medium | The spine has no named `CompileRefusal` variant or stage for the S4 source resolution or for a library's nested refusal, and `region` is relative to the unit | ADR-015 D-1 step 2; FR-027-AC-8; `qsl-replay/src/spine.rs:36-81`; FR-091-AC-24 (#445) |
| FND-006 | medium | Replay refusal variants are unnamed. Stale bytes reach `ReplayRefusal::Recompile`, while an entry-only `package_id` change and a list mismatch need variants that do not exist | ADR-015 D-4; FR-098 Behavior, AC-6, AC-7; `qsl-replay/src/execute.rs:59` |
| FND-007 | medium | Once #445 lands, its text says E3 refuses every import and uses a `sha256:` import digest, which this change contradicts | #445: FR-091 "Unsupplied import", FR-091-AC-24; ADR-011 §2.4 QSL-255 paragraph, OQ-5; FR-087 Status AC-13, AC-14; FR-098 Status; TC-379; tests.md |
| FND-008 | medium | The stage limits of each library compile, and the bound on the number of libraries, are not stated for the CLI or replay | ADR-015 D-1 step 2; FR-099 Behavior; FR-027 Inputs; ADR-013 QC-8 |
| FND-009 | low | ADR-011's I2 row and FR-087-AC-6 still say a `WireNodeId` becomes a `NodeKey` by lookup only at E4 and E9, and the E4 row says "digest-addressed source" | ADR-011 §2 I2 row, E4 row; FR-087-AC-6; TC-255; ADR-015 Amendments |
| FND-010 | low | An empty identity or version in the CLI `libraries` or a library caller's input has no named refusal | ADR-015 D-3; FR-099-AC-4; FR-027 |
| FND-011 | low | FR-087-AC-11 still requires `LibraryName` and `ImportDeclaration` to keep their old shapes, which D-2 and D-3 change | FR-087-AC-11; ADR-015 D-2, D-3; `qsl-semantics/src/library/mod.rs:91,180` |
| FND-012 | low | D-4 step 5 does not say what happens to an entry whose identity is not in the recompiled closure | ADR-015 D-4 step 5; FR-098 Behavior |
| FND-013 | low | FR-071-AC-9 and TC-186 step 7 test an empty identity but not an empty version | FR-071 Behavior, AC-9; TC-186 step 7 |

## Finding details

### FND-001 Conflicting diamond refuses with the wrong code

D-1 supplies one library per identity, compiles it once and compares every
import of that identity with that one compile. Take a unit that imports `L`
at version 1 and a dependency `A` that imports `L` at version 2. They reach
`L` by two paths that name different versions, which is FR-307's conflicting
diamond: `invalid_package`/`conflicting-definition`, with both dependency
paths. Under D-1 step 1, the path whose version differs from the supplied
library refuses `stale_dependency`/`revision-mismatch`. Two different
recorded digests refuse `byte-digest-mismatch` at step 3. Either way the
code is not the one FR-307 requires, and the refusal names one import, not
both paths. #445's `link_with` diamond rule (FR-087-AC-14) never runs,
because the resolution refuses first.

Fix: add a step before D-1 step 1 and a matching FR-099 Behavior bullet.
"When an import names an identity that an earlier import in the closure
already named, in depth-first source order, with a different version or
recorded digest, the resolution refuses
`invalid_package`/`conflicting-definition`, naming both dependency paths.
It does not compare the later import with the supplied library." Only the
first import of an identity is compared with the supplied library. Add to
FR-099-AC-3 and TC-446 step 3 a case where the unit imports `test/geometry`
at version `1` and `test/a` imports it at version `2`. The expected result is
`invalid_package`/`conflicting-definition` naming both paths. In D-1, say
that `link_with`'s diamond check stays as the E4 invariant for direct
callers.

### FND-002 A removed replay entry cannot reach `invalid-value`

D-4 builds the dependency input from the entries and then recompiles (step
3). With the only entry removed, the proved source's import finds no
supplied library, and D-1 step 1 refuses `missing_import`/`missing-selection`
inside the recompile. Step 5, which would give `invalid-value` at
`/package/dependencies`, is never reached. FR-098-AC-7 ("or lack one entry"),
TC-444 step 7 ("then remove the entry … refuses `invalid_package`/`invalid-value`")
and QSpec TC-277 BP-08 all expect `invalid-value`. FR-323 has the same
wording.

Fix: in FR-098-AC-7 and TC-444's expected results, the removed entry refuses
as a recompile refusal (`ReplayRefusal::Recompile`) carrying
`missing_import`/`missing-selection` at the import. Keep the swapped-entries
case, and add a case with an extra entry naming an identity nothing imports.
Both refuse `invalid_package`/`invalid-value` at `/package/dependencies`. In
D-4, add after step 3: "An entry missing for an imported identity refuses
there, as `missing_import`/`missing-selection` at the import." Ask the QSpec
change to do the same in FR-323 and BP-08. The `invalid-value` refusal covers
an entry list that differs from a recompile that succeeded.

### FND-003 Dependency-declared types in the importing graph

The last paragraph of D-5 lowers a type that the importing package holds
only through an imported signature "under the key the dependency's graph
gives it". It says that node carries no `declaration` member. That breaks
FR-322 in two ways:

- `declaration` "enters the node identity preimage" (FR-322 `declaration`).
  A strict reader "recomputes all required raw and semantic digests". A
  declared record copied without its `declaration` recomputes to a different
  id than the key it carries, so IR refuses the package.
- NodeBase requires `occurrences` with at least one item. The
  DeclarationOccurrenceRule allows a `declaration` member only with a
  `declaration`-role occurrence, which the importing unit does not have.

The Boolean result in FR-099-AC-5 is a builtin shared by content, so no AC
tests this. The rule also does not say whether the types that a copied type
references (field types, element types) are copied with it.

Fix: rewrite the paragraph to decide it:

1. The copied node keeps the dependency's `declaration` member, so its id
   recomputes.
2. It carries a `type`-role occurrence at each use in the importing unit
   whose type it is.
3. Every node it references is copied by the same rule.

Add an ADR-013 QC row asking QSpec to let the DeclarationOccurrenceRule admit
a `declaration` member without a `declaration` occurrence when the node's
owner is not one of this package's `sources`. The QSpec change's FR-322 text
("which the referencing node's `result_type` names by node id in the
importing graph") needs this rule anyway. Add FR-099-AC-7 and a TC-446 step:
`test/geometry` declares `record R { x: Int[0, 9] }` and
`function mk using v(x: Int[0, 9]): R pure { … }`, and the unit calls
`g::mk(y)`. `R`'s node id in the emitted package equals its id in
`test/geometry`'s graph, and IR's v2 reader admits the package.

### FND-004 One source owner in two roles

D-1 takes each library's four labels from its supplier. It never checks that
a library's `SourceOwner` (authority, identity) differs from the importing
unit's or another library's. ADR-013 O-04 relies on "within one check and one
replay … one `package_id` per owner, so two revisions of one owner never meet
in one check". Suppose a library is supplied under the unit's own labels at
another revision. Then a declaration in each source with the same qualified
name and structure shares one `NodeKey`, and D-5's lookup and its type
copying cannot tell them apart.

Fix: add to D-1 and FR-099 Behavior: "Two sources of one compile, whether the
unit or a supplied library, that share authority and identity refuse
`invalid_package`/`conflicting-definition` when the dependency input is
built, naming both library identities or the unit." Add the case to
FR-099-AC-3 and TC-446 step 3.

### FND-005 Spine refusal shape for the resolution

`CompileRefusal` (`qsl-replay/src/spine.rs:36`) has one variant per stage.
Each variant has a `region` inside the unit's own `RawSourceRef`, and CLI
`compile` reports the stage (FR-027-AC-8: `source`, `forms`, `assembly`,
`check`, `emit`, plus `intake`). D-1 does not say which stage reports a
resolution refusal. It also does not say how a library's own refusal
("located in the library's own source") is carried. #445 reports
`missing_import` at stage `assembly` (FR-091-AC-24).

Fix: in D-1 and FR-099, decide:

- The resolution's own refusals are `CompileRefusal::Intake` at stage
  `intake`. The resolution, like I1, admits package inputs between S2 and E3.
- A library's refusal is a new
  `CompileRefusal::Dependency { path: Vec<LibraryName>, refusal: Box<CompileRefusal> }`.
  It reports the inner stage, and the inner region carries the library's
  `SourceIdentity`.

Amend FR-027-AC-8 to list this case. Move #445's FR-091-AC-24 to the same
stage (FND-007).

### FND-006 Replay refusal variants

FR-098 says each refusal is "a typed `ReplayRefusal` variant". Under D-4:

- A stale dependency's bytes refuse inside spine `compile`, so they reach the
  caller as `ReplayRefusal::Recompile`, not as a `DependencyIdentityMismatch`
  variant.
- An entry whose `package_id` alone is changed (step 5) needs a variant that
  does not exist.
- A list mismatch needs one too.

TC-444 step 7 names the code but not the variants, so it can pass on any
shape.

Fix: in FR-098 Behavior and TC-444's expected results, name three variants:

- `ReplayRefusal::Recompile` wrapping the spine's `DependencyIdentityMismatch`
  for stale bytes;
- a new `ReplayRefusal::DependencyIdentityMismatch { identity: LibraryName, recorded: DigestRecord, recompiled: PackageId }`
  for step 5's first check;
- a new `ReplayRefusal::DependencySelections` for
  `invalid_package`/`invalid-value` at `/package/dependencies`.

### FND-007 Text left by #445

The change avoids #445's lines on purpose. After both land, these
statements contradict ADR-015:

- FR-091's "Unsupplied import" bullet ("Spine `compile` takes no dependency
  input … so every `import` refuses").
- FR-091-AC-24, which uses `digest "sha256:…"`. D-2 refuses that at S1 with
  `invalid_digest`, before assembly.
- ADR-011 §2.4's "Amended (2026-09-25, QSL-255)" paragraph ("E3 still
  refuses a unit that declares an `import`") and OQ-5.
- FR-087 Status AC-13, and AC-14's "belong to QSL-255 part (b)".
- #445's FR-098 Status paragraph ("does not yet say which of its
  package-reference sources is the proved package").
- TC-379's Status and the TC-379 and TC-405 rows in `tests.md`.

Fix: after rebasing onto #445, in the same PR:

- Rewrite FR-091's bullet: an import whose library the dependency input does
  not supply refuses (ADR-015 D-1).
- Change FR-091-AC-24's digest to bare 64-hex, "with no library supplied as
  `test/units`", at the FND-005 stage.
- Replace the §2.4 paragraph, OQ-5's last sentence, the FR-087 AC-13 and
  AC-14 status text, the FR-098 Status paragraph and TC-379's Status with a
  pointer to ADR-015 and FR-099.
- Mark TC-379 unblocked.

### FND-008 Resource bounds of the resolution

D-1 step 2 compiles each library "under the same stage limits". It does not
say whether each library gets the full per-unit limits or shares one budget
with the unit. With per-unit limits, total work grows with the number of
libraries. Nothing bounds that number on the CLI: FR-027 does not say whether
library sources count toward FR-026's file-count and aggregate-byte limits.
Replay copies the proving run's limits (ADR-013 QC-8), so a different
reading in CG and QSL gives a different refusal.

Fix: in D-1 and FR-099, state that each library's compile is charged the
full S1 to S4 stage limits as its own unit, exactly as the importing unit is.
Recursion depth and the number of compiles are bounded by the number of
supplied libraries, since each compiles at most once. In FR-027 Inputs, say
that each `libraries` source counts toward FR-026's file-count and
aggregate-byte limits. In D-4, say that the replay request's reader bound
bounds the entries.

### FND-009 Stale lookup and E4 wording in ADR-011 and FR-087

The ADR-015 Amendments update ADR-011's E3 row and ADR-013 O-04. They leave
three texts:

- ADR-011's I2 row says "A `WireNodeId` becomes a `NodeKey` only by lookup in
  a checked package: at E4". FR-087-AC-6 and TC-255 name the lookup paths
  `library`, E4 and E9.
- ADR-011's E4 row still says each dependency is "compiled from its
  digest-addressed source". An ordinary compile's library sources are not
  digest-addressed.

Fix: add three amendments to ADR-015 and apply them:

- the I2 row reads "at E3, in the dependency's checked graph, and at E4, in
  its checked package";
- FR-087-AC-6 adds E3's lookup in `check` as a permitted, non-minting path;
- the E4 row reads "compiled from its source (ADR-015 D-1)".

### FND-010 Empty identity or version in a supplied library

D-3's constructor refuses the empty string, and FR-099-AC-4 tests only the
constructor. No refusal is named for an empty identity or version that
reaches a compile, through a library caller's dependency input or the CLI's
`libraries`.

Fix: in FR-027, a `libraries` entry with an empty identity or version refuses
with `invalid-request`, exit 20. In FR-099, dependency-input construction
refuses it with `invalid_identifier` (`HostCause::SelectionIdentity` or
`SelectionVersion`, both existing). Add both cases to TC-446.

### FND-011 FR-087-AC-11 fixes the old shapes

FR-087-AC-11 requires `LibraryName` and `ImportDeclaration` to be "unchanged
in shape" after the move to `library`. D-3 changes `LibraryName` from
`Box<[String]>` segments to one string, and D-2 changes
`ImportDeclaration.package_id` from `PackageId` to a `DigestRecord`
(`qsl-semantics/src/library/mod.rs:91,180`). A TC-281 check that reads AC-11
literally would fail the implementation.

Fix: add "except as ADR-015 D-2 and D-3 reshape `ImportDeclaration` and
`LibraryName`" to FR-087-AC-11, and list it in ADR-015's Amendments.

### FND-012 An entry outside the closure

D-4 step 5 first compares "each entry's `package_id` … with the recompiled
closure's selection of the entry's identity". An entry whose identity nothing
imports has no such selection, so the first check has no defined result.

Fix: reorder step 5. First, "an entry naming an identity absent from the
recompiled `dependency_selections` refuses `invalid_package`/`invalid-value`
at `/package/dependencies`". Then the per-entry `package_id` check. Then the
order check.

### FND-013 Empty version at decode

FR-071 Behavior refuses "an empty identity or version" at decode. AC-9 and
TC-186 step 7 test only the empty identity.

Fix: add "or an empty version" to FR-071-AC-9 and to TC-186 step 7 and its
expected result.

## Dispositions

Fixed in the commit that follows this review, except where stated.

- FND-001: ADR-015 D-1 step 0 and an FR-099 Behavior bullet refuse a
  conflicting diamond `invalid_package`/`conflicting-definition` naming both
  paths; FR-099-AC-3 and TC-446 step 3 cover it.
- FND-002: FR-098-AC-7, TC-444 step 7 and D-4 step 3 expect
  `missing_import`/`missing-selection` for a removed entry; an extra entry
  refuses `invalid-value`. QSpec FR-323 and TC-277 carry the same rule.
- FND-003: resolved by the QSpec rule (FR-322, FR-322-AC-37): an imported
  name E3 accepts is a function whose signature types carry no
  `declaration`, so no declared type is copied; any other use refuses
  `ill_typed`/`operator-ineligible` (D-5, FR-099-AC-5, TC-446 step 5).
- FND-004: the dependency input refuses a second source of one owner.
- FND-005: the resolution reports stage `intake`; a library's own refusal
  is `CompileRefusal::Dependency { path, refusal }`.
- FND-006: `ReplayRefusal::Recompile`, `DependencyIdentityMismatch` and
  `DependencySelections` are named.
- FND-007: deferred to the implementing PR, because the lines exist only in
  PR #445. After rebasing onto #445, the coder rewrites FR-091's "Unsupplied
  import" bullet and FR-091-AC-24 (bare-hex digest, no library supplied,
  stage `intake`), and points ADR-011 §2.4's QSL-255 amendment, OQ-5's last
  sentence, FR-087's AC-13 status, FR-098's status paragraph and TC-379's
  status at ADR-015 and FR-099.
- FND-008: per-unit limits, the library-count bound, FR-026 file limits and
  the FR-071 reader bound are stated.
- FND-009: the E4 row and ADR-015's amendments (FR-087-AC-6) are updated.
- FND-010: `invalid_identifier` in the dependency input, `invalid-request`
  in the CLI, both in TC-446.
- FND-011: listed in ADR-015's amendments; FR-087-AC-11's text is left to
  the implementing PR, since #445 edits the adjacent lines.
- FND-012: D-4 step 5 and FR-098 order the checks as suggested.
- FND-013: FR-071-AC-9 and TC-186 step 7 add the empty version.
