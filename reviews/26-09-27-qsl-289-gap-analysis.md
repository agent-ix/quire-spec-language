---
id: SR-753
title: "QSL-289 gap analysis of PR 498 (FR-056 value-type reader; FR-103-AC-1, AC-3; TC-458)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ebcc62d94065581527c7a13a14ebe462afed7f68; qsl-semantics/src/model/intake.rs; spec/functional/FR-056-admit-domain-package-model-declarations.md; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md; spec/functional/FR-104-check-state-clauses.md; spec/test-cases/TC-458-spine-admits-model-operations-and-frames.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md; spec/tests.md; qsl-semantics/tests/it/model_operations.rs; qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-458
    type: reviews
---
## Summary

Ticket: QSL-289. PR: quire-spec-language#498 at ebcc62d9.

The ticket's done condition is "model intake reads the bound-scalar value
type, and TC-458 runs with `Int[0, 1000]`". The PR delivers the first half as
a reader with two unit tests that call `read_value_type` directly. It
delivers none of the second half, and says so in its scope note.

Test oracle checks:

- `reads_a_bound_integer_value_type` asserts the whole `ScalarTypeRecord` by
  equality. Swapped bounds (`lower: 1000, upper: 0`), dropped bounds, or a
  swapped `min`/`max` mapping (which would hit the `lower > upper` refusal)
  all fail it.
- The coder's red-check claim is plausible for the function body. With
  `read_value_type` reverted to `unsupported_at`, the first test fails (it
  expects `Ok`), and the table test fails on every `invalid_model_binding` row
  and on the `operations[0]` and `constraints[1]` detail prefixes. Only the
  `scalar: "string"` row would still pass.
- The table's 11 rows reach 10 distinct branches. The empty-constraints row
  and the min-only row both reach the same "requires both" branch with the
  same message.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The ticket's done condition is not met. TC-458 still types `versionNumber` as native `Integer`, and six spec rows still defer to QSL-289 with text that is now false ("FR-056's `value-type/v1` scalar reader, which does not exist yet"): FR-103-AC-1, FR-103-AC-3 (`delta: VersionNumber`), FR-104-AC-1, the TC-459 row in spec/tests.md, and TC-465 rows 20 and 30 (`versionNumber` `-1` and `1001` refusing `invalid-value`). The module docs of tests/it/model_operations.rs and state_clauses.rs also say no reader exists. Merging as-is closes QSL-289 with those rows still Unverified and no ticket owning them. Either add at least one TC-458 test that admits a `VersionNumber` value type through `admit_unit` and `assemble` and asserts `Int[0, 1000]` on the field and the `delta` parameter (a second fixture, leaving the shared `ConfigVersion` fixture alone if its blast radius is the concern), and update the stale prose; or file a follow-up ticket for TC-458/TC-459/TC-465 rows 20 and 30, repoint every "QSL-289" deferral to it, and correct "does not exist yet". | spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:96; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:98; spec/functional/FR-104-check-state-clauses.md:233; spec/tests.md:241; spec/test-cases/TC-465-admission-refuses-each-input-defect.md:48; spec/test-cases/TC-465-admission-refuses-each-input-defect.md:58; qsl-semantics/tests/it/model_operations.rs:11-31 |
| FND-002 | medium | The dispatch wiring has no test. Both new tests call `read_value_type` directly, and no test or fixture in the crate declares a `value-type/v1` construct through `read_type_node`. Deleting the new `meaning::VALUE_TYPE` arm makes the type fall back to the generic `unsupported_at` arm, and every test stays green. The coder's red-check covered the function body, not the wiring. One intake-level test (a document whose `constructs[]` binds a kind to `value-type/v1`) closes this, and the FND-001 TC-458 test would close it too. | qsl-semantics/src/model/intake.rs:2232-2236; qsl-semantics/src/model/intake.rs:4109-4116 |
| FND-003 | low | Weak rows in `refuses_a_malformed_or_unsupported_bound_scalar`. (a) The `scalar: "string"`, `exclusiveMax` and `operations` rows assert only the location prefix (`"$.types[0]:"`), not the `what` payload, so the `scalar` row passes for any `unsupported_at` at the node, including the pre-fix behaviour. (b) The empty-constraints and min-only rows reach the same branch. No row has `max` without `min`, so an implementation that checks only `upper` and defaults `lower` to 0 passes the whole table. (c) No row has `operands` missing or not an object, or a duplicate `max`. | qsl-semantics/src/model/intake.rs:4141-4222 |
| FND-004 | low | Traceability. Both new tests are tagged `TC-458`/`FR-103-AC-1`, but they are FR-056 reader unit tests, and TC-458 and FR-103-AC-1 still say the bound-scalar half is Unverified. The tags claim coverage the spec itself denies. Tag them with an FR-056 AC (the PR body cites `read_record_value_type`'s `TC-146`/`FR-056-AC-3` precedent, but these tests do not follow it), or keep the TC-458 tag only on a real TC-458 spine test (FND-001). | qsl-semantics/src/model/intake.rs:4122; qsl-semantics/src/model/intake.rs:4139 |

## Verdict

Request changes. FND-001 blocks: the ticket's own done condition is unmet, and
the spec keeps six "does not exist yet" deferrals pointing at a ticket this PR
would close. FND-002 should be fixed in the same round. FND-003 and FND-004
are small.
