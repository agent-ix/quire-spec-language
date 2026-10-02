---
id: SR-712
title: "Delta code review of the PR #484 fix round"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@bee599c7a2f174864f0e90ed3911b2aec5d39e6c; qsl-semantics/src/check/assemble/tests.rs; qsl-forms/tests/it/value_forms.rs; spec/test-cases/TC-402-assembler-reads-no-cst.md; reviews/"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
---
## Summary

Transcribed verbatim from the reviewer's Linear comment https://linear.app/agent-ix/issue/QSL-141/adr-011-m-3b-per-family-parsed-form-types-incremental-with-m-6a-m-6e#comment-c6de6739. Ticket QSL-141, PR quire-spec-language#484.

Delta review of the fix round, `5a51beb0..bee599c7`. It covers new tests added for SR-707 and SR-708.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | (test-intent) `no_qsl_cst_type_is_re_exported_from_qsl_semantics` scans a crate that cannot re-export `qsl_cst` at all. qsl-cst is only a dev-dependency there. The re-export that can compile is one in qsl-forms, and it passes this test. | Reviewer comment below |
| FND-002 | medium | (test-intent) `the_assembler_tests_reach_qsl_cst_only_to_run_s1` visits only `syn::Path`. A `use qsl_cst::CstNode` in the assembler's tests, or a `qsl_cst::` path inside a macro, passes it. | Reviewer comment below |
| FND-003 | low | (coverage) The TC-403 step-3 walk asserts `seen >= 7`, but the two bodies have 9 non-root nodes. The walk also follows the span tree, so a node with no span ends the walk without failing. | Reviewer comment below |

## Reviewer comment (verbatim)

<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-712 method=code-review lang=rust pr=quire-spec-language#484 reviewed=bee599c7a2f174864f0e90ed3911b2aec5d39e6c date=2026-09-26 -->

Delta review of the fix round, `5a51beb0..bee599c7`. It covers new tests added for SR-707 and SR-708.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | test-intent | `no_qsl_cst_type_is_re_exported_from_qsl_semantics` scans a crate that cannot re-export `qsl_cst` at all. qsl-cst is only a dev-dependency there. The re-export that can compile is one in qsl-forms, and it passes this test. |
| FND-002 | medium | test-intent | `the_assembler_tests_reach_qsl_cst_only_to_run_s1` visits only `syn::Path`. A `use qsl_cst::CstNode` in the assembler's tests, or a `qsl_cst::` path inside a macro, passes it. |
| FND-003 | low | coverage | The TC-403 step-3 walk asserts `seen >= 7`, but the two bodies have 9 non-root nodes. The walk also follows the span tree, so a node with no span ends the walk without failing. |

+++ [reviewer data]

```yaml
scope:
  - {id: qsl-semantics/src/check/assemble/tests.rs, path: "qsl-semantics/src/check/assemble/tests.rs"}
  - {id: qsl-forms/tests/it/identity_free_forms.rs, path: "qsl-forms/tests/it/identity_free_forms.rs"}
  - {id: qsl-forms/tests/it/value_forms.rs, path: "qsl-forms/tests/it/value_forms.rs"}
  - {id: TC-398, path: "spec/test-cases/TC-398-value-builder-layering-and-identity-free-forms.md"}
  - {id: TC-402, path: "spec/test-cases/TC-402-assembler-reads-no-cst.md"}
  - {id: TC-403, path: "spec/test-cases/TC-403-value-expression-nodes-carry-spans.md"}
  - {id: reviews/26-09-26-qsl-141-pr484-code-review.md, path: "reviews/26-09-26-qsl-141-pr484-code-review.md"}
  - {id: reviews/26-09-26-qsl-141-pr484-spec-review-base.md, path: "reviews/26-09-26-qsl-141-pr484-spec-review-base.md"}
findings:
  - fnd: FND-001
    severity: medium
    method: code-review
    check: test-intent
    artifact_id: TC-402
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "542-578"
    excerpt: |-
      /// TC-402 step 2, re-exported types: no item of `qsl-semantics` re-exports
      /// (`pub use`) anything from `qsl_cst`, so the assembler cannot reach a CST
      /// type through a `qsl-semantics` path either.
      #[trace("FR-091-AC-20", "TC-402")]
      #[test]
      fn no_qsl_cst_type_is_re_exported_from_qsl_semantics() {
      ...
          rust_files(
              &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
    finding: "qsl-cst appears in qsl-semantics only under [dev-dependencies] (Cargo.toml:63).
      So non-test code there cannot `pub use qsl_cst` at all, and the test can fail only on a
      #[cfg(test)] re-export nobody can reach. The assembler reaches types through qsl_forms,
      which is a normal dependency and depends on qsl-cst. Mutation at bee599c7:
      `pub use qsl_cst::Production as CstProduction;` in qsl-forms/src/lib.rs plus
      `use qsl_forms::CstProduction as _P;` in check/assemble.rs left all 41 check::assemble
      tests green. Fix: scan the normal dependencies that depend on qsl-cst (today qsl-forms)
      for a public re-export of qsl_cst. Or resolve each qsl_forms name that assemble.rs and
      units.rs import, and refuse any that comes from qsl_cst."
  - fnd: FND-002
    severity: medium
    method: code-review
    check: test-intent
    artifact_id: TC-402
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "580-610"
    excerpt: |-
      impl<'ast> syn::visit::Visit<'ast> for Reach {
          fn visit_path(&mut self, path: &'ast syn::Path) {
              let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
              if names.first().is_some_and(|first| first == "qsl_cst") {
    finding: "TC-402 step 3 says to resolve every qsl_cst edge in the #[cfg(test)] items.
      A use tree is syn::UseTree, not syn::Path, and syn::visit does not enter macro tokens.
      So `use qsl_cst::CstNode as _Mutant;` passes, and so does a qsl_cst:: path inside
      assert!/matches!. Mutation at bee599c7: adding that `use` line to assemble/tests.rs
      left the test green. Fix: judge use trees as well, with the use_tree_names_cst walk
      already in this file extended to check the second segment, and scan macro tokens the
      way identity_free_forms.rs's visit_macro does."
  - fnd: FND-003
    severity: low
    method: code-review
    check: coverage
    artifact_id: TC-403
    path: qsl-forms/tests/it/value_forms.rs
    lines: "598-635"
    excerpt: |-
      while let Some(span) = spans.at(&{
          let mut child = path.clone();
          child.push(index);
          child
      }) {
      ...
      assert!(seen >= 7, "the walk visits the nested nodes, saw {seen}");
    finding: "TC-403 expects that every node carries a span. The walk stops at the first
      child index whose span is None, and the bound is 7, but the two bodies have 9
      non-root nodes: If with a, b and Add(c, d), and Multiply with Add(a, b) and c.
      So up to two nodes can lack a span and the test still passes. Fix: assert
      seen == 9, or walk the Expression tree and require a span at each node's path."
```

