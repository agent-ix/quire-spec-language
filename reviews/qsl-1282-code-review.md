---
id: SR-1918001
title: "code review (with rust-review) of PR #1918 (QSL-1282): replay by transition ordinal, no recorded key compare"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1918 git diff origin/main...36dd3b39d (qsl-eval/src/simulation/sample.rs, qsl-eval/src/simulation/trace.rs, qsl-eval/tests/it/finite_simulation.rs)"
review_set: subset
---
# SR-1918001: code review of PR #1918 (QSL-1282)

## Summary

Ticket: QSL-1282. Head 36dd3b39d. The rust-review lane is folded in here. No build was run: every check below was settled by reading the code.

The PR removes `ReplayError::KeyMismatch` and the recorded-digest comparison from `replay`. It adds `Step.ordinal`, which `sample` writes and `replay` reads through `filter(transition == step.transition).nth(ordinal)`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test checks the ordinal that `sample` writes. `sample_then_replay_round_trips_and_refuses_tampered_traces` samples seed 2 through two `recv` edges and asserts only `steps[0].key == key("2")` and `replay == Ok`. Replay now ignores `key`. Both successors are terminal and have no findings, and `max_steps` is 1, so the trace replays whether `sample` writes ordinal 0 or 1. The hand-built ordinal test covers only the replay side. So a wrong `sample` ordinal would pass every test: counted after `swap_remove`, counted as the raw `index`, or always 0. Fix: assert `sampled.steps[0].ordinal == 1`. Also update this test's doc comment and inline comment, which still say replay "matches by digest" and that a "tampered ... step digest refuses". | qsl-eval/tests/it/finite_simulation.rs:1510-1541; qsl-eval/src/simulation/sample.rs:167-171 |

## Verdict

One medium test gap. The production code is correct.

- Ordinal and canonical order agree. `sample` and `replay` both take their successors from the same `order::expand`. It sorts by transition JCS bytes, then by post-state key bytes. `sample` counts the ordinal over `successors[..index]` before `swap_remove`, which acts only on that step's own fresh vector, so the reordering never reaches the count. The count is the number of earlier same-transition successors. That is the zero-based position that `nth` reads. There is no off-by-one.
- `index < n` follows from the `next_index` contract, so the slice and the index cannot panic.
- `FindingsCheck` still means something. It compares depth and the recomputed findings of the state that replay reached by following transition and ordinal. It also refuses a recorded entry left over at `finish`. Only the recorded `StateFindings.state` digest is dropped from the comparison, as FR-101-AC-5 requires.
- `simulation::Step` is constructed only in `sample.rs` and in this test file, so no other producer is left without an ordinal.
- No leftover `KeyMismatch` or `current_digest` remains in replay.
