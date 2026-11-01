<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310141 round=2 model=gpt-6 run=957BE8D8-5F94-4036-BD2C-C9D396DA411E -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Distinct dimension roles now survive on the same structural key; focused regression verifies their IDs and ordinals. |
| FND-002 | fixed | UnitId resolution uses the exact type key retained by checking; missing and ambiguous mappings refuse. |
| FND-003 | fixed | Witness selection and the shared-key regression now bind on both application and occurrence. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      if !visited.insert((key.clone(), role)) {
          continue;
      }
  - fnd: FND-002
    outcome: fixed
    after_excerpt: |-
      let quantity_key = &unit_fact.structural_node;
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      witness.application() == identity.application()
          && witness.occurrence() == identity.occurrence()
```

+++
