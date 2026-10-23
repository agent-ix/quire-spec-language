---
id: SR-16234
title: "spec-review of FR-480 linking several source units into one checked package (quire-spec-language PR 1345)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c93138ae; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md; spec/test-cases/TC-1210-compile-link-and-replay-a-package-of-several-source-units.md; spec/functional/FR-098-execute-a-replay-request.md; spec/functional/FR-278-parse-select-check-and-package-as-library-operations.md; spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md; spec/test-cases/TC-472-invariant-violations-are-recorded-and-undecided-expansions-stop.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
---

# Spec review of FR-480 linking several source units into one checked package (quire-spec-language PR 1345)

## Summary

Ticket: STD-384 (QSL-939). This is one spec-review pass over
`git diff origin/main...c93138ae`. It covers the new FR-480 and TC-1210, the
amendments to FR-098, FR-278, FR-362, FR-096, FR-091 and ADR-033, and
FR-120's new AC-19 and AC-20 with the removal of its "Remaining work". The
oracles were QSpec FR-151-AC-16 and AC-19, TC-196 D10, FR-142 "source
order", TC-187 U11q, FR-322 (`SourceOwner`, AC-51 to AC-53, occurrence
ordinals outside the identity projection) and FR-323 (`sources`), together
with QSL FR-092 and FR-093 (occurrence ordinals) and FR-027.

Gates: `make check-spec-validation` at c93138ae exits 0, with no finding on a
touched file. The PR merges cleanly with origin/main, with #1344 and with
#1326, but origin/main is not an ancestor of the head. FR-480, its AC-1 to
AC-8, TC-1210 and FR-120-AC-19 and AC-20 are unique across open PRs.

The question was whether canonical unit order and request order can give
two different packages for one input. Admission or refusal does not depend
on the order: every check refuses on any defect, and the order only picks
which defect is reported (U11q, which matches QSpec FR-142). `package_id`
does not depend on the order either, because node keys are owner-based and
occurrences are outside the identity projection. The package bytes can
depend on the order (FND-001).

Examined, clean:

- **AC-1** follows D10 and FR-151-AC-19.
- **AC-6** follows U11q in both orders: the reworked `z` still hits row 1
  first.
- **AC-5** follows FR-322-AC-53: a node moved to another owner gets another
  key and another `package_id`.
