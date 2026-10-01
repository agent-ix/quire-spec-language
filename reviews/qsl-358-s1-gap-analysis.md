---
id: SR-953
title: "QSL-358 slice 1 gap analysis of PR 565: quire-semantic-value against the slice goal"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a7df1ff07f56e2b37249841138600c86c1b3d1b5; QSL-358 slice 1 items (create quire-semantic-value; move stop, quantity, unit runtime half, semantic_node vocabulary; ADR-011 SV/FB-05/T-12/X-11; arch-lint SHARED_LEAVES; T12-B design only); PR title and body; FR-059-AC-9, FR-060-AC-5, FR-061-AC-5, FR-068-AC-6 and their tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-061
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 1). PR: quire-spec-language#565 at a7df1ff0.

Slice items against the diff:

- **Crate created.** `quire-semantic-value` is a workspace member, `#![no_std]`,
  with the exact dependency set TC-390 asserts, and a `make ci` thumbv7em build.
- **Moved.** `stop` and `quantity` (git sees renames, 82% and 97% similar), the
  runtime half of `unit`, and the `semantic_node` vocabulary
  (`InvalidSemanticGraph`, `SemanticGraphCause`, `check_terms`, `IDENTITY_LIMITS`).
  The compile-side half (preimage readers, digests, `admit_unit_graph`) stays in
  `qsl-semantics`. Every caller imports from `quire_semantic_value::` directly.
- **ADR-011 amended.** SV row, crate map, FB-05 class, SV bullet, §6.2 rows,
  §7.1 graph and edge row, X-11, T-12 and the T12-B design. Accuracy issues are in
  the spec review (SR-954).
- **arch-lint.** `SHARED_LEAVES` replaces `KERNEL_LEAF` in edge extraction only;
  `classify` is unchanged, so FR-061 still counts both leaves. The api-surface,
  canonical-encoder, string-edge and TC-390 scans all add
  `quire-semantic-value/src`.
- **T12-B carve-out.** Designed in T-12, not built. No `decode_admitted` exists in
  `quire-exact`. Matches the slice scope.
- **containment.** Not moved; agreed for slice 2. The PR body says so.

Acceptance criteria and their tests (each test checked for a failing case, not
only a passing one):

- FR-059-AC-9: `tc_arch_lint_metadata_008` asserts the edge list is exactly
  `qsl-eval` and `qsl-semantics` and the FB-05 report flags exactly those. Would
  fail if SV were not exempt. Correct binding.
- FR-060-AC-5: `tc_arch_lint_api_surface_026` plants `NodeKey::from_digest` in
  `quire-semantic-value/src/unit.rs` and expects one T12-B violation with module
  `unit` under that crate's `src`. Would fail if the crate were not scanned.
  Correct binding.
- FR-061-AC-5: `tc_arch_lint_duplicate_revisions_008` gives two QSL git sources
  for `quire-semantic-value` and expects one QSL finding. Correct binding.
- FR-068-AC-6: the TC-175 fixtures move from `value::quantity` to `value::unit`
  (synthetic paths; the classification logic is unchanged). Correct binding.
- QSpec-FR-142-AC-5: `a_runtime_compound_unit_carries_the_golden_compound_unit_id`
  compares against the unchanged golden digest. Correct binding.

Merge with main: #561 (the `quire_contract_ir` to `quire_contract_model` rename)
has already merged as `0e3426e9`. A `git merge-tree` of a7df1ff0 against it shows
six files changed on both sides (`Cargo.toml`, `qsl-package/Cargo.toml`,
`checked.rs`, `checked_v2.rs`, `emit.rs`, `emit/tests.rs`) and no textual
conflict. The PR adds no `quire_contract_ir` identifier. Rebase and run the
pre-merge gate on the rebased head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR title still lists `containment` ("move stop/containment/quantity/unit runtime half"), but `value::containment` stays in `qsl-semantics` until slice 2, as the body says. The squash-merge commit takes the title, so `main`'s history would record a move that did not happen. Drop `containment` and add `semantic_node` vocabulary. | PR #565 title |

## Verdict

Every slice-1 item is delivered and each new AC has a test that fails on the
defect it names. The one gap is the PR title. Accuracy defects in the ADR and FR
text are in SR-954; code defects are in SR-952.
