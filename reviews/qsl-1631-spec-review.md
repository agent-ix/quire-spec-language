---
id: SR-2285003
title: "Spec review of PR #2285: FR-092 source enum identity rows and FR-251-AC-42 (QSL-1631)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@592acce0e; spec/functional/FR-092-key-type-parameter-and-declared-nodes.md; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md; ticket QSL-1631"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: references
---

# Spec review of PR #2285 (QSL-1631)

## Summary

Ticket: QSL-1631. Reviewed the new FR-092 "Source enum identity rows" section, FR-092-AC-19 and the
FR-251 retained-row list and FR-251-AC-42 edits. Checked against QSpec FR-141 (enum declaration and
member identity) and the locked IR row types and IR FR-354/FR-357 (the consumers' statement of what
the rows carry). `quire validate` on both files: only module duplicate-archetype warnings.

The section's field list matches the IR row types exactly. "Collection rows need no producer" is
right: Exact `ValueType::Collection` is a structural `CollectionType` (kind, element, bound) with no
`NodeKey` or declaration identity, the locked IR has no collection row type, and FR-141 gives
declaration identity only to enums.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-092 "Source enum identity rows" | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:66-79 | examined |
| FR-092-AC-19 | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:180 | examined |
| FR-251 retained-row list | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:119-125 | examined |
| FR-251-AC-42 | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:504 | examined |
| QSpec FR-141 enum identity | quire-specification FR-141 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-092-AC-19's verification cell is bare `Test` while its siblings (AC-16..18) bind a test case (`Test (TC-1021)`), so the AC has no TC and the test traces only the AC id. Bind a TC (extend TC-1021 or add one) or confirm the repo accepts a TC-less AC here. | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:180 |
| FND-002 | low | The section does not state the order of the emitted rows; the code emits them ascending by `NodeKey` (members not in declaration order). Order is deterministic, but a consumer cannot rely on it without a stated rule. State "ascending by node key" (or that order carries no meaning). | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:66-79 |
| FND-003 | low | IR FR-357 (cross-repo, IR-owned) still says QSL-1631 supplies "collection and union witnesses"; this PR's FR-092 states a collection has no row and union is blocked on IR-1323. The two specs now disagree; an IR ticket should drop the collection witness and point union at IR-1323. No change in this PR. | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:77-79 |

## Verdict

Mergeable. The requirement is EARS-shaped ("SHALL carry"), atomic, and matches both the locked IR
row contract and QSpec FR-141's identities (declaration id is the `scalar_type`/`enum` node id,
member id is the `value`/`enum_value` node over that declaration and case). The model-variant
exclusion matches FR-141's separate `model`/`variant_type` path. FR-251's edit is a list extension
and AC-42 keeps its existing structure. Findings are wording/traceability only.
