---
id: SR-9013
title: "Base spec review — QSL-750 model-profile selection and gating (PR #813)"
type: SpecReview
analysis: base
date: 2026-10-13
scope: "agent-ix/quire-spec-language@aefcc4103c331ab54907b1dd60c97f8fb5d96b64; spec/functional/FR-361, FR-102, FR-110, FR-302, FR-304, FR-308, FR-309; spec/test-cases/TC-1012..TC-1017; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-309
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1014
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1016
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1017
    type: reviews
---

## Summary

Ticket: QSL-750. PR #813 (base impl/business-model-meanings) adds FR-361 and TC-1012 to TC-1017, and edits FR-102, FR-110, FR-302, FR-304, FR-308 and FR-309. The review is against QSpec main (shared grammar, model-complete, state-core, complete-value and complete-model locks, native-diagnostics, ADR-006, FR-450, FR-453) and the planner rulings (a) to (f) recorded on QSL-750.

## Verdict

**CHANGES REQUESTED.** One high finding: the `model`-declaration gate (ruling e) makes every state-layer unit unsatisfiable. The PR applies rulings (a) to (f) faithfully otherwise. Clean: the selection definition (item 1: header only, no inference, no fallback to `quire.value.complete/v1`), E3 selects and S3 refuses (items 8 and 9), and the refusal origin. The construct's source span follows FR-110's layer rule and QSpec native-diagnostics' located refusal. FR-154's node-and-member rule locates domain-package IR node refusals at intake and does not apply to source constructs. The retained family, owning declaration and layers cover the native-diagnostics retention set. Public-repo check is clean: no local paths and no private text; the QSpec file names it cites are already cited in FR-110 and in code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Gating the `model` declaration (item 5a, family `model-import`) on the model profile breaks every state-layer unit. QSpec's shared grammar writes every state clause against a model alias: `invariant ... on type-name` and `pre`/`post ... on operation-name`, where `type-name = ident '::' member-name` and the first segment is the model alias. QSpec state core is defined as operation contracts over supplied model contracts. So a unit that selects `quire.state.core/v1`, `quire.state.queries/v1`, `quire.state.graph/v1` or `quire.value.complete/v1` cannot write any `pre`, `post` or `invariant` without a `model` declaration, and item 5a refuses that declaration. FR-110-AC-7 (each of the five layers compiles a unit using only its own forms) then cannot pass for the state layers. FR-361-AC-5's own second sentence (a value-profile unit with a `pre` or `post` over value operands compiles), TC-1016 step 2 and TC-1015 step 1 cannot pass either. The PR applies planner ruling (e) faithfully, so the ruling itself needs revisiting. One option: gate only the model-complete forms and leave the `model` import ungated, as QSpec does. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:104-107,187 |
| FND-002 | medium | Item 6 says a clause whose target names a model operation reaches it only through a `model` declaration, which the `model-import` family refuses. QSpec's shared grammar says an operation-body's first segment is a declared model alias, which may be a `model` declaration or an import `as` alias. Item 11 says a library's selection is not passed to its importer. So a value-only unit that imports a library selecting the model profile can name that library's model through its import alias, with no `model` declaration. The `model-import` gate does not close the leak the item claims it closes. The statement should name the import-alias path and say whether it is gated. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:113-115 |
| FND-003 | medium | Item 6 and TC-1015's title say QSpec states no model gate for navigation, `Population<T>` and `allInstances`. The QSpec shared grammar's model-forms paragraph opens by referring to the model forms listed earlier in the grammar, which include `Population<T>` and `allInstances`. It names field and relationship-end navigation and closes with a refusal rule for a package that does not select `quire.model.complete/v1`, which reads as covering these forms. Leaving these ungated is the planner's ruling (d), and the PR applies it. But the spec presents it as QSpec's text rather than QSL's reading. It should be recorded like item 5a: QSL's reading, with a QSpec clarification requested. Item 6 also puts `reaches` in the state-clause family that "state core admits", but QSpec state core refuses `reaches`, which is a state-graph form. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:108-112 |
| FND-004 | medium | AC-9, item 2 and TC-1017 step 5 fix `quire.protocol.complete/v1` as `unknown_profile`/`unsupported-selection`. FR-110 gives that pair only when no catalog row matches "and no clause family's catalog holds it". ADR-011's `profile_selections` row says a header `profile` that selects a clause profile (`protocol_profile`) is resolved by the owning clause family, and QSpec ADR-006 makes `quire.protocol.complete/v1` the one protocol admission profile. The identity reaches FR-110's table today only because no clause family is registered yet. This is target-design text, so the AC will become wrong once `ProtocolClause` registers. Ruling (a), "unknown_profile refuses every identity outside the five", conflicts with FR-110's clause-family row in the same way. Use an identity that no catalog holds, or make the outcome conditional on whether a clause family is registered. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:67-77,191 |
| FND-005 | medium | The 'not enforced yet' sentence in FR-102 (repeated in FR-302:72-76) still says "FR-304-AC-8 and FR-308 state the same rule without code behind it". This PR changed FR-308 to the protocol facet and made it state that no gate applies, so FR-308 no longer states the model rule. The sentence is now false. It also gives no deletion condition. FR-361 says only "Until this requirement is built", and it has its own present-state sentence (lines 45-48) that needs the same condition. State that the FR-102, FR-302 and FR-361 sentences are deleted in the code slice that enforces FR-361-AC-1 to AC-7, as QSL-750 item 4 says. | spec/functional/FR-102-build-state-clause-forms.md:82-88 |
| FND-006 | low | FR-308's description, and FR-309's in the same way, now make the S2 build conditional on "under `quire.protocol.complete/v1`". The new behaviour bullet in the same file says S2 builds each form "whether or not the unit selects `quire.protocol.complete/v1`", and FR-361-AC-9 refuses that identity in a header. Read literally, the trigger is never met, so FR-308-AC-1 to AC-5 have no triggering condition. Drop the condition from both descriptions, or restate it as the facet that QSpec assigns to these forms. | spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md:23-29,83-86 |

## Dispositions

Round 1, reviewed at 485c2fad7374250b5ceb6a95cf7a4c8c4031ba9b.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e01e00c9f |
| FND-002 | fixed | e01e00c9f |
| FND-003 | fixed | e01e00c9f |
| FND-004 | fixed | e01e00c9f |
| FND-005 | fixed | e01e00c9f |
| FND-006 | fixed | e01e00c9f |
