---
id: SR-1315
title: "Spec review of FR-286 and TC-770 for quire-spec-language PR #638"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5dd6bacc001141ba412f385c0562ae8ee5e6f64d; spec/functional/FR-286-*.md, spec/test-cases/TC-770-*.md, spec/decisions/ADR-029-*.md (LC-1, LC-2, CB-1, CB-4, OP-1), spec/functional/FR-284-*.md, spec/functional/FR-100-*.md (spine-run-result/1), spec/tests.md; QSpec FR-331 and FR-300 (quire-specification main checkout) as context"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-770
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: reviews
---
## Summary

Ticket: QSL-592 (LC4). PR: quire-spec-language#638. The PR changes no spec file.
This review covers the records it implements, where the implementation showed a
defect in the spec.

Checked and clean:
- FR-284: "a renderer other than the JSON serialization of its own outcome
  types" allows this serializer in `qsl-replay`, a core crate. FR-286 says the
  same ("the one renderer a core crate may hold").
- ADR-029 CB-4 and FR-286 list the same seven members. The rule that the
  `undefined` label never appears on a proof item matches the QSL-366 ruling
  recorded in both.
- Ownership of the CLI writer: ADR-029 CB-1 makes the CLI the driver's `quire`
  binary and deletes QSL's `cli`, `main` and `quire-spec` when the driver verbs
  land. FR-286's Overlap gives "`quire --format json` writes this document" to
  the driver repository. QSL's root crate `command` keeps request handling as a
  library operation. So rewiring QSL's `quire-spec` CLI to `quire-outcome/1` is
  not this PR's job.

## Verdict

Changes needed in the spec: 1 high, 4 medium and 1 low finding. The worst is
FR-286's member list, which has nowhere to put an `execute` outcome's result.
So the one outcome document cannot replace FR-100's `run` output.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-286's members have no place for a non-item evaluation outcome's result. `items` is only for prove, analyze and monitor, and `diagnostics` need a catalog code. So an `execute` document cannot carry a completed call's value, an undefined call's reason (`sum-out-of-domain`), or an incomplete call's exhausted limit. FR-100's `spine-run-result/1` carries all three. A driver that writes `quire run` as `quire-outcome/1` would print `category: success` with no value. Fix: give FR-286 a member for the evaluation result of `execute` (and a clause run): its value or its undefined reason, or its limit. Or allow `items` for `execute` with the FR-100 outcome record. Add an AC over FR-100-AC-1 (value 7) and FR-100-AC-10 (reason kept). | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:27-48 |
| FND-002 | medium | FR-286-AC-1 requires the `check` document's `artifacts` to hold "the package's `package_id`". But ADR-029 OP-1 gives `check` the output `Staged<CheckedPackage>`, and `package_id` is minted only by `package` (E4, FR-322 preimage, `EmittedPackage::package_id`). As written, AC-1 forces either the serializer or `check` to run E4. The implementation chose the serializer (SR-1313 FND-002). Fix: move the `package_id` expectation to a `package` outcome AC (`operation` `package`, `last_stage` S4, `artifacts` holding the `package_id`). The `check` success document then has empty `artifacts`. Or state that the caller supplies the identity from `package`. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:75; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:320-325 |
| FND-003 | medium | FR-286 says an item holds "its terminal record" but never says in which spelling. It spells the one cause it names `UndefinedEvaluation{where, cause}`, while QSpec FR-331's wire table spells it `undefined-evaluation{where, cause}`, and spells `replay-parity`, `cancelled{source}` and `limit-reached{…}` in kebab case. With no rule, the code invented a mix of PascalCase and snake spellings (SR-1313 FND-003). Fix: FR-286 states that each item carries its FR-331 terminal record in FR-331's wire spelling, and uses `undefined-evaluation` in its prose and in AC-4. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:37-44, 78 |
| FND-004 | medium | FR-100 (and FR-267 and FR-026, which point to it) still specify `spine-run-result/1` as the document a function run writes. Nothing relates it to `quire-outcome/1`. ADR-029 CB-4 says every verb writes one outcome document, so the spec now names two documents for one `execute` outcome. ADR-029 CB-1 does delete `quire-spec` when the driver verbs land, but FR-100 does not say its document goes too. Fix: FR-100 states that `spine-run-result/1` belongs to `quire-spec` and is deleted with it (CB-1), and that the driver's `run` writes FR-286's document. This depends on FND-001, because until then FR-286 cannot carry what `spine-run-result/1` carries. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:95-110; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:176-187 |
| FND-005 | medium | TC-770's title says "…and the CLI writes it unchanged", but its procedure has no CLI step, and FR-286's Overlap gives the CLI check to the driver repository (QSpec FR-300-AC-3, FR-301-AC-1). spec/tests.md gives TC-770 a different title ("…with the undefined label on non-proof outcomes only"). Fix: retitle TC-770 to match spec/tests.md and its scope, and leave the CLI half to the driver's TC. | spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-and-the-cli-writes.md:3, 8; spec/tests.md:984 |
| FND-006 | low | FR-286 does not say what `last_stage` is when no stage is known: a check cancelled before its first charge, or an `InternalFault`, whose stage is an untyped string. It also does not say whether `items` appears for operations other than prove, analyze and monitor. The code writes `null` and `[]`, which is reasonable but unstated. Fix: state that `last_stage` is `null` when no stage is known, and that `items` is always present, empty for an operation without requested items. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:31-37 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | FR-286's `items` bullet now gives `cancelled{source}` and `limit-reached{limit, value, setting}` as examples of the FR-331 spelling. But QSL's `TerminalValue::Incomplete` carries only `IncompleteCause`, with no source, limit, value or setting, so the serializer writes `{"kind": "cancelled"}` and `{"kind": "limit-reached"}` with no members. The kind spelling is right; the payload the examples imply cannot be written. Fix: drop those two examples, or say their members come with the FR-281/FR-283 outcome types (QSL-596, QSL-597). | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:46-53 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 17619203d (FR-286 and ADR-029 CB-4 gain the `result` member, with completed, undefined and incomplete forms and FR-286-AC-5; the counter wording was settled in 9e42b2c56) |
| FND-002 | fixed | 17619203d (FR-286-AC-1: the `check` document has empty `artifacts`; the `package` outcome carries the `package_id` (ADR-029 OP-1)) |
| FND-003 | fixed | 17619203d (items carry the FR-331 terminal record in FR-331's wire spelling; prose and AC-4 use `undefined-evaluation`) |
| FND-004 | fixed | 17619203d (FR-100, FR-026 and FR-267 say `spine-run-result/1` is the `quire-spec` CLI's current output, retired with it under ADR-029 CB-1, and that the outcome document is FR-286's) |
| FND-005 | fixed | 17619203d (TC-770 retitled to match spec/tests.md; it says the CLI half is the driver's test) |
| FND-006 | fixed | 17619203d (FR-286 states when `last_stage` is `null` and that `items` is always present, in both the members list and Behavior) |
| FND-007 | fixed | 3f0ba08fa (FR-286's items bullet keeps only the `undefined-evaluation{where, cause}` and `replay-parity` examples) |
