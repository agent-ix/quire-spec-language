---
id: FR-145
title: "Replay a refinement counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: depends_on
---
# FR-145: Replay a refinement counterexample

## Description

QSL SHALL replay a refinement counterexample (FR-142, FR-143) through the
layer-6 replay facade at ADR-011 E9 (ADR-020 RC-2, RC-3). The
`ReplaySource::ModelTrace` arm, entered through
`qsl_replay::replay_model_trace` (FR-128), recompiles the concrete package
and the abstract subject's package, re-executes the concrete steps,
recomputes history values, mapped states and candidate sets along the
trace, and reruns `check_step` (FR-141) at the failing position, or for a
lasso checks it fair under `F_C` and evaluates the violated `F_A`
constraint over its loop. Replay needs no model checker.

## Use case

A verification operator, or an auditor with only the source and the
documents, receives a refuted refinement. Replay rebuilds both models,
re-runs the concrete steps, recomputes what the abstract model saw, and
reaches the same failure at the same position. A counterexample edited by
hand, or produced by an engine that disagrees with `check_step`, does not
count.

## Inputs

- FR-128's inputs, with these additions: the byte provision also holds the
  abstract subject's initial snapshots, and, when the abstract model is a
  dependency's, that dependency's digest-addressed source (ADR-015).
- A `WitnessEnvelope<TemporalCounterexample>` whose source is
  `ReplaySource::ModelTrace` naming the refinement node, and whose payload's
  `refinement` member holds the recorded `RefinementFailure`.

## Outputs

- An FR-072 replay result on the `ModelTrace` arm whose arm result names
  the `RefinementFailure` it reproduced, or a typed `ReplayRefusal` with no
  partial result.

## Behavior

- The executor SHALL recompile and check the concrete package by FR-098's
  rules and the abstract subject's package with it (the same package, or
  the dependency by ADR-015), refusing by FR-098's stale `package_id` rule
  on a mismatch.
- It SHALL resolve the refinement node by the envelope's identity and
  refuse `stale_dependency`/`revision-mismatch`, naming both identities,
  when it differs from the recompile. It SHALL read `F_C` and `F_A` from the
  recompiled node, refusing `stale_dependency`/`revision-mismatch` naming
  both sets when the payload's fairness set differs.
- It SHALL re-admit both subjects, the abstract one with the derived
  universes (ADR-020 RM-2), and re-execute the concrete prefix and loop by
  FR-128's rules, with FR-128's refusals for a step not enabled, a
  post-state that differs from the recorded one, an `initial` index out of
  range and a loop that does not close.
- While the concrete side is a protocol subject, the executor SHALL
  re-execute through `ProtocolSystem` with ADR-027 PX-2's refusals.
- Along the trace the executor SHALL recompute history values (FR-138),
  mapped states (FR-140) and candidate sets by `check_initial` and
  `check_step` (FR-141).
- **Prefix.** For a safety failure it SHALL run `check_step` (or
  `check_initial` for `InitialNotAbstract`) at each position and stop at
  the first failure. A failure equal in kind and position to the recorded
  one SHALL settle `reproduced-with-evaluated-witness`. A failure of a
  different kind or at a different position, or no failure, SHALL settle
  `inconclusive`, `ReplayParity`.
- **Lasso.** For `Divergence` or `AbstractUnfair` it SHALL recompute history
  values and candidate sets over the prefix and one pass of the loop, and
  refuse `invalid_runtime_input`/`invalid-value` when those at the loop's
  last position differ from those at its entry. It SHALL check the loop
  fair under `F_C`, reading enabledness from the concrete system at every
  loop state (over a protocol subject, ADR-027 PA-1's classes with the
  scheduler constraints), refusing an unfair loop with
  `invalid_runtime_input`/`invalid-value`. It SHALL then check that the
  recorded `F_A` constraint is enabled at every loop state's mapped state
  and taken by no loop step, and classify the loop as `Divergence` when
  every loop step is `Taken::Stutter` and `AbstractUnfair` otherwise.
  Agreement with the recorded failure SHALL settle
  `reproduced-with-evaluated-witness`; disagreement SHALL settle
  `inconclusive`, `ReplayParity`.
- **Undefined mapping.** For `kind: UndefinedEvaluation` it SHALL run
  `check_initial` and `check_step` along the prefix and stop at the first
  `Undefined`. One at `where`, with an undefined cause equal to the
  payload's, SHALL settle `reproduced-with-evaluated-witness` (ADR-020
  RC-2, ADR-018 UE-5); any other result SHALL settle `inconclusive`,
  `ReplayParity`.
- A `MappingUndetermined` or `UndecidedSuccessor` during replay SHALL
  settle `inconclusive`, `ReplayParity`.
- An internal fault SHALL refuse with an `InternalFault`. Replay SHALL read
  no path, environment variable, clock or search location, and SHALL give
  the same result for the same request and envelope.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-145-AC-1 | FR-142-AC-2's lost-update counterexample replays, recomputing mapped values 0, 0, 0, 1, 1, to `reproduced-with-evaluated-witness` naming `AbstractStepRejected{position: 4, transition: inc(c), cause: Postcondition}`. FR-142-AC-3's `InitialNotAbstract` and `StutterChanged` counterexamples, FR-142-AC-4's broken history update and FR-142-AC-5's `Coin` frame failure each reproduce. | Test (TC-550) |
| FR-145-AC-2 | FR-143-AC-1's divergence lasso reproduces `Divergence{constraint: inc}`; FR-143-AC-2's terminal-stutter lasso reproduces `Divergence`; FR-143-AC-3's lasso reproduces `AbstractUnfair{constraint: flipB}`. | Test (TC-550) |
| FR-145-AC-3 | The lost-update counterexample with its recorded failure changed to position 3 settles `inconclusive`, `ReplayParity`; the divergence lasso with its recorded failure changed to `AbstractUnfair` settles `inconclusive`, `ReplayParity`; the divergence lasso replayed against the `CasRefinesCounter` envelope's fairness, carrying that refinement's node identity and `F_C`, refuses as unfair, since `beginA` is enabled and never taken at `(0, 0, f, 0, f)`. | Test (TC-550) |
| FR-145-AC-4 | Refusals settle no result: an envelope naming another refinement node (`stale_dependency`/`revision-mismatch` naming both identities); a payload fairness set with `inc`'s granularity changed (`stale_dependency`/`revision-mismatch` naming both sets); a lasso over `RegisterHistory` with `ensure fair weak Spec::Register::write`, stem `write(r, 1)` and loop `write(r, 1)`, whose loop returns to its entry concrete state (`value` 1) with `last` 0 at entry and 1 at its end (`invalid_runtime_input`/`invalid-value`); an abstract package source edit that changes its `package_id` (FR-098's stale rule). Replaying one envelope twice gives equal results. | Test (TC-550) |
| FR-145-AC-5 | FR-142-AC-8's counterexample replays to `reproduced-with-evaluated-witness`; the same payload with its cause changed to `precondition-false` settles `inconclusive`, `ReplayParity`. | Test (TC-554) |

## Dependencies

- ADR-020 §6 RC-1 to RC-3; ADR-018 §5 CX-3; ADR-015; ADR-027 PX-1 and
  PX-2.
- [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-098](FR-098-execute-a-replay-request.md),
  [FR-128](FR-128-replay-a-model-counterexample.md) (the `ModelTrace` arm
  and its refusals),
  [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md).
- FR-144 settles the item from the replay result.

## References

- The QSpec half (the `RefinementFailure` wire and replay rules): QSpec FR-379 (Linear STD-133).
