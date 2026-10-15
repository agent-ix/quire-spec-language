---
id: SR-9124
title: "Gap analysis of quire-spec-language PR #844: protocol reference nodes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9309d436002b08a21ead02a6122ceb9662fcdb7a; FR-308-AC-6, FR-218-AC-8, FR-218-AC-9, FR-092-AC-16, FR-092-AC-17, FR-092-AC-18, TC-1021 against qsl-forms/tests/it/protocol_reference_forms.rs, qsl-semantics/src/check/profile.rs, qsl-package/src/emit/tests/protocol_reference.rs, qsl-package/src/emit/tests/admission_corpus.rs, qsl-eval/tests/it/protocol_reference.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #844

## Summary

Ticket: QSL-764. Planless gap analysis, scoped to the ACs PR #840 added
(Plan completion: not assessed). Each AC was traced to its tagged tests,
and each test was read to confirm it can fail.

Examined:
- FR-308-AC-6: target, whole-expression span, two forms in source order,
  declaration beside a reference, formatter round trip, and the three
  invalid_syntax refusals (protocol_reference_forms.rs) (examined, clean).
- FR-218-AC-8: Missing gives MissingName("Missing"); a qualified `M::Fill`
  gives MissingName; a local function gives ill_typed/operator-ineligible;
  `protocol Fill and protocol Fill` gives unsupported_construct/
  expression-form at the second, naming a different first location
  (examined, clean).
- FR-218-AC-9: the target node is removed from the checked graph and
  emission refuses StaleProtocolReference with the exact reference and
  target ids, missing_declaration/missing-name (examined, clean).
- FR-092-AC-16: carrier body, semantic_type, dependencies, single
  expression occurrence, absent declaration/owner/preimage/recursion group,
  no extra collection, source region equals `protocol Fill`, reader
  Read::Verified (examined, clean).
- FR-092-AC-17: Fill and Drain give different keys, a repeat gives the same
  key, the key equals the independent structural-node preimage digest, and
  a renamed target changes the key (examined, clean).
- FR-092-AC-18: protocol_clause body, law, `sequence` operator, Boolean
  type, declaration occurrence, profile_selections row, exactly two protocol
  nodes (examined, clean).
- Admission corpus rows for ProtocolClause and ProtocolReference against
  the real reader (examined, clean).
- qsl-eval: a reference refuses unsupported-feature (examined, clean).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS for coverage.** Every AC in scope has a tagged test that can fail.
The one code defect (an event- or commit-rooted protocol refuses as an
internal fault) is in an untested shape, and it is reported once, under
the code review (SR-9123 FND-001), with its regression test as part of the
fix.

The following are recorded and do not block:
- FR-092-AC-17 also says the package identity changes; the test asserts
  only the reference key.
- FR-218-AC-8's import-alias clause is tested with a model alias
  (`M::Fill`), because an import cannot export a protocol.
- The missing-selection test is traced to FR-092-AC-18, but it tests a
  precondition no AC states (see SR-9123 deviation (c)).
