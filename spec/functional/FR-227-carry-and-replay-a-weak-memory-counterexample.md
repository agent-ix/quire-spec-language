---
id: FR-227
title: "Carry and replay a weak-memory counterexample"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-439
    type: depends_on
---
# FR-227: Carry and replay a weak-memory counterexample

## Description

A counterexample over a subject with a weak `parallel` SHALL be ADR-027
PX-1's protocol counterexample whose steps include memory steps, whose `ra`
loads and stores name their message or slot by depth, and whose every step
carries its post-state's memory component in canonical form; it SHALL name
the resolved model of each outermost `parallel` and list the memory
constraints with their `memory` origin (ADR-025 MK-1). Replay SHALL
re-execute each step through the protocol system under the resolved models,
recompute the memory component, check a lasso against the memory
constraints, and replay a race counterexample by vector clocks over the
replayed prefix (ADR-025 MK-2, DR-6).

## Use case

A verification operator receives a `tso` counterexample to store buffering
and replays it without the model checker. Each step is re-run, each buffer
is recomputed and compared with the one shown, and the claim evaluates
false at the last position. A race report is replayed by an independent
happens-before computation, so a summary bug cannot produce a false race.

## Inputs

- FR-217's replay request and envelope, whose subject names its resolved
  models, whose steps carry memory components, and whose `kind` is
  `Formula`, `Deadlock`, `Race{location, earlier, later}` or
  `UndefinedEvaluation{where, cause}`.
- The subject's memory bounds: `max_store_buffer: Option<u64>` and
  `max_messages: Option<u64>`, the values the run used, each `None`
  meaning 4 (FR-225).

## Outputs

- An FR-072 replay result on the `ModelTrace` arm, or a typed
  `ReplayRefusal`.

## Behavior

- The executor SHALL build the protocol system with the envelope's resolved
  models and re-execute each step by FR-217: an attempt through the FR-120
  rule over the thread observation, a memory step by its model.
- The executor SHALL build each `Tso` component under the subject's
  `max_store_buffer` and each `Ra` component under its `max_messages`, as
  the run built them (FR-225).
- If a step takes a store that the bound cuts at its pre-state, then the
  executor SHALL refuse it with `invalid_runtime_input`/`invalid-value`,
  naming the step: the cut store is enabled but left unexpanded by the
  bound, so no run under the bound takes it. The executor SHALL NOT
  otherwise compare the bounds with the run's.
- The executor SHALL read a cut store as enabled wherever it reads
  enabledness, the lasso fairness check against the memory and scheduler
  constraints included, as FR-225 does (ADR-025 MB-3). A store is a step of
  its thread's branch, in that branch's FR-212 class, so a cut store keeps
  its branch's class enabled; `flush` and the other memory steps are in no
  class (ADR-025 MF-4).
- When a step names a message or slot depth that does not exist at its
  pre-state, the executor SHALL refuse `invalid_runtime_input`/
  `invalid-value`, naming the step.
- When the recomputed memory component of a step differs from the carried
  one, the executor SHALL refuse `stale_dependency`/`content-mismatch`,
  naming the step.
- The executor SHALL check a lasso against the authored constraints, the
  scheduler constraints and the memory constraints (FR-228), and refuse an
  unfair lasso with `invalid_runtime_input`/`invalid-value`.
- For `kind: UndefinedEvaluation`, the executor SHALL evaluate the
  claim's letters along the replayed prefix and settle by ADR-018 UE-5
  (ADR-025 MV-1) when `where.locus` names a letter of the claim, and SHALL
  replay by FR-217's constraint-locus rule when it names a state constraint.
  The executor SHALL evaluate the constraint at each replayed state over the
  recomputed memory component as FR-158 "Protocol subjects" reads it: a
  shared location of a weak `parallel` holds its memory value (under `tso`
  the value in memory, under `ra` the value of its mo-last message), and a
  value still in a store buffer or a message that is not mo-last is never
  read.
