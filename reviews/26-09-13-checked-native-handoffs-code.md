---
id: SR-420
title: "Code and Rust review of checked native owner handoffs"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#90; FR-051; TC-139; Cargo.toml; schemas/checked-*-v1.schema.json; src/protocol_artifact/checked_*; src/protocol_artifact/temporal_subject.rs; tests/compiled_protocol_v2.rs; tests/contract_model_architecture.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-051
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-139
    type: reviews
---

## Summary

The Rust review covered the complete FR-051 owner implementation and the QSL
side of the cycle-free Contract Model pin. The final code uses typed admitted
inputs, closed serialized records, independently derived canonical bytes,
constructor-private validated views, explicit static-only vocabulary, bounded
iterative graph walks, and a non-retaining strict-reader census.

The promotion rereview merged QSL `main` at `93c411b`, replaced the provisional
Contract Model candidate with the promoted merge revision
`53cc03c639e2e26528132d34d96dc56449df78e8`, and replayed the complete gate on
that exact dependency graph. The merge introduced no conflict or behavioral
delta in the reviewed owner modules.

## Verdict

**PASS** — the allocation accounting, reachability, convergence, and API
documentation findings discovered during review were repaired; no actionable
Rust, code/test-alignment, integrity, or spec-faithfulness finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings remain after remediation. | - |

## Rust review

- The public boundary has one `thiserror` envelope with stable codes and paths,
  documented APIs, no public constructor for validated authority, and no
  `unsafe`, callback, trait-object, parser, evaluator, Boolean coercion, trust
  flag, Contract-IR wire type, TL value, or runtime-result vocabulary.
- All untrusted byte, string, collection, field, expression-depth, and output
  work is owner-clamped. JSON depth and raw string size are scanned before
  decoding; the Serde census retains no document tree; graph child vectors use
  checked population limits and fallible reservation; traversals and canonical
  measurement convergence have explicit ceilings.
- Predicate admission proves exact Boolean typing and accepted ownership. A
  temporal `holds` leaf must be reachable from its checked root. Protocol
  Boolean leaves are selected only from the reachable control graph plus the
  declaration's activation, compensation, and finish guards.
- Cargo metadata proves the production alias resolves to the reviewed
  `quire-contract-model` revision and that the historical compatibility
  package is unreachable from the normal dependency graph.
- No CI workflow, lint level, coverage threshold, or warning policy changed.
  No applicable `AssuranceProfile`, `deny.toml`, repository Rust override, or
  async/concurrent path exists in this scope.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| Clippy, all targets, no default features, `-D warnings` | pass |
| Full no-default-feature unit/integration/doc suite | pass; 3 named inherited private-packet tests ignored |
| Clean no-default-feature build against Contract Model `53cc03c` | pass |
| `fixture-audit self-test` | pass; 6 negative controls |
| `fixture-audit model-bytes tests/fixtures` | pass; 5 exact producer checkpoints |
| TC-139 owner, adverse, reachability, resource, schema and architecture tests | pass |

The final full-suite replay after review remediation is the merge gate recorded
with this change. Hosted CI was not dispatched.
