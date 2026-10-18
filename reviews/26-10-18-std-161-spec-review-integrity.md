---
id: SR-12805
title: "integrity review of STD-161 FR-361 item 17 (ObjectNavigation exception retirement)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@472c7d314c6204d6a5070801a6eeb781e37cbdb7; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md (item 7, item 17, FR-361-AC-13)"
review_set: subset
---

## Summary

Ticket: STD-161 (PR quire-spec-language#965). Checked the new FR-361 item 17,
the item 7 addition and FR-361-AC-13 against FR-110 "Layer selection", FR-361
items 9 and 12, and the layer table in `qsl-semantics/src/value/layer.rs`. The
deletion list (a)-(b) matches the code and FR-110 exactly. The closing sentence
of item 17 states admission at unit level ("its selected layers' closure"),
which contradicts FR-110's per-declaration `using`-alias layer rule; the fixture
scope in (c) inherits the same unit-level reading.

## Verdict

CHANGES REQUESTED (one HIGH, one MEDIUM, both one-sentence fixes). AC-13 itself
is single-header and consistent with FR-110.

## Examined Scope

```yaml
scope:
  - {id: FR-361-item-17, path: spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, role: examined, excerpt: "After the deletion a unit holding `deref` or `reaches` is admitted exactly when its selected layers' closure names `quire.state.graph/v1`, and otherwise is refused by FR-110's layer rule naming the selected layers. No model-gate refusal (item 11) applies to the forms."}
  - {id: FR-361-item-7, path: spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, role: examined, excerpt: "The object navigation forms `deref` and `reaches` are gated by that layer rule alone, with no exception: item 17 states it."}
  - {id: FR-361-AC-13, path: spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, role: examined, excerpt: "A unit that holds a `deref` and a unit that holds a `reaches`, each under a header that selects only `quire.value.complete/v1`, refuse once with FR-110's layer rule naming the selected layer and retain no family of item 5; the same units under `quire.state.graph/v1` and under `quire.model.complete/v1` compile. The layer table's `quire.value.complete/v1` row holds no `ObjectNavigation` entry."}
  - {id: FR-110-layer-selection, path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, role: context_only, excerpt: "When a declaration's `using` alias names a header profile resolved to a layer, S3 SHALL admit the declaration under exactly the admitted-form set of that layer's `admitted_forms` definition, together with the sets of the layers its `requires` closure names."}
  - {id: FR-110-exception-bullet, path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, role: context_only, excerpt: "Until QSL-750 ... the admitted-form set of `quire.value.complete/v1` also holds the object navigation forms (`deref`, `reaches`), which QSpec's `quire.state.graph/v1` adds. This is a temporary exception, removed when QSL-750 lands."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Item 17's last sentence says a unit holding `deref`/`reaches` is admitted exactly when "its selected layers' closure" names `quire.state.graph/v1` and is refused "naming the selected layers". FR-110's layer rule is per declaration: the form is admitted under the closure of the layer the declaration's own `using` alias resolves to, and the refusal names that one layer (code: `LayerAdmission::new(lock, layer, ..)` per alias). Failure: a unit with `profile g = state.graph` and `profile v = value.complete` whose `invariant ... using v` writes `reaches(...)` is admitted per item 17 but refused per FR-110; an implementer of QSL-750 following item 17 would add a unit-level admission that FR-110 and item 12 forbid. Fix: "a declaration that writes `deref` or `reaches` is admitted exactly when its `using` alias resolves to a layer whose `requires` closure names `quire.state.graph/v1`, and otherwise is refused by FR-110's layer rule naming that layer." | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:220-223; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:144-153; qsl-semantics/src/check/layer.rs:23-45 |
| FND-002 | medium | Item 17(c) scopes the fixture move to units "under a header that selects only `quire.value.complete/v1`". Under the per-alias rule the fixtures that break are those whose declaration writing `deref`/`reaches` uses an alias resolving to `value.complete`, including multi-profile units that also select another layer through a different alias. Failure: such a fixture is left unmoved by a builder reading (c) literally and goes red after (a). Fix: scope (c) by the declaration's `using` alias, not by the unit's header set. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:216-219 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Item 17 now decides per declaration `using` alias and names that layer (a425f7631, FR-361:220-224). |
| FND-002 | fixed | Item 17(c) now scopes fixtures by the declaration's `using` alias, whatever other profiles the header selects (a425f7631, FR-361:215-219). |
