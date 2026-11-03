---
id: SR-2026110216379
title: "Gap analysis — QSL-1523 model admission limits"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-506, FR-255, FR-259, TC-1301; qsl-foundation/, qsl-semantics/, qsl-replay/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-506", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-255", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-259", type: references }
---

## Summary

Audited the changed model-admission behavior and its FR-506/FR-255 trace evidence against FR-259 and TC-1301. Quoin/Quire matrix computation did not complete within the command timeout, so the coverage assessment below is a manual acceptance-to-test map; AC-5's identity-byte part remains a concrete gap and a specification conflict.

## Verdict

**FAIL** — AC-5's identity-input-bytes subpart has no corresponding test or model-admission behavior, and the requested structural preimage conflicts with FR-259.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-506-AC-5's identity-preimage requirement is untested and conflicts with FR-259's retirement of structural node/package preimages | FR-506-AC-5 | wrong-requirement |
| FND-002 | medium | FR-506-AC-2's nested option-of-sequence maximum case is not constructed by its tagged test | qsl-semantics/tests/it/model_admission_limits.rs:187 | correct-requirement-no-evidence |

## Coverage

- Reconciliation: manual acceptance-to-test map; `quoin matrix --repo . --json` and `quire matrix --scope . --format json` did not complete before timeout (Quire emitted standard catalog metadata warnings).
- Plan completion: not assessed
- Criteria: FR-506-AC-1 covered by `declaration_records_charge_each_type_field_operation_parameter_result_and_frame_entry` (exact 11, 10, 0); AC-2 covered for direct field/parameter/result and unused field maxima at 10,000/10,001, raised limit, zero, and 3/4, but not nested option-of-sequence; AC-3 covered by zero and 3/4; AC-4 covered at text maxLength 1,048,576/1,048,577; AC-5 document bytes covered exact/one-short/zero and s3.nodes covered by threshold bisection/zero, identity.input_bytes preimage subpart untested and contradictory under FR-259; AC-6 covered by 200 nested option/sequence levels; AC-7 covered through builder, replay stage_limits, and settings operand; AC-8 covered at u32::MAX and rejected operand u32::MAX+1.
- Focused model-admission tests: 5 passed; focused replay `s3.nodes` threshold test: 1 passed.
- Untraced behaviors / stubs: none identified in the changed production paths. `#[trace]` tags are present, but the tagged AC-5 test does not cover all its subparts.
- Semantic review: manual intent-to-test-to-code comparison over FR-506-AC-1..8; optional repository-wide semantic review not run.
- FR-255 mapping: the new setting is present in `IntakeLimits`, the model limits projection and the operand/stage-limits path; test covers setting entry points and representable-width rejection.

## Dispositions

| FND | Outcome | Reason / after excerpt |
| --- | --- | --- |
| FND-001 | deferred | QSL-1571 owns reconciling FR-506 and FR-259: define a contract-owned identity boundary or remove the model identity-preimage subcase, specify which existing identity input exercises the limit, and update TC-1301 as needed. It forbids a duplicate encoder or structural node-key byte preimages. The original gap remains open pending that change and evidence. |
| FND-002 | fixed | The maximum-case helper now includes `("a nested option of a sequence", bytes(&nested))`; the test checks `documents_with_maximum(10_000)` at default, `documents_with_maximum(10_001)` at default refusal, and `run(&over, with_sequence_maximum(10_001))` for raised-limit admission. |

### Round 2

**Disposition result: PASS** — FND-001 is now fixed by the merged FR-506/TC-1301 correction; FND-002 remains fixed from round 1. The rebase introduced no new findings.

The focused `model_admission_limits::a_declared_collection_maximum_is_bounded_by_the_raisable_sequence_maximum` test passed on the rebased head (1/1).

| FND | Outcome | Reason / after excerpt |
| --- | --- | --- |
| FND-001 | fixed | FR-506-AC-5 now distinguishes model limits from identity-byte limits; TC-1301 row 11 covers only the reachable `s3.nodes` model boundary. |

After excerpt (exact FR-506-AC-5 suffix and TC-1301 row 11):

> The structural model keys have no `identity.input_bytes` preimage; identity-byte boundaries are tested by requirements that produce actual identity inputs, as FR-259 specifies.
>
> | 11 | `s3.nodes` at exact usage, one node over, and 0 | admitted, refused naming `s3.nodes`, refused; identity-byte boundaries are covered at actual identity-producing inputs under FR-259, not model structural keys |
