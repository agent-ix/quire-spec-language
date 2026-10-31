---
id: SR-1957002
title: "spec-review/base review of QSL-1372 observation authority"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; QSL-1372"
review_set: subset
---

## Summary

Ticket: QSL-1372; PR 1957. IDs remain contiguous (FR-106 62–64; FR-390 20–22), owning value/input types are existing FR-106 types and concrete new QSL-owned authority APIs, and the shared record/path references resolve to actual FR-130 and merged QSV APIs. Verification cells on new criteria name Test without TC ids. Shared result duplication is recorded once in SR-1957001 and prevents overall merge readiness; this base checklist does not mark it fixed. The domain population manifest is existing FR-364 semantic input data, not a new tracking inventory or digest mechanism. No revision pin, digest, checksum or source-scanning criterion is introduced. New criteria have not been implemented, which is expected at this specification stage.

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
      "excerpt": "pub enum ProtocolObservationAdmission<'view> {\n    Admitted(AdmittedProtocolObservation<'view>),\n    Refused(quire_semantic_value::admission::AdmissionRecord),\n    Incomplete(quire_semantic_value::admission::AdmissionRecord),\n    Fault(quire_semantic_value::catalog::InternalFault),\n}"
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
    },
    {
      "id": "FR-390-AC-6",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "A value of the wrong shape, a record of another type, a value outside its type's domain, a reference to another model's population and a dangling reference each refuse with the code and cause TC-1071 lists; no observation with a reference, and an incomplete population, return `Incomplete`; an observation FR-106 refuses is refused with its record."
    },
    {
      "id": "FR-390-AC-11",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "context_only",
      "excerpt": "Running one call twice gives equal results and charge logs; a call over the package the contract reader returns for FR-383's emitted nodes gives the same result and charge log as over the checked package that was emitted; a refused call leaves the meter as it was."
    },
    {
      "id": "FR-130",
      "path": "agent-ix/quire-semantic-value/spec/functional/FR-130-share-one-input-path-admission-record-below-qsl-and-quire-eval.md",
      "role": "context_only",
      "excerpt": "QSV SHALL own only the shared record and path vocabulary. QSV SHALL NOT\n   depend on QSL or implement QSL FR-106 admission behavior. `quire-eval` SHALL\n   NOT depend on QSL to carry the record."
    },
    {
      "id": "FR-401-AC-2",
      "path": "spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md",
      "role": "context_only",
      "excerpt": "In QSL, TC-390's table check asserts that `qsl-eval` and `qsl-replay` name `quire-eval` in `[dependencies]` and that `qsl-foundation`, `qsl-cst`, `qsl-source`, `qsl-forms`, `qsl-semantics`, `qsl-package` and `qsl-route` name it in no table; a fixture manifest in which `qsl-semantics` depends on `quire-eval` fails the check, naming the crate and the table."
    }
  ]
}
```

## Integration Dependency

The lead has routed canonical observation-result placement below layer 4 through IR-1200. That route does not resolve the duplicated declaration on this frozen branch or authorize a forbidden layer-4-to-E dependency. No upstream API shape is guessed by this review.
