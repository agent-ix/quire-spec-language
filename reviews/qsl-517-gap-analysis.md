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
