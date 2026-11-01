<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310139 round=1 model=gpt-6.1-sol run=3EA773AD-21CF-45FA-B88F-E2647C259A9B -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      assert_eq!(
          (fault.code(), fault.tag()),
          (Code::UndefinedExpression, "unproved-range")
      );
      assert_eq!(fault.span(), Some(first(&text, "rational(3, 4)")));
      assert_eq!(
          (fault.code(), fault.tag()),
          (Code::IllTyped, "ambiguous-literal")
      );
      assert_eq!(fault.span(), Some(first(&text, "rational(1, 2)")));
  - fnd: FND-002
    outcome: fixed
    after_excerpt: |-
      /// A relation body admits the same shared value forms as a Boolean function
      /// body checked by the value typer.
```

+++
