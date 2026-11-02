---
id: SR-7838
title: "Code review — quire-spec-language QSL-852 strict reader parity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/tests/it/protocol_slots/acceptance.rs, qsl-package/src/checked_v2.rs, qsl-package/src/evaluation_view.rs, qsl-package/src/lib.rs; QSL-852"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-049", type: references }
---

## Summary

Reviewed the frozen QSL-852 diff at `6f336c755`, including the Rust implementation and TC-142 parity assertions. The projection changes are coherent with the selected reader graph and existing checked evaluator; the new test includes two FR-390 criterion tags that are not declared by this repository's local spec and cannot bind in its local matrix.

## Verdict

**CONDITIONAL** — the only finding is a test traceability defect; no implementation defect was found in the changed reader projection paths.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-142 includes FR-390-AC-11, which is not declared by this repository's local spec and cannot bind through the local matrix. | qsl-eval/tests/it/protocol_slots/acceptance.rs:2404 |
| FND-002 | medium | TC-142 includes FR-390-AC-15, which is not declared by this repository's local spec and cannot bind through the local matrix. | qsl-eval/tests/it/protocol_slots/acceptance.rs:2404 |

## Review scope

- `qsl-eval/src/value/expression/evaluate.rs`: examined reader `Local`, `Field`, and `Attribute` projections.
- `qsl-eval/src/value/expression/mod.rs`: examined dispatch of the three reader projection forms into the existing Machine.
- `qsl-eval/tests/it/protocol_slots/acceptance.rs`: examined TC-142 test setup, five selected compensation slots, exact result/location/loss comparisons, cumulative charge log and counters, missing-binder refusal, and trace tags.
- `qsl-package/src/checked_v2.rs`: examined direct-reference and Record/Object field projection selection from admitted identity rows.
- `qsl-package/src/evaluation_view.rs`: examined reader-node membership and optional plain-data projection behavior.
- `qsl-package/src/lib.rs`: examined public re-export of `ReaderProtocolField`.
- `FR-049-AC-9`: “A strictly read `/2` package evaluates its exact compensation guard, retry and recovery handles through the shared state evaluator. Crossed handles and artifact/authority substitutions refuse with their existing typed causes; an unpublished compiler emission returns `Refusal::UnpublishedArtifact`; missing inputs and exhausted work remain distinct; the version-1 API remains source-compatible.”
- `NFR-009-AC-4`: “Re-evaluation starts with fresh counters, leaves the borrowed artifact and input view unchanged, and returns the same result and usage when the immutable inputs and sufficient effective limits are equal for both version-exact entry points.”
- `TC-142`: examined its existing V2 compensation behavior and the added strict-reader parity assertions.
- `FR-390-AC-11` and `FR-390-AC-15`: searched local `spec/`; neither is declared there.

## Verification

`git diff --check origin/main...peter/qsl-852-ac2-ac10` passed. Aggregate cargo gates were left to the lead. `quire matrix --scope . --format json --strict` did not return within 90 seconds in this checkout; static binding for the changed test was checked against the local spec files directly.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | FR-390-AC-11 is declared locally at spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:693, and the test case assigned to it, TC-1073, scopes itself to FR-390-AC-9 through AC-11 in spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md. The trace is valid. |
| FND-002 | rejected | FR-390-AC-15 is declared locally at spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:697. The new test explicitly tags that criterion; absence from TC-1073's narrower AC-9 through AC-11 scope does not invalidate the direct criterion tag. |
