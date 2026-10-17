---
id: SR-12058
title: "Gap analysis of quire-spec-language PR #927: FR-340, FR-343, FR-344 against TC-860, TC-863 to TC-866"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2eaaa4cd5c5a462053415cc7600f1ec8926d6ada; PR #927: FR-340-AC-1..6, FR-343-AC-1..5, FR-344-AC-1..5 and AC-7 (TC-860, TC-863, TC-864, TC-865, TC-866) against xtask/src/refinement/** , xtask/tests/refinement_versioning.rs, Makefile and tests/fixtures/refinement/versioning/"
review_set: subset
---
# Gap analysis of quire-spec-language PR #927

## Summary

Ticket: QSL-517. Planless. Each acceptance criterion in scope was traced to a
tagged test by reading the test and the code it drives. No build was run.

Every in-scope AC has a tagged test that asserts it. FR-340-AC-1, AC-2, AC-4
and AC-5 map to TC-860. FR-343-AC-1 maps to TC-863, the hand-written
2352-combination literal table. FR-343-AC-2 to AC-4, FR-344-AC-5 and
FR-340-AC-6 map to TC-864. FR-343-AC-5 and FR-344-AC-4 map to TC-865.
FR-344-AC-1 to AC-3 and AC-7 map to TC-866. FR-344-AC-6 (layering) is out
of scope (QSL-39).

The author recorded these as planner questions, and they are not findings
here:

- FR-340-AC-3 and TC-860 step 3 talk about revision labels, but `pair.json`
  carries none, so the test swaps units instead.
- How a pair handles an unbuildable case.
- A stdout write failure exits 2.

## Verdict

Changes requested: one medium finding on the real corpus that `make ci`
runs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The real corpus that `make refinement-versioning` (now in `CI_CHECKS`) runs holds one pair whose `prior.path` and `superseding.path` are the same file, `unit.native`. The gate compares a unit with itself, so the CI lane can never report a regression. The only failures it can catch are a tool failure or nondeterminism, and the integration test `the_real_corpus_passes_and_the_seeded_copy_fails` already covers both. FR-340-AC-6 passes, but it passes vacuously. Fix: make the real corpus a genuine revision pair, for example a meaning-preserving superseding revision such as the test's `heavier()` unit, so that a wrong classification or comparison turns `make ci` red. The other option is a planner ruling that drops the CI lane until real revisions exist. | tests/fixtures/refinement/versioning/config-version-identity/pair.json:2-11; Makefile:64-68 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The new `repo:` arm of `read_in` refuses an absolute path, a path that climbs above the repository root, and a link that resolves outside it. No test exercises any of these on a `repo:` path. FR-340 now confines `repo:` reads to the repository, but FR-340-AC-2 and its test cover escapes from the pair directory only. A regression in the `repo:` branch, such as a root swap or a dropped `starts_with` check, would let a corpus read any file on the machine and still pass. Fix: add `repo:../outside` and `repo:/etc/hostname` malformed pairs to `each_malformed_pair_is_one_tool_failure_and_the_rest_still_run`, and widen FR-340-AC-2 to name them. | xtask/src/refinement/versioning.rs:424-458; xtask/tests/refinement_versioning.rs:370-445 |

## Dispositions

Round 1, reviewed at `fd2fa6bd42ce32f3c7739a8510122d9fd8b989fd` (local
branch, not yet the PR head).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fd2fa6bd4 |

FND-001: the real corpus is now `config-version-revision`. Its `prior` is
`unit.native` and its `superseding` is `superseding.native`, the same unit
with `ParentOrder`'s antecedent written `not not present(self.parent)`.

- The change keeps the meaning. `present()` is Boolean, and `not not b` is
  `b`. That holds whichever way `not` and `implies` bind, because
  `not not (A implies B)` is also `A implies B`.
- The CI lane can now fail on real drift, not only because the seeded test
  exists:
  - If negation or evaluation regresses so that the two forms disagree on
    the healthy-parent snapshot, the result is `regression` and the lane
    exits 10.
  - If the example package changes against the digest literal in the units,
    the result is `tool failure` and the lane exits 30.
- What it cannot catch is a change that breaks both revisions the same way.
  That is inherent to a refinement gate and is other tests' job.