- **Alias locality** follows FR-151-AC-16.
- **FR-120's "Remaining work" removal** is backed by AC-19 and AC-20.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The package bytes depend on request order, which contradicts FR-480's claim and AC-1. FR-093 numbers a node's occurrence ordinals "in insertion order: the order `check` records them in", and FR-092 orders reference occurrences "by source position". FR-480 defines source order as request order and has `check` take the units in request order. It puts only the writing of nodes in canonical unit order. A node shared by two units gets ordinals and `source_map` rows numbered in request order, so `[U1, U2]` and `[U2, U1]` emit different bytes with one `package_id`. AC-1's own fixture has such a node: the `false` body of `pre PA` (U1) and of `pre PB2` (U2), and shared `self` parameter nodes. The rules as stated do not support AC-1's "byte-equal packages". Fix: state in FR-480 that occurrences, ordinals and `source_map` rows across units are numbered in canonical unit order, then in insertion order within a unit. Amend FR-093's and FR-092's ordinal rules to match, and add a vector over a shared node. | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:86-96,165-172; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:72-81 | wrong-requirement |
| FND-002 | high | Scope-deferral: FR-027 is left describing one program source. The PR body says "FR-027 is not edited, to avoid colliding with #1326's rewrite". FR-480 redefines FR-027's native-compile/1 `program` as "one source selection, or a non-empty array". FR-027 still says the command compiles "the selected program source" and that "Each program source goes to exactly one compiler". It also does not say what an array means for a `0-draft` program. The command half of the ticket is split away from the requirement that owns it. Under the standing rule this blocks the PR. Fix: amend FR-027 here, coordinating the text with #1326, and specify the `0-draft` case. | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:67-70; spec/functional/FR-027-export-compiled-native-package.md:23,103-107 | missing-requirement |
| FND-003 | medium | Canonical order for `sources` is a QSL-only rule that refuses references QSpec admits. QSpec FR-322 and FR-323 give no order for `lock.sources` or a package reference's `sources`: FR-323 says only that `sources` "is the proved package's own FR-322 lock `sources`". FR-098 now refuses an out-of-order list with `invalid_package`/`invalid-value`. That includes a list from another FR-322 producer, or one written in request order. Fix: open a QSpec STD ticket that fixes the order of `lock.sources` and reference this rule from it. Otherwise canonicalize on recompile instead of refusing. | spec/functional/FR-098-execute-a-replay-request.md:105-113,579; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:181-190 | wrong-requirement |
| FND-004 | medium | "A unit SHALL NOT import another unit of its own package" has no refusal of its own. AC-2 tests only the incidental case, where the dependency input does not supply the library. Behavior is unspecified when the dependency input supplies a library whose (authority, identity) equals a sibling unit's: compile against it, refuse, or shadow. Fix: give the rule a code, cause and locus, and add the supplied-library vector. | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:135-136,197 | missing-requirement |
| FND-005 | low | Two wording defects. ADR-033's amendment embeds process language ("lead default, not an owner answer; Peter may veto") in spec content. That flag belongs in the PR body, and the spec should state the behavior and its rationale. FR-480-AC-1's "and with `PB1` in `U1` and `PB2` in `U2`" does not say that this placement, which is the QSpec D10 split, also runs in both link orders. TC-472 step 7 and FR-120-AC-19 say "in both link orders"; AC-1 should say the same. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:179-184; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:196 | wrong-requirement |

## Verdict

Not mergeable. FND-001 means FR-480's order-independence claim and AC-1's
byte-equality do not follow from the stated rules. FND-002 is a scope-deferral
of FR-027's command shape. FND-003 and FND-004 are real gaps. The namespace,
alias, header, ownership and U11q parts are correct and consistent with
QSpec. A main merge is needed (clean).

## New findings (disposition pass 1)

Re-checked at 5ff99f20 (fix commit 411029e1; origin/main is an ancestor;
`make check-spec-validation` exits 0).

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | high | The FR-027 amendment copies open #1303's FR-027 (STD-343, stacked on #1326 STD-355 and #1321 STD-352) and adds this PR's array rule. It brings in three other tickets' changes without the rest of their PRs. It deletes `0-draft` compilation ("Every other declared edition, `0-draft` included, ... `command` refuses it with `unknown_edition`"). It removes FR-027-AC-1, AC-4 and AC-6, retitles the FR, and drops FR-019 and FR-020. It moves stream behaviour to the driver and adds AC-13. Merged alone, this head leaves dangling and contradicting references. TC-430's scope still lists FR-027-AC-4. TC-435 still traces FR-027-AC-6 (step 3, a `0-draft` compile). Main's `tests/it/compile_command.rs` still traces FR-027-AC-1, AC-4 and AC-6 over `0-draft` programs that the command compiles. The head also conflicts textually with #1303, #1326 and #1321. Fix: amend FR-027 from main's text with only this PR's change: the array `program`, the per-unit digest and edition rule, the unit-list `dependent_bytes` rule and AC-14. State the `0-draft` case against main's behaviour (a unit list is `1-draft` only, so a `0-draft` unit in an array refuses). Otherwise stack this PR on #1303 once it merges. | spec/functional/FR-027-export-compiled-native-package.md:1-140; spec/test-cases/TC-430-native-requests-and-outputs-carry-the-four-source-labels.md:23,42; spec/test-cases/TC-435-cli-compile-routes-a-program-by-its-declared-edition.md:46 | wrong-requirement |

