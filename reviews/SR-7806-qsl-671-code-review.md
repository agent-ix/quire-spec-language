---
id: SR-7806
title: "Code review of QSL-671 PR 705 specification delta"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@04e584d63f0a7272d6addbfddf8a50432714aaa; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/test-cases/TC-917-evaluator-invariant-producers-have-explicit-classifications.md, spec/tests.md, qsl-eval/src/value/expression/evaluate.rs, Cargo.lock"
review_set: subset
---

## Summary

Reviewed the exact spec delta and traced its new caller and dependency-source claims. The Rational caller proof matches the locked kernel source, but the new normative verification still anchors itself to a pinned upstream revision through Cargo.lock.

## Verdict

**FAIL** — the changed specification newly directs verification through a locked upstream git revision, which the repository policy prohibits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new inspection obligation anchors its proof to the `quire-exact` git revision selected by Cargo.lock, preserving a pin-based source check after replacing the explicit revision URL. | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:531 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc |

### Disposition details

- FND-001: `fixed` at `a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc`. The FR-090 text now grounds the proof in the typed API and production caller and contains no Cargo.lock or source-revision anchor.

  After excerpt: For the fixed Rational caller only, QSL SHALL verify `ieee-conversion-rejected` by temporary source Inspection rather than a fabricated execution fixture. Inspection SHALL establish from the typed API and production caller that `NodeKind::IeeeToRational` calls `ieee_to_exact` with `IeeeExactTarget::Rational(domain)`, and that this target supplies the Rational domain and returns `Ok(Outcome::from_stop(to_exact(...)))`; only Decimal, Integer and BoundedInteger targets return outer `Err(IllTyped)`.

### Disposition pass 2 check

Rechecked FND-001 against `04e584d63f0a7272d6addbfddf8a50432714aaa`; its latest disposition remains `fixed` at `a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc`. No new code-review finding; no duplicate disposition row added.
