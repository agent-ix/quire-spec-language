---
id: SR-1957007
title: "spec-review/ears-conformance review of QSL-1372 observation authority"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; QSL-1372"
review_set: subset
---

## Summary

Ticket: QSL-1372; PR 1957. The installed deterministic grammar check reports both explicit requirements grammar-clean and zero findings. Manual review of the changed SHALL statements found concrete named actors, conditional refusal behavior and observable responses. The mandatory outcomes are record identity, successful/failed arm distinction, read-only authority, specific required-population precedence and unchanged meter. No vague responsiveness claim or new unmeasurable property is introduced. API declarations and table criteria supplement those obligations; they are not substitute prose grammars.

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
      "excerpt": "Or an `AdmissionFailure`: `Refused(AdmissionRecord)` or\n  `Incomplete(AdmissionRecord)`, each with the exact shared\n  `quire_semantic_value::admission::AdmissionRecord`; an admission invariant\n  break retains its existing `Fault(InternalFault)` arm."
    },
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
      "excerpt": "pub fn admit_protocol_observation<'view>(\n    &'view self,\n    model: &DomainPackageRef,\n    packages: &PackageInput,\n    model_limits: ModelNormalizationLimits,\n    snapshots: &BTreeMap<DocumentRef, Vec<u8>>,\n    selected: &DocumentRef,\n    limits: ObservationLimits,\n) -> qsl_package::observation::ProtocolObservationAdmission<'view>;"
    },
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "A successful authority SHALL\nown its admitted observation data and borrow its originating view; it SHALL\nborrow neither snapshot bytes nor caller-owned provisions. The entry SHALL\naccept no evaluation `Meter`, evaluate no expression and create no replacement\nmeter. Its admission usage and limits SHALL remain FR-106's, separate from\nS6a charges."
    },
    {
      "id": "FR-106",
      "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
      "role": "examined",
      "excerpt": "Its `authority(&self) -> Result<&AdmittedProtocolObservation<'view>, &Self>`\nSHALL return the borrowed authority for `Admitted`, and the original borrowed\nnon-admitted arm for every other result. It SHALL NOT flatten absence of\nauthority to `None`."
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
      "excerpt": "Shared FR-130 governs the QSV record and `quire-eval`'s downstream call-record\nand population-locus transport; QSL owns the observation API above and the\nadmission decision."
    },
    {
      "id": "FR-390",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "  - `observation`: `Option<&'a ProtocolObservationAdmission<'a>>`, using the\n    one `qsl_package::observation` result defined by FR-106. Its `Admitted`\n    arm carries a current-snapshot authority for the evaluated view and the\n    protocol's selected model; its other arms retain the original admission\n    result. It is required exactly when a supplied source value contains a\n    reference, including a nested reference. It carries no raw snapshot,\n    provision or caller-constructed `Observation`."
    },
    {
      "id": "FR-390",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "   order above. When observation admission is required, the call SHALL inspect\n   the supplied typed result after checks 1 through 5: `Refused` and\n   `Incomplete` retain their respective failure kinds and the entire original\n   QSV admission record; `Fault` returns the original `InternalFault` through\n   `CallFailure::Fault`; no non-admitted arm yields an authority."
    }
  ]
}
```

## Integration Dependency

The lead has routed canonical observation-result placement below layer 4 through IR-1200. That route does not resolve the duplicated declaration on this frozen branch or authorize a forbidden layer-4-to-E dependency. No upstream API shape is guessed by this review.
