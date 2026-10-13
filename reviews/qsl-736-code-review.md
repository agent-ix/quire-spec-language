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

## New findings (disposition pass 1)

Reviewed at agent-ix/quire-spec-language@77a645298 (fix commit 4d84820a5 plus a merge of `impl/business-model-meanings` that adds the M19 paths and makes M07 run 3 name the process).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | One reader site still puts the field only in `detail`. The bare-string-kind refusal in `read_domain_package` (`ctx.malformed("kind: a bare-string core kind ...")`) gives `member` `TypeDefinition`, while its sibling at `read_type_node` was converted to `TypeDefinition.kind`. | qsl-semantics/src/model/intake.rs:4165-4167 |
| FND-009 | low | `identity_member` maps every record other than a population or an operation to `TypeDefinition.identity`. A field, clause, state, transition or relationship record whose own key is malformed is then reported at its owning type's identity, not its own (for example `Relationship.identity`, or `TypeDefinition.fields[n].identity`). | qsl-semantics/src/model/admitted.rs:107-114 |
| FND-010 | low | The relationship position is counted among relationships whose *source end type* equals this one's, not among the declaring type's `relationships` array. `read_relationship` does not require the source end to be the declaring type, so a relationship declared on one type with another type's source end gets a position from the wrong array. | qsl-semantics/src/model/normalize.rs:2293-2303 |
| FND-011 | low | `ConflictingBinding.key` is documented as "The key more than one record declares". For a duplicate process step it now holds the process key (M07 run 3), and for a duplicate state or transition it still holds the member key (M06 runs 5-6). The doc no longer describes the field, and one variant gives two meanings to the same field. | qsl-semantics/src/model/refusal.rs:580-586; qsl-semantics/src/model/process.rs:73-83 |
| FND-012 | low | `semantic_member_path` gives a root IR pointer (`/ir` or empty) a single empty `Name`, which `Display`s as `$.`. The type's own `FromStr` rejects that, so the value breaks its round-trip guarantee. JSON-pointer escapes (`~0`, `~1`) are not decoded either. | qsl-semantics/src/model/intake.rs:1890-1915 |
| FND-013 | low | An operation's precondition is spelled two ways. `Operation.pre` is hard-coded for M08 run 6 and for every operation-owned clause record, even a postcondition one. Path-derived refusals inside an operation spell it `TypeDefinition.operations[m].pre[k]...`, which is what #792 (QSL-726, open) will produce for its contract clauses. | qsl-semantics/src/model/normalize.rs:2104-2108; qsl-semantics/src/model/intake.rs:2753-2760 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4d84820a5 |
| FND-002 | fixed | 4d84820a5 |
| FND-003 | fixed | 4d84820a5 (remaining member-record cases are FND-009) |
| FND-004 | fixed | 4d84820a5 (one residual site is FND-008) |
| FND-005 | fixed | 4d84820a5 |
| FND-006 | fixed | 4d84820a5 |
| FND-007 | fixed | 4d84820a5 |
