---
id: SR-202610310153
title: "Code review — checked equality emission rows"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit.rs, qsl-package/src/emit/tests/converted_equality.rs, AGENTS.md, QSpec FR-322-AC-288"
review_set: subset
---

## Summary

Reviewed PR #2041's typed equality witnesses, producer-data accessor, and enum member rows against its emitter paths and focused tests. The joins are exact in the implementation, but the tests do not establish declaration-bearing Plan types or distinct source/lowered enum node mappings, and one new test uses a tracker ticket as a trace artifact.

## Verdict

**CONDITIONAL** — the implementation's joins appear exact, but the acceptance-critical declaration and enum identity cases lack discriminating tests and the new trace tag violates repository conventions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Plan ValueType regression covers only a primitive type, not the declaration-bearing type required by the ticket | qsl-package/src/emit/tests/converted_equality.rs:707-711 |
| FND-002 | medium | Enum join test does not distinguish exact source and lowered declaration ID mappings | qsl-package/src/emit/tests/converted_equality.rs:792-865 |
| FND-003 | low | New enum test tags the tracker ticket QSL-1445 as a trace artifact | qsl-package/src/emit/tests/converted_equality.rs:786-789 |

## Review notes

- `EqualityFactWitnessRow::value_type()` and `EqualityProducerEmissionRow::producer_data()` are read-only. `equality_fact_witness_rows` joins checker-held types by exact operand position, role, and structural key and refuses missing or duplicate matches. Enum declaration and member IDs are looked up by their retained structural keys, with missing identities refusing emission.
- Focused gates passed: `cargo fmt --all -- --check`; `git diff origin/main...HEAD --check`; `CARGO_TARGET_DIR=/private/tmp/qsl-review-target cargo test --locked -p qsl-package --lib converted_equality` (23 passed); `CARGO_TARGET_DIR=/private/tmp/qsl-review-target cargo clippy --locked -p qsl-package --lib --tests -- -D warnings`; `cargo deny check bans`.
- No workflow files changed. Full-root Quire/Quoin matrix attempts stalled without output; the installed Quoin has no `gap-analysis` command. The manual acceptance mapping is recorded separately in `qsl-1445-gap-analysis.md`.
