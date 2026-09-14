---
id: SR-424
title: "Code and Rust review of the native temporal owner boundary"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#95; FR-052; TC-140; schemas/native-temporal-*-v1.schema.json; src/protocol_artifact/native_temporal; src/protocol_artifact/temporal_subject.rs; tests/native_temporal_owner.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-140
    type: reviews
---

## Summary

The combined Golden Path code review and idiomatic Rust review covers the full
FR-052 implementation. QSL publishes immutable request and result schemas,
canonical domain-separated documents, bounded strict readers, a
constructor-private request authority sharing the exact FR-051 admitted
package, formula-wide evaluation through the existing native evaluator, typed
non-values and direct immutable correction lineage.

The public API contains no Contract-IR or TL vocabulary and accepts no parser,
evaluator callback, plug-in, network path or trust override. Callers provide
only native observations and authority inputs; result truth, settlement and
support are derived by QSL and strict result reading re-evaluates them.

## Verdict

**PASS** — every finding discovered by the code and Rust review was repaired,
the complete repository gate passes, and no actionable scoped finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: strict request admission initially cloned the entire admitted package again. The final representation performs one bounded clone when the FR-051 subject is admitted, retains it behind `Arc`, and lets each `ValidatedRequest` share that exact authority with an infallible `Arc` clone. | `temporal_subject.rs`; `native_temporal/request.rs` |
| FND-002 | medium | Resolved: producer conversion allocated trigger, capture, valuation and eviction collections before proving all coupled limits. Counts now use checked arithmetic before allocation, owned inputs move into wire values, and each retained vector uses fallible reservation. | `native_temporal/request.rs`; FR-052-AC-7 |
| FND-003 | low | Resolved: correction relation selection contained a logically unreachable production panic and test fixtures used unchecked numeric casts. Correction kinds now use an explicit helper and all boundary conversions are checked. | `native_temporal/result.rs`; `native_temporal_owner.rs` |
| FND-004 | medium | Resolved: FR-052-AC-8 was exercised but stranded as the sixth tag on one test symbol. A dedicated closed-public-surface control now checks forbidden seams and every downstream structural accessor; Quire reports all eight criteria backed. | FR-052-AC-8; TC-140 |
| FND-005 | high | Resolved after downstream replay: the first package-sharing remediation added a lifetime parameter to the public FR-051 subject type and broke the merged QProtocol owner API. The final `Arc` representation restores the non-generic public type, preserves constructor privacy, and compiles through the QProtocol/QCI dependency chain. | `temporal_subject.rs`; `quire-protocol::result::Input`; #95 |

## Rust review

- Production code contains no `unsafe`, panic macro, placeholder, unchecked
  wire conversion, async/blocking bridge, lock or shared mutable state.
- Reader-selected limits are fixed-width on the wire, checked on conversion,
  independently clamped to owner maxima, and applied to input/output bytes,
  JSON depth, string bytes, formula nodes/depth, positions, valuations,
  captures, support, history, evaluation steps, lineage and visited work.
- Canonical decoding rejects duplicate, unknown, missing, reordered and
  trailing material. Semantic identities are distinct from raw byte digests;
  position, request and result domains cannot substitute for one another.
- The tests use the real compiler, v2 package reader, FR-051 subject reader and
  native evaluator. There are no mocks, callback seams, tautological result
  inputs or duplicated evaluator implementation.
- Constructor privacy is compiled as negative Rust doctests for the FR-051
  subject, FR-052 request and FR-052 result authorities.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| Clippy, all targets, no default features, one job, `-D warnings` | pass |
| Full no-default-feature unit/integration/doc suite, one job and serial tests | pass; 3 inherited private-packet tests ignored |
| Independent clean no-default-feature build | pass |
| TC-140 owner, semantics, mutation, correction, resource, schema and public-surface controls | pass; 11/11 |
| `quire validate --scope . 'spec/**/*.md' 'plan/**/*.md' 'reviews/**/*.md' --summary` | pass; 714/714 documents grammar-clean |
| Immutable schema digest verification | pass |
| Downstream QProtocol exact locked gate against corrected QSL revision | pass; fmt, all-target clippy, unit/integration/doc tests |

Hosted CI was not dispatched.
