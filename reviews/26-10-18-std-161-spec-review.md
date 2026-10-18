---
id: SR-12806
title: "base review of STD-161 FR-361-AC-13 and TC-1015 step 3"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@472c7d314c6204d6a5070801a6eeb781e37cbdb7; spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md (FR-361-AC-13), spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md"
review_set: subset
---

## Summary

Ticket: STD-161 (PR quire-spec-language#965). Base checklist over the new
acceptance criterion FR-361-AC-13 and the TC-1015 step 3 addition: ID format,
uniqueness (AC-13 and item 17 are new and free; open PR #964 adds neither),
AC-to-TC link, and oracle strength.

## Verdict

PASS with low findings. The TC-1015 step 3 oracle is strong: `value.complete`
requires only `state.core`, so its closure admits `ObjectNavigation` only
through the exception entry; if `deref`/`reaches` stayed ungated, both
value-profile units compile and expected result 3 ("refused once by FR-110's
layer rule") fails. "Retaining no family of item 5" also rejects a wrong
implementation that refuses the forms through the model gate, and the
graph/model-profile half rejects an over-refusal. No hash, digest or pin use.

## Examined Scope

```yaml
scope:
  - {id: FR-361-AC-13, path: spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, role: examined, excerpt: "A unit that holds a `deref` and a unit that holds a `reaches`, each under a header that selects only `quire.value.complete/v1`, refuse once with FR-110's layer rule naming the selected layer and retain no family of item 5; the same units under `quire.state.graph/v1` and under `quire.model.complete/v1` compile. The layer table's `quire.value.complete/v1` row holds no `ObjectNavigation` entry."}
  - {id: TC-1015, path: spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md, role: examined, excerpt: "3. Compile one unit holding a `deref` and one holding a `reaches`, each first with the header profile `quire.value.complete/v1`, then with `quire.state.graph/v1`, then with `quire.model.complete/v1`. ... 3. Under `quire.value.complete/v1` each unit is refused once by FR-110's layer rule naming that layer, retaining no family of item 5; under the other two headers each compiles."}
  - {id: FR-361-item-17, path: spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md, role: context_only, excerpt: "Building this requirement SHALL delete that exception, as one change with the gate: (a) the `ObjectNavigation` entry of the `ValueComplete` row of the layer table, with the comment that records the exception; (b) the FR-110 bullet that records the exception; and (c) the fixtures ..."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-13's last sentence ("the layer table's `quire.value.complete/v1` row holds no `ObjectNavigation` entry") is a code-structure claim inside a `Test (TC-1015)` AC, and no TC-1015 step checks it. The behavioural step 3 already subsumes it; drop the sentence or add an inspection step. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:241; spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md:18-25 |
| FND-002 | low | TC-1015 Description still says the case "catches a gate that refuses a state clause or another form"; step 3 catches the opposite defect (an admission that must be refused). Add one sentence naming it. | spec/test-cases/TC-1015-forms-not-gated-raise-no-model-gate-refusal.md:13 |
| FND-003 | low | Item 17 (a)-(c) is a list of edits to make ("Building this requirement SHALL delete ... the comment", "the FR-110 bullet"), a build instruction rather than system behaviour. It was the ticket's explicit ask and matches the description paragraph's existing pattern, so this is wording only. | spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md:206-219 |
