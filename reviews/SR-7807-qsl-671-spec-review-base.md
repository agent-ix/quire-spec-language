---
id: SR-7807
title: "Base spec review of QSL-671 PR 705 delta"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@04e584d63f0a7272d6addbfddf8a50432714aaa; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/test-cases/TC-917-evaluator-invariant-producers-have-explicit-classifications.md, spec/tests.md"
review_set: subset
---

## Summary

Reviewed the changed FR-090 obligations, TC-917 procedures, trace index and their surrounding contract. The caller proof and adoption cleanup are stated clearly, but the new inspection obligation tracks the locked upstream source revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new inspection obligation anchors its proof to the `quire-exact` git revision selected by Cargo.lock, preserving a pin-based source check after replacing the explicit revision URL. | FR-090-AC-19 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | TC-917 directs inspection of an immutable kernel source linked in FR-090, but the revised FR-090 contains no such source link; the test procedure's source reference is unresolved. | TC-917 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc |
| FND-002 | fixed | 04e584d63f0a7272d6addbfddf8a50432714aaa |

### Disposition details

- FND-001: `fixed` at `a6db4f66a496ec71b1992d6d9605a6a8ba2bbfbc`. The FR-090 text now grounds the proof in the typed API and production caller and contains no Cargo.lock or source-revision anchor.

  After excerpt: For the fixed Rational caller only, QSL SHALL verify `ieee-conversion-rejected` by temporary source Inspection rather than a fabricated execution fixture. Inspection SHALL establish from the typed API and production caller that `NodeKind::IeeeToRational` calls `ieee_to_exact` with `IeeeExactTarget::Rational(domain)`, and that this target supplies the Rational domain and returns `Ok(Outcome::from_stop(to_exact(...)))`; only Decimal, Integer and BoundedInteger targets return outer `Err(IllTyped)`.

- FND-002: New medium finding from this disposition pass. TC-917 lines 53–54 still say to inspect the immutable kernel source linked in FR-090, but the revised FR-090 has no link. Clarify the procedure so its source/API/caller inspection target is resolvable without restoring a revision pin.

- FND-002 round 2: `fixed` at `04e584d63f0a7272d6addbfddf8a50432714aaa`. TC-917 now identifies the production `Machine::apply` arm and QSL caller path, the typed `quire_exact::ieee_to_exact` / `IeeeExactTarget` API, and its implementation path `quire-exact/src/ieee.rs`. The API and implementation were inspected in the Cargo-resolved dependency source, and the production caller passes `IeeeExactTarget::Rational(domain)`; the API implementation returns outer `Err(IllTyped)` only for Decimal, Integer, and BoundedInteger. FR-090 remains free of a Cargo.lock/revision anchor.

  After excerpt: `NodeKind::IeeeToRational` arm in `Machine::apply` in `qsl-eval/src/value/expression/evaluate.rs`, together with the typed API `quire_exact::ieee_to_exact` and `quire_exact::IeeeExactTarget` and their implementation in `quire-exact/src/ieee.rs`.
