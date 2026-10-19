---
id: SR-12810
title: "Gap analysis of quire-spec-language PR #967: FR-155 acceptance criteria against TC-574, TC-575 and TC-589 tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@52899349d5b0460cfed6acde3ad0a2b47a0ffb31; PR #967: FR-155-AC-1 to FR-155-AC-6 and QSpec FR-383 read-footprint items 1-6 against qsl-eval/tests/it/footprints.rs, qsl-semantics/src/model/footprint.rs and qsl-eval/src/simulation/model/footprints.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #967

## Summary

Ticket: QSL-542. This is a planless gap analysis of FR-155 against the PR's
tests. It starts from the computed matrix (`quire matrix --scope .`, quire
0.36.2, engine 0.50.2), adds a focused intent check of each tagged test, and
maps QSpec FR-383's read-footprint items 1-6 to evidence. Plan completion:
not assessed.

The matrix reports all six criteria as `tagged`, all in
`qsl-eval/tests/it/footprints.rs`: AC-1 at :606, AC-2 at :648, AC-3 at :723,
AC-4 at :779, AC-5 at :973 and :1108, and AC-6 at :1015.

AC-1 and AC-5 are fully exercised with strong oracles. The full-set
equalities on `bump`, `boom` and `pick` would catch an extra or a missing
location.

The tags overclaim AC-2, AC-3, AC-4 and AC-6. A mutation on a scratch copy
of the head commit replaced the post-clause `check.verify()?`
(`frames.rs:932`) with a no-op. All 608 `qsl-eval` tests still passed (82
unit and 526 integration).

FR-383 read-footprint items, mapped to evidence:

1. Precondition reads: covered (`bump`, `give`, `boom`).
2. Postcondition `pre(...)` reads: covered (`boom`'s `pre(self.cell.v)`).
3. Post-state reads outside `W(t)`: covered. AC-1's reads equality pins the
   exclusion of `self.v`, and `copy`'s keeps `AnyField{v}`.
4. Receiver and argument membership: covered (`give`).
5. Delete-blocking referrer reads: covered for a single type (`Job.peer`,
   `Job.target`). Not covered for subtypes, where the code is wrong (SR-12809
   FND-001).
6. Creation and reference-write target-membership reads: no assertion
   (FND-005).

## Verdict

Not mergeable. FND-001 is high: AC-3's subject, post-clause enforcement, is
not tested, and the mutation proves it. FND-002 to FND-005 are coverage
overclaims. Each one needs a test, or an explicit owner-approved deferral
with the tag narrowed to match.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-155-AC-3 is about `bump(a)`'s **post** clause. The test removes `Field{a, v}` from `reads` only, so the violation fires in the precondition `self.v < 2`, and the test asserts `Observation::Pre`. The post-clause `ReadCheck` is never made to fail. Mutation evidence: with the post-clause `check.verify()?` turned into a no-op, every `qsl-eval` test passes. So a regression that drops post-clause enforcement, or that admits post reads wrongly, ships green. The AC's "the item settles `failed`, category internal failure" is also untested; only `FootprintViolation::cause()` is checked. Fix: add a post-only violation, for example narrow `copy(a)` by removing `AnyField{Counter/v}`, so the post read of `b.v` stops with `Observation::Post`. Keep a case where a post read inside `W(t)` is admitted. | qsl-eval/tests/it/footprints.rs:721-760; qsl-eval/src/simulation/model/frames.rs:931-933 |
| FND-002 | medium | FR-155-AC-6's second half, "the reduced run returns `Violated` by `set, pick` over 6 states, as the unreduced run does", is not exercised. There is no reduced run, as the PR body discloses, yet the test is tagged AC-6 without qualification. Either test it when FR-156/FR-162 land, with the deferral recorded on the ticket, or split the AC so that the tag covers only the footprint half. | qsl-eval/tests/it/footprints.rs:1011-1040 |
| FND-003 | medium | FR-155-AC-4 says "every step of AC-1's reduced run passes the frame decision with its receiver". The test expands only the initial state, and its oracle is `!enforced.successors.is_empty()`; no step's frame decision is checked. The determinism half is tautological for instantiation: `footprint_of` twice reads the same `Application.footprint` cached at `ModelSystem::new`. The symbolic half re-derives from the same in-memory graph, not from a second check of the package. The hand-built `b.v` frame violation is properly tested. | qsl-eval/tests/it/footprints.rs:777-865 |
| FND-004 | medium | FR-155-AC-2's two quantification clauses, "pre is `forall o in counters: o.v < 2` reads `AnyField{v}` and `AnyMembership{counters}`" and "pre quantifies over `counters` reads `AnyMembership{counters}`", have no test. The `AllInstances`/`Lookup` arm of `node_reads` and the binder-field arm, where `Holder::Other` gives `AnyField`, never run. The PR says no clause can name a population today. If so, the AC is not satisfiable as written and needs an owner ruling; it should not be tagged as covered. | qsl-eval/tests/it/footprints.rs:642-712; qsl-semantics/src/model/footprint.rs:523-536 |
| FND-005 | medium | FR-383 read-footprint item 6 has no assertion. `spawn()` (`creates Counter`) should read `AnyMembership{counters}`, plus `peer`'s target population, but only `spawn.writes` is checked. `unlink()` writes the reference field `Job/peer` and should read `AnyMembership{jobs}`, but only its writes are checked. Mutation evidence: with both `read_target_memberships` calls replaced by no-ops on the scratch copy, every `footprints` test still passes. | qsl-eval/tests/it/footprints.rs:700-712; qsl-eval/tests/it/footprints.rs:1155-1160; qsl-semantics/src/model/footprint.rs:361-383 |
