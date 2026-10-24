---
id: SR-12131
title: "QSL-584 PR #958 code review: EN-7 closed-form schedulability and its checker (FR-246, FR-247, FR-248)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@402ae5b3f63a936eb6e5fc40244340129ad36989; qsl-analyze/Cargo.toml, qsl-analyze/src/lib.rs, qsl-analyze/src/schedulability.rs, qsl-analyze/src/schedulability/amc.rs, qsl-analyze/src/schedulability/arith.rs, qsl-analyze/src/schedulability/busy_period.rs, qsl-analyze/src/schedulability/edf.rs, qsl-analyze/src/schedulability/fixed_priority.rs, qsl-analyze/tests/it/main.rs, qsl-analyze/tests/it/schedulability.rs, qsl-replay/Cargo.toml, qsl-replay/src/certificate.rs, qsl-replay/src/certificate/closed_form.rs, qsl-replay/src/certificate/closed_form/outcome.rs, qsl-replay/src/certificate/closed_form/recompute.rs, qsl-replay/src/certificate/closed_form/settle.rs, qsl-replay/src/certificate/closed_form/task_set.rs, qsl-replay/src/lib.rs, qsl-replay/src/outcome.rs, qsl-replay/src/proof_result.rs"
review_set: subset
---

## Summary

Ticket: QSL-584. PR quire-spec-language#958, branch `closed-form-schedulability`. This is a code review with the rust-review lane (agent-skills `skills/rust-review/SKILL.md`), scoped to `git diff origin/main...HEAD` (21 files, +2489/-3). It was a static review only. No cargo or make command ran, because disk space is tight. The gates in the PR body are the lead's receipt and were not re-run.

Soundness was checked first. `recompute` in `qsl-replay` calls nothing of `qsl-analyze`, and the dependency runs only from analyzer to checker. It has its own copies of the recurrence, `ceil`/`floor`, `dbf`, the busy period and the QPA stepping. Through the intended path, `recompute` on the same subject and then `settle`, a tampered outcome does not settle `proved`. A foreign-subject outcome does not either: evidence is checked against the subject it is given, and a shape mismatch gives `Shape`. Every numeric example in the acceptance criteria was checked by hand against both the engine and the checker: Ctl FP `1, 3, 12` with `5, 9, 12, 12`; EDF L = 12 and QPA `[8]`; WCET 6 gives `6, 10, 13` and `13/12`; the jitter-and-blocking set gives `5, 7`; AMC Mc gives low 2 and high 4, `C(HI) = 6` fails high mode, and `C(LO) = C(HI) = 5` misses in low mode at `5, 7`; the FR-248-AC-2 job-2 evidence disagrees at job 1 (completion 6); and the FR-247-AC-4 QPA sequence is `16, 15, 14, 9, 7` with L = 20. All of them agree with the specs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The EDF analysis and its checker ignore the stated jitter `J` and blocking `B`: utilization, `dbf` and the busy period use only `C`, `T` and `D`, and admission accepts an `edf` claim over a task with `J > 0` or `B > 0`. That breaks RT-2 ("a verdict holds for exactly those values"). The one-task set `C = 1, T = 2, D = 1, J = 1` (or `B = 1`) is refuted under fixed priority (`R + J = 2 > 1`) but settles `Proved{ClosedForm{EdfQpa}, Certified}` under EDF, although with one task every policy schedules identically. The checker cannot catch it, because it recomputes the same formula. The fix is either to refuse an `edf` claim when any task states nonzero `J` or `B`, or to use the jitter-aware `dbf` (`floor((t + J_i - D_i) / T_i) + 1`) with a blocking term. FR-247 and QSpec FR-419 need the same amendment, because both fix a `dbf` that has no `J`. | qsl-analyze/src/schedulability/edf.rs:13-48, qsl-replay/src/certificate/closed_form/recompute.rs:617-657, qsl-replay/src/certificate/closed_form/task_set.rs:345-370 |
| FND-002 | medium | `settle` takes a `ClosedFormCheck` from its caller and never binds it to the `(subject, outcome)` it settles. `ClosedFormCheck::Agrees` is publicly constructible, so `settle(s, tampered, &ClosedFormCheck::Agrees)`, or a check computed for another subject, gives `Proved{ClosedForm{..}, Certified}`. The basis analysis is taken from whichever subject is passed. The trusted settlement therefore rests on every future caller pairing them correctly. No production caller exists yet (`check_closed_form` is not implemented), which keeps this at medium. The fix is for `settle` to run `recompute` itself, or for `Agrees` to be constructible only by `recompute`. | qsl-replay/src/certificate/closed_form/settle.rs:168-203, qsl-replay/src/certificate/closed_form/recompute.rs:106-116 |