+++


## Dispositions

Transcribed verbatim from the reviewer's Linear comment https://linear.app/agent-ix/issue/QSL-141/adr-011-m-3b-per-family-parsed-form-types-incremental-with-m-6a-m-6e#comment-8fafb046.

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-712 pr=quire-spec-language#484 reviewed=5e11d336ab776ceafd592ce12802a3f5c3f99d2a date=2026-09-26 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5e11d336 |
| FND-002 | fixed | 5e11d336 |
| FND-003 | fixed | 5e11d336 |

Each fix was verified by mutation in a scratch worktree at 5e11d336. Each mutation that the finding said went undetected now fails its test. At the unmutated head, the check::assemble tests passed 41 of 41, and the qsl-forms every_expression and value_module tests passed 3 of 3.

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    fix_sha: 5e11d336ab776ceafd592ce12802a3f5c3f99d2a
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "547-624"
    after_excerpt: |-
      fn no_qsl_cst_type_is_re_exported_to_the_assembler() {
          let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
          let mut found = Vec::new();
          let mut names = Vec::new();
          for dir in [manifest.join("src"), manifest.join("../qsl-forms/src")] {
              let (lines, exported) = cst_re_exports(&dir);
              found.extend(lines);
              names.extend(exported);
          }
          assert!(found.is_empty(), "qsl_cst is re-exported: {found:?}");
    evidence: "`pub use qsl_cst::Production as CstProduction;` in qsl-forms/src/lib.rs + `use qsl_forms::CstProduction as _P;` in assemble.rs -> FAILED, qsl_cst is re-exported: [.../qsl-forms/src/lib.rs:48]"
  - fnd: FND-002
    outcome: fixed
    fix_sha: 5e11d336ab776ceafd592ce12802a3f5c3f99d2a
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "631-720"
    after_excerpt: |-
      impl<'ast> syn::visit::Visit<'ast> for Reach {
          fn visit_path(&mut self, path: &'ast syn::Path) {
              let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
              self.judge(&names);
              syn::visit::visit_path(self, path);
          }
          fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
              self.use_tree(&item.tree, &mut Vec::new());
          }
          fn visit_macro(&mut self, mac: &'ast syn::Macro) {
              self.tokens(mac.tokens.clone());
              syn::visit::visit_macro(self, mac);
          }
      }
    evidence: "`use qsl_cst::CstNode as _Mutant;` in tests.rs -> FAILED, beyond S1: [qsl_cst::CstNode]; `stringify!(qsl_cst::CstNode)` in a test in tests.rs -> FAILED the same way"
  - fnd: FND-003
    outcome: fixed
    fix_sha: 5e11d336ab776ceafd592ce12802a3f5c3f99d2a
    path: qsl-forms/tests/it/value_forms.rs
    lines: "634"
    after_excerpt: |-
      assert_eq!(seen, 9, "every non-root node of both bodies carries a span");
    evidence: "ExpressionSpans::child mutated to drop child index 2 (qsl-forms/src/spans.rs:150) -> FAILED, left: 6, right: 9"
```

+++

