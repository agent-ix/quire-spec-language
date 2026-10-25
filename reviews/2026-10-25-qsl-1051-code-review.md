---
id: SR-60012
title: "Code review of QSL-1051 temporal population domains"
type: SpecReview
analysis: code-review
scope: "qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/state_clause.rs, qsl-semantics/src/check/temporal_clause.rs, qsl-semantics/tests/it/model_subject_bound.rs; FR-336-AC-7 and TC-846"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: references
---

## Summary

The scoped Rust review found the temporal-record population query includes either subtype direction, retains declaration-key ordering, and leaves existing state-clause coverage unchanged. Focused tests and clippy pass; one full-crate integration stress test could not complete its `/usr/bin/time -l` measurement in the sandbox.

## Verdict

**CONDITIONAL** — the implementation and focused gates pass; the full all-features integration suite needs the unsandboxed rerun for its system-time measurement.

## Assurance Context

No applicable `AssuranceProfile` exists under `spec/` for these files.

## Coverage

- `AdmittedModel::populations_overlapping` uses both conformance directions and `any` per declaration, so a population enters once even when multiple declared member types overlap.
- The temporal path sorts matching populations by `DeclarationKey`, filters authored bounds as before, and supplies their canonical domain keys to the production temporal requirement record.
- `populations_of` and state-clause resolution retain their prior coverage semantics.
- The new traced integration test exercises a real admitted package containing a base population, a subtype population, a population with both matching member types, and an unrelated population. It asserts exact order, uniqueness, record contents, and unrelated-domain absence.

## Checks

- `cargo fmt --all -- --check`: pass.
- `cargo clippy -p qsl-semantics --all-targets --all-features -- -D warnings`: pass.
- `cargo test -p qsl-semantics --features test-support --test it model_subject_bound::`: 4 passed.
- `cargo test -p qsl-semantics --all-features`: 773 unit tests passed, 1 ignored; integration suite 1,235 passed and 1 failed because `/usr/bin/time -l` queried `sysctl kern.clockrate` and the sandbox returned `Operation not permitted`. The child measurement test itself completed successfully.
- Temporary scoped `quire matrix` over FR-336, TC-846 and the touched test file: FR-336-AC-7 is `tagged`. Other FR-336 rows were outside this change and not included in the minimal source scope.
- `quoin gap-analysis` is not an installed command in the resolved Quoin release. The full-root `quire matrix` has no document filter and is not treated as a touched-file matrix.
- No workflow or CI files changed. `deny.toml` explicitly configures only `check bans`, so the documented `make cargo-deny-bans` is the applicable dependency-policy gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The full all-features integration gate needs an unsandboxed rerun because this host denies `sysctl kern.clockrate` to `/usr/bin/time -l`; no code failure was observed in that test's child run. | qsl-semantics/tests/it/model_identity_full_size.rs:179 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred: unsandboxed full-crate rerun | The prior operator instruction assigns this host-only measurement rerun to Peter. |
