---
id: SR-7839
title: "Gap analysis — QSL-852 strict reader parity trace"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-852 changed Rust files and TC-142; local FR-049-AC-9 and NFR-009-AC-4"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-049", type: references }
---

## Summary

Scoped the spec/test/code chain affected by QSL-852's strict-reader parity change. The local FR-049-AC-9 path has a traced TC-142 integration test; the new tags for FR-390-AC-11 and FR-390-AC-15 have no matching criterion in this repository's `spec/`, leaving the parity obligation unbound in the local trace model.

## Verdict

**CONDITIONAL** — two medium traceability gaps remain for the new external criterion tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The added TC-142 test names FR-390-AC-11, but local spec declares no such criterion, so the changed parity behavior has no local matrix binding for this obligation. | qsl-eval/tests/it/protocol_slots/acceptance.rs:2404 |
| FND-002 | medium | The added TC-142 test names FR-390-AC-15, but local spec declares no such criterion, so the changed parity behavior has no local matrix binding for this obligation. | qsl-eval/tests/it/protocol_slots/acceptance.rs:2404 |

## Coverage

- Reconciliation: `quire matrix --scope . --format json --strict` did not return within 90 seconds; scoped static tags were checked against local `spec/` directly. `quoin matrix --repo . --json` also did not return in the available run window.
- Plan completion: not assessed
- Criteria examined: FR-049-AC-9; NFR-009-AC-4; TC-142; claimed FR-390-AC-11 and FR-390-AC-15 (no local declarations found)
- Untraced behaviors / stubs: 2 scoped traceability gaps; no changed-path stub found
- Semantic review: skipped (not requested)

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | rejected | FR-390-AC-11 is declared locally at spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:693, and TC-1073's scope explicitly includes AC-9 through AC-11 in spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md. The trace is valid. |
| FND-002 | rejected | FR-390-AC-15 is declared locally at spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:697. The new test directly tags that criterion; the trace is valid. |

> Note (FND-006, SR-2122001): this review was authored by the implementing lane against frozen head 6f336c755. It is not independent evidence. The independent review of #2122 is SR-2122001 (Linear QSL-852 comment 41842143).