## Verdict

FIX. FND-001 is a soundness bug: EDF proves claims that are false under their stated premises. FND-002 is a trust-boundary gap in the settlement API. The fixed-priority and AMC-rtb paths are correct, and the checker is independent of the analyzer.

Checked and clean:

- Fixed-priority RTA: the start is `(q + 1) C_i + B_i`, the steps are `ceil((w + J_j) / T_j) C_j` over `hp(i)`, and the response time is `w + J_i - q T_i`. The busy period stays open while `w_q > (q + 1) T_i - J_i` (Tindell, Lehoczky). Every loop ends at a fixpoint or when the response time passes the bound, so no cap is needed.
- The checker's `job_fixpoints` requires the chain to start at `base`, each step to equal the recurrence, the last two iterates to be equal (least fixpoint, by monotonicity), the response time to be at most the bound, and every non-final job open with the final job closed. `fixed_priority_miss` checks that only the last iterate passes the bound, and recomputes each earlier job's completion to show the busy period stays open, as FR-248 requires.
- AMC-rtb: low mode uses `C(LO)` throughout. High mode uses `C(HI)` over `HI` interferers, and charges `LO` interferers a constant `ceil((R_i^LO + J_k) / T_k) C_k(LO)` from the task's own low-mode job-0 completion, the same in engine and checker. A low-mode miss gives `Miss` and refutes. `SufficientTestFailed` needs exact low-mode coverage and a valid high-mode miss chain for a `HI` task.
- EDF/QPA, without jitter: `U > 1` misses, L is the least fixpoint of `sum ceil(w / T) C`, the start is the greatest absolute deadline below L, and the stepping rule is `t <- h(t)` if `h(t) < t`, otherwise the deadline below `t`, until `h(t) <= d_min` (Zhang and Burns). The checker requires the exact sequence and its termination, and a miss needs `dbf(t) = demand > t > 0`.
- The `max_demand_points` stop happens before checking point `max + 1`, and cancellation is polled in every loop of both crates.

Recorded mediums and lows, not findings:

- `settle` maps `Stopped` to `Incomplete(cause)` and drops the `ScheduleLimit` setting and value that FR-248's row names. The PR body discloses this as a spec gap (STD-159).
- `AmcDeadlineBeyondPeriod` is a refusal that FR-246 and QSpec FR-419 do not list. It is justified, because AMC-rtb assumes `D <= T`, but the spec should state it.
- `ScheduleSubject::{higher_priority, by_priority, analysed, bound}` are shared by the engine and the checker, so a defect in the interference set would be common-mode. They live in the checker's crate, so CF-6 independence still holds for the arithmetic.
- The checker's EDF busy-period recomputation iterates to its fixpoint (up to the hyperperiod), so it is not linear in the evidence as FR-248 says.
- `quotient`/`ratio_or_zero` silently return 0 on a zero divisor, where an admission invariant would be clearer. The `TaskSetRefusal` messages print `{member:?}` Debug text.
- The checker has no tamper test for AMC high-mode evidence, for an EDF demand witness, or for a stated utilization.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6885f2141: `ScheduleSubject::new` now refuses an `edf` claim over any task with nonzero jitter or blocking, as `TaskSetRefusal::EdfIgnoresTerm { task, member }` (`invalid_model_binding`/`malformed-declaration`). `ScheduleSubject` has private fields and its only constructor is `new`, so `analyze`, `recompute` and `settle` all receive admitted subjects. `an_edf_claim_refuses_jitter_and_blocking` asserts the exact variant and member for J=1 and for B=1, asserts the code, asserts that the same sets miss under fixed priority, and asserts that plain `Ctl` stays schedulable under EDF. The PR body records the spec amendment needed (STD-159). |
| FND-002 | fixed | 6885f2141: `settle(subject, outcome, cancel)` now calls `recompute` itself and no longer takes a `ClosedFormCheck`, so a caller cannot pair an outcome with a check the checker did not compute for that subject. |
