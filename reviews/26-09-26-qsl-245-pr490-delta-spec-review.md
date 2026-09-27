---
id: SR-741
title: "PR 490 fix-round delta spec review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@94e49221c3de22bd167de06a2e02e195efaf75c7; delta 985e0d01..94e49221; spec/spec.md; FR-093; FR-096; qsl-eval/tests/it/kernel_refusal_payloads.rs; qsl-eval/tests/it/collection_queries.rs; qsl-semantics/tests/it/quantities.rs; qsl-replay/src/spine/call/tests.rs; src/command/output.rs; qsl-foundation/src/diagnostic/stage.rs; reviews/26-09-26-qsl-245-pr490-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-245. PR: quire-spec-language#490. This covers the fix round only, 985e0d01..94e49221.

The new tests are sound:

- **`kernel_refusal_payloads.rs`:**
  - Each refusal is raised through its real kernel entry point: `convert_ieee_width`, `evaluate_ieee`, `exact_to_ieee`, `ieee_to_exact`, `modulo`, `divide` over every `DivisionProfile`, integer and rational arithmetic, `evaluate_decimal`, `placement` and `admit_text`.
  - Each full record is checked.
  - The division operands give exactly the claimed cause for every profile: 7/2 fails only the quotient in `Int[0, 1]`, 7/4 only the remainder, and 15/4 both.
- **Empty sum:** the empty-sum test reaches the path under `CheckMode::Kernel`. Disabling the check makes it fail.
- **Mutation re-check:** five mutants were re-run in a scratch worktree, and all were killed. They were the NaN swap, the Binary32 hard-wire, modulo `Int[0, 0]`, Coerce `Int[0, 0]` and the empty-sum check disabled. The unmutated tree passes `qsl-eval --test it`, the replay step-4 test and the CLI reasons test.
- **Payload-blind sites:** 38 `{ .. }` payload matches remain in tests. None of them matters. Every raise site now has a dedicated payload test. The quantity decimal-target path reuses the `DecimalType` raise sites that `decimal_refusals_name_the_declared_target` covers.
- **Review files:** the three committed review files are byte-identical to the posted SR-738, SR-739 and SR-740 artifacts.

## Verdict

**Approve.** Only one low wording finding is new.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The reworded FR-096 row in spec.md now states the code/cause fact twice: "all twelve record-building kernel refusals return the key table's code and cause, `Refusal::code()`/`cause()` return the key table's code and cause". Drop one of the two clauses. | spec/spec.md:510 |
