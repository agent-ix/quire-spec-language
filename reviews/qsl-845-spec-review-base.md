---
id: SR-12107
title: "Spec review of quire-spec-language PR #946: evaluate a protocol node's expression from a checked package"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@28b1448d23bdfc817a73a1dca3ed7c0b4c627308; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md, spec/functional/FR-107-evaluate-state-clauses-at-s6a.md, spec/functional/FR-109-run-a-state-clause-through-the-spine.md, spec/spec.md, spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md, spec/test-cases/TC-1071-refuse-a-protocol-slot-selection-or-supply-before-any-charge.md, spec/test-cases/TC-1072-stop-a-protocol-slot-on-an-unavailable-source-or-an-undefined-capture.md, spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md, spec/test-cases/TC-1074-keep-an-unordered-capture-graph-out-of-evaluation.md"
review_set: subset
---
# Spec review of quire-spec-language PR #946

## Summary

Ticket: QSL-845 (SP-D under QSL-839). One pass of spec-review over `git diff
origin/main...HEAD`, with the integrity, failure-domain and scope lenses
folded in. Spec-only PR, so no code review. Major issues only, at most six.

Oracles: QSpec FR-464, FR-465, FR-466 (slots, stages, binders-only slots,
compensation layout, reader order), QSpec FR-272 (catalog pairs); QSL
FR-381, FR-218, TC-1057 (fixture R), FR-107, FR-255, FR-106, FR-100, FR-093,
FR-094; FR-383 on open PR #945 (branch `s4-protocol-emission-spec`); the
native behaviour in `src/state/evaluation.rs` (`evaluate_v2`),
`tests/it/compiled_protocol_v2.rs` (TC-142) and the evaluate half of
`tests/it/native_compensation_emission.rs` (TC-134).

