---
id: SR-616
title: "Integrity review of the QSL-225 node dependencies rule and the v2 fixture comparison"
type: SpecReview
analysis: integrity
scope: "Uncommitted QSL-225 diff on 3c0eea23: FR-093 Node dependencies (rules 1 to 5, the type-annotation paragraph, the recursion-group paragraph, the FR-322 correspondence paragraph, the per-node-kind table), Comparison with QSpec's v2 positive fixtures, AC-12, AC-13, and ADR-013 QC-27"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The five rules are consistent with one another and with the table. The
checks below were made by hand and with the scratch script of SR-615.

- **Complete for QSL's node kinds.** Each emitted node kind gets its list
  from the rules. Builtin scalars, literals, parameters and model nodes have
  bodies with no `reference`, so their lists are empty. Composite types,
  record and tuple values, functions and compound units get their lists
  from rule 1. Bounded domains get theirs from rule 3, applications from
  rules 1 and 2, and enum values, units and dimensions from rule 4.
- **No double counting.** Where a rule names a node that is also the
  node's `semantic_type`, as for an enum value, a unit or a bounded domain,
  "unique" removes the duplicate. The type-annotation paragraph says this.
- **Agrees with FR-322 where FR-322 speaks.** Rule 4 matches the nominal
  joins in FR-322's `nominal_identity_preimage` bullet (lines 186-190) and
  IR's `identity.rs` checks (lines 525, 572-575, 605-608). Rule 5 matches
  FR-340 and IR's `frame_defect`. Rule 1 on the wire and the
  `group_reference` preimage spelling match FR-322's
  `application_node_preimage` (lines 126-132).
- **Closure and cycles.** A reader already follows `semantic_type` and body
  targets. Adding a `bounded_domain`'s base or a group member to
  `dependencies` adds no edge that IR's cycle check does not already hold,
  so it cannot make a package cyclic. A group member reached this way stays
  inside its own `recursion_group` label.
- **Fixture claims.** Both are exact: four fixtures follow the rules at
  every node, and `positive-all-families.json` departs at `cccc…`, `eeee…`
  and `7070…` and nowhere else.

The integrity gaps are one equality claim that is narrower than the rule it
names, and one comparison member whose order the AC drops.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "Rules 1 and 2 are FR-322's rule for an application node" is false as written. FR-322's rule takes "member declarations of its body", and FR-322's `member` can be a `field`, `position`, `element`, `relationship_end`, `operation` or `type_argument` member, each "naming its declaring node". Rule 2's parenthetical lists only `field`, `operation` and `type_argument`, and "The five rules give the whole list" makes that list closed. QSL's application rows use only those three today, so no emitted list differs. But a row that adds a `position` or `relationship_end` member would follow rule 2 as written and leave out a dependency that FR-322 requires, which is `invalid_semantic_graph`. Fix: change rule 2 to "each declaration an application's `operation.member` names (every FR-322 member kind that names a declaring node; QSL's rows use `field`, `operation` and `type_argument`);". | FR-093 Node dependencies rule 2 and the FR-322 correspondence paragraph; QSpec FR-322 lines 87-89, 135-138 |
| FND-002 | low | AC-13 and the Comparison section disagree on laws. The section compares "law roles in order", but AC-13 says only "law roles". A test written to AC-13 can pass with the roles reordered, and FR-322 fixes `laws` as an ordered list whose roles are the catalog entry's roles. Fix: in AC-13, change "law roles" to "law roles in order". | FR-093-AC-13; FR-093 Comparison with QSpec's v2 positive fixtures; QSpec FR-322 lines 74-75 |

## Resolution

Resolved in the same change: every finding is fixed as its recommended fix states.
