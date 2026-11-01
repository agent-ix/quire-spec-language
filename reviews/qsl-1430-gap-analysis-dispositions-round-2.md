<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310142 round=2 model=gpt-6 run=957BE8D8-5F94-4036-BD2C-C9D396DA411E -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Distinct source/conversion dimension roles remain available with exact structural IDs and ordinals. |
| FND-002 | fixed | The tests now bind each witness query to the exact application occurrence, including shared-key occurrences. |
| FND-003 | fixed | Compound UnitId facts join through the exact checker-retained type key, with missing/ambiguous refusal coverage. |

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
      witness.application() == identity.application()
          && witness.occurrence() == identity.occurrence()
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      let quantity_key = &unit_fact.structural_node;
```

+++
