---
id: Plan-010
title: "Accepted Producer interface pin"
type: Plan
status: complete
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: references
---
# Implementation Plan: Accepted Producer interface pin

## Requirements Summary

### Functional Requirements

- [x] **FR-036-AC-1/3/4/6**: The composed compiler consumes the accepted
  Producer 1.2 model and correspondence contract without inventing runtime
  assessment facts or weakening any refusal.

## Dependency Graph

- `filament-core-data FR-117 + FR-127 + FR-129 -> Task-038`
  Reason: the native consumer can select a producer revision only after the
  Producer 1.2 bundle, correspondence and refusal contracts are accepted.
- `Task-038 -> quire-specification FS02 closeout`
  Reason: FS02 requires the native consumer to compile and exercise the exact
  accepted producer, rather than a pull-request checkpoint.

### The seam

`Cargo.toml` and `Cargo.lock` select the immutable producer revision. The
existing public adapter in `src/linking/composed/producer.rs` and the eight real
tests in `tests/producer_correspondence.rs` exercise that dependency directly;
no new model reader, copied producer type or runtime assessment path is needed.

## Test Plan

### Integration tests

- [x] **IT-009**: Run all eight `producer_correspondence` tests against the
  accepted revision, including exact cross-family identity, independent
  producer-axis refusals, relationship direction/type/export-kind controls and
  fail-closed related occurrences.
- [x] **TC-114 / TC-115 regression**: Run the complete no-default-features suite
  so the accepted pin cannot weaken existing composed dependency, role,
  capability or static/assessment separation behavior.

### Static verification

- [x] Confirm `cargo metadata` resolves `agent-ix-baseline-producer` at exactly
  `404288282402d60de007295ccbafa960532b955e` and that the selected crate remains
  `publish = false` and `AGPL-3.0-only`.
- [x] Pass formatting, strict Clippy, clean build, fixture self-test and fixture
  model-byte audit with the isolated `target-codex-backends` target tree.

## Remaining Work

### Track A: Critical path (serial)

- **A1 = Task-038** Accepted Producer pin — Easy; exit: manifest, lock,
  provenance and all local behavior select and qualify the accepted FCD merge.
- **Gate = Task-038** PR readiness — measures exact selection and unchanged
  behavior; pass: every scoped test and static gate passes, followed by PASS
  `/rust-review` and `/gap-analysis` artifacts.

## Parallel Execution Summary

`FCD accepted merge -> Task-038 exact pin and evidence -> FS02 closeout`

The one mutable consumer boundary makes this a deliberately serial one-task
plan; no parallel implementation lane exists.

## Task File Mapping

| Task | Track | Owns (references) | Verified by (verifies) | Status |
| --- | --- | --- | --- | --- |
| Task-038 | A | FR-036 | TC-114, TC-115, IT-009 | done |

## Coordination Rules

- Change only the consumer selection and its truthful specification,
  dependency provenance and review evidence; do not alter Producer semantics.
- Use Rust and local commands only, with `publish = false` and
  `target-codex-backends`; do not edit `resources/native-v1/` or dispatch hosted
  CI.
- Update issue #93 and the owning quire-specification FS02 trackers after the
  accepted consumer PR merges.
