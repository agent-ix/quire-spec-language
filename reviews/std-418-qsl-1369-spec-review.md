---
id: SR-16299
title: "Spec review of quire-spec-language PR 1369: STD-418"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9412adb0; spec/functional/FR-056-admit-domain-package-model-declarations.md; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-1231-native-bytes-jsonobject-and-duration-in-the-unit-language.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1231
    type: references
---

# SR-16299: Spec review of quire-spec-language PR 1369 (STD-418)

## Summary

Ticket: STD-418. Head 9412adb0 (PR 1369, contains 1357 head 73558f64). Own delta: commit 29e80a4d (FR-056, FR-070, FR-091, FR-093, FR-106, new TC-1231). The canonical-payload rules were derived by hand against RFC 8785: `{"a":-0}` and `{"a":9007199254740993}` (shortest binary64 is 9007199254740992) and `{"a":1e999}` are noncanonical, `{"a":9007199254740992}` is canonical, members sort by UTF-16 code units, Bytes are lowercase two-digit hex. The `length`/`concat` evaluation vectors (E7 to E13) and the lowering (`quire.op.bytes.*`, `quire.op.json_object.*`, `value_kind` bytes / json_object) match the QSpec stack names. Duration as exactly `Quantity<s>` with the three refusals follows QSpec FR-322. No deferral or narrowing language; the kernel half is quire-exact PR 82 (separate repo).

Also checked:

- FR-056-AC-46..49, FR-091-AC-56, FR-093-AC-38, FR-106-AC-61 and TC-1231 are unique across the other open PR heads. Gate exit 0, 20 warnings (same as 1357, no new). Merge-tree with main clean; conflicts with open PRs 1335, 1333, 1314, 706.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-056-AC-46 lists the JSON number `0` among Bytes payloads that 'refuse with the noncanonical Bytes cause'. TC-1231 vector P5 and its note (line 146: 'P5 and P21 refuse before the kernel') say the same input is `wrong-value-kind` at the wire with no kernel call, so the AC and the test disagree on the cause and on whether the kernel constructor is reached. Fix: remove the JSON number from AC-46's kernel list and give it to FR-056-AC-47's wire vectors. | spec/functional/FR-056-admit-domain-package-model-declarations.md:1031 | wrong-requirement |

## Verdict

**Mergeable after** FND-001 (medium) are fixed or dispositioned by the lead.
