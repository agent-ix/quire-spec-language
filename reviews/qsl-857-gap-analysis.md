---
id: SR-12114
title: "Gap analysis of quire-spec-language PR 949: FR-382-AC-1 to AC-6 and FR-124-AC-4 against their tests and code"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7e2480dc5d66f1a03473dafe3979455f774158fd; PR 949: FR-382-AC-1 to FR-382-AC-6 (TC-1059), FR-124-AC-4 (TC-1060) against qsl-forms/tests/it/terminal_declaration_forms.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-cst/tests/it/terminal_declaration.rs, qsl-forms/src/terminal_declaration.rs, qsl-semantics/src/check/assemble.rs, qsl-cst/src/grammar.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR 949

## Summary

Ticket: QSL-857. Planless gap analysis over the PR diff. Plan completion: not
assessed.

Every acceptance criterion the PR adds has a test tagged with
`#[trace(TC, AC)]` that checks the whole criterion:

- FR-382-AC-1: `terminal_when_builds_its_name_aliases_and_predicate` checks the
  name, alias and model text and spans, the `Not(Boolean(false))` root, the root
  span `not false`, and the form span with `;`.
- FR-382-AC-2: `terminal_any_builds_the_any_kind` checks the name, the model,
  `Any`, and the span.
- FR-382-AC-3: `repeated_terminal_declarations_build_in_source_order_beside_other_forms`
  checks the order terminal, temporal, terminal, the two kinds and the shared
  model alias. Neither form is merged and `build_unit` succeeds, so neither is
  refused.
- FR-382-AC-4: `a_recovering_terminal_declaration_builds_no_form` checks a
  missing `;` (both kinds) and `when true;`, each refusing
  `FormsCause::RecoveringCst`.
- FR-382-AC-5: `building_a_terminal_declaration_twice_gives_equal_forms` covers
  both kinds.
- FR-382-AC-6: `a_terminal_declaration_refuses_as_an_unsupported_declaration_form`
  checks the exact single-error vector, the span of the whole declaration, the
  code, and the catalog pair, for both kinds.
- FR-124-AC-4: the two `qsl-cst` tests check the node texts, that the `Block`
  sits only in the `when` form, and the four malformed rows with
  `invalid_syntax` at the offending token.

No production code lacks an owning requirement. The `Terminal` leading-token
dispatch, the form types, the S1 production and the assembly refusal are each
covered by FR-382 or FR-124. No test is a stub or a tautology. Each asserts
values that a wrong mapping would change. FR-124-AC-1 to AC-3 (the S3 check and
the request writer) are outside this PR. The refusal FR-382 states holds until
they land.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. Each new criterion is backed by a test that checks all of it, and the
trace tags sit only on criteria their test checks.
