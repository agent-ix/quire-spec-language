---
id: SR-12047
title: "QSL-818 PR #922 gap analysis: FR-367 and TC-1053"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@efddc9649d53fa4849d7dcbe7056db8b8c272cd8; qsl-replay/src/spine/bind.rs, qsl-replay/src/spine/bind/tests.rs, spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md, spec/test-cases/TC-1053-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md"
review_set: subset
---

## Summary

Ticket: QSL-818. PR quire-spec-language#922. This is a planless gap analysis of FR-367-AC-1 to AC-10 against TC-1053's steps and the 11 tagged tests in `qsl-replay/src/spine/bind/tests.rs`. Plan completion: not assessed. `quoin matrix` reports all ten FR-367 criteria `tagged`.

The author recorded these gaps, which are not re-reported here:

- AC-6's second half diverges from the spec; the planner will reconcile it.
- AC-4's foreign-model-selection and foreign-type cases are untested.
- AC-8's undefined half is untested.
- AC-10's quantifier-bound case is untested.
- The signature adds `objects` and `cancel`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-367-AC-8 `value_occurrences` clause ("returns `Incomplete(CallIncomplete)` naming that limit, as `run` does") has a test whose oracle is the implementation's own hand-built record (`limit 0`, `consumed 0`, `next_charge 1`). The test never runs `run` on the same call. `run` would complete that call, because FR-100 charges no argument occurrences. So the test passes on behaviour the AC rules out. The AC-3 and AC-4 tests do compare against `run`. See SR-12046 FND-002 for the code side. | qsl-replay/src/spine/bind/tests.rs:774-784 |

## Verdict

FIX, because of FND-001, which goes together with SR-12046 FND-002.

The other oracles are strong:

- **AC-1:** exact `BoundOperand` equality, including provenance indices, nodes, levels, ranges, `package_id` and the swapped `b - a`.
- **AC-2:** literal graph node, range `(1, 1)`, `a - a` at two ordinals of one index, unary `-a`. The `{ node: None }` case is checked through `Resolver::bind` on a hand-built Contract IR operand, because source never yields one.
- **AC-3 and AC-4:** equality with `run`'s refusal, compared through `Debug`.
- **AC-5:** every cause, plus a shared-parameter-node function bound only under its own name.
- **AC-7:** `i64::MAX` binds, `MAX + 1` refuses, and a literal past `i64` refuses at ordinal 1.
- **AC-9:** `check_operands` accepts the binding as the FR-357 operator claim's operand list. Its values equality is tautological, because the claim is built from the binding, but position order is checked against the preimage.
- **AC-10:** the exact name and level.

The AC-6 `OperandNotParameter` sub-test feeds the outer application's own node as its operand child, not the inner `a + 1`. It still exercises the application-node branch.
