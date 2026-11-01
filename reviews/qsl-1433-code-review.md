---
id: SR-090
title: "Code review of QSL-1433 refinement budget test optimization (PR #2026)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it/refinement_budget.rs; FR-149-AC-5, FR-141-AC-9, FR-145-AC-6, TC-556, TC-555, TC-550; QSL-1433 / PR #2026"
review_set: subset
---

## Summary

Ticket QSL-1433 / PR #2026 changes only the shared budget helper in `qsl-analyze/tests/it/refinement_budget.rs`. The exact `Holds` threshold search is exponentially bracketed then binary-refined, and cached once per integration-test process; the related acceptance assertions remain intact. No defects found.

## Verdict

**PASS** — the cached threshold preserves the existing oracle and exact under-budget certificate stop assertion; the focused test and module filter passed, and workspace Clippy, formatting and the configured Cargo deny check passed.

## Coverage

- No applicable `AssuranceProfile` is present in `spec/assurance/`; its active entries are architecture and measurement artifacts, outside this test-only change.
- The installed Quoin 0.29.1 has no `gap-analysis` command. I performed a manual acceptance-criteria-to-tests check against FR-149-AC-5 / TC-556 step 6, FR-141-AC-9 / TC-555 step 5, and FR-145-AC-6 / TC-550 step 5. The modified helper is used by the four existing tests in the file; all assertions remain unchanged.
- Search correctness: for the profiled threshold B=124, the exponential phase probes 1 through 128 and the binary phase takes six probes to return 124 (14 calls total rather than 20). The search still uses the same `SafetyOutcome::Holds` predicate and the previous maximum. `OnceLock` shares only the immutable computed budget among tests in this process.
- The required `EvaluationMeter(WorkUnits), value: b` `Stopped` assertion and the large-meter `Accepted` assertion in the HeavyEnq certificate case are unchanged. The tests do not weaken or conditionally skip the “not Rejected” behavior.

## Reviewed Scope

| ID | Role | Path | Excerpt |
| --- | --- | --- | --- |
| FR-149-AC-5 | examined | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md | A valid certificate is never rejected for a meter it ran out of, in each of four places, each returning `Stopped(LimitReached{EvaluationMeter, B})`, not `Rejected`; (d) a `HeavyEnq` certificate (FR-141-AC-9), built under a larger meter, checked under B, where the abstract side runs out. |
| TC-556 | examined | spec/test-cases/TC-556-the-simulation-certificate-checker-accepts-and-rejects.md | Build a `HeavyEnq` certificate under a larger meter, then check it under B (FR-149-AC-5). |
| FR-141-AC-9 | context_only | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | Fixture `HeavyEnq`: `RingIsQueue` whose abstract `enq` postcondition is a conjunction over every ring slot costing more than the per-evaluation meter budget B. |
| TC-555 | examined | spec/test-cases/TC-555-a-mapping-row-or-argument-leaves-the-refinement-undetermined-or-refutes-it-an-abstract-budget-stop-is-incomplete-and-reads-are-ordered.md | Over `HeavyEnq` (FR-141-AC-9) with the per-evaluation meter at B, run `check_step` on `put(1)`, run FR-142's check, and settle the item. |
| FR-145-AC-6 | examined | spec/functional/FR-145-replay-a-refinement-counterexample.md | Replaying a hand-built payload (a recorded `AbstractStepRejected` over `HeavyEnq`'s `put(1)` step) under the meter B settles V-7: execution `resource-incomplete`, `Incomplete(LimitReached{EvaluationMeter, B, setting})`, never `inconclusive`/`ReplayParity`. |
| TC-550 | examined | spec/test-cases/TC-550-the-replay-facade-replays-a-refinement-counterexample-and-recomputes-what-the-abstract-model-saw.md | Replay a payload whose prefix reaches `HeavyEnq`'s `put(1)` step under the meter B (FR-145-AC-6). |

## Check Evidence

- `cargo fmt --all -- --check` — passed.
- `make cargo-deny-bans` — passed (`bans ok`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- Coder's focused test — 1 passed; serial module filter — 5 passed in 65.27s. Both timings were nonisolated (`other_busy=1`); the issue and PR describe that limitation. Focused baseline and candidate timings also overlapped other builders and are directional only.
- `cargo test --locked -p qsl-analyze --all-targets --all-features` compiled and ran 123 unit tests successfully, then I interrupted the command with Ctrl-C while integration tests were still running, after the lead instructed me not to run further builds. I—not the lead—stopped it. Before interruption, one environment-dependent manifest test had failed with `QSPEC_DIR` unset; rerunning that specific test directly with `QSPEC_DIR=/home/peter/dev/quire-specification` passed. The full integration suite did not complete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
