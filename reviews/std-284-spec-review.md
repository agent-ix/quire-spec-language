---
id: SR-16100
title: "Spec review — editor profile check admits the header-selectable layers (FR-110)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md at 58912552a"
review_set: subset
evaluated_revision: "58912552a48c20718ffb985146315db973b2d52c"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-490
    type: reviews
---

## Summary

Ticket: STD-284 (unblocks QSL-826). PR quire-spec-language#1270 adds FR-110
`### Editor profile check` (the production catalog `complete::header_profile_catalog()`,
admission and refusal agreement with E3), FR-110-AC-12 and TC-490 step 11. Reviewed
`git diff origin/main...HEAD` at 58912552a against FR-110 (AC-2, AC-7, Header-selectable
layers) and QSL main code: `qsl-foundation/src/selection.rs` (`ProfileCatalog`),
`src/complete/editor.rs` (`profile_refusal`, `validate_catalog_profiles`),
`src/complete/mod.rs` (`parse_with_catalog`) and `qsl-semantics/src/value/definition.rs`
(`DefinitionLock::pinned`, `layers`).

Examined: the Editor profile check paragraph and its two SHALL bullets, FR-110-AC-12 and
TC-490 step 11 with its expected result. `complete::header_profile_catalog` exists in no
crate on main or on any open PR head; the PR body flags the name as a lead default, and a
spec may mandate a new public function, so the name itself is not a finding. The four
editor entry points the paragraph names exist on main and take a caller-supplied
`ProfileCatalog`, as stated. FR-110-AC-12 is unique across main and all 24 open PR heads.
`make check-spec-validation` exits 0. Wording is behaviour-only. The head is behind
origin/main but merges into it and into every other open QSL spec PR without a textual
conflict (`git merge-tree`).

## Verdict

NOT MERGEABLE as written: two medium findings, both small fixes in this PR.
`ProfileCatalog` is a `BTreeSet` with only `new` and `contains`, so "holding exactly the
identities ... in that order" names an order the type cannot hold and an exactness no test
can observe. The `qualification_catalog` clause, the one case where the editor and E3 give
different causes, has no AC. The production-catalog-as-library-default decision and the
FR-110 home are consistent with the ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-110-AC-12 and the Editor profile check paragraph require the catalog to hold exactly the identities of `DefinitionLock::pinned().layers()` "in order". `ProfileCatalog` (qsl-foundation/src/selection.rs:253) stores a `BTreeSet<String>` and exposes only `new` and `contains`, so it holds no order and a test cannot list its contents: "in order" cannot hold, and "exactly" cannot be checked beyond `contains` on chosen identities. Drop "in that order" and state exactness as behaviour (each layer identity admitted, every non-layer identity refused, as the AC's later clauses do), or have the spec require an accessor that lists the identities. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:154-161,254; qsl-foundation/src/selection.rs:253-290 |
| FND-002 | medium | The refusal bullet says a `qualification_catalog` row's identity, which E3 refuses as `wrong-selection-role`, receives `unsupported-selection` in the editor. This is the one case where editor and E3 disagree on the cause, and FR-110-AC-12 and TC-490 step 11 test only `test:unknown-profile`. Add a clause to AC-12 and step 11: a unit whose header profile names a `qualification_catalog` identity reports `unknown_profile`/`unsupported-selection` at the identity literal through `parse_with_catalog` with the production catalog. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:165-169,254; spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md:63 |

## Dispositions

Round 1, checked against fix commit 8e3fee2ca (`git show 8e3fee2ca`; the branch also merged origin/main, which is not this PR's change). `make check-spec-validation` exits 0; main is an ancestor of the head; no textual conflict with any current open PR (stale PRs #667-#995 conflict with main itself). The `ieee_profile` row is a `qualification_catalog` row whose identity E3 refuses `wrong-selection-role` (FR-110 table row 98, AC-3), so the new clause exercises the case FND-002 named.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Order dropped; the statement says the catalog contains each layer identity and no other, and AC-12 checks it through `contains` (each layer identity true; `test:unknown-profile` and every `qualification_catalog` identity false). |
| FND-002 | fixed | AC-12 and TC-490 step 11 add the `ieee_profile` (qualification_catalog) unit: `unknown_profile`/`unsupported-selection` at the identity literal, against E3's `wrong-selection-role`. |
