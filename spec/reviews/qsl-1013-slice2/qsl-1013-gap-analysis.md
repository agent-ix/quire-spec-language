---
id: SR-16363
title: "Gap analysis of QSL-1013 slice 2 at 77276554c"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-230 (AC-1..5, AC-7, AC-8, AC-32), TC-685 step 26; TC-685-tagged tests in the 77276554c diff; qsl-package/src/emit.rs S4 guard"
review_set: subset
---

## Summary

Ticket: QSL-1013. PR agent-ix/quire-spec-language#1403, reviewed SHA 77276554c518595877ff2728e341865a8bfab905. Planless gap analysis: FR-230 and TC-685 at head (taken unchanged from main by the merge), the trace tags on the slice's tests (`quire trace --id TC-685`: 28 structural claims, 0 citations), and the two `#[ignore]`d end-to-end tests, run explicitly with `--include-ignored`.

Plan completion: not assessed. `quire matrix --format tsv` at head completed in about 4 minutes: FR-230-AC-1..35 are all `untagged`, and AC-36/37 are tagged by qsl-package/src/checked_v2/tests.rs:1781 (from main). The slice claims no FR-230 criterion, which matches its partial scope.

## Verdict

**CHANGES REQUESTED.** The slice tags its tests `TC-685` only, with no `FR-230-AC-n` tags, so no acceptance criterion is counted as covered by partial evidence. That is correct for a prerequisite slice. AC-3 vectors (tick resolution, period 1/2, period 0 → `invalid_model_binding`/`malformed-declaration`, missing operation → `missing-name`), the AC-5 source-unit vectors, the AC-2 duplicate-source and non-Time-unit vectors, and the AC-7 facet-gate refusals for time, invariant and urgent are each exercised by a passing test. Ignored test 2 (`package_time_units_reach_s3_with_their_model_owner`) fails exactly as its reason states (`TimedNotChecked{TimeSource}` at the qualified `Config::ms`) and is tracked by QSL-1031. Leaving it ignored is acceptable. Ignored test 1 does not fail where its reason says (FND-002). One TC-685-tagged test asserts the behavior TC-685 forbids (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_checked_time_source_cannot_emit_as_an_untimed_package` is tagged `#[trace("TC-685")]` and asserts `EmitRefusal::TimedModelNotEmitted`. TC-685 step 26 requires "the S4 path has no `TimedModelNotEmitted` refusal", so the trace counts as TC-685 evidence a test of the opposite behavior. | qsl-package/src/emit/tests/timed_source.rs:7-10, spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:81 |
| FND-002 | medium | The ignore reason on `a_package_clock_field_reaches_the_timed_model_check` ("QSL model intake does not admit the ix://quire/native/Clock field typeRef") is inaccurate for the test as written. Run with `--include-ignored`, it fails earlier, at FCD I1: `agent-ix.semantic-ir.INVALID_CLOCK_FIELD` at `fields[2].multiplicity`, because `clock_document()` writes multiplicity `"required", 1`. QSL-1030 notes the fixture needs `unique:false`. The test also asserts only that S2 assembly succeeds, although its name says it reaches the timed-model check. | qsl-semantics/tests/it/timed_declarations.rs:40-48, qsl-semantics/tests/it/timed_declarations.rs:108-121 |
| FND-003 | low | Both `#[ignore]` reasons omit their tracking tickets (QSL-1030, QSL-1031). `quire trace --id TC-685` lists both ignored tests as `verifies` claims, the same as passing ones. | qsl-semantics/tests/it/timed_declarations.rs:109, qsl-semantics/tests/it/timed_declarations.rs:124 |
