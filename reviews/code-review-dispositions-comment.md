<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310139 round=1 model=gpt-6.1-sol run=1A0752F5-E878-43C3-A0F4-AED7C4BECFDA -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `60c4b386` verifies the canonical serialized rows by application and occurrence, then checks null versus the exact emitted ms conversion target. The focused regression passes. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      if facts.authored_operands()[0].target().is_some() {
          assert_ne!(decision["left"]["conversion_target"], Value::Null);
          assert_eq!(
              decision["left"]["conversion_target"], decision["right"]["source_type"],
              "the converted occurrence targets the exact emitted ms type"
          );
          assert!(converted_type
              .replace(decision["right"]["source_type"].clone())
              .is_none());
      } else {
          assert_eq!(decision["left"]["conversion_target"], Value::Null);
      }
```

+++