## New findings (disposition pass 2)

Re-checked at 2e715ed1 (fix commit e507f3bc).

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-007 | medium | One input gets two refusal codes. FR-027-AC-14 says an array whose second unit declares `edition "0-draft"` beside a `1-draft` first unit refuses `unknown_edition` at that unit's edition literal. FR-480-AC-4 (TC-1210 step 4) says a unit whose header declares `0-draft` beside a `1-draft` unit refuses `invalid_package`/`conflicting-definition` at the later unit's header, naming both headers. Neither AC says which entry point it goes through: the command, whose edition check runs before any compile, or the spine `compile` and FR-278 operations, which apply FR-480's header agreement. An implementer can't tell which code a `[1-draft, 0-draft]` unit list returns. Fix: state that FR-027's per-unit edition check runs before FR-480's header agreement, and that FR-480-AC-4 goes through spine `compile` (no command edition check). Or give FR-480-AC-4 a header mismatch that is not an edition, such as differing profile selections, and keep the `0-draft` case in FR-027-AC-14 alone. | spec/functional/FR-027-export-compiled-native-package.md:113-120,149; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:217 | wrong-requirement |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-480 numbers occurrences, ordinals and `source_map` rows across units in canonical unit order, with insertion order within a unit. FR-093 and FR-092 are amended to match. AC-1 adds the shared `false` body node, numbered `U1`'s first in both orders. Byte-equality now follows from the stated rules. |
| FND-002 | fixed | FR-027 now states the array `program`, the per-unit digest and edition check in request order, the refusal of a `0-draft` unit and of an empty array, and the unit-list `dependent_bytes` rule, with FR-027-AC-14 (TC-1210 step 9). The way it was amended introduces FND-006. |
| FND-003 | fixed | FR-098 and FR-480 accept `sources` in any order and recompile it in canonical unit order. The out-of-order refusal is removed, and AC-7 expects the reversed reference to replay. |
| FND-004 | fixed | A dependency-input library whose source has a unit's (authority, identity) refuses `invalid_package`/`conflicting-definition` before any parse, naming the unit and the library entry. AC-2 and TC-1210 step 2 carry the vector. |
| FND-005 | fixed | ADR-033's process language is removed and replaced by the FR-151 and FR-322 rationale. AC-1 says "again in both link orders" for the second placement. |
| FND-006 | still-open | New in this pass. It needs a fix round. |
| FND-006 | fixed | FR-027 is now main's text plus only the unit-list paragraph, the per-unit digest and edition rule, the array `dependent_bytes` sentence and AC-14. `0-draft` compilation and FR-027-AC-1, AC-4 and AC-6 are unchanged, so TC-430, TC-435 and `tests/it/compile_command.rs` resolve. FR-027-AC-14 does not collide with #1303's or #1321's AC-13. |
| FND-007 | still-open | New in this pass. It needs a fix round. |
| FND-007 | fixed | FR-027 states that its per-unit edition check runs before FR-480's header agreement, and FR-480 "Headers" says the agreement check runs in the spine compile, after parse. A command request that mixes `1-draft` and `0-draft` units refuses `unknown_edition` and never reaches it. FR-480-AC-4 (TC-1210 step 4) now goes through FR-278's composition with non-edition mismatches (profile order, profile set). FR-027-AC-14 goes through `command`. Each input now has one code. |

Verdict after disposition pass 1: FND-001 to FND-005 are fixed. Not mergeable
until FND-006 is fixed.

Verdict after disposition pass 2: FND-001 to FND-006 are fixed, `make check-spec-validation` exits 0 at 2e715ed1, and the head merges cleanly with origin/main 28a76640. Not mergeable until FND-007 is fixed.

Verdict after disposition pass 3: every finding is fixed at 03f7e1be. `make check-spec-validation` exits 0, and origin/main cee6127d is an ancestor. Mergeable.
