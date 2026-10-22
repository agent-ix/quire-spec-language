---
id: SR-14093
title: "integrity review of STD-248 canonical store-buffering counterexamples (PR #1196, QSpec PR #335)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@060cff6e3fa4f10da34432d2e2c49f9ad3c3bc03; spec/decisions/ADR-025-weak-memory-models-for-parallel.md, spec/functional/FR-221-explore-the-x86-tso-memory-model.md, spec/functional/FR-222-explore-the-release-acquire-memory-model.md; agent-ix/quire-specification@f605fded924fd505168e61ed772de5f609c2b11b; spec/functional/memory/FR-436-explore-total-store-order.md, spec/test-cases/TC-390-store-buffering-under-each-model.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-436
    type: reviews
---
# SR-14093: integrity review of STD-248 (PR #1196, QSpec PR #335)

## Summary

Ticket: STD-248. Each diff is against main; neither PR is stacked.

The canonical order was recomputed by hand from ADR-027 TS-2 (successors in
ascending JCS bytes of the identity) and from the identity encoding in
`qsl-eval/src/simulation/protocol/identity.rs`. The SB attempts in
`qsl-analyze/tests/it/litmus.rs` were read too: `Cell::set` takes no
parameter, and `Core::readN(c)` takes the cell reference. `arguments` is the
first key, so a store (`[]`) sorts before a load (`[{…}]`), and `Lx` (cell
`x`) sorts before `Ly` (cell `y`). With equal arguments, `memory_choice`
comes next (read depth ascending), then `node` (`left` before `right`), so
`Sx` sorts before `Sy`. A flush identity has no `arguments` key, so attempts
sort before flushes.

Breadth-first from the fork, the first depth-2 state is (`Sx`, `Sy`). Its
first successor is `Lx`: under `ra`, depth 0 gives `c2.r = 1` and depth 1
gives `x0`. Under `tso`, the first successor of (`Sx`, `Sy`, `Lx`) is `Ly`,
and it violates. Under `ra`, every successor of the depth-0 branch has
`c2.r = 1`. In the depth-1 branch, `Ly` at depth 0 reads `y1`, and `Ly` at
depth 1 (`y0`) violates. Both stores have one slot each (slot depth 0).

The new `tso` and `ra` tables are therefore the canonical counterexamples.
The messages, message views and `cur` columns check out row by row:
`y1{x0, y1}` because `right`'s `cur` is `(x0, y0)` at `Sy`, and
`x0`/`y0` stay live under both the old and the STD-240 pin rule. The old
orders reach the same violating state. "Under `sc` … `c2.r = 1` at position
3" is correct.

#1189 (STD-233) head 542a60c no longer touches ADR-025 section 10, FR-221
or TC-666 (item 6a dropped). #1196 is self-consistent: FR-221-AC-1,
FR-222-AC-1, section 10 and TC-666/TC-667 ("§10's counterexample") agree.

Gates: `make check-spec-validation` exit 0 with no diagnostic on a touched
file, and `tools/check-index-completeness.sh` passes. No `docs/` file and no
test-parsed vector table is touched. QSpec `quire validate` reports 134
failing documents, as on main, none of them touched. `cargo test --test
checked_package_v2 --test backend_provider_v1`: 14 and 117 passed. Both PRs
merge cleanly onto main and onto the other part-C branches.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The stated ordering rule does not fix position 1. ADR-025 section 10 and TC-390 SB-02 say nothing on how `Sx` and `Sy` (both with no argument) order: `left` before `right`, by node bytes. SB-02 also omits "attempts sort before flushes", which the `tso` positions 3 and 4 depend on. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:413, spec/test-cases/TC-390-store-buffering-under-each-model.md:48 |

## Verdict

Mergeable once FND-001 is fixed, or as is if the lead accepts it. The
counterexample content is correct, the two halves agree, and the PR bodies
contain no review-finding paraphrase.
