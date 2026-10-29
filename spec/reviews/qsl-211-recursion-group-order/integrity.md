---
id: SR-610
title: "Integrity review of the QSL-211 recursion-group order and the FR-092 to FR-094 fixture corrections"
type: SpecReview
analysis: integrity
scope: "Commit 7be2d02a: FR-092 Recursion groups (definition, dependency order, the group order, groups that collide, Dependencies, Status), FR-093 (quantity.convert mode, IR-242 bullet, Status), FR-094 (quantity division, C3, P9, Status), FR-065-AC-7, ADR-013 QC-24 and QC-26, and the TC-413 to TC-419 edits"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-413
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-414
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-415
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-417
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-418
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-419
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The group order is well defined, deterministic and not circular. Each
refinement round refines the one before, because `h(r+1)` includes `h0` and
the targets' `hr`. The count of distinct values therefore never falls, and
the stopping rule is unambiguous. Equal full signatures imply equal targets
class by class, so they imply equal preimages, as step 5 claims. The full
partition refines the anonymous partition, so the members of one class sit
next to each other in the order. No step reads a member key. Rebuilding every
group with its members in reverse order gives the same keys.

The claim that an application node names members of its own group only
through body `reference` terms holds. `result_type`, literal types and member
declarations are type or model nodes. A type or model node names no
expression, value or function node, so it is never in an application node's
group.

The FR-093 `quantity.convert` mode `exact` agrees with the catalog.
`quire.op.quantity.convert` has member `type_argument` with `member_family`
`quantity`, and `type_pinned_modes.rounding` lists `quantity`. QSpec FR-142
reports no loss for a conversion into an exact target. ADR-013 QC-26 records
this as a request.

The integrity gaps are in how FR-092 relates to FR-322 and to IR-242, and in
one justification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QC-24 and FR-092 say the FR-322 ordinal is circular because "the v2 graph is ordered by node key". FR-322 says no such thing. `identity_projection` is typed-key ordered, but the graph's node order is a separate input: FR-322-AC-14 says editing "node order" changes the package id. The v2 README's ascending-digest order is the reader's check order. IR-242 (`quire-contract-ir` 210d47a) takes a group's ordinals from the array order of `semantic_graph.nodes`. Its review fix pins a test in which array order and digest order disagree, and the package is admitted. So FR-322's rule is not canonical, since the writer picks the order, but it is not circular. QSL can conform now: its emission can write each group's members in the v2 array in ordinal order. Then FR-322's graph-order ordinal equals FR-092's ordinal, and IR-242 recomputes QSL's application keys with no QSpec change. FR-093 gives the emission arm the graph order and leaves it unconstrained. Fix: (1) In FR-093's "Who builds the lowering", require the S1b emission arm to write each recursion group's members in ascending FR-092 ordinal. (2) Restate QC-24, FR-092:204-210 and the FR-092 Dependencies bullet: FR-322's graph-order ordinal is writer-chosen, so it is not canonical, and QSL fixes it by content and writes that order. The QSpec requests then shrink to adopting the content order as canonical, adding the group identity (OQ-1), and the structural `group` and `semantic_type` shapes. (3) Change FR-093's IR-242 bullet to match. | FR-092:204-210, :938-946; FR-093:196-199, :241-243; ADR-013 QC-24; QSpec FR-322 lines 55-56 and AC-14; `quire-contract-ir` 210d47a `validate_application_keys` |
| FND-002 | medium | FR-322 admits a cycle only when every member carries the same explicit `recursion_group` label. FR-092 now derives the group itself as a strongly connected component, but nothing says what label the v2 node carries. FR-092's Inputs still lists "the node's recursion group … (FR-322 `recursion_group`)" as an input, although the Recursion groups section computes it. FR-093 says only that emission "adds … `recursion_group`". Fix: remove the group from FR-092 Inputs, since Behavior derives it. State the label: for example, the group digest as lowercase hex, which is already content-derived and unique per group. Add it to AC-11 or to TC-416. | FR-092:54, :126-135; FR-093:196-199; QSpec FR-322 lines 60-62 |
| FND-003 | low | The reason given for "no QSL source forms a one-member group" is wrong. It says a declared record is never its own field type, but `record R { next: R; }` makes `R` its own field type. What actually excludes it is FR-143's recursion rule: `TypeEnvironment::check_recursion` refuses a non-escaping cycle, so a record reaches itself only through an `Option` or collection node, which makes a second member. Tuples cannot recurse at all, because unnamed cycles are refused. So "every group holds a declared record, tuple or function" names a tuple case that never happens. Fix: cite FR-143's recursion rule as the reason, and write "a declared record or function". | FR-092:669-671, :235; `qsl-semantics/src/value/declaration.rs` `check_recursion` (`RecursionEdges::NonEscaping`, `Unnamed`) |
| FND-004 | low | The Status sections of FR-092, FR-093 and FR-094 and TC-413 to TC-419 name the unmerged branch `task/156-a4b-application-key`, individual test functions and "pending merge". The branch name and "pending merge" go stale the moment A4b merges. The repo's CLAUDE.md asks for minimal status bookkeeping. Fix: state what the code does and which ACs are unbacked, and name QSL-156 A4b as the owner. Leave out the branch name and the "pending merge" phrasing. | FR-092:948-971; FR-093:253-262; FR-094:710-721; TC-413 to TC-419 Status |

## Resolution

FND-001 to FND-003 are fixed; FND-004 is fixed in part. FND-001: FR-092 states that FR-322's ordinal comes from the writer's graph order, FR-093's emission text writes each group's members in ordinal order, FR-093-AC-7 and TC-416 step 5 check it for `f`, and QC-24, the FR-092 Dependencies bullet and FR-093's IR-242 bullet are restated; QC-24 asks QSpec to adopt the content order as canonical. FND-002: FR-092 Inputs names the key-naming edges from which `check` derives the group, and FR-093 makes the group digest the `recursion_group` label. FND-003: G1's note cites FR-143's cycle rule, the group-definition paragraph states that FR-143 admits no cycle through a tuple position, and "Groups that collide" names a declared record or function. FND-004: the branch name is removed; "implemented, pending merge" stays because QSL-211 asks the Status sections to record A4b's state that way.
