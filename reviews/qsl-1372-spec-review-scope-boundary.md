---
id: SR-1957006
title: "spec-review/scope-boundary review of QSL-1372 observation authority"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; QSL-1372"
review_set: subset
---

## Summary

Ticket: QSL-1372; PR 1957. QSV owns the shared input-path record and path vocabulary, with actual public fields inspected. QSL FR-106 owns observation admission; layer 4 owns the view-bound authority and construction over provisions; layer 5 performs slot-source integration without reading bytes or importing a second checker. E owns call-failure transport and a population locus, not QSL semantic decisions. Cross-repository record/transport behavior is guaranteed by the named FR-130 contract and future consumer integration checks, not inferred from an enum name. The newly shipped canonical generic observation result creates the placement/duplication conflict in SR-1957001; no E dependency exception is granted here.

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
      "excerpt": "Shared FR-130 governs the QSV record and `quire-eval`'s downstream call-record\nand population-locus transport; QSL owns the observation API above and the\nadmission decision."
    },
    {
      "id": "FR-390",
      "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
      "role": "examined",
      "excerpt": "  - `observation`: `Option<&'a ProtocolObservationAdmission<'a>>`, using the\n    one `qsl_package::observation` result defined by FR-106. Its `Admitted`\n    arm carries a current-snapshot authority for the evaluated view and the\n    protocol's selected model; its other arms retain the original admission\n    result. It is required exactly when a supplied source value contains a\n    reference, including a nested reference. It carries no raw snapshot,\n    provision or caller-constructed `Observation`."
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
