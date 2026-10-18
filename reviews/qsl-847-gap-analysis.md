---
id: SR-12120
title: "Gap analysis of quire-spec-language PR #951: test and trace preservation across the protocol_clause split"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@15c9be0288460b20e167c7dc6b18595541af614a; PR #951: the 31 tests and 30 #[trace] tags of the old qsl-semantics/src/check/protocol_clause.rs test module against qsl-semantics/src/check/protocol_clause/tests.rs and tests/{anchors,attempts,binders,coverage,node_keys}.rs; tests/it/formatter_identity.rs (FR-003-AC-9)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #951

## Summary

Ticket: QSL-847 (C0 under QSL-839). Planless. Plan completion: not assessed.
The PR changes no requirement, acceptance criterion or test case, so the
Test Matrix can change only if a tagged test was dropped, duplicated or lost
its tag.

- Tests: 31 `#[test]` functions in the old module, 31 in the new tree, each
  name present exactly once, each body identical after path normalisation.
- Trace tags: 30 `#[trace(...)]` attributes in old and new. For every function
  the set of `#[test]`/`#[trace]`/`#[cfg]` attributes matches name for name
  (TC-511/FR-113-AC-1..3, TC-512/FR-113-AC-4.., TC-513/FR-114-AC-*, and the
  rest), so the matrix rows they back are unchanged.
- Test oracles: unchanged, since no test body changed. The one untagged test
  stays untagged.
- `private_protocol_named_contract_keeps_format_identity` (FR-003-AC-9) still
  reads the literal `post Done ...` from
  `an_empty_contracts_list_binds_with_no_refusal`, now in
  `protocol_clause/tests/attempts.rs`, where it is the single match.
- No production code without an owning requirement was added: the new files
  hold moved code only.

## Verdict

PASS. Every test that existed still exists once, with its trace tags and its
oracle. The one defect this review set found is a doc-build failure, recorded
in the code-review artifact (SR-12119 FND-001), not a test gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | Accepted | Placeholder row; the analysis found no gaps, so there is nothing to fix. |
