---
id: SR-20261109172912000058
title: "Traceability review of checker-retained expression keys"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1818 / QSL-602; PR #2624; attempted whole-repository computed audit and manual changed-behavior trace/ownership check"
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-093, type: references }
  - { target: ix://agent-ix/quire-spec-language/NFR-011, type: references }
---
## Summary

The changed accessor, retention mechanism and four tests are owned by the current FR-093 operation-key contract. The manual PR-delta trace and oracle check passes. The whole-repository computed matrix and reverse-coverage audit did not complete; this artifact makes no whole-repository coverage or assurance PASS claim.

## Assurance Context

No applicable AssuranceProfile was found in the repository spec tree. FR-093-AC-62 is the current product obligation; the withdrawn AC-10 is not used as authority. FR-258 supplies flat traversal context and NFR-011 supplies work-bound context. No plan was supplied. No profile, exception, assurance control, CI rule, source dependency or verification method is changed by this PR.

## Verdict

CONDITIONAL — manual PR-delta trace/ownership/oracle check passes, but the whole-repository computed audit is NOT COMPLETED. There is no basis to report current whole-repository criterion counts, untracked-symbol counts, baseline equivalence or a full assurance verdict. Separately, code-review SR-20261109172912000057 reports a medium settlement-work defect requiring a fix before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No introduced trace/ownership gap found by the completed manual checks (placeholder). | - |

## Coverage

Plan completion: not assessed

Whole-repository commands were actually attempted from the frozen worktree:

- quire matrix --scope . --format json: stopped by the lead's direction after 521 elapsed seconds and 00:08:46 recorded process CPU time, with zero JSON bytes produced.
- quire coverage --scope . --format json: stopped after 295 elapsed seconds and 00:04:58 recorded process CPU time, with zero JSON bytes produced.

These are incomplete computations, not proof that the tools are unavailable or broken. Only the reviewer's own probe processes were stopped; no other lane's process was interrupted. Exact process observations and commands are in /tmp/l5-nodekey-accessor-review/computed-audit-attempts.txt. The empty output files and diagnostic logs are preserved as matrix.json/matrix.err and coverage.json/coverage.err. Diagnostics report module/archetype duplication warnings, not a completed matrix. No hand-written matrix, invented count or narrowed computed report replaces the missing results.

The completed manual check covers all changed production/test code in check/ir.rs, check/lowering.rs, check/refusal.rs, check/lowering/tests.rs and check/lowering/tests/node_keys.rs. FR-093's statement requires the checker-assigned structural node key beside location; AC-62 owns the exact shared-addition/multiplication source case. The key is retained from actual lowering output and settled through the existing producer mechanism. No local identity algorithm, body-index-as-key, location join, wire vocabulary, copied owner implementation, or unowned leaf accessor is introduced. Inline absence is documented rather than mapped to a false identity.

All four new Rust test functions carry actual trace attributes naming TC-415 and FR-093-AC-62. The source-level AC-62 test asserts the full selected graph/key distinction, using real SemanticGraph function-body and application-target references. The recursive, inline/local and cloned-body ownership tests are supplementary controls on the same accessor. None is ignored, mocked, empty, guarded away or a mere import/type assertion. The reviewer directly reran these four existing unit-binary tests: 4 passed, 0 failed, exit 0 in focused-tests.log. Coder evidence separately records 77 lowering tests, formatting and test-inclusive lint passing.

The whole repository contains substantially more requirements and source than this changed-behavior inspection. Its reverse-gap inventory and computed coverage were not completed in this invocation. Existing general assurance gaps are not declared resolved, and none of the clock-reference feature's other acceptance criteria is credited by these accessor tests.

## Review Data

```json
{
  "clean": true,
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
  "findings": [],
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
  ],
  "computed_audit": {
    "status": "not_completed",
    "matrix_elapsed_seconds": 521,
    "matrix_cpu_time": "00:08:46",
    "coverage_elapsed_seconds": 295,
    "coverage_cpu_time": "00:04:58",
    "json_bytes": 0
  }
}
```
