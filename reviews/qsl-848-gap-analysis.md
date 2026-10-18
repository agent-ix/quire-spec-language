---
id: SR-12112
title: "Gap analysis of quire-spec-language PR 948: FR-308-AC-7 to AC-12 (TC-1055) against qsl-forms protocol member tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@27217c8fcee844f1816899585ec6c2a46e0dd49a; PR #948: FR-308-AC-1..13, TC-887, TC-1055 rows 1-13 against qsl-forms/tests/it/protocol_member_forms.rs and qsl-forms/src/protocol_clause.rs"
review_set: subset
---

## Summary

Ticket: QSL-848. This is a planless gap analysis of the PR diff. It maps each
acceptance criterion to tests and checks how strong each test oracle is.
Plan completion: not assessed. `quoin matrix` was not used for the verdict.
Each criterion was mapped by hand from the `#[trace]` tags in the diff.

- FR-308-AC-7 to AC-12 each have one `#[trace("TC-1055", "FR-308-AC-n")]`
  test in `protocol_member_forms.rs`. Each test asserts typed fields
  (`DeclaredName`, `NameForm`, `TypeFormHead`, `IntervalUpper`,
  `CompensationCommit` variants, `ExprNode::Field` and `ExprNode::Call`),
  spans sliced back to source text, and diagnostic codes (`Code::InvalidSyntax`
  at the count offset, `FormsCause::RecoveringCst`). No test asserts on a
  rendered message.
- The AC-11 overflow row parses `attempts 18446744073709551616`. It asserts
  that the parse is not admissible, that the first diagnostic is
  `invalid_syntax` starting at the count's byte offset, and that `build_unit`
  refuses. A count of zero builds 0, and `[31,30]` builds as written.
- The AC-12 loop breaks each of the five member kinds. It checks that S1
  recovered before it asserts `RecoveringCst`, so the refusal cannot pass
  vacuously. The double build compares the two forms for equality.
- FR-308-AC-1..4 and TC-887 are unchanged and present. No test is traced to
  them. Their forms are not built here (control constructs and channels
  belong to the C1b slice).
- FR-308-AC-13 (TC-1065) is not built and no test is traced to it. On main,
  the grammar has no `loss` selector.
- All traces use `#[trace(...)]`, with no legacy `//trace` comments. Each
  traced test fully asserts its AC, apart from the gap in FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-308-AC-11 test cannot catch a wrong mapping of the compensation's three `Block` bodies. The `compensation()` fixture writes `{ true }` for the `when` guard, the `retry` relation and the `recover` predicate. The test asserts only `expression_text(..) == "true"` for each, so an implementation that swaps `guard`, `relation` and `predicate` (the `let [guard, relation, predicate]` destructuring) still passes. Also, `undo.span` is checked only with `starts_with("compensate Undo")`, so a span cut short at any later member passes. Distinct body texts and a full-span equality would close both gaps. | qsl-forms/tests/it/protocol_member_forms.rs:78-81,298-334; qsl-forms/src/protocol_clause.rs:529 |

## Verdict

PASS. FR-308-AC-7 to AC-12 are covered by typed, span-checked tests that
would fail on most plausible wrong implementations. These include a capture
in the wrong stage, swapped retry binders, `never` built as a node, a merged
duplicate requirement, a missing recovery refusal, and an S2 diagnostic for a
zero count. The one weak oracle is the three identical compensation bodies
(FND-001, medium). AC-1..4 and AC-13 are untested here because their forms
belong to other slices. They are not regressions.
