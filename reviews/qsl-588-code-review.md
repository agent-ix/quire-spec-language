---
id: SR-1988005
title: "Code review — QSL-588 timed statistical sampling PR 1998"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/, qsl-analyze/tests/it/statistical_model.rs, qsl-eval/src/, qsl-eval/tests/it/timed_subject.rs, qsl-replay/src/, qsl-semantics/src/, qsl-semantics/tests/it/probabilistic_declarations.rs; FR-253-AC-1..4, FR-254-AC-1..4"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-253", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-254", type: references }
---

## Summary

Reviewed the Rust changes in PR #1998, including requirement alignment, timed sampling and witness replay, test oracles, and Rust gates. The new stochastic race rejects free-delay transitions where FR-253 requires the race to preserve them as scheduler choices; the workspace all-target Clippy gate also fails on a new test key type.

## Verdict

**FAIL** — the FR-253 free-delay race path is not implemented, and a high-severity finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The timed race refuses every free-delay transition as `NotStochastic`, so it cannot expose the Markov decision process and scheduler-selected delay required by FR-253-AC-4. | qsl-eval/src/simulation/model/timed/race.rs:342-346 |
| FND-002 | medium | The new race oracle checks FR-253-AC-2 behavior but its test carries only the FR-253-AC-1 trace tag, leaving AC-2 unbound in the computed matrix. | qsl-eval/tests/it/timed_subject.rs:460-462 |
| FND-003 | medium | Workspace all-target Clippy fails on the new `BTreeMap<Vec<(&str, Rational)>, _>` key because `Rational` has interior mutability, blocking the strict lint gate. | qsl-eval/tests/it/timed_subject.rs:513 |

## Coverage

- FR-253-AC-1 examined: exact reply and timeout windows are asserted in `stochastic_rpc_exposes_checked_delay_windows`.
- FR-253-AC-2 examined: its three path probabilities are asserted, but the test is tagged only with FR-253-AC-1.
- FR-253-AC-3 examined: delay declaration refusals and `NotStochastic` cases are checked in timed declaration tests.
- FR-253-AC-4 examined: the only free-delay race branch returns `NotStochastic`; no scheduler-choice/MDP result is exposed.
- FR-254-AC-1..4 examined: timed event decisions, exact grid delays, deterministic replay and altered-delay rejection, elapsed measure, free-delay refusal, step budget and local time-lock are asserted by traced statistical tests.
- EN-5 protected-path check: no changed path under `qsl-analyze/src/exact_probabilistic/` or the exact certificate/witness implementation; `git diff --name-only` contains no such path.
- Gates: `cargo fmt --check` passed; `cargo deny check bans` passed; scoped no-default-features Clippy passed; workspace `cargo clippy --workspace --all-targets --all-features -- -D warnings` failed at the new mutable-key test. Focused PR test commands passed: `timed_subject` (73), `probabilistic_declarations` (52), `statistical_model` (27), and `statistical_witness_replay` (12). The full touched-crate command stopped at qsl-analyze `it`: 1076 passed, 60 failed, with OperationMemberMismatch and unavailable QSPEC_DIR failures; no changed timed-sampling test was in the failure list.

## Bindings

- `TC-708` → `FR-253-AC-1`: correct; the checked delay windows are directly asserted.
- `TC-709` → `FR-254-AC-1`, `FR-254-AC-2`, `FR-254-AC-3`, `FR-254-AC-4`: correct; the tagged tests assert the corresponding statistical outcomes and replay behavior.
