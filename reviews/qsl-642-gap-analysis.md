---
id: SR-1362
title: "Gap analysis of quire-spec-language PR #655: i128 integer bounds and literals end to end (QSL-642)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@90bc007a7d5a66e7ca1122e8cf431c4970625d15; PR #655 diff against origin/main: acceptance criteria FR-033-AC-6, FR-056-AC-16, FR-082-AC-9, FR-091-AC-36 to AC-38, FR-092-AC-14, FR-092-AC-15, FR-098-AC-11 against their tagged tests and the production code in the diff"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
---
# Gap analysis of quire-spec-language PR #655

## Summary

Ticket: QSL-642. Planless gap analysis scoped to the PR diff. Plan completion: not assessed. Computed matrix: `quoin matrix --json` (quoin 0.28.1) at 90bc007a7.

Matrix state for the PR's criteria: FR-056-AC-16 (1 binder), FR-082-AC-9 (1), FR-091-AC-36 (1), FR-091-AC-37 (1), FR-091-AC-38 (4), FR-092-AC-14 (2), FR-092-AC-15 (3) and FR-098-AC-11 (2) are tagged. FR-033-AC-6 is untagged; it is planned on IR-662 under the rulings, and TC-912 and native-lowering/tests.md say so.

Per-criterion check of each test against its AC text:

- FR-091-AC-36: `bounds_up_to_i128_resolve_exactly` compiles all four sources through the spine and asserts the exact `ValueType::Int`. Covered.
- FR-091-AC-37: `a_bound_outside_i128_refuses_at_its_type_form` asserts site, value, limit, the covered span (the type form), the Assembly stage and `ill_typed/type-mismatch`, for both limits. Covered.
- FR-091-AC-38: the spine tests compile real source. They cover the fold with plain, spaced and commented layout (no `Negate`, the literal keys to L7), `-(2^127 - 1)` staying a `Negate`, and the four refusals at the literal span (`2^127`, `-(2^127)`, `- (2^127)`, `-(2^127+1)`). The lowering-level tests add L7 byte identity and both limits. Covered.
- FR-092-AC-14: T13 to T16 and T4 are asserted byte for byte, both as type nodes and as parameters in one checked unit. Covered.
- FR-092-AC-15: the counter's serializer is tested at 2^53-1, 2^53 and u64::MAX. Member position and group_reference ordinal are each tested at both spellings, and QSpec's two recursion vectors are read from `QSPEC_DIR` at run time, with nothing copied. Covered.
- FR-082-AC-9: admits at 2^63 and refuses `field-domain` at 2^63+1, over `[0, u64::MAX]` and `[0, 2^63]`. Covered.
- FR-098-AC-11: all four replay cases settle `reproduced-without-witness` with `false`, and the `meter_over` model recompiles to its `package_id`. Covered.
- FR-056-AC-16: partly covered. See FND-001.

Examined:
- FR-033-AC-6 (examined; planned on IR-662, per ruling)
- FR-056-AC-16 (examined)
- FR-082-AC-9 (examined)
- FR-091-AC-36, FR-091-AC-37, FR-091-AC-38 (examined)
- FR-092-AC-14, FR-092-AC-15 (examined)
- FR-098-AC-11 (examined)

No unowned production code: every changed production item traces to FR-091, FR-092, FR-056, FR-082 or FR-098. No stubs, and no tautological asserts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-056-AC-16 has three branches, and its only test (`reads_decimal_string_bounds_up_to_i128`, plus the new refusal rows) exercises one of them: the scalar reader on a pre-built `serde_json::Value`. Two branches have no test. (a) `max` written as the JSON number `18446744073709551615` must refuse `noncanonical_wire`/`inexact-integer` at that number's pointer. That goes through the one-parse, which `read_version_number` bypasses. (b) An admitted package's `Wide` must resolve to `Int[0, 18446744073709551615]` (and `Int[0, 9223372036854775808]`). The test asserts only `ScalarTypeRecord.upper`, not the resolved value type. Add a package-level test that admits the document through intake, asserts the inexact-integer refusal and its pointer for the JSON-number bound, and asserts the resolved `Int` for the string bounds. | qsl-semantics/src/model/intake.rs:4688-4713 |

## Verdict

Every criterion the PR implements is tagged and asserted except FR-056-AC-16, whose JSON-number branch and resolved-type branch have no test (FND-001). FR-033-AC-6 is planned on IR-662 under the rulings. Mergeable once FND-001 is fixed in this PR.

## Dispositions

Round 1, reviewed at fe7a43674bff9296046a1222336d6a8606839e5b.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | Part (a) is fixed in 73f15d363 and fe7a43674. `a_wide_bound_is_a_decimal_string_and_a_wide_json_number_is_inexact` parses a real package document and asserts `noncanonical_wire`/`inexact-integer` at `/types/0/constraints/1/operands/value`. Part (b) is tested at reader level through the one parse plus `read_nodes`, and asserts the exact `ScalarTypeRecord` `[0, u64::MAX]`. Full admission via `read_records` waits on the FCD semantic-ir validator rejecting a string min/max operand, routed upstream as AGE-2228 (Backlog). Until then string bounds do not admit end to end. |
