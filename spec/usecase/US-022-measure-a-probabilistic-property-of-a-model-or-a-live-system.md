---
id: US-022
title: "Measure a probabilistic property of a model or a live system"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-190
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-191
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-022: Measure a probabilistic property of a model or a live system

## Story

**As a** verification operator with a QSL state model, a workload that says
how often each operation runs, and requirements such as "p95 latency under
5 ms" or "available 99.9% of the time"
**I want** those requirements stated as probabilistic claims over the
model, measured with QSL's own simulator at the confidence the claim states,
and the same statistics monitored over the live system's trace
**So that** I get a measurement that says it is a measurement, with its
method, sample count and confidence, that fails my pipeline when it rejects
the requirement, that reproduces from its seed, and that is never counted as
a proof.

## Context

ADR-024 designs probabilistic models (random parameters, workloads,
rewards), the property forms, the statistical engine EN-4 and window
aggregates in monitors. FR-120's `ModelSystem` gives the successor relation
and FR-101's sampler draws traces. The new requirements add the declarations
at S3, step probabilities, weighted sampling, the engine, the `measured`
verdict and aggregate atoms.

## Acceptance Examples (Illustrative)

### US-022-EX-1: A latency requirement is measured and accepted

- **Given** ADR-024 §7.2's `Service` model, the workload `Steady` and
  `quantile 0.95 of (accumulate duration …) <= 5 ms` with `α = β = 0.01`
  and `ι = 0.01`.
- **When** the operator requests the claim with statistical evidence and the
  fixed-sample method.
- **Then** it settles "measured: accepted" with an interval above 0.95, the
  sample count 23,026 and finite-sample coverage, and it is not counted as
  proof.

### US-022-EX-2: A tighter requirement is rejected and fails the pipeline

- **Given** the same claim at `<= 2 ms`.
- **When** the operator requests it with the sequential method.
- **Then** it settles "measured: rejected", the pipeline fails as on a
  violation, the value is never `refuted`, and a sampled witness with
  latency 3 ms replays through the model.

### US-022-EX-3: Long-run availability is measured with its weaker coverage

- **Given** ADR-024 §7.3's `Avail` model and `long-run fraction
  holds(v.up) >= 0.999`.
- **When** the operator requests it with the regenerative method.
- **Then** it settles "measured: accepted" with coverage `Asymptotic`.

### US-022-EX-4: The same statistic is monitored on the live system

- **Given** a monitor clause `quantile 0.95 of e.latency where holds(e is
  Response) over past[0, 300 s] <= 5 ms` over an observed timestamped trace.
- **When** the monitor evaluates it.
- **Then** a window whose 95th-percentile latency exceeds 5 ms is a
  violation at that position, and a window with too few responses is
  undefined there, not a pass.

## Priority and Risk (Informative)

Priority: High. Latency and availability requirements are the most common
non-functional requirements, and neither is a Boolean property of a
behaviour. Risk: probabilities near 1 need many samples; the sequential
method reduces that cost.

## Traceability (Informative)

- [FR-185](../functional/FR-185-declare-random-parameters-workloads-and-rewards.md)
- [FR-186](../functional/FR-186-check-probabilistic-claim-forms-at-s3.md)
- [FR-187](../functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md)
- [FR-188](../functional/FR-188-draw-weighted-choices-with-the-revised-sampler.md)
- [FR-189](../functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md)
- [FR-190](../functional/FR-190-measure-a-long-run-fraction-by-regeneration.md)
- [FR-191](../functional/FR-191-bound-and-reproduce-a-statistical-run.md)
- [FR-192](../functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md)
- [FR-193](../functional/FR-193-keep-and-replay-sampled-witnesses.md)
- [FR-194](../functional/FR-194-evaluate-window-aggregates-in-monitors.md)
