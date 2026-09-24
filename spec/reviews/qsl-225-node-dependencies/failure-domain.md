---
id: SR-617
title: "Failure-domain review of the QSL-225 node dependencies rule"
type: SpecReview
analysis: failure-domain
scope: "Uncommitted QSL-225 diff on 3c0eea23: FR-093 Node dependencies (its identity effect through the identity_projection, recursion-group members, the own-graph claim), AC-12, TC-416 step 6, and ADR-013 QC-27"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The checklist was run over the dependencies rule.

- **Topological robustness.** Recursion groups behave as stated. A group
  member is named by its wire `node_id`, so rule 1 and rule 3 reach it like
  any other node. Its list stays inside the group's cycle, which the
  members' shared `recursion_group` label already admits (G4 to G6, G7 to
  G9). No rule makes a node list itself outside a one-member group, which
  FR-092 says no QSL source forms.
- **Evaluation purity.** Each list is a function of the node's own wire
  members. It reads no other node, no lock evidence and no source position,
  so emission order and parallel emission cannot change it.
- **Extension points.** A new node kind gets its list from rules 1 and 3
  with no new rule, unless it carries references outside `reference` terms,
  as a frame does.

Three failure modes are unstated: two conforming writers give one package
two ids, a cross-package reference has no stated treatment, and the rebuild
check in TC-416 can be satisfied by the code it checks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-093 says `dependencies` "enter `package_id` through the `identity_projection`", but not what follows from that. FR-322 fixes the list only for application, nominal and frame nodes. For every other node it admits any list, and no v2 reader refuses one: IR at `c0ba691` joins only the nominal and frame lists, and does not join even the application list. So QSL, and a writer that follows `positive-all-families.json`, give one package two `identity_projection`s and two `package_id`s. A consumer that selects or compares packages by `package_id` sees them as different packages, and nothing reports why. This is the same writer-choice identity split that QC-24 records for in-group ordinals. Fix: in FR-093 Node dependencies, after the `identity_projection` sentence, add: "FR-322 fixes the list only for application, nominal and frame nodes, and no v2 reader refuses another list for any other node, so until QSpec adopts rules 1 and 3 (ADR-013 QC-27) two writers can give one package two `package_id`s." In QC-27, before "QSL asks QSpec to correct those three lists", add: "Until then, `package_id` depends on the writer's choice for these nodes." | FR-093 Node dependencies, paragraph 1; ADR-013 QC-27, QC-24; QSpec FR-322 lines 53-56, 135-138, AC-14; `quire-contract-ir` `c0ba691` `v2/operations.rs` `validate_application_keys`, `v2/identity.rs` |
| FND-002 | low | Rule 1 and the sentence "Each dependency names a node of the package's own graph" do not say what a body reference to a dependency package's node adds. FR-322 carries that reference as a `dependency_reference` `PackageNodeKey{package, node}` (ADR-013 T-3, QC-10). The wire `dependencies` list holds only this graph's node ids, so it cannot hold one. FR-322's application rule ("exactly the unique ... reference targets") also does not say whether such a target counts. A `Call` to an imported function would reach this case, and the emitter has no stated rule for it. Fix: after "Each dependency names a node of the package's own graph", add: "A `dependency_reference` (FR-322, ADR-013 QC-10) names a node of a dependency package; it adds no entry, and a reader reaches it through `dependency_selections`." Add the same point to QC-27's request, so QSpec states whether an application's join counts it. | FR-093 Node dependencies, rule 1 and the type-annotation paragraph; ADR-013 T-3, QC-10, QC-27; QSpec FR-322 lines 147-159 |
| FND-003 | low | TC-416 step 6 checks each emitted list against "the list rules 1 to 5 rebuild from the wire node". If the test calls the emission's own dependency function to rebuild, the equality holds for any rule, including a wrong one. Then only the nine pinned lists guard AC-12. IR does not catch a wrong list either (FND-001). Fix: in TC-416 step 6, write "rebuild its `dependencies` with a test-side walker over the emitted JSON that calls no `qsl-package` function". | TC-416 step 6; FR-093-AC-12, FR-093-CON-2 |

## Resolution

Resolved in the same change: every finding is fixed as its recommended fix states.
