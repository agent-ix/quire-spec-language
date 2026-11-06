---
id: SR-20261105101065
title: "EARS review of QSL-1690 unit root coverage"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; QSL-1690; PR #2431; model/tests.rs, tests/it/model_type_unit_roots.rs, tests/it/main.rs, FR-094-AC-15 and TC-419"
review_set: subset
---

## Summary

Independent review of frozen PR #2431 found no new defect. Both requested fix-forward obligations are addressed at the evidence levels specified by the change.

## Verdict

**PASS** for the scoped change, subject to the lead's one final scoped gate. The owning SHALL is unchanged in this delta and was examined with its new AC. It names units referenced by selected model declarations as the subject and mandates a concrete checked-graph root response even without expression reachability. The new acceptance row adds verifiable outputs and bounded evidence scope, without a new vague or competing SHALL. Existing Quire validation evidence was inspected; no whole-spec grammar rerun was performed by this reviewer.

## Scope

- FR-094 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: Units named by a selected model's type declarations SHALL be roots of the
checked graph even when no checked expression names them. This includes a
unit named by an alias, a collection, option or map element, a field, tuple
position, variant payload, operation signature or Rational scalar. The root
uses the admitted unit identity and the ordinary unit-node walk below, so its
dimension, base dimensions and target-unit closure retain their existing
identity, owner, scale, offset and declaration occurrences. Unused source
units that no selected model type names do not become roots by this rule.
- FR-094-AC-15 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: A selected model names source unit `U` in exactly one otherwise unused type position: a field, tuple position, sequence item, option inner type, alias target, operation parameter, operation result or Rational scalar unit. For each position independently, with no expression naming `U`, check and handoff emit its source-owned unit node, its dimension and its target-unit dependency closure; the unit is typed by its dimension and has exactly the declared nominal dependencies. An unrelated declared unit is omitted. The normal checked-package/v2 reader admits each emitted package.
- FR-094-AC-15 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: Separately, synthetic contract fixtures normalized through the production normalizer independently cover a Variant payload and a Map value naming a unit: lowering emits that unit and exactly its nominal closure.
- FR-094-AC-15 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: This defensive record-arm coverage does not claim public admission of those forms; unit variant payloads remain refused under FR-056-AC-25, and the current Map original-provenance intake refusal remains outside this criterion. The Rational fixture declares its unit in the model namespace as required by TypeDefinition.unit.
- TC-419 (examined), spec/test-cases/TC-419-quantity-type-nodes-and-compound-units.md: 8. Independently place `ix://quire/unit/U` in each of FR-094-AC-15's eight publicly admitted model type positions, leaving every other model type unitless. Select that model from a source declaring dimension `Money`, root unit `Base`, derived unit `U = 2 × Base` and unrelated unit `Unused`. For Rational, also declare the model's own `Money` dimension and `U` root unit, satisfying its namespace rule. Check only a Boolean expression that names no unit. Write each checked handoff through the production writer and read it through the production IR reader.
- TC-419 (examined), spec/test-cases/TC-419-quantity-type-nodes-and-compound-units.md: 9. Independently for the defensive Variant and Map record arms, construct a synthetic domain package containing an otherwise unused variant payload or map value typed by an admitted unit. Run the real model normalizer, admit its resulting contract view, and invoke production model-type unit-root lowering without any expression lowering. This is contract-fixture coverage, not evidence of public intake accepting a unit payload. The existing FR-056-AC-25/TC-995 test continues to assert that public refusal.
- TC-419 (examined), spec/test-cases/TC-419-quantity-type-nodes-and-compound-units.md: - Step 8: each package contains source-owned `U`, `Base` and `Money` nodes; `U` is a unit typed by `Money` with exactly `Money` and `Base` as dependencies, and `Base` is typed by `Money`. No `Unused` node is emitted. Each independently emitted package is admitted by IR's checked-package/v2 reader.
- TC-419 (examined), spec/test-cases/TC-419-quantity-type-nodes-and-compound-units.md: - Step 9: each normalized record roots its unit and exactly the unit's nominal dimension closure; an unrelated unit and its dimension are absent. The unit has the lowering source owner and its admitted dimension type.
- FR-056-AC-25 (context_only), spec/functional/FR-056-admit-domain-package-model-declarations.md: A case with a resolving admitted value `payloadType` is admitted as a union member with that one typed payload position; a case without `payloadType` is admitted as a nullary member. Refusals for unresolved or unsupported payloads follow QSpec FR-154's intake code. In particular, a Clock payload refused by the FCD reader as `INVALID_CLOCK_MEMBER` surfaces as `unsupported_construct`/`declaration-form` at node `.../variant/Tier-Gold`, path `Variant.payloadType`, per FR-154-AC-25 and FR-208-AC-15. No declaration of the package is admitted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and limits

Source review only; this reviewer ran no builds, tests, Cargo commands, final gate, lock edits or source edits. Inspected coder evidence: /tmp/qsl1690-restored-public.log (8 passed), /tmp/qsl1690-restored-normalized.log (2 passed), /tmp/qsl1690-collector-disabled-public.log (Alias passes; 7 MissingDeclaration failures), /tmp/qsl1690-collector-disabled-normalized.log (2 missing-unit failures), /tmp/qsl1690-coverage-clippy.log (focused integration Clippy completes), /tmp/qsl1690-coverage-spec-validation.log and supplied formatter evidence. Production model.rs is unchanged in the frozen diff. Final scoped gate remains with the lead after review-artifact custody; no whole-repository or individual-mutation claim.

No active AssuranceProfile document applies in the repository's spec tree. No new API, unsafe, concurrent, wire schema, production implementation, Cargo/lock or CI change. Production gap-analysis is not applicable to this test/spec-only delta; the manual scoped collector coverage mapping above is the acceptance check. Model identity is unavailable in this harness: model=unavailable is explicit metadata, not an inferred model ID.
