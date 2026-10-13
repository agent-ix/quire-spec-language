---
id: SR-7877
title: "QSL-734 PR 799 gap analysis: FR-056-AC-24..28 against TC-994..997"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@437648288cf5f990a76adbf5e8d0c5336014a532; FR-056-AC-24, FR-056-AC-25, FR-056-AC-26, FR-056-AC-27, FR-056-AC-28; spec/test-cases/TC-994..TC-997; qsl-semantics/tests/it/variant_type.rs; qsl-semantics/src/model/intake.rs; qsl-semantics/src/model/normalize.rs; qsl-semantics/src/check/assemble.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-734. `quoin matrix --json` (quoin 0.28.3) at the reviewed sha shows
FR-056-AC-24..AC-28 all `static_status: tagged`. Twelve tests in
`qsl-semantics/tests/it/variant_type.rs` carry `#[trace("TC-99x",
"FR-056-AC-2x")]`. Each is `no run evidence`: this review ran no tests. Below,
each TC step is checked against what its test asserts. I also checked whether
the test would fail if the rule were removed, because the tests were written
after the code.

- TC-994 steps 1-3: declarations, `enum` name and sorted members, `variant`
  case, owner, key and export key, a case's original with a source origin,
  `Level`'s distinct key, reorder invariance, and `Gold`/`gold` are asserted.
  Each would fail without its rule. The bundle is hand-authored, not lifted
  (FND-004). The `enum`'s `sourceSpan` is not asserted: `Tier` has a
  generated origin in the fixture.
- TC-995 steps 1-2: code, cause variant and case node are asserted.
  `Standard` raising nothing is covered by `len() == 1`. The path is not
  asserted (FND-001).
- TC-996: steps 1, 2, 5 and 6 assert code, cause and node, and would fail
  without their rule. Step 3 would not fail without its rule (FND-002).
  Step 4's reader half asserts only the cause family (FND-003), and its lift
  half is untested (FND-004).
- TC-997 steps 1-3: the clause cause and catalog code are asserted, and
  `admit_unit` panicking on refusal shows the package was admitted. Step 2's
  "exports equal those of step 1" compares a document with itself (FND-005).
  Step 3 asserts code, cause and field node, but not the path (FND-001).

## Verdict

PASS WITH FINDINGS. Every AC has a discriminating test for its main rule. The
gaps:

- No typed member path is asserted anywhere, although every TC says
  "Assertions compare typed code, cause, node identity and path".
- One step's test cannot fail.
- One TC step is untested.

I judge FND-001 acceptable to defer only to the #796 adaptation. PR #796 is
open on the same base and gives these causes a typed `member`. Whichever of the
two lands second must adapt (it will not compile otherwise). That adaptation
must add the `member` assertions listed in FND-001, not just the field.

Out of AC scope (no AC binds it): QSpec FR-208:43 says "Normalization admits it
as an effective type". The PR gives a variant type no effective identity:
`normalizable_types` excludes it, as it already excludes value types.
FR-056-AC-24 asks for "one declaration" (FR-154 original declarations), not an
effective one, so this is not a gap against AC-24..28. It needs its own
ticket against FR-208.

The refusal node for a mis-minted case identity is unspecified (FR-056:500-502
"refuse the case", TC-996 step 5 gives no node). The code names the type node
`Tier`. That is defensible, but it should be stated in the spec.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test asserts the TC-required path. Paths appear only in `detail`, which the tests correctly do not assert. Sites that need a typed `member` assertion once #796 lands: `a_payload_on_a_case_refuses_as_an_unsupported_declaration_form` (`Variant.payloadType`), `a_variant_type_with_no_case_refuses_as_malformed` (`TypeDefinition.variants`), `a_case_with_no_source_span_refuses_as_malformed` (`TypeDefinition.variants[0].origin`), `a_variant_meaning_on_a_record_shaped_construct_refuses_as_malformed` (`TypeDefinition.kind`), `equal_case_names_with_distinct_identities_conflict_at_the_type` (`TypeDefinition.variants[1]`, via #796's `ConflictingBinding.member`), `a_case_identity_that_is_not_minted_refuses_as_malformed` (`TypeDefinition.variants[0].identity`) and `a_field_typed_by_a_variant_type_refuses_at_the_field` (`Field.typeRef`). | qsl-semantics/tests/it/variant_type.rs:340-353; qsl-semantics/tests/it/variant_type.rs:384-464; qsl-semantics/tests/it/variant_type.rs:537-566; spec/test-cases/TC-995-refuse-a-variant-payload.md; spec/test-cases/TC-996-refuse-malformed-and-duplicate-variant-cases.md; spec/test-cases/TC-997-clause-or-field-naming-a-variant-type.md |
| FND-002 | medium | TC-996 step 3's test cannot fail if the shape rule is removed. It sets shape `record` and also deletes `variants`. `NodeCtx::array_field` returns an empty slice for a missing member, so the empty-`variants` rule refuses the same node with the same code and cause. Keep `variants` present (a valid case list) so that only the shape is wrong. | qsl-semantics/tests/it/variant_type.rs:401-411; qsl-semantics/src/model/intake.rs:2305-2313; qsl-semantics/src/model/intake.rs:3608-3618 |
| FND-003 | low | TC-996 step 4's reader half asserts only that every refusal is `IntakeMalformedDeclaration`. It does not assert that the retained refusal is the reader's duplicate-identity diagnostic, nor which node it names (the case node `variant/Tier-Gold`). Any malformed refusal would pass. | qsl-semantics/tests/it/variant_type.rs:416-434 |
| FND-004 | medium | TC-996 step 4's second half ("lift a bundle whose enumeration rows slug alike (`Snap Shot`, `Snap_Shot`)... the lift raises `UNSLUGGABLE_NAME`") has no test. TC-994 step 1 says "Lift a bundle", but every vector is hand-authored. No test runs the lift's real variant output (construct shape, minted case identities, case origins) through the new reader, although the minted-identity check depends on matching the lift's `slug`. `model_intake.rs` already lifts bundles through `agent_ix_extraction_frontend`, so both can be tested here. | qsl-semantics/tests/it/variant_type.rs:131-145; qsl-semantics/tests/it/variant_type.rs:416-434; qsl-semantics/tests/it/model_intake.rs:1490-1570; spec/test-cases/TC-996-refuse-malformed-and-duplicate-variant-cases.md |
| FND-005 | low | TC-997 step 2 says the exports "equal those of step 1". The test compares `admitted(&document).records` with `admitted(&tiers(&["Gold", "Standard"])).records`, where `document` is `tiers(&["Gold", "Standard"])`. Both sides are the same input, so the comparison cannot fail. It never takes the package from the compile that refused the clause. | qsl-semantics/tests/it/variant_type.rs:522-529 |
