---
id: SR-1275
title: "Code review of quire-spec-language HL1: an import names its library by identity alone"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4965fe5daac7032cd45d86a39f00857b23db9b96; HL1's own commits only, git diff 2f0581fe0..4965fe5da (2f0581fe0 is LC1's head): qsl-cst/src/grammar.rs, qsl-cst/src/parser.rs, qsl-foundation/src/selection.rs, qsl-semantics/src/library/mod.rs, qsl-package/src/checked.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, and their tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
---
# Code review of quire-spec-language HL1

## Summary

Ticket: QSL-471 (HL1), with QSL-40 and QSL-39. There is no PR yet: HL1 is
stacked on the unmerged LC1 branch. Reviewed HL1's three commits
(6f5b4108b, 2be02efa6, 4965fe5da) on `task/471-hl1-header-identity`. The Rust
lane (rust-review) is folded into this file.

Rulings applied, not raised: the header is identity only; imports become
`import "L" [as a];` with no compat path; `SuppliedLibrary.version` and the
replay-request dependency versions are A2's (QSL-395).

What the change does, checked against the code:

- S1: `ImportDeclaration` is `import Identity OptionalAlias ;`. A trailing
  `version` or `digest` token is now an `invalid_syntax` error, and no
  selection is recorded. `ImportSelection` drops `version` and `digest`.
- `ImportDeclaration`, `Import`, `VisitedImport`, `ImportRefusal::{Diamond,
  RevisionMismatch, DependencyIdentityMismatch}`,
  `LinkRefusal::DependencyIdentityMismatch`, `StalePin::Import`, and
  `Library{Refusal,Cause}::ConflictingDefinition` are deleted. Each one existed
  only to compare an import's written version or digest with the library.
  With one library supplied per identity (`DependencyInput` refuses a second),
  the spine resolution can no longer see two selections of one identity, so
  dropping `Diamond` loses nothing on the spine path. `LinkRefusal::
  ConflictingDefinition` stays at E4 as FR-307's diamond rule over closures.
- The spine resolution reuses a library compiled earlier in the closure and
  hands E4 the supplied version. The lock's `package_id` is the recomputed one.

Focused tests run at 4965fe5da (`cargo test -p qsl-replay -p qsl-semantics
-p qsl-cst -p qsl-package` filtered to dependency, library, selection,
import, diamond and tc_444): all pass, no warnings.

Rust lane: no new `unwrap`, `expect`, panic or `unsafe` on a production path,
and no wildcard arm. No new limit, pin or compatibility layer.

## Verdict

Request changes for one medium finding. The deletions are version-claim
ceremony, as the ruling says, with one exception: `resolve_libraries` lost
the only thing that told two supplied packages of one identity apart, and
now silently picks the first. The replay-ordering finding, which is a spec
contradiction, is in SR-1277 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `resolve_libraries` selects "the first supplied package" of an import's identity. Given two supplied packages of one identity with different `package_id`s, it silently binds the first and ignores the second. Before HL1 the import's digest picked the right one. The comment's claim that duplicates are refused earlier by `DependencyInput` does not hold here: `resolve_libraries` takes a bare `&[LibraryPackage]` and no `DependencyInput` runs before it. Commit 2be02efa6 shows the defect: `l07_migration_creates_new_identities_and_never_relabels_evidence` used to supply `[old_library, new_library]` and expect the new one, and it now supplies only `new_library` so the test passes. Fix: refuse a second supplied package of an identity already held (the `DependencyInput` rule, `invalid_package`/`conflicting-definition`), and test it with the old two-package `l07` input. | qsl-semantics/src/library/mod.rs:930-936; qsl-semantics/tests/it/library_resolution.rs:361 |
| FND-002 | low | With `StalePin::Import` removed, `StalePin` has one variant, `Pinned(Box<PinMismatch>)`. Every match on it is now irrefutable. `LibraryRefusal::StaleDependency.pin` can hold `Box<PinMismatch>` directly. | qsl-semantics/src/library/mod.rs:308-315 |

## Dispositions

Round 1, reviewed at ad18594412cb4cdd849104a7643491b231cd79ee. HL1 is now
rebased onto LC1 #624 head ddd162c7. `git range-diff` shows the three
original commits unchanged (cae1d1588, be1926420, 62aa2f7d1), and fix commit
ad1859441 touches only the HL1 files and the three SR files. Focused tests pass,
including `l07_migration_creates_new_identities_and_never_relabels_evidence` and
`conflicting_definition_classifies_to_i2_rule_2`, and `cargo fmt --check` is
clean. This round adds no new finding.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | ad1859441 |
| FND-002 | fixed | ad1859441 |
