---
id: SR-20261104091645
title: "Code review — FR-204 digital cycle refusal identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-analyze/src/exact_probabilistic.rs, qsl-analyze/src/exact_probabilistic/settle.rs, qsl-analyze/tests/it/digital_clocks.rs; FR-204-AC-4; TC-639"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
---

## Summary

Reviewed the digital zero-delay cycle identity encoder, refusal settlement, and the timed and monitorless integration cases against FR-204-AC-4. The encoder covers all `ProbKey` members with typed tags and framed values, and the tests verify actual cycle refusals through FR-331 serialization.

## Verdict

**PASS** — no correctness, Rust API/style, or test-intent defects were found in this scoped review.

## Scope

- `qsl-analyze/src/exact_probabilistic.rs`: examined the public cycle identity API and exhaustive monitor, accumulator, and pending encodings.
- `qsl-analyze/src/exact_probabilistic/settle.rs`: examined the refusal-to-terminal mapping and failure handling.
- `qsl-analyze/tests/it/digital_clocks.rs`: examined the timed and monitorless cycle tests, wire assertions, and trace tags.
- `spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md`, FR-204 description (context only): “Strict constraints, real-valued delay distributions, zero-delay cycles and other form shapes SHALL settle `unsupported` with a cause that names them.”
- `spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md`, FR-204-AC-4 (examined): “A variant of `Retx` whose `send` has no `x >= 1 ms` guard and no reset settles `unsupported`, `ZeroDelayCycle`, naming the cycle's states. `probability >= 0.99 [always[0 ms, 4 ms] holds(not m.delivered)]` over `Retx` settles `unsupported`, `TimedFormShape`.”

No workflow files, dependencies, or wire schemas changed. `AGENTS.md` and `CLAUDE.md` were read; no applicable AssuranceProfile was found. Repository formatting passed. The focused `digital_clocks` suite passed 15/15, `cargo check -p qsl-analyze --tests` passed, and default/all-feature Clippy passed with `-D warnings`. `cargo deny check --disable-fetch` could not acquire the read-only advisory DB lock; no dependency changed.

## Findings

| ID      | Severity | Summary                          | Refs |
| ------- | -------- | -------------------------------- | ---- |
| FND-001 | low      | No findings (placeholder)        | -    |

## Coverage

The new tests bind `zero_delay_cycle_and_form_shape_are_unsupported` and `unbounded_digital_cycles_settle_with_monitorless_state_identity` to TC-639 and FR-204-AC-4. Both exercise the real digital builder, verify the named cycle states survive settlement and serialization, and check that the timed-form source span remains in the wire result. No optional semantic review was requested.
