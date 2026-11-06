---
id: SR-20261105101054
title: Scoped trace review of selected ordinary object roots
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language; QSL-1762; PR #2429; child object-rooting diff,
  FR-094 and TC-417'
review_set: subset
---

## Summary

No defects found in this selected lens.

## Verdict

**PASS**. All four active new integration tests bind FR-094-AC-14 and TC-417. The source root pass has an owning requirement and the assertions cover native E-P success, Standard-E stability, repeated aliases, real witness omission/foreigning and an absent real object node. The isolated old-root removal reproduces the original native refusal. No stubs, ignored tests or vacuous positive assertions were found in the changed scope.

## Scope

- FR-094-AC-14 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: With TC-1070's standard E and E-P documents selected, the normal handoff emits each ordinary object declaration exactly once with its original effective identity, model owner, correspondence and a generated occurrence at the selecting alias. E-P includes Node, Other and Elsewhere even though Elsewhere is used only by an unused population. Strict read and the public native selected-type projection succeed for both documents, preserving their complete record/object facts. Omitting or foreigning a real object declaration witness returns a typed native refusal without a partial projection. The ide
- FR-094 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: For every source-selected model alias, `check` SHALL root every admitted ordinary object-type declaration in the selected domain document, including an object named only by another model member or an unused population. An object selected through several aliases SHALL be rooted once, at the first selecting alias's declaration location, using the existing model node, original declaration key, effective type identity and model correspondence. The root SHALL have a generated occurrence at that location. No source expression or frame need reach the object. This does not root systems-interface objec
- TC-417 (examined), spec/test-cases/TC-417-model-reference-and-population-nodes-match-key-vectors.md: 9. Compile TC-1070's genuine standard E and E-P documents through the normal    handoff writer, strictly read the packages and read their required identity    files. Project selected type declarations using the public native reader.    E-P includes Other and the unused-population member Elsewhere. Compare    original identities, owners and complete native facts, then omit and foreign    a real object witness and request the same projection.
- TC-417 (examined), spec/test-cases/TC-417-model-reference-and-population-nodes-match-key-vectors.md: Every fixture admits the domain package `acme/orders` that FR-094's Vector index vectors section describes, at version `1.0.0` unless a step says otherwise, with `M::Order`, `M::Invoice` and `M::Sub` resolving to its object types. `Sub` has supertype `Order`, inherits `total` and redefines `size`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed the frozen child feature only against its QSL-1760 base; the parent carrier/codec was not re-reviewed. No Cargo commands, builds, tests, gates, locks or source edits were run. Author evidence in /tmp/qsl1762-logs/focused.log records focused Clippy, four native handoff controls and thirty model-lowering controls passing; old-root-red.log records UnresolvedSelectedDeclaration / selected_document_type / work 123 after removing the object-root call, followed by restored green evidence. Parent QSL-1760 fixes and fresh-main reconciliation are still pending for this child and require this same reviewer's follow-up. The lead owns the one final scoped gate. Normal downstream driver adoption is explicitly unqualified. No applicable active AssuranceProfile was found. The installed matrix CLI has repository scope only and no criterion filter; scoped computed coverage is unavailable, so active tags and assertions were checked manually without running a global census. Plan completion: not assessed. Exact reviewer model metadata is unavailable in this harness; comments identify it as unknown rather than inventing a model name.

## Parent-base reconciliation (disposition round 1)

The seven original feature commits remain patch-identical after the rebase. The additional fix scopes TC-417's `acme/orders` premise to steps 1–8 and explicitly names step 9's `test/orders` E/E-P documents. Source inspection confirms the same ordinary-object root helper still calls the existing `model_node`, preserves first-alias provenance and deduplicates original declarations; the parent catalog and key-bridge fixes introduce no competing object-root or identity path. No child manifest, lock, schema version or CI change appears. The latest parent differs from the rebased parent only by review-artifact custody. The parent review is not repeated here.

Reconciled author logs `/tmp/qsl1762-logs/reconciled-focused.log` and `reconciled-docs.log` record Clippy, four native E/E-P handoff controls, thirty-one model-lowering controls, formatting and document validation passing. The reviewer inspected that evidence but ran no builds, tests or gates. Both changed documents are grammar-clean on reviewer validation. No new findings were found in reconciliation. The scoped feature review is PASS after the fixed disposition; the lead's one final scoped gate and normal downstream driver adoption remain outstanding.
