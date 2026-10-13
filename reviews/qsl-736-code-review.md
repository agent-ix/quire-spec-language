---
id: SR-7865
title: "QSL-736 PR 796 code review: typed MemberPath on intake refusal causes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1e6ac7d34879ab98616ae75956ed7909c7ff05e6; PR #796 diff against impl/business-model-meanings: qsl-semantics/src/model/refusal.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/admitted.rs, qsl-semantics/tests/it/tc235_business_meanings.rs, qsl-semantics/tests/it/tc235_state_machine.rs, qsl-semantics/tests/it/business_meanings.rs, qsl-semantics/tests/it/model_intake.rs"
review_set: subset
---

## Summary

Ticket: QSL-736. PR quire-spec-language#796, one commit (1e6ac7d3), base
`impl/business-model-meanings`. The rust-review lane is folded into this file.
Reviewer-only: diff and source read at the reviewed sha; no build or test run.

The PR adds `MemberPath` (`MemberRoot` plus `MemberSegment::Name`/`Index`),
with a builder, `Display`, `FromStr` and `from_document_location`, and a
`member: MemberPath` field on `IntakeMalformedDeclaration` and
`UnsupportedDeclarationForm`. The reader fills it from the node's document
location (`$.types[3].fields[0]` -> `TypeDefinition.fields[0]`) or sets it
explicitly (`Operation.pre`, `constructs[].meaning`, `Relationship.target`).
The TC-235 helpers `assert_malformed_at` / `assert_unsupported_at` now compare
the typed value with `==`.

## Verdict

PASS WITH FINDINGS. The type is sound and consistent with its siblings: it
derives `Clone, Debug, Eq, PartialEq`, as `ModelRefusalCause` does, and no
serde or wire projection carries the cause's node/artifact/span either, so no
golden or replay consumer needs the new field. Every TC-235 path the rows name
for these two causes, with single-element arrays (M03, M04, M05 runs 4 and 6,
M06 runs 1-3, M08, M09, M11, M12, M21 runs 3-4), matches what the code
produces. The derived mapping is the same for every field name used
(`fields`, `operations`, `persists`, `members`, `clauses`, `identityFields`,
`supertypes`, `transitions`, `steps`). The defects are where a path is
hard-coded or counted rather than read from the document. Those paths can name
the wrong member once an array has more than one element, and no test uses
more than one.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Record-value and event-type operation refusals always say `TypeDefinition.operations[0]`, but name `operations.first()` after `sort_by(key)`. With operations `[zeta, alpha]`, the refusal names `alpha` (authored position 1) at `operations[0]`, which is `zeta`. The typed path names the wrong member. | qsl-semantics/src/model/normalize.rs:1121-1149; qsl-semantics/src/model/normalize.rs:1183-1209 |
| FND-002 | medium | The relationship source-owner refusal hard-codes `TypeDefinition.relationships[0]` for every refused relationship. A non-object type with two relationships gets two identical refusals (same node `Money`, same path), and the second names the wrong member. | qsl-semantics/src/model/normalize.rs:2216-2234 |
| FND-003 | medium | `identity_refusal` sets `$.identity`, the package document's own `identity` member, for a declaration whose key is under the wrong package or whose node is neither `ix://<pkg>/<id>` nor `<owner>/<name>`. The defect is in the declaration's identity, so the path names the wrong member. | qsl-semantics/src/model/admitted.rs:105-116 |
| FND-004 | medium | Only some reader sites moved to `malformed_in`. About 25 `ctx.malformed("<field>: ...")` calls remain (`kind`, `kind.module`, `sourceEnd`, `targetEnd.type`, `construct.meaning`, `origin`, `term`, `operands.value`, ...). For these, the typed `member` stops at the declaration root (`TypeDefinition`) while `detail` names the member, so the typed value and the detail disagree. An unmapped semantic-IR pointer also falls back to `$`, the package root, which no row means. | qsl-semantics/src/model/intake.rs:2328-2336; qsl-semantics/src/model/intake.rs:3320-3332; qsl-semantics/src/model/intake.rs:3571; qsl-semantics/src/model/intake.rs:1473 |
| FND-005 | low | The path for an operation-owned clause is counted over record order and spelled `Operation.clauses[n]`. `Operation` has no `clauses` member in the spec (it has `pre`/`post`), and record order is arbitrary for a package built from records. `semantic_member_path` also maps every `/constructs/...` pointer to `constructs[].meaning`, whatever the pointer's tail. | qsl-semantics/src/model/normalize.rs:2022-2039; qsl-semantics/src/model/intake.rs:1860 |
| FND-006 | low | `FromStr` is documented as "the inverse of Display" but accepts malformed text without error: `[x]` becomes `Name("x")` and prints as `.x`, an unclosed `[` becomes `Name("")`, and `..` gives an empty name. Only the root is validated. | qsl-semantics/src/model/refusal.rs:156-213 |
| FND-007 | low | The intake unit-test helper `malformed(node, detail)` derives the expected `member` from the detail's `$...` prefix, using the production `from_document_location`. The location-to-path mapping is therefore checked against itself in every intake unit test that uses it. The only independent checks are one unit assertion (`$.types[3].fields[0]`) and the TC-235 integration rows. | qsl-semantics/src/model/intake.rs:7291-7304 |
