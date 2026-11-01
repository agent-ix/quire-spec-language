---
id: SR-1957025
title: PR 1946 residual LT-16 helper gap-analysis
type: SpecReview
analysis: gap-analysis
date: '2026-11-01'
scope: 'agent-ix/quire-spec-language; QSL-1338 PR #1946; qsl-analyze/tests/it/fill.rs
  reachable helper at 6320fe8381e6aa1db40a58d0651c20532a845888'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-223
  type: references
- target: ix://agent-ix/quire-spec-language/FR-101
  type: references
---

## Summary

Ticket: QSL-1338. Static review of the residual helper optimization.

Frozen head: 6320fe8381e6aa1db40a58d0651c20532a845888. Actual diff against origin/main is only qsl-analyze/tests/it/fill.rs (+2/-1). No source/test/spec/branch edits were made by the reviewer. No applicable AssuranceProfile was found (the sole textual search match is a prior review saying none applies).

The two key APIs call the same encoder on self.key(state), with the same identity limits and error mapping. Previously state_key cloned the encoded bytes before the known-state lookup; now the same complete bytes are copied only after lookup misses. StateKey derives Eq and Hash over Vec<u8>; HashMap still compares full bytes, so hash collisions cannot merge unequal states. Encoding remains once per admission attempt. New states retain identical Reached.key bytes. The FIFO queue, successor sorting, edge appending, initial states and complete traversal remain unchanged. No limits, caps, skips, ignores, candidate reductions, assertions, fixtures, CI or Cargo files change.

The LT-16 test still asserts 7,607 states and 19,574 edges and rejects registers [2,3,3,0] with final X=1. The fixture explicitly remaps the distinct last message value 4 to 0 in its bounded Int[0,3] domain. This residual change neither changes that mapping nor the expected outcome. LT-16 is a direct reachable-graph regression, not a newly independent RC11 comparison. The existing rc11_support checker separately enumerates read-from and modification-order candidates without importing qsl-eval; it and compare() are unchanged. Reusing the authoritative canonical encoder for enumeration identity does not replace the memory-order oracle or its assertions.

Focused clippy evidence inspected: /home/peter/dev/.agents/tmp/pr1946-checks/clippy-resume.log records cargo clippy --locked -j1 -p qsl-analyze --test it --all-features -- -D warnings, Finished and Exit status: 0 / exit=0. The dispatch reports fmt passed and its fmt.log is empty; this reviewer did not independently rerun it. Debug no-run build remains pending in the coder's existing lane; no duplicate Cargo work was started. Current-head LT-16 correctness/timing, focused helper callers, final dependency-set release timing after PR #1930, and the lead-owned final aggregate gate remain pending. No residual speedup is measured. Historical combined-branch debug/release timings cannot establish benefit from these three lines. PR body now accurately distinguishes residual helper scope and pending evidence.

## Scope

```yaml
scope:
- id: reachable::admit
  path: qsl-analyze/tests/it/fill.rs
  role: examined
  excerpt: 'let key = SubjectSystem::canonical_state_key(system, &state).expect("a
    key");

    if let Some(index) = known.get(&key) { return *index; }

    let index = reached.len();

    let bytes = key.as_bytes().to_vec();'
- id: lt16_collected_sc_predecessors_still_constrain_the_sc_load
  path: qsl-analyze/tests/it/rc11_oracle.rs
  role: examined
  excerpt: '#[trace("TC-668", "FR-223-AC-8")]

    assert_eq!(reached.len(), 7_607, "LT-16 reachable state count");

    assert_eq!(transitions, 19_574, "LT-16 successor edge count");'
- id: FR-223-AC-8
  path: spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md
  role: examined
  excerpt: "ADR-025 \xA715's LT-16 does not reach a = 2, b = 3, c = 3, d = 4 and final\
    \ `X = 1` under `ra`: `w`'s view covers `L := 2` through the view of `L := 3`\
    \ and the acquire fence, though the message `d` reads (`L := 4`) does not; when\
    \ `L := 2` has been collected, `w`'s view holds its predecessors, `w`'s `seq_cst`\
    \ load takes them, and the slot for `w`'s `X := 2` before `X := 1` in modification\
    \ order is not offered."
- id: FR-101-AC-2
  path: spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md
  role: context_only
  excerpt: The state key is the JCS encoding of FR-181's typed canonical form, and
    exploration coalesces two states exactly when their full key bytes are equal.
    The state whose `semantic` member is `float64` bits `0000000000000000` and every
    map empty has its JCS key bytes as its key; with bits `8000000000000000` (negative
    zero) it has its own, different, key bytes, and the two remain two states. Two
    NaNs with different payloads are two states. Each key is a `StateKey` under `quire.simulation.state-key/v1`.
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

CONDITIONAL: static review clean, no introduced defects. Merge readiness remains conditional on the pending correctness/timing evidence and lead-owned final gate. This is not full QSL-1338 completion or a measured residual performance claim.

## Coverage

Scoped manual criterion/test mapping per dispatch. Whole-repository computed matrix not rerun: the lead reports two earlier identical broad attempts produced no output after 3–4 minutes and were stopped. Accordingly this is a scoped fallback, not a successful computed matrix or a repository-wide assurance claim. Existing #[trace("TC-668", "FR-223-AC-8")] binds the LT-16 forbidden-outcome test to FR-223-AC-8. FR-101-AC-2 supplies identity context only; this helper is not claimed to test every signed-zero/NaN subcase. No new externally specified behavior or requirement is needed for an equivalent private helper optimization. No introduced orphan behavior, stubs or inflated coverage were found. Optional whole-repository semantic review was not run. No plan was supplied.

Plan completion: not assessed

