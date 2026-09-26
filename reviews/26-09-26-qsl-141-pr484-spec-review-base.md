---
id: SR-708
title: "Spec review of the FR-091 test-row status flips"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5a51beb0e6ccd69b5086c2510cf22d89ed907e15; spec/tests.md; spec/spec.md; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md; spec/test-cases/TC-392..TC-403, TC-406; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; qsl-forms/tests/it/value_forms.rs; qsl-forms/tests/it/identity_free_forms.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-eval/tests/it/source_call.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
---
## Summary

Ticket: QSL-141. PR: quire-spec-language#484 at 5a51beb0. Base checklist over
the three changed spec files. For each `spec/tests.md` row the PR flips to
Passed, I checked that the traced test exists, carries that TC tag, and asserts
the TC file's steps.

Clean: TC-392 (`a_unit_builds_one_form_per_declaration_in_source_order`),
TC-393 (steps 1-3 in `value_forms.rs`, step 4 by the `ValueType`/`NodeKey`
scan), TC-394 (both tests, every mapping row and the four grouping cases),
TC-397 (bound 8; depths 7, 8 and 20 with the right spans), TC-399
(`source_call.rs` runs S1, S2, the assembler, check, link and `call` to `2`) and
TC-400 (all five steps in `the_assembler_reports_every_error`). All are tagged
and their oracles match the expected results. TC-401 and TC-406 stay Partial,
which is right. The FR-091 row in `spec/spec.md` agrees with `spec/tests.md`,
including TC-480 to TC-482 Passed and TC-483 Partial (#477 and #480 are merged).
The FR-110 bullet is accurate: #481 is merged into this head's ancestry,
`FR-108:51` and TC-452 step 4's fixture (line 40) both spell `version "1-draft.2"`,
and "step 4" corrects the old text's "step 1".

## Verdict

**Changes requested.** Two medium findings: TC-398's own Status contradicts the
flipped row, and TC-402 is flipped to Passed on the same grounds TC-401 is kept
Partial.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-398's own Status section is stale. It still reads "Partial: ... Step 1's family-module edges are not written" and does not name the new test, while `spec/tests.md` now shows TC-398 Passed on that test. A reader of the TC file gets the opposite answer. Fix: update the Status to name `value_module_has_edges_only_to_the_forms_core_and_the_lower_crates` and drop "not written". | spec/test-cases/TC-398-value-builder-layering-and-identity-free-forms.md:66-77; spec/tests.md:184 |
| FND-002 | medium | TC-402 is flipped to Passed, but its step 3 (every `#[cfg(test)]` `qsl_cst` edge feeds S2) is "by inspection" per its own Status. Step 2's "or to any type it re-exports" is not checked either: `CstEdges` counts only a literal `qsl_cst` segment, so a `qsl_cst` type reached through another crate's re-export passes. TC-401 is kept Partial for exactly this reason (its step 5 "holds by search but is not a test"), so the two rows are graded inconsistently. Fix: keep TC-402 Partial and name the untested step, or add the step-3 scan. | spec/tests.md:188; spec/test-cases/TC-402-assembler-reads-no-cst.md; qsl-semantics/src/check/assemble/tests.rs:470-503 |
| FND-003 | low | TC-403 is flipped to Passed, but step 3 (walk every node; each span lies inside its parent's) is not a test. Its Status says it "holds by construction" via `ExpressionSpans::push_child`, and `declaration.spans()` is an `Option`, so "every node carries a span" is not asserted. Same inconsistency as FND-002. Fix: keep it Partial, or add the walk to `every_expression_node_carries_its_span`. | spec/tests.md:189; qsl-forms/tests/it/value_forms.rs:569-593 |
| FND-004 | low | TC-396's last expected result, "No step-3 refusal has code `unsupported_construct`", is not asserted. `nested_constructs_of_other_families_refuse_with_their_own_causes` uses `refusals.iter().any(...)` for the wanted cause, which still passes if an `unsupported_construct` refusal comes alongside it. S2's side is covered because the S2 test expects `Ok`. Fix: also assert that no refusal's code is `UnsupportedConstruct`. | spec/tests.md:182; qsl-semantics/src/check/assemble/tests.rs:316-333 |
