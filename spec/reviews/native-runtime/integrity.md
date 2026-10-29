---
id: SR-090
title: "Native runtime integrity review"
type: SpecReview
analysis: integrity
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

Each stage has one authority and an explicit success gate. The revised packet distinguishes structural construction, runtime validity, concrete truth and portable/backend acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: a draft-only constructor error cannot honestly contain an authored native span or computed artifact digest; InputError retains supplied labels and a typed draft path until validation can attach real source/model loci. | FR-018-AC-7; FR-007-AC-13; TC-055; TC-064 |
| FND-002 | medium | Resolved: event entry now preflights expression/depth/event capacity before committing a step or entry event. Antecedent completion has its own capacity check after actual operand work. | FR-008-AC-9; FR-008-AC-16; FR-008-AC-17; TC-073 |
| FND-003 | low | Resolved: local ValueId integers cannot prove which draft a caller originally copied them from; the contract explicitly promises only local bounds/ordering checks. | FR-018-AC-2; TC-055 |

## Traceability and atomicity

| User need | Scoped requirement | Stakeholder lineage | Verification |
| --- | --- | --- | --- |
| US-003 finite assessment | FR-018 exact input construction | StR-001 through US-003 | TC-055–057 |
| US-003 honest input outcome | FR-007 validation | StR-001 through US-003 | TC-058–066 and TC-077 |
| US-003 concrete truth | FR-008 reference execution | StR-001 through US-003 | TC-067–077 |
| US-003 bounded operation | NFR-006 constrains all three FRs | StR-001 | TC-057/065/074/075 |

Each FR has a named subject, one main obligation, concrete typed I/O and adverse
criteria. The measurable NFR uses catalog metric rows rather than synthetic AC
numbers. Construction never establishes model validity; validation never
evaluates; the interpreter consumes constructor-private context. Exact bytes
are distinct from semantic equality. Structural storage equality admits
Option/Seq for frames without broadening source-language equality.

The existing model frame grants object-field/create/delete permissions only.
Preserving State-root storage values is the closed-frame interpretation of
that exact admitted permission set; a replacement permission is explicitly a
future versioned model extension. Missing counterpart data is incomplete.
Precondition context matches the adopted rules, independent of retrospective
pre/post effect validation.

No FR invokes an external CLI, authenticated API, pagination, generator process
or network retry. No interactive/CI dual mode is implicit. Multiple artifacts
with the same labels are errors rather than first-wins selection. Existing
landed APIs supply all implementation prerequisites; missing full package/
projection acceptance is not covered by a stub or hardcoded success.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.

## Constructor setup correction — 2026-09-09

Reviewed dc63f337fcdaee1320d221383eca38599239f62c. The TC-055–057 shared
CheckedPackage setup sentence contradicted FR-018's pre-validation boundary;
the correction removes that hidden prerequisite. US-003 → FR-018 → TC-055–057,
NFR-006-M-1..5 and the existing three-case matrix mapping remain unchanged.
All behavior remains observable through public constructors and structured
errors. No norm, alternative interpretation, external call or new fallback was
introduced. PASS; the setup inconsistency is resolved.
