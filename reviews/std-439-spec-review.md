---
id: SR-16264
title: "spec-review of FR-147 dependency protocol clause as a refinement's abstract side (quire-spec-language PR 1378)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@14441e2c; spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md; spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md; spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md; spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-148
    type: references
---

# Spec review of FR-147 dependency protocol abstract side (quire-spec-language PR 1378)

## Summary

Ticket: STD-439. One spec-review pass over `git diff origin/main...14441e2c`.
The oracles were QSpec FR-375 "Selection", "Protocol subject",
"Checked-package node" and FR-375-AC-11 at QSpec main d2fab1b0, and QSL
ADR-015 D-1 and D-5.

Examined, clean:

- **Resolution** among the protocol clauses of the abstract subject package,
  looked up among protocol clauses only, matches FR-375 "Protocol subject".
  `abstract Spec::Thrice` and `abstract Spec::Counter` refusing `missing-name`
  at the second segment match FR-375-AC-11.
- **The selectable-clause check** for dependency and local clauses matches
  FR-375 ("in every case"). That lead default is QSpec's own text.
- **The dependency's `NodeKey`** is found by lookup in its `CheckedGraph`, never
  minted (ADR-015 D-5, ADR-013 O-04), and the node rows and binder records
  are typed from that graph.
- **`AbstractSide::Protocol { alias, clause, dependency: Option<PackageId> }`**
  is added to FR-135's enum, which matches FR-147 Outputs.
- **FR-148** builds the abstract `ProtocolSystem` from the package that holds
  the clause.
- **FR-147-AC-15 and AC-16** are testable, and TC-552 steps 15 and 16 match
  them. `CompileRefusal::Dependency` is ADR-015 D-1's.
- The "Remaining work" note is deleted, and nothing in scope is deferred.
  "not yet built" at lines 245 and 262 is the pre-existing unbuilt-feature
  rule, not in the diff.

Gates: `make check-spec-validation` at 14441e2c exits 0, with no warning on a
touched file. The PR merges cleanly with origin/main and with #1332, #1310,
#1311, #1343, #1346, #1327 and #1356. origin/main is not an ancestor of the
head. No other open PR edits FR-135, FR-147, FR-148 or TC-552, and
FR-147-AC-15 and AC-16 are unique.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The emitted refinement node for a dependency's clause is not specified. QSpec FR-375 "Checked-package node" gives a protocol subject a body member `protocol`, the selected clause's `PackageNodeKey`, and makes that clause's node one of the node's `dependencies`. For a dependency's clause, that key names the dependency package. FR-147 states the `CheckedRefinement` and the obligation identity for the dependency case, but not the node's `protocol` key or `dependencies`. AC-15 compares obligation identities only, not the node or node id against the local `Twice`. Fix: state that S4 writes `protocol` as the dependency clause's `PackageNodeKey` (the dependency's `package_id` and the looked-up `NodeKey`) and lists it in `dependencies`. Extend AC-15 so the dependency and local forms give different refinement node ids. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:70-78,321 | missing-requirement |
| FND-002 | medium | The new rule resolves `<protocol>` in "the dependency's package when `<A>` names a dependency's model". So a protocol clause that the declaring unit declares over a dependency's model is not selectable and refuses `missing_declaration`/`missing-name`. That consequence of FR-375 "Protocol subject" has no vector. An implementation that also searched the unit's own clauses would pass every AC. Fix: add to FR-147-AC-15 (and TC-552 step 15) a local `protocol LocalTwice` acting on the dependency's `Spec`, with `abstract Spec::LocalTwice` refusing `missing-name` at `LocalTwice`. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:87-96,321 | missing-requirement |

## Verdict

Not mergeable until FND-001 and FND-002 are fixed. Resolution, lookup, the
selectable-clause check and the `CheckedRefinement` shape follow QSpec FR-375
and ADR-015. The emitted node and one resolution boundary need stating and
testing.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (3c3a3ba4): S4 writes `protocol` as the dependency clause's `PackageNodeKey`, the dependency's `package_id` with the clause's node id. The refinement node id then differs from the local form, and FR-147-AC-15 and TC-552 step 15 test both. The PR adds no `dependencies` entry for a dependency's clause, and that deviation is correct. QSpec FR-322 says that a node's `dependencies` names nodes of its own graph only, and that a `dependency_reference` target is never listed there. FR-375's "The selected protocol clause's node is also one of the node's `dependencies`" therefore applies only to a clause of the declaring package. ADR-015 D-5 governs only the `NodeKey` lookup, not this point; the FR-147 bullet cites FR-322 for it, which is right. |
| FND-002 | fixed | Round 1: FR-147 states that a protocol the declaring unit declares over a dependency's model belongs to the declaring package and refuses `missing-name`. FR-147-AC-15 and TC-552 step 15 add `abstract Spec::LocalTwice`, refusing at `LocalTwice`. |
