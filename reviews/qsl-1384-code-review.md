---
id: SR-20261104074840
title: "Code and Rust review — QSL-1384 temporal provenance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit.rs; qsl-package/src/emit/tests/temporal_clause.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/temporal.rs; qsl-replay/src/spine/clause/tests/temporal.rs; FR-050-AC-5; FR-050-AC-6; TC-150"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-050", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/TC-150", type: reviews }
---

## Summary

Ticket: QSL-1384 (child QSL-1446). Combined Golden Path code review with the Rust lane, covering PR #2279 at head `71ef62b415b9369ac6821b091429f91e0a51f9a9` against `origin/main`. Strict package read-back, exact declaration lookup, trace binding refusals, result provenance and the changed temporal tests were examined.

## Verdict

**CONDITIONAL** — one medium finding: a successful authenticated result drops the exact source-map locus of the selected checked declaration.

## Scope examined

- `qsl-package/src/emit.rs:489-520, 6060-6095`
- `qsl-package/src/emit/tests/temporal_clause.rs:102-170`
- `qsl-replay/src/spine/clause.rs:900-920, 1049-1205`
- `qsl-replay/src/spine/clause/temporal.rs:393-400, 432-530, 536-565, 680-865`
- `qsl-replay/src/spine/clause/tests/temporal.rs:2643-2718, 2720-2743`
- `spec/functional/FR-050-publish-authenticated-temporal-artifacts.md` (FR-050-AC-5 and FR-050-AC-6)
- `spec/test-cases/TC-150-preserve-authenticated-temporal-selections-on-checked-route.md` (checked-successor procedure and expected results)
- `spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md` (FR-522-AC-6)

## Rust review

The checked-package bytes are admitted with the bounded strict reader before temporal evaluation. The temporal emitter retains `(NodeKey, occurrence) -> CheckedNodeId`, and the evaluator uses that exact mapping to resolve the admitted subject. The shared authenticator checks profile, clock name and typed configuration before visiting positions; mismatches return typed `invalid_runtime_input/invalid-value` refusals without authenticated result data. No new panic, unsafe, async, dependency, or unchecked wire-conversion issue was found.

The gap is in what the success report preserves: `TemporalSubjectView` resolves the checked source-map entry, but `AuthenticatedTemporalSelection` carries only declaration node identity, profile, clock name and clock. The new test checks the profile and clock values and only that `declaration.path` is nonempty; it cannot verify the exact source locus joined to the admitted declaration.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | Authenticated temporal results omit the exact source-map locus of the joined declaration. | qsl-replay/src/spine/clause.rs:911-919; FR-050-AC-6 |

## Verification

- `cargo fmt --check` — pass.
- `cargo clippy --locked -p qsl-package -p qsl-replay --all-targets --all-features -- -D warnings` — pass.
- `CARGO_TARGET_DIR=/tmp/qsl-1384-review/target cargo test --locked -p qsl-package --lib emitted_temporal` — pass; 2 tests.
- `QSPEC_DIR=/home/peter/dev/quire-specification cargo test --locked -p qsl-replay --lib temporal` — could not acquire `/home/peter/.cargo-target/debug/.cargo-build-lock` (read-only filesystem).
- `quoin matrix --repo . --json` and `quire matrix --scope . --format json` — each stalled without returning matrix data and was interrupted; see separate gap-analysis artifact for scoped manual coverage and limitation.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The authenticated result now retains the checked source regions, and the regression test asserts the source authority, identity and exact declaration-name byte range. |

After excerpt:

```rust
pub source_regions: Vec<quire_contract_model::CheckedSourceRegion>,
...
source_regions: subject.source_span().regions.clone(),
```
