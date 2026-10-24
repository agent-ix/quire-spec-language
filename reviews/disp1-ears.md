<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13546 round=1 model=claude-opus-5-5 run=9F8790A4-6653-4A89-BBBF-6C1FC5C1A890 -->

SR file: `reviews/qsl-570-pr1098-spec-review-ears.md` (SR-13546), round 1 at head 7ff6d6491.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |

Verdict: FND-001 is fixed. The sentence is now one EARS unwanted-behaviour statement with EN-4 as its subject.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: If the sampler returns an event sample for a mean-of-fraction claim, then EN-4 SHALL count it as the fraction
    1 when the event holds and 0 when it does not.
```

+++
