---
id: SR-708
title: "Spec review of the FR-091 test-row status flips"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/tests.md; spec/spec.md; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md; spec/test-cases/TC-392..TC-403, TC-406; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; qsl-forms/tests/it/value_forms.rs; qsl-forms/tests/it/identity_free_forms.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-eval/tests/it/source_call.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
---
## Summary

Ticket: QSL-141. PR: quire-spec-language#484. Base checklist over
the three changed spec files. For each `spec/tests.md` row the PR flips to
Passed, I checked that the traced test exists, carries that TC tag, and asserts
the TC file's steps.

Clean: TC-392 (`a_unit_builds_one_form_per_declaration_in_source_order`),
TC-393 (steps 1-3 in `value_forms.rs`, step 4 by the `ValueType`/`NodeKey`
scan), TC-394 (both tests, every mapping row and the four grouping cases),
TC-397 (bound 8; depths 7, 8 and 20 with the right spans), TC-399
(`source_call.rs` runs S1, S2, the assembler, check, link and `call` to `2`) and
TC-400 (all five steps in `the_assembler_reports_every_error`). All are tagged
and their oracles match the expected results. TC-401 and TC-406 stay Partial,
which is right. The FR-091 row in `spec/spec.md` agrees with `spec/tests.md`,
including TC-480 to TC-482 Passed and TC-483 Partial (#477 and #480 are merged).
The FR-110 bullet is accurate: #481 is merged into this head's ancestry,
`FR-108:51` and TC-452 step 4's fixture (line 40) both spell `version "1-draft.2"`,
and "step 4" corrects the old text's "step 1".

## Verdict

**Changes requested.** Two medium findings: TC-398's own Status contradicts the
flipped row, and TC-402 is flipped to Passed on the same grounds TC-401 is kept
Partial.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-398's own Status section is stale. It still reads "Partial: ... Step 1's family-module edges are not written" and does not name the new test, while `spec/tests.md` now shows TC-398 Passed on that test. A reader of the TC file gets the opposite answer. Fix: update the Status to name `value_module_has_edges_only_to_the_forms_core_and_the_lower_crates` and drop "not written". | spec/test-cases/TC-398-value-builder-layering-and-identity-free-forms.md:66-77; spec/tests.md:184 |
| FND-002 | medium | TC-402 is flipped to Passed, but its step 3 (every `#[cfg(test)]` `qsl_cst` edge feeds S2) is "by inspection" per its own Status. Step 2's "or to any type it re-exports" is not checked either: `CstEdges` counts only a literal `qsl_cst` segment, so a `qsl_cst` type reached through another crate's re-export passes. TC-401 is kept Partial for exactly this reason (its step 5 "holds by search but is not a test"), so the two rows are graded inconsistently. Fix: keep TC-402 Partial and name the untested step, or add the step-3 scan. | spec/tests.md:188; spec/test-cases/TC-402-assembler-reads-no-cst.md; qsl-semantics/src/check/assemble/tests.rs:470-503 |
| FND-003 | low | TC-403 is flipped to Passed, but step 3 (walk every node; each span lies inside its parent's) is not a test. Its Status says it "holds by construction" via `ExpressionSpans::push_child`, and `declaration.spans()` is an `Option`, so "every node carries a span" is not asserted. Same inconsistency as FND-002. Fix: keep it Partial, or add the walk to `every_expression_node_carries_its_span`. | spec/tests.md:189; qsl-forms/tests/it/value_forms.rs:569-593 |
| FND-004 | low | TC-396's last expected result, "No step-3 refusal has code `unsupported_construct`", is not asserted. `nested_constructs_of_other_families_refuse_with_their_own_causes` uses `refusals.iter().any(...)` for the wanted cause, which still passes if an `unsupported_construct` refusal comes alongside it. S2's side is covered because the S2 test expects `Ok`. Fix: also assert that no refusal's code is `UnsupportedConstruct`. | spec/tests.md:182; qsl-semantics/src/check/assemble/tests.rs:316-333 |

## Dispositions

Transcribed verbatim from the reviewer's Linear comment https://linear.app/agent-ix/issue/QSL-141/adr-011-m-3b-per-family-parsed-form-types-incremental-with-m-6a-m-6e#comment-3a4b2496 (SR-708 dispositions).

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public id=SR-708 pr=quire-spec-language#484 date=2026-09-26 -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | still-open | Step 3 now has a test, but neither new test can fail on the two ways a CST edge actually gets in. The re-export scan reads the wrong crate, and the step-3 scan skips `use` trees. Both were proved by mutation. Details in SR-712 FND-001 and FND-002. TC-402 should stay Partial until they are fixed. |
| FND-003 | fixed | (with a low follow-up in SR-712 FND-003) |
| FND-004 | fixed | - |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    path: spec/test-cases/TC-398-value-builder-layering-and-identity-free-forms.md
    lines: "66-81"
    after_excerpt: |-
      Passed locally. Step 1's crate edges
      (`tests/it/family_outcome_layering.rs::no_crate_below_layer_three_depends_on_the_check_core`)
      ... Step 1's family-module
      edges are scanned by `value_module_has_edges_only_to_the_forms_core_and_the_lower_crates`
      (`identity_free_forms.rs`): `value.rs`'s `use` trees, inline paths and macro-body
      paths may name only `qsl_cst`, `qsl_foundation`, `quire_exact`, std and the
      forms core (`dispatch`, `spans`, `syntax`), and the test fails on any other
      edge.
  - fnd: FND-002
    outcome: still-open
    reason: >-
      The step-3 "by inspection" half now has a test, the_assembler_tests_reach_qsl_cst_only_to_run_s1
      (TC-402, qsl-semantics/src/check/assemble/tests.rs:585). The step-2 "any type it re-exports"
      half is not effectively tested. no_qsl_cst_type_is_re_exported_from_qsl_semantics (:548) scans
      qsl-semantics/src, but qsl-cst is only a dev-dependency of qsl-semantics (Cargo.toml:63), so
      non-test code there cannot `pub use qsl_cst` at all. The channel that can compile is a
      re-export from qsl-forms, a normal dependency that depends on qsl-cst. Mutation:
      adding `pub use qsl_cst::Production as CstProduction;` to qsl-forms/src/lib.rs and
      `use qsl_forms::CstProduction as _P;` to qsl-semantics/src/check/assemble.rs left all 41
      check::assemble tests and the qsl-forms identity_free tests green. The step-3 scan also
      visits only syn::Path, so `use qsl_cst::CstNode as _Mutant;` added to assemble/tests.rs left
      it green. spec/tests.md:188 still shows TC-402 Passed. New findings: SR-712 FND-001 and FND-002.
  - fnd: FND-003
    outcome: fixed
    path: qsl-forms/tests/it/value_forms.rs
    lines: "595-635"
    after_excerpt: |-
      /// TC-403 step 3: every `Expression` node's span lies inside its parent's.
      #[trace("FR-091-AC-10", "TC-403")]
      #[test]
      fn every_expression_span_lies_inside_its_parents() {
          ...
                  assert!(
                      span.start >= parent.0 && span.end <= parent.1,
                      "child {path:?}/{index} {span:?} escapes its parent {parent:?}"
                  );
          ...
          for source in ["if a then b else c + d", "(a + b) * c"] {
          ...
          assert!(seen >= 7, "the walk visits the nested nodes, saw {seen}");
    note: "The two bodies have 9 non-root nodes, so `seen >= 7` lets two nodes go without a span. Low follow-up is SR-712 FND-003."
  - fnd: FND-004
    outcome: fixed
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "313-318"
    after_excerpt: |-
      assert!(
          refusals
              .iter()
              .all(|refusal| refusal.cause.code() != qsl_foundation::Code::UnsupportedConstruct),
          "no step-3 refusal is unsupported_construct: {refusals:?}"
      );
    note: "Applied to the pre, decreases(pre) and deref bodies. The allInstances body matches exactly one UnresolvedTypeName assembly error, so it cannot carry unsupported_construct either."
```

+++


## Dispositions, round 2

Transcribed verbatim from the reviewer's Linear comment https://linear.app/agent-ix/issue/QSL-141/adr-011-m-3b-per-family-parsed-form-types-incremental-with-m-6a-m-6e#comment-f16a8ec6 (SR-708 round 2).

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public id=SR-708 pr=quire-spec-language#484 date=2026-09-26 -->

Second disposition pass. This updates FND-002 only. FND-001, FND-003 and FND-004 stay fixed, as recorded in the earlier SR-708 dispositions comment.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | This supersedes the earlier still-open outcome. A `pub type` alias gap remains and is a low finding in SR-721 FND-001. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-002
    outcome: fixed
    supersedes: "still-open (SR-708 dispositions)"
    path: qsl-semantics/src/check/assemble/tests.rs
    lines: "600-624, 631-720"
    after_excerpt: |-
      #[trace("FR-091-AC-20", "TC-402")]
      #[test]
      fn no_qsl_cst_type_is_re_exported_to_the_assembler() {
          let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
          ...
          for dir in [manifest.join("src"), manifest.join("../qsl-forms/src")] {
          ...
          assert!(found.is_empty(), "qsl_cst is re-exported: {found:?}");
      ...
              fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
                  self.use_tree(&item.tree, &mut Vec::new());
              }
              fn visit_macro(&mut self, mac: &'ast syn::Macro) {
                  self.tokens(mac.tokens.clone());
    evidence: >-
      Mutations in a scratch worktree. (1) `pub use qsl_cst::Production as CstProduction;`
      in qsl-forms/src/lib.rs plus `use qsl_forms::CstProduction as _P;` in check/assemble.rs fails
      no_qsl_cst_type_is_re_exported_to_the_assembler ("qsl_cst is re-exported: [.../qsl-forms/src/lib.rs:48]").
      (2) `use qsl_cst::CstNode as _Mutant;` in assemble/tests.rs fails
      the_assembler_tests_reach_qsl_cst_only_to_run_s1 ("beyond S1: [qsl_cst::CstNode]").
      (3) `stringify!(qsl_cst::CstNode)` in a new test in tests.rs fails it the same way.
      TC-402 Status names both tests, and they are traced FR-091-AC-20 / TC-402.
    residual: "A `pub type X = qsl_cst::...;` alias in qsl-forms still passes. Recorded as SR-721 FND-001 (low)."
```

+++

