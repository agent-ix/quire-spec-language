---
id: SR-7808
title: "Integrity review of QSL-671 PR 705 spec delta"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@04e584d63f0a7272d6addbfddf8a50432714aaa; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/test-cases/TC-917-evaluator-invariant-producers-have-explicit-classifications.md, spec/tests.md"
review_set: subset
---

## Summary

The new criteria distinguish present caller inspection from future API-adoption cleanup, and TC-917 carries both obligations. Integrity review found a high-severity policy conflict: the present inspection proof is tied to the exact upstream git source revision in Cargo.lock.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new inspection obligation anchors its proof to the `quire-exact` git revision selected by Cargo.lock, preserving a pin-based source check after replacing the explicit revision URL. | FR-090-AC-19 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc |

### Disposition details

- FND-001: `fixed` at `a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc`. The FR-090 text now grounds the proof in the typed API and production caller and contains no Cargo.lock or source-revision anchor.

  After excerpt: For the fixed Rational caller only, QSL SHALL verify `ieee-conversion-rejected` by temporary source Inspection rather than a fabricated execution fixture. Inspection SHALL establish from the typed API and production caller that `NodeKind::IeeeToRational` calls `ieee_to_exact` with `IeeeExactTarget::Rational(domain)`, and that this target supplies the Rational domain and returns `Ok(Outcome::from_stop(to_exact(...)))`; only Decimal, Integer and BoundedInteger targets return outer `Err(IllTyped)`.

### Disposition pass 2 check

Rechecked FND-001 against `04e584d63f0a7272d6addbfddf8a50432714aaa`; its latest disposition remains `fixed` at `a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc`. No new integrity finding; no duplicate disposition row added.
