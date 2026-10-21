---
id: SR-12270
title: "QSL-925 PR #1155 gap analysis: FR-134-AC-3..6 and TC-535 against the single invalid-fairness-kinds cause"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f2e1f7770a7aa29f54230a35c3eee881ad20fdd; FR-134 'Manifest fairness_kinds admission' and 'Remaining work', FR-134-AC-3, FR-134-AC-4, FR-134-AC-5, FR-134-AC-6, TC-535 V4-V17; QSpec FR-272 and FR-368-AC-8/AC-9 (by id); qsl-route/tests/it/fairness_kinds.rs"
review_set: subset
---

## Summary

Ticket: QSL-925. PR quire-spec-language#1155 at head 2f2e1f777. Planless gap analysis scoped to the diff. Plan completion: not assessed. Criteria were traced to tagged tests by reading, and the focused `fairness_kinds` run was used as evidence. QSpec is cited by id only.

Bindings examined:

- FR-134-AC-3: `v4_v5_v6_v11_v14_an_unknown_element_is_refused_naming_the_first` (V4, V5, V6, V11, V14 → `UnknownLabel`) and `v12_v13_a_member_that_is_not_a_list_is_refused` (`null`, `"weak"`, `{}` → `NotAList`). Correct.
- FR-134-AC-4: `v7_v8_v10_v15_a_repeat_is_refused_and_an_unknown_element_wins` (`DuplicateLabel("weak")`, and V10 → `UnknownLabel("eventual")`). Correct.
- FR-134-AC-5: `v9_an_absent_member_of_a_temporal_backend_is_refused` (`Absent`), and the unchanged V1-V3, step-4 and step-5 tests. Correct.
- FR-134-AC-6: `v16_v17_a_pair_or_domains_defect_wins_over_a_fairness_defect` (`unknown-kind`, `invalid-domains`). Correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-134 "Remaining work" still reads "The code that emits the three earlier causes (`qsl-route/src/lib.rs`, `qsl-route/tests/it/fairness_kinds.rs`) is Linear QSL-925." This PR is that work. Once it merges, the line describes pending work that is done. Delete the bullet (and the section, since it is the only one) in this PR. | spec/functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md:140-143 |

## Verdict

PASS (no major). Every TC-535 vector V4-V17 asserts `invalid_capability/invalid-fairness-kinds` with the exact defect and value, or the earlier pair/domains cause. The precedence the spec states (`absent`, `not-a-list`, `unknown-label`, `duplicate-label`, unknown over repeat) is implemented and tested. QSpec FR-368-AC-8/AC-9 forms (`"weak"`, `null`, an unknown element, `["weak","weak"]`, unknown plus repeat) are covered by the same vectors. No stub, no tautology, no over-claimed tag in the diff.

## Dispositions

| Finding | Disposition |
| --- | --- |
| FND-001 | Fixed: the FR-134 "Remaining work" section is deleted. |
