---
id: SR-20261107070951249606
title: "Code review — IR-1003 QSL occurrence classification"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; tests/it/mapping_support_occurrences.rs, tests/it/main.rs; FR-527-AC-28"
review_set: subset
---

## Summary

Ticket: IR-1003. Reviewed frozen f584577532995e5188f12870f961dfe486dca285 against origin/main 8c0574211, applying code-review and its Rust lane with manual AC alignment. Only the new integration test and module registration change.

## Verdict

**PASS** — no defects found. The real emitter feeds the existing strict Contract IR reader, which admits with domain evidence before classification. Two original names and distinct clock authorities are independent fixture expectations; exactly one clause node and two occurrences are asserted. Wrong declaration, role and ordinal are tested independently on valid baseline keys. Two selected model identities have explicit expected owner order opposite authored alias and input order. No alias map is constructed.

The public-field mutations are meaningful for AC-28: fresh production reclassification is compared against changed support, emptied premises and retained name, with equal sealed selection and retained admission membership. This verifies complete value equality detects forged public facts; it does not claim an implemented downstream bridge authorization gate. Emptying premises also fails the test if the genuine fixture starts with no premises. Existing shared-name tests were inspected; this adds distinct occurrence clocks, key negatives, owner identity ordering and public mutation evidence rather than another fixture/helper implementation.

## Examined scope

```yaml
scope:
- id: FR-527-AC-28
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: For two declaration occurrences sharing one clause node, the classification
    retains the node and no single name, while the bridge input's independently selected
    occurrence resolves its own name and clock by declaration/role/ordinal. Selected
    model package identities retain the checked owner's order and no authored model
    alias map is inferred. A changed public classification field is unequal to reclassification
    of the original subject even when its sealed selection is unchanged.
bindings:
- test_id: shared_classification_preserves_occurrence_keys_and_detects_changed_public_facts
  ac_id: FR-527-AC-28
  trace: correct
```

FR-527 bridge-input prose was context for the AC: reclassify the bound subject with the same surrounding closure and compare the complete result before trusting public facts. Checked subject lookup and Classification equality implementations were inspected. No applicable AssuranceProfile was found.

## Verification

Inspected /tmp/ir1003-qsl-focused-final.log: focused root integration compile, Clippy, formatting and one new test passed at the frozen candidate; no unnecessary repeat was run. No workflow, production source, dependency or gate changed. Broader Rust gates, make ci, Kani and hosted CI were not run under the dispatch's explicit scoped-verification instruction.

Full gap-analysis, matrix and coverage recalculation were intentionally skipped: tests-only diff and user reserves the one strict census for Luna after IR-724 lands. Spec review is inapplicable because requirements are unchanged. FR-527-AC-20/27 remain unchanged Inspection rows; FR-531-AC-29..33 remain deferred to unpublished owner PR 2489. No measured after-count is claimed. PR body accurately states this remainder; both commits carry the requested Claude Sonnet 5.5 trailer and the PR has the requested Claude Code footer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
