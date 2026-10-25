---
id: SR-17224
title: "gap analysis of QSL-1081 capture emission and observation, FR-337-AC-8 to AC-14 and TC-1267 (quire-spec-language PR 1554)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@dc5789a5f3435e744ecea1089f0db734677bc2bc; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1267
    type: reviews
---

# SR-17224: gap analysis of QSL-1081 capture emission and observation (quire-spec-language PR 1554)

## Summary

Ticket: QSL-1081. Plan completion: not assessed. AC to test trace, from the `#[trace("TC-1267", ...)]` tags on the PR head:

- FR-337-AC-8: `s3_checks_and_keeps_each_capture`, `s3_refuses_each_capture_defect_at_its_locus` (11 vectors), `a_repeated_capture_name_refuses_naming_both_declarations` (qsl-package emit tests). Covered; the vectors match the AC text.
- FR-337-AC-9: `the_captures_aggregate_and_nodes_are_emitted`. Covered, including levels, types, occurrences and the I2 reader.
- FR-337-AC-10: `a_capture_changes_the_clause_key` covers the key half. The `requires temporal Due;` half is covered only for binding position (FR-218-AC-16, TC-1056), not for the key (FND-001).
- FR-337-AC-11: three tests in `model_check_captures.rs`. Covered with one substitution: `when (c.value = 7)` is replaced by `when (false)` because S3 refuses `c.value = 7` over an `Int[0, 3]` field (FND-004).
- FR-337-AC-12: `a_holds_read_of_a_capture_is_refused_before_exploration` covers the four refusal vectors and the unread control; the RequiresBound precedence vector is missing (FND-002).
- FR-337-AC-13: violated-plus-replay half covered by `a_clause_with_a_capture_replays_through_the_production_letters` (an `on origin` clause, not the AC's `on each` guard); the FR-338 half by the closure test over an unguarded clause; the undefined-capture replay and the FR-339 half are not covered (FND-003).
- FR-337-AC-14: `a_capture_initializer_joins_the_activation_footprint` asserts a count only (FND-005).

No stub, `todo!`, `unimplemented!` or tautological assertion was found in the new tests; each assertion compares against a computed value (prefixes, codes, tags, offsets, node digests, reachability through the wire graph).

Code with no owning requirement: `CaptureReading`, `evaluate_capture`, `UndefinedLocus::Capture`, `TemporalCause::DuplicateCapture`, `CapturedAt` and `refuse_capture_read` are all owned by FR-337 "Capture observation" and "Captures at S3". The liveness/lasso capture reader has an owner (FR-337) and no test (code review SR-17223 FND-004).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-337-AC-8 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-9 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-10 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-11 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-12 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-13 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-14 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| TC-1267 | spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md | examined |
| FR-218-AC-16 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | context_only |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | TC-1267 step 4 and FR-337-AC-10 require the node the `requirements` set holds for `Due` to be the clause node, with a key equal to the clause key and different with and without captures. No test reads the protocol's requirement node or its key; `a_requirement_binds_a_clause_with_an_activation_and_a_capture` checks position and the absence of a temporal refusal only, and carries no `FR-337-AC-10` tag. | qsl-semantics/src/check/protocol_requirement/tests.rs:198-229 | correct-requirement-no-evidence |
| FND-002 | medium | TC-1267 step 6 and FR-337-AC-12 require `RequiresBound` for a capture-reading clause with an unbounded root; no test runs it, so the stated precedence is unguarded. | qsl-analyze/tests/it/model_check_captures.rs:156-204 | correct-requirement-no-evidence |
| FND-003 | medium | TC-1267 step 7 and FR-337-AC-13 require the undefined `z` to replay to `UndefinedEvaluation` located at `z` and its certificate to be rejected `BadState` by FR-338 and FR-339. Replay refuses every authored `UndefinedEvaluation` (`AuthoredUndefined`, QSL-898 remainder), the FR-338 rejection is tested over a different unguarded clause, and FR-339 is not tested. A ticket for the remainder and an amended AC are needed so the AC is not read as met. | qsl-replay/src/model_check/replay.rs:868-871 | correct-requirement-no-evidence |
| FND-004 | low | Spec vectors that cannot run as written: AC-11 and TC-1267 step 5 use `when (c.value = 7)` over an `Int[0, 3]` field, which S3 refuses; AC-13 pairs a guarded clause (`when (c.value = 7)`) with an FR-338 certificate, but a guarded clause has no closure certificate (`certified = !guarded`). The PR substitutes `when (false)` and an unguarded clause. The spec should be amended to the vectors the tests actually run. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:438-442 | wrong-requirement |

## Verdict

PASS with findings: every AC has at least one real test, none is a stub; FR-337-AC-10 (requirement key), AC-12 (precedence) and AC-13 (undefined replay, FR-339) are only partly covered. Fix FND-001 and FND-002 in this PR and ticket FND-003 with an AC amendment, with FND-004 as the same spec edit.
