---
id: FR-226
title: "State the code-link preconditions of a memory-model verdict and route by resolved model"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-439
    type: depends_on
---
# FR-226: State the code-link preconditions of a memory-model verdict and route by resolved model

## Description

A proof under resolved model `M` transfers to concurrent code only under the
preconditions MX-1 to MX-4 below, which QSL states as preconditions of the
code claim (ADR-025 §8). S3 SHALL warn of each load-buffering shape,
whatever the model a request later resolves (ADR-025 MX-4). EN-1's provider manifest SHALL advertise the
memory models it decides, and QSL's `route` candidate step (FR-075) SHALL
give an item over a subject with a weak `parallel` only candidates that
advertise its resolved model (ADR-025 MX-7).

## Use case

A verification operator has proved a claim under `ra` and wants to rely on
it for their Rust code. They read which conditions the code must meet: one
atomic access per attempted operation, at least the declared ordering, race
freedom for plain fields, and a target that compiles RC11 soundly. The
compiler warns them where a relaxed load followed by a store needs a target
that excludes load buffering.

## Inputs

- The checked protocol clause with its classified accesses (FR-220) and
  source defaults (FR-219).
- The registered backends and their QSpec FR-290 provider manifests
  (FR-075).

## Outputs

- The S3 warning `memory.load-buffering-shape`, naming the load and the
  store.
- EN-1's provider manifest entry listing `sc`, `tso` and `ra`.
- The candidate set of an item over a subject with a weak `parallel`.

## Preconditions

A code claim inherited from a verdict under resolved model `M`, read through
the abstraction relation (QSpec FR-353, ADR-017 AR-2), holds when all of the
following hold, together with the per-function proofs of each bound
function against its operation's contract (ADR-017 AR-5):

- **MX-1 Threads.** Each branch of a weak `parallel` runs as one thread
  whose program order follows the branch's causal order; entering the
  `parallel` and its `join` are thread spawn and join; a channel's `send` and
  `receive` synchronise as a release and an acquire.
- **MX-2 Locations.** Each shared location with an atomic access binds to a
  Rust atomic of a width that holds the field's domain; each shared location
  whose accesses are all non-atomic binds to plain state; a location with
  both binds to an atomic whose non-atomic accesses are `unsafe` non-atomic
  reads and writes; each branch-local location binds to state only its own
  thread accesses.
- **MX-3 Accesses.** The function bound to an attempted operation performs,
  on shared locations, exactly the one access its classification gives: a
  load, a store, or one RMW instruction. An atomic access's ordering is at
  least the declared ordering in C11's order. A `fence` control binds to a
  `std::sync::atomic::fence` with at least its ordering.
- **MX-4 Per model.** Under `sc`, every attempted operation that touches a
  shared location runs atomically with respect to every other branch: one
  `SeqCst` access, or a critical section under one lock guarding every
  shared location it touches. Under `tso`, the target is x86-64 with the
  standard C11-to-x86 mapping, every atomic shared load is at least
  `acquire` and every atomic shared store at least `release`; non-atomic
  shared accesses are covered by the race-freedom item below. Under `ra`, the target has a
  compilation scheme sound for RC11, and a branch with a load-buffering
  shape also needs a target whose compiled code excludes load buffering.
  Under either weak model with a non-atomic access, the subject's
  race-freedom item (FR-224) is proved.

## Behavior

- When a branch of a `parallel` has a `relaxed` or non-atomic load followed
  in program order by a store to another shared location, with no `acquire`
  or `seq_cst` load and no `acquire` fence between them, S3 SHALL report the
  warning `memory.load-buffering-shape`, naming the load and the store. The
  warning SHALL leave the package's checked result unchanged.
- EN-1's provider manifest SHALL advertise the memory models `sc`, `tso` and
  `ra` by the names of the `memory` clause.
- The `route` candidate step SHALL include in the candidate set of an item
  over a subject with a weak `parallel` only backends whose manifest
  advertises the subject's resolved model, and SHALL write the request with
  the resolved model unchanged.
- When no backend advertises the resolved model, the `route` candidate
  step SHALL give the item an empty candidate set, and the item is written
  like any other with an empty set (FR-075).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-226-AC-1 | A branch `r1 := x (relaxed); y := 1 (relaxed)` produces the warning `memory.load-buffering-shape` naming both attempts, and the package compiles. With the load `acquire`, or an `acquire` fence between the two, no warning is produced. A load followed by a store to the same location produces none. | Test (TC-671) |
| FR-226-AC-2 | EN-1's registered provider manifest lists the memory models `sc`, `tso` and `ra`. | Test (TC-671) |
| FR-226-AC-3 | With EN-1 and a test candidate whose manifest advertises only `sc`, an item over `SB` resolved to `tso` has the candidate set `{EN-1}`. With the test candidate alone, the item's candidate set is empty, and the `sc` request over `SB` has the candidate set `{test candidate}`. | Test (TC-671) |

## Dependencies

- ADR-025 §8 MX-1 to MX-7; ADR-017 AR-2, AR-5.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md)
  (candidates and manifests), FR-219, FR-220, FR-224.
- QSpec owns the provider manifest's memory-model member and the routing by
  resolved model (QSpec FR-439, FR-290) and the abstraction relation (QSpec
  FR-353).

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-439 (Linear STD-138).
