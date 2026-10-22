---
id: SR-15960
title: "Spec review of quire-spec-language PR #1247: STD-275, equality admitted by operand family per layer"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9739bc5897c142db96863bd49910d33b2680a690; PR #1247 (git diff origin/main...9739bc589): spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md; read for context: QSpec definitions state-core.md, state-queries.md, state-graph.md, value-complete.md, complete-value-lock.json header_selectable_layers; QSpec FR-149; QSL FR-361 items 5-12, qsl-semantics/src/value/layer.rs LAYER_ROWS; agent-ix/quire-specification main 5ee0071f"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-490
    type: reviews
---
# SR-15960: Spec review of quire-spec-language PR #1247: STD-275, equality admitted by operand family per layer

## Summary

Ticket: STD-275. Sub-analyses folded in: integrity, EARS, criterion strength (by hand: no Jev client), failure-domain. The head 9739bc589 is behind main 423ac6aa0 and merges cleanly. `make check-spec-validation` on the merge exits 0 with no diagnostic on FR-110 or TC-490. FR-110-AC-10 is new, and no other open PR touches FR-110 or TC-490. The Known-limit bullet is replaced with behaviour, with no status wording. The table matches QSpec: state-core.md refuses whole-record, sequence and option equality and reference identity; state-queries.md keeps reference identity refused; state-graph.md adds object and reference identity equality; value-complete.md admits records, tuples, unions and collections (the FR-149 matrix). The `requires` closures in complete-value-lock.json give the AC-10 outcomes: value.complete requires only state.core, so it does not admit reference equality, and model.complete closes over both. Every AC-10 vector was checked by hand against the table and the closures. Reference-typed operands can be written under state core, because LAYER_ROWS gates only `deref`/`reaches`, not a bare read. Typing refusals come first, so event-type equality (FR-149 `operator-ineligible`) and float equality (`ill_typed`) never reach the layer check. The lead defaults are sound: the refusal shape matches the FR-110 layer rule, tuple/union/set/bag/ordered-set are whole values as FR-149 groups them, and leaving reference identity out of value.complete follows from the lock. No QSpec follow-up is needed.

Units examined:

- `FR-110-AC-10` (examined): spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
- `FR-110` (examined): spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
- `TC-490` (examined): spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md
- `state-core.md` (context_only): agent-ix/quire-specification:proposals/quire-v1/definitions/state-core.md
- `state-graph.md` (context_only): agent-ix/quire-specification:proposals/quire-v1/definitions/state-graph.md

## Verdict

**MERGEABLE after a sync with main, with two minor findings to fix or accept.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | [minor] The equality section does not say which declarations it applies to. The FR-110 layer rule restricts only a declaration whose `using` alias resolves to a layer, and a unit with no profile selects no layer (its lock is the value.complete default). The new bullet admits a family only when 'the family's row names one of [the selected layers]'. Read literally, a declaration with no selected layer admits no family, so `r = q` over records (or even `x = y` over `Int`, since 'every layer' names none) would be refused in a no-profile unit. State that the check applies only to a declaration whose `using` alias resolves to a layer, as 'Layer selection' does, and add a no-profile case to AC-10 (for example `r = q` compiles in a unit with no header profile). | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:152-179 |
| FND-002 | medium | [minor] The family is taken from the top-level operand type only. A whole value that contains a reference, such as `Sequence<Reference<Order>>` or a record with a `Reference<T>` field, is in the whole-value row and so compiles under `quire.value.complete/v1`. But FR-149 compares its reference leaves by identity triple, and value.complete does not admit reference identity (it requires only state.core). This lets a value.complete declaration compare references without state.graph. Say that a whole value containing a `Reference<T>` at any depth needs both rows (admitted by model.complete, refusal naming state.graph when value.complete is selected), or state that the top-level rule is intended. Add one vector either way, for example `s = t` over `Sequence<Reference<Order>>` under value.complete. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:161-164,176-179 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | [minor] Regression from the any-depth rule. AC-10 does not give the element types of `r`, `s` and `o`, and they now decide the outcome. Under state core, `o = p` over `Option<Reference<Order>>` (TC-1050's `buddy` shape) first fails the reference row and names `quire.state.graph/v1`, not the `quire.value.complete/v1` AC-10 expects. Under value.complete it refuses where AC-10 says it compiles. State scalar element types, for example `Sequence<Int[0, 9]>`, `Option<Int[0, 9]>` and records with only scalar fields. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:230 |
| FND-004 | medium | [minor] 'every type it contains at any depth' does not say whether a `Reference<T>` contains its target `T`. A reference compares by identity triple only (FR-149), but a literal reader can take `Reference<Order>` to contain `Order` and its record field `total: Money`. Then `a = b` would be whole-value as well, and AC-10's `a = b` compiling under state.graph would refuse naming value.complete. Say that the target type of a `Reference<T>` is not a contained type. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:169-171 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 9060cc9aa: the section now opens 'For a declaration whose `using` alias names a header profile resolved to a layer' and adds 'A declaration that selects no layer is not restricted by this section'; FR-110-AC-11 adds the no-profile vector. |
| FND-002 | fixed | 9060cc9aa: families include every contained type at any depth, an equality is admitted only when every family is, and the refusal names the first layer of the first unadmitted row in table order. Checked by hand: under value.complete `Sequence<Reference<Order>>` first fails the reference row (state.graph); under state.graph it first fails the whole-value row (value.complete); model.complete admits both. AC-11 states exactly these. |

Disposition pass 1 verdict (head 9060cc9aa): **NOT MERGEABLE until FND-003 and FND-004 are fixed or accepted (both minor).**