- For `kind: Race`, the executor SHALL compute happens-before over the
  replayed prefix from scratch with vector clocks over ADR-025 DR-2's
  synchronisation, independent of the race summary, and check that the
  accesses at `earlier` and `later` are by different threads, touch
  `location`, include a write and a non-atomic access, and are unordered.
  The executor SHALL settle agreement `reproduced-with-evaluated-witness`
  and disagreement `inconclusive`, `ReplayParity`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-227-AC-1 | `SB`'s `tso` counterexample carries, at each of its four positions, both buffers as in ADR-025 §10's table and names `SB::Both` resolved to `tso`; replay recomputes each buffer and settles `reproduced-with-evaluated-witness`. Replaying the same steps with the resolved model changed to `sc` refuses `stale_dependency`/`content-mismatch` at the first step whose post-state differs. | Test (TC-672) |
| FR-227-AC-2 | `SB`'s `ra` counterexample names `Lx`'s message by depth 1 and replays to `reproduced-with-evaluated-witness`. Changing that depth to 2 refuses `invalid_runtime_input`/`invalid-value`; changing a carried message view refuses `stale_dependency`/`content-mismatch`. | Test (TC-672) |
| FR-227-AC-3 | FR-224-AC-2's race counterexample under `ra` replays to `reproduced-with-evaluated-witness`. An envelope whose `earlier` names the flag store, which is not an access to `data`, settles `inconclusive`, `ReplayParity`; so does the AC-1 shape of FR-224, whose `data` load runs only after its flag load read 1, with a `release`/`acquire` flag and a forged race on `data`. | Test (TC-672) |
| FR-227-AC-4 | The lasso of FR-228-AC-2, in which `w`'s buffer stays `[x := 1]` through `r`'s repeated load of `x`, refuses `invalid_runtime_input`/`invalid-value` as unfair under `flush(w)`'s constraint. Under `ra`, replaying an FR-228-AC-3 lasso whose loop contains a repeated load of location `ℓ` by thread `b` and no load by `b` from `ℓ`'s mo-last message refuses `invalid_runtime_input`/`invalid-value` as unfair under the strong visibility constraint for `(b, ℓ)`. | Test (TC-672) |
| FR-227-AC-5 | `SB`'s `tso` counterexample replays to `reproduced-with-evaluated-witness` with `max_store_buffer` unset and with 1, since no buffer holds more than one store. `SB`'s `ra` counterexample replays to `reproduced-with-evaluated-witness` with `max_messages` unset (4); with `max_messages` 1 it refuses `invalid_runtime_input`/`invalid-value` at its first store, which would add a second message to its location. | Test (TC-1230) |
| FR-227-AC-6 | FR-228-AC-2's program under `ra` with `max_messages` 1, where `w`'s store `x := 1` is cut at every state because it would add a second message to `x`: the hand-built lasso whose stem is `fork` and whose loop is `r`'s load reading 0 refuses `invalid_runtime_input`/`invalid-value` as unfair under `w`'s scheduler constraint, since `w`'s store is enabled at every loop state and never taken. A replay that read the cut store as not enabled would find `w` with no enabled step and accept the lasso. | Test (TC-1230) |
| FR-227-AC-7 | Over FR-217-AC-9's `SB` with `Edge` named by the request and `SB::Both` resolved to `tso`, `Sx` alone leaves `x`'s memory value 0, so `Edge` holds after it, and the run's `Violated` counterexample with cause `UndefinedEvaluation` is two steps, a store and its own thread's `flush` (`Sx`, `flush(left)` or `Sy`, `flush(right)`), ending at the first reached state where a cell's memory value is 1, with `where.locus` naming `Edge`; it replays to `reproduced-with-evaluated-witness`. The same payload with its `flush` step removed, which ends with the store still buffered and both memory values 0, settles `inconclusive`, `ReplayParity`. Resolved to `ra`, the counterexample is one store step, because the stored location's mo-last message holds 1 at once, and it replays to `reproduced-with-evaluated-witness`. | Test (TC-672) |

## Dependencies

- ADR-025 §9 MK-1 and MK-2, §14 DR-6 (as amended by ADR-027); ADR-027 PX-1,
  PX-2; ADR-018 CX-2, CX-3.
- [FR-128](FR-128-replay-a-model-counterexample.md), FR-217 (protocol
  replay), FR-221, FR-222, FR-224, FR-228.
- QSpec owns the counterexample wire for memory components and races and
  their replay (QSpec FR-439, FR-438).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-439 (Linear STD-138).
