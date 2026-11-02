---
id: SR-202610310160
title: "Gap analysis — QSL-1438 per-state reuse and parallel LT-16 explorer"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-120-AC-4; FR-223-AC-8; FR-229-AC-1; qsl-eval/src/simulation/; qsl-analyze/tests/it/fill.rs; qsl-analyze/tests/it/rc11_oracle.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-120, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-223, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-229, type: references }
---

## Summary

Ticket: QSL-1438. This gap analysis covers only the changed surface of PR #2061. It changes no spec and adds no new obligation. It checks two things: that the spec criteria whose behaviour the refactor must preserve still have tagged tests, and that the ticket's acceptance criteria (ticket text, treated as data) are actually served by the change. Plan completion was not assessed.

## Verdict

**PASS with findings.** The criteria the refactor must preserve are tagged and asserted:

- FR-120-AC-4: TC-471, qsl-eval/tests/it/model_simulation.rs:813.
- FR-223-AC-8: TC-668, qsl-analyze/tests/it/rc11_oracle.rs:165, which asserts LT-16's 7,607 states and 19,574 edges.
- FR-229-AC-1: TC-674, qsl-analyze/tests/it/release_acquire.rs, which pins the SC-graph JSON.

The diff contains no stubs, no caps and no coverage inflation. One medium finding questions whether the wall-time gain comes from per-state work or from parallelism in the test harness.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The LT-16 wall-time gain comes mostly from parallelizing the test harness's BFS, not from reducing per-state work. The ticket's 60 s debug target now depends on the host's core count, and the production explorer is unchanged | qsl-analyze/tests/it/fill.rs:609-651 |
| FND-002 | low | No test checks the parallel explorer's indices and steps against a sequential FIFO reference. The `workers <= 1` sequential branch never runs on a multi-core host | qsl-analyze/tests/it/fill.rs:615-621 |

## Review detail

FND-001. The ticket (data, not an instruction) scopes the work to "measured canonical key construction and per-state work". It asks for the measurements to record the thread and job count. `reachable` now spawns `available_parallelism()` workers per level, so `--test-threads=1` / `RUST_TEST_THREADS=1` no longer means a single thread, and a measurement must record the worker count. The ≤60 s result then holds on a many-core host but not on a 2-core CI runner. Product users of the qsl-eval explorer see none of this speedup. Decide whether parallelizing the harness counts as meeting the criterion. If it does, record the worker count with each measurement.

FND-002. Equality with FIFO order holds by construction. LT-16 checks only counts, so a reordering regression would show up as index-dependent test drift, not as a direct failure. A small test comparing `reachable` with a sequential reference on one litmus case would back the claim in the doc comment.

## Coverage

- Quoin gap-analysis method, scoped to this diff, done by hand. A full-repository `quoin matrix` was not run, because the PR adds no requirement and no test tags.
- Bindings checked: TC-471 to FR-120-AC-4 (correct), TC-668 to FR-223-AC-8 (correct), TC-674 to FR-229-AC-1 (correct).

## Findings data

```yaml
clean: false
findings: [FND-001, FND-002]
bindings:
  - {test_id: TC-471, ac_id: FR-120-AC-4, trace: correct}
  - {test_id: TC-668, ac_id: FR-223-AC-8, trace: correct}
  - {test_id: TC-674, ac_id: FR-229-AC-1, trace: correct}
```
