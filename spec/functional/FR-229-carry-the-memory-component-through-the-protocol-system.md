---
id: FR-229
title: "Carry the memory component through the protocol system's memory-model seam"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-434
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-426
    type: depends_on
---
# FR-229: Carry the memory component through the protocol system's memory-model seam

## Description

`Tso` and `Ra` SHALL implement ADR-027's `MemoryModel` seam (FR-215),
giving the state key's memory member while a weak `parallel` is active
(ADR-025 MS-1), the thread observation an access evaluates over and the
write its shared delta goes through (ADR-025 MA-5), the memory values atoms
read (ADR-025 MS-2), memory steps as positions with a `memory` anchor
(ADR-025 MS-2, ADR-027 PB-2), and memory footprints and visibility beside
ADR-027's protocol footprints (ADR-025 MR-1, MR-2), with terminal states
read as ADR-025 MS-4 reads them.

## Use case

A verification operator reads a weak-memory counterexample and a claim's
atoms. A claim about `x` reads the value every thread would eventually see,
not a value still sitting in one thread's buffer; a precondition reads the
value the acting thread sees. State-space reduction knows that a buffered
store is invisible to a claim about `x` and that the flush is the step that
matters.

## Inputs

- A protocol state (FR-205) and an access's classification (FR-220).

## Outputs

- The memory member of `ProtocolKey` (FR-205) in QSpec FR-181's typed
  canonical form: buffers as sequences; message sequences in mo; views as
  maps from location to message position; the SC event graph; the race
  summary.
- `Tso` and `Ra` values of `MemoryModel`'s observe, write, internal steps,
  gate, split and merge, atom values and fairness.
- Memory locations `buf(b)`, `mem(ℓ)`, `msgs(ℓ)`, `view(b)`, `S`, the SC
  event graph and `race(ℓ)` in FR-216's footprints.

## Behavior

### State and observation

- The memory member SHALL hold the active weak `parallel`'s component and
  SHALL be empty while no weak `parallel` is active.
- An access SHALL apply its operation by the FR-120 rule over the thread
  observation: the model state with each shared location holding the value
  the model gives the acting thread, and branch-local locations as they are.
  Its precondition SHALL read that value, so a load is enabled only for
  values that satisfy it.
- The postcondition's delta at the shared location SHALL go to the memory
  model; branch-local writes SHALL apply in place.
- An RMW application whose post-state leaves its location unchanged SHALL
  apply as a load with the read part of its ordering.
- A `holds` atom SHALL read each shared location's memory value (under
  `tso`, memory; under `ra`, the mo-last message's value) and each
  branch-local location's thread value.
- `flush(b)` SHALL be a position observed with a `memory` anchor naming the
  step, and memory steps and fence steps SHALL be uncounted for intervals
  (QSpec FR-431, FR-435-AC-6).

### Terminal states

- Under `tso`, every terminal state SHALL have every buffer empty, and a
  fence, locked access, `send` or `join` waiting on a buffer SHALL never be
  a deadlock.
- Under `ra`, an SC access whose every message or slot choice would make the
  SC event graph cyclic SHALL be disabled at that state.

### Footprints and visibility

- Under `tso`, a store by `b` SHALL read and write `buf(b)`; a load by `b` of
  `ℓ` SHALL read `buf(b)` and `mem(ℓ)`; `flush(b)` SHALL read and write
  `buf(b)` and write `mem(ℓ)` for each `ℓ` that `b` may store; a locked
  access SHALL read `buf(b)` and read and write `mem(ℓ)`; a `seq_cst` fence
  SHALL read `buf(b)`.
- Under `ra`, a load by `b` of `ℓ` SHALL read `msgs(ℓ)` and read and write
  `view(b)`; a store or RMW SHALL read and write both; a fence SHALL read
  and write `view(b)`, and a `seq_cst` fence also `S`; every `seq_cst`
  access and fence SHALL also read and write the SC event graph.
- Under both, an access to a location with a non-atomic access SHALL also
  read and write `race(ℓ)`.
- A step SHALL be visible when it writes a memory value an atom reads: under
  `tso` a flush or locked access to such a location; under `ra` a store or
  RMW to it. For the race-freedom item, every access to a location with a
  non-atomic access SHALL be visible.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-229-AC-1 | Over ADR-025 §10's `SB`, the memory member under `tso` after `Sx` holds `left: [x := 1]`, `right: []`; under `ra` after `Sx` it holds messages `x0`, `x1` and `y0` with the views of §10's table; under `sc` it is empty at every state; under `tso` and `ra` it is empty in every joined state. | Test (TC-674) |
| FR-229-AC-2 | Under `tso`, after `Sx` alone, `holds(x.v = 1)` is false at that position and true after `flush(left)`, and `flush(left)`'s position carries a `memory` anchor. `holds(c1.r = 0)` reads `c1`'s register after `Ly`. A load of `x` by `left` whose precondition requires `x.v = 1` is enabled after `Sx`, because `left` sees its own buffered store; the same load by `right` is not. | Test (TC-674) |
| FR-229-AC-3 | A `compare_exchange(1, 2)` on `x` at 0 under `ra` applies as a load: it adds no message and moves only the acting thread's view. | Test (TC-674) |
| FR-229-AC-4 | Under `tso`, `Sx` is invisible to `always holds(x.v <= 1)` and `flush(left)` is visible; under `ra`, `Sx` is visible. Under `tso`, `Ly` and `Sy` are independent, and `flush(left)` and `Lx` are dependent through `mem(x)`. | Test (TC-674) |
| FR-229-AC-5 | Every terminal state of `SB` under `tso` has empty buffers, and its deadlock-freedom item settles `proved`. | Test (TC-674) |

## Dependencies

- ADR-025 §2 MA-5, §4 MS-1 to MS-4, §7 MR-1 and MR-2 (as amended by
  ADR-027); ADR-027 SE-1 to SE-3, PB-2, PB-3, FT-1.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  application rule), FR-205 (the state key), FR-215 (the seam), FR-216
  (footprints), FR-220 (classification).
- QSpec owns the memory-model contributions, the memory member of the state
  key and the memory-step transition identity forms (QSpec FR-434, FR-426,
  FR-181).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-434, FR-426 (Linear STD-138).
