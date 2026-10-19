<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13265 round=1 model=claude-opus-5-5 run=2A534AD9-7674-409A-9821-975EA1D37A8D -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: When one operand has type `Int[lo,hi]` and the other, ignoring parentheses, is a binary `+`, `-` or `*` whose operands are `Integer` or `Int[..]`, the checker SHALL type the arithmetic node `Int[lo,hi]` with the obligation that its interval, under the facts that hold where it executes, lies inside `[lo,hi]`, and SHALL refuse an unproved obligation with `undefined_expression`/`unproved-range` located at the arithmetic node, never at the equality, without clamping, wrapping or widening the value.
```

+++
