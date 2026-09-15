---
id: SR-435
title: "Code and Rust review of initialized capture evaluation"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#105; Plan-009/Task-041; FR-049; NFR-009; TC-142; src/state/evaluation.rs; tests/compiled_protocol_v2.rs"
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

`/rust-review` examined the QSL #105 Rust delta at PR readiness. The shared
state evaluator discovers a capture initializer's external inputs, evaluates
the admitted initializer at its original anchor, caches the immutable value for
the request, and never accepts a caller-supplied capture value.

## Verdict

**PASS** — the two review findings were fixed before this artifact was
recorded; no actionable Rust, API-boundary, test-alignment, panic, unsafe,
conversion, resource, or stub finding remains.

## Resolved findings

| Finding | Resolution |
| --- | --- |
| Initializer lookup crossed wire indices into host indexing with unchecked casts | replaced both conversions with checked `usize::try_from` and the existing typed invariant refusal |
| The first capture test proved only one Boolean direction | added the opposite source value and verdict, direct caller substitution, exact missing-source, Full/Partial, foreign and cyclic controls |

## Rust review

- Production adds no public API, `unsafe`, panic, callback, parser, dynamic
  dispatch, lock, async path or blocking work.
- Capture resolution is confined to admitted `BinderKind::Capture`; lexical
  let/query binders retain their existing scope-owned local semantics.
- The admitted value graph remains the authority for local ownership, type,
  source order and acyclicity. Defensive owner/index failures remain explicit
  `AdmittedInvariant` refusals rather than guessed values.
- `expression_at` selects the initializer's authored anchor and restores the
  caller anchor. `bind_local` provides one immutable value per request and
  charges the existing retained-output dimension.
- Required-input discovery follows the same admitted initializer edge, so a
  missing external observation remains `MissingBinding`; the initialized
  capture itself remains surplus if offered by a caller.
- TC-142 executes both Full and Partial retry/recovery captures in both Boolean
  directions, replays exact value and usage, checks exact/one-short limits, and
  rejects malformed foreign/cyclic initializer graphs through the strict reader.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| Clippy, all targets, no default features, warnings denied, serial | pass after remediation |
| Full no-default-feature unit/integration/doc suite, serial | pass; 3 inherited private-packet tests ignored |
| `compiled_protocol_v2` and `composed_state_evaluation` suites | 19/19 and 20/20 pass |
| No-default-feature build | pass |
| Fixture audit self-test and model-byte audit | pass |
| Repository constraints | no hosted CI dispatch or workflow change; `resources/native-v1/` unchanged; package remains `publish = false` |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved code or Rust finding remains after remediation. | #105; Task-041; FR-049; NFR-009; TC-142 |
