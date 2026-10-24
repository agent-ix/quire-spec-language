<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-1360 round=4 reviewed=4865d61bdbe85914e6d8342276152bcaef686a95 model=gpt-6 run=9ac0f31a-6e1f-4ead-8326-46a58791c760 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-007 | fixed | 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-008 | fixed | 4865d61bdbe85914e6d8342276152bcaef686a95 |
| FND-009 | still-open | ADR-014 §2 still says QSpec FR-290 has ten kinds, conflicting with FR-057's twelve-label vocabulary. |

FND-006 (FR-075-AC-7, `spec/functional/FR-075-compute-candidates-from-registered-backends.md`; TC-982):

> A repeat differing only in any of those compared members is unequal and follows FR-075-AC-4; manifest digest does not affect descriptor equality.

FND-007 (`spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md`):

> complete descriptor -- same identity, provider origin, advertised pairs, retained `domains`, and evidence-kind set -- are one registration, and a repeat is not refused.

FND-008 (`spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md`):

> QSL S3 records `Requirements{kind, evidence_kind, extent, bounds}` per item (§4); `evidence_kind` carries the item's unmodified label when present for `probabilistic-satisfaction` and remains absent when the item omits it, so negotiation can distinguish valid, absent and unknown evidence.

FND-009 remains open at ADR-014 §2, lines 83-87: “QSpec FR-290 has ten kinds”.

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-006
    outcome: fixed
    fix_sha: 4865d61bdbe85914e6d8342276152bcaef686a95
    after_excerpt: "Given a registration naming a `BackendId` already held by the registry with equal identity, provider origin, advertised pairs, retained `domains`, and evidence-kind set, the repeat is one registration and is not refused; the registry and its candidate sets are unchanged. A repeat differing only in any of those compared members is unequal and follows FR-075-AC-4; manifest digest does not affect descriptor equality. A different identity is a different registry key."
  - fnd: FND-007
    outcome: fixed
    fix_sha: 4865d61bdbe85914e6d8342276152bcaef686a95
    after_excerpt: "complete descriptor -- same identity, provider origin, advertised pairs, retained `domains`, and evidence-kind set -- are one registration, and a repeat is not refused."
  - fnd: FND-008
    outcome: fixed
    fix_sha: 4865d61bdbe85914e6d8342276152bcaef686a95
    after_excerpt: "QSL S3 records `Requirements{kind, evidence_kind, extent, bounds}` per item (§4); `evidence_kind` carries the item's unmodified label when present for `probabilistic-satisfaction` and remains absent when the item omits it, so negotiation can distinguish valid, absent and unknown evidence."
  - fnd: FND-009
    outcome: still-open
    reason: "ADR-014 §2, lines 83-87, still states that QSpec FR-290 has ten kinds, contradicting the twelve labels in FR-057."
```

+++
