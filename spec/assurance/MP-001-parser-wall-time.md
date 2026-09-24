---
id: MP-001
title: S1 parser wall time
type: MeasurementPlan
status: active
owner: peter
metric: qsl.bench.parser.wall_time
definition_version: qsl.bench.parser.wall_time-v2
stage: baseline
ground_truth_kind: mechanical
objective:
  direction: lower
subject_identity:
  name: qsl-cst S1 parser (qsl_cst::parse)
  version: "893269992f40f83086cbc54d91d913ff3e1a37f4"
statistical_design:
  population: >-
    The fixed, generated inputs of qsl-bench/benches/parser.rs, one
    dimension per criterion benchmark id; every input is deterministic.
  sampling: >-
    criterion 0.8.2 defaults unless the bench sets them: 3 s warm-up, then
    linearly growing iteration batches until the sample count is reached;
    the whole suite is run as one `cargo bench -p qsl-bench --bench parser`
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
    baseline: prior-collection
protected_apparatus:
  - qsl-bench/benches/parser.rs
  - qsl-bench/src/parse.rs
  - qsl-bench/Cargo.toml
negative_controls:
  - kind: gain-within-noise
    description: >-
      A candidate counts as an improvement on a benchmark only in one
      interleaved session that alternates the baseline revision and the
      candidate revision, five rounds each, on one machine: its median must
      be below the baseline revision's same-session median by more than
      the larger of the benchmark's stated variance in
      qsl-bench/BASELINE.md and that session's own MAD/median. A smaller
      change is recorded as no change (QSL-196 acceptance criterion 3). The
      absolute baseline table is informational and is never the comparison
      point.
  - kind: apparatus-edit
    description: >-
      The bench file, its input generator and the bench crate manifest are
      protected apparatus; a change that edits them re-baselines rather
      than claiming an improvement.
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
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: measures
---

# S1 parser wall time

## Decision Use

The benchmark set and noise floor that QSL-202 to QSL-206 are judged
against. A performance change claims an improvement on a benchmark only by
beating the baseline revision in one interleaved same-session A/B run, by more
than the larger of the benchmark's stated variance and that session's own
MAD/median. The recorded baseline values are informational: on a shared
machine the same code differs by up to half between sessions. The plan grades
no release and gates no merge.

## Population

Every criterion benchmark in `qsl-bench/benches/parser.rs`: one
`qsl_cst::parse` call over a generated complete-V1 source: N nested parenthesis
pairs (`parser/depth/N`) or N one-line functions (`parser/volume/N`). Each
benchmark id is one observation dimension. Inputs are generated, not sampled,
so the population is fixed by the protected apparatus.

## Collection Procedure

Run `make bench-parser` five times on one machine at one source revision,
recording `/proc/loadavg` before and after each run. The observed value per
benchmark is the median of the five runs' criterion point estimates, in
seconds per call.

## Interpretation

The figures are wall time on a shared, loaded machine. The decision rule's
`prior-collection` is the baseline revision's collection from the same
interleaved session, never the collection recorded here. The rule cannot state
a relative margin per benchmark, so the margin, max(stated variance, session
MAD/median), is applied as stated in `qsl-bench/BASELINE.md`.
A refused input (a parse that returns `resource_exhausted`) is still timed. The
time to reach the refusal is part of the baseline, and a fix that makes the
input parse changes what the benchmark measures.
