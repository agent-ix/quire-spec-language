---
id: SR-12172
title: "Gap analysis of quire-spec-language PR #1014 against QSL-885 and FR-218"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5b258e4a32cb3bf0a8d9b4a1373d0991bd95cdf8; PR #1014 (base 2009fd339): qsl-semantics/src/check/{protocol_requirement, protocol_relationship, protocol_slots, protocol_clause, refusal.rs, mod.rs}, qsl-replay/src/spine.rs, qsl-package/src/emit/tests.rs; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, FR-211, FR-208"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
---
# Gap analysis of quire-spec-language PR #1014

## Summary

Ticket: QSL-885. Planless gap analysis over the PR diff. Plan completion:
not assessed. I checked each AC against the tests by reading them. No
matrix was recomputed and no build was run. QSpec FR-464, FR-465 and TC-947
were read as normative references.

ACs examined and their tests:

- FR-218-AC-1, tagged: `the_reference_protocols_check_with_their_edges`.
  Fill, Pay, Chatter and the replicated-role protocol all check, with the
  Fill branch and join edges and Chatter's message edge. Covered.
- FR-218-AC-3, tagged: `readers_are_recorded_and_a_scope_role_records_its_region`.
  The only reader, the out-of-scope read refusal, and the region `S`, then
  `Main`. Covered.
- FR-218-AC-14, TC-1056 only: `a_relationship_binds_its_model_relationship`.
  Key and end types, missing name, wrong-export for an object type and a
  record, ambiguity. The port case and the "both keys" member are not
  covered (declared). Correctly untagged.
- FR-218-AC-15, tagged: `a_related_clause_types_each_end_by_its_relationship`.
  Covered except one assertion (FND-002).
- FR-218-AC-16, TC-1056 only: `a_requirement_binds_its_temporal_clause_by_position`.
  The import-alias case is not tested (declared). Correctly untagged.
- FR-218-AC-25, tagged: `a_requirement_whose_clause_reaches_its_protocol_is_a_cycle`.
  Covered for its cycle behaviour (FND-003 on the package half).
- FR-211-AC-1, TC-656 only. The checking half is covered by the two terminal
  tests. The package-identity half waits on FR-383 (declared).
- FR-208-AC-1, TC-653 only. S3 checking of its protocol is covered. Spawn and
  retire belong to the protocol system (declared).
- QSpec FR-464 (AC-11 compensate rule), tagged `QSpec-FR-464`:
  `a_compensate_template_needs_a_temporal_profile`. Covered.
- TC-947 role vectors RL11, RL14, RL15 and RL16 (max 0, a capture read, a
  run-tree read, a non-Boolean `until`) and TM10 (a capture read under
  `on each`) are each asserted at S3.

Declared spec conflicts or gaps, routed and not counted here: the
replicated-role binder name (role name against TC-947's `w`), the "both
declaration keys" in AC-14, requirement binding by position until FR-337, the
`scope` region reading, and `join predicate` (STD-190).

No production code without an owning requirement was found. Each new module
cites FR-218, FR-211, or QSpec FR-464/FR-465. No stub was found: every new
`Checked*` field is filled by a real check and read by a test.

## Verdict

Mergeable. No high finding. One medium (the port-ended relationship guard has
no test) and two lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nothing tests the one remaining `Unimplemented` path this PR adds, a `relationship` whose ends are not both object types, such as a systems connection between ports. That path is the only thing that stops such a relationship from compiling with untyped ends. Deleting the `let (Some(source), Some(target_end)) = ... else` refusal would pass every test. The same missing fixture is why AC-14's `M::out` (a port) wrong-export case is untested. One `Endpoint` export and one relationship between endpoints in the fixture would cover both: a refusal at the port name for `M::out`, and `unsupported_construct`/`not-yet-implemented` for the endpoint relationship. | qsl-semantics/src/check/protocol_relationship.rs:140-156; qsl-semantics/src/check/protocol_relationship/tests.rs:183 |
| FND-002 | low | FR-218-AC-15 says "the node holds the clause". The test never asserts `CheckedRelated::node`, so a clause recorded under the wrong node would pass. In this fixture only `Got` writes `related by`, so the risk is small. One `assert_eq!(clause.node, <Got's id>)` closes it. | qsl-semantics/src/check/protocol_relationship/tests.rs:250-260 |
| FND-003 | low | FR-218-AC-25 "Each refusal produces no package", and its doc line "No refused unit compiles", are vacuous here. `protocol_refusals` expects an error for every unit, because any unit with a temporal clause refuses `not-emitted`, so a unit would show no package even if the cycle refusal were missing. The cycle refusal itself is asserted, so the tag stands. The "at the requirement" span is also never asserted. | qsl-semantics/src/check/protocol_requirement/tests.rs:84-98,186-246 |

The findings table holds defects only.
