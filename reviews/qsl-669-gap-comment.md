<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-4961 method=gap-analysis lang=spec pr=quire-spec-language#720 reviewed=cfe61582fd9a51f727be92b3659bc6dd744f9b51 model=gpt-6 run=c4d68ece-ba1a-423d-b229-82f2c7ec511d -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | The changed effective-declaration golden-vector test asserts the exact published preimages and digests, but `effective_declaration_identities_match_their_golden_vectors` is tagged only `QSpec-TC-195` at `identity_golden_vectors.rs:149`. The only inspected test tagged to QSpec FR-150-AC-6 that covers the diamond (`n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities`) checks SHA-256 shape, uniqueness, path count and rerun stability, not equality to the published effective-declaration vectors. Thus the PR changes the two effective-declaration vectors without a test-to-AC binding that carries the AC's vector-equality obligation. Add the FR-150-AC-6 trace to the effective-declaration vector test (and retain the TC trace), or add an equivalent correctly traced vector assertion. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-150-AC-4, path: "agent-ix/quire-specification/spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md", role: examined, excerpt: "Replaying the ordered derivation from the same domain package reproduces the `quire.model.effective-view/v1` identity, independent of IR node order."}
  - {id: FR-150-AC-6, path: "agent-ix/quire-specification/spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md", role: examined, excerpt: "Diamond paths reaching one original declaration collapse to one effective member retaining every path, and each effective identity equals its published vector."}
  - {id: FR-150-AC-7, path: "agent-ix/quire-specification/spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md", role: examined, excerpt: "Declaration keys are fixed by the domain package identity and IR node identity: two versions of one package with equal nodes yield equal effective declaration identities and different effective views."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-150-AC-6
    related_ids: []
    path: qsl-semantics/tests/it/identity_golden_vectors.rs
    lines: "149-169"
    excerpt: |-
      Diamond paths reaching one original declaration collapse to one effective member retaining every path, and each effective identity equals its published vector.
    finding: "The changed effective-declaration golden-vector test asserts the exact published preimages and digests, but `effective_declaration_identities_match_their_golden_vectors` is tagged only `QSpec-TC-195` at `identity_golden_vectors.rs:149`. The only inspected test tagged to QSpec FR-150-AC-6 that covers the diamond (`n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities`) checks SHA-256 shape, uniqueness, path count and rerun stability, not equality to the published effective-declaration vectors. Thus the PR changes the two effective-declaration vectors without a test-to-AC binding that carries the AC's vector-equality obligation. Add the FR-150-AC-6 trace to the effective-declaration vector test (and retain the TC trace), or add an equivalent correctly traced vector assertion."
bindings: []
```

+++
