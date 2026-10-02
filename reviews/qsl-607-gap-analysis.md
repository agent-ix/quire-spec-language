---
id: SR-1222
title: "QSL-607 gap analysis of PR #601 (FR-271, FR-272, TC-746, TC-747)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e83cc06c14295817b0bbccfc6c13e0c580f7732b; PR #601 diff against origin/main; spec/functional/FR-271-*.md; spec/functional/FR-272-*.md; spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-271
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-272
    type: reviews
---
## Summary

Ticket: QSL-607. PR: quire-spec-language#601.

Trace, AC to test. Each fixture test asserts the exact finding list (rule,
identifier, both sites):
- FR-271-AC-1: `tc_746_the_tag_line_defines_the_canonical_set`. This is a
  strong oracle.
- FR-271-AC-2: `tc_746_misplaced_and_duplicate_tags_are_findings`. This is a
  strong oracle.
- FR-272-AC-1: `tc_747_identifier_and_reexport_rules` covers every listed
  case, including the `[[bin]]` member outside `src/`. This is a strong
  oracle. No case has a relative-module re-export (SR-1221 FND-002).
- FR-272-AC-2: `tc_747_copy_rule_across_the_ecosystem`. This is a strong
  oracle.
- FR-272-AC-3: `tc_747_backend_run_and_ir_boundary_types`. The fixture half
  is strong. The real-workspace half is FND-001.
- FR-272-AC-4: `tc_747_every_finding_is_reported_and_exits` checks two
  findings, exit 1, two lines, and a clean fixture that is `Ok`.

Value test:
- The tag replaces a name list, and the deleted `SEMANTIC_VALUE_NAMESAKES`
  was ceremony.
- The remaining `definition_scan` lists stay by team-leader ruling.
- The `#[path]` include is SR-1221 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-272-AC-3 says `quire-contract-model`'s `CollectionType`, `ComparisonOperator`, `EnumDeclaration`, `IntegerDomain` and `ValueType` give no `copy` finding "because their members differ from the tagged types'". Of the five, only `ValueType` is tagged at this head. The other four are not canonical, so `scan` skips them before any member comparison, and the real-workspace assertion holds for them whatever the members are. So 4 of the 5 checks are vacuous. In the test, assert that each listed name is in the canonical set before asserting no copy. Then either tag the four, if ADR-013 §3 makes them owner rows, or drop them from AC-3's list and the test. | xtask/src/canonical_types.rs:1078-1090; spec/functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md (FR-272-AC-3) |

## Verdict

Changes requested: one medium finding. When the SR-1221 fixes land, they
need fixture cases: a relative-module re-export (FND-002), a backend facade
re-export (FND-003) and a cross-crate glob (FND-004).
