---
id: SR-20261109172912000057
title: "Review checker-retained expression node keys"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1818 / QSL-602; PR #2624; checked-body identity storage, lowering retention/settlement, refusal and graph-oracle tests"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-093, type: references }
  - { target: ix://agent-ix/quire-spec-language/NFR-011, type: references }
---
## Summary

Independent code review, including the Rust language lane, of the checker-owned expression-key accessor. The producer-side ownership and graph-oracle tests are sound; one newly introduced quadratic settlement walk requires correction before merge.

## Assurance Context

No applicable AssuranceProfile was found under this repository's spec tree. Ordinary code/Rust review applies. ADR-030 and FR-258 supply flat traversal/lifecycle context; NFR-011 supplies checking-work context. MP-002 was read as performance-measurement context: no wall-time improvement/regression percentage or benchmark qualification is claimed here. The finding below follows the actual control-flow visit count, without changing protected benchmark apparatus. No new profile, exception, CI rule or dependency policy is introduced.

## Verdict

CONDITIONAL — fix FND-001 before merging. No other introduced or in-scope defect found. The lead owns the aggregate gate after review/fixes and artifact custody.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Every function_group calls settle, and the added settlement loop visits and clones every previously retained reference, including already-admitted keys. For N independent functions with m retained reference occurrences each, this performs m*N*(N+1)/2 visits instead of visiting new or pending references once. This unmetered, uncancellable metadata rescan adds quadratic checking work for large legal packages. Track/drain only references requiring this settlement (or use an equivalent delta), preserve recursive resolution, and add a deterministic work-count/scaling control rather than a timing threshold. | qsl-semantics/src/check/lowering.rs:3912-3917 |

## Coverage

Reviewed all five changed files: check/ir.rs, check/lowering.rs, check/lowering/tests.rs, check/lowering/tests/node_keys.rs and check/refusal.rs. No manifest, dependency lock, CI workflow, wire schema or upstream-owned implementation is copied or changed. Application: qsl-semantics 0.1.0.

FR-093-AC-62: the public CheckedNode::node_key accessor returns a borrowed retained key beside location(). Its actual producer is QSL lowering; quire-checked's plain NodeRead descriptor has no graph context and needs no unused wrapper. LowerFrame::Retain observes the actual LeafTerm. Exact live body allocation plus arena index locates metadata only; neither is a structural key preimage. Pending references use existing SCC resolution. Inline literals/dependency references legitimately have no standalone graph node; local reads name their parameter node.

The Arc destination remains alive while indexed, preventing address reuse. OnceLock publishes immutable metadata; shared exact clones cannot overwrite a different key map. PartialEq and Debug continue to inspect nodes rather than lowering-state metadata. Flat Vec storage preserves nonrecursive clone/drop behavior. CheckedBody::relocate changes source sites without using them as identity, and the test exercises identical relocated locations.

All four new tests were inspected for oracle strength. The AC-62 test parses actual source and compares accessor values with function-body and application-target keys from the real SemanticGraph; it checks shared additions, distinct locations and the different multiplication. The recursive test reaches an admitted Call application and real callee key. The inline/local control checks genuine absence and the graph's parameter node. The conflict control first establishes a different key from a second graph package before asserting refusal and preservation of the original key; changing only SourceOwner labels correctly does not imply a new structural key.

Coder evidence was read: formatting, test-inclusive Clippy and all 77 lowering tests pass in /tmp/l5-nodekey-final-focused-{fmt,lint,tests}.log; timing records acquisition and exit codes. The reviewer directly reran the existing unit binary under the thermal lock: all four new tests pass, exit 0, in /tmp/l5-nodekey-accessor-review/focused-tests.log. No Cargo, compilation, or aggregate gate was invoked by this reviewer.

Plan completion: not assessed

## Examined units

- FR-093-AC-62, examined: "For `function w using v(a: Integer, b: Integer): Integer pure { (a + b) * (a + b) }`, the checked unit's accessor returns for the multiplication the `NodeKey` of its `expression` node in the checked graph, and for both `a + b` operations the one `NodeKey` of their shared `expression` node, with two different `Location`s; a function whose body holds `a * b` returns a key different from all three."
- NFR-011, examined: "If the next checking charge would exceed a selected checking ceiling, then the checker shall stop with a stage limit, reported as `stage_limit_exceeded` with the ceiling's kind and its selected bound, without performing that charge."
- FR-258, context_only: checked expressions and lowered bodies retain explicit heap traversal without a fixed nesting cap.

## Review Data

```json
{
  "clean": false,
  "scope": [
    {
      "id": "FR-093-AC-62",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "examined",
      "excerpt": "For `function w using v(a: Integer, b: Integer): Integer pure { (a + b) * (a + b) }`, the checked unit's accessor returns for the multiplication the `NodeKey` of its `expression` node in the checked graph, and for both `a + b` operations the one `NodeKey` of their shared `expression` node, with two different `Location`s; a function whose body holds `a * b` returns a key different from all three."
    },
    {
      "id": "NFR-011",
      "path": "spec/non-functional/NFR-011-bound-value-checking-work.md",
      "role": "examined",
      "excerpt": "If the next checking charge would exceed a selected checking ceiling, then the\nchecker shall stop with a stage limit, reported as `stage_limit_exceeded`\nwith the ceiling's kind and its selected bound, without performing that\ncharge. The ceilings are stage limits, not a caller work budget\n(`quire.native.diagnostics/v1` `stage_limit_exceeded` row; [FR-096](../functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md))."
    },
    {
      "id": "FR-258",
      "path": "spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md",
      "role": "context_only",
      "excerpt": "The S3 checker SHALL type, check obligations for, lower and key expressions\nand types of any depth, bounded only by the checking node limit, the\nper-declaration preimage byte limit and the checking work budget (ADR-030\nD-4.3, NFR-011). QSpec FR-146 defines no expression depth or nesting limit\nfor checking; this requirement carries that rule through QSL's checker."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "code-bug",
      "artifact_id": "qsl-semantics/src/check/lowering.rs::settle",
      "related_ids": [
        "FR-093-AC-62",
        "NFR-011"
      ],
      "path": "qsl-semantics/src/check/lowering.rs",
      "lines": "3912-3917",
      "excerpt": "for body in self.checked_body_keys.values_mut() {\n            for (_, node) in &mut body.nodes {\n                if let Some(node) = node {\n                    *node = resolve(node).map_err(|()| unresolved(&body.location))?;\n                }\n            }\n        }",
      "finding": "Every function_group calls settle, and the added settlement loop visits and clones every previously retained reference, including already-admitted keys. For N independent functions with m retained reference occurrences each, this performs m*N*(N+1)/2 visits instead of visiting new or pending references once. This unmetered, uncancellable metadata rescan adds quadratic checking work for large legal packages. Track/drain only references requiring this settlement (or use an equivalent delta), preserve recursive resolution, and add a deterministic work-count/scaling control rather than a timing threshold."
    }
  ],
  "bindings": [
    {
      "test_id": "checked_operation_keys_are_the_graphs_shared_structural_identities",
      "ac_id": "FR-093-AC-62",
      "trace": "correct"
    },
    {
      "test_id": "checked_operation_keys_resolve_recursive_call_drafts",
      "ac_id": "FR-093-AC-62",
      "trace": "correct"
    },
    {
      "test_id": "inline_literals_have_no_expression_key_and_locals_name_their_parameter",
      "ac_id": "FR-093-AC-62",
      "trace": "correct"
    },
    {
      "test_id": "a_cloned_body_cannot_silently_reuse_keys_from_another_package",
      "ac_id": "FR-093-AC-62",
      "trace": "correct"
    }
  ]
}
```
