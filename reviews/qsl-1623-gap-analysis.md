---
id: SR-20261104025140
title: "Scoped gap analysis — QSL-1623 FR-383-AC-23"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1623 / FR-383-AC-23; qsl-semantics/src/check/lowering/protocol.rs; qsl-package/src/emit/tests/protocol_reference.rs; qsl-package/src/emit/tests/protocol_graph.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: reviews
---

## Summary

The scoped AC-23 evidence is present and exercises the production path: the checked static whole-model role uses its selected model identity, the emitted wire carries the identity literal and empty replication Group without a model-node dependency, whole-model selections yield distinct keys from one another and `M::Cell`, and `read_v2` verifies the package.

## Verdict

CONDITIONAL — the scoped acceptance path is covered, but the computed repository matrix did not finish within the review session, so repository-wide criterion coverage and reverse-gap status remain unverified.

## Coverage

Plan completion: not assessed.

- FR-383-AC-23: covered by `a_whole_model_role_is_emitted_and_admitted`, tagged `FR-383-AC-23` and `TC-1021`; the test checks the specified values and invokes the real QSL I2 reader.
- The old refusal assertion was removed from the unwritten-construct list because AC-23 now requires the previously unsupported role to emit. The new positive test replaces that obsolete assertion for this construct.
- `quire matrix --strict --scope /home/peter/dev/worktrees/qsl-1623-code --format json` and `quoin matrix --repo /home/peter/dev/worktrees/qsl-1623-code --json` were attempted. They emitted module diagnostics but did not return a matrix during the review window. Repository-wide untagged criteria, reverse gaps, and stub status are therefore unavailable. This review is limited to the changed behavior and acceptance criterion.
- Optional semantic review was not requested and was skipped.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No gaps found in the scoped QSL-1623 acceptance check. | - |

## Method

Manual scoped comparison of FR-383-AC-23, its trace binding, the changed lowering path, and the end-to-end test. Full repository matrix verification was attempted but unavailable within the review window. No plan was provided.

## Finding data

```yaml
clean: true
scope:
  - id: FR-383-AC-23
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "| FR-383-AC-23 | ADR-027 §7's `Fill` with `role w on M` emits a `role` node whose `model` argument is the text `identity` of the package `M` selects, with an empty `replication` Group and no `model` dependency, and QSL's I2 reader admits it (QSpec FR-465-AC-9); a clause with `role w on M` and `role u on N` emits two role nodes whose texts are the two selections' identities, with different keys, and a role on the whole model and `role v on M::Cell` have different keys. | Test |"
findings: []
bindings:
  - {test_id: a_whole_model_role_is_emitted_and_admitted, ac_id: FR-383-AC-23, trace: correct}
```
