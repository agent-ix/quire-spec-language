---
id: FR-356
title: "Walk nested structures through one iterative walker toolkit"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-460
    type: depends_on
  - target: ix://agent-ix/quire-walk/FR-356
    type: depends_on
---
# FR-356: Walk nested structures through one iterative walker toolkit

## Description

QSL SHALL walk every nested structure iteratively, in arena order or on one
shared `no_std` walker toolkit, the crate `quire-walk` in its own repository,
`agent-ix/quire-walk`, so that no walk's native stack use grows
with the input's depth (ADR-030 D-1 items 2, 6 and 7; QSpec FR-460). The
toolkit's own behaviour (arena order, the walker, its Kani harnesses;
FR-356 AC-2 to AC-4, TC-898, TC-899) is specified and tested in that
repository. The qualified core (the S0 to S4 checker, the prove path and the certificate
checkers; ADR-029 CB-2) is iterative only. Outside the core, growing the
stack on demand is a justified exception through one wrapper.

## Use case

A specification author writes a 100,000-term sum, or a 100,000-deep `Option`
type, and compiles it under limits raised to fit (US-027). Every stage that
walks it, from the parser through the checker to the prove path, runs that
walk on the toolkit's heap stack, so the compile finishes or stops with a
limit outcome naming its setting, and never overflows the stack. An
assessor who qualifies the core reads one Kani-verified traversal instead of
a hand-written stack in each walk.

## Behavior

1. **Arena order.** Moved to `agent-ix/quire-walk` (FR-356 Behavior 1).
2. **The walker toolkit.** Moved to `agent-ix/quire-walk` (FR-356 Behavior 2).
3. **One shared leaf crate.** QSL SHALL depend on `quire-walk`, the
   `#![no_std]` shared leaf that depends only on `core` and `alloc`, as a git
   dependency at `branch = "main"` of `agent-ix/quire-walk` (ADR-011 §6.1
   layer W). It is in FB-05's shared-leaf class beside `quire-exact` and
   `quire-semantic-value`, and arch-lint's `SHARED_LEAVES` lists it, so QSL,
   IR, CG and RT each depend on it without depending on any `qsl-*` crate
   (ADR-011 FB-05). `quire-exact` stays a leaf with no dependency and keeps
   its hand-written iterative traits (ADR-030 D-4.7).
4. **Kani-verified.** Moved to `agent-ix/quire-walk` (FR-356 Behavior 4).
5. **Iterative only in the core.** No walk in the qualified core SHALL grow
   or switch the native stack, and no crate in the core SHALL depend on
   `stacker` or on `qsl-walk-grow`, directly or through a feature.
6. **One `maybe_grow` wrapper outside the core.** Outside the core, a walk
   whose conversion to the toolkit is awkward MAY recurse natively through
   the `maybe_grow` wrapper. The wrapper SHALL be the only item of the crate
   `qsl-walk-grow`, a std-only QSL crate outside the qualified core (ADR-011
   §6.1 layer WG) that depends only on `stacker`. It grows the stack on
   demand, and is a plain call of its closure under `cfg(kani)`. It has no
   `no_std` build, since no `no_std` crate may depend on it. It is not a
   shared leaf: only QSL's crates outside the core depend on it, and
   `quire-walk` stays `#![no_std]` with no features. No other code SHALL
   call `stacker`.
   Each call site of the wrapper SHALL have a test that drives a 100,000-deep
   recursion through it on a thread with a 512 KiB stack.
7. **Deep tests on every public core entry point.** Each public entry point
   of the qualified core SHALL have a test that runs a 100,000-deep input
   through it on a thread with a 512 KiB stack, under limits raised to fit
   the input, so leftover or hidden recursion fails at test time.
8. **Deep-input fuzz target.** QSL SHALL ship a fuzz
   target that generates deeply nested QSL sources (nested brackets, sums,
   `else if` chains, nested `let`s and nested `Option` types) and drives
   each through the S1 parser and the S3 checker.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-356-AC-1 | arch-lint's `SHARED_LEAVES` holds `quire-walk`, and its direction check admits an IR, RT or CG edge to `quire-walk`, sourced from `agent-ix/quire-walk`, and refuses one to any `qsl-*` crate other than CG's normal edge to `qsl-replay` (FB-05). The crate's own `no_std` build, features and dependency tree are FR-356-AC-1 of `agent-ix/quire-walk`. | Test (`tools/arch-lint`: `tc_arch_lint_metadata_009`, `tc_arch_lint_direction_004`) |
| FR-356-AC-5 | Each public entry point of the qualified core (the S0 to S4 checker, the prove path and the certificate checkers) has a test that runs a 100,000-deep input through it on a thread with a 512 KiB stack, under limits raised to fit, and returns its result. No crate in the core depends on `stacker` or on `qsl-walk-grow`, either in its own `cargo tree` or in the resolved workspace build, and arch-lint's direction check refuses a core crate's edge to `qsl-walk-grow`. | Test (TC-902) |
| FR-356-AC-6 | `qsl-walk-grow`'s `maybe_grow` called on a 100,000-deep native recursion completes on a thread with a 512 KiB stack, and is a plain call of its closure under `cfg(kani)`. Each call site of `maybe_grow` outside the core has a test driving a 100,000-deep recursion through it on a thread with a 512 KiB stack, and no code outside `maybe_grow` calls `stacker`. | Test (TC-902) |
| FR-356-AC-7 | The deep-input fuzz target generates sources nested from 1 to 100,000 levels deep and drives each through the S1 parser and the S3 checker. A run of 10,000 inputs ends with every input returning a result or a stated limit outcome, and no panic, abort or stack overflow. | Test (TC-903) |

## Status

Partly implemented. `quire-walk` lives in `agent-ix/quire-walk` with its
tests and Kani harnesses (TC-898 and TC-899 there). `qsl-walk-grow` and the
deep-input fuzz target (TC-903) land here.
Until ADR-030 slice 1 deletes the S2 forms and S3 checker depth caps, a
fuzz input nested deeper than either cap ends in that cap's limit outcome,
so it does not yet reach the walks past it. AC-5 and the call-site half of
AC-6 (TC-902) wait on the walks that later slices move onto the toolkit.

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-1 items 2, 3, 6 and 7, and D-6.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer W and FB-05's shared-leaf class.
- [FR-256](FR-256-parse-source-at-any-nesting-depth.md) and
  [FR-258](FR-258-check-and-lower-expressions-at-any-depth.md): the parser
  and the checker whose walks run on the toolkit.

## References

- QSpec FR-460, the ecosystem depth rule.
- The qualified core: ADR-029 CB-2, Linear QSL-390.
- Linear QSL-381.
