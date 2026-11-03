---
id: SR-20261103112802
title: "QSL-1565 r1 gap analysis of PR #2194"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2194 head 103e438d4; FR-049-AC-2, FR-049-AC-3, TC-136; qsl-eval slot_admission/evaluate/mod; qsl-eval/tests/it/equality_matrix.rs; qsl-replay/src/execute/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
---

## Summary

Ticket: QSL-1565. Plan completion: not assessed. This analysis is scoped to the PR diff. It covers FR-049-AC-2 and AC-3 as TC-136 exercises them through the new tests: entry refusal, by-name reads, record, union and map members, and exact-path witnesses. All new and restated tests carry `#[trace("TC-136", "FR-049-AC-2", "FR-049-AC-3")]`, and the trace bindings are correct.

Mutation evidence: the entry check, the by-name lookup, the read-time refusal and `widened=false` are each killed by a test. `widened` forced to `true` survives `qsl-eval --test it` (891) and `--lib`. Every witness in the tests is hand-built in the shape the PR's path model expects. No test drives a witness produced by FR-106 check 6.5 through `call` or `evaluate`, which is how FND-001 of the code review went unseen.

## Verdict

**GAPS.** The tests back the AC on the hand-built shapes. Two behaviours have no failing test: the real FR-106 witness shape, and the un-witnessed refusal of a range-widened carried type. Replay mapping of an entry refusal is also untested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test admits an environment whose witnesses come from FR-106 check 6.5 (`admit_population_values` with `PostStateRange` witnessing) and then calls `call` or `evaluate`. The hand-built witnesses never use the `OptionValue`-prefixed steps FR-106 emits for optional attributes and members, so the valid-environment refusal (code review FND-001) passes the suite. | qsl-eval/tests/it/equality_matrix.rs:3029,3095; qsl-semantics/src/model/observation/document.rs:1868-1882 |
| FND-002 | low | Forcing `widened` to true (accepting `Integer`-widened carried option and collection types without any witness) survives the whole qsl-eval suite. No test shows that an un-witnessed environment refuses a `Seq<Integer>` or `Option<Integer>` value at a `Seq<Int[0, 9]>` or `Option<Int[0, 9]>` attribute. | qsl-eval/src/value/expression/slot_admission.rs:150-153 |
| FND-003 | low | No test covers how replay reports an entry refusal of the object environment (bind, clause, observed window). The one replay test touched asserts the qsl-eval result directly, so the `Admission`-to-internal-fault mapping (code review FND-002) is never exercised. | qsl-replay/src/execute/tests.rs:1720-1750; qsl-replay/src/spine/call.rs:879-884 |

## Dispositions

- FND-001: fixed. `witnessed_environment.rs` drives witnesses produced by FR-106 check 6.5 through `evaluate`, covering optional attribute, optional record member, collection element and nested cases, and checks both acceptance and refusal of an unwitnessed integer.
- FND-002: fixed. `an_unwitnessed_range_widened_carried_type_refuses_at_entry` refuses an in-range `Sequence<Integer>` and `Option<Integer>` at `Int[0, 9]` attributes with no witness, so forcing `widened` true fails it.
- FND-003: fixed. The spine and execute replay tests above assert the typed replay mapping of an entry refusal produced by the real admission.
