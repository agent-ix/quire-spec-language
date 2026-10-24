---
id: SR-8727
title: "Cross-reference and cross-repo dependency review of FR-360 (QSL-738)"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@0e8fe78c290748d5e81ba2318514920aa846a9af; spec/functional/FR-360-*.md (relationships, Cross-repo requirements, Dependencies); QSpec FR-141, FR-144, FR-208, FR-322 and the shared grammar read at origin/main"
review_set: subset
date: 2026-10-13
---
# Cross-reference and cross-repo dependency review of FR-360 (QSL-738)

## Summary

Ticket: QSL-738. PR: quire-spec-language#809 (spec only). Cross-reference check of FR-360's citations and three cross-repo requirements against QSpec (read-only, cited by id), quire-exact FR-376/FR-377, ADR-012 section 16, FR-070, FR-106, FR-317 and FR-359.

## Scope examined

- `FR-360 cross-repo 1` (examined)
- `FR-360 cross-repo 2` (examined)
- `FR-360 cross-repo 3` (examined)
- `FR-360 item 4` (examined)
- `FR-360 item 10` (examined)
- `FR-360 item 9` (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Cross-repo requirement 1 asks for only a step-2 row. QSpec FR-322 step 4's element-type table also needs a variant-type row, and QSpec FR-141 also needs a declaration identity for a model-owned enum. Today a typeRef outside the step 4 table 'gives the member no type' and refuses `ill_typed`/`operator-ineligible`. QSpec FR-141 defines enum declaration identity only as a `scalar_type`/`enum` node's preimage, and member identity over that node. Requirement 1 must also name the step 4 row (element type and node) and the FR-141 change for an `enum_value` whose `semantic_type` is a `model`/`variant_type` node. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:172-176; QSpec FR-322 'Model-owned members' step 4; QSpec FR-141 |
| FND-002 | low | Cross-repo requirement 3 asks QSpec's shared grammar for a three-segment production that already exists syntactically. The QSpec gap is the resolution rule. The shared grammar already has `qualified-name = ident, { '::', member-name }`. QSL's parser already parses `Model::Enum::Variant` as `ExprKind::EnumValue` for native model enums, and the linker resolves it. FR-317 resolves only `q::m`. Restate requirement 3 as a resolution rule for a model-alias-qualified enum member, and cite the existing native-model form as precedent in item 4. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:183-185; src/parser/expression.rs `ExprKind::EnumValue`; QSpec shared grammar `qualified-name` |
| FND-003 | medium | FR-360 supersedes a rule that QSpec FR-208 states today: a variant `typeRef` is `malformed-declaration` and a clause naming one is `unsupported_construct`. No QSpec ticket tracks the change. Until QSpec changes, QSL and the shared semantics owner contradict each other, and the QSpec conformance vectors (TC-235 rows) still expect the refusal. Record a Linear ticket id for each cross-repo requirement in FR-360 (as `Remaining work:`) so the divergence has an owner. | spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md:169-182 |

## Verdict

Findings. The citations to FR-070's enum shape, FR-106's record form, ADR-012 section 16.2 ('no `case` over an enum') and QSpec FR-141-AC-2/AC-5 are accurate. Cross-repo requirement 1 is incomplete, requirement 3 asks for something that already exists, and the QSpec divergence has no tracking ticket.

## Dispositions

Round 1, reviewed at df6246c9d97bdd68697696aa6701a527ab142714.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3d7e167f0 |
| FND-002 | fixed | 3d7e167f0 |
| FND-003 | fixed | df6246c9d |
