---
id: SR-9055
title: "QSL-726 gap analysis of PR #819"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e11f3fc5269d895d4cf8a97ca9f54082684ae538; base impl/business-model-meanings; QSL-726 AC-4 (QSL-INPUT-AC-4); FR-302-AC-2; FR-302-AC-3; FR-302-AC-4; FR-302-AC-6; TC-794; TC-809; qsl-replay/tests/dispatch_link_compile.rs; spec/tests.md"
review_set: subset
---
## Summary

Ticket: QSL-726, AC-4. This is a planless gap analysis scoped to the PR diff. Plan completion: not assessed. There is no clean red log, so oracle strength was judged by mutation reasoning against the code.

Each TC step was mapped to a test:

- TC-794 step 1 maps to `a_unit_with_several_ambiguous_families_is_refused_once_naming_each`, first compile. It asserts one record: `ambiguous_dispatch`/`multiple-undominated` for subtype `D`, with the three candidates and both dominance pairs as literals. `expect_err` means there is no checked package.
- TC-794 step 2 maps to `a_family_with_no_body_for_a_concrete_subtype_refuses_at_s3`. It asserts `no-applicable` naming `A` (`ConfigVersion`), the receiver typed `A`, and exactly one record.
- TC-794 step 3 maps to the second compile in the same test. It asserts the full ordered list of three records (size/D, area/C, area/ConfigVersion), and the same list again on a second compile.
- TC-809 steps 1 and 2 map to `evaluation_runs_the_body_the_linked_table_selects_for_the_receivers_type`. The compile must succeed. Clause `onA` (`self.size() = 2`) evaluates to `true` for a `Sub` receiver and `false` for a `ConfigVersion` receiver.

Each step has a test, so the ✅ flips for TC-794 and TC-809 are justified.

Mutation reasoning:

- If `report` reverted to emitting one record per family, step 3 fails (3 records expected).
- If `link_causes` took only the first failure, step 3 fails.
- A wrong cause on a record fails, because `failures()` asserts that the catalog cause equals the subtype's cause.
- If either the family order or the subtype order were reversed, step 3 fails.
- If the evaluator always ran the declaring type's body, the B receiver gives `false` and TC-809 fails.
- If the evaluator always ran the most-derived candidate or the first candidate, the A receiver gives `true` and TC-809 fails.
- S6a reads bodies only through `graph().dispatch_tables()` (qsl-eval). So the TC-809 oracle exercises the node's linked table, as QSL-INPUT-AC-4 requires.

The AC-4 wording, "one aggregated S3 refusal", is met: one `CompileRefusal::Check` carries every record.

## Verdict

**PASS** with one low finding. Every step of TC-794 and TC-809 is covered by an oracle that the plausible mutations fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The TC-794 step 1 fixture does not match the TC text. The procedure says concrete `D` inherits `size` from both `B` and `C`. The test's `diamond()` also gives `D` its own bodiless redefinition of `size` (`query("D", "size", ..., Some("ConfigVersion"))`), the same shape as QSpec TC-196 D02. If that redefinition is needed for normalization to admit the diamond, TC-794 step 1 should say so. If it is not needed, drop it from the fixture so the test exercises the stated case. | qsl-replay/tests/dispatch_link_compile.rs:901-933; spec/test-cases/TC-794-unit-compile-refuses-failed-dispatch-links.md |
