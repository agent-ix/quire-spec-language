---
id: FR-262
title: "Evaluate, key and handle values and calls at any depth"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
---
# FR-262: Evaluate, key and handle values and calls at any depth

## Description

The S6a evaluator and simulation SHALL evaluate calls of any depth and
handle values and value types of any depth, bounded by the execution budget
under `quire.value.accounting/v1` (ADR-030 D-4.7). QSpec FR-146 defines
execution fuel as the `work_units` limit, with one `function.call` charge per
call and no call-depth counter; this requirement carries it through QSL's
evaluator.

## Behavior

1. **Explicit task stack.** Expression evaluation and calls SHALL run on an
   explicit heap task stack, so no evaluation step uses native recursion
   whose depth grows with the input.
2. **Fuel.** Each call SHALL charge `function.call` under
   `quire.value.accounting/v1`. When a charge would exceed `work_units`, the
   evaluator SHALL return `incomplete { limit_kind: work_units, ... }` at the
   denied charge, naming the counter, which is its setting.
3. **Iterative value types.** `quire_exact::ValueType` SHALL clone, compare,
   hash, format for debug and drop over an explicit heap stack, as
   `quire_exact::Value` does, including on `no_std` targets.
4. **State keys.** A simulation state key and a transition identity SHALL be
   written into `quire-canonical`'s canonical writer from an explicit stack
   through an event-source bound on `TransitionSystem::Key` and
   `TransitionId`. `quire_exact::Value` SHALL implement that bound
   iteratively, and a key of fixed shape SHALL implement it through its
   `FixedShape` serde path (FR-259).
5. **Exploration limits.** Simulation SHALL keep its exploration limits
   (FR-101), which bound states, transitions and path length.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-262-AC-1 | On a thread with a 512 KiB stack, the recursive function `function count using v(n: Int[0, 1000000]): Integer pure decreases(n) { if n = 0 then 0 else count(n - 1) + 1 }`, called with `100000` under a `work_units` budget sized for it, completes with `100000`. With `work_units` one below the run's measured spend, the same call returns `incomplete { limit_kind: work_units }` at the denied charge, and completes once `work_units` is raised to the measured spend. | Test (TC-734) |
| FR-262-AC-2 | On a thread with a 512 KiB stack, a recursive list value 100,000 long (`record List { head: Int[0, 9]; tail?: List; }`), built by a recursive function, evaluates; it keys as simulation state, and its state-key bytes equal the RFC 8785 text of its canonical JSON form; it compares equal to its clone, and it and its clone drop. A `ValueType` of 100,000 nested `Option`s around `Boolean` clones, compares equal to its clone, hashes equal to its clone, formats for debug and drops on the same thread. | Test (TC-735) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.7 and D-6.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md)
  defines exploration and its state keys.
- [FR-259](FR-259-encode-identities-and-read-json-through-quire-canonical-at-any-depth.md)
  defines the event API and the fixed-shape path.

## References

- QSpec FR-146 (execution fuel is `work_units`; call nesting depth is not a
  counter).
- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
