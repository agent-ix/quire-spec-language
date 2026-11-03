<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026110216375 round=2 model=gpt-6.1-sol run=9233d71b-20b4-4e4c-84dc-17041204d4ad -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-002 | still-open | The coverage check has no test for a duplicate observation identity at a new stamp, and implementation uses identity alone. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-002
    outcome: still-open
    reason: "FR-502 AC-7 requires refusing an admitted position at/before retained through that the retained assertion did not cover. `LedgerEntry.covered` drops ObservedPosition.stamp, so the same observation identity at a new stamp appears covered. Add an identity-plus-stamp regression test and retain both position fields. See qsl-eval/src/temporal/ledger.rs:93,240,306-308."
```

+++