Examined:
- FR-390 Description, Inputs, slot table, Outputs, Behavior (arm, API contract, charge points, admission checks 1 to 7, Evaluation, shared accounting), AC-1 to AC-14 (examined)
- TC-1070 fixture E and rows 1 to 25 (examined)
- TC-1071 rows 1 to 23 (examined)
- TC-1072 fixture U and rows 1 to 8 (examined)
- TC-1073 rows 1 to 13 (examined)
- TC-1074 rows 1 to 7 (examined)
- FR-107 `ProtocolClause` arm pointer; FR-109 Dependencies pointer; spec.md `contains` edge and index row (examined)
- FR-381 stage table and capture rules, FR-218 decision facts, TC-1057 fixture R (context_only)
- FR-383 (PR #945) Outputs, Binders and Reading the nodes back, AC-19 (context_only)
- QSpec FR-464 Binders and Slots that bar model state, FR-466 control layouts and Compensation (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Captures are not frozen. FR-390 recomputes every capture lazily from the call's one supplied observation, but QSpec FR-466 evaluates registration captures at registration and activation captures once at activation, and FR-218 calls a protocol capture a frozen value. In fixture E `target = forward.total` and `order = view.total` dereference references into the call's observation, so the retry relation or recovery predicate evaluated at a later attempt reads the current total, not the registered one. One `observation` also cannot hold both the registration-time `forward` and the retry-time `earlier`/`later`. State what freezes a capture (the caller supplies a capture's value from a previous call, or the binders are record values, not references), and add a row where the total changes between registration and retry. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:91-100,255-260; spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md:24-41 |
| FND-002 | high | Fixture E cannot pass the QSpec reader, so the read-back rows cannot hold. Its retry binders and attempt record type are the object type `M::Node` (TC-1057 R), but QSpec FR-466 requires `attempts.record_type` to be a `record_value_type` or `event_type` node (an object type is `model`/`object_type`, FR-094). Its trigger guard, retry and recovery read fields of object references, which FR-093 lowers through `quire.op.model.deref`, and QSpec FR-464 refuses that operation in those slots (`ill_typed`/`operator-ineligible`). So FR-390-AC-12 and TC-1073 row 12 (equal results over the read-back package) and TC-1074 row 6 (the unmutated package reads back) fail. TC-1074 rows 4 and 5 hit the reader's step-4 compensation refusal before the expected `definition-cycle` and `missing-name`. FR-390 also charges `model.deref`/`model.navigate` in slots that QSpec makes binders-only. Rewrite E with record-typed binders and record values, or narrow the read-back claims. Drop the conditional "once the reader's protocol step exists" text in FR-390 Behavior and TC-1073 row 12. | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md:22-60; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:161-164,309; spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md:55; spec/test-cases/TC-1074-keep-an-unordered-capture-graph-out-of-evaluation.md:31-33 |
| FND-003 | high | `prepared_for` is a pin. Admission check 1 refuses `stale_dependency`/`content-mismatch` when an optional caller-declared `package_id` differs from the package's own. It is new: the native code has no package-id check (TC-142's crossed axis was per-binder authority). Nothing breaks without it, because checks 4 to 7 already admit every value against this package's own binders, types and observation. It is a version-tracking assertion, the hash/pin antipattern the repository CLAUDE.md asks to remove. Delete check 1, the `prepared_for` field, FR-390-AC-7 and TC-1071 rows 8 and 9. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:99-100,195-197,304; spec/test-cases/TC-1071-refuse-a-protocol-slot-selection-or-supply-before-any-charge.md:41-42 |
| FND-004 | medium | Port loss: the activated variant of the second template is gone. TC-142 asserts, for both compensations, that the retry relation is false when the trigger is ready. TC-1070 has that only for `Refund` (row 9). No `Partial` row supplies `partialTrigger = on`, so an evaluator that ignores `not partialActivated`, or reads `Refund`'s capture, passes rows 15 to 17. Add `Partial.retry.relation` and `Partial.recovery.predicate` with `partialTrigger = on`, expecting false and `captures()` `partialTarget`, `partialActivated`. | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md:84,90-92 |
| FND-005 | medium | The slot table does not cover every QSpec expression term and does not say which ones it leaves out. QSpec FR-466 also holds a `repeat`'s `proof.invariant` (a Boolean term) and `proof.variant`, the `from`/`to` end terms of the `related` records of `send`, `receive`, `attempt`, `effect` and `event`, and the `visible` expression entries of `choice` and `repeat`. FR-390's Description says it evaluates "one expression of one protocol node", and admission check 3 refuses these members as `missing-name`. A runtime that checks a loop invariant or a `related by` end has no API. Add them as slots, or state that they are excluded and why. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:53-57,103-121 |
| FND-006 | medium | FR-390-AC-10 says "a meter at exactly `U` on each charged counter completes", but TC-1073 bounds only `work_units` (row 2) and `result_units` (row 5), each alone. TC-142 set every dimension to its usage at once. No row can fail when another charged counter (for example `integer_bits` or `value_occurrences`) is under- or over-charged. Add a row with every counter at its measured usage, or narrow AC-10 to the two counters. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:307; spec/test-cases/TC-1073-account-protocol-slot-evaluation-on-one-shared-meter.md:45,48 |

## Verdict

**FIX: 3 high (FND-001 to FND-003), 3 medium (FND-004 to FND-006).**

Clean, examined:
- Catalog pairs: every pair FR-390 uses exists in QSpec FR-272 with the meaning used: `stale_dependency`/`content-mismatch`, `missing_declaration`/`missing-name`, `invalid_model_binding`/`malformed-declaration` and `wrong-model-selection`, `invalid_runtime_input`/`duplicate-member`, `unknown-member`, `missing-member`, `wrong-value-kind`, `invalid-value`, `foreign_reference`/`foreign-universe`, `unavailable_observation`/`missing-required-artifact`, `incomplete_population`/`incomplete-scope`, `dangling_reference`/`absent-target-in-complete-population`, `resource_exhausted`/`insufficient-next-charge`, `invalid_package`/`definition-cycle`. `precondition-false` is the family reason of FR-301.
- Slots: the table covers QSpec FR-464's activation guard, clause captures, role `until`, channel key, the five compensation stages, the case and `repeat` guards, `check`, the six step constraints, `finish` and `terminal`, each once. The gaps are in FND-005.
- Accounting: the charge points and their order are stated, the TCs assert relations and never absolute counts, and the ten FR-255 counters are named. Recomputed TC-1073 rows 8 to 10: the sum budget completes G, T and V in either order; the sum less one stops V at its last charge, because each call's charge is independent of order. Row 13 is consistent with "admission charges nothing".
- Capture closure: lazy, term-order, once per call, charged at the read and skipped under a short-circuit. This matches FR-381's order rules and QSpec FR-466's stage permissions for the source sets in TC-1070 rows 4, 6, 8, 12 and 21 to 25. FND-001 is about when captures are evaluated, not about these source sets.
- Port: every other TC-142 and TC-134 evaluate-half assertion maps to a TC-1070 to TC-1074 row. The changes are stated: in-process evaluation, `Unavailable` only when read, the reader's `foreign` code, and per-binder authority replaced by keyed sources. A few `Partial` refusal rows narrowed (surplus `partialTarget`, missing `partialTrigger`, `Partial.trigger.guard`); none of them is blocking.
- Cross-file: FR-107 and FR-109 point to FR-390 and add no second arm. FR-390-AC-14 and TC-1074 row 7 keep the one `ProtocolClause` arm. FR-390 depends on FR-383 (PR #945 merges first).
- IDs FR-390 and TC-1070 to TC-1074 are free on main and on every remote branch. spec.md has the `contains` edge and the index row. The text is present tense with no ticket ids, apart from the conditional in FND-002.
- Gates: `quire validate --scope .` on the nine touched files EXIT=0 with no diagnostics on them; `make check-spec-validation` EXIT=0; `tools/check-index-completeness.sh` passes (quire 0.36.2, engine 0.50.2).

Noted, not blocking: FR-049-AC-9 still cites TC-142 and keeps "the version-1 API remains source-compatible". The ticket says this slice re-homes it, and this diff leaves it unchanged. TC-1072 row 6 assumes S3 admits a capture initializer whose dispatched call may be undefined, while FR-381 says "An initializer is total". Confirm that FR-104's definedness obligations do not include a dispatch precondition.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | A capture slot is a call that evaluates the initializer from its stage's supply and returns a frozen value. Later slots take captured values as sources; the evaluator never recomputes one. FR-390 Description, Inputs, Behavior and AC-2, and TC-1070 rows 2 to 28 (including the registered-total-differs rows 23 and 24), are rewritten. |
| FND-002 | fixed | Fixture E is redefined with record-valued binders and a record attempt type; only protocol captures read the observation. The "once the reader's protocol step exists" hedge is removed. TC-1074 rows 4 and 5 mutate an initializer to read its own binder or another template's capture binder, and expect the reader's `missing-name` with `read` `forward` or `foreign`. A capture cannot form a definition cycle, so the cycle claim is removed. |
| FND-003 | fixed | `prepared_for`, its admission check, FR-390-AC-7 and TC-1071 rows 8 and 9 are deleted; the checks and ACs are renumbered. |
| FND-004 | fixed | TC-1070 rows 16 and 19 expect false for `Partial` with `partialActivated = true`, and row 18 for a differing recovered total. |
| FND-005 | fixed | `visible`, `related.from` and `related.to` and the case guards are slots with TC-1070 rows 29 to 32 over fixture L. `proof.invariant` and `proof.variant` are stated as not slots: S3 discharges the proof and a termination claim is their only reader (FR-218). |
| FND-006 | fixed | FR-390-AC-9 and TC-1073 rows 6, 7 and 9 set every charged counter at its usage and each in turn one below. |
| Noted: FR-049-AC-9 | no change | FR-390 does not contradict it: it describes the native path, which retires. |
| Noted: TC-1072 row 6 | fixed | A dispatched call's false precondition is not a FR-104 definedness obligation (FR-301), so S3 admits the initializer; FR-390 now states this beside the totality rule. |
