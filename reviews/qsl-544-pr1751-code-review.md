---
id: SR-1751001
title: "PR #1751 (QSL-544) code review: refinement requirement record, routing, deadlock item, obligation identity"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/qsl-1181e-memory...ef62a25cc (qsl-semantics check/mod.rs, check/refinement.rs, lowering/refinement_rows.rs, family/requirements.rs; qsl-route deadlock.rs, model_request.rs, request.rs; qsl-analyze model_check/request.rs, zone_check/certificate.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
---

# PR #1751 code review (Rust)

## Summary

Ticket: QSL-544. Head ef62a25cc, base qsl-1181e-memory.

**What the PR does:**
- S3 writes one `temporal-satisfaction` record for each emitted refinement. The record is keyed by the `declaration` occurrence of the refinement's V2 node and carries `PropertyForm::Refinement{liveness: !F_A.is_empty()}`.
- Its result bound is a Boolean type node. When no declaration has located that node, it gets a `generated` occurrence at the refinement.
- Two refinements of one meaning share one record. Removing the equal-record arm fails the existing `two_refinements_of_one_meaning_refuse_naming_both`.

**What I checked and found correct:**
- **Record keying.** A refinement that is not emitted gets no record, and the `v2_emittable` gate agrees with the row and Boolean loop.
- **Obligation identity (AC-5).** An obligation binds the concrete subject, the refinement's occurrence (its node) and the abstract initial snapshots, held as a `BTreeSet<DocumentRef>`.
  - Two requests that differ only in the abstract initial snapshot get different obligations, and a repeated request maps to the same index.
- **Concrete-only deadlock item.** A refinement item brings its concrete subject's deadlock-freedom item and none for the abstract subject.
- **The refinement opt-out.** It matches `terminal.model() == refinement.concrete_model()`. Both are model aliases of the declaring unit, read from `package.terminals()` of the refinement's own package, so a same-named model in another package cannot match.
- **Package identity.** The Boolean node and the record are added only to packages that emit a refinement, so no other package's identity changes.
  - The package suite (401 passed, 5 ignored, c14/i2 included) and the refinement fixtures pass at the new identities.

Gates at ef62a25cc, all with `--all-features` and debug info off:

| Crate or check | Result |
| --- | --- |
| qsl-semantics | 810 + 1333 + 30 |
| qsl-package lib | 401 passed, 5 ignored |
| qsl-route | 908 |
| qsl-replay | all passing |
| qsl-analyze | it 1003, lib 93 |
| Workspace clippy `-D warnings` | exit 0 |
| fmt | exit 0 |

## Verdict

**Changes requested for FND-001.** FND-002 is a nit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Temporal and state-graph items still opt out on any `terminal any` in the package, of any model. With `terminal any` on the abstract `Spec` only, a temporal item over the concrete `Impl` subject gets no deadlock-freedom item, and adding the refinement item brings it back. | qsl-route/src/deadlock.rs:217-223 |
| FND-002 | low | `refinement_result_type` was inserted between `located`'s doc lines. `located`'s two doc lines now lead `refinement_result_type`'s doc, and `located` has none. | qsl-semantics/src/check/lowering/refinement_rows.rs:127-142 |
