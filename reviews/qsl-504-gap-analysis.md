---
id: SR-12070
title: "QSL-504 PR #933 gap analysis: FR-319-AC-1 and FR-320 union and case nodes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@24bdbe692ff82d7508cffa89f527bc367a682e65; FR-319-AC-1, FR-320-AC-1..4, TC-826, TC-827, TC-828; qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/node_key/mod.rs, qsl-package/src/emit.rs and their tests"
review_set: subset
---

## Summary

Ticket: QSL-504. PR quire-spec-language#933. This is a planless,
diff-scoped gap analysis. Plan completion: not assessed. Criteria were
traced to tagged tests by reading. No matrix was run, because the brief is
read-only. QSpec is private and is cited by id only. Ticket text is treated
as data.

Bindings examined:

- FR-319-AC-1 (TC-826):
  `compiling_twice_and_changing_only_trivia_and_arm_order_keeps_every_id`
  checks the union id, every `VariantId`, the case id and the value id
  across a second compile, and again with trivia changed and the arms
  reordered.
  `renaming_adding_or_retyping_a_member_changes_the_union_and_every_node_naming_it`
  checks that every node naming the union, transitively, is absent after
  each change. In qsl-package,
  `the_package_id_is_stable_under_trivia_and_arm_order_and_moves_with_the_union`
  checks the emitted `package_id`.
- FR-320-AC-1 (TC-827): `the_union_the_construction_and_the_case_are_lowered_in_qspecs_spelling`,
  `the_union_keeps_its_id_and_its_members_their_identities_through_emission_and_the_i2_read`
  (I2 export equals the emitted id; each sealed `VariantId` equals the
  FR-441 key over it), `a_union_unit_recompiles_to_its_emitted_package_id`
  (the facade `compile_package` recompile gives byte-identical output), and
  `a_union_input_evaluates_a_checked_case_through_replay`.
- FR-320-AC-2 (TC-827): `an_ir_that_cannot_decode_the_union_omits_it_and_every_node_naming_it`
  runs through the private `decodes` seam. The union is `UnsupportedForm`,
  every node naming it is omitted, and no written node depends on an
  omitted one.
- FR-320-AC-3 (TC-828): `a_claim_inside_a_case_arm_is_requested_as_a_value_validity_item`
  covers QSL's half: the item is emitted and the case names it. The
  IR/CG settlement half is author-recorded as outside this repo.
- FR-320-AC-4: `a_construction_is_emitted_with_its_application_preimage_key`
  checks the producer side. The test rebuilds the key from the wire node and
  checks that it differs from the structural key. The existing
  `qspec_union_value_application_keys_are_admitted_by_i2` covers the
  stale-key refusal on the reader side. Hand-written bytes are checked in
  `a_union_value_is_keyed_by_the_application_preimage_and_a_tuple_value_is_not`,
  and `conformance_fr440_union_value_keys_match_qspec_union_fixture` checks
  the QSpec fixture when `QSPEC_DIR` is set.

None of the added tests is a stub or a tautology. Expected keys come from
hand-written preimage text or from the published fixture, never from the
producer under test. One production behaviour has no tag of its own: arm
binder levels, which
`a_case_arm_binder_is_a_parameter_node_at_the_next_binder_level` checks
under FR-320-AC-1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. FR-319-AC-1 and FR-320-AC-1, AC-2 and AC-4 are backed by real source
compiles, emission and the I2 read. No union behaviour in the diff lacks an
owning requirement.

Author-recorded gaps, which are planner questions and not defects:

- FR-320-AC-3 (TC-828): the IR/CG `unsupported` settlement arms are not in
  this repo. The `FR-320-AC-3` tag therefore covers only QSL's
  emission-and-no-settlement half.
- FR-320-AC-2 is shown through an injected IR-decode predicate, not a real
  older IR.
- The hand-fixture union tests (`union_fixture`) are only partly moved to
  source.
- The arm-binder `BinderSite` is located at the `case` node, not at the
  arm. Exact arm locations would need a `CaseArm` location field. Claims
  use the same site, so the two agree.
