---
id: SR-202611020538
title: "Gap analysis — QSL-1438 direct RFC 8785 key writer"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-101-AC-2; FR-101-AC-11; FR-120-AC-4; qsl-eval/src/simulation/; qsl-analyze/tests/it/rc11_oracle.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-101, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-120, type: references }
---

## Summary

Ticket: QSL-1438. A gap analysis of PR #2069, scoped to the acceptance criteria that the changed code serves. The new `Canonicalizer` now produces every model and protocol state key, the observation memo identities, and the RA key names. The analysis checked whether tests bind those paths to the ACs. Plan completion was not assessed. It was planless and scoped to this PR, and the full `quoin matrix` was not run.

## Verdict

**Gaps found.**

- **FR-101-AC-2** (the state key is the JCS encoding of FR-181's form). For successful encodes, two tests cover the new path. TC-471 (FR-120-AC-4) pins one model-key envelope. The new rc11 test checks ra and tso protocol keys against `quire_canonical::to_vec`, but it has no trace tag.
- **FR-101-AC-11** (a key with no RFC 8785 encoding refuses with `KeyEncoding`). Its TC-453 tests (finite_simulation.rs:578, graph_core.rs:769) run on toy `TransitionSystem`s through `key::state_key`, the event-writer path. No test drives a refusal through `Canonicalizer`, `object_with`, or `MemoryModel::key_bytes`, which is the code this PR adds.
- **Untraced code.** No owning AC covers the `Canonicalizer`'s own refusal ordering, which its doc claims matches the writer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test drives FR-101-AC-11 refusals through the new Canonicalizer, `object_with` or the `key_bytes` path. The TC-453 refusal tests use toy systems on the event-writer path | qsl-eval/src/simulation/protocol.rs:1806-1818; qsl-eval/src/simulation/key.rs:143-152; qsl-eval/tests/it/finite_simulation.rs:578 |
| FND-002 | low | The new key-equivalence test has no `#[trace]` binding to FR-101-AC-2, so the matrix does not count it | qsl-analyze/tests/it/rc11_oracle.rs:507-511 |

## Coverage

- Bindings checked:
  - TC-453 at finite_simulation.rs:545 and :622 to FR-101-AC-2: correct, toy systems.
  - TC-453 at finite_simulation.rs:578 and graph_core.rs:769 to FR-101-AC-11: correct, but they do not reach the new path.
  - TC-471 at model_simulation.rs:813 to FR-120-AC-4: correct, and it reaches `Canonicalizer` through `ModelSystem::state_key`.
- The code-review findings on this PR are in SR-202611020537.

## Findings data

```yaml
clean: false
findings: [FND-001, FND-002]
```
