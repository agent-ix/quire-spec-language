---
id: MP-006
title: S3 checker one-shot wall time and peak RSS
type: MeasurementPlan
status: active
owner: peter
metric: qsl.bench.probe.check_wall_time_and_rss
definition_version: qsl.bench.probe.check_wall_time_and_rss-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: qsl-semantics S3 checker (PackageDeclarations::check)
  version: "67f449895414c38af7907f7a4c27292c66911f3c"
statistical_design:
  population: >-
    The fixed, generated inputs `qsl-bench-probe check <shape> <n>` builds,
    one observation dimension per shape, size and quantity (`wall_time` or
    `peak_rss`); every input is deterministic.
  sampling: >-
    One probe process per input per repetition: the process builds the
    input, times one `PackageDeclarations::check` over the whole package,
    and reads its own peak RSS (`VmHWM`) after the check.
  repetitions: 5
  estimator: median
  error_model: >-
    Between-repetition drift from concurrent builds on the shared machine
    (load average recorded before and after every repetition), CPU
    frequency and cache state. Peak RSS is near-deterministic for a given
    build; wall time is not.
  uncertainty: >-
    Per dimension, the median absolute deviation of the five repetitions
    relative to their median; the full min-max range is recorded beside it.
  decision_rule:
    comparator: lt
    baseline: prior-collection
protected_apparatus:
  - qsl-bench/src/bin/qsl-bench-probe.rs
  - qsl-bench/src/check.rs
  - qsl-bench/src/rss.rs
negative_controls:
  - kind: gain-within-noise
    description: >-
      A candidate improves on a dimension only in one interleaved session
      that alternates the baseline revision and the candidate revision,
      five rounds each, on one machine: its median must be below the
      baseline revision's same-session median by more than the larger of
      either side's MAD/median in that session. A smaller change is
      recorded as no change (qsl-bench/BASELINE.md, "How to claim an
      improvement").
  - kind: apparatus-edit
    description: >-
      The probe, its input generator and its RSS reader are protected
      apparatus; a change that edits what they build or read re-baselines
      rather than claiming an improvement.
  - kind: selective-reporting
    description: >-
      Every round of an interleaved session is recorded, both sides; a
      session is not rerun until it passes.
  - kind: stale-evidence
    description: >-
      Every recorded collection names the exact source revision and the
      machine it ran on; a result from another revision or machine is not
      compared against this baseline.
relationships:
  - target: ix://agent-ix/quire-specification/FR-146
    type: measures
---

# S3 checker one-shot wall time and peak RSS

## Decision Use

Inputs too slow for criterion to sample repeatedly, such as call chains of
4,000 and 8,000 functions, and the memory a check needs. QSL-203 used this plan
to show the termination fix's gain on long chains, and the memory growth
before and after it. The plan grades no release and gates no merge.

## Population

Each `qsl-bench-probe check <shape> <n>` run, where `<shape>` is `chain`,
`independent` or `self-recursive`. Each (shape, size, quantity) triple is one
observation dimension:

- `wall_time`, in seconds: one `PackageDeclarations::check` over the whole
  package. Building the input is excluded.
- `peak_rss`, in KiB: `VmHWM` of the probe process, which builds and checks
  only that one input.

## Collection Procedure

Run `qsl-bench-probe check <shape> <n>` once per size per round, one process
each, five rounds, on one machine at one source revision. Record
`/proc/loadavg` before and after each round. The observed value per dimension
is the median of the five rounds.

## Interpretation

The wall-time figures are wall time on a shared, loaded machine. The decision
rule's `prior-collection` is the baseline revision's collection from the same
interleaved session. Peak RSS is process-wide and monotone, which is why each
process measures a single input.
