---
id: SR-1957005
title: "spec-review/failure-domain review of QSL-1372 observation authority"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; QSL-1372"
review_set: subset
---

## Summary

Ticket: QSL-1372; PR 1957. Trust boundary is explicit: only successful FR-106 construction yields immutable authority for the exact selected view/model; no arbitrary object environment or document label grants authority. Refused, Incomplete and Fault remain distinct, retain original data, and yield no successful authority. Source/slot failures precede inspection of a supplied failed observation. Nested references and recursive population/relationship closure reuse FR-106's finite checked facts and ordering; existing snapshot/model limits bound construction and there is no new evaluator checker or raw byte read. Empty/unlisted population identities are retained, and present n5 cannot bypass completeness. Canonical carrier duplication is retained under SR-1957001, not accepted as a new failure vocabulary.

## Verdict

PASS for this method. Overall PR readiness is blocked by SR-1957001. No implementation completeness claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assurance Context

No applicable AssuranceProfile exists: the frontmatter search found none in spec/. spec/assurance contains AD-001 (proposed ArchitectureDescription) and MP-001 through MP-006 (MeasurementPlan), not enforced review policies. Repository AGENTS/CLAUDE and the assigned subset select direct review. No human acknowledgement gate is inferred.

The assigned frozen diff changes FR-106 and FR-390 only. No production implementation, runtime acceptance or aggregate gate is claimed. The new criteria are implementation obligations, not completed test evidence. Full AC-6/AC-11 remain open. Installed Quire validation of both explicit requirements exits zero with 2/2 grammar-clean, zero grammar findings. Existing module DuplicateArchetype, DuplicateInverseEdge and inline-data-schema advisories remain; no old tool fallback was used.

Methods selected: code-review (documentation lane), base, integrity, dependency, failure-domain, scope-boundary and ears-conformance. Rust/React review are inapplicable because no source/manifests changed. No gap-analysis/matrix or whole-repository qualification: pure specification scope, explicitly excluded by the brief. Object/app/security/evidence/risk-complexity lenses do not have changed objects, assurance allocations or risk artifacts here; criterion strength/Jev is not selected. Runtime model identity was verified from this reviewer's own turn-context metadata as gpt-6.1-sol.

## Examined Scope

```yaml
{
  "scope": [
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "layer-4 `qsl-package` SHALL retain successful current-snapshot admission as an\nopaque, immutable\n`qsl_package::observation::AdmittedProtocolObservation<'view>` tied to the exact\n`AdmittedEvaluationView` that selected the models."
    },
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "An authority SHALL expose read accessors only: no public fields, mutable\nenvironment, mutable completeness flag, mutable model provenance or mutable\npopulation association. Only the successful FR-106 admission path SHALL\nconstruct an authority; converting a caller-built `Observation`, an\n`ObjectEnvironment`, a document identity or a model label into one SHALL be\nimpossible through the public API."
    },
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "The entry SHALL reuse FR-106 current-snapshot admission with no clause,\ninvocation or anchor comparison. It SHALL verify that `model` is one of the\nview's own selected models and bind the snapshot to that exact model using\nFR-106's model-selection check."
    },
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "Admission SHALL test required-population completeness before target presence,\neven when every supplied reference already resolves to an admitted object.\nThe first required incomplete population in FR-106 walk order SHALL yield\n`incomplete_population`/`incomplete-scope` naming its population declaration\nnode and retaining the original FR-106 record. An unrelated incomplete\npopulation SHALL NOT make the call incomplete."
    },
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "A successful authority SHALL\nown its admitted observation data and borrow its originating view; it SHALL\nborrow neither snapshot bytes nor caller-owned provisions. The entry SHALL\naccept no evaluation `Meter`, evaluate no expression and create no replacement\nmeter. Its admission usage and limits SHALL remain FR-106's, separate from\nS6a charges."
    },
    {
      "id": "FR-390",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "   order above. When observation admission is required, the call SHALL inspect\n   the supplied typed result after checks 1 through 5: `Refused` and\n   `Incomplete` retain their respective failure kinds and the entire original\n   QSV admission record; `Fault` returns the original `InternalFault` through\n   `CallFailure::Fault`; no non-admitted arm yields an authority."
    },
    {
      "id": "FR-106-AC-62",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "A current snapshot admitted through the view entry returns `ProtocolObservationAdmission::Admitted`; its authority retains the exact selected model, document identity, population manifest/bundles and admission usage after the caller drops the provisions. Public struct construction, mutation of its environment or completeness/provenance, and conversion from a caller-built `Observation` fail to compile; reading the documented accessors compiles."
    },
    {
      "id": "FR-106-AC-63",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "A snapshot of another model yields `Refused` with the original QSV `AdmissionRecord` and `invalid_model_binding`/`wrong-model-selection`; a nested-value refusal preserves every `catalog`, `fields`, `path` and `kernel` component. `authority()` on `Refused`, `Incomplete` and `Fault` returns that same distinct arm and payload; `Fault` preserves the original `InternalFault`. The QSL admission implementation uses QSV's record/path types with no local copies."
    },
    {
      "id": "FR-106-AC-64",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "Over TC-1071 row 20, an authority retaining `n5` but marking its required `nodes` population incomplete yields `incomplete_population`/`incomplete-scope` naming that population's exact node, before any dangling check. With `nodes` complete and a second unreferenced population incomplete, the same supplied source admits. A recursively required incomplete population is reported in FR-106 walk order; completing it exposes any subsequent check-8 dangling defect."
    },
    {
      "id": "FR-390-AC-20",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "TC-1071 row 20 uses a successful immutable observation authority in the typed result; although `n5` exists, `nodes` incomplete returns `CallFailure::Admission` with category incomplete, the population locus and the unchanged QSV FR-106 record. An unrelated incomplete population does not alter the successful call. The meter is unchanged on either admission failure."
    },
    {
      "id": "FR-390-AC-21",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "TC-1071 row 21 supplies the `Refused` result of FR-106 current-snapshot admission for another model and returns the original `invalid_model_binding`/`wrong-model-selection` record without a foreign-reference replacement. `Incomplete` preserves its kind/record and `Fault` preserves its `InternalFault`; none supplies successful authority or charges the meter."
    },
    {
      "id": "FR-390-AC-22",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "A malformed slot, duplicate source, unknown source, missing source and wrong-shaped value combined with a refused observation result return the corresponding check-1-through-5 failure before the observation result is inspected. With those defects absent, a reference-valued source and no observation returns the existing unavailable-observation record; a present failed result returns its original arm. All failures leave the caller's existing meter unchanged, and TC-1071 row 22 charges only its successful evaluation."
    }
  ]
}
```

## Integration Dependency

The lead has routed canonical observation-result placement below layer 4 through IR-1200. That route does not resolve the duplicated declaration on this frozen branch or authorize a forbidden layer-4-to-E dependency. No upstream API shape is guessed by this review.
