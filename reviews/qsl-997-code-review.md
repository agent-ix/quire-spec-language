---
id: SR-17170
title: "QSL-997 code and Rust review of the FR-220-AC-14 ordering-mismatch tests (quire-spec-language PR 1522)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@70cd9be9a070346cabaad698bf85d947527f7d46; git diff origin/main...HEAD (1 file, +130): qsl-analyze/tests/it/weak_memory.rs; context: qsl-eval/src/simulation/protocol/access.rs, qsl-eval/src/simulation/protocol/steps.rs, qsl-analyze/tests/it/litmus.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
---

## Summary

Ticket: QSL-997 (PR quire-spec-language#1522). This is `/code-review` with
the `/rust-review` lane, run over `git diff origin/main...HEAD` at
`70cd9be9a`. The PR changes tests only. It adds `weak_shapes`, `mismatch`,
`each_ordering_mismatch_settles_its_shape_under_tso_and_ra` and
`a_multi_location_attempt_is_not_tested_for_ordering_and_the_pre_check_continues`,
both tagged `#[trace("TC-665", "FR-220-AC-14")]`.

The production code these tests exercise is on main: `ordering_mismatch`,
`WeakAccessShape::ordering_mismatch` and `StateWeakAccesses::first_shape` in
`qsl-eval/src/simulation/protocol/access.rs`. The companion gap-analysis
(SR-17171) records that review clause by clause.

Reviewer probes ran in a private worktree at HEAD, never in the coder
worktree. They used a separate `CARGO_TARGET_DIR` under `~/.cache`, and every
mutated file was restored afterwards (`git status` clean). The mutants were
applied to `access.rs`:

| Mutant | Change | Result |
|---|---|---|
| M2 | the pre-check returns at the first `MultiLocation` entry (the "pre-check continues" clause, which the coder did not mutation-check) | killed at weak_memory.rs:981 |
| M4 | the RMW `null` ordering is written `"none"` | killed |
| M5 | the `Local` arm never mismatches | killed |
| M6 | the `Rmw` arm always mismatches | killed by the acq_rel RMW check at weak_memory.rs:954 |
| M7 | `release` is dropped from the load arm | killed |
| M8 | `acquire` is dropped from the store arm | killed |
| M9 | `local` is written `load` | killed |
| M1 | a multi-location attempt also gets a `local`/`acquire` ordering-mismatch entry | **survives**: the new tests, all 37 `weak_memory` tests and all 785 `it` tests pass |
| M3 | a multi-location attempt also gets an `rmw`/`acquire` ordering-mismatch entry | **survives**: the new tests and the 37 `weak_memory` tests pass |
| M10 | failures are sorted by descending transition bytes | **survives** all 785 `it` tests (see SR-17171) |

A probe replaced the two negative assertions with `ModelCheckOutcome::Holds`.
Every case passes under `sc`, and the acq_rel RMW passes under `tso` and
`ra`.

Gates on the private copy:

- `cargo fmt --all -- --check` is clean.
- `cargo clippy -p qsl-analyze --all-targets -- -D warnings` is clean.
- The new tests pass.

## Verdict

CHANGES REQUESTED. The new tests do check the exact wire shape of each
mismatch kind under `tso` and `ra`, and the tags are honest: every assertion
belongs to FR-220-AC-14. There are no tautologies. The "pre-check continues"
clause is truly guarded: M2 is killed.

One clause is guarded too weakly. The test for "the multi-location attempt is
not tested for ordering mismatch" rejects only `store` entries, so an
ordering entry of any other class for that attempt goes unnoticed (FND-001).
Two low findings cover assertions that accept any other outcome and a
misleading comment.

The test is not mergeable as a proof of "test every clause" until FND-001 is
fixed. The low findings should be fixed in the same round.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | The "not tested for ordering mismatch" clause rejects only `ordering-mismatch` entries with `access == "store"`. A wrong `local` or `rmw` ordering entry for the multi-location attempt `Bx` passes. Mutants M1 and M3 survive this test, and M1 survives all 785 `it` tests. Fix: assert the exact ordered failure list, with each entry's site and shape, or assert that no `ordering-mismatch` entry has site `Both/left/L/Bx` | qsl-analyze/tests/it/weak_memory.rs:982-987 |
| FND-002 | low | The negative assertions accept any other outcome. The `sc` check is `!matches!(…, Memory { .. })`, so a refusal or inconclusive result passes. The acq_rel RMW check is `weak_shapes(..) == None`, and `weak_shapes` returns `None` for every outcome that is not `WeakAccessShape`, not only for "explores" as its doc says. Every case actually returns `Holds`, so assert `ModelCheckOutcome::Holds { .. }`, as the sibling tests do (weak_memory.rs:690-696) | qsl-analyze/tests/it/weak_memory.rs:881,941-947,953-955 |
| FND-003 | low | The `one_branch` comment says the first attempt sits "beside a relaxed read of `x` … so that `x` is shared". The local case passes `other = ""` (no ordering), and nothing in that case shares `x`. The comment is right only for the RMW case | qsl-analyze/tests/it/weak_memory.rs:898-900,932 |

## Coverage

These tests were examined: `weak_shapes`, `mismatch`,
`each_ordering_mismatch_settles_its_shape_under_tso_and_ra` and
`a_multi_location_attempt_is_not_tested_for_ordering_and_the_pre_check_continues`.
The pre-existing `ordering_mismatches_are_accumulated_at_the_first_state` was
read for context.

The litmus helpers were read to confirm what each case builds: `sb_run`,
`sb_unit`, `attempt`, `protocol`, `contracts`, and the `Cell::add`,
`Cell::both` and `Core::copy` frames.

- The RMW case is `AddX` reading and writing `x.v`, beside a relaxed `Lx`
  reading it.
- The local case is `CopyR`, which reads `r1` and writes `r3`. That is
  branch-local.
- The multi-location case is `Bx` writing `x.v` and `x.w`, beside `Lx`
  (release) and `Lw` (relaxed) reading them.

Rust lane: the PR adds no `unwrap` in production code. The `expect`s in the
tests carry messages. There is no `#[allow]`, no `unsafe` and no integer
conversion. The tests use `trace` attributes, not `/// Trace:` lines.

## Dispositions

Disposition pass 1 covers head `220c3a0cd`. It was re-run on a private copy:
the 38 `weak_memory` tests pass, `fmt --check` is clean, and clippy
`-p qsl-analyze --all-targets -D warnings` is clean.

| FND | outcome | reason |
|-----|---------|--------|
| FND-001 | fixed | The multi-location test now asserts exactly two entries in order: `Both/left/L/Bx` multi-location, then `Both/right/R/Lx` load/release mismatch. A reviewer re-run kills M1 and M3 (weak_memory.rs:1013), and M2 is still killed |
| FND-002 | fixed | The sc admission check and the acq_rel RMW check under tso and ra now assert `ModelCheckOutcome::Holds { .. }`. M6 is still killed (weak_memory.rs:973) |
| FND-003 | fixed | The `one_branch` comment now says a relaxed read makes `x` shared when `first` writes it, and the unordered read keeps the local case's mismatch its own |
