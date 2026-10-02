---
id: SR-721
title: "Second delta code review of the PR #484 fix round"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5e11d336ab776ceafd592ce12802a3f5c3f99d2a; qsl-semantics/src/check/assemble/tests.rs; qsl-forms/tests/it/value_forms.rs; spec/test-cases/TC-402-assembler-reads-no-cst.md; reviews/"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---
## Summary

Transcribed verbatim from the reviewer's Linear comment https://linear.app/agent-ix/issue/QSL-141/adr-011-m-3b-per-family-parsed-form-types-incremental-with-m-6a-m-6e#comment-a257651e. Ticket QSL-141, PR quire-spec-language#484.

Delta review of the second fix round, `bee599c7..5e11d336`. It covers the TC-402 and TC-403 test changes and the review files the coder transcribed. All three findings are low. None of them lets a real CST edge into the assembler today.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | (test-intent) The TC-402 step-2 re-export scan sees only `pub use`. A `pub type X = qsl_cst::...;` alias in qsl-forms, imported by the assembler, passes every check::assemble test. | Reviewer comment below |
| FND-002 | low | (other) The coder's transcribed SR-712 file fails `quire validate`: no `## Summary`, no `## Findings` table, and no `scope` in the frontmatter. | Reviewer comment below |
| FND-003 | low | (soundness) The name loop in `no_qsl_cst_type_is_re_exported_to_the_assembler` can never fail. It runs only once `found` is empty, and then `names` is empty too. | Reviewer comment below |

## Reviewer comment (verbatim)

<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-721 method=code-review lang=rust pr=quire-spec-language#484 reviewed=5e11d336ab776ceafd592ce12802a3f5c3f99d2a date=2026-09-26 -->

Delta review of the second fix round, `bee599c7..5e11d336`. It covers the TC-402 and TC-403 test changes and the review files the coder transcribed. All three findings are low. None of them lets a real CST edge into the assembler today.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | test-intent | The TC-402 step-2 re-export scan sees only `pub use`. A `pub type X = qsl_cst::...;` alias in qsl-forms, imported by the assembler, passes every check::assemble test. |
| FND-002 | low | other | The coder's transcribed SR-712 file fails `quire validate`: no `## Summary`, no `## Findings` table, and no `scope` in the frontmatter. |
| FND-003 | low | soundness | The name loop in `no_qsl_cst_type_is_re_exported_to_the_assembler` can never fail. It runs only once `found` is empty, and then `names` is empty too. |

+++ [reviewer data]

```yaml
scope:
  - {id: qsl-semantics/src/check/assemble/tests.rs, path: "qsl-semantics/src/check/assemble/tests.rs"}
  - {id: qsl-forms/tests/it/value_forms.rs, path: "qsl-forms/tests/it/value_forms.rs"}
  - {id: TC-402, path: "spec/test-cases/TC-402-assembler-reads-no-cst.md"}
  - {id: reviews/26-09-26-qsl-141-pr484-code-review.md, path: "reviews/26-09-26-qsl-141-pr484-code-review.md"}
  - {id: reviews/26-09-26-qsl-141-pr484-spec-review-base.md, path: "reviews/26-09-26-qsl-141-pr484-spec-review-base.md"}
  - {id: reviews/26-09-26-qsl-141-pr484-delta-code-review.md, path: "reviews/26-09-26-qsl-141-pr484-delta-code-review.md"}
findings:
  - fnd: FND-001
    severity: low
    method: code-review
    check: test-intent
    artifact_id: TC-402
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "568-570"
    excerpt: |-
      impl<'ast> syn::visit::Visit<'ast> for ReExports {
          fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
              if !matches!(item.vis, syn::Visibility::Inherited) && use_tree_names_cst(&item.tree) {
    finding: "TC-402 step 2 says 'or to any type it re-exports'. A public type alias is a
      re-export in effect, and this repo's own tests/it/layer_crate_reexports.rs counts
      `pub type` as one. Mutation at 5e11d336: `pub type CstProduction = qsl_cst::Production;`
      in qsl-forms/src/lib.rs plus `use qsl_forms::CstProduction as _P;` in check/assemble.rs
      left all 41 check::assemble tests green. No such alias exists today. Fix: also
      visit ItemType with non-inherited visibility whose type names a qsl_cst path."
  - fnd: FND-002
    severity: low
    method: code-review
    check: other
    artifact_id: SR-712
    path: reviews/26-09-26-qsl-141-pr484-delta-code-review.md
    lines: "1-8"
    excerpt: |-
      ---
      id: SR-712
      title: "Delta code review of the PR #484 fix round"
      type: SpecReview
      analysis: code-review
      ---
    finding: "`quire validate --scope . reviews/26-09-26-qsl-141-pr484-*.md` at 5e11d336 fails
      on this file: required 'summary' (section_body(Summary)) is missing, and required
      'findings' (table_row(under Findings)) is missing. The frontmatter also has no
      `scope: agent-ix/quire-spec-language@bee599c7...`. The content matches the Linear
      comment verbatim. Only the SpecReview wrapper is missing. make ci does not run
      quire validate, so nothing is red. Fix: add `scope`, a `## Summary`, and a
      `## Findings` table with the three rows (ID/Severity/Summary/Refs), keeping
      the transcribed comment below them."
  - fnd: FND-003
    severity: low
    method: code-review
    check: soundness
    artifact_id: TC-402
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "615-623"
    excerpt: |-
      assert!(found.is_empty(), "qsl_cst is re-exported: {found:?}");
      for source in [include_str!("../assemble.rs"), include_str!("units.rs")] {
          for name in &names {
              assert!(
                  !source.contains(name.as_str()),
                  "the assembler names re-export {name}"
              );
    finding: "`names` gets entries only from a use item that was also pushed to `found`, so
      once `found.is_empty()` holds, `names` is empty and the loop asserts nothing. The
      doc comment's claim that 'the assembler's own files name no item a qsl_cst re-export
      could carry' is not what this code tests. If it ever ran, `source.contains(name)`
      would also match substrings (for example `*`). Fix: delete the loop and that
      clause of the doc comment, or collect the names before the `found` assert."
```

+++

