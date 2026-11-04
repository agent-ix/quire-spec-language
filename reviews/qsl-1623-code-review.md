---
id: SR-20261104025139
title: "Code review — QSL-1623 PR #2267"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2267 head dd9a02b16ae32a37296d82f5e840af5a26243348; qsl-semantics/src/check/lowering/protocol.rs; qsl-package/src/emit/tests/protocol_reference.rs; qsl-package/src/emit/tests/protocol_graph.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: reviews
---

## Summary

Ticket: QSL-1623. Reviewed PR #2267 at head `dd9a02b16ae32a37296d82f5e840af5a26243348` against `origin/main`. The diff uses the checked role's selected package identity for static whole-model role literals; model-type roles continue to use object-node references. The new FR-383-AC-23 test checks two selected identities, empty replication groups, no model-node dependencies, key distinction from each other and `M::Cell`, and successful QSL I2 admission. The criterion already requires this wire form, so removing the old refusal assertions is justified.

The `codex exec review` command could not initialize because the in-process app-server hit a read-only filesystem error. I completed a manual Rust review of the changed code and the relevant caller, spec, and tests. No build gates were run as reviewer. The exact acceptance test had passed before review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ProtocolConstruct::WholeModelRole` still documents all static whole-model roles as an IR-874 gap, and `tracking()` still points to IR-874 even though the selected whole-model role is now emitted and admitted. If the remaining guard path is reached, the public diagnostic metadata misstates the unsupported case and points to a completed ticket. Update or remove this stale variant/documentation. | qsl-semantics/src/check/refusal.rs:820-824,896 |

## Verdict

CONDITIONAL — the implemented behavior and AC-23 test are sound, but the stale unsupported-construct documentation and tracking id should be corrected before merge.

## Scope and method

Examined the three changed files, FR-383-AC-23 and the FR-383 explanation of the previous IR limitation, `CheckedRole` construction and `model_selection` semantics, and the `WholeModelRole` refusal metadata. The old test asserted a refusal directly contradicted FR-383-AC-23 and the new passing end-to-end case, so its removal is warranted. No spec or workflow files changed. No applicable AssuranceProfile was found. `codex exec review --base origin/main` was attempted but could not initialize its app-server in this read-only environment; review findings are from manual inspection. Rust build gates were not run by the reviewer.

## Finding data

```yaml
clean: false
scope:
  - id: FR-383-AC-23
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "| FR-383-AC-23 | ADR-027 §7's `Fill` with `role w on M` emits a `role` node whose `model` argument is the text `identity` of the package `M` selects, with an empty `replication` Group and no `model` dependency, and QSL's I2 reader admits it (QSpec FR-465-AC-9); a clause with `role w on M` and `role u on N` emits two role nodes whose texts are the two selections' identities, with different keys, and a role on the whole model and `role v on M::Cell` have different keys. | Test |"
  - id: FR-383
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: context_only
    excerpt: "and a role on a whole model (IR-874: the reader admits only the reference form of the role's `model`, so the text form QSpec FR-465 specifies cannot be proved admitted)."
findings:
  - fnd: FND-001
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: FR-383-AC-23
    related_ids: [FR-383]
    path: qsl-semantics/src/check/refusal.rs
    lines: "820-824,896"
    excerpt: "| FR-383-AC-23 | ADR-027 §7's `Fill` with `role w on M` emits a `role` node whose `model` argument is the text `identity` of the package `M` selects, with an empty `replication` Group and no `model` dependency, and QSL's I2 reader admits it (QSpec FR-465-AC-9); a clause with `role w on M` and `role u on N` emits two role nodes whose texts are the two selections' identities, with different keys, and a role on the whole model and `role v on M::Cell` have different keys. | Test |"
    finding: "`ProtocolConstruct::WholeModelRole` still documents all static whole-model roles as an IR-874 gap, and `tracking()` still points to IR-874 even though the selected whole-model role is now emitted and admitted. If the remaining guard path is reached, the public diagnostic metadata misstates the unsupported case and points to a completed ticket. Update or remove this stale variant/documentation."
bindings:
  - {test_id: a_whole_model_role_is_emitted_and_admitted, ac_id: FR-383-AC-23, trace: correct}
```

## Dispositions

**Round 1, re-checked at `dd9a02b16ae32a37296d82f5e840af5a26243348`.** The fix removes the `WholeModelRole` variant, its `IR-874` tracking mapping, and the `unemitted()` path that produced this refusal. The obsolete diagnostic is no longer present in `qsl-semantics` or `qsl-package`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `dd9a02b16` removes the obsolete role refusal and tracking entry; whole-model roles are now handled by the text-identity lowering path. |

After excerpt:

```rust
pub enum ProtocolConstruct {
    /// A control of the `run` tree other than a `sequence`, by its keyword
    /// (FR-466): QSL-1126.
    Control(&'static str),
```
