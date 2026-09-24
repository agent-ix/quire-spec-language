---
id: MP-004
title: Model-layer stage wall time
type: MeasurementPlan
status: active
owner: peter
metric: qsl.bench.model.wall_time
definition_version: qsl.bench.model.wall_time-v1
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: qsl-semantics model layer (crate::model)
  version: "893269992f40f83086cbc54d91d913ff3e1a37f4"
statistical_design:
  population: >-
    The fixed, generated inputs of qsl-bench/benches/model.rs, one
    dimension per criterion benchmark id; every input is deterministic.
  sampling: >-
    criterion 0.8.2 defaults unless the bench sets them: 3 s warm-up, then
    linearly growing iteration batches until the sample count is reached;
    the whole suite is run as one `cargo bench -p qsl-bench --bench model`
    invocation per repetition.
  repetitions: 5
  estimator: median
  error_model: >-
    Between-repetition drift from concurrent builds on the shared machine
    (load average recorded before and after every repetition), CPU
    frequency and cache state; within-repetition sampling noise.
  uncertainty: >-
    Per benchmark, the stated variance in qsl-bench/BASELINE.md: the larger
    of the median absolute deviation of the five repetitions' point
    estimates relative to their median, and the widest criterion 95%
    confidence-interval half-width relative to its estimate. The full
    min-max range of the five repetitions is recorded beside it.
  decision_rule:
    comparator: lt
    baseline: external-reference
protected_apparatus:
  - qsl-bench/benches/model.rs
  - qsl-bench/src/model.rs
  - qsl-bench/Cargo.toml
negative_controls:
  - kind: gain-within-noise
    description: >-
      A later run counts as an improvement on a benchmark only when its
      median over five repetitions is below that benchmark's baseline
      median reduced by the benchmark's stated variance, in a back-to-back
      A/B run against the baseline revision; a smaller change is recorded
      as no change (QSL-196 acceptance criterion 3).
  - kind: apparatus-edit
    description: >-
      The bench file, its input generator and the bench crate manifest are
      protected apparatus; a change that edits them re-baselines rather
      than claiming an improvement.
  - kind: stale-evidence
    description: >-
      Every recorded collection names the exact source revision and the
      machine it ran on; a result from another revision or machine is not
      compared against this baseline.
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: measures
---

# Model-layer stage wall time

## Decision Use

The baseline that QSL-202 to QSL-206 compare against. A performance change
claims an improvement on a benchmark only when it clears this baseline by more
than the benchmark's recorded uncertainty. The plan grades no release and gates
no merge.

## Population

Every criterion benchmark in `qsl-bench/benches/model.rs`: one call of each
model-layer stage (`intake::admit`, `intake::read_records`, `normalize`,
`object_universe_of`, `admit_binding`, `admit_invocation`, `all_instances`) and
one bare parse by each JSON parser intake runs, over an N-type `DomainPackage`
built through FR-154 intake. Each benchmark id is one observation dimension.
Inputs are generated, not sampled, so the population is fixed by the protected
apparatus.

## Collection Procedure

Run `make bench-model` five times on one machine at one source revision,
recording `/proc/loadavg` before and after each run. The observed value per
benchmark is the median of the five runs' criterion point estimates, in
seconds per call.

## Interpretation

The figures are wall time on a shared, loaded machine. They compare only with
runs on the same machine under similar load. The decision rule's per-benchmark
reference is external to this plan: the benchmark's baseline median multiplied
by one minus its stated variance, both taken from `qsl-bench/BASELINE.md`.
A refused input (a parse that returns `resource_exhausted`) is still timed. The
time to reach the refusal is part of the baseline, and a fix that makes the
input parse changes what the benchmark measures.
