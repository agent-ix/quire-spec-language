---
id: SR-16247
title: "spec-review of the QSL analyze provider entry, analyze request, engine choice and canonical request (quire-spec-language PR 1341)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ea319988b28efc0cb9191f0880b02c5c3ff2f085; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md; spec/functional/FR-281-analyze-claims-with-in-process-engines.md; spec/functional/FR-292-key-cached-results-by-content-identity.md; spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md; spec/test-cases/TC-762-analyze-settles-proved-refuted-and-unsupported-items-with-one-record.md; spec/test-cases/TC-776-cache-keys-change-with-every-meaning-input-and-nothing-else-and-reads.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-292
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: references
---

# Spec review of the QSL analyze provider entry, analyze request, engine choice and canonical request (quire-spec-language PR 1341)

## Summary

Ticket: STD-370 (QSL-596). This is one spec-review pass over
`git diff origin/main...ea319988b`: FR-280's
`qsl_analyze::provider::run` entry and `RoutedItem`, FR-281's `analyze`
signature, `AnalyzeRequest`, `SubjectBinding`, `ClaimItem`,
`AnalyzeLimits`, `AnalyzeOutcome`, `AnalyzeRefusal` and the new "Engine
choice", FR-292's "Canonical analyze request" member table, the ADR-029
PV-3 paragraph, and the TC-761, TC-762 and TC-776 steps. The oracles were
STD-370's gap list, FR-075, FR-120, FR-288, ADR-029 OP-2 and PV-3, QSpec
FR-290 "Candidate set and negotiation", and QSpec FR-331 (items,
`candidates`, `declined` causes, `hybrid.limits.max_solver_time`, AC-4).

Gates: `QSPEC_DIR=~/dev/quire-specification make check-spec-validation`
at ea319988b exits 0 with no warning on a touched file. origin/main
(c21bc57a) is an ancestor of the head. FR-280-AC-6, FR-281-AC-8,
FR-281-AC-9 and FR-292-AC-4 appear in no other open PR.

Examined, clean:

- **The QSL-owned entry.** `run(&CheckedPackage, &SubjectBinding,
  &[RoutedItem], &AnalyzeLimits, &Cancel) -> Vec<TerminalRecord>` names no
  driver, CG or RT type. This resolves the contradiction between FR-280
  and FR-280-AC-3, and the ADR-029 PV-3 paragraph places the adapter in the
  driver.
- **The binding refusal through the entry** settles each item `declined`
  with `invalid-input` or `incomplete-input`. These are QSpec FR-331
  `declined` causes, and FR-280-AC-6 fixes the FR-120 codes.
- **The canonical request table.** Its members are `format`, `binding`,
  `items` and `limits`. It excludes `deadline_ms`, `candidates` and
  `hybrid.limits.max_solver_time`, the wall-clock settings QSpec FR-331
  keeps out of request identity. It includes the subject binding by digest,
  which moves results. FR-292-AC-4 covers the binding members and the
  reordering of domain packages.
- **The lead defaults are flagged** in the PR body as "lead default, not an
  owner answer".

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The engine-choice rule contradicts QSpec FR-290 "Candidate set and negotiation" and QSpec FR-331-AC-4. QSpec says negotiation "SHALL NOT select among several candidates". Registration order never changes a disposition, and "the caller resolves a multi-candidate item by naming one backend". More than one candidate with no backend named settles `invalid-request`, `invalid_capability`/`ambiguous-backend`. An unknown-backend mark settles `invalid-request`, `invalid_capability`/`unknown-backend`, not `unsupported`. Candidates match on kind alone, and the mode is compared only at the single candidate's arm. FR-281 instead runs the first candidate in bytewise `BackendId` order whose pairs hold the item's mode, and it settles an unknown named backend `unsupported`. FR-281-AC-8 builds this contradiction into a test. This also breaks FR-280's rule that `prove` and `analyze` produce the same record for an item (ADR-029 OP-2): through `prove`, the driver's `negotiate_*` settles AC-8's unnamed item `invalid-request`/`ambiguous-backend`, while `analyze` runs it on `qsl.test-a`. Fix: follow FR-290's candidate table in `analyze`, and restate AC-8. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:50-63, :139-141, :183; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:51 | wrong-requirement |
| FND-002 | medium | FR-281-AC-8 cannot be set up through the stated interface. It needs test engines `qsl.test-a` and `qsl.test-b` in `analyze`'s registry. But `analyze(package, request, limits, cancel)` takes no registry, and the registry "is built from the QSL engines' provider manifests". No requirement states a seam through which a test registers an engine. Fix: state the registry input or test seam that AC-8 and AC-5's test engines use. | spec/functional/FR-281-analyze-claims-with-in-process-engines.md:50-52, :68-73, :183 | missing-requirement |
| FND-003 | low | FR-280's equal-record rule ("the terminal record the provider entry returns for an item shall equal the record `analyze` returns for the same item") has no case for a binding that admission refuses. In that case the entry returns one `declined` record per item, while `analyze` returns `StageFailure::Refused` and no record. Fix: exclude a refused binding from the equal-record rule, and say so. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:84-89, :133-136, :142-144 | missing-requirement |

## Verdict

Not mergeable. FND-001 makes `analyze` select among several candidates,
which QSpec FR-290 forbids. It also makes `analyze` and `prove` settle the
same item differently, so the FR-280 equal-record guarantee fails. FND-002
and FND-003 are a missing test seam and a missing case. The entry
signature, the binding refusal and the canonical member table are sound.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | d1e39d04: FR-281 "Negotiated items": "`analyze` does not negotiate and never chooses among engines". Each `NegotiatedItem` carries the settlement CG's `negotiate_*` gave it under QSpec FR-290's candidate table. An unnamed two-candidate item reaches `analyze` settled `invalid-request` `invalid_capability`/`ambiguous-backend`, as through `prove`, and an unheld candidate settles `invalid-request`/`unknown-backend`. FR-281-AC-8 is restated to match, so FR-280's equal-record rule holds. |
| FND-002 | fixed | d1e39d04: `analyze` and `qsl_analyze::provider::run` take `engines: &Engines`. `Engines::qsl()` holds the layer-A engines, and `Engines::with` adds a `qsl_analyze::Engine`, which is the seam AC-5's and AC-8's test engines use. |
| FND-003 | fixed | d1e39d04: "If FR-120 admission does not admit the subject binding, then the equal-record rule shall not apply: the provider entry returns one `declined` record per item, while `analyze` returns `StageFailure::Refused` and no record." |
