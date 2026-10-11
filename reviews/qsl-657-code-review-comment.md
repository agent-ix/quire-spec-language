<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-4942 method=code-review lang=rust pr=quire-spec-language#716 reviewed=4865d61bdbe85914e6d8342276152bcaef686a95 model=gpt-6 run=9ac0f31a-6e1f-4ead-8326-46a58791c760 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |

+++ [reviewer data]

```yaml
clean: true
scope:
  - {id: TC-197, path: "spec/test-cases/TC-197-empty-candidate-set-reports-unsupported-warning-data.md", role: examined, excerpt: "Verify that when no registered backend advertises an item's matching capability requirement, the registry returns an ordinary, well-formed empty candidate set (no error, no panic) retaining its capability kind and, when named, the requested backend's identity. Scope: FR-076-AC-1, FR-076-AC-2."}
  - {id: FR-076-AC-1, path: "spec/functional/FR-076-settle-backend-absence-as-unsupported.md", role: examined, excerpt: "Given a well-formed item and a registry with no registrant advertising its capability kind, or (for a probabilistic item) its exact evidence kind, the registry's candidate-set computation returns an empty set as an ordinary value; no error, panic or refusal-shaped result is observed at the registry boundary. Absent or unknown probabilistic evidence is excluded from this unsupported case and follows FR-076-AC-4."}
  - {id: FR-076-AC-2, path: "spec/functional/FR-076-settle-backend-absence-as-unsupported.md", role: examined, excerpt: "Given the same empty-candidate-set case, the registry's output retains the item's capability kind, its evidence kind when applicable, and, when the request named a backend, that backend's identity, so a caller can name them in a downstream warning without a separate lookup."}
  - {id: FR-076-AC-4, path: "spec/functional/FR-076-settle-backend-absence-as-unsupported.md", role: context_only, excerpt: "Given a probabilistic item with absent or unknown evidence and any registry contents, the candidate-stage output preserves the absent state or unknown label for negotiation, and the negotiation result is `invalid-request` with `absent-evidence-kind` or `unknown-evidence-kind`, respectively, never `unsupported`."}
  - {id: TC-197, path: "qsl-route/tests/it/route_registry.rs", role: examined, excerpt: "#[trace(\"TC-197\", \"FR-076-AC-1\", \"FR-076-AC-2\")] fn empty_candidate_set_carries_the_data_an_unsupported_warning_needs()"}
  - {id: TM-001, path: "spec/tests.md", role: examined, excerpt: "| TC-197 | Empty candidate set carries the data an unsupported warning needs | Unit | P1 | FR-076-AC-1, FR-076-AC-2 | ✅ |"}
findings: []
```

+++
