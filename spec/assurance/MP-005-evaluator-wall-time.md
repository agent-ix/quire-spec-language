---
id: MP-005
title: Evaluator cost per call frame
type: MeasurementPlan
status: active
owner: peter
metric: qsl.bench.evaluator.wall_time
definition_version: qsl.bench.evaluator.wall_time-v2
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: qsl-eval S6a evaluator (CheckedPackageEvaluation::call)
  version: "qsl-eval/Cargo.toml"
statistical_design:
  population: >-
    The fixed, generated inputs of qsl-bench/benches/evaluator.rs, one
    dimension per criterion benchmark id; every input is deterministic.
  sampling: >-
    criterion defaults unless the bench sets them: 3 s warm-up, then
    linearly growing iteration batches until the sample count is reached;
    the whole suite is run as one `cargo bench -p qsl-bench --bench evaluator`
    invocation per repetition.
  repetitions: 5
  estimator: median
  error_model: >-
    Between-repetition drift from concurrent builds on the shared machine
    (load average recorded before and after every repetition), CPU
    frequency and cache state; within-repetition sampling noise.
  uncertainty: >-
    Per benchmark, the session noise is computed from the five current-main and five candidate observations in one interleaved session: the largest MAD/median on either side and the widest criterion 95% confidence-interval half-width relative to its estimate. The full min-max range for both sides is recorded.
  decision_rule:
    comparator: lt
    baseline: prior-collection
protected_apparatus:
  - qsl-bench/benches/evaluator.rs
  - qsl-bench/src/check.rs
  - qsl-bench/Cargo.toml
negative_controls:
  - kind: gain-within-noise
    description: >-
      A candidate counts as an improvement on a benchmark only in one
      interleaved session that alternates the baseline revision and the
      candidate revision, five rounds each, on one machine: its median must
      be below the baseline revision's same-session median by more than
      the session noise computed from those same-session A/B observations, as defined in qsl-bench/BENCHMARKING.md. A smaller change is recorded as no change.
  - kind: apparatus-edit
    description: >-
      The bench file, its input generator and the bench crate manifest are
      protected apparatus; a change that edits them requires a fresh
      current-main/candidate comparison with the changed apparatus.
  - kind: selective-reporting
    description: >-
      Every round of an interleaved session is recorded, both sides; a
      session is not rerun until it passes.
  - kind: stale-evidence
    description: >-
      Every recorded collection names the machine it ran on; a result from
      another machine is not compared with this paired session.
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: measures
---

# Evaluator cost per call frame

## Decision Use

A performance claim compares current main with the candidate in one interleaved same-session A/B run. It uses only the session noise defined in qsl-bench/BENCHMARKING.md; measurements from earlier sessions are not comparison thresholds. This plan grades no release and gates no merge.

## Population

Every criterion benchmark in `qsl-bench/benches/evaluator.rs`: one
`CheckedPackage::call` of `f0(5)` on a checked, linked N-function call chain,
which pushes N call frames (`evaluator/call_chain/N`). Each benchmark id is one
observation dimension. Inputs are generated, not sampled, so the population is
fixed by the protected apparatus.

## Collection Procedure

Run `make bench-evaluator` five times on one machine,
recording `/proc/loadavg` before and after each run. The observed value per
benchmark is the median of the five runs' criterion point estimates, in
seconds per call.

## Interpretation

The decision rule’s `prior-collection` is the current-main side of the same interleaved session. The margin is the session noise computed from that paired run, as stated in `qsl-bench/BENCHMARKING.md`; no earlier collection is used.
A refused input (a parse that returns `resource_exhausted`) is still timed. The
time to reach the refusal is part of the measured outcome; a fix that makes the input parse changes what the benchmark measures.
