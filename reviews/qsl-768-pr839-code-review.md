---
id: SR-9106
title: "Code review of quire-spec-language PR #839: Population<T>[N] emitted-package fixtures"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@facab37ff15fc5b874e806b8ce529cd91483b715; qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
---
# Code review of quire-spec-language PR #839

## Summary

Ticket: QSL-768. Code review with the rust-review lane folded in, scoped to
the diff `origin/main...HEAD` (one commit, test-only: `population_package`,
`population_nodes`, `max_of` and the test
`population_bounds_are_emitted_admitted_and_typed`).

Examined:
- `population_package(bounds)` builds `pop` over the TC-442 spine model from
  forms (`TypeForm` with head `Population`, argument `M::Gadget`, bound
  `N.to_string()`), checks and links through the same `modelled` helper and
  `Emitted::whole` emit path as the existing `ModelPopulation` fixture. No
  package JSON is hand-written or copied (examined)
- `read` is the existing QSL I2 read: `read_checked_package_v2` (IR's
  checked-package/v2 reader) pinned at the emitted `package_id`, with the spine
  model document as evidence. The test also compares the read's `package_id`
  with the emitted one (examined)
- Mutation analysis by reading (no builds run):
  - Role Expression instead of Type: the role string would be `expression`,
    and `assert_eq!(roles, &["type"])` fails. An empty or missing role gives
    `""` and also fails. An empty `occurrences` gives `[]` and fails (caught)
  - Wrong bound (bound off by one, or every node getting the same bound): the
    sorted `max` values differ from `bounds` (caught)
  - N = 0 dropped or mis-encoded: `omitted == []`, the admitted count of 1,
    `populations.len() == 1`, and `max_of` returning `Some(0)` (else it
    panics) each catch it (caught)
  - Two parameters sharing one node: the [2, 5] case then has one population
    node, so `populations.len()` is 1, not 2, and the admitted count is 1, not 2 (caught)
  - Missing node: both counts catch it (caught)
  - Filter field names wrong (`node_tag`/`semantic_form`): `populations` would
    be empty, and every case has at least one bound, so the count assertion
    fails. The filter cannot match nothing silently (caught)
  - `node_kinds()` is a per-node slice (IR `quire-contract-model`
    checked_package/v2), so the admitted count is per node, not per kind.
    The [2, 5] case passing with 2 confirms this (examined)
- Gate: `/home/peter/dev/worktrees/logs/qsl768-ci.log` ends `EXIT 0`. Its
  `test result` lines sum to 9035 passed, with
  `population_bounds_are_emitted_admitted_and_typed ... ok` at lines 1543 and
  7600. The log finished at 09:14:31 and the only branch commit is
  facab37ff, committed at 09:14:52. The worktree is clean at facab37ff (examined)
- rust-review lane: no `unwrap` on production paths (test-only module). The
  helpers use `expect`/`panic!` with the case in the message. No integer
  conversion or bounds risk (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS.** The test is characterization only (it passed on its first run).
Reading each requested mutation shows an assertion that fails for it, so the
test is not vacuous.

Recorded, not reported: low, `population_package` duplicates the existing
`Fixture::ModelPopulation` builder (bound 3). The existing builder could call
`population_package(&[3])`.
