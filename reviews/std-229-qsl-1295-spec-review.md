---
id: SR-16204
title: "spec-review of FR-350 determinism across runs and test thread counts (quire-spec-language PR 1295)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@55519b5c; spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md; spec/test-cases/TC-1192-the-runner-gives-equal-bytes-across-runs-and-test-thread-counts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1192
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-352
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: references
---

# Spec review of FR-350 determinism across runs and test thread counts (quire-spec-language PR 1295)

## Summary

Ticket: STD-229. This is one spec-review pass over
`git diff origin/main...55519b5c`. The diff adds the sequential-execution
paragraph, the compared-bytes table and the determinism-sample rule to
FR-350 "Determinism", adds FR-350-AC-6 (Test) and FR-350-AC-7 (Inspection),
and adds TC-1192. The oracles were FR-350 on main (Outputs, Subjects and
applicability, Settlement, Isolation), FR-351, FR-352 (report contents and
ordering, FR-352-AC-3 byte equality), FR-353 (exit contract), FR-295
(determinism of JIT/AOT runs) and NFR-006 (no thread pool).

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation`
exits 0, with no finding on FR-350 or TC-1192. origin/main (5ba013c7) is an
ancestor of the head. FR-350-AC-6, FR-350-AC-7 and TC-1192 appear in no
other open PR, and no other open PR edits FR-350.

Examined, clean:

- **Sequential execution is consistent with the rest of FR-350.** The
  panic-to-`tool-failure` rule, per-vector isolation and fresh accounting
  all work on the calling thread. NFR-006 adds no thread pool, and FR-295
  already forbids generated code from reading thread state. AC-7 is a
  property of the runner's source, so Inspection is the right method.
- **The compared-bytes table matches the subjects table.** It has
  `compile`, `run`/`replay` and `format`, the same three entry points, and
  FR-286 is the `run`/`replay` outcome document. FR-295 says an outcome
  names no backend, so the `run` bytes do not depend on which execution
  backend ran.
- **The ACs fit FR-352 and FR-353.** AC-6 adds the thread-count axis to
  FR-350-AC-5 and FR-352-AC-3 and contradicts neither. No exit code changes.
- **Wording describes behaviour only.** The lead defaults are flagged in the
  PR body as "lead default, not an owner answer".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-6 compares the output bytes of each passed vector, but no runner output carries those bytes. Under FR-350 Outputs, a vector outcome holds the vector id, the capability id and, only when the vector is not `passed`, the expected and actual result. FR-352 lists no detail for a passed vector. So the new SHALL on "the bytes its subject's entry point returned" constrains a value the runner never exposes, and TC-1192 step 2 cannot "keep" it through the runner. Fix: say where the bytes come from. Either the outcome of a passed vector carries its entry point's output bytes (or their digest) as a runner output, or the test reaches them through a named seam of the runner. Add that to FR-350 Outputs. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:41-47, :124-137, :152 | missing-requirement |
| FND-002 | medium | The sample rule can pick a vector that never runs. The rule takes "within each capability of the default scope, for each vector class that capability holds, the vector with the smallest id". But FR-350 runs a vector only when its own applicability also names a QSL consumer type, and a capability in the default scope can hold vectors for other consumers. The smallest-id vector of a class can therefore be one that FR-350 does not execute. It then has no outcome and no output bytes. TC-1192 step 1 still expects "exactly one vector per (capability, class) pair", so the check covers fewer vectors than it claims. Fix: take the smallest id among the vectors that FR-350 runs. A class with no such vector contributes none. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:139-141; spec/test-cases/TC-1192-the-runner-gives-equal-bytes-across-runs-and-test-thread-counts.md:22-24, :44-45 | correct-requirement-no-evidence |
| FND-003 | medium | AC-6 can pass without any thread contrast, and it does not say how runs in two processes are compared. The harness default is the machine's available parallelism, which is 1 on a single-CPU runner. A test filtered to TC-1192 alone runs no other test beside it at any thread count. AC-6 asks only for the default count. "Other tests of the suite running alongside" appears in TC-1192 step 3 but not in the AC. Also, `RUST_TEST_THREADS` is fixed per process, so the four runs need at least two processes, and neither the AC nor the TC says how their reports and bytes are brought together for comparison. Fix: in AC-6, require the multi-thread runs to use at least two harness threads, with at least one other test running at the same time. Name the mechanism, for example one test that starts two child `cargo test` processes and compares the files they write. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:152; spec/test-cases/TC-1192-the-runner-gives-equal-bytes-across-runs-and-test-thread-counts.md:26-36 | wrong-requirement |
| FND-004 | low | "The vector with the smallest id" does not say how ids are ordered (bytewise or by numeric suffix). Two implementers could pick different samples. FR-352's "ascending vector-id order" has the same gap, so a fix here can state the order once for both: bytewise on the id string, as FR-075 orders `BackendId`s. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:139-141 | wrong-requirement |

## Verdict

Not mergeable yet. Fix FND-001 to FND-003 in this PR; FND-004 may be fixed
in the same edit. Sequential execution, AC-7 as an Inspection criterion,
the compared-bytes table and the lead defaults are sound. Every finding is
about making AC-6 observable and non-vacuous: the bytes need a source
(FND-001), the sample must hold only vectors that run (FND-002), and the
thread contrast must be real and comparable across processes (FND-003).
