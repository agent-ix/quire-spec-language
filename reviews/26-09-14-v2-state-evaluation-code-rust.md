---
id: SR-432
title: "Code and Rust review of v2 state evaluation"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#101; FR-049; NFR-009; TC-142; src/state; tests/compiled_protocol_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-142
    type: reviews
---

## Summary

`/rust-review` examined the QSL #101 Rust delta at PR readiness. The public
`state::evaluate_v2` path reads the retained schema and optional executable
artifact from a strictly admitted v2 package, delegates to the same bounded
evaluator as v1, and admits compensation guard, retry and recovery binders only
when their exact declaration, subject, type, model, producer and anchor
authority agrees. A compiler-local package without a published artifact refuses
explicitly instead of approximating execution.

## Verdict

**PASS** — both review findings were fixed before this artifact was recorded;
no actionable Rust, API-boundary, test-alignment, panic, unsafe, conversion,
resource, or stub finding remains.

## Resolved findings

| Finding | Resolution |
| --- | --- |
| Compensation declaration indices crossed a wire-to-host boundary with an unchecked cast | replaced the cast with checked `usize::try_from` conversion and typed refusal |
| Ordinary declaration-root selection was repeated while generalizing the evaluator | extracted one shared `declaration_root_type` path used by the generic evaluator |

## Rust review

- Production adds no `unsafe`, panic, unchecked numeric conversion, callback,
  parser, dynamic dispatch, lock, async path or blocking work.
- `StatePackage` is a private adapter over v1 and v2 admitted packages. Both
  public entry points instantiate the same generic evaluator; there is no
  conversion, JSON round trip or second truth implementation.
- Compensation binders are selected by exact role and anchor. Subject identity
  is recovered from the owning compensation declaration and checked against the
  request; model and type authority are also exact.
- The optional v2 artifact is consumed directly. Its absence is the stable
  `UnpublishedArtifact` refusal and cannot fall through to incomplete or a
  guessed result.
- Existing expression and retained-population ceilings remain the single source
  of accounting. Fresh retry, one-short and zero-budget cases exercise the same
  deterministic boundary.
- TC-142 uses a strictly read real package and non-constant activation, retry
  and recovery expressions. Crossed owner, trigger, authority, type, anchor and
  population cases all refuse.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass after remediation |
| Clippy, all targets and all features, warnings denied, serial | pass after remediation |
| Full default-feature unit/integration/doc suite | pass; 3 inherited private-packet tests ignored |
| Full all-feature unit/integration/doc suite, serial | pass; 3 inherited private-packet tests ignored |
| `compiled_protocol_v2` and `composed_state_evaluation` suites | 18/18 and 20/20 pass |
| Targeted FR/NFR/TC/TestMatrix Quire validation | 4/4 grammar-clean |
| Repository constraints | no hosted CI dispatch or workflow change; `resources/native-v1/` unchanged; package remains `publish = false` |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved code or Rust finding remains after remediation. | #101; FR-049; NFR-009; TC-142 |
